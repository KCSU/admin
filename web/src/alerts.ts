// Alerts raised by KCSU services.

export interface Alert {
  id: string;
  /** The service that raised it, e.g. "admin/uptime". */
  source: string;
  /** A stable name for the problem, e.g. "down:library". */
  dedup_key: string;
  summary: string;
  detail: string | null;
  extra: Record<string, string>;
  /** RFC 3339 timestamp. */
  occurred_at: string;
  /** RFC 3339 timestamp. */
  expires_at: string;
}

/** Alerts that share a source and dedup key. */
export interface Problem {
  key: string;
  source: string;
  dedup_key: string;
  /** Newest first. */
  alerts: Alert[];
}

/** The body of GET /api/alerts. */
export interface AlertsResponse {
  /** Newest first. */
  alerts: Alert[];
}

/** Firestore deletes alerts after this many days. */
export const RETENTION_DAYS = 180;
/** Days of history in each problem's activity bars. */
export const ACTIVITY_DAYS = 30;

export const DAY_MS = 24 * 60 * 60 * 1000;

/** How recently something happened: within a day, within a week, or longer ago. */
export type Recency = 'day' | 'week' | 'older';

export function recency(at: Date, now: number): Recency {
  const age = now - at.getTime();
  return age < DAY_MS ? 'day' : age < 7 * DAY_MS ? 'week' : 'older';
}

const TIME_ZONE = 'Europe/London';

/** The alerts of the last `days` days, newest first. */
export async function fetchAlerts(days: number): Promise<AlertsResponse> {
  const response = await fetch(`/api/alerts?days=${days}`, { headers: { Accept: 'application/json' } });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

export function problemKey(alert: Alert): string {
  return `${alert.source}\u0000${alert.dedup_key}`;
}

export function groupByProblem(alerts: Alert[]): Problem[] {
  const problems = new Map<string, Problem>();
  for (const alert of alerts) {
    const key = problemKey(alert);
    let problem = problems.get(key);
    if (!problem) {
      problem = { key, source: alert.source, dedup_key: alert.dedup_key, alerts: [] };
      problems.set(key, problem);
    }
    problem.alerts.push(alert);
  }
  // Problems are ordered by their latest alert.
  return [...problems.values()];
}

const LONDON_DAY = new Intl.DateTimeFormat('en-CA', {
  timeZone: TIME_ZONE, year: 'numeric', month: '2-digit', day: '2-digit',
});

/** The day `date` falls on in London, as YYYY-MM-DD. */
export function londonDay(date: Date): string {
  return LONDON_DAY.format(date);
}

/**
 * The last `count` calendar days in London, oldest first, ending today. Counts calendar
 * days rather than 24-hour steps, so a clock change doesn't skip or repeat a day.
 */
export function lastLondonDays(count: number, now: number): { day: string; date: Date }[] {
  const [year, month, day] = londonDay(new Date(now)).split('-').map(Number);
  return Array.from({ length: count }, (_, i) => {
    // Noon UTC is the same calendar day in London, whether it's on GMT or BST.
    const date = new Date(Date.UTC(year, month - 1, day - (count - 1 - i), 12));
    return { day: londonDay(date), date };
  });
}

const DAY_FORMAT = new Intl.DateTimeFormat('en-GB', {
  timeZone: TIME_ZONE, weekday: 'short', day: 'numeric', month: 'short',
});
const DAY_MONTH_FORMAT = new Intl.DateTimeFormat('en-GB', {
  timeZone: TIME_ZONE, day: 'numeric', month: 'short',
});
const TIME_FORMAT = new Intl.DateTimeFormat('en-GB', {
  timeZone: TIME_ZONE, hour: '2-digit', minute: '2-digit', hourCycle: 'h23',
});
const DATE_TIME_FORMAT = new Intl.DateTimeFormat('en-GB', {
  timeZone: TIME_ZONE, day: 'numeric', month: 'short', year: 'numeric',
  hour: '2-digit', minute: '2-digit', hourCycle: 'h23',
});

/** e.g. "Sat 17 Oct". */
export const formatDay = (date: Date) => DAY_FORMAT.format(date);
/** e.g. "17 Oct". */
export const formatDayMonth = (date: Date) => DAY_MONTH_FORMAT.format(date);
/** e.g. "14:05". */
export const formatTime = (date: Date) => TIME_FORMAT.format(date);
/** e.g. "17 Oct 2026, 14:05". */
export const formatDateTime = (date: Date) => DATE_TIME_FORMAT.format(date);

/** e.g. "5 min ago", "3 h ago", "2 days ago". */
export function formatAgo(date: Date, now: number): string {
  const minutes = Math.round((now - date.getTime()) / 60_000);
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  return days === 1 ? 'yesterday' : `${days} days ago`;
}
