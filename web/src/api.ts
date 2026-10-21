// Requests to the console's API.

/** The console refused a request. */
export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}

/** The response's JSON, or an {@link ApiError} with the console's message. */
export async function json<T>(response: Response): Promise<T> {
  if (response.ok) return response.json();
  const error = await response.json().then((body) => body.error, () => undefined);
  throw new ApiError(response.status, typeof error === 'string' ? error : `Something went wrong (HTTP ${response.status}).`);
}

/** A document the console edits whole (crates/console/src/documents.rs). */
export interface Versioned<T> {
  /** `null` if never written. */
  version: string | null;
  data: T;
}

export async function fetchDocument<T>(path: string): Promise<Versioned<T>> {
  return json(await fetch(path, { headers: { Accept: 'application/json' } }));
}

/** Returns the server's validated copy. Fails with a 409 if someone saved since `version`. */
export async function saveDocument<T>(path: string, version: string | null, data: T): Promise<Versioned<T>> {
  return json(await fetch(path, {
    method: 'PUT',
    headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: JSON.stringify({ version, data }),
  }));
}
