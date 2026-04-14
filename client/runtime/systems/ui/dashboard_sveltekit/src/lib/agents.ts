import type { DashboardAgentRow } from '$lib/chat';

export type DashboardTerminatedAgentRow = {
  agent_id: string;
  agent_name: string;
  role: string;
  contract_id: string;
  termination_reason: string;
  terminated_at: string;
};

export type DashboardTemplateRow = {
  name: string;
  description: string;
  category: string;
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

export async function readTerminatedAgents(): Promise<DashboardTerminatedAgentRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/agents/terminated');
  const rows = Array.isArray(payload.entries) ? payload.entries : [];
  return rows
    .map((row) => asRecord(row))
    .filter((row) => String(row.agent_id || '').trim())
    .map((row) => ({
      agent_id: String(row.agent_id || '').trim(),
      agent_name: String(row.agent_name || row.name || row.agent_id || '').trim(),
      role: String(row.role || 'analyst').trim(),
      contract_id: String(row.contract_id || '').trim(),
      termination_reason: String(row.termination_reason || row.reason || 'terminated').trim(),
      terminated_at: String(row.terminated_at || '').trim(),
    }));
}

export async function archiveAgent(agent: DashboardAgentRow): Promise<string> {
  try {
    await requestJson('DELETE', `/api/agents/${encodeURIComponent(agent.id)}`);
    return `Archived ${String(agent.name || agent.id)}`;
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause || 'archive_failed');
    if (message.includes('agent_not_found')) return `Removed stale agent ${String(agent.name || agent.id)}`;
    throw cause;
  }
}

export async function clearAgentHistory(agentId: string): Promise<string> {
  await requestJson('DELETE', `/api/agents/${encodeURIComponent(agentId)}/history`);
  return 'History cleared';
}

export async function cloneAgent(agent: DashboardAgentRow, newName?: string): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', `/api/agents/${encodeURIComponent(agent.id)}/clone`, {
    new_name: String(newName || `${agent.name || agent.id}-copy`).trim(),
  });
  return String(payload.name || payload.agent_id || 'Clone created').trim();
}

export async function reviveTerminatedAgent(entry: DashboardTerminatedAgentRow): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', `/api/agents/${encodeURIComponent(entry.agent_id)}/revive`, {
    role: entry.role || 'analyst',
  });
  return String(payload.agent_id || entry.agent_id || 'Agent revived').trim();
}

export async function deleteTerminatedAgent(entry: DashboardTerminatedAgentRow): Promise<string> {
  let url = `/api/agents/terminated/${encodeURIComponent(entry.agent_id)}`;
  if (entry.contract_id) url += `?contract_id=${encodeURIComponent(entry.contract_id)}`;
  const payload = await requestJson<JsonRecord>('DELETE', url);
  const removed = Number(payload.removed_history_entries || 0);
  return removed > 0 ? `Deleted ${entry.agent_id} and ${removed} archived record(s)` : `Deleted ${entry.agent_id}`;
}

export async function readTemplates(): Promise<DashboardTemplateRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/templates');
  const rows = Array.isArray(payload.templates) ? payload.templates : [];
  return rows
    .map((row) => asRecord(row))
    .filter((row) => String(row.name || '').trim())
    .map((row) => ({
      name: String(row.name || '').trim(),
      description: String(row.description || '').trim(),
      category: String(row.category || 'General').trim() || 'General',
    }));
}

export async function spawnTemplateAgent(templateName: string): Promise<string> {
  const template = await requestJson<JsonRecord>('GET', `/api/templates/${encodeURIComponent(templateName)}`);
  const manifestToml = String(template.manifest_toml || '').trim();
  if (!manifestToml) throw new Error('template_manifest_missing');
  const payload = await requestJson<JsonRecord>('POST', '/api/agents', { manifest_toml: manifestToml });
  return String(payload.agent_id || payload.name || templateName).trim();
}
