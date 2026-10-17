// The inputs for one role.

import { useEffect, useId, useRef, useState } from 'preact/hooks';

import { ApiError, type Role, uploadPhoto } from '../exec';
import { ImageUpIcon, XIcon } from './icons';

/** A role's fields as typed, before lists are split. */
interface RoleForm {
  title: string;
  /** Comma-separated. */
  holders: string;
  /** Comma-separated. */
  emails: string;
  /** Comma-separated, with or without "@". */
  instagram: string;
  bio: string;
  photoUrl: string | null;
}

export function formFromRole(role: Role): RoleForm {
  return {
    title: role.title,
    holders: role.holders.map((h) => h.name).join(', '),
    emails: role.emails.join(', '),
    instagram: role.instagram.map((h) => `@${h}`).join(', '),
    bio: role.bio ?? '',
    photoUrl: role.photo_url,
  };
}

export function roleFromForm(id: string, form: RoleForm): Role {
  return {
    id,
    title: form.title.trim(),
    holders: list(form.holders).map((name) => ({ name })),
    emails: list(form.emails),
    instagram: list(form.instagram).map((handle) => handle.replace(/^@/, '')),
    bio: form.bio.trim() || null,
    photo_url: form.photoUrl,
  };
}

/** `focusTitle` focuses and selects the title when the fields appear, e.g. for a new role. */
export default function RoleFields({ form, focusTitle = false, onChange }: {
  form: RoleForm; focusTitle?: boolean; onChange: (form: RoleForm) => void;
}) {
  const id = useId();
  const title = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (focusTitle) title.current?.select();
  }, []);
  const set = (field: keyof RoleForm) => (e: { currentTarget: { value: string } }) =>
    onChange({ ...form, [field]: e.currentTarget.value });

  return (
    <>
      <div class="field">
        <label for={`${id}-title`} class="gap-0.5">Title<RequiredMark /></label>
        <input ref={title} id={`${id}-title`} type="text" required value={form.title} onInput={set('title')} />
      </div>
      <div class="field">
        <label for={`${id}-holders`}>Held by</label>
        <input
          id={`${id}-holders`}
          type="text"
          placeholder="Leave empty while vacant"
          value={form.holders}
          onInput={set('holders')}
        />
        <p class="-mt-1.5 text-xs text-muted-foreground">Separate several names with commas.</p>
      </div>
      <div class="field">
        <label for={`${id}-emails`} class="gap-0.5">Email<RequiredMark /></label>
        <input
          id={`${id}-emails`}
          type="text"
          required
          placeholder="president@kcsu.org.uk"
          value={form.emails}
          onInput={set('emails')}
        />
      </div>
      <div class="field">
        <label for={`${id}-instagram`}>Instagram</label>
        <input id={`${id}-instagram`} type="text" placeholder="@your_instagram_account_here" value={form.instagram} onInput={set('instagram')} />
        <p class="-mt-1.5 text-xs text-muted-foreground">Separate several handles with commas.</p>
      </div>
      <div class="field">
        <label for={`${id}-bio`}>Bio</label>
        <textarea id={`${id}-bio`} rows={5} value={form.bio} onInput={set('bio')} />
      </div>
    </>
  );
}

/**
 * Marks a field's label as mandatory.
 */
function RequiredMark() {
  return <span class="text-red-400" aria-hidden="true">*</span>;
}

/**
 * A role's avatar as a button that uploads or replaces the photo, with a button to remove it.
 */
export function PhotoPicker({ name, photoUrl, onChange, class: className = '' }: {
  name: string; photoUrl: string | null; onChange: (photoUrl: string | null) => void; class?: string;
}) {
  const fileInput = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const label = uploading ? 'Uploading…' : photoUrl ? 'Replace photo' : 'Upload photo';

  const upload = async (file: File) => {
    setUploading(true);
    try {
      onChange(await uploadPhoto(file));
    } catch (err) {
      console.error('uploading the photo failed:', err);
      alert(err instanceof ApiError ? err.message : 'The photo couldn’t be uploaded. Please try again.');
    } finally {
      setUploading(false);
    }
  };

  return (
    <div class="group relative">
      <button
        type="button"
        class="block rounded-full"
        aria-label={label}
        title={label}
        aria-busy={uploading}
        disabled={uploading}
        onClick={() => fileInput.current?.click()}
      >
        <Avatar name={name} photoUrl={photoUrl} class={className} />
        <span class={`absolute inset-0 flex items-center justify-center rounded-full bg-black/50 text-white transition-opacity ${uploading ? 'opacity-100' : 'opacity-0 group-hover:opacity-100 group-focus-within:opacity-100'}`}>
          {uploading ? <span class="size-6 animate-spin rounded-full border-2 border-white/40 border-t-white" /> : <ImageUpIcon class="size-6" />}
        </span>
      </button>
      {photoUrl && !uploading && (
        <button
          type="button"
          class="absolute top-0 right-0 rounded-full border bg-popover p-1 text-muted-foreground hover:text-red-400"
          aria-label="Remove photo"
          title="Remove photo"
          onClick={() => onChange(null)}
        >
          <XIcon class="size-3.5" />
        </button>
      )}
      <input
        ref={fileInput}
        type="file"
        accept="image/jpeg,image/png,image/webp"
        class="hidden"
        onChange={(e) => {
          const file = e.currentTarget.files?.[0];
          e.currentTarget.value = '';
          if (file) upload(file);
        }}
      />
    </div>
  );
}

/** A role's photo, or the first letter of `name` as a placeholder. */
export function Avatar({ name, photoUrl, class: className = '' }: {
  name: string; photoUrl: string | null; class?: string;
}) {
  return photoUrl ? (
    <img src={photoUrl} alt="" class={`shrink-0 rounded-full bg-muted object-cover ${className}`} />
  ) : (
    <span class={`flex shrink-0 items-center justify-center rounded-full bg-muted font-semibold text-muted-foreground ${className}`}>
      {name.trim().charAt(0).toUpperCase() || '?'}
    </span>
  );
}

/** The non-empty, trimmed entries of a comma-separated list. */
function list(value: string): string[] {
  return value.split(',').map((item) => item.trim()).filter(Boolean);
}
