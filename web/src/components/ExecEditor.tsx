// Editor for exec information.

import { useEffect, useState } from 'preact/hooks';

import type { Role } from '../exec';
import { PlusIcon, TrashIcon } from './icons';
import RoleFields, { Avatar, PhotoPicker, formFromRole, roleFromForm } from './RoleFields';

export type RoleChange = 'new' | 'edited';

export default function ExecEditor({ roles, changes, selectedId, onSelect, onAdd, onUpdate, onRemove }: {
  roles: Role[];
  changes: Map<string, RoleChange>;
  /** The role being edited. The first role if it's null. */
  selectedId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onUpdate: (index: number, role: Role) => void;
  onRemove: (index: number) => void;
}) {
  const index = Math.max(0, roles.findIndex((role) => role.id === selectedId));
  const selected = roles[index];
  return (
    <div class="grid gap-4 md:grid-cols-[16rem_minmax(0,1fr)]">
      <div class="self-start">
        <ol class="divide-y overflow-hidden rounded-surface border bg-card">
          {roles.map((role) => (
            <li key={role.id}>
              <RoleListItem
                role={role}
                change={changes.get(role.id)}
                selected={role === selected}
                onClick={() => onSelect(role.id)}
              />
            </li>
          ))}
        </ol>
        <button
          type="button"
          class="mt-2 flex w-full items-center justify-center gap-2 rounded-surface border border-dashed px-3 py-2 text-sm text-muted-foreground hover:bg-foreground/[0.03] hover:text-foreground"
          onClick={onAdd}
        >
          <PlusIcon class="size-4" />
          Add a role
        </button>
      </div>

      {selected && (
        <div class="min-w-0 space-y-3 self-start">
          <div class={`rounded-surface border bg-card ${changes.has(selected.id) ? 'border-sky-400/50' : ''}`}>
            <p class="border-b px-5 py-2 text-xs text-muted-foreground">
              This is a preview. Click the photo to change it.
            </p>
            <RoleCard
              role={selected}
              change={changes.get(selected.id)}
              onPhotoChange={(photoUrl) => onUpdate(index, { ...selected, photo_url: photoUrl })}
            />
          </div>
          <RoleEditor
            key={selected.id}
            role={selected}
            isNew={changes.get(selected.id) === 'new'}
            onChange={(role) => onUpdate(index, role)}
            onRemove={() => onRemove(index)}
          />
        </div>
      )}
    </div>
  );
}

function RoleListItem({ role, change, selected, onClick }: {
  role: Role; change: RoleChange | undefined; selected: boolean; onClick: () => void;
}) {
  return (
    <button
      type="button"
      class={`flex w-full items-center gap-3 px-3 py-2 text-left hover:bg-foreground/[0.03] ${selected ? 'bg-foreground/[0.06]' : ''}`}
      aria-current={selected ? 'true' : undefined}
      onClick={onClick}
    >
      <Avatar name={role.holders[0]?.name ?? role.title} photoUrl={role.photo_url} class="size-8 text-xs" />
      <span class="min-w-0 flex-1">
        <span class="block truncate text-sm font-medium">{role.title}</span>
        <span class="block truncate text-xs text-muted-foreground"><Names role={role} /></span>
      </span>
      {change ? (
        <span class="size-2 shrink-0 rounded-full bg-sky-400" title={change === 'new' ? 'New' : 'Edited'} />
      ) : warning(role) && (
        <span class="size-2 shrink-0 rounded-full bg-amber-400" title={warning(role)!} />
      )}
    </button>
  );
}

function RoleEditor({ role, isNew, onChange, onRemove }: {
  role: Role; isNew: boolean; onChange: (role: Role) => void; onRemove: () => void;
}) {
  const [form, setForm] = useState(() => formFromRole(role));
  useEffect(() => {
    if (JSON.stringify(roleFromForm(role.id, form)) !== JSON.stringify(role)) setForm(formFromRole(role));
  }, [role]);
  return (
    <div class="grid gap-5 rounded-surface border bg-card p-5">
      <RoleFields
        form={form}
        focusTitle={isNew}
        onChange={(next) => {
          setForm(next);
          onChange(roleFromForm(role.id, next));
        }}
      />
      <div class="flex justify-end">
        <button
          type="button"
          class="btn bg-destructive/60 text-white hover:bg-destructive/70"
          data-variant="destructive"
          data-size="sm"
          onClick={onRemove}
        >
          <TrashIcon />
          Remove role
        </button>
      </div>
    </div>
  );
}

/** Displays pending changes as a card. */
function RoleCard({ role, change, onPhotoChange }: {
  role: Role; change: RoleChange | undefined; onPhotoChange: (photoUrl: string | null) => void;
}) {
  return (
    <div class="flex gap-5 p-5">
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-x-2">
          <h2 class="text-lg font-semibold">{role.title}</h2>
          {change && <span class="text-xs font-medium text-sky-400">{change === 'new' ? 'New' : 'Edited'}</span>}
        </div>
        <div class="text-xs text-muted-foreground italic break-all">{role.emails.join(', ')}</div>
        {role.holders.length === 0 ? (
          <p class="mt-3 text-sm text-muted-foreground italic">This position is currently vacant.</p>
        ) : role.bio ? (
          <p class="mt-3 text-sm leading-relaxed whitespace-pre-line">{role.bio}</p>
        ) : null}
        {role.instagram.length > 0 && (
          <div class="mt-3 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
            {role.instagram.map((handle) => <span key={handle}>@{handle}</span>)}
          </div>
        )}
      </div>
      <div class="flex w-28 shrink-0 flex-col items-center text-center">
        <PhotoPicker
          name={role.holders[0]?.name ?? role.title}
          photoUrl={role.photo_url}
          onChange={onPhotoChange}
          class="size-24 text-3xl"
        />
        <div class="mt-2 text-sm font-semibold break-words"><Names role={role} /></div>
      </div>
    </div>
  );
}

function Names({ role }: { role: Role }) {
  const names = role.holders.map((h) => h.name).join(', ');
  return names ? <>{names}</> : <span class="text-muted-foreground italic">Vacant</span>;
}

function warning(role: Role): string | null {
  if (role.holders.length === 0) return 'Vacant';
  return role.bio ? null : 'No bio';
}
