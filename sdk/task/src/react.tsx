// React for plans (docs/plan.feature PL6): usePlan reads a plan from text or from a source that changes (the app's
// store, a file an agent edits), and edits it by task id; the components draw a task, the outline, an agenda and
// board lanes as plain elements with data- attributes to style, nothing else. They work in raea's own window too:
// no portals, no dangerouslySetInnerHTML, nothing loaded later.
import { useCallback, useMemo, useSyncExternalStore, type ReactNode } from "react"
import { applyEdits, toggled, type PlanEdit } from "./edit.ts"
import { plans, view, type PlanRegistry } from "./extend.ts"
import { isClosed, readPlan, type Plan, type State, type Task } from "./model.ts"
import { agenda, board, outline, type OutlineNode } from "./views.ts"

/** Where a plan's text lives: read it, hear when it changes, and (when it can be written) edit it. */
export interface PlanSource {
  read(): string | undefined
  subscribe(onChange: () => void): () => void
  edit?(edits: PlanEdit[]): void
}

/** A plan kept in memory: a composition's own, or a test's. */
export function memorySource(text = ""): PlanSource & { text: string } {
  const subs = new Set<() => void>()
  const src = {
    text,
    read: () => src.text,
    subscribe: (f: () => void) => (subs.add(f), () => void subs.delete(f)),
    edit: (edits: PlanEdit[]) => {
      src.text = applyEdits(src.text, edits).text
      subs.forEach((f) => f())
    },
  }
  return src
}

const fixed = (text: string | undefined): PlanSource => ({ read: () => text, subscribe: () => () => {} })

/** The plan, read again whenever its source changes, and its edits. Text alone is read-only. */
export function usePlan(source: string | PlanSource | undefined, registry: PlanRegistry = plans): { plan: Plan; text: string; edit: (edits: PlanEdit[]) => void } {
  const src = useMemo(() => (source && typeof source === "object" ? source : fixed(source)), [source])
  const subscribe = useCallback((f: () => void) => src.subscribe(f), [src])
  const read = useCallback(() => src.read(), [src])
  const text = useSyncExternalStore(subscribe, read, read) ?? ""
  const plan = useMemo(() => readPlan(text, registry), [text, registry])
  const edit = useCallback(
    (edits: PlanEdit[]) => {
      if (!src.edit) throw new Error("this plan is read-only: its source has no edit")
      src.edit(edits)
    },
    [src],
  )
  return { plan, text, edit }
}

/** A view of the plan by name (extend.ts defineView), built again when the plan changes. */
export function useView<T = unknown, O = never>(plan: Plan, name: string, options?: O, registry: PlanRegistry = plans): T {
  return useMemo(() => view<T, O>(plan, name, options, registry), [plan, name, options, registry])
}

export interface TaskProps {
  task: Task
  /** its checkbox: called with the task; edits(toggled(task)) is the usual answer */
  onToggle?: (task: Task) => void
  onOpen?: (task: Task) => void
  className?: string
  children?: ReactNode
}

/** One task on one line: its checkbox, priority, words, progress, deadline and tags. A heading has no checkbox. */
export function TaskLine({ task, onToggle, onOpen, className, children }: TaskProps) {
  const done = isClosed(task.state)
  return (
    <div className={className} data-task={task.ref} data-state={task.state ?? "heading"} data-keyword={task.keyword} data-priority={task.priority}>
      {task.state && (
        <button type="button" role="checkbox" aria-checked={done} aria-label={`${done ? "Open again" : "Mark done"}: ${task.text}`} disabled={!onToggle} onClick={() => onToggle?.(task)} data-part="box">
          {done ? "✓" : ""}
        </button>
      )}
      {task.priority && <span data-part="priority">{task.priority}</span>}
      {onOpen ? (
        <button type="button" onClick={() => onOpen(task)} data-part="text">
          {task.text}
        </button>
      ) : (
        <span data-part="text">{task.text}</span>
      )}
      {task.progress && <span data-part="progress">{`${task.progress.done}/${task.progress.total}`}</span>}
      {task.deadline && <span data-part="deadline">{task.deadline.date}</span>}
      {task.tags.map((t) => (
        <span key={t} data-part="tag">
          {t}
        </span>
      ))}
      {children}
    </div>
  )
}

type Render = (task: Task) => ReactNode

interface ListProps {
  plan: Plan
  onToggle?: (task: Task) => void
  onOpen?: (task: Task) => void
  /** draws a task in place of TaskLine */
  renderTask?: Render
  className?: string
}

function line(p: ListProps, task: Task) {
  return p.renderTask ? p.renderTask(task) : <TaskLine task={task} onToggle={p.onToggle} onOpen={p.onOpen} />
}

/** The plan as a tree: each headline, the tasks under it nested. */
export function PlanOutline(p: ListProps & { filter?: (task: Task) => boolean }) {
  const keep = (n: OutlineNode): boolean => !p.filter || p.filter(n.task) || n.children.some(keep)
  const tree = (nodes: OutlineNode[]): ReactNode => (
    <ul data-part="outline">
      {nodes.filter(keep).map((n) => (
        <li key={n.task.ref} data-task={n.task.ref}>
          {line(p, n.task)}
          {n.children.length > 0 && tree(n.children)}
        </li>
      ))}
    </ul>
  )
  return <div className={p.className}>{tree(outline(p.plan))}</div>
}

/** What's due and scheduled, day by day, with what's overdue first. */
export function Agenda(p: ListProps & { from?: string; days?: number; label?: (date: string) => ReactNode }) {
  const a = agenda(p.plan, { from: p.from, days: p.days })
  return (
    <div className={p.className} data-part="agenda">
      {a.overdue.length > 0 && (
        <section data-part="overdue">
          <h3>Overdue</h3>
          <ul>
            {a.overdue.map((e) => (
              <li key={`${e.task.ref}-${e.why}`} data-why={e.why} data-date={e.date}>
                {line(p, e.task)}
              </li>
            ))}
          </ul>
        </section>
      )}
      {a.days.map((d) => (
        <section key={d.date} data-part="day" data-date={d.date} data-empty={d.entries.length === 0 || undefined}>
          <h3>{p.label ? p.label(d.date) : d.date}</h3>
          <ul>
            {d.entries.map((e) => (
              <li key={`${e.task.ref}-${e.why}`} data-why={e.why}>
                {line(p, e.task)}
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  )
}

const LANE: Record<State, string> = { todo: "To do", making: "Making", review: "To review", done: "Done", dropped: "Dropped" }

/** Board lanes by state: to do, making, to review, done. */
export function BoardLanes(p: ListProps & { states?: State[]; labels?: Partial<Record<State, string>>; filter?: (task: Task) => boolean }) {
  return (
    <div className={p.className} data-part="board">
      {board(p.plan, { states: p.states, filter: p.filter }).map((lane) => (
        <section key={lane.state} data-part="lane" data-state={lane.state}>
          <h3>
            {p.labels?.[lane.state] ?? LANE[lane.state]} <span data-part="count">{lane.tasks.length}</span>
          </h3>
          <ul>
            {lane.tasks.map((t) => (
              <li key={t.ref}>{line(p, t)}</li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  )
}

export { toggled }
