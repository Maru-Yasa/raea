// PL5: a plan's tools for an agent (src/tools.ts), answered on a plan in memory the way the app and a headless run do.
import { describe, expect, test } from "vitest"
import { answerPlanTool, applyEdits, find, readPlan, planToolEdits, type PlanToolHost } from "../src"

const NOW = new Date(2026, 10, 14, 16, 40)
const START = `#+TITLE: Stay
#+TODO: TODO MAKING REVIEW | DONE DROPPED

* Look
** TODO Light the door
:PROPERTIES:
:ID:       t-door
:END:
- [ ] warm key
- [ ] cool fill
`

/** A host on a plan in memory: the page's and the headless feed's, minus where the text is kept. */
function host(text = START): PlanToolHost & { now: () => string } {
  let n = 0
  let at = text
  return {
    who: "a-light",
    text: () => at,
    apply: (edits) => {
      const r = applyEdits(at, edits, { now: NOW, newId: () => `t-new${++n}` })
      at = r.text
      return r
    },
    now: () => at,
  }
}

describe("an agent's plan tools", () => {
  test("plan_read gives the open tasks by id, and the whole file with full", async () => {
    const h = host()
    const r = (await answerPlanTool("plan_read", {}, h)) as { file: string; open: string; text?: string }
    expect(r.file).toBe("plan.org")
    expect(r.open).toContain("t-door  TODO  Light the door")
    expect(r.text).toBeUndefined()
    expect(((await answerPlanTool("plan_read", { full: true }, h)) as { text: string }).text).toBe(START)
  })

  test("plan_add writes tasks under their stage, held by the agent that asked unless it names others", async () => {
    const h = host()
    const r = (await answerPlanTool("plan_add", { tasks: [{ title: "Grade the hall", under: "Look", priority: "A", deadline: "2026-11-20", tags: ["look"] }, { title: "Score it", agents: ["a-score"] }] }, h)) as { ids: string[] }
    expect(r.ids).toEqual(["t-new1", "t-new2"])
    const plan = readPlan(h.now())
    expect(find(plan, "t-new1")).toMatchObject({ stage: "Look", level: 2, priority: "A", agents: ["a-light"], tags: ["look"], deadline: { date: "2026-11-20" } })
    expect(find(plan, "t-new2")).toMatchObject({ level: 1, agents: ["a-score"] })
    expect(h.now()).toContain("** TODO [#A] Grade the hall")
  })

  test("plan_update marks, ticks, adds a checkbox, moves and sets fields by id", async () => {
    const h = host()
    await answerPlanTool("plan_update", { changes: [{ id: "t-door", state: "making", check: 0, add_check: "rim light", item: "i-door-2" }] }, h)
    const t = find(readPlan(h.now()), "t-door")!
    expect([t.keyword, t.item, t.checks.map((c) => `${c.done ? "x" : " "} ${c.text}`)]).toEqual(["MAKING", "i-door-2", ["x warm key", "  cool fill", "  rim light"]])
    expect(t.progress).toEqual({ done: 1, total: 3 })
  })

  test("plan_close closes with CLOSED, the item it made and a note; dropped keeps the task", async () => {
    const h = host()
    await answerPlanTool("plan_close", { id: "t-door", item: "i-door-3", note: "Warm key at 3200K." }, h)
    expect(h.now()).toContain("** DONE Light the door\nCLOSED: [2026-11-14 Sat 16:40]")
    expect(h.now()).toContain(":ITEM:     i-door-3")
    expect(h.now()).toContain("Warm key at 3200K.")
    const d = host()
    await answerPlanTool("plan_close", { id: "t-door", state: "dropped" }, d)
    expect(find(readPlan(d.now()), "t-door")!.state).toBe("dropped")
  })

  test("a wrong input or a task that isn't there is said, and nothing is written", async () => {
    const h = host()
    expect(() => planToolEdits("plan_add", { tasks: [] }, "a")).toThrow(/at least one/)
    expect(() => planToolEdits("plan_update", { changes: [{ id: "t-door" }] }, "a")).toThrow(/nothing to change/)
    expect(() => planToolEdits("plan_close", { id: "t-door", state: "making" }, "a")).toThrow(/done or dropped/)
    await expect(answerPlanTool("plan_close", { id: "t-door", state: "MAKING" }, h)).rejects.toThrow(/MAKING doesn't close/)
    await expect(answerPlanTool("plan_close", { id: "t-gone" }, h)).rejects.toThrow(/no task t-gone/)
    await expect(answerPlanTool("plan_add", { tasks: [{ title: "x", under: "t-gone" }] }, h)).rejects.toThrow(/no task t-gone/)
    expect(h.now()).toBe(START)
  })

  test("plan_read gives a task written by hand without an id one first, so the agent never names a task by its place", async () => {
    const h = host(`${START}** TODO Written in Emacs\n`)
    const r = (await answerPlanTool("plan_read", {}, h)) as { open: string }
    expect(r.open).toContain("t-new1  TODO  Written in Emacs")
    expect(r.open).not.toMatch(/#\d/)
    expect(h.now()).toContain("** TODO Written in Emacs\n:PROPERTIES:\n:ID:       t-new1\n:END:\n")
  })
})
