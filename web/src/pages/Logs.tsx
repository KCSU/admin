import type { ComponentChildren } from 'preact';
import { useEffect, useMemo, useState } from 'preact/hooks';

import { fetchAlerts } from '../alerts';
import { ApiError } from '../api';
import { ChevronIcon, ExternalLinkIcon, SearchIcon } from '../components/icons';
import LogVolume, { type Alert } from '../components/LogVolume';
import {
  DAY, HOUR_MINUTE, LEVEL, LEVELS, type LogEntry, type Logs as LogsData, type Severity, type Volume, fetchLogs, fetchVolume,
} from '../logs';

const MINUTE = 60_000;
const RANGES = [
  { id: '1h', label: 'Last 1 hour', hours: 1 },
  { id: '6h', label: 'Last 6 hours', hours: 6 },
  { id: '24h', label: 'Last 24 hours', hours: 24 },
  { id: '7d', label: 'Last 7 days', hours: 7 * 24 },
] as const;
type RangeId = (typeof RANGES)[number]['id'];

const PAGE = 100;
const SEARCH_DELAY_MS = 400;
const NO_ALERTS: Alert[] = [];

const TIME = new Intl.DateTimeFormat('en-GB', {
  timeZone: 'UTC', hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23',
});
const utc = (at: number) => new Date(at).toISOString().replace('T', ' ').replace('Z', ' UTC');

interface Loaded extends LogsData {
  volume: Volume | null;
  from: number;
  to: number;
}

export default function Logs() {
  const [range, setRange] = useState<RangeId>('24h');
  // Set by dragging across the chart. Replaces `range` until reset.
  const [zoom, setZoom] = useState<{ from: number; to: number } | null>(null);
  const [service, setService] = useState('all');
  const [contains, setContains] = useState('');
  const search = useDebounced(contains, SEARCH_DELAY_MS);
  // undefined while first loading, null if loading failed.
  const [data, setData] = useState<Loaded | null | undefined>(undefined);
  const [loadError, setLoadError] = useState('');
  const [loading, setLoading] = useState(true);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [services, setServices] = useState<Map<string, string>>(new Map());
  const [levels, setLevels] = useState<Set<Severity>>(new Set());
  const levelsQuery = useDebounced([...levels].sort().join(','), SEARCH_DELAY_MS);
  const [showTime, setShowTime] = useState(true);
  const [showLabels, setShowLabels] = useState(true);
  const [wrap, setWrap] = useState(false);
  const [showAlerts, setShowAlerts] = useState(true);
  const [shownCount, setShownCount] = useState(PAGE);
  const [open, setOpen] = useState<string | null>(null);
  const hours = RANGES.find((r) => r.id === range)!.hours;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    const to = zoom?.to ?? Date.now();
    const from = zoom?.from ?? to - hours * 60 * MINUTE;
    const range = zoom ?? { hours };
    const only = service === 'all' ? null : service;
    const counting = search.trim()
      ? Promise.resolve(null)
      : fetchVolume(range, only).catch((err) => {
        console.error('counting the logs failed:', err);
        return null;
      });
    Promise.all([fetchLogs(range, only, search, levelsQuery), counting]).then(
      ([logs, volume]) => {
        if (cancelled) return;
        setData({ ...logs, volume, from, to });
        setShownCount(PAGE);
        setServices((known) => new Map([...known, ...logs.entries.map((entry) => [entry.service_id, entry.service] as const)]));
      },
      (err) => {
        console.error('loading the logs failed:', err);
        if (cancelled) return;
        setLoadError(err instanceof ApiError ? err.message : 'Please try again later.');
        setData(null);
      },
    ).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [hours, zoom, service, search, levelsQuery]);

  useEffect(() => {
    const from = zoom?.from ?? Date.now() - hours * 60 * MINUTE;
    fetchAlerts(Math.ceil((Date.now() - from) / (24 * 60 * MINUTE))).then(
      (response) => setAlerts(response.alerts.map((alert) => ({ label: alert.summary, at: Date.parse(alert.occurred_at) }))),
      (err) => console.error('loading alerts failed:', err),
    );
  }, [hours, zoom]);

  const matching = useMemo(
    () => (!data ? [] : levels.size ? data.entries.filter((entry) => levels.has(entry.severity)) : data.entries),
    [data, levels],
  );
  const shown = matching.slice(0, shownCount);

  const toggleLevel = (level: Severity) => {
    const next = new Set(levels);
    if (next.has(level)) next.delete(level); else next.add(level);
    setLevels(next);
    setShownCount(PAGE);
  };

  return (
    <>
      <div class="flex flex-wrap items-end gap-2">
        <div class="mr-auto">
          <h1 class="text-2xl font-semibold tracking-tight">Logs</h1>
          <p class="mt-1 text-sm text-muted-foreground">
            Logs from KCSU's services on Google Cloud, with times in UTC. Logs are kept for 30 days.
          </p>
        </div>
        <label>
          <span class="sr-only">Time range</span>
          <select
            class="select h-8 font-medium"
            value={zoom ? 'zoom' : range}
            onChange={(e) => { setZoom(null); setRange(e.currentTarget.value as RangeId); }}
          >
            {zoom && (
              <option value="zoom" disabled>
                {DAY.format(zoom.from)} {HOUR_MINUTE.format(zoom.from)}–{DAY.format(zoom.to) === DAY.format(zoom.from) ? '' : `${DAY.format(zoom.to)} `}{HOUR_MINUTE.format(zoom.to)} UTC
              </option>
            )}
            {RANGES.map((r) => <option key={r.id} value={r.id}>{r.label}</option>)}
          </select>
        </label>
        {zoom && (
          <button type="button" class="btn" data-variant="outline" data-size="sm" onClick={() => setZoom(null)}>
            Reset zoom
          </button>
        )}
        <a class="btn" data-variant="outline" data-size="sm" href="https://console.cloud.google.com/logs/query?project=kcsu-admin" target="_blank" rel="noreferrer">
          View in Logs Explorer
          <ExternalLinkIcon />
        </a>
      </div>

      <div class="mt-6 flex flex-wrap items-center gap-2">
        <label>
          <span class="sr-only">Service</span>
          <select class="select" value={service} onChange={(e) => setService(e.currentTarget.value)}>
            <option value="all">All services</option>
            {[...services].sort(([, a], [, b]) => a.localeCompare(b)).map(([id, name]) => <option key={id} value={id}>{name}</option>)}
          </select>
        </label>
        <label class="relative min-w-56 flex-1">
          <span class="sr-only">Line contains</span>
          <SearchIcon class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="search"
            class="input w-full pl-9 font-mono text-sm"
            placeholder="Line contains…"
            value={contains}
            onInput={(e) => setContains(e.currentTarget.value)}
          />
        </label>
      </div>

      {data === undefined ? (
        <div class="mt-8 space-y-4" aria-busy="true" aria-label="Loading logs">
          <div class="skeleton h-64 rounded-surface" />
          <div class="skeleton h-96 rounded-surface" />
        </div>
      ) : data === null ? (
        <div class="mt-8 rounded-surface border border-dashed px-6 py-16 text-center">
          <h2 class="font-medium">The logs couldn't be loaded</h2>
          <p class="mt-1 text-sm text-muted-foreground">{loadError}</p>
        </div>
      ) : (
        <div class={`transition-opacity ${loading ? 'opacity-60' : ''}`} aria-busy={loading}>
          <section class="mt-8" aria-label="Logs volume">
            <div class="flex justify-end">
              <div class="flex flex-wrap gap-1">
                {LEVELS.map((level) => (
                  <LegendButton
                    key={level.id}
                    pressed={levels.has(level.id)}
                    dimmed={levels.size > 0 && !levels.has(level.id)}
                    onClick={() => toggleLevel(level.id)}
                  >
                    <span class={`h-1 w-3.5 rounded-full ${level.color}`} />
                    {level.label}
                  </LegendButton>
                ))}
                <LegendButton pressed={showAlerts} dimmed={!showAlerts} onClick={() => setShowAlerts(!showAlerts)}>
                  <span class="size-2 rotate-45 bg-red-400" />
                  alert
                </LegendButton>
              </div>
            </div>
            <LogVolume
              entries={data.entries}
              volume={data.volume}
              alerts={showAlerts ? alerts : NO_ALERTS}
              levels={levels}
              from={data.from}
              to={data.to}
              onZoom={(from, to) => setZoom({ from, to: Math.max(to, from + MINUTE) })}
            />
          </section>

          {data.truncated && (
            <p class="mt-4 text-sm text-muted-foreground">
              Only the newest {data.entries.length.toLocaleString('en-GB')} lines are shown. Choose a shorter time range or filter to see older ones.
            </p>
          )}

          <section class="mt-8 overflow-hidden rounded-surface border bg-card">
            <header class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-3 py-2">
              <h2 class="text-sm font-medium">Logs</h2>
              <span class="text-xs text-muted-foreground tabular-nums">{matching.length.toLocaleString('en-GB')} lines</span>
              <div class="ml-auto flex flex-wrap items-center gap-x-4 gap-y-2 text-xs">
                <Toggle label="Time (UTC)" on={showTime} onChange={setShowTime} />
                <Toggle label="Labels" on={showLabels} onChange={setShowLabels} />
                <Toggle label="Wrap lines" on={wrap} onChange={setWrap} />
              </div>
            </header>
            {shown.length === 0 ? (
              <p class="px-3 py-12 text-center text-sm text-muted-foreground">No logs match. Try a longer time range, or clear the filters.</p>
            ) : (
              <div class="overflow-x-auto py-1 font-mono text-xs leading-5">
                {shown.map((entry) => (
                  <Line
                    key={entry.id}
                    entry={entry}
                    showTime={showTime}
                    showLabels={showLabels}
                    wrap={wrap}
                    term={search.trim()}
                    isOpen={open === entry.id}
                    onToggle={() => setOpen(open === entry.id ? null : entry.id)}
                  />
                ))}
              </div>
            )}
            {shown.length < matching.length && (
              <div class="flex items-center justify-between gap-4 border-t px-3 py-2 text-xs text-muted-foreground">
                <span>Showing {shown.length} of {matching.length.toLocaleString('en-GB')}</span>
                <button type="button" class="btn" data-variant="outline" data-size="sm" onClick={() => setShownCount(shownCount + PAGE)}>
                  Show older
                </button>
              </div>
            )}
          </section>
        </div>
      )}
    </>
  );
}

function useDebounced<T>(value: T, delay: number): T {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);
  return settled;
}

function LegendButton({ pressed, dimmed, onClick, children }: {
  pressed: boolean; dimmed: boolean; onClick: () => void; children: ComponentChildren;
}) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      class={`inline-flex items-center gap-1.5 rounded-control px-1.5 py-0.5 text-xs hover:bg-foreground/[0.06] ${dimmed ? 'opacity-40' : ''}`}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function Toggle({ label, on, onChange }: { label: string; on: boolean; onChange: (on: boolean) => void }) {
  return (
    <label class="inline-flex cursor-pointer items-center gap-1.5 text-muted-foreground">
      <input type="checkbox" role="switch" class="input" checked={on} onChange={(e) => onChange(e.currentTarget.checked)} />
      {label}
    </label>
  );
}

function Line({ entry, showTime, showLabels, wrap, term, isOpen, onToggle }: {
  entry: LogEntry; showTime: boolean; showLabels: boolean; wrap: boolean; term: string; isOpen: boolean; onToggle: () => void;
}) {
  const level = LEVEL[entry.severity];
  const text = wrap ? entry.message : entry.message.split('\n')[0] + (entry.message.includes('\n') ? ' …' : '');
  return (
    <div class={isOpen ? 'bg-foreground/[0.04]' : ''}>
      <button
        type="button"
        aria-expanded={isOpen}
        class="flex w-full items-start gap-3 py-px pr-3 text-left hover:bg-foreground/[0.04]"
        onClick={onToggle}
      >
        <span class={`w-[3px] shrink-0 self-stretch ${level.color}`} />
        <ChevronIcon open={isOpen} class={`mt-0.5 size-3.5 ${isOpen ? '' : '-rotate-90'}`} />
        {showTime && (
          <span class="shrink-0 tabular-nums text-muted-foreground" title={utc(entry.timestamp)}>
            {DAY.format(entry.timestamp)} {TIME.format(entry.timestamp)}
          </span>
        )}
        <span class={`w-11 shrink-0 ${level.text}`}>{level.label.slice(0, 4)}</span>
        {showLabels && <span class="hidden w-52 shrink-0 truncate text-sky-300/80 md:block" title={entry.service}>{entry.service}</span>}
        <span class={`min-w-0 flex-1 ${wrap ? 'whitespace-pre-wrap break-words' : 'truncate'}`}><Highlight text={text} term={term} /></span>
      </button>
      {isOpen && <Details entry={entry} term={term} />}
    </div>
  );
}

function Highlight({ text, term }: { text: string; term: string }) {
  if (!term) return <>{text}</>;
  const lower = text.toLowerCase();
  const needle = term.toLowerCase();
  const parts: ComponentChildren[] = [];
  let at = 0;
  for (let i = lower.indexOf(needle); i !== -1; i = lower.indexOf(needle, at)) {
    parts.push(text.slice(at, i), <mark key={i} class="rounded-[2px] bg-yellow-300 text-black">{text.slice(i, i + needle.length)}</mark>);
    at = i + needle.length;
  }
  parts.push(text.slice(at));
  return <>{parts}</>;
}

function Details({ entry, term }: { entry: LogEntry; term: string }) {
  const fields: [string, string][] = [
    ['timestamp', utc(entry.timestamp)],
    ['severity', entry.severity],
    ['service', entry.service],
    ['revision_name', entry.revision],
    ...(entry.http ? [
      ['http.method', entry.http.method],
      ['http.host', entry.http.host],
      ['http.path', entry.http.path],
      ['http.status', String(entry.http.status)],
      ['http.latency', `${entry.http.latency_ms} ms`],
    ] as [string, string][] : []),
    ...(entry.trace ? [['trace', entry.trace]] as [string, string][] : []),
  ];
  return (
    <div class="ml-[3px] border-l border-border py-3 pr-4 pl-8 font-sans">
      {entry.message.includes('\n') && (
        <pre class="mb-3 max-h-72 overflow-auto rounded-control border bg-background/60 px-3 py-2 font-mono text-xs leading-5 whitespace-pre-wrap break-words">
          <Highlight text={entry.message} term={term} />
        </pre>
      )}
      <table class="w-full max-w-3xl text-xs">
        <caption class="mb-1 text-left text-xs font-medium text-muted-foreground">Fields</caption>
        <tbody>
          {fields.map(([key, value]) => (
            <tr key={key} class="border-t border-border/60">
              <th scope="row" class="w-40 py-1 pr-4 text-left font-mono font-normal text-muted-foreground">{key}</th>
              <td class="py-1 font-mono break-all"><Highlight text={value} term={term} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
