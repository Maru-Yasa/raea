// A plan's tools for an agent (docs/plan.feature PL5): plan_read, plan_add, plan_update and plan_close, as the raea
// app offers them to every agent it launches. Any host answers them the same way: it says how to read the plan's text
// and how to make edits on it, and these turn a tool's input into plan edits (edit.ts) and its answer into the open
// tasks with their ids. The app's page answers them through its plans, a headless run on the folder's plan.org.
import { planDigest } from "./digest.ts"
import type { PlanEdit, TaskFields } from "./edit.ts"
import { isClosed, readPlan } from "./model.ts"

export const PLAN_TOOLS = ["plan_read", "plan_add", "plan_update", "plan_close"] as const
export type PlanTool = (typeof PLAN_TOOLS)[number]

/** A task as plan_add takes it. Left without agents, it's the calling agent's. */
export interface NewTaskInput extends Omit<TaskFields, "title"> {
  title: string
  state?: string
  under?: string
  before?: string
  after?: string
  checks?: string[]
}

/** One change as plan_update takes it: fields to set, and at most one of state, check, move or remove. */
export interface TaskChange extends TaskFields {
  id: string
  state?: string
  /** a checkbox under it, by its place (0 is the first): ticked, or as `done` says */
  check?: number
  done?: boolean
  /** a checkbox added under it */
  add_check?: string
  move?: { before?: string; after?: string; under?: string }
  remove?: boolean
}

export interface PlanToolHost {
  /** the calling agent's id: a new task with no agents is its */
  who: string
  /** the plan's text now */
  text(): string
  /** the edits made on the plan: its text after, and each edit's task id */
  apply(edits: PlanEdit[]): Promise<{ text: string; refs: string[] }> | { text: string; refs: string[] }
  /** where the plan is, for the agent to say or open */
  file?: string
}

const list = <T>(v: unknown, what: string): T[] => {
  if (!Array.isArray(v) || !v.length) throw new Error(`${what}: a list, with at least one`)
  return v as T[]
}

/** The fields of a change that set a task's own values (title, priority, tags, dates, item, agents, properties, note). */
function fieldsOf(c: TaskChange): TaskFields | undefined {
  const { id: _, state: _s, check: _c, done: _d, add_check: _a, move: _m, remove: _r, ...f } = c
  return Object.keys(f).length ? f : undefined
}

/** A tool's input as plan edits, in order. Throws, naming what's wrong, on an input that isn't one. */
export function planToolEdits(tool: Exclude<PlanTool, "plan_read">, input: Record<string, unknown>, who: string): PlanEdit[] {
  if (tool === "plan_add")
    return list<NewTaskInput>(input.tasks, "tasks").map((t) => {
      if (!t || typeof t.title !== "string" || !t.title.trim()) throw new Error("each task needs a title")
      return { ...t, op: "add" as const, agents: t.agents ?? [who] }
    })
  if (tool === "plan_close") {
    if (typeof input.id !== "string") throw new Error("id: the task to close (plan_read lists them)")
    const state = input.state === undefined ? "done" : String(input.state)
    if (!/^(done|dropped)$/i.test(state) && state !== state.toUpperCase()) throw new Error(`state: done or dropped (or a closing keyword of the plan's), not ${state}`)
    return [{ op: "state", ref: input.id, state, ...(typeof input.item === "string" && { item: input.item }), ...(typeof input.note === "string" && { note: input.note }) }]
  }
  return list<TaskChange>(input.changes, "changes").flatMap((c): PlanEdit[] => {
    if (!c || typeof c.id !== "string") throw new Error("each change names its task by id (plan_read lists them)")
    if (c.remove) return [{ op: "remove", ref: c.id }]
    const out: PlanEdit[] = []
    const f = fieldsOf(c)
    if (f) out.push({ op: "set", ref: c.id, ...f })
    if (c.state) out.push({ op: "state", ref: c.id, state: c.state })
    if (c.check !== undefined) out.push({ op: "check", ref: c.id, index: c.check, ...(c.done !== undefined && { done: c.done }) })
    if (c.add_check) out.push({ op: "check", ref: c.id, add: c.add_check })
    if (c.move) out.push({ op: "move", ref: c.id, ...c.move })
    if (!out.length) throw new Error(`nothing to change on ${c.id}: give a field, state, check, move or remove`)
    return out
  })
}

/** A tool answered: plan_read gives the open tasks by id (and the whole text with full), giving any task without an id
 *  one first; an edit gives the ids it touched and the open tasks after it. */
export async function answerPlanTool(tool: PlanTool, input: Record<string, unknown>, h: PlanToolHost): Promise<unknown> {
  const file = h.file ?? "plan.org"
  if (tool === "plan_read") {
    let text = h.text()
    // a task written without an id (by hand, in Emacs) is given one first: an agent names tasks by id, never by place
    if (readPlan(text).tasks.some((t) => !t.id)) text = (await h.apply([])).text
    return { file, open: planDigest(readPlan(text), { max: 200 }), ...(input.full ? { text } : {}) }
  }
  const edits = planToolEdits(tool, input, h.who)
  // plan_close closes: a keyword of the plan's that isn't after its bar (MAKING) is plan_update's
  const close = edits[0]
  if (tool === "plan_close" && close.op === "state" && !/^(done|dropped)$/i.test(close.state) && !isClosed(readPlan(h.text()).words.get(close.state)))
    throw new Error(`state: done or dropped, or a closing keyword of the plan's; ${close.state} doesn't close (plan_update sets it)`)
  const r = await h.apply(edits)
  return { file, ids: [...new Set(r.refs.filter(Boolean))], open: planDigest(readPlan(r.text)) }
}
