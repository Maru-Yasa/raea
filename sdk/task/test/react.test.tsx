// PL6: the React side (src/react.tsx): a plan read from a source that changes, and the components drawn as plain
// elements. Run by the app's vitest, which has react-dom (apps/app vite.config.ts test.include).
import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, test } from "vitest"
import { readPlan } from "../src"
import { Agenda, BoardLanes, memorySource, PlanOutline, TaskLine, toggled, usePlan } from "../src/react"

const TEXT = "* Act one\n** TODO [#A] Rome storyboard [0/1]\n:PROPERTIES:\n:ID: t-rome\n:END:\n*** TODO The forum\nDEADLINE: <2026-11-14 Sat>\n:PROPERTIES:\n:ID: t-forum\n:END:\n** DONE Cast\n"

describe("PL6: React", () => {
  test("TaskLine: a checkbox, the words, priority, progress, deadline; a heading has no checkbox", () => {
    const p = readPlan(TEXT)
    const html = renderToStaticMarkup(<TaskLine task={p.entries[1]} onToggle={() => {}} />)
    expect(html).toContain('data-task="t-rome"')
    expect(html).toContain('role="checkbox" aria-checked="false"')
    expect(html).toContain('<span data-part="priority">A</span>')
    expect(html).toContain('<span data-part="text">Rome storyboard</span>')
    expect(html).toContain('<span data-part="progress">0/1</span>')
    expect(renderToStaticMarkup(<TaskLine task={p.entries[0]} />)).not.toContain("checkbox")
  })

  test("the outline nests, the board has its lanes, the agenda its days", () => {
    const p = readPlan(TEXT)
    const outline = renderToStaticMarkup(<PlanOutline plan={p} />)
    expect(outline.indexOf("The forum")).toBeGreaterThan(outline.indexOf("Rome storyboard"))
    expect(outline.match(/<ul/g)!.length).toBe(3)
    const board = renderToStaticMarkup(<BoardLanes plan={p} labels={{ todo: "Next" }} />)
    expect(board).toMatch(/data-state="todo"><h3>Next <span data-part="count">2<\/span>/)
    expect(board).toMatch(/data-state="done"><h3>Done <span data-part="count">1<\/span>/)
    const agenda = renderToStaticMarkup(<Agenda plan={p} from="2026-11-14" days={2} />)
    expect(agenda).toMatch(/data-date="2026-11-14"><h3>2026-11-14<\/h3><ul><li data-why="deadline">.*The forum/)
  })

  test("usePlan reads a source and edits it by id; a source of text alone is read-only", () => {
    const src = memorySource(TEXT)
    let seen: ReturnType<typeof usePlan> | undefined
    const Probe = () => {
      seen = usePlan(src)
      return null
    }
    renderToStaticMarkup(<Probe />)
    expect(seen!.plan.tasks.length).toBe(3)
    seen!.edit([toggled(seen!.plan.tasks[1])])
    expect(readPlan(src.read()!).tasks[1].state).toBe("done")
    expect(src.read()).toContain("** TODO [#A] Rome storyboard [1/1]")
    renderToStaticMarkup(<Probe />)
    expect(seen!.plan.tasks[1].state).toBe("done")
    const Fixed = () => {
      seen = usePlan(TEXT)
      return null
    }
    renderToStaticMarkup(<Fixed />)
    expect(() => seen!.edit([])).toThrow(/read-only/)
  })
})
