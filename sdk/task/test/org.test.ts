// PL2: the plan.org read and written without losing a byte (src/org.ts, src/time.ts).
import { describe, expect, test } from "vitest"
import { checksOf, dayOf, formatStamp, linksIn, linkText, parseOrg, parseStamp, printOrg, readHeadline } from "../src"

const KW = new Set(["TODO", "MAKING", "REVIEW", "DONE", "DROPPED"])

const GNARLY = `#+TITLE: The Ides of March
#+TODO: TODO MAKING REVIEW | DONE DROPPED
#+STARTUP: overview
Some words before any headline, kept as they are.

* Act one                                                            :act_one:
** DONE [#A] Cast the narrator                                         :score:
   CLOSED: [2026-11-02 Mon 10:14] DEADLINE: <2026-11-01 Sun>
   :PROPERTIES:
   :ID:       t-cast
   :AGENT:    a-score
   :CUSTOM_thing: kept   as   written
   :END:
   :LOGBOOK:
   - State "DONE"       from "TODO"       [2026-11-02 Mon 10:14]
   :END:
** TODO Rome storyboard [1/3]
SCHEDULED: <2026-11-20 Fri 10:00-11:00 +1w -2d>
- [X] the forum
- [ ] the steps
  - [x] a nested one
- [-] the senate
| a | table |
|---+-------|
#+BEGIN_SRC tsx
* not a headline inside a block? Org says it is one, and so do we
#+END_SRC
*bold* at the start of a line is no headline
* WAITING an unknown keyword is part of the title
*** Deep, skipping a level
:PROPERTIES:
:ID: unterminated drawer, so this is body
* COMMENT a commented heading
* Last line without a newline`

describe("PL2a: a round trip changes nothing", () => {
  test("a file with drawers, logbooks, tables, blocks, odd spacing and no final newline prints back byte for byte", () => {
    expect(printOrg(parseOrg(GNARLY, KW))).toBe(GNARLY)
    expect(printOrg(parseOrg(`${GNARLY}\n`, KW))).toBe(`${GNARLY}\n`)
  })
  test("CRLF files, empty files, a file of only a head, trailing blank lines", () => {
    for (const t of ["", "\n", "\n\n", "#+TITLE: x\n", "* TODO a\r\n** DONE b\r\n   :PROPERTIES:\r\n   :ID: x\r\n   :END:\r\n", "* a\n\n\n", "text\n* h\nbody\n\n"]) expect(printOrg(parseOrg(t, KW))).toBe(t)
  })
})

describe("PL2b: what a headline says", () => {
  test("stars, keyword, priority, title and tags", () => {
    expect(readHeadline("** DONE [#A] Cast the narrator      :score:x@y:", KW)).toEqual({ level: 2, keyword: "DONE", priority: "A", title: "Cast the narrator", tags: ["score", "x@y"] })
    expect(readHeadline("* TODO", KW)).toEqual({ level: 1, keyword: "TODO", title: "", tags: [] })
    expect(readHeadline("* WAITING on the cut", KW)).toEqual({ level: 1, title: "WAITING on the cut", tags: [] })
    expect(readHeadline("*bold*", KW)).toBeNull()
    expect(readHeadline("* Tags need a space before them:no:", KW)!.tags).toEqual([])
  })
  test("planning, drawer and body are told apart; an unterminated drawer is body", () => {
    const d = parseOrg(GNARLY, KW)
    const cast = d.sections[1]
    expect(cast.planning.closed).toEqual({ active: false, date: "2026-11-02", time: "10:14" })
    expect(cast.planning.deadline).toEqual({ active: true, date: "2026-11-01" })
    expect(cast.props).toEqual([["ID", "t-cast"], ["AGENT", "a-score"], ["CUSTOM_thing", "kept   as   written"]])
    expect(cast.body[0]).toBe("   :LOGBOOK:")
    const rome = d.sections[2]
    expect(rome.planning.scheduled).toEqual({ active: true, date: "2026-11-20", time: "10:00", end: "11:00", repeat: "+1w", warn: "-2d" })
    expect(checksOf(rome.body).map((c) => [c.done, c.partial, c.indent, c.text])).toEqual([
      [true, false, 0, "the forum"],
      [false, false, 0, "the steps"],
      [true, false, 2, "a nested one"],
      [false, true, 0, "the senate"],
    ])
    const deep = d.sections.find((s) => s.title.startsWith("Deep"))!
    expect(deep.props).toBeUndefined()
    expect(deep.body).toEqual([":PROPERTIES:", ":ID: unterminated drawer, so this is body"])
  })
})

describe("PL2c: a changed entry is written again part by part", () => {
  test("a new keyword rewrites the headline, keeps the planning line and drawer as they were", () => {
    const d = parseOrg(GNARLY, KW)
    d.sections[2].keyword = "MAKING"
    const out = printOrg(d)
    expect(out).toContain("** MAKING Rome storyboard [1/3]\nSCHEDULED: <2026-11-20 Fri 10:00-11:00 +1w -2d>\n- [X] the forum")
    expect(out.replace("MAKING Rome", "TODO Rome")).toBe(GNARLY)
  })
  test("a property added keeps every other property's line, indentation and all", () => {
    const d = parseOrg(GNARLY, KW)
    d.sections[1].props = [...d.sections[1].props!, ["ITEM", "i-cast"]]
    const out = printOrg(d)
    expect(out).toContain("   :CUSTOM_thing: kept   as   written\n   :ITEM:     i-cast\n   :END:")
    expect(out.split("\n").length).toBe(GNARLY.split("\n").length + 1)
  })
  test("tags end at column 77; a new drawer and planning line are written Org's way", () => {
    const d = parseOrg("* TODO Grade it\n", KW)
    d.sections[0].tags = ["trailer"]
    d.sections[0].planning = { deadline: { active: true, date: "2026-11-03" } }
    d.sections[0].props = [["ID", "t-1"]]
    const out = printOrg(d).split("\n")
    expect(out[0].length).toBe(77)
    expect(out[0]).toMatch(/^\* TODO Grade it +:trailer:$/)
    expect(out.slice(1, 5)).toEqual(["DEADLINE: <2026-11-03 Tue>", ":PROPERTIES:", ":ID:       t-1", ":END:"])
  })
  test("a headline whose keyword or words change keeps its own spacing, and its tags end where they ended", () => {
    const d = parseOrg("*   TODO   [#B]   Foo   bar     :a:b:\n** NEXT\tTabbed\n", new Set([...KW, "NEXT"]))
    d.sections[0].keyword = "DONE"
    d.sections[0].title = "Foo   bar [1/2]"
    d.sections[1].keyword = undefined
    expect(printOrg(d)).toBe("*   DONE   [#B]   Foo   bar [1/2] :a:b:\n** Tabbed\n")
    // tags aligned to a column stay at it
    const aligned = parseOrg("** TODO Light the door                                                   :look:\n", KW)
    aligned.sections[0].keyword = "MAKING"
    expect(printOrg(aligned)).toBe("** MAKING Light the door                                                 :look:\n")
  })
  test("a planning line raea can't wholly read (a range of days, a habit) is still the planning line", () => {
    const text = "* TODO Shoot the steps\n  SCHEDULED: <2026-11-20 Fri>--<2026-11-22 Sun> DEADLINE: <2026-11-23 Mon .+2d/3d>\n  :PROPERTIES:\n  :ID:       t-steps\n  :END:\n"
    const d = parseOrg(text, KW)
    expect(d.sections[0].props).toEqual([["ID", "t-steps"]])
    expect(d.sections[0].planning).toEqual({ deadline: { active: true, date: "2026-11-23", repeat: ".+2d/3d" }, rest: ["SCHEDULED: <2026-11-20 Fri>--<2026-11-22 Sun>"] })
    expect(printOrg(d)).toBe(text)
    // closed: CLOSED comes first, and the rest is written as it was, in its order
    d.sections[0].keyword = "DONE"
    d.sections[0].planning = { ...d.sections[0].planning, closed: { active: false, date: "2026-11-14", time: "16:40" } }
    expect(printOrg(d).split("\n")[1]).toBe("  CLOSED: [2026-11-14 Sat 16:40] DEADLINE: <2026-11-23 Mon .+2d/3d> SCHEDULED: <2026-11-20 Fri>--<2026-11-22 Sun>")
  })
  test("a CRLF file's new lines end in CRLF", () => {
    const d = parseOrg("* TODO a\r\n", KW)
    d.sections[0].props = [["ID", "x"]]
    expect(printOrg(d)).toBe("* TODO a\r\n:PROPERTIES:\r\n:ID:       x\r\n:END:\r\n")
  })
})

describe("timestamps and links", () => {
  test("read and written back; a day as a person writes it", () => {
    for (const s of ["<2026-11-14 Sat>", "[2026-11-14 Sat 16:40]", "<2026-11-14 Sat 10:00-11:30 .+1d --3d>"]) expect(formatStamp(parseStamp(s)!)).toBe(s)
    expect(parseStamp("<2026-11-14 Sat]")).toBeUndefined()
    expect(dayOf("11-03", new Date(2026, 0, 1))).toEqual({ active: true, date: "2026-11-03" })
    expect(dayOf("2026-02-30")).toBeUndefined()
    expect(dayOf("soon")).toBeUndefined()
  })
  test("links by type: a feed item, a task by id, a web page", () => {
    expect(linksIn("Made [[item:i-hero][the trailer]], see [[id:t-cast]] and [[https://raea.io]]").map((l) => [l.type, l.path, l.desc])).toEqual([
      ["item", "i-hero", "the trailer"],
      ["id", "t-cast", undefined],
      ["https", "//raea.io", undefined],
    ])
    expect(linkText("Grade [[item:i-hero][the trailer]] after [[id:t-1]]")).toBe("Grade the trailer after t-1")
  })
})
