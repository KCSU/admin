// Timeline view of the alerts.

import { Fragment } from 'preact';
import { useState } from 'preact/hooks';

import { type Alert, formatAgo, formatDayMonth, formatTime } from '../alerts';
import AlertDetail from './AlertDetail';
import { ChevronIcon } from './icons';

export default function AlertLog({ alerts, now }: { alerts: Alert[]; now: number }) {
  const [open, setOpen] = useState<string | null>(null);
  const toggle = (id: string) => setOpen((current) => (current === id ? null : id));
  return (
    <div class="overflow-x-auto rounded-surface border bg-card">
      <table class="w-full font-mono text-xs">
        <thead class="border-b bg-muted/40 text-left text-muted-foreground">
          <tr>
            <th scope="col" class="px-3 py-2 font-medium">When</th>
            <th scope="col" class="hidden px-3 py-2 font-medium md:table-cell">Source</th>
            <th scope="col" class="hidden px-3 py-2 font-medium md:table-cell">Key</th>
            <th scope="col" class="px-3 py-2 font-medium">Summary</th>
            <th scope="col" class="w-8"><span class="sr-only">Details</span></th>
          </tr>
        </thead>
        <tbody>
          {alerts.map((alert, i) => {
            const at = new Date(alert.occurred_at);
            const isOpen = open === alert.id;
            return (
              <Fragment key={alert.id}>
                <tr
                  class={`cursor-pointer hover:bg-foreground/[0.03] ${i % 2 ? 'bg-foreground/[0.015]' : ''}`}
                  onClick={() => toggle(alert.id)}
                >
                  <td class="whitespace-nowrap px-3 py-1.5 tabular-nums" title={formatAgo(at, now)}>
                    {formatDayMonth(at)} {formatTime(at)}
                  </td>
                  <td class="hidden whitespace-nowrap px-3 py-1.5 text-muted-foreground md:table-cell">{alert.source}</td>
                  <td class="hidden whitespace-nowrap px-3 py-1.5 text-muted-foreground md:table-cell">{alert.dedup_key}</td>
                  <td class="px-3 py-1.5 font-sans text-sm break-words">{alert.summary}</td>
                  <td class="py-1 pr-2">
                    <button
                      type="button"
                      class="btn"
                      data-variant="ghost"
                      data-size="icon-xs"
                      aria-expanded={isOpen}
                      aria-label={`${isOpen ? 'Hide' : 'Show'} details of ${alert.summary}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        toggle(alert.id);
                      }}
                    >
                      <ChevronIcon open={isOpen} />
                    </button>
                  </td>
                </tr>
                {isOpen && (
                  <tr>
                    <td colSpan={5} class="border-y bg-black/30 px-4 py-5 font-sans">
                      <AlertDetail alert={alert} />
                    </td>
                  </tr>
                )}
              </Fragment>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
