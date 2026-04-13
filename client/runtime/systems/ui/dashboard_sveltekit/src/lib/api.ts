export type JsonRecord = Record<string, unknown>;

export function asRecord(value: unknown): JsonRecord {
  return value && typeof value === 'object' ? (value as JsonRecord) : {};
}

function readAuthToken(): string {
  if (typeof window === 'undefined' || !window.localStorage) return '';
  try {
    return String(window.localStorage.getItem('infring-api-key') || '').trim();
  } catch {
    return '';
  }
}

function requestHeaders(withBody: boolean): Record<string, string> {
  const headers: Record<string, string> = {};
  const token = readAuthToken();
  if (withBody) headers['Content-Type'] = 'application/json';
  if (token) headers.Authorization = `Bearer ${token}`;
  return headers;
}

export async function requestJson<T>(method: string, url: string, body?: unknown): Promise<T> {
  const response = await fetch(url, {
    method,
    cache: 'no-store',
    headers: requestHeaders(body !== undefined),
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!response.ok) {
    let message = `${method} ${url} failed`;
    try {
      const payload = asRecord(await response.json());
      message = String(payload.error || payload.message || message);
    } catch {
      message = (await response.text().catch(() => message)) || message;
    }
    throw new Error(message);
  }
  if (response.status === 204) return {} as T;
  return (await response.json()) as T;
}

export async function readJson<T>(url: string, fallback: T): Promise<T> {
  try {
    return await requestJson<T>('GET', url);
  } catch {
    return fallback;
  }
}
