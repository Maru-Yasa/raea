// PL3: every change by task id, the rest of the file untouched (src/edit.ts), and the plan as values (src/model.ts).
import { describe, expect, test } from "vitest"
import { applyEdits, createRegistry, defineKeywords, find, newPlan, planDigest, readPlan, toggled, type PlanEdit } from "../src"

const NOW = new Date(2026, 10, 14, 16, 40)
let n = 0
const ids = () => `t-${++n}`
const apply = (text: string, edits: PlanEdit[]) => applyEdits(text, edits, { now: NOW, newId: ids })

const PLAN = `#+TITLE: Ides
#+TODO: TODO MAKING REVIEW | DONE DROPPED

* Act one
** TODO Rome storyboard [0/2]
:PROPERTIES:
:ID:       t-rome
:END:
Notes a person wrote, kept.
*** TODO The forum
:PROPERTIES:
:ID:       t-forum
:END:
*** TODO The steps
:PROPERTIES:
:ID:       t-steps
:END:
** DONE Cast the narrator
CLOSED: [2026-11-02 Mon 10:14]
:PROPERTIES:
:ID:       t-cast
:END:
* Sound
** TODO Foley for the fire
`

describe("PL2: the plan as values", () => {
  const p = readPlan(PLAN)
  test("tasks and the headings that group them; states from the keywords; stages, parents and progress", () => {
    expect(p.title).toBe("Ides")
    expect(p.entries.map((t) => [t.ref, t.keyword ?? "", t.state ?? "", t.stage])).toEqual([
      ["#0", "", "", ""],
      ["t-rome", "TODO", "todo", "Act one"],
      ["t-forum", "TODO", "todo", "Act one"],
      ["t-steps", "TODO", "todo", "Act one"],
      ["t-cast", "DONE", "done", "Act one"],
      ["#5", "", "", ""],
      ["#6", "TODO", "todo", "Sound"],
    ])
    expect(find(p, "t-rome")!.children).toEqual(["t-forum", "t-steps"])
    expect(find(p, "t-rome")!.progress).toEqual({ done: 0, total: 2 })
    expect(find(p, "t-rome")!.text).toBe("Rome storyboard")
    expect(find(p, "t-rome")!.line).toBe(4)
    expect(find(p, "t-cast")!.closed?.date).toBe("2026-11-02")
  })
  test("a file's own keywords are read with their states, Org's way", () => {
    const own = readPlan("#+TODO: OPEN NEXT | SHIPPED KILLED\n* NEXT a\n* KILLED b\n* REVIEW c\n")
    expect(own.tasks.map((t) => [t.keyword, t.state])).toEqual([
      ["NEXT", "making"],
      ["KILLED", "dropped"],
      ["REVIEW", "review"],
    ])
  })
})

describe("PL3: edits by id", () => {
  test("closing a subtask stamps CLOSED, links what was made and counts the cookie; nothing else moves", () => {
    const r = apply(PLAN, [{ op: "state", ref: "t-forum", state: "done", item: "i-forum-1", note: "Made the forum still." }])
    expect(r.refs).toEqual(["t-forum"])
    expect(r.text).toContain("** TODO Rome storyboard [1/2]\n")
    expect(r.text).toContain("*** DONE The forum\nCLOSED: [2026-11-14 Sat 16:40]\n:PROPERTIES:\n:ID:       t-forum\n:ITEM:     i-forum-1\n:END:\nMade the forum still.\n")
    // every line no edit touched is as it was
    const kept = PLAN.split("\n").filter((l) => !/Rome storyboard|The forum/.test(l))
    for (const l of kept) expect(r.text.split("\n")).toContain(l)
    // and opened again: CLOSED comes off
    const back = apply(r.text, [toggled(find(r.plan, "t-forum")!)])
    expect(back.text).toContain("*** TODO The forum\n:PROPERTIES:")
    expect(back.text).not.toContain("CLOSED: [2026-11-14")
  })
  test("add: under a heading by its words (made when missing), with priority, tags, deadline, agent and checkboxes", () => {
    n = 0
    const r = apply(PLAN, [
      { op: "add", title: "Senate theme, final mix", under: "Sound", priority: "a", tags: ["trailer", "final-mix"], deadline: "11-20", agents: ["a-score"], from: "m-1", checks: ["stems", "master"] },
      { op: "add", title: "Colour the titles", under: "Look" },
    ])
    expect(r.refs).toEqual(["t-1", "t-2"])
    const head = r.text.split("\n").find((l) => l.includes("Senate theme"))!
    expect(head).toMatch(/^\*\* TODO \[#A\] Senate theme, final mix +:trailer:final_mix:$/)
    expect(head.length).toBe(77)
    expect(r.text).toContain(`${head}\nDEADLINE: <2026-11-20 Fri>\n:PROPERTIES:\n:ID:       t-1\n:AGENT:    a-score\n:FROM:     m-1\n:END:\n- [ ] stems\n- [ ] master\n`)
    expect(r.text.endsWith("* Look\n** TODO Colour the titles\n:PROPERTIES:\n:ID:       t-2\n:END:\n")).toBe(true)
    // the task with no id got one on this write (t-3), so every task can be named from now on
    expect(find(r.plan, "t-3")?.text).toBe("Foley for the fire")
    expect(find(r.plan, "t-1")).toMatchObject({ priority: "A", tags: ["trailer", "final_mix"], agents: ["a-score"], from: "m-1", stage: "Sound", progress: { done: 0, total: 2 } })
  })
  test("#N names a headline with no id as the plan was before the edits, even after an add moved it", () => {
    const r = apply(PLAN, [
      { op: "add", title: "First", before: "t-rome" },
      { op: "set", ref: "#6", title: "Foley for the fire and the crowd" },
    ])
    expect(r.text).toContain("** TODO Foley for the fire and the crowd\n")
  })
  test("move: a subtree goes before another task, taking its level; under a heading; never into itself", () => {
    const r = apply(PLAN, [{ op: "move", ref: "t-rome", before: "#6" }])
    expect(readPlan(r.text).entries.map((t) => `${t.level}${t.text}`)).toEqual(["1Act one", "2Cast the narrator", "1Sound", "2Rome storyboard", "3The forum", "3The steps", "2Foley for the fire"])
    const up = apply(PLAN, [{ op: "move", ref: "t-forum", under: "Sound" }])
    expect(find(up.plan, "t-forum")).toMatchObject({ level: 2, stage: "Sound" })
    expect(find(up.plan, "t-rome")!.title).toBe("Rome storyboard [0/1]")
    expect(() => apply(PLAN, [{ op: "move", ref: "t-rome", after: "t-steps" }])).toThrow(/into itself/)
  })
  test("set and remove; a checkbox ticked and one added; what's wrong is said and nothing is written", () => {
    const r = apply(PLAN, [
      { op: "set", ref: "t-steps", deadline: "2026-11-21", tags: ["gate"], props: { SHOT: "12" } },
      { op: "remove", ref: "t-cast" },
    ])
    expect(r.text).toContain("*** TODO The steps")
    expect(find(r.plan, "t-steps")).toMatchObject({ deadline: { date: "2026-11-21" }, tags: ["gate"], props: { ID: "t-steps", SHOT: "12" } })
    expect(find(r.plan, "t-cast")).toBeUndefined()
    const c = apply("* TODO Grade [0%]\n:PROPERTIES:\n:ID: g\n:END:\n- [ ] warm\n- [ ] cool\n", [{ op: "check", ref: "g", index: 1 }, { op: "check", ref: "g", add: "contrast" }])
    expect(c.text).toBe("* TODO Grade [33%]\n:PROPERTIES:\n:ID: g\n:END:\n- [ ] warm\n- [X] cool\n- [ ] contrast\n")
    expect(() => apply(PLAN, [{ op: "state", ref: "t-nope", state: "done" }])).toThrow(/no task t-nope/)
    expect(() => apply(PLAN, [{ op: "state", ref: "t-rome", state: "finished" }])).toThrow(/neither a keyword nor a state/)
    expect(() => apply(PLAN, [{ op: "set", ref: "t-rome", deadline: "someday" }])).toThrow(/YYYY-MM-DD/)
  })
  test("closing one task in a file kept by hand changes that task's lines and nothing else", () => {
    const mine = `#+TITLE:   Kept in Emacs
#+STARTUP: content
#+OPTIONS: toc:nil

* Notes                                                              :notes:
#+BEGIN_SRC typescript :tangle yes
const x = { a: 1 }
* a headline-looking line is one in Org too, so it is one here
#+END_SRC
| col | other |
|-----+-------|
| 1   | 2     |
#+TBLFM: $2=$1*2
- plain list
  - nested
* Work
**   TODO   [#B]   Light   the door     :look:
  SCHEDULED:   <2026-11-20 Fri>    DEADLINE: <2026-11-22 Sun>
   :PROPERTIES:
   :ID:      t-door
   :Effort:   1:00
   :END:
   :LOGBOOK:
   CLOCK: [2026-11-14 Sat 10:00]--[2026-11-14 Sat 11:00] =>  1:00
   :END:
** TODO Score it
:PROPERTIES:
:ID: t-score
:END:
`
    const r = apply(mine, [{ op: "state", ref: "t-door", state: "done" }])
    const before = mine.split("\n")
    const after = r.text.split("\n")
    expect(after.filter((l) => !before.includes(l))).toEqual(["**   DONE   [#B]   Light   the door     :look:", "  CLOSED: [2026-11-14 Sat 16:40] SCHEDULED: <2026-11-20 Fri> DEADLINE: <2026-11-22 Sun>"])
    expect(before.filter((l) => !after.includes(l))).toEqual(["**   TODO   [#B]   Light   the door     :look:", "  SCHEDULED:   <2026-11-20 Fri>    DEADLINE: <2026-11-22 Sun>"])
    expect(after.length).toBe(before.length)
  })
  test("under a task's id that isn't there is a mistake, not a stage to make", () => {
    expect(() => apply(PLAN, [{ op: "add", title: "x", under: "t-gone" }])).toThrow(/no task t-gone/)
    expect(() => apply(PLAN, [{ op: "move", ref: "t-forum", under: "#99" }])).toThrow(/no task #99/)
  })
  test("a plugin's keyword written into a file that doesn't name it gets a #+TODO: line, so Emacs cycles it", () => {
    const reg = createRegistry()
    defineKeywords({ name: "shots", words: "SHOT LIT | APPROVED" }, reg)
    const r = applyEdits(PLAN, [{ op: "add", title: "Shot 12", state: "SHOT" }], { now: NOW, registry: reg })
    expect(r.text.startsWith("#+TITLE: Ides\n#+TODO: TODO MAKING REVIEW | DONE DROPPED\n#+TODO: SHOT LIT | APPROVED\n\n")).toBe(true)
    // raea's own words in a file with no #+TODO: line: Org's TODO and DONE need none, MAKING does
    expect(applyEdits("* TODO a\n:PROPERTIES:\n:ID: a\n:END:\n", [{ op: "state", ref: "a", state: "done" }], { now: NOW }).text.startsWith("* DONE a")).toBe(true)
    expect(applyEdits("#+TITLE: x\n* TODO a\n:PROPERTIES:\n:ID: a\n:END:\n", [{ op: "state", ref: "a", state: "making" }], { now: NOW }).text.startsWith("#+TITLE: x\n#+TODO: TODO MAKING REVIEW | DONE DROPPED\n* MAKING a")).toBe(true)
  })
  test("a new plan is raea's keywords spelled out; the digest an agent reads lists open tasks by id", () => {
    const r = apply(newPlan("Pong"), [{ op: "add", title: "Tune the paddle", under: "Feel" }, { op: "add", title: "Old idea", state: "dropped" }])
    expect(r.text.startsWith("#+TITLE: Pong\n#+TODO: TODO MAKING REVIEW | DONE DROPPED\n\n* Feel\n** TODO Tune the paddle\n")).toBe(true)
    expect(planDigest(r.plan)).toMatch(/^Feel:\n {2}- t-\d+ {2}TODO {2}Tune the paddle\n1 closed \(done or dropped\)\.$/)
  })
})
