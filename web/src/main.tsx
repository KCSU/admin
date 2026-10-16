import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';

import Logo from './components/Logo';
import './style.css';

/** The signed-in user, from /api/me. */
interface User {
  email: string;
}

/** Clears IAP's sign-in cookie. */
const SIGN_OUT_URL = '/?gcp-iap-mode=CLEAR_LOGIN_COOKIE';

function App() {
  // undefined while loading, null if failed.
  const [user, setUser] = useState<User | null | undefined>(undefined);

  useEffect(() => {
    fetch('/api/me', { headers: { Accept: 'application/json' } })
      .then((response) => (response.ok ? response.json() : Promise.reject(new Error(`HTTP ${response.status}`))))
      .then(setUser, (err) => {
        console.error('fetching /api/me failed:', err);
        setUser(null);
      });
  }, []);

  if (user === undefined) return null;

  return (
    <main class="min-h-screen flex items-center justify-center p-6">
      <div class="w-full max-w-sm flex flex-col items-center text-center gap-6">
        <Logo class="h-12" />
        {user ? (
          <>
            <h1 class="text-2xl font-semibold">Hello, {user.email}</h1>
            <a class="btn" data-variant="outline" href={SIGN_OUT_URL}>
              Sign out
            </a>
          </>
        ) : (
          <p class="text-muted-foreground">The admin API can't be reached right now. Please try again later.</p>
        )}
      </div>
    </main>
  );
}

render(<App />, document.getElementById('app')!);
