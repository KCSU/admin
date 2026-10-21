import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';

import Layout from './components/Layout';
import Logo from './components/Logo';
import Alerts from './pages/Alerts';
import Exec from './pages/Exec';
import Status from './pages/Status';
import { usePath } from './router';
import './style.css';

/** The signed-in user, from /api/me. */
interface User {
  email: string;
}

function App() {
  // undefined while loading, null if failed.
  const [user, setUser] = useState<User | null | undefined>(undefined);
  const path = usePath();

  useEffect(() => {
    fetch('/api/me', { headers: { Accept: 'application/json' } })
      .then((response) => (response.ok ? response.json() : Promise.reject(new Error(`HTTP ${response.status}`))))
      .then(setUser, (err) => {
        console.error('fetching /api/me failed:', err);
        setUser(null);
      });
  }, []);

  if (user === undefined) return null;

  if (user === null) {
    return (
      <main class="min-h-screen flex items-center justify-center p-6">
        <div class="w-full max-w-sm flex flex-col items-center text-center gap-6">
          <Logo class="h-12" />
          <p class="text-muted-foreground">The admin API can't be reached right now. Please try again later.</p>
        </div>
      </main>
    );
  }

  return (
    <Layout email={user.email} path={path}>
      {path === '/exec' ? <Exec /> : path === '/status' ? <Status /> : <Alerts />}
    </Layout>
  );
}

render(<App />, document.getElementById('app')!);
