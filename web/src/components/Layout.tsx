import type { ComponentChildren } from 'preact';
import { useEffect, useState } from 'preact/hooks';

import { onLinkClick } from '../router';
import { MenuIcon, SignOutIcon, XIcon } from './icons';
import Logo from './Logo';

/** Clears IAP's sign-in cookie. */
const SIGN_OUT_URL = '/?gcp-iap-mode=CLEAR_LOGIN_COOKIE';

const NAV = [
  { href: '/alerts', label: 'Alerts' },
  { href: '/exec', label: 'Exec' },
  { href: '/status', label: 'Status Page' },
];

export default function Layout({ email, path, children }: {
  email: string;
  path: string;
  children: ComponentChildren;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  useEffect(() => setMenuOpen(false), [path]);

  return (
    <div class="min-h-screen bg-background">
      <header class="sticky top-0 z-20 border-b bg-background/85 backdrop-blur">
        <div class="mx-auto flex h-14 max-w-5xl items-center gap-6 px-4">
          <a href="/" class="flex shrink-0 items-center" aria-label="KCSU Admin" onClick={(e) => onLinkClick(e, '/')}>
            <Logo class="h-5" />
          </a>
          <span class="-mx-2 hidden h-5 w-px shrink-0 bg-border sm:block" aria-hidden="true" />

          <nav aria-label="Main" class="hidden h-full items-stretch gap-5 text-sm sm:flex">
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

          <div class="ml-auto hidden min-w-0 items-center gap-2 sm:flex">
            <span
              class="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-semibold uppercase"
              title={email}
              aria-hidden="true"
            >
              {email.charAt(0)}
            </span>
            <span class="truncate text-sm text-muted-foreground">{email}</span>
            <span class="mx-1 h-5 w-px bg-border" aria-hidden="true" />
            <a class="btn" data-variant="ghost" data-size="sm" href={SIGN_OUT_URL}>
              <SignOutIcon />
              Sign out
            </a>
          </div>

          <button
            type="button"
            class="btn -mr-2 ml-auto sm:hidden"
            data-variant="ghost"
            data-size="icon"
            aria-expanded={menuOpen}
            aria-controls="mobile-menu"
            onClick={() => setMenuOpen(!menuOpen)}
          >
            {menuOpen ? <XIcon class="size-5" /> : <MenuIcon class="size-5" />}
            <span class="sr-only">{menuOpen ? 'Close menu' : 'Open menu'}</span>
          </button>
        </div>

        {menuOpen && (
          <div id="mobile-menu" class="border-t px-4 py-2 sm:hidden">
            <nav aria-label="Main" class="-mx-2 flex flex-col">
              {NAV.map(({ href, label }) => {
                const current = path === href;
                return (
                  <a
                    key={href}
                    href={href}
                    aria-current={current ? 'page' : undefined}
                    onClick={(e) => onLinkClick(e, href)}
                    class={`rounded-control px-2 py-3 text-base ${
                      current ? 'bg-foreground/[0.06] font-medium text-foreground' : 'text-muted-foreground hover:text-foreground'
                    }`}
                  >
                    {label}
                  </a>
                );
              })}
            </nav>
            <div class="mt-2 flex items-center justify-between gap-3 border-t pt-2">
              <span class="min-w-0 truncate text-sm text-muted-foreground">{email}</span>
              <a class="btn -mr-2 text-base" data-variant="ghost" href={SIGN_OUT_URL}>
                <SignOutIcon class="size-5" />
                Sign out
              </a>
            </div>
          </div>
        )}
      </header>
      <main class="mx-auto max-w-5xl px-4 py-8">{children}</main>
    </div>
  );
}
