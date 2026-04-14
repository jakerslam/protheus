import type { DashboardModelRow } from '$lib/chat';

export type DashboardProviderRow = {
  id: string;
  display_name: string;
  auth_status: string;
  api_key_env: string;
  base_url: string;
  is_local: boolean;
};

export type DashboardSystemInfo = {
  version: string;
  platform: string;
  arch: string;
  uptime_seconds: number;
  agent_count: number;
  default_provider: string;
  default_model: string;
};

type JsonRecord = Record<string, unknown>;

function asRecord(value: unknown): JsonRecord {
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

async function requestJson<T>(method: string, url: string, body?: unknown): Promise<T> {
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

export async function readProviders(): Promise<DashboardProviderRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/providers');
  const rows = Array.isArray(payload.providers) ? payload.providers : [];
  return rows
    .map((row) => asRecord(row))
    .filter((row) => String(row.id || '').trim())
    .map((row) => ({
      id: String(row.id || '').trim(),
      display_name: String(row.display_name || row.id || '').trim(),
      auth_status: String(row.auth_status || '').trim(),
      api_key_env: String(row.api_key_env || '').trim(),
      base_url: String(row.base_url || '').trim(),
      is_local: row.is_local === true,
    }));
}

export async function readSettingsModels(): Promise<DashboardModelRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/models');
  const rows = Array.isArray(payload.models) ? payload.models : [];
  return rows
    .map((row) => asRecord(row))
    .filter((row) => String(row.id || row.display_name || '').trim())
    .map((row) => ({
      id: String(row.id || row.display_name || '').trim(),
      provider: String(row.provider || '').trim(),
      display_name: String(row.display_name || row.id || '').trim(),
      local: row.local === true || row.is_local === true,
    }));
}

export async function readSystemInfo(): Promise<DashboardSystemInfo> {
  const [version, status] = await Promise.all([
    requestJson<JsonRecord>('GET', '/api/version'),
    requestJson<JsonRecord>('GET', '/api/status'),
  ]);
  return {
    version: String(version.version || '-').trim() || '-',
    platform: String(version.platform || '-').trim() || '-',
    arch: String(version.arch || '-').trim() || '-',
    uptime_seconds: Number(status.uptime_seconds || 0) || 0,
    agent_count: Number(status.agent_count || 0) || 0,
    default_provider: String(status.default_provider || '-').trim() || '-',
    default_model: String(status.default_model || '-').trim() || '-',
  };
}

export async function saveProviderKey(providerId: string, key: string): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', `/api/providers/${encodeURIComponent(providerId)}/key`, { key });
  return String(payload.message || `API key saved for ${providerId}`).trim();
}

export async function removeProviderKey(providerId: string): Promise<string> {
  await requestJson('DELETE', `/api/providers/${encodeURIComponent(providerId)}/key`);
  return `API key removed for ${providerId}`;
}

export async function testProvider(providerId: string): Promise<{ status: string; latency_ms: number; error: string }> {
  const payload = await requestJson<JsonRecord>('POST', `/api/providers/${encodeURIComponent(providerId)}/test`, {});
  return {
    status: String(payload.status || 'unknown').trim(),
    latency_ms: Number(payload.latency_ms || 0) || 0,
    error: String(payload.error || '').trim(),
  };
}

export async function saveProviderUrl(providerId: string, baseUrl: string): Promise<string> {
  const payload = await requestJson<JsonRecord>('PUT', `/api/providers/${encodeURIComponent(providerId)}/url`, { base_url: baseUrl });
  return payload.reachable === true ? `${providerId} URL saved and reachable` : `${providerId} URL saved`;
}

export async function addCustomModel(model: {
  id: string;
  provider: string;
  context_window: number;
  max_output_tokens: number;
}): Promise<string> {
  await requestJson('POST', '/api/models/custom', model);
  return `Added ${model.id}`;
}

export async function deleteCustomModel(modelId: string): Promise<string> {
  await requestJson('DELETE', `/api/models/custom/${encodeURIComponent(modelId)}`);
  return `Deleted ${modelId}`;
}
