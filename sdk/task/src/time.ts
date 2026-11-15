// Org-mode timestamps (orgmode.org/manual/Timestamps.html): <2026-11-14 Sat> is active (it shows in an agenda),
// [2026-11-14 Sat 16:40] inactive (a record, like CLOSED). A time range, a repeater (+1w, .+1d, ++1m) and a warning
// (-2d) are read and written back; the day name is whatever was written, and English when raea writes one.

export interface Timestamp {
  active: boolean
  /** YYYY-MM-DD */
  date: string
  /** HH:MM */
  time?: string
  /** HH:MM, the end of a range on the same day */
  end?: string
  /** +1w, .+1d or ++1m; a habit's .+2d/3d */
  repeat?: string
  /** -2d or --2d */
  warn?: string
}

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
const STAMP = /^([<[])(\d{4}-\d{2}-\d{2})(?:[ \t]+([^\s\d>\]+-][^\s>\]]*))?(?:[ \t]+(\d{1,2}:\d{2})(?:-(\d{1,2}:\d{2}))?)?((?:[ \t]+(?:\.\+|\+\+|\+|--|-)\d+[hdwmy](?:\/\d+[hdwmy])?)*)[ \t]*([>\]])$/

/** A timestamp as written, or undefined when it isn't one (its brackets must match). */
export function parseStamp(s: string): Timestamp | undefined {
  const m = STAMP.exec(s.trim())
  if (!m || (m[1] === "<") !== (m[7] === ">")) return undefined
  const t: Timestamp = { active: m[1] === "<", date: m[2] }
  if (m[4]) t.time = m[4].padStart(5, "0")
  if (m[5]) t.end = m[5].padStart(5, "0")
  for (const part of m[6].trim().split(/[ \t]+/).filter(Boolean)) {
    if (part.startsWith("-")) t.warn = part
    else t.repeat = part
  }
  return t
}

/** The English day name of a YYYY-MM-DD date. */
export function dayName(date: string): string {
  const [y, m, d] = date.split("-").map(Number)
  return DAYS[new Date(Date.UTC(y, m - 1, d)).getUTCDay()]
}

/** A timestamp written the way Org writes it: <2026-11-14 Sat 10:00-11:00 +1w -2d>. */
export function formatStamp(t: Timestamp): string {
  const parts = [t.date, dayName(t.date)]
  if (t.time) parts.push(t.end ? `${t.time}-${t.end}` : t.time)
  if (t.repeat) parts.push(t.repeat)
  if (t.warn) parts.push(t.warn)
  return t.active ? `<${parts.join(" ")}>` : `[${parts.join(" ")}]`
}

const pad = (n: number) => String(n).padStart(2, "0")

/** A local date as YYYY-MM-DD. */
export const dateOf = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`

/** Now as Org records a change: an inactive stamp with the time, [2026-11-14 Sat 16:40]. */
export function nowStamp(now = new Date()): Timestamp {
  return { active: false, date: dateOf(now), time: `${pad(now.getHours())}:${pad(now.getMinutes())}` }
}

/** A day as a person or an agent writes it: YYYY-MM-DD, MM-DD (this year, as a brief wrote due dates), or an Org
 *  timestamp. Undefined when it is none of those. */
export function dayOf(s: string, now = new Date()): Timestamp | undefined {
  const v = s.trim()
  if (/^[<[]/.test(v)) return parseStamp(v)
  const full = /^(\d{4})-(\d{2})-(\d{2})$/.exec(v)
  const short = /^(\d{2})-(\d{2})$/.exec(v)
  const date = full ? v : short ? `${now.getFullYear()}-${short[1]}-${short[2]}` : undefined
  if (!date) return undefined
  const [y, m, d] = date.split("-").map(Number)
  const back = new Date(Date.UTC(y, m - 1, d))
  if (back.getUTCMonth() !== m - 1 || back.getUTCDate() !== d) return undefined
  return { active: true, date }
}
