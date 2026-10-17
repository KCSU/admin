// An alert's detail text and fields.

import { type Alert, formatDateTime } from '../alerts';

export default function AlertDetail({ alert, class: className = '' }: { alert: Alert; class?: string }) {
  const fields: [string, string][] = [
    ...Object.entries(alert.extra),
    ['source', alert.source],
    ['dedup_key', alert.dedup_key],
    ['occurred_at', formatDateTime(new Date(alert.occurred_at))],
    ['id', alert.id],
  ];
  return (
    <div class={`space-y-4 text-sm ${className}`}>
      {alert.detail && (
        <pre class="max-h-72 overflow-auto rounded-surface border bg-background/60 px-4 py-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-words">
          {alert.detail}
        </pre>
      )}
      <dl class="grid grid-cols-[8rem_minmax(0,1fr)] gap-x-4 gap-y-1.5 text-xs">
        {fields.map(([name, value]) => (
          <Field key={name} name={name} value={value} />
        ))}
      </dl>
    </div>
  );
}

function Field({ name, value }: { name: string; value: string }) {
  const link = /^https?:\/\//.test(value);
  return (
    <>
      <dt class="truncate font-mono text-muted-foreground" title={name}>{name}</dt>
      <dd class="min-w-0 break-words">
        {link ? <a class="underline underline-offset-4" href={value} target="_blank" rel="noreferrer">{value}</a> : value}
      </dd>
    </>
  );
}
