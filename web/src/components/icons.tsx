// Line icons, from Lucide.

import type { ComponentChildren } from 'preact';

function Icon({ class: className, children }: { class?: string; children: ComponentChildren }) {
  return (
    <svg
      class={className}
      viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export function ChevronIcon({ open = false, class: className = '' }: { open?: boolean; class?: string }) {
  return (
    <Icon class={`size-4 shrink-0 text-muted-foreground transition-transform ${open ? 'rotate-180' : ''} ${className}`}>
      <path d="m6 9 6 6 6-6" />
    </Icon>
  );
}

export function SearchIcon({ class: className }: { class?: string }) {
  return (
    <Icon class={className}>
      <circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" />
    </Icon>
  );
}

export function BellIcon({ class: className }: { class?: string }) {
  return (
    <Icon class={className}>
      <path d="M10.268 21a2 2 0 0 0 3.464 0" />
      <path d="M3.262 15.326A1 1 0 0 0 4 17h16a1 1 0 0 0 .74-1.673C19.41 13.956 18 12.499 18 8A6 6 0 0 0 6 8c0 4.499-1.411 5.956-2.738 7.326" />
    </Icon>
  );
}

export function SignOutIcon({ class: className }: { class?: string }) {
  return (
    <Icon class={className}>
      <path d="m16 17 5-5-5-5" /><path d="M21 12H9" /><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
    </Icon>
  );
}
