import { useEffect, useMemo, useState } from 'preact/hooks';

import { formatDateTime } from '../alerts';
import ExecEditor, { type RoleChange } from '../components/ExecEditor';
import { ExternalLinkIcon } from '../components/icons';
import { ApiError } from '../api';
import { type Exec as ExecData, PUBLIC_PAGE, type Role, fetchExec, publishExec, roleId } from '../exec';
import { setNavigationGuard } from '../router';

export default function Exec() {
  // undefined while loading, null if loading failed.
  const [published, setPublished] = useState<ExecData | null | undefined>(undefined);
  const [draft, setDraft] = useState<Role[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [publishing, setPublishing] = useState(false);

  const load = () => {
    setPublished(undefined);
    fetchExec().then(
      (exec) => {
        setPublished(exec);
        setDraft(exec.roles);
      },
      (err) => {
        console.error('loading the exec failed:', err);
        setPublished(null);
      },
    );
  };
  useEffect(load, []);

  const changes = useMemo(() => describeChanges(published?.roles ?? [], draft), [published, draft]);
  const dirty = changes.count > 0;
  useUnsavedWarning(dirty);

  const add = () => {
    const title = 'New role';
    const id = roleId(title, draft);
    setDraft([...draft, { id, title, emails: [], holders: [], bio: null, photo_url: null, instagram: [] }]);
    setSelectedId(id);
  };

  const update = (index: number, role: Role) => setDraft(draft.map((existing, i) => (i === index ? role : existing)));

  const remove = (index: number) => {
    const role = draft[index];
    if (confirm(`Remove ${role.title} from the draft?`)) setDraft(draft.filter((_, i) => i !== index));
  };

  const publish = async () => {
    if (!confirm('Are you sure? Your changes will be viewable publicly on the KCSU website.')) return;
    setPublishing(true);
    try {
      const exec = await publishExec(published!.updated_at, draft);
      setPublished(exec);
      // As the console tidied it, e.g. trimmed.
      setDraft(exec.roles);
      alert('Your changes will be on the website soon; this may take a couple of minutes.');
    } catch (err) {
      console.error('publishing the exec failed:', err);
      alert(err instanceof ApiError ? err.message : 'The exec couldn\'t be published. Please try again.');
    } finally {
      setPublishing(false);
    }
  };

  return (
    <>
      <div class="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 class="text-2xl font-semibold tracking-tight">Exec</h1>
          <p class="mt-1 text-sm text-muted-foreground">
            The committee as shown on the KCSU website (kcsu.org.uk).
          </p>
          {published && published.updated_at > 0 && (
            <p class="mt-1 text-xs text-muted-foreground">
              Last published {formatDateTime(new Date(published.updated_at * 1000))}
            </p>
          )}
        </div>
        <a class="btn" data-variant="outline" data-size="sm" href={PUBLIC_PAGE} target="_blank" rel="noreferrer">
          View on kcsu.org.uk
          <ExternalLinkIcon />
        </a>
      </div>

      <div class="mt-6">
        {published === undefined ? (
          <div class="space-y-2" aria-busy="true" aria-label="Loading the exec">
            {[0, 1, 2, 3].map((i) => <div key={i} class="skeleton h-16 rounded-surface" />)}
          </div>
        ) : published === null ? (
          <div class="rounded-surface border border-dashed px-6 py-12 text-center">
            <h2 class="font-medium">The exec couldn't be loaded</h2>
            <button type="button" class="btn mt-4" data-variant="outline" data-size="sm" onClick={load}>
              Try again
            </button>
          </div>
        ) : (
          <ExecEditor
            roles={draft}
            changes={changes.roles}
            selectedId={selectedId}
            onSelect={setSelectedId}
            onAdd={add}
            onUpdate={update}
            onRemove={remove}
          />
        )}
      </div>

      {dirty && <div class="h-24" />}

      {dirty && (
        <div class="fixed inset-x-0 bottom-4 z-30 px-4">
          <div class="mx-auto flex max-w-2xl flex-wrap items-center gap-3 rounded-surface border bg-popover px-4 py-3 shadow-lg shadow-black/40">
            <span class="size-2 rounded-full bg-sky-400" />
            <span class="text-sm">
              <span class="font-medium">{changes.count} unpublished {changes.count === 1 ? 'change' : 'changes'}</span>
              <span class="text-muted-foreground"> · {changes.summary}</span>
            </span>
            <div class="ml-auto flex gap-2">
              <button
                type="button"
                class="btn"
                data-variant="ghost"
                data-size="sm"
                disabled={publishing}
                onClick={() => confirm('Discard all unpublished changes?') && setDraft(published!.roles)}
              >
                Discard
              </button>
              <button type="button" class="btn" data-size="sm" disabled={publishing} onClick={publish}>
                {publishing ? 'Publishing…' : 'Publish'}
              </button>
            </div>
          </div>
        </div>
      )}

    </>
  );
}

/** Calculate what differs between the published roles and the draft. */
function describeChanges(published: Role[], draft: Role[]) {
  const before = new Map(published.map((role) => [role.id, role]));
  const after = new Set(draft.map((role) => role.id));
  const roles = new Map<string, RoleChange>();
  for (const role of draft) {
    const old = before.get(role.id);
    if (!old) roles.set(role.id, 'new');
    else if (JSON.stringify(old) !== JSON.stringify(role)) roles.set(role.id, 'edited');
  }
  const removed = published.filter((role) => !after.has(role.id)).length;

  const added = [...roles.values()].filter((c) => c === 'new').length;
  const edited = roles.size - added;
  const parts = [
    added && `${added} added`,
    edited && `${edited} edited`,
    removed && `${removed} removed`,
  ].filter(Boolean);
  return { roles, count: added + edited + removed, summary: parts.join(', ') };
}

/** Asks before leaving with unpublished changes. */
function useUnsavedWarning(dirty: boolean) {
  useEffect(() => {
    if (!dirty) return;
    const onBeforeUnload = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener('beforeunload', onBeforeUnload);
    setNavigationGuard(() => confirm('You have unpublished changes. Leave anyway?'));
    return () => {
      window.removeEventListener('beforeunload', onBeforeUnload);
      setNavigationGuard(null);
    };
  }, [dirty]);
}
