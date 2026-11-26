//! Whether an edit's bundle writes an <Audio> that always plays (raea-runtime-export: a render with no sound although it
//! does is an error, not a silent file).

/// Whether a sound's file is there, saying so once when it isn't (the caller remembers each file): a render dropped
/// it without a word and then refused as silent, with nothing naming the file (0.1.9: three reports in a day, each
/// read as a Windows bug; the files were in the project's assets/, which an edit in compositions/ didn't search).
pub fn there(file: &std::path::Path, src: &str) -> bool {
    let there = file.exists();
    if !there {
        eprintln!("raea: no sound file {} (src \"{src}\"), so it's silent: staticFile looks in the project's media/, assets/ and public/", file.display());
    }
    there
}

/// True when the bundle's JSX has an <Audio> call, `(…jsx)(Audio, {` or `jsx(Audio, {`, that isn't behind a condition
/// directly or through any enclosing wrapper (such as `<Sequence>`, `<Mix>`, `if`/`else`, ternary, `&&`, `||`, `??`,
/// or `.map(...)`). One in an unconditional wrapper's children (`(…jsx)(Mix, { children: (…jsx)(Audio, {`) still
/// returns true so a wrapper that drops its children is caught.
pub fn always_plays_audio(bundle: &str) -> bool {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Delim { Paren, Bracket, Brace }
    #[derive(Clone)]
    struct Scope {
        delim: Delim,
        inherited_cond: bool,
        expr_cond: bool,
        stmt_cond: bool,
        ternary_depth: usize,
        pending_block_cond: bool,
    }
    impl Scope {
        fn is_cond(&self) -> bool { self.inherited_cond || self.expr_cond || self.stmt_cond }
    }

    let b = bundle.as_bytes();
    let n = b.len();
    let mut stack = vec![Scope { delim: Delim::Brace, inherited_cond: false, expr_cond: false, stmt_cond: false, ternary_depth: 0, pending_block_cond: false }];
    let mut tpl_braces: Vec<usize> = Vec::new();
    let mut i = 0;
    let mut prev_word = String::new();
    let mut prev_had_dot = false;
    let mut word_after_dot = false;
    let mut prev_close_paren = false;
    let mut just_closed_cond_paren = false;
    let mut just_saw_else = false;
    let mut can_start_regex = true;

    let is_ident_cont = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'$';

    while i < n {
        let c = b[i];
        if c.is_ascii_whitespace() { i += 1; continue; }
        // comments
        if c == b'/' && i + 1 < n && b[i + 1] == b'/' {
            i += 2;
            while i < n && b[i] != b'\n' { i += 1; }
            continue;
        }
        if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(b[i] == b'*' && b[i + 1] == b'/') { i += 1; }
            i = (i + 2).min(n);
            continue;
        }
        // string literals
        if c == b'\'' || c == b'"' {
            let q = c;
            i += 1;
            while i < n {
                if b[i] == b'\\' { i += 2; continue; }
                if b[i] == q { i += 1; break; }
                i += 1;
            }
            prev_word.clear();
            prev_had_dot = false;
            word_after_dot = false;
            prev_close_paren = false;
            just_closed_cond_paren = false;
            just_saw_else = false;
            can_start_regex = false;
            continue;
        }
        // template literal
        if c == b'`' {
            i += 1;
            while i < n {
                if b[i] == b'\\' { i += 2; continue; }
                if b[i] == b'`' { i += 1; break; }
                if b[i] == b'$' && i + 1 < n && b[i + 1] == b'{' {
                    i += 2;
                    tpl_braces.push(stack.len());
                    let inh = stack.last().is_some_and(Scope::is_cond);
                    stack.push(Scope { delim: Delim::Brace, inherited_cond: inh, expr_cond: false, stmt_cond: false, ternary_depth: 0, pending_block_cond: false });
                    break;
                }
                i += 1;
            }
            prev_word.clear();
            prev_had_dot = false;
            word_after_dot = false;
            prev_close_paren = false;
            just_closed_cond_paren = false;
            just_saw_else = false;
            can_start_regex = tpl_braces.last() == Some(&(stack.len().saturating_sub(1)));
            continue;
        }
        // regex literal
        if c == b'/' && can_start_regex {
            i += 1;
            let mut in_class = false;
            while i < n {
                if b[i] == b'\\' { i += 2; continue; }
                if b[i] == b'[' { in_class = true; }
                else if b[i] == b']' { in_class = false; }
                else if b[i] == b'/' && !in_class { i += 1; break; }
                i += 1;
            }
            while i < n && b[i].is_ascii_alphabetic() { i += 1; }
            prev_word.clear();
            prev_had_dot = false;
            word_after_dot = false;
            prev_close_paren = false;
            just_closed_cond_paren = false;
            just_saw_else = false;
            can_start_regex = false;
            continue;
        }

        // resolve pending braceless `if (cond)` / `else` when next token isn't `{`
        if just_closed_cond_paren && c != b'{' {
            if let Some(top) = stack.last_mut() { top.stmt_cond = true; }
            just_closed_cond_paren = false;
        }
        if just_saw_else && c != b'{' {
            let is_else_if = bundle[i..].starts_with("if") && !b.get(i + 2).copied().is_some_and(is_ident_cont);
            if !is_else_if {
                if let Some(top) = stack.last_mut() { top.stmt_cond = true; }
            }
            just_saw_else = false;
        }

        if !c.is_ascii_digit() && is_ident_cont(c) {
            let start = i;
            i += 1;
            while i < n && is_ident_cont(b[i]) { i += 1; }
            let word = &bundle[start..i];
            if !prev_had_dot && word == "else" {
                just_saw_else = true;
            }
            can_start_regex = !prev_had_dot && (matches!(word, "return" | "throw" | "case" | "in" | "of" | "typeof" | "delete" | "await" | "yield" | "new" | "else" | "do") || word == concat!("vo", "id"));
            word_after_dot = prev_had_dot;
            prev_word = word.to_string();
            prev_had_dot = false;
            prev_close_paren = false;
            continue;
        }

        if c.is_ascii_digit() {
            while i < n && (b[i].is_ascii_alphanumeric() || b[i] == b'.' || b[i] == b'_') { i += 1; }
            prev_word.clear();
            prev_had_dot = false;
            word_after_dot = false;
            prev_close_paren = false;
            can_start_regex = false;
            continue;
        }

        match c {
            b'(' => {
                let is_jsx_callee = prev_close_paren || matches!(prev_word.as_str(), "jsx" | "jsxs" | "jsxDEV");
                if is_jsx_callee && bundle[i + 1..].starts_with("Audio") {
                    let rest = bundle[i + 6..].trim_start_matches(|ch: char| ch.is_ascii_digit()).trim_start();
                    if rest.strip_prefix(',').is_some_and(|r| r.trim_start().starts_with('{')) {
                        if !stack.last().is_some_and(Scope::is_cond) {
                            return true;
                        }
                    }
                }
                let pending_block_cond = !word_after_dot && matches!(prev_word.as_str(), "if" | "switch" | "for" | "while");
                let iter_cond = word_after_dot && matches!(prev_word.as_str(), "map" | "flatMap" | "filter" | "forEach");
                let inh = stack.last().is_some_and(Scope::is_cond) || iter_cond;
                stack.push(Scope { delim: Delim::Paren, inherited_cond: inh, expr_cond: false, stmt_cond: false, ternary_depth: 0, pending_block_cond });
                prev_word.clear();
                prev_had_dot = false;
                word_after_dot = false;
                prev_close_paren = false;
                can_start_regex = true;
                i += 1;
            }
            b'[' => {
                let inh = stack.last().is_some_and(Scope::is_cond);
                stack.push(Scope { delim: Delim::Bracket, inherited_cond: inh, expr_cond: false, stmt_cond: false, ternary_depth: 0, pending_block_cond: false });
                prev_word.clear();
                prev_had_dot = false;
                word_after_dot = false;
                prev_close_paren = false;
                can_start_regex = true;
                i += 1;
            }
            b'{' => {
                let inh = just_closed_cond_paren || just_saw_else || stack.last().is_some_and(Scope::is_cond);
                just_closed_cond_paren = false;
                just_saw_else = false;
                stack.push(Scope { delim: Delim::Brace, inherited_cond: inh, expr_cond: false, stmt_cond: false, ternary_depth: 0, pending_block_cond: false });
                prev_word.clear();
                prev_had_dot = false;
                word_after_dot = false;
                prev_close_paren = false;
                can_start_regex = true;
                i += 1;
            }
            b')' | b']' | b'}' => {
                let want = match c { b')' => Delim::Paren, b']' => Delim::Bracket, _ => Delim::Brace };
                let mut closed_cond = false;
                if let Some(pos) = stack.iter().rposition(|s| s.delim == want) {
                    closed_cond = stack[pos].pending_block_cond;
                    stack.truncate(pos.max(1));
                }
                just_closed_cond_paren = c == b')' && closed_cond;
                prev_close_paren = c == b')';
                prev_word.clear();
                prev_had_dot = false;
                word_after_dot = false;
                can_start_regex = false;
                i += 1;
                if c == b'}' && tpl_braces.last() == Some(&stack.len()) {
                    tpl_braces.pop();
                    while i < n {
                        if b[i] == b'\\' { i += 2; continue; }
                        if b[i] == b'`' { i += 1; break; }
                        if b[i] == b'$' && i + 1 < n && b[i + 1] == b'{' {
                            i += 2;
                            tpl_braces.push(stack.len());
                            let inh = stack.last().is_some_and(Scope::is_cond);
                            stack.push(Scope { delim: Delim::Brace, inherited_cond: inh, expr_cond: false, stmt_cond: false, ternary_depth: 0, pending_block_cond: false });
                            can_start_regex = true;
                            break;
                        }
                        i += 1;
                    }
                }
            }
            b',' => {
                if let Some(top) = stack.last_mut() {
                    top.expr_cond = false;
                    top.ternary_depth = 0;
                }
                prev_word.clear(); prev_had_dot = false; word_after_dot = false; prev_close_paren = false; can_start_regex = true; i += 1;
            }
            b';' => {
                if let Some(top) = stack.last_mut() {
                    top.expr_cond = false;
                    top.stmt_cond = false;
                    top.ternary_depth = 0;
                }
                prev_word.clear(); prev_had_dot = false; word_after_dot = false; prev_close_paren = false; can_start_regex = true; i += 1;
            }
            b'?' => {
                if i + 1 < n && b[i + 1] == b'?' {
                    if let Some(top) = stack.last_mut() { top.expr_cond = true; }
                    prev_had_dot = false;
                    can_start_regex = true;
                    i += 2;
                } else if i + 1 < n && b[i + 1] == b'.' && !b.get(i + 2).is_some_and(|ch| ch.is_ascii_digit()) {
                    prev_had_dot = true;
                    can_start_regex = false;
                    i += 2;
                } else {
                    if let Some(top) = stack.last_mut() {
                        top.expr_cond = true;
                        top.ternary_depth += 1;
                    }
                    prev_had_dot = false;
                    can_start_regex = true;
                    i += 1;
                }
                prev_word.clear(); word_after_dot = false; prev_close_paren = false;
            }
            b':' => {
                if let Some(top) = stack.last_mut() && top.ternary_depth > 0 {
                    top.ternary_depth -= 1;
                    top.expr_cond = true;
                }
                prev_word.clear(); prev_had_dot = false; word_after_dot = false; prev_close_paren = false; can_start_regex = true; i += 1;
            }
            b'&' if i + 1 < n && b[i + 1] == b'&' => {
                if let Some(top) = stack.last_mut() { top.expr_cond = true; }
                prev_word.clear(); prev_had_dot = false; word_after_dot = false; prev_close_paren = false; can_start_regex = true; i += 2;
            }
            b'|' if i + 1 < n && b[i + 1] == b'|' => {
                if let Some(top) = stack.last_mut() { top.expr_cond = true; }
                prev_word.clear(); prev_had_dot = false; word_after_dot = false; prev_close_paren = false; can_start_regex = true; i += 2;
            }
            b'.' => {
                prev_had_dot = !prev_word.is_empty() || prev_close_paren;
                word_after_dot = false;
                prev_close_paren = false;
                can_start_regex = false;
                i += 1;
            }
            _ => {
                prev_word.clear();
                prev_had_dot = false;
                word_after_dot = false;
                prev_close_paren = false;
                can_start_regex = matches!(c, b'=' | b'!' | b'&' | b'|' | b'^' | b'~' | b'+' | b'-' | b'*' | b'%' | b'<' | b'>');
                i += 1;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::always_plays_audio;

    #[test]
    fn an_audio_behind_a_condition_isnt_one_that_always_plays() {
        let starter = r#"children: c.kind === "audio" ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Audio, { src: c.src }) : /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Video, { src: c.src })"#;
        assert!(!always_plays_audio(starter));
        assert!(!always_plays_audio(r#"music && /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Audio2, { src: music })"#));
        // ternary else branch and direct jsx() form
        assert!(!always_plays_audio(r#"c.kind === "video" ? jsx(Video, { src: v }) : /* @__PURE__ */ jsx(Audio, { src: a })"#));
        // conditional wrapper sequences (&&, ternary, if/else, .map)
        assert!(!always_plays_audio(r#"show && /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Sequence, { from: 0, children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" }) })"#));
        assert!(!always_plays_audio(r#"show ? /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Sequence, { from: 30, children: [(0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" })] }) : null"#));
        assert!(!always_plays_audio(r#"(0, import_jsx_runtime.jsx)(Mix, { children: show && /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Sequence, { children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" }) }) })"#));
        assert!(!always_plays_audio(r#"function C({ on }) { if (on) { return (0, import_jsx_runtime.jsx)(Sequence, { children: (0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" }) }); } return null; }"#));
        assert!(!always_plays_audio(r#"function C({ on }) { if (on) return (0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" }); return null; }"#));
        assert!(!always_plays_audio(r#"fallback || /* @__PURE__ */ jsx(Audio, { src: "a.wav" }); primary ?? jsx(Audio, { src: "b.wav" })"#));
        assert!(!always_plays_audio(r#"hits.map((f) => (0, import_jsx_runtime.jsx)(Sequence, { from: f, children: (0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" }) }))"#));
        assert!(always_plays_audio(r#"const re = /[(?]/; const s = `ok ${1} &&`; return (0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" })"#));
        // a wrapper's child, and a call on its own: both always play
        assert!(always_plays_audio(r#"(0, import_jsx_runtime.jsx)(Mix, { children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Audio, { src: "bed.wav" }) })"#));
        assert!(always_plays_audio(r#"return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(AbsoluteFill, { children: [(0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" })] })"#));
        assert!(always_plays_audio(r#"return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Sequence, { from: 0, children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Audio, { src: "a.wav" }) })"#));
        // the component's own definition and a type check aren't calls of it
        assert!(!always_plays_audio("function Audio(props) {}\nif (c.type === Audio) {}"));
    }
}
