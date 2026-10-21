import type { ComponentChildren } from 'preact';

import { onLinkClick } from '../router';
import { SignOutIcon } from './icons';
import Logo from './Logo';

/** Clears IAP's sign-in cookie. */
const SIGN_OUT_URL = '/?gcp-iap-mode=CLEAR_LOGIN_COOKIE';

const NAV = [
  { href: '/', label: 'Alerts' },
  { href: '/exec', label: 'Exec' },
  { href: '/status', label: 'Status Page' },
];

export default function Layout({ email, path, children }: {
  email: string;
  path: string;
  children: ComponentChildren;
}) {
  return (
    <div class="min-h-screen bg-background">
      <header class="sticky top-0 z-20 border-b bg-background/85 backdrop-blur">
        <div class="mx-auto flex h-14 max-w-5xl items-center gap-6 px-4">
          <a href="/" class="flex shrink-0 items-center" aria-label="KCSU Admin" onClick={(e) => onLinkClick(e, '/')}>
            <Logo class="h-5" />
          </a>
          <span class="-mx-2 h-5 w-px shrink-0 bg-border" aria-hidden="true" />

          <nav aria-label="Main" class="flex h-full items-stretch gap-5 text-sm">
            {NAV.map(({ href, label }) => {
              const current = path === href;
              return (
                <a
                  key={href}
                  href={href}
                  aria-current={current ? 'page' : undefined}
                  onClick={(e) => onLinkClick(e, href)}
                  class={`relative flex items-center transition-colors ${
                    current
                      ? 'font-medium text-foreground after:absolute after:inset-x-0 after:-bottom-px after:h-0.5 after:bg-foreground'
                      : 'text-muted-foreground hover:text-foreground'
                  }`}
                >
                  {label}
                </a>
              );
            })}
          </nav>

          <div class="ml-auto flex min-w-0 items-center gap-2">
            <span
              class="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold uppercase"
              title={email}
              aria-hidden="true"
            >
              {email.charAt(0)}
            </span>
            <span class="hidden truncate text-sm text-muted-foreground sm:inline">{email}</span>
            <span class="mx-1 hidden h-5 w-px bg-border sm:block" aria-hidden="true" />
            <a class="btn" data-variant="ghost" data-size="sm" href={SIGN_OUT_URL}>
              <SignOutIcon />
              Sign out
            </a>
          </div>
        </div>
      </header>
      <main class="mx-auto max-w-5xl px-4 py-8">{children}</main>
    </div>
  );
}
