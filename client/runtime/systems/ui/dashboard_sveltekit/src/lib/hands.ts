import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type DashboardHandRequirement = {
  key: string;
  label: string;
  type: string;
  satisfied: boolean;
  message: string;
};

export type DashboardHandSetting = {
  key: string;
  label: string;
  type: string;
  description: string;
  default_value: string;
};

export type DashboardHandRow = {
  id: string;
  name: string;
  description: string;
  icon: string;
  requirements_met: boolean;
  requirements: DashboardHandRequirement[];
  settings: DashboardHandSetting[];
};

export type DashboardHandInstanceRow = {
  instance_id: string;
  hand_id: string;
  agent_id: string;
  agent_name: string;
  status: string;
  activated_at: string;
  updated_at: string;
};

function normalizeRequirement(value: unknown): DashboardHandRequirement {
  const row = asRecord(value);
  return {
    key: String(row.key || '').trim(),
    label: String(row.label || row.key || 'Requirement').trim(),
    type: String(row.type || '').trim(),
    satisfied: row.satisfied === true,
    message: String(row.message || '').trim(),
  };
}

function normalizeSetting(value: unknown): DashboardHandSetting {
  const row = asRecord(value);
  return {
    key: String(row.key || '').trim(),
    label: String(row.label || row.key || 'Setting').trim(),
    type: String(row.type || 'text').trim() || 'text',
    description: String(row.description || '').trim(),
    default_value: String(row.default_value || row.default || '').trim(),
  };
}

function normalizeHand(value: unknown): DashboardHandRow {
  const row = asRecord(value);
  return {
    id: String(row.id || '').trim(),
    name: String(row.name || row.id || '').trim(),
    description: String(row.description || '').trim(),
    icon: String(row.icon || '').trim(),
    requirements_met: row.requirements_met === true,
    requirements: (Array.isArray(row.requirements) ? row.requirements : []).map(normalizeRequirement),
    settings: (Array.isArray(row.settings) ? row.settings : []).map(normalizeSetting),
  };
}

export async function readHandsCatalog(): Promise<DashboardHandRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/hands');
  const rows = Array.isArray(payload.hands) ? payload.hands : [];
  return rows.map(normalizeHand).filter((row) => row.id);
}

export async function readActiveHands(): Promise<DashboardHandInstanceRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/hands/active');
  const rows = Array.isArray(payload.instances) ? payload.instances : [];
  return rows
    .map((value) => asRecord(value))
    .filter((row) => String(row.instance_id || '').trim())
    .map((row) => ({
      instance_id: String(row.instance_id || '').trim(),
      hand_id: String(row.hand_id || '').trim(),
      agent_id: String(row.agent_id || '').trim(),
      agent_name: String(row.agent_name || row.agent_id || '').trim(),
      status: String(row.status || '').trim(),
      activated_at: String(row.activated_at || '').trim(),
      updated_at: String(row.updated_at || '').trim(),
    }));
}

export async function readHandDetail(handId: string): Promise<DashboardHandRow> {
  const payload = await requestJson<JsonRecord>('GET', `/api/hands/${encodeURIComponent(handId)}`);
  return normalizeHand(payload);
}

export async function checkHandDependencies(handId: string): Promise<DashboardHandRow> {
  const payload = await requestJson<JsonRecord>('POST', `/api/hands/${encodeURIComponent(handId)}/check-deps`, {});
  return {
    id: handId,
    name: handId,
    description: '',
    icon: '',
    requirements_met: payload.requirements_met === true,
    requirements: (Array.isArray(payload.requirements) ? payload.requirements : []).map(normalizeRequirement),
    settings: [],
  };
}

export async function activateHand(handId: string, config: Record<string, unknown>): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', `/api/hands/${encodeURIComponent(handId)}/activate`, { config });
  return String(payload.agent_name || payload.agent_id || handId || 'Hand activated').trim();
}

export async function pauseHandInstance(instanceId: string): Promise<string> {
  await requestJson('POST', `/api/hands/instances/${encodeURIComponent(instanceId)}/pause`, {});
  return 'Instance paused';
}

export async function resumeHandInstance(instanceId: string): Promise<string> {
  await requestJson('POST', `/api/hands/instances/${encodeURIComponent(instanceId)}/resume`, {});
  return 'Instance resumed';
}

export async function deleteHandInstance(instanceId: string): Promise<string> {
  await requestJson('DELETE', `/api/hands/instances/${encodeURIComponent(instanceId)}`);
  return 'Instance removed';
}
