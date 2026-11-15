// A lossless reader and writer for the slice of Org mode (orgmode.org) a plan uses: headlines with a TODO keyword, a
// [#A] priority and :tags:, the planning line under them (SCHEDULED, DEADLINE, CLOSED), the PROPERTIES drawer, and
// everything else as the lines it was. printOrg(parseOrg(text)) is text, byte for byte. An entry that changed is
// written again part by part: a headline that didn't change keeps its line (one whose keyword or words changed keeps
// its spacing and tag column), a drawer keeps the lines of the properties that didn't change, a planning line keeps
// its order and what raea can't read, and body text (lists, other drawers, prose) is never rewritten.
import { formatStamp, parseStamp, type Timestamp } from "./time.ts"

export interface Planning {
  closed?: Timestamp
  deadline?: Timestamp
  scheduled?: Timestamp
  /** the parts raea doesn't read (a range across days, a stamp it can't parse, a key written twice), each as written
   *  ("SCHEDULED: <2026-11-20 Fri>--<2026-11-22 Sun>"), kept when the line is written again */
  rest?: string[]
}

/** A PROPERTIES drawer, in order, keys as written. */
export type Props = [key: string, value: string][]

export interface Section {
  level: number
  keyword?: string
  priority?: string
  title: string
  tags: string[]
  planning: Planning
  /** the PROPERTIES drawer; undefined when the entry has none */
  props?: Props
  /** every line after the headline, its planning line and its drawer, up to the next headline, as written */
  body: string[]
  /** how it was written, so what didn't change prints as it was; none on a new entry */
  src?: Source
}

interface Source {
  headline: string
  /** the headline's parts as read */
  head: Head
  planning?: string
  pk: string
  drawer?: string[]
  props: Props
  /** the indentation of its planning line or drawer, kept for lines written into them */
  indent: string
}

export interface OrgDoc {
  /** the lines before the first headline (#+TITLE:, #+TODO:, text), as written */
  head: string[]
  sections: Section[]
  /** lines end in CRLF: a line raea writes ends so too */
  cr: boolean
  /** the text ends with a line break */
  final: boolean
}

const HEAD = /^(\*+)[ \t]+(.*)$/
const TAGS = /(?:^|[ \t]+)(:[\p{L}\p{N}_@#%:]+:)$/u
// a planning line's parts: each stamp may be a range of days, <2026-11-20 Fri>--<2026-11-22 Sun>
const STAMP_TEXT = String.raw`[<[][^>\]]*[>\]](?:--[<[][^>\]]*[>\]])?`
const PLAN_LINE = new RegExp(String.raw`^[ \t]*(?:(?:SCHEDULED|DEADLINE|CLOSED):[ \t]*${STAMP_TEXT}[ \t]*)+$`)
const PLAN_PART = new RegExp(String.raw`(SCHEDULED|DEADLINE|CLOSED):[ \t]*(${STAMP_TEXT})`, "g")
const PLAN_KEYS = ["closed", "deadline", "scheduled"] as const
const DRAWER_START = /^[ \t]*:PROPERTIES:[ \t]*$/i
const DRAWER_END = /^[ \t]*:END:[ \t]*$/i
const PROP = /^[ \t]*:([^\s:]+):(?:[ \t]+(.*?))?[ \t]*$/

/** A line without the \r of a CRLF ending. */
export const bare = (line: string) => (line.endsWith("\r") ? line.slice(0, -1) : line)

/** A headline's parts, or null when the line isn't a headline. `keywords` are the TODO keywords the file knows. */
export function readHeadline(line: string, keywords: ReadonlySet<string>): Omit<Section, "planning" | "body"> | null {
  const m = HEAD.exec(bare(line))
  if (!m) return null
  let rest = m[2].trimEnd()
  let keyword: string | undefined
  const kw = /^(\S+)(?:[ \t]+|$)/.exec(rest)
  if (kw && keywords.has(kw[1])) {
    keyword = kw[1]
    rest = rest.slice(kw[0].length)
  }
  let priority: string | undefined
  const pr = /^\[#([A-Z0-9])\](?:[ \t]+|$)/.exec(rest)
  if (pr) {
    priority = pr[1]
    rest = rest.slice(pr[0].length)
  }
  let tags: string[] = []
  const tg = TAGS.exec(rest)
  if (tg) {
    tags = tg[1].split(":").filter(Boolean)
    rest = rest.slice(0, tg.index)
  }
  return { level: m[1].length, ...(keyword && { keyword }), ...(priority && { priority }), title: rest.trim(), tags }
}

/** A planning line's stamps. Whatever it can't read is kept in `rest`, so the line is still the planning line and the
 *  drawer under it is still read. */
function readPlanning(line: string): Planning | null {
  const b = bare(line)
  if (!PLAN_LINE.test(b)) return null
  const p: Planning = {}
  const rest: string[] = []
  for (const m of b.matchAll(PLAN_PART)) {
    const key = m[1].toLowerCase() as (typeof PLAN_KEYS)[number]
    const t = parseStamp(m[2])
    if (t && !p[key]) p[key] = t
    else rest.push(m[0])
  }
  if (rest.length) p.rest = rest
  return p
}

/** The drawer starting at `at`: its properties and where it ends, or null when the lines there aren't one. */
function readDrawer(lines: string[], at: number, end: number): { props: Props; next: number } | null {
  if (at >= end || !DRAWER_START.test(bare(lines[at]))) return null
  const props: Props = []
  for (let i = at + 1; i < end; i++) {
    const b = bare(lines[i])
    if (DRAWER_END.test(b)) return { props, next: i + 1 }
    const m = PROP.exec(b)
    if (!m) return null
    props.push([m[1], m[2] ?? ""])
  }
  return null
}

type Head = Pick<Section, "level" | "keyword" | "priority" | "title" | "tags">
const headKey = (s: Head) => JSON.stringify([s.level, s.keyword ?? "", s.priority ?? "", s.title, s.tags])
const planKey = (p: Planning) => (p.closed || p.deadline || p.scheduled || p.rest?.length ? JSON.stringify([p.closed, p.deadline, p.scheduled, p.rest]) : "")

/** Reads an Org file. `keywords` are the TODO keywords to read headlines with (the file's #+TODO lines and the
 *  registry's; model.ts keywordsOf gathers them). */
export function parseOrg(text: string, keywords: ReadonlySet<string>): OrgDoc {
  const final = text.endsWith("\n")
  const lines = text === "" ? [] : text.split("\n")
  if (final) lines.pop()
  const doc: OrgDoc = { head: [], sections: [], cr: text.includes("\r\n"), final }
  const starts: number[] = []
  lines.forEach((l, i) => HEAD.test(bare(l)) && starts.push(i))
  doc.head = lines.slice(0, starts[0] ?? lines.length)
  starts.forEach((start, n) => {
    const end = starts[n + 1] ?? lines.length
    const h = readHeadline(lines[start], keywords)!
    let at = start + 1
    let planning: Planning = {}
    let planningLine: string | undefined
    const p = at < end ? readPlanning(lines[at]) : null
    if (p) {
      planning = p
      planningLine = lines[at]
      at++
    }
    const d = readDrawer(lines, at, end)
    const drawer = d ? lines.slice(at, d.next) : undefined
    if (d) at = d.next
    const indent = /^[ \t]*/.exec(bare(planningLine ?? drawer?.[0] ?? ""))![0]
    const s: Section = { ...h, planning, ...(d && { props: d.props }), body: lines.slice(at, end) }
    s.src = { headline: lines[start], head: { level: h.level, keyword: h.keyword, priority: h.priority, title: h.title, tags: [...h.tags] }, planning: planningLine, pk: planKey(planning), drawer, props: d ? d.props.map(([k, v]) => [k, v]) : [], indent }
    doc.sections.push(s)
  })
  return doc
}

/** A headline as Org writes it, its tags ending at column 77 (org-tags-column -77). */
export function renderHeadline(s: Head): string {
  const left = [`${"*".repeat(s.level)}`, s.keyword, s.priority && `[#${s.priority}]`, s.title].filter(Boolean).join(" ")
  if (!s.tags.length) return left
  const tags = `:${s.tags.join(":")}:`
  return `${left}${" ".repeat(Math.max(1, 77 - left.length - tags.length))}${tags}`
}

/** A headline whose keyword, priority or words changed, written into its old line the way Org's org-todo does: the
 *  line's own spacing kept, its tags ending where they ended. Undefined when its stars or tags changed too, or the
 *  old line can't be followed; renderHeadline writes it then. */
function editHeadline(line: string, was: Head, s: Head): string | undefined {
  if (was.level !== s.level || was.tags.join(":") !== s.tags.join(":") || !was.title || !s.title) return undefined
  let at = /^\*+[ \t]+/.exec(line)![0].length
  const prefix = line.slice(0, at)
  /** a part of the old line (its keyword, its priority) and the space after it */
  const part = (text: string | undefined) => {
    if (!text) return { gap: " " }
    if (!line.startsWith(text, at)) return undefined
    const gap = /^[ \t]*/.exec(line.slice(at + text.length))![0]
    at += text.length + gap.length
    return { gap }
  }
  const kw = part(was.keyword)
  const pr = kw && part(was.priority && `[#${was.priority}]`)
  if (!pr || !line.startsWith(was.title, at)) return undefined
  const end = at + was.title.length
  const left = `${prefix}${s.keyword ? s.keyword + kw!.gap : ""}${s.priority ? `[#${s.priority}]${pr.gap}` : ""}${s.title}`
  const tail = line.slice(end)
  const pad = /^[ \t]*/.exec(tail)![0]
  const next = s.tags.length ? left + " ".repeat(Math.max(1, pad.length + end - left.length)) + tail.slice(pad.length) : left + tail
  // read back as it was meant, or not used
  const back = readHeadline(next, new Set(s.keyword ? [s.keyword] : []))
  return back && headKey(back) === headKey(s) ? next : undefined
}

/** A planning line again: its parts in the order they were written, a new CLOSED first (as Org puts it) and a new
 *  DEADLINE or SCHEDULED last, then the parts raea doesn't read, as written. */
function renderPlanning(p: Planning, was?: string): string {
  const order = was ? [...bare(was).matchAll(/(SCHEDULED|DEADLINE|CLOSED):/g)].map((m) => m[1].toLowerCase()) : []
  const keys = [...PLAN_KEYS.filter((k) => k === "closed" && !order.includes(k)), ...order, ...PLAN_KEYS.filter((k) => k !== "closed" && !order.includes(k))]
  const parts = [...new Set(keys)].flatMap((k) => {
    const t = p[k as (typeof PLAN_KEYS)[number]]
    return t ? [`${k.toUpperCase()}: ${formatStamp(t)}`] : []
  })
  return [...parts, ...(p.rest ?? [])].join(" ")
}

/** A property line the way Org's org-property-format writes it ("%-10s %s"). */
const renderProp = (indent: string, k: string, v: string) => `${indent}${`:${k}:`.padEnd(10)}${v ? ` ${v}` : ""}`.trimEnd()

/** One entry's lines: the parts that didn't change as they were written, the rest written again. */
export function printSection(s: Section, cr: boolean): string[] {
  const nl = (l: string) => (cr ? `${l}\r` : l)
  const src = s.src
  const indent = src?.indent ?? ""
  const out = [src && headKey(src.head) === headKey(s) ? src.headline : nl((src && editHeadline(bare(src.headline), src.head, s)) ?? renderHeadline(s))]
  const pk = planKey(s.planning)
  if (src?.planning !== undefined && src.pk === pk) out.push(src.planning)
  else if (pk) out.push(nl(indent + renderPlanning(s.planning, src?.planning)))
  if (s.props) {
    const was = src?.drawer
    if (was && JSON.stringify(src!.props) === JSON.stringify(s.props)) out.push(...was)
    else {
      // each property that is as it was keeps its line
      const used = new Set<number>()
      const lines = s.props.map(([k, v]) => {
        const j = src ? src.props.findIndex(([k0, v0], i) => !used.has(i) && k0 === k && v0 === v) : -1
        if (j >= 0 && was) {
          used.add(j)
          return was[j + 1]
        }
        return nl(renderProp(indent, k, v))
      })
      out.push(was?.[0] ?? nl(`${indent}:PROPERTIES:`), ...lines, was?.at(-1) ?? nl(`${indent}:END:`))
    }
  }
  out.push(...s.body)
  return out
}

/** The file again: what wasn't changed exactly as it was read. */
export function printOrg(doc: OrgDoc): string {
  const lines = [...doc.head, ...doc.sections.flatMap((s) => printSection(s, doc.cr))]
  return lines.join("\n") + (doc.final && lines.length ? "\n" : "")
}

/** A line to add to the file, with its line ending's \r when the file uses CRLF. */
export const lineOf = (doc: Pick<OrgDoc, "cr">, text: string) => (doc.cr ? `${text}\r` : text)

/** A #+KEY: line's value in the file's head (TITLE, FILETAGS), the last one when there are several. */
export function headValue(head: readonly string[], key: string): string | undefined {
  const re = new RegExp(`^#\\+${key}:[ \\t]*(.*?)[ \\t]*$`, "i")
  let v: string | undefined
  for (const l of head) {
    const m = re.exec(bare(l))
    if (m) v = m[1]
  }
  return v
}

/** The keyword sequences the file declares with #+TODO:, #+SEQ_TODO: or #+TYP_TODO:, each as its words before and
 *  after the bar. Fast-access keys and logging marks ("DONE(d!)") are dropped. */
export function fileSequences(head: readonly string[]): { open: string[]; done: string[] }[] {
  const out: { open: string[]; done: string[] }[] = []
  for (const l of head) {
    const m = /^#\+(?:SEQ_|TYP_)?TODO:[ \t]*(.*)$/i.exec(bare(l))
    if (!m) continue
    const words = m[1].trim().split(/[ \t]+/).filter(Boolean).map((w) => w.replace(/\(.*\)$/, ""))
    const bar = words.indexOf("|")
    if (bar >= 0) out.push({ open: words.slice(0, bar), done: words.slice(bar + 1).filter((w) => w !== "|") })
    else if (words.length) out.push({ open: words.slice(0, -1), done: words.slice(-1) })
  }
  return out
}

export interface Check {
  /** the line in the entry's body */
  line: number
  done: boolean
  /** [-]: some of its own sub-items are done */
  partial: boolean
  text: string
  indent: number
}

const CHECK = /^([ \t]*)(?:[-+*]|\d+[.)])[ \t]+\[([ xX-])\](?:[ \t]+(.*))?$/

/** The checkbox items in an entry's body: "- [ ] what", "- [X] what", "1. [-] what". */
export function checksOf(body: readonly string[]): Check[] {
  return body.flatMap((l, line) => {
    const m = CHECK.exec(bare(l))
    return m ? [{ line, done: m[2] === "X" || m[2] === "x", partial: m[2] === "-", text: (m[3] ?? "").trim(), indent: m[1].replace(/\t/g, "        ").length }] : []
  })
}

/** The body line with its checkbox ticked or cleared. */
export const setCheck = (line: string, done: boolean) => line.replace(/\[[ xX-]\]/, done ? "[X]" : "[ ]")

export interface Link {
  /** as written inside the brackets: item:i-abc, id:t-1, https://... */
  target: string
  desc?: string
  /** its type before the colon (item, id, comment, agent, file, https), or "fuzzy" for a plain [[name]] */
  type: string
  /** what follows the type */
  path: string
}

const LINK = /\[\[([^[\]]+)\](?:\[([^[\]]*)\])?\]/g

/** The links in a line of text: [[target]] and [[target][description]]. */
export function linksIn(text: string): Link[] {
  return [...text.matchAll(LINK)].map((m) => {
    const t = /^([a-z][\w+-]*):(.*)$/i.exec(m[1])
    return { target: m[1], ...(m[2] !== undefined && { desc: m[2] }), type: t ? t[1].toLowerCase() : "fuzzy", path: t ? t[2] : m[1] }
  })
}

/** Text with each link as its description (or its path), as a reader sees it. */
export const linkText = (text: string) => text.replace(LINK, (_, target: string, desc?: string) => desc ?? target.replace(/^[a-z][\w+-]*:/i, ""))

/** A progress cookie in a headline: [2/5] or [40%]. */
export const COOKIE = /\[(\d*)\/(\d*)\]|\[(\d*)%\]/
