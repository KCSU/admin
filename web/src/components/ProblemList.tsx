// Alerts grouped by problem.

import { useState } from 'preact/hooks';

import {
  ACTIVITY_DAYS, type Alert, type Problem, type Recency, formatAgo, formatDateTime, formatDay, formatDayMonth,
  formatTime, lastLondonDays, londonDay, recency,
} from '../alerts';
import AlertDetail from './AlertDetail';
import { ChevronIcon } from './icons';

const SECTIONS: { id: Recency; title: string; dot: string }[] = [
  { id: 'day', title: 'Raised in the last 24 hours', dot: 'bg-red-500 animate-pulse' },
  { id: 'week', title: 'Raised in the last 7 days', dot: 'bg-amber-500' },
  { id: 'older', title: 'Older', dot: 'bg-muted-foreground/50' },
];

const RECENCY_TEXT: Record<Recency, string> = {
  day: 'text-red-400',
  week: 'text-amber-400',
  older: 'text-muted-foreground',
};

const GRID = 'grid items-center gap-x-4 grid-cols-[6.5rem_minmax(0,1fr)_3rem_1rem] '
  + 'md:grid-cols-[7.5rem_minmax(0,1fr)_3.5rem_14.25rem_1rem]';

const last = (problem: Problem) => new Date(problem.alerts[0].occurred_at);

export default function ProblemList({ problems, history, now }: {
  problems: Problem[];
  history: Map<string, Alert[]>;
  now: number;
}) {
  const [open, setOpen] = useState<string | null>(null);
  const [showOlder, setShowOlder] = useState(false);
  const toggle = (key: string) => setOpen((current) => (current === key ? null : key));

  return (
    <div class="space-y-8">
      {SECTIONS.map((section) => {
        const inSection = problems.filter((p) => recency(last(p), now) === section.id);
        if (!inSection.length) return null;
        const collapsed = section.id === 'older' && !showOlder;
        return (
          <section key={section.id}>
            <h2 class={`mb-2 flex items-center gap-2 text-sm font-semibold ${RECENCY_TEXT[section.id]}`}>
              <span class={`size-2 rounded-full ${section.dot}`} />
              {section.title}
              <span class="font-normal text-muted-foreground">{inSection.length}</span>
            </h2>
            {collapsed ? (
              <button
                type="button"
                class="w-full rounded-surface border border-dashed px-4 py-3 text-sm text-muted-foreground hover:bg-foreground/[0.03]"
                onClick={() => setShowOlder(true)}
              >
                Show {inSection.length} older {inSection.length === 1 ? 'problem' : 'problems'}
              </button>
            ) : (
              <div class={`overflow-hidden rounded-surface border bg-card ${section.id === 'day' ? 'border-red-500/40' : ''}`}>
                <div class={`${GRID} border-b bg-muted/40 px-4 py-2 text-xs font-medium text-muted-foreground`}>
                  <span>Last raised</span>
                  <span>Problem</span>
                  <span class="text-right">Alerts</span>
                  <span class="hidden text-right md:block">Last {ACTIVITY_DAYS} days</span>
                  <span />
                </div>
                <ul class="divide-y">
                  {inSection.map((problem) => {
                    const at = last(problem);
                    return (
                      <li key={problem.key}>
                        <button
                          type="button"
                          class={`${GRID} w-full px-4 py-3 text-left hover:bg-foreground/[0.03]`}
                          aria-expanded={open === problem.key}
                          onClick={() => toggle(problem.key)}
                        >
                          <span>
                            <span class={`block font-semibold ${RECENCY_TEXT[section.id]}`}>{formatAgo(at, now)}</span>
                            <span class="block text-xs text-muted-foreground tabular-nums">{formatDateTime(at)}</span>
                          </span>
                          <span class="min-w-0">
                            <span class="block break-words font-medium md:truncate" title={problem.alerts[0].summary}>{problem.alerts[0].summary}</span>
                            <code class="block break-words font-mono text-xs text-muted-foreground md:truncate">{problem.source} · {problem.dedup_key}</code>
                          </span>
                          <span class="text-right text-xl font-semibold tabular-nums">{problem.alerts.length}</span>
                          <span class="hidden justify-end md:flex"><Activity alerts={history.get(problem.key) ?? problem.alerts} now={now} /></span>
                          <ChevronIcon open={open === problem.key} />
                        </button>
                        {open === problem.key && <Occurrences alerts={problem.alerts} now={now} />}
                      </li>
                    );
                  })}
                </ul>
              </div>
            )}
          </section>
        );
      })}
    </div>
  );
}

/** Alerts per day for the last ACTIVITY_DAYS days, as small bars. */
function Activity({ alerts, now }: { alerts: Alert[]; now: number }) {
  const counts = new Map<string, number>();
  for (const alert of alerts) {
    const day = londonDay(new Date(alert.occurred_at));
    counts.set(day, (counts.get(day) ?? 0) + 1);
  }
  const bars = lastLondonDays(ACTIVITY_DAYS, now).map(({ day, date }) => ({ day, date, count: counts.get(day) ?? 0 }));
  const max = Math.max(1, ...bars.map((d) => d.count));
  return (
    <div class="flex h-8 items-end gap-px" aria-label={`Alerts per day, last ${ACTIVITY_DAYS} days`}>
      {bars.map(({ day, date, count }) => (
        <span
          key={day}
          class={`w-1.5 rounded-full ${count ? 'bg-destructive/80' : 'bg-foreground/10'}`}
          style={{ height: count ? `${25 + (75 * count) / max}%` : '15%' }}
          title={`${formatDay(date)}: ${count} ${count === 1 ? 'alert' : 'alerts'}`}
        />
      ))}
    </div>
  );
}

/** Every alert of a problem, newest first. */
function Occurrences({ alerts, now }: { alerts: Alert[]; now: number }) {
  return (
    <ol class="border-t bg-black/30">
      {alerts.map((alert) => (
        <li key={alert.id} class="border-b last:border-b-0">
          <Occurrence alert={alert} now={now} />
        </li>
      ))}
    </ol>
  );
}

/** One alert of a problem, which expands to show its detail and extra fields. */
function Occurrence({ alert, now }: { alert: Alert; now: number }) {
  const [open, setOpen] = useState(false);
  const at = new Date(alert.occurred_at);
  return (
    <>
      <button
        type="button"
        class="flex w-full items-baseline gap-4 px-4 py-2.5 text-left text-sm hover:bg-foreground/[0.03]"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <time
          class="w-24 shrink-0 tabular-nums text-muted-foreground"
          dateTime={alert.occurred_at}
          title={formatAgo(at, now)}
        >
          {formatDayMonth(at)}, {formatTime(at)}
        </time>
        <span class="min-w-0 flex-1 break-words md:truncate" title={alert.summary}>
          {alert.summary}
        </span>
        <ChevronIcon open={open} class="self-center" />
      </button>
      {open && <AlertDetail alert={alert} class="px-4 pt-1 pb-5" />}
    </>
  );
}
