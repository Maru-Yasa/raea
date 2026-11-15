// PL6: a plugin's keywords, properties, kinds and views (src/extend.ts, src/views.ts).
import { describe, expect, test } from "vitest"
import { applyEdits, checkPlan, createRegistry, defineKeywords, definePlanKind, defineProperty, defineView, readPlan, view, type Task } from "../src"

describe("PL6: extending plans", () => {
  test("a keyword sequence a plugin defines is read in any plan, with the states it gives", () => {
    const reg = createRegistry()
    defineKeywords({ name: "film", words: "SHOOT CUT GRADE | DELIVERED", states: { GRADE: "review" } }, reg)
    const p = readPlan("* SHOOT the steps\n* GRADE the steps\n* DELIVERED the trailer\n", reg)
    expect(p.tasks.map((t) => [t.keyword, t.state])).toEqual([
      ["SHOOT", "todo"],
      ["GRADE", "review"],
      ["DELIVERED", "done"],
    ])
    // the default registry doesn't know them: a plain heading
    expect(readPlan("* SHOOT the steps\n").tasks).toEqual([])
    expect(() => defineKeywords({ name: "bad", words: "to do | done" }, reg)).toThrow(/capitals/)
  })

  test("a kind names its sequence, the properties it needs and a check of its own; checkPlan says what's wrong", () => {
    const reg = createRegistry()
    defineKeywords({ name: "film", words: "SHOOT CUT | DELIVERED" }, reg)
    defineProperty({ key: "shot", about: "the shot's number", check: (v) => (/^\d+$/.test(v) ? undefined : "a shot is a number") }, reg)
    definePlanKind({ name: "shot", keywords: "film", props: ["SHOT"], check: (t: Task) => (t.deadline ? undefined : "a shot needs a deadline"), defaults: { tags: ["shot"] } }, reg)
    const r = applyEdits("", [{ op: "add", id: "s1", title: "The steps, wide", kind: "shot", state: "SHOOT", props: { SHOT: "x" } }, { op: "add", id: "s2", title: "No kind", kind: "nope" }], { registry: reg })
    expect(r.plan.tasks[0]).toMatchObject({ kind: "shot", keyword: "SHOOT", tags: ["shot"] })
    // with no state, a task of a kind starts at its sequence's first word
    const first = reg.sequences.get("film")!.words[0].word
    expect(applyEdits("", [{ op: "add", title: "Another", kind: "shot" }], { registry: reg }).plan.tasks[0].keyword).toBe(first)
    expect(checkPlan(r.plan, reg).map((d) => `${d.ref}: ${d.message}`)).toEqual(["s1: :SHOT: a shot is a number", "s1: a shot needs a deadline", "s2: no kind nope is defined (shot)"])
    const dup = readPlan("* TODO a\n:PROPERTIES:\n:ID: x\n:END:\n* TODO b\n:PROPERTIES:\n:ID: x\n:END:\n", reg)
    expect(checkPlan(dup, reg)).toMatchObject([{ ref: "x", severity: "error", line: 4 }])
    expect(() => definePlanKind({ name: "take", keywords: "nope" }, reg)).toThrow(/defineKeywords it first/)
  })

  test("views: raea's board and agenda, and one a plugin adds", () => {
    const reg = createRegistry()
    const p = readPlan(
      "* TODO a\nDEADLINE: <2026-11-13 Fri>\n* MAKING b\nSCHEDULED: <2026-11-15 Sun>\n* REVIEW c\n* DONE d\nDEADLINE: <2026-11-14 Sat>\n* DROPPED e\n* TODO f :x:\n:PROPERTIES:\n:AGENT: a-grade\n:END:\n",
      reg,
    )
    const lanes = view<{ state: string; tasks: Task[] }[]>(p, "board", undefined, reg)
    expect(lanes.map((l) => `${l.state}:${l.tasks.map((t) => t.text).join("")}`)).toEqual(["todo:af", "making:b", "review:c", "done:d"])
    const a = view<{ overdue: { task: Task }[]; days: { date: string; entries: { task: Task; why: string }[] }[] }, { from: string; days: number }>(p, "agenda", { from: "2026-11-14", days: 3 }, reg)
    expect(a.overdue.map((e) => e.task.text)).toEqual(["a"])
    expect(a.days.map((d) => `${d.date}:${d.entries.map((e) => `${e.task.text}/${e.why}`).join(",")}`)).toEqual(["2026-11-14:", "2026-11-15:b/scheduled", "2026-11-16:"])
    defineView({ name: "by-agent", build: (plan) => Object.groupBy(plan.tasks.filter((t) => t.agents.length), (t) => t.agents[0]) }, reg)
    expect(Object.keys(view<Record<string, Task[]>>(p, "by-agent", undefined, reg))).toEqual(["a-grade"])
    expect(() => view(p, "nope", undefined, reg)).toThrow(/no plan view nope: board, agenda, outline, by-agent/)
  })
})
