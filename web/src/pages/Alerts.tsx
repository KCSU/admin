import { useEffect, useMemo, useState } from 'preact/hooks';

import {
  ACTIVITY_DAYS, type AlertsResponse, DAY_MS, RETENTION_DAYS, fetchAlerts, groupByProblem,
} from '../alerts';
import AlertLog from '../components/AlertLog';
import { BellIcon, SearchIcon } from '../components/icons';
import Segmented from '../components/Segmented';
import ProblemList from '../components/ProblemList';

const RANGES = [
  { id: '24h', label: '24 hours', days: 1 },
  { id: '7d', label: '7 days', days: 7 },
  { id: '30d', label: '30 days', days: 30 },
  { id: 'all', label: 'All', days: RETENTION_DAYS },
] as const;
type RangeId = (typeof RANGES)[number]['id'];

type View = 'problems' | 'timeline';
/** How often times like "5 min ago" and the 24-hour section move on. */
const TICK_MS = 60 * 1000;

export default function Alerts() {
  // undefined while loading, null if loading failed.
  const [data, setData] = useState<AlertsResponse | null | undefined>(undefined);
  const [query, setQuery] = useState('');
  const [source, setSource] = useState('all');
  const [range, setRange] = useState<RangeId>('30d');
  const [view, setView] = useState<View>('problems');
  const now = useNow(TICK_MS);
  const rangeDays = RANGES.find((r) => r.id === range)!.days;
  const fetchDays = Math.max(ACTIVITY_DAYS, rangeDays);

  useEffect(() => {
    let cancelled = false;
    fetchAlerts(fetchDays).then(
      (response) => { if (!cancelled) setData(response); },
      (err) => {
        console.error('loading alerts failed:', err);
        // A failed fetch for a longer range keeps what's already shown.
        if (!cancelled) setData((current) => current ?? null);
      },
    );
    return () => { cancelled = true; };
  }, [fetchDays]);

  const alerts = data === undefined || data === null ? data : data.alerts;

  const sources = useMemo(() => [...new Set((alerts ?? []).map((a) => a.source))].sort(), [alerts]);

  // Lowercased once per fetch, not on every keystroke.
  const searchable = useMemo(() => (alerts ?? []).map((alert) => ({
    alert,
    text: [alert.source, alert.dedup_key, alert.summary, alert.detail ?? '', ...Object.values(alert.extra)]
      .join(' ').toLowerCase(),
  })), [alerts]);

  const shown = useMemo(() => {
    const since = now - rangeDays * DAY_MS;
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    return searchable
      .filter(({ alert, text }) =>
        (source === 'all' || alert.source === source)
        && Date.parse(alert.occurred_at) >= since
        && words.every((word) => text.includes(word)))
      .map(({ alert }) => alert);
  }, [searchable, query, source, rangeDays, now]);

  const problems = useMemo(() => groupByProblem(shown), [shown]);
  // Every fetched alert per problem, ignoring the filters, for the activity bars.
  const history = useMemo(
    () => new Map(groupByProblem(alerts ?? []).map((problem) => [problem.key, problem.alerts])),
    [alerts],
  );

  return (
    <>
      <div class="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 class="text-2xl font-semibold tracking-tight">Alerts</h1>
          <p class="mt-1 text-sm text-muted-foreground">
            All alerts that have been raised by KCSU's services. Alerts are kept for {RETENTION_DAYS} days.
          </p>
        </div>
      </div>

      {alerts === null ? (
        <EmptyState title="Alerts couldn't be loaded" body="Please try again later." />
      ) : (
        <>
          <div class="mt-6 flex flex-wrap items-center gap-2">
            <label class="relative min-w-56 flex-1">
              <span class="sr-only">Search alerts</span>
              <SearchIcon class="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <input
                type="search"
                class="input w-full pl-9"
                placeholder="Search summaries, details, keys…"
                value={query}
                onInput={(e) => setQuery(e.currentTarget.value)}
              />
            </label>
            <label>
              <span class="sr-only">Source</span>
              <select class="select" value={source} onChange={(e) => setSource(e.currentTarget.value)}>
                <option value="all">All sources</option>
                {sources.map((s) => <option key={s} value={s}>{s}</option>)}
              </select>
            </label>
            <Segmented
              label="Time range"
              options={RANGES.map((r) => ({ id: r.id, label: r.label }))}
              value={range}
              onChange={setRange}
            />
            <Segmented
              label="View"
              options={[{ id: 'problems', label: 'By problem' }, { id: 'timeline', label: 'Timeline' }]}
              value={view}
              onChange={setView}
            />
          </div>

          <div class="mt-4">
            {alerts === undefined ? (
              <Loading />
            ) : shown.length === 0 ? (
              <EmptyState
                title={alerts.length === 0 ? `No alerts in the last ${fetchDays} days` : 'No alerts match'}
                body={alerts.length === 0 ? 'Nothing has gone wrong. Long may it last.' : 'Try a longer time range, or clear the search.'}
              />
            ) : view === 'problems' ? (
              <ProblemList problems={problems} history={history} now={now} />
            ) : (
              <AlertLog alerts={shown} now={now} />
            )}
          </div>
        </>
      )}
    </>
  );
}

/** The current time, updated every `interval` milliseconds. */
function useNow(interval: number): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), interval);
    return () => clearInterval(timer);
  }, [interval]);
  return now;
}

function Loading() {
  return (
    <div class="space-y-3" aria-busy="true" aria-label="Loading alerts">
      {[0, 1, 2].map((i) => <div key={i} class="skeleton h-20 rounded-surface" />)}
    </div>
  );
}

function EmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div class="mt-6 rounded-surface border border-dashed px-6 py-16 text-center">
      <BellIcon class="mx-auto size-8 text-muted-foreground" />
      <h2 class="mt-3 font-medium">{title}</h2>
      <p class="mt-1 text-sm text-muted-foreground">{body}</p>
    </div>
  );
}
