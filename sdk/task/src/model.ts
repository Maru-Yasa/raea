// A plan as values (docs/plan.feature PL2): every headline of the plan.org as an entry, the ones with a TODO keyword
// as tasks, each with its state, priority, tags, dates, properties, checkboxes, progress and links. Read-only: edits
// go through edit.ts, by task id, and come back as text.
import { plans, sequence, type PlanRegistry } from "./extend.ts"
import { bare, checksOf, COOKIE, fileSequences, headValue, linksIn, linkText, parseOrg, printSection, type Check, type Link, type OrgDoc } from "./org.ts"
import type { Timestamp } from "./time.ts"

/** Where a task stands, whatever its keyword is called: the board's lanes, and done or dropped for closed. */
export type State = "todo" | "making" | "review" | "done" | "dropped"

/** Done and dropped are closed: Org's words after the bar. */
export const isClosed = (s: State | undefined) => s === "done" || s === "dropped"

export interface Task {
  /** how an edit names it: its :ID:, or #N (its place among the headlines) for one that has none yet */
  ref: string
  id?: string
  /** its place among the plan's headlines */
  index: number
  /** 0-based line of its headline in the file */
  line: number
  level: number
  keyword?: string
  /** undefined for a heading that only groups tasks */
  state?: State
  priority?: string
  /** the headline as written: links and a progress cookie included */
  title: string
  /** the headline as a reader sees it: each link as its words, no cookie */
  text: string
  tags: string[]
  /** tags from the headings above it and #+FILETAGS (Org's tag inheritance) */
  inherited: string[]
  scheduled?: Timestamp
  deadline?: Timestamp
  closed?: Timestamp
  /** every property, keys in upper case */
  props: Record<string, string>
  /** :ITEM:, the feed item it stands on */
  item?: string
  /** :AGENT:, who has it, by id */
  agents: string[]
  /** :FROM:, what asked for it */
  from?: string
  kind?: string
  verb?: string
  parent?: string
  children: string[]
  /** the nearest heading above it that isn't a task ("" at the top): the stage it's in */
  stage: string
  checks: Check[]
  /** its tasks under it, done of all; or, with none, its checkboxes */
  progress?: { done: number; total: number }
  links: Link[]
  /** the text under it after its planning line and drawer, as written */
  body: string
}

export interface Plan {
  title?: string
  filetags: string[]
  /** every headline in file order: the tasks and the headings that group them */
  entries: Task[]
  /** the entries with a keyword */
  tasks: Task[]
  /** the keywords this plan knows, each with its state: the registry's and its own #+TODO: lines' */
  words: Map<string, State>
  doc: OrgDoc
}

/** The keywords a plan's head and the registry know, with their states (the registry's win). */
export function keywordsOf(head: readonly string[], reg: PlanRegistry = plans): Map<string, State> {
  const words = new Map(reg.words)
  for (const s of fileSequences(head)) for (const w of sequence("file", s.open, s.done, {}, reg.words).words) if (!words.has(w.word)) words.set(w.word, w.state)
  return words
}

/** Reads a plan.org. Never throws: anything it doesn't read is kept as text. */
export function readPlan(text: string, reg: PlanRegistry = plans): Plan {
  const lines = text.split("\n")
  const first = lines.findIndex((l) => /^\*+[ \t]/.test(bare(l)))
  const words = keywordsOf(first < 0 ? lines : lines.slice(0, first), reg)
  return planOf(parseOrg(text, new Set(words.keys())), words)
}

const split = (v: string | undefined) => (v ? v.split(/[\s,]+/).filter(Boolean) : [])
const shown = (title: string) => linkText(title).replace(COOKIE, "").replace(/\s+/g, " ").trim()

/** The plan of an Org document already read. */
export function planOf(doc: OrgDoc, words: Map<string, State>): Plan {
  const filetags = split(headValue(doc.head, "FILETAGS")?.replace(/:/g, " "))
  const entries: Task[] = []
  const stack: Task[] = []
  let line = doc.head.length
  doc.sections.forEach((s, index) => {
    const props: Record<string, string> = {}
    for (const [k, v] of s.props ?? []) props[k.toUpperCase()] = v
    while (stack.length && stack.at(-1)!.level >= s.level) stack.pop()
    const parent = stack.at(-1)
    const heading = [...stack].reverse().find((t) => !t.state)
    const id = props.ID || undefined
    const t: Task = {
      ref: id ?? `#${index}`,
      ...(id && { id }),
      index,
      line,
      level: s.level,
      ...(s.keyword && { keyword: s.keyword, state: words.get(s.keyword) }),
      ...(s.priority && { priority: s.priority }),
      title: s.title,
      text: shown(s.title),
      tags: s.tags,
      inherited: [...new Set([...filetags, ...(parent ? [...parent.inherited, ...parent.tags] : [])])],
      ...(s.planning.scheduled && { scheduled: s.planning.scheduled }),
      ...(s.planning.deadline && { deadline: s.planning.deadline }),
      ...(s.planning.closed && { closed: s.planning.closed }),
      props,
      ...(props.ITEM && { item: props.ITEM }),
      agents: split(props.AGENT),
      ...(props.FROM && { from: props.FROM }),
      ...(props.KIND && { kind: props.KIND }),
      ...(props.VERB && { verb: props.VERB }),
      ...(parent && { parent: parent.ref }),
      children: [],
      stage: heading?.text ?? "",
      checks: checksOf(s.body),
      links: linksIn([s.title, ...s.body].join("\n")),
      body: s.body.map(bare).join("\n"),
    }
    parent?.children.push(t.ref)
    stack.push(t)
    entries.push(t)
    line += printSection(s, doc.cr).length
  })
  const byRef = new Map(entries.map((t) => [t.ref, t]))
  for (const t of entries) {
    const kids = t.children.map((r) => byRef.get(r)!).filter((c) => c.state)
    if (kids.length) t.progress = { done: kids.filter((c) => isClosed(c.state)).length, total: kids.length }
    else if (t.checks.length) {
      const top = Math.min(...t.checks.map((c) => c.indent))
      const own = t.checks.filter((c) => c.indent === top)
      t.progress = { done: own.filter((c) => c.done).length, total: own.length }
    }
  }
  return { title: headValue(doc.head, "TITLE"), filetags, entries, tasks: entries.filter((t) => t.state), words, doc }
}

/** An entry by its ref: its :ID:, or #N. */
export function find(plan: Plan, ref: string): Task | undefined {
  return plan.entries.find((t) => t.ref === ref || t.id === ref)
}

/** The open tasks (not done or dropped), in the plan's order. */
export const openTasks = (plan: Plan) => plan.tasks.filter((t) => !isClosed(t.state))

/** A deadline or schedule as the app shows it: MM-DD this year, YYYY-MM-DD otherwise. */
export function shortDay(t: Timestamp | undefined, now = new Date()): string | undefined {
  if (!t) return undefined
  return t.date.startsWith(`${now.getFullYear()}-`) ? t.date.slice(5) : t.date
}
