// Status page notices, edited through /api/notices.

import { type Versioned, fetchDocument, saveDocument } from './api';

interface Update {
  /** Unix seconds. */
  at: number;
  text: string;
}

export interface Notice {
  id: string;
  kind: 'incident' | 'maintenance' | 'info';
  title: string;
  body: string | null;
  site_ids: string[];
  /** Unix seconds. */
  starts_at: number;
  /** Unix seconds. */
  ends_at: number | null;
  /** Oldest first. */
  updates: Update[];
}

export const PUBLIC_PAGE = 'https://status.kcsu.org.uk';

// todo(khm39): get this dynamically from somewhere
export const SITES = [
  { id: 'website', name: 'KCSU Website', short: 'Website' },
  { id: 'library', name: 'KCSU Library of Things', short: 'Library of Things' },
  { id: 'kords', name: "KORDs v5 (King's Open Room Database)", short: 'KORDs' },
  { id: 'store', name: 'KiFoMaSy (KCSU Formal Ticket Store)', short: 'KiFoMaSy' },
];

const DAY_FORMAT = new Intl.DateTimeFormat('en-GB', {
  weekday: 'short', day: 'numeric', month: 'short', timeZone: 'Europe/London',
});
const TIME_FORMAT = new Intl.DateTimeFormat('en-GB', {
  hour: '2-digit', minute: '2-digit', hourCycle: 'h23', timeZone: 'Europe/London',
});

/** As the status page formats an update's time, e.g. "Sat 18 Oct, 14:12". */
export function formatUpdateTime(unix: number, now: number): string {
  const day = (at: number) => DAY_FORMAT.format(new Date(at * 1000));
  const time = TIME_FORMAT.format(new Date(unix * 1000));
  return day(unix) === day(now) ? time : `${day(unix)}, ${time}`;
}

const UPDATED_FORMAT = new Intl.DateTimeFormat('en-GB', {
  day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit',
  hourCycle: 'h23', timeZone: 'Europe/London', timeZoneName: 'short',
});
const OFFSET_FORMAT = new Intl.DateTimeFormat('en-GB', { timeZone: 'Europe/London', timeZoneName: 'shortOffset' });

/** As the status page formats "Last updated". */
export function formatUpdated(unix: number): string {
  const at = new Date(unix * 1000);
  const text = UPDATED_FORMAT.format(at);
  const offset = OFFSET_FORMAT.formatToParts(at).find((part) => part.type === 'timeZoneName')?.value;
  return !offset || /^GMT([+-]0)?$/.test(offset) ? text : `${text} (${offset})`;
}

/** The status page's banner, from the public /uptime. */
export interface Banner {
  updated_at: number;
  all_up: boolean;
}

export async function fetchBanner(): Promise<Banner> {
  const response = await fetch('https://api.kcsu.org.uk/uptime');
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const uptime: { updated_at: number; sites: { latest: { error: string | null } | null }[] } = await response.json();
  return { updated_at: uptime.updated_at, all_up: uptime.sites.every((site) => !site.latest || site.latest.error === null) };
}

/** Unix seconds to a `datetime-local` value, in local time. */
export function toInput(at: number | null): string {
  if (at === null) return '';
  const date = new Date(at * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function fromInput(value: string): number | null {
  return value ? Math.floor(new Date(value).getTime() / 1000) : null;
}

export interface Notices {
  /** Newest first. */
  notices: Notice[];
}

const PATH = '/api/notices';

export const fetchNotices = () => fetchDocument<Notices>(PATH);

export const publishNotices = (version: string | null, notices: Notice[]): Promise<Versioned<Notices>> =>
  saveDocument(PATH, version, { notices });
