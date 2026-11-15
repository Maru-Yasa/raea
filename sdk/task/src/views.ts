// raea's own views of a plan (extend.ts registers them): board lanes by state, an agenda by day, and the outline as a
// tree. Pure functions of the plan; a plugin's views are built the same way (defineView).
import type { Plan, State, Task } from "./model.ts"

export interface Lane {
  state: State
  tasks: Task[]
}

/** Lanes by state, in the board's order. Dropped tasks are on no lane unless asked for. */
export function board(plan: Plan, options?: { states?: State[]; filter?: (t: Task) => boolean }): Lane[] {
  const states = options?.states ?? ["todo", "making", "review", "done"]
  const tasks = plan.tasks.filter((t) => !options?.filter || options.filter(t))
  return states.map((state) => ({ state, tasks: tasks.filter((t) => t.state === state) }))
}

export interface AgendaDay {
  date: string
  entries: { task: Task; why: "deadline" | "scheduled" }[]
}

export interface Agenda {
  /** open tasks whose deadline or schedule is before the first day */
  overdue: { task: Task; why: "deadline" | "scheduled"; date: string }[]
  days: AgendaDay[]
}

const addDays = (date: string, n: number) => {
  const [y, m, d] = date.split("-").map(Number)
  const t = new Date(Date.UTC(y, m - 1, d + n))
  return t.toISOString().slice(0, 10)
}

/** What's due and scheduled day by day from `from` (YYYY-MM-DD, today by default) for `days` days, and what's
 *  overdue. Closed tasks are left out. */
export function agenda(plan: Plan, options?: { from?: string; days?: number }): Agenda {
  const now = new Date()
  const from = options?.from ?? `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`
  const n = Math.max(1, options?.days ?? 7)
  const open = plan.tasks.filter((t) => t.state !== "done" && t.state !== "dropped")
  const dated = open.flatMap((task) => [...(task.deadline ? [{ task, why: "deadline" as const, date: task.deadline.date }] : []), ...(task.scheduled ? [{ task, why: "scheduled" as const, date: task.scheduled.date }] : [])])
  const days = Array.from({ length: n }, (_, i) => addDays(from, i)).map((date) => ({ date, entries: dated.filter((e) => e.date === date).map(({ task, why }) => ({ task, why })) }))
  return { overdue: dated.filter((e) => e.date < from).sort((a, b) => a.date.localeCompare(b.date)), days }
}

export interface OutlineNode {
  task: Task
  children: OutlineNode[]
}

/** The plan as a tree of its headlines. */
export function outline(plan: Plan): OutlineNode[] {
  const nodes = new Map(plan.entries.map((t) => [t.ref, { task: t, children: [] as OutlineNode[] }]))
  const roots: OutlineNode[] = []
  for (const t of plan.entries) (t.parent ? nodes.get(t.parent)!.children : roots).push(nodes.get(t.ref)!)
  return roots
}
