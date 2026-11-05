export interface JsonRequestOptions extends RequestInit {
  fetch?: typeof fetch;
}

export async function readJsonError(response: Response): Promise<Error> {
  const detail = await response.json().catch(() => null);
  return new Error(detail?.message ?? detail?.error ?? `GateKeeper request failed (${response.status})`);
}

export async function requestJson<T>(url: string, options: JsonRequestOptions = {}): Promise<T> {
  const { fetch: fetchOverride, ...init } = options;
  const fetchFn = fetchOverride ?? fetch;
  const headers = new Headers(init.headers);

  if (init.body && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  if (!headers.has('Accept')) {
    headers.set('Accept', 'application/json');
  }

  const response = await fetchFn(url, {
    credentials: 'same-origin',
    ...init,
    headers
  });

  if (!response.ok) throw await readJsonError(response);
  return response.status === 204 ? (null as T) : ((await response.json()) as T);
}
