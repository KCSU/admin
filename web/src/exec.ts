// The KCSU exec, as shown at kcsu.org.uk/about-us/exec.

interface Holder {
  name: string;
}

/** A role on the exec, which may be shared by several holders. */
export interface Role {
  /** A stable identifier, e.g. "president". */
  id: string;
  title: string;
  emails: string[];
  /** Empty while the role is vacant. */
  holders: Holder[];
  bio: string | null;
  photo_url: string | null;
  /** Handles, without the leading "@". */
  instagram: string[];
}

export interface Exec {
  /** Unix seconds. */
  updated_at: number;
  /** In display order. */
  roles: Role[];
}

export const PUBLIC_PAGE = 'https://kcsu.org.uk/about-us/exec';

/** The console refused a request. */
export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}

/** The response's JSON, or an {@link ApiError} with the console's message. */
async function json<T>(response: Response): Promise<T> {
  if (response.ok) return response.json();
  const error = await response.json().then((body) => body.error, () => undefined);
  throw new ApiError(response.status, typeof error === 'string' ? error : `Something went wrong (HTTP ${response.status}).`);
}

/** The published exec data. */
export async function fetchExec(): Promise<Exec> {
  return json(await fetch('/api/exec', { headers: { Accept: 'application/json' } }));
}

/**
 * Replaces the exec with `roles`, and returns the server-validated copy.
 * If someone else published since, this fails with a 409.
 * {@link ApiError}.
 */
export async function publishExec(updatedAt: number, roles: Role[]): Promise<Exec> {
  return json(await fetch('/api/exec', {
    method: 'PUT',
    headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: JSON.stringify({ updated_at: updatedAt, roles }),
  }));
}

/**
 * Uploads a photo, and returns its public URL.
 */
export async function uploadPhoto(file: File): Promise<string> {
  const photo = await preparePhoto(file);
  const response = await fetch('/api/exec/photos', {
    method: 'POST',
    headers: { Accept: 'application/json', 'Content-Type': photo.type },
    body: photo,
  });
  return (await json<{ url: string }>(response)).url;
}

/** Photos are shown at most 128px wide, so this is plenty even on high-DPI screens. */
const MAX_PHOTO_SIZE = 512;

/**
 * Shrinks a photo to fit in {@link MAX_PHOTO_SIZE} square and re-encodes it as WebP, which
 * also drops its metadata, such as where it was taken.
 */
async function preparePhoto(file: File): Promise<Blob> {
  const bitmap = await createImageBitmap(file, { imageOrientation: 'from-image' });
  const scale = Math.min(1, MAX_PHOTO_SIZE / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(bitmap.width * scale);
  canvas.height = Math.round(bitmap.height * scale);
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  return new Promise((resolve, reject) =>
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error('encoding the photo failed'))), 'image/webp', 0.85),
  );
}

/** An ID for a new role from its title, e.g. "Welfare Officer" -> "welfare-officer". */
export function roleId(title: string, taken: Role[]): string {
  // The console allows IDs up to 64 characters, leaving room for a suffix.
  const base = title.toLowerCase().replace(/[^a-z0-9]+/g, '-').slice(0, 60).replace(/^-|-$/g, '') || 'role';
  let id = base;
  for (let n = 2; taken.some((role) => role.id === id); n += 1) id = `${base}-${n}`;
  return id;
}
