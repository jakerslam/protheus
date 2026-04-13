import { requestJson, type JsonRecord, asRecord } from '$lib/api';
import { readSidebarAgents } from '$lib/chat';

export type DashboardSessionRow = {
  session_id: string;
  agent_id: string;
  agent_name: string;
  updated_at: string;
};

export type DashboardMemoryKvRow = {
  key: string;
  value: unknown;
};

export async function readSessions(): Promise<DashboardSessionRow[]> {
  const [payload, agents] = await Promise.all([
    requestJson<JsonRecord>('GET', '/api/sessions'),
    readSidebarAgents().catch(() => []),
  ]);
  const agentMap = new Map(agents.map((row) => [row.id, String(row.name || row.id || '').trim()]));
  const rows = Array.isArray(payload.sessions) ? payload.sessions : [];
  return rows
    .map((row) => asRecord(row))
    .filter((row) => String(row.session_id || '').trim())
    .map((row) => ({
      session_id: String(row.session_id || '').trim(),
      agent_id: String(row.agent_id || '').trim(),
      agent_name: agentMap.get(String(row.agent_id || '').trim()) || String(row.agent_name || row.agent_id || '').trim(),
      updated_at: String(row.updated_at || row.created_at || '').trim(),
    }));
}

export async function deleteSession(sessionId: string): Promise<string> {
  await requestJson('DELETE', `/api/sessions/${encodeURIComponent(sessionId)}`);
  return 'Session deleted';
}

export async function readAgentMemoryKv(agentId: string): Promise<DashboardMemoryKvRow[]> {
  const payload = await requestJson<JsonRecord>('GET', `/api/memory/agents/${encodeURIComponent(agentId)}/kv`);
  const rows = Array.isArray(payload.kv_pairs) ? payload.kv_pairs : [];
  return rows.map((row) => {
    const item = asRecord(row);
    return {
      key: String(item.key || '').trim(),
      value: item.value,
    };
  });
}

export async function upsertAgentMemoryKv(agentId: string, key: string, value: unknown): Promise<string> {
  await requestJson('PUT', `/api/memory/agents/${encodeURIComponent(agentId)}/kv/${encodeURIComponent(key)}`, { value });
  return `Saved ${key}`;
}

export async function deleteAgentMemoryKv(agentId: string, key: string): Promise<string> {
  await requestJson('DELETE', `/api/memory/agents/${encodeURIComponent(agentId)}/kv/${encodeURIComponent(key)}`);
  return `Deleted ${key}`;
}
