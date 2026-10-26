// The logs of KCSU's Cloud Run services and jobs.

import { json } from './api';

export type Severity = 'DEFAULT' | 'DEBUG' | 'INFO' | 'WARNING' | 'ERROR';

export interface LogEntry {
  id: string;
  /** Unix milliseconds. */
  timestamp: number;
  severity: Severity;
  service: string;
  service_id: string;
  revision: string;
  message: string;
  http?: { method: string; host: string; path: string; status: number; latency_ms: number };
  trace?: string;
}

export interface Logs {
  /** Newest first. */
  entries: LogEntry[];
  /** Whether older entries in the range were left out. */
  truncated: boolean;
}

export const LEVELS: { id: Severity; label: string; color: string; text: string }[] = [
  { id: 'DEFAULT', label: 'unknown', color: 'bg-zinc-500', text: 'text-muted-foreground' },
  { id: 'DEBUG', label: 'debug', color: 'bg-blue-400', text: 'text-blue-300' },
  { id: 'INFO', label: 'info', color: 'bg-emerald-500', text: 'text-emerald-300' },
  { id: 'WARNING', label: 'warning', color: 'bg-yellow-400', text: 'text-yellow-300' },
  { id: 'ERROR', label: 'error', color: 'bg-red-500', text: 'text-red-300' },
];
export const LEVEL = Object.fromEntries(LEVELS.map((level) => [level.id, level])) as Record<Severity, (typeof LEVELS)[number]>;

/** e.g. "26/10", in UTC. */
export const DAY = new Intl.DateTimeFormat('en-GB', { timeZone: 'UTC', day: '2-digit', month: '2-digit' });
/** e.g. "14:05", in UTC. */
export const HOUR_MINUTE = new Intl.DateTimeFormat('en-GB', {
  timeZone: 'UTC', hour: '2-digit', minute: '2-digit', hourCycle: 'h23',
});

export type TimeRange = { hours: number } | { from: number; to: number };

/** Lines per time slot, from the `log_lines` metric. */
export interface Volume {
  /** Unix milliseconds: when the first slot starts. */
  start: number;
  periodMs: number;
  /** Oldest first. */
  counts: Record<Severity, number>[];
}

export async function fetchVolume(range: TimeRange, service: string | null): Promise<Volume> {
  const params = rangeParams(range);
  if (service) params.set('service', service);
  const volume = await json<{ start: number; period: number; counts: Record<Severity, number>[] }>(
    await fetch(`/api/logs/volume?${params}`, { headers: { Accept: 'application/json' } }),
  );
  return {
    start: volume.start * 1000,
    periodMs: volume.period * 1000,
    counts: volume.counts,
  };
}

function rangeParams(range: TimeRange): URLSearchParams {
  return new URLSearchParams('hours' in range
    ? { hours: String(range.hours) }
    : { from: String(Math.floor(range.from / 1000)), to: String(Math.ceil(range.to / 1000)) });
}

/** `levels` is comma-separated, e.g. "WARNING,ERROR"; empty for all. */
export async function fetchLogs(range: TimeRange, service: string | null, contains: string, levels: string): Promise<Logs> {
  const params = rangeParams(range);
  if (service) params.set('service', service);
  if (contains.trim()) params.set('contains', contains.trim());
  if (levels) params.set('levels', levels);
  const logs = await json<{ entries: (Omit<LogEntry, 'timestamp'> & { timestamp: string })[]; truncated: boolean }>(
    await fetch(`/api/logs?${params}`, { headers: { Accept: 'application/json' } }),
  );
  return {
    entries: logs.entries.map((entry) => ({ ...entry, timestamp: Date.parse(entry.timestamp) })),
    truncated: logs.truncated,
  };
}
