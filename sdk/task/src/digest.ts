// A plan in a few lines for an agent's prompt (docs/plan.feature PL5): the open tasks with their ids, keywords,
// priorities, dates and links, under the headings they sit in, and how many are done. An agent reads this each turn
// and edits the plan by those ids.
import { isClosed, openTasks, type Plan, type Task } from "./model.ts"

/** One task as a line: its id, keyword, priority, words, tags, dates, item and who has it. */
export function taskLine(t: Task): string {
  const bits = [t.ref, t.keyword ?? "", t.priority ? `[#${t.priority}]` : "", t.text]
  if (t.tags.length) bits.push(`:${t.tags.join(":")}:`)
  if (t.deadline) bits.push(`DEADLINE ${t.deadline.date}`)
  if (t.scheduled) bits.push(`SCHEDULED ${t.scheduled.date}`)
  if (t.progress) bits.push(`[${t.progress.done}/${t.progress.total}]`)
  if (t.item) bits.push(`item ${t.item}`)
  if (t.agents.length) bits.push(`agent ${t.agents.join(" ")}`)
  if (t.from) bits.push(`from ${t.from}`)
  return bits.filter(Boolean).join("  ")
}

/** The open tasks under their headings, at most `max` (40) of them, and a count of the closed ones. */
export function planDigest(plan: Plan, o: { max?: number } = {}): string {
  const open = openTasks(plan)
  const max = o.max ?? 40
  const lines: string[] = []
  let stage: string | undefined
  for (const t of open.slice(0, max)) {
    if (t.stage !== stage) {
      stage = t.stage
      if (stage) lines.push(`${stage}:`)
    }
    lines.push(`${"  ".repeat(Math.max(0, t.level - 1))}- ${taskLine(t)}`)
  }
  if (open.length > max) lines.push(`... and ${open.length - max} more open`)
  const closed = plan.tasks.filter((t) => isClosed(t.state)).length
  if (!open.length) lines.push("No open tasks.")
  if (closed) lines.push(`${closed} closed (done or dropped).`)
  return lines.join("\n")
}
