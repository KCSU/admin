// Editor for the incidents on the status page.

import { useEffect, useId, useMemo, useState } from 'preact/hooks';

import { formatDateTime } from '../alerts';
import { ApiError, type Versioned } from '../api';
import { ExternalLinkIcon, PlusIcon, TrashIcon, XIcon } from '../components/icons';
import {
  type Banner, type Notice, type Notices, PUBLIC_PAGE, SITES,
  fetchBanner, fetchNotices, formatUpdateTime, formatUpdated, fromInput, publishNotices, toInput,
} from '../notices';
import { setNavigationGuard } from '../router';

/** Reads as plain text until hovered or focused. */
const INLINE_FIELD = 'h-auto rounded-control border-0 bg-transparent px-2 py-2 text-sm leading-6 shadow-none outline outline-1 outline-transparent hover:outline-input focus:outline-input';

const nowSeconds = () => Math.floor(Date.now() / 1000);
const formatAt = (at: number) => formatDateTime(new Date(at * 1000));
const resolved = (notice: Notice) => notice.ends_at !== null && notice.ends_at <= nowSeconds();

export default function Status() {
  // undefined while loading, null if loading failed.
  const [stored, setStored] = useState<Versioned<Notices> | null | undefined>(undefined);
  const [draft, setDraft] = useState<Notice[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [publishing, setPublishing] = useState(false);
  const published = stored?.data.notices ?? [];

  const load = () => {
    setStored(undefined);
    fetchNotices().then(
      (notices) => {
        setStored(notices);
        setDraft(notices.data.notices);
      },
      (err) => {
        console.error('loading the incidents failed:', err);
        setStored(null);
      },
    );
  };
  useEffect(load, []);
  // null until loaded.
  const [banner, setBanner] = useState<Banner | null>(null);
  useEffect(() => {
    fetchBanner().then(setBanner, (err) => console.error('loading /uptime failed:', err));
  }, []);

  const changes = useMemo(() => describeChanges(published, draft), [published, draft]);
  const dirty = changes.count > 0;
  useUnsavedWarning(dirty);

  const selected = draft.find((notice) => notice.id === selectedId) ?? draft[0];

  const add = () => {
    const id = crypto.randomUUID();
    // Newest first.
    setDraft([{ id, kind: 'incident', title: '', body: null, site_ids: [], starts_at: nowSeconds(), ends_at: null, updates: [] }, ...draft]);
    setSelectedId(id);
  };

  const update = (notice: Notice) => setDraft(draft.map((existing) => (existing.id === notice.id ? notice : existing)));

  const remove = (notice: Notice) => {
    if (confirm(`Delete “${notice.title || 'Untitled'}” and its updates?`)) {
      setDraft(draft.filter((existing) => existing.id !== notice.id));
    }
  };

  const publish = async () => {
    if (!confirm('Are you sure? Your changes will be viewable publicly on status.kcsu.org.uk.')) return;
    setPublishing(true);
    try {
      // The server gives new notices their IDs.
      const ids = new Set(published.map((notice) => notice.id));
      const notices = draft.map((notice) => (ids.has(notice.id) ? notice : { ...notice, id: '' }));
      const saved = await publishNotices(stored!.version, notices);
      setStored(saved);
      setDraft(saved.data.notices);
    } catch (err) {
      console.error('publishing the incidents failed:', err);
      alert(err instanceof ApiError ? err.message : 'The incidents couldn’t be published. Please try again.');
    } finally {
      setPublishing(false);
    }
  };

  return (
    <>
      <div class="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 class="text-2xl font-semibold tracking-tight">Incidents</h1>
          <p class="mt-1 text-sm text-muted-foreground">
            As shown on status.kcsu.org.uk. Edits are local until you hit publish.
          </p>
        </div>
        <a class="btn" data-variant="outline" data-size="sm" href={PUBLIC_PAGE} target="_blank" rel="noreferrer">
          View status page
          <ExternalLinkIcon />
        </a>
      </div>

      <div class="mt-6 grid gap-4 md:grid-cols-[16rem_minmax(0,1fr)]">
        <div class="self-start">
          <IncidentList notices={draft} changes={changes.notices} selected={selected} onSelect={setSelectedId} />
          <button
            type="button"
            class="mt-2 flex w-full items-center justify-center gap-2 rounded-surface border border-dashed px-3 py-2 text-sm text-muted-foreground hover:bg-foreground/[0.03] hover:text-foreground"
            onClick={add}
          >
            <PlusIcon class="size-4" />
            New incident
          </button>
        </div>

        {stored === undefined ? (
          <div class="skeleton h-64 rounded-surface" aria-busy="true" aria-label="Loading the incidents" />
        ) : stored === null ? (
          <div class="rounded-surface border border-dashed px-6 py-12 text-center">
            <h2 class="text-sm font-medium">The incidents couldn't be loaded</h2>
            <button type="button" class="btn mt-4" data-variant="outline" data-size="sm" onClick={load}>
              Try again
            </button>
          </div>
        ) : selected ? (
          <div class="min-w-0 space-y-3 self-start">
            <StatusPreview notice={selected} banner={banner} />
            <IncidentEditor key={selected.id} notice={selected} onChange={update} onRemove={() => remove(selected)} />
          </div>
        ) : (
          <div class="rounded-surface border border-dashed px-6 py-12 text-center text-sm text-muted-foreground">
            No incidents.
          </div>
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
                onClick={() => confirm('Discard all unpublished changes?') && setDraft(published)}
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

function IncidentList({ notices, changes, selected, onSelect }: {
  notices: Notice[];
  changes: Map<string, 'new' | 'edited'>;
  selected: Notice | undefined;
  onSelect: (id: string) => void;
}) {
  const groups = [
    { label: 'Ongoing', notices: notices.filter((notice) => !resolved(notice)) },
    { label: 'Resolved', notices: notices.filter(resolved) },
  ];
  return (
    <div class="space-y-4">
      {groups.map(({ label, notices }) => notices.length > 0 && (
        <section key={label}>
          <h2 class="mb-2 px-1 text-xs font-medium text-muted-foreground">{label}</h2>
          <ol class={`divide-y overflow-hidden rounded-surface border bg-card ${label === 'Ongoing' ? 'border-2 border-dashed border-rose-400/70 divide-rose-400/30' : ''}`}>
            {notices.map((notice) => {
              const change = changes.get(notice.id);
              const current = notice === selected;
              return (
                <li key={notice.id}>
                  <button
                    type="button"
                    class={`flex w-full items-center gap-3 px-3 py-2 text-left hover:bg-foreground/[0.03] ${current ? 'bg-foreground/[0.06]' : ''}`}
                    aria-current={current ? 'true' : undefined}
                    onClick={() => onSelect(notice.id)}
                  >
                    <span class="min-w-0 flex-1">
                      <span class="block truncate text-sm font-medium">{notice.title || 'Untitled'}</span>
                      <span class="block truncate text-xs text-muted-foreground">
                        {resolved(notice) ? `Resolved ${formatAt(notice.ends_at!)}` : `Since ${formatAt(notice.starts_at)}`}
                      </span>
                    </span>
                    {change && (
                      <span class="size-2 shrink-0 rounded-full bg-sky-400" title={change === 'new' ? 'New' : 'Edited'} />
                    )}
                  </button>
                </li>
              );
            })}
          </ol>
        </section>
      ))}
    </div>
  );
}

function IncidentEditor({ notice, onChange, onRemove }: {
  notice: Notice; onChange: (notice: Notice) => void; onRemove: () => void;
}) {
  const id = useId();
  const [text, setText] = useState('');
  const set = <K extends keyof Notice>(field: K, value: Notice[K]) => onChange({ ...notice, [field]: value });
  const isResolved = resolved(notice);

  const post = () => {
    set('updates', [...notice.updates, { at: nowSeconds(), text: text.trim() }]);
    setText('');
  };

  const setUpdate = (index: number, change: Partial<Notice['updates'][number]>) =>
    set('updates', notice.updates
      .map((u, i) => (i === index ? { ...u, ...change } : u))
      .sort((a, b) => a.at - b.at));

  return (
    <div class="overflow-hidden rounded-surface border bg-card">
      <div class="grid gap-5 p-5">
        <div class="field">
          <label for={`${id}-title`}>Title</label>
          <input
            id={`${id}-title`}
            type="text"
            placeholder="e.g. Security issue affecting KORDs"
            value={notice.title}
            onInput={(e) => set('title', e.currentTarget.value)}
          />
        </div>
        <div class="field">
          <label>Affects</label>
          <div class="flex flex-wrap gap-2">
            {SITES.map((site) => {
              const on = notice.site_ids.includes(site.id);
              return (
                <button
                  key={site.id}
                  type="button"
                  aria-pressed={on}
                  class={`rounded-full border px-3 py-1 text-sm transition-colors ${on ? 'border-rose-400/50 bg-rose-400/15 text-rose-200' : 'text-muted-foreground hover:text-foreground'}`}
                  onClick={() => set('site_ids', on ? notice.site_ids.filter((s) => s !== site.id) : [...notice.site_ids, site.id])}
                >
                  {site.short}
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <div class="border-t p-5">
        <textarea
          rows={2}
          class="w-full text-sm"
          aria-label="New update"
          placeholder="Post an update."
          value={text}
          onInput={(e) => setText(e.currentTarget.value)}
        />
        <div class="mt-2 flex justify-end">
          <button type="button" class="btn" data-size="sm" disabled={!text.trim()} onClick={post}>
            Post update
          </button>
        </div>
      </div>

      {notice.updates.length > 0 && (
        <ol class="divide-y border-t">
          {notice.updates.map((u, i) => ({ u, i })).reverse().map(({ u, i }) => (
            <li key={`${u.at}-${i}`} class="group grid items-start gap-x-2 px-4 py-2 sm:grid-cols-[auto_minmax(0,1fr)_auto]">
              <input
                type="datetime-local"
                class={`${INLINE_FIELD} w-36 text-muted-foreground tabular-nums [&::-webkit-calendar-picker-indicator]:hidden`}
                aria-label="Posted at"
                value={toInput(u.at)}
                onChange={(e) => {
                  const at = fromInput(e.currentTarget.value);
                  if (at !== null) setUpdate(i, { at });
                }}
              />
              <textarea
                rows={1}
                class={`${INLINE_FIELD} field-sizing-content min-h-0 resize-none`}
                aria-label="Update text"
                value={u.text}
                onInput={(e) => setUpdate(i, { text: e.currentTarget.value })}
              />
              <button
                type="button"
                class="btn mt-1 opacity-0 group-hover:opacity-100 focus:opacity-100"
                data-variant="ghost"
                data-size="icon-sm"
                aria-label="Remove update"
                title="Remove update"
                onClick={() => confirm('Remove this update?') && set('updates', notice.updates.filter((_, j) => j !== i))}
              >
                <XIcon />
              </button>
            </li>
          ))}
        </ol>
      )}

      <div class="flex flex-wrap items-center gap-3 border-t bg-muted/30 px-5 py-3 text-sm">
        {isResolved ? (
          <button type="button" class="btn" data-size="sm" onClick={() => set('ends_at', null)}>Reopen</button>
        ) : (
          <button type="button" class="btn" data-size="sm" onClick={() => set('ends_at', nowSeconds())}>Resolve</button>
        )}
        <button type="button" class="btn ml-auto text-destructive" data-variant="ghost" data-size="sm" onClick={onRemove}>
          <TrashIcon />
          Delete
        </button>
      </div>
    </div>
  );
}

/** The incident as status.kcsu.org.uk shows it, in dark mode. */
function StatusPreview({ notice, banner }: { notice: Notice; banner: Banner | null }) {
  const now = nowSeconds();
  const affected = SITES.filter((site) => notice.site_ids.includes(site.id));
  const hidden = resolved(notice);
  return (
    <div class="overflow-hidden rounded-surface border">
      <p class="border-b bg-card px-5 py-2 text-xs text-muted-foreground">
        Preview of status.kcsu.org.uk
      </p>
      <div class={`bg-[#111] p-5 text-base leading-normal text-[#eee] ${hidden ? 'opacity-50' : ''}`} style={{ fontFamily: "'Figtree', system-ui, sans-serif" }}>
        <div class="rounded-t-lg bg-[#d0343a] px-4 py-3 text-[20px] font-normal text-white">
          {banner && !banner.all_up ? 'Some systems are down' : 'Some systems are having problems'}
          {banner && (
            <small class="block text-[0.8rem] opacity-85">Last updated: {formatUpdated(banner.updated_at)}</small>
          )}
        </div>
        <section class="flex flex-col gap-2 rounded-b-lg border border-t-0 border-[#5a2a2d] bg-[#2a1517] p-4">
          <h2 class="text-[18px] leading-[1.3] font-semibold text-[#f29a9e]">
            {notice.title || <span class="italic opacity-60">No title</span>}
          </h2>
          {notice.body && <p class="whitespace-pre-line">{notice.body}</p>}
          {affected.length > 0 && (
            <p class="text-[14px] opacity-75">
              Affects:{' '}
              {affected.map((site, i) => (
                <span key={site.id}>
                  {i > 0 && ', '}
                  <span class="underline">{site.name}</span>
                </span>
              ))}
            </p>
          )}
          {notice.updates.length > 0 && (
            <ol class="mt-1 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-2.5 border-t border-[#5a2a2d] pt-3 text-[14px]">
              {[...notice.updates].reverse().map((update, i) => (
                <li key={i} class="col-span-2 grid grid-cols-subgrid">
                  <time class="text-[#999] tabular-nums">{formatUpdateTime(update.at, now)}</time>
                  <span class="whitespace-pre-line">{update.text}</span>
                </li>
              ))}
            </ol>
          )}
        </section>
      </div>
    </div>
  );
}

/** What differs between the published incidents and the draft. */
function describeChanges(published: Notice[], draft: Notice[]) {
  const before = new Map(published.map((notice) => [notice.id, notice]));
  const after = new Set(draft.map((notice) => notice.id));
  const notices = new Map<string, 'new' | 'edited'>();
  for (const notice of draft) {
    const old = before.get(notice.id);
    if (!old) notices.set(notice.id, 'new');
    else if (JSON.stringify(old) !== JSON.stringify(notice)) notices.set(notice.id, 'edited');
  }
  const removed = published.filter((notice) => !after.has(notice.id)).length;

  const added = [...notices.values()].filter((c) => c === 'new').length;
  const edited = notices.size - added;
  const parts = [
    added && `${added} added`,
    edited && `${edited} edited`,
    removed && `${removed} deleted`,
  ].filter(Boolean);
  return { notices, count: added + edited + removed, summary: parts.join(', ') };
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
