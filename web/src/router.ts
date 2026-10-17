// A minimal client-side router.

import { useEffect, useState } from 'preact/hooks';

const EVENT = 'navigate';

/** Normalize URL's path to one without a trailing slash. */
function currentPath(): string {
  return location.pathname.replace(/(.)\/+$/, '$1');
}

/** Asked before leaving the page, e.g. to protect unpublished changes. */
let guard: (() => boolean) | null = null;

/** Sets a check that must pass before navigating away, or clears it with `null`. */
export function setNavigationGuard(check: (() => boolean) | null) {
  guard = check;
}

function navigate(path: string) {
  if (path === currentPath() || (guard && !guard())) return;
  history.pushState(null, '', path);
  window.dispatchEvent(new Event(EVENT));
  window.scrollTo(0, 0);
}

/** The current path, updated on navigation and on the browser's back and forward. */
export function usePath(): string {
  const [path, setPath] = useState(currentPath);
  useEffect(() => {
    const update = () => setPath(currentPath());
    window.addEventListener(EVENT, update);
    window.addEventListener('popstate', update);
    return () => {
      window.removeEventListener(EVENT, update);
      window.removeEventListener('popstate', update);
    };
  }, []);
  return path;
}

/** For a link's onClick: navigate in place, unless the visitor wants a new tab. */
export function onLinkClick(event: MouseEvent, path: string) {
  if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
  event.preventDefault();
  navigate(path);
}
