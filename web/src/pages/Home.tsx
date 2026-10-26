import { onLinkClick } from '../router';

const CARDS = [
  { href: '/alerts', title: 'Alerts', subtitle: 'View alerts that have been raised by KCSU’s services.' },
  { href: '/exec', title: 'Exec Details', subtitle: 'The committee as shown on the KCSU website (kcsu.org.uk).' },
  { href: '/status', title: 'Status Page', subtitle: 'Incidents and maintenance, as shown on status.kcsu.org.uk.' },
];

export default function Home() {
  return (
    <>
      <h1 class="sr-only">KCSU Admin</h1>
      <div class="grid gap-4 md:grid-cols-2">
        {CARDS.map(({ href, title, subtitle }, i) => (
          <a
            key={href}
            href={href}
            onClick={(e) => onLinkClick(e, href)}
            class="flex items-center gap-4 rounded-surface border bg-card p-4 transition-colors hover:bg-foreground/[0.04] focus-visible:outline-2 focus-visible:outline-foreground"
          >
            <span aria-hidden="true" class="w-10 shrink-0 text-center text-3xl font-semibold tabular-nums">
              {String(i + 1).padStart(2, '0')}
            </span>
            <span aria-hidden="true" class="w-px self-stretch bg-border" />
            <div class="min-w-0">
              <h2 class="text-lg font-semibold tracking-tight">{title}</h2>
              <p class="mt-1 text-sm text-muted-foreground">{subtitle}</p>
            </div>
          </a>
        ))}
      </div>
    </>
  );
}
