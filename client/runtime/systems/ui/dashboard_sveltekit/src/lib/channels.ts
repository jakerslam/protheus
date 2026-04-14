import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type DashboardChannelField = {
  key: string;
  label: string;
  type: string;
  required: boolean;
  advanced: boolean;
  placeholder: string;
  value: string;
};

export type DashboardChannelRow = {
  name: string;
  display_name: string;
  description: string;
  quick_setup: string;
  difficulty: string;
  setup_time: string;
  icon: string;
  configured: boolean;
  has_token: boolean;
  connected: boolean;
  setup_type: string;
  channel_tier: string;
  real_channel: boolean;
  fields: DashboardChannelField[];
  setup_steps: string[];
};

export type DashboardWhatsappQrState = {
  available: boolean;
  connected: boolean;
  expired: boolean;
  session_id: string;
  qr_data_url: string;
  message: string;
  help: string;
};

function normalizeField(value: unknown): DashboardChannelField {
  const row = asRecord(value);
  return {
    key: String(row.key || '').trim(),
    label: String(row.label || row.key || 'Field').trim(),
    type: String(row.type || 'text').trim() || 'text',
    required: row.required === true,
    advanced: row.advanced === true,
    placeholder: String(row.placeholder || '').trim(),
    value: String(row.value || '').trim(),
  };
}

function normalizeQrState(payload: JsonRecord): DashboardWhatsappQrState {
  return {
    available: payload.available === true,
    connected: payload.connected === true,
    expired: payload.expired === true,
    session_id: String(payload.session_id || '').trim(),
    qr_data_url: String(payload.qr_data_url || '').trim(),
    message: String(payload.message || '').trim(),
    help: String(payload.help || '').trim(),
  };
}

export async function readChannels(): Promise<DashboardChannelRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/channels');
  const rows = Array.isArray(payload.channels) ? payload.channels : [];
  return rows
    .map((value) => asRecord(value))
    .filter((row) => String(row.name || '').trim())
    .map((row) => {
      const configured = row.configured === true;
      const hasToken = row.has_token === true;
      return {
        name: String(row.name || '').trim(),
        display_name: String(row.display_name || row.name || '').trim(),
        description: String(row.description || '').trim(),
        quick_setup: String(row.quick_setup || '').trim(),
        difficulty: String(row.difficulty || 'Medium').trim(),
        setup_time: String(row.setup_time || '').trim(),
        icon: String(row.icon || '').trim(),
        configured,
        has_token: hasToken,
        connected: row.connected === true || (configured && hasToken),
        setup_type: String(row.setup_type || '').trim(),
        channel_tier: String(row.channel_tier || '').trim(),
        real_channel: row.real_channel === true,
        fields: (Array.isArray(row.fields) ? row.fields : []).map(normalizeField),
        setup_steps: (Array.isArray(row.setup_steps) ? row.setup_steps : []).map((step) => String(step || '').trim()).filter(Boolean),
      };
    });
}

export async function configureChannel(name: string, fields: Record<string, string>): Promise<string> {
  await requestJson('POST', `/api/channels/${encodeURIComponent(name)}/configure`, { fields });
  return 'Channel saved';
}

export async function testChannel(name: string): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', `/api/channels/${encodeURIComponent(name)}/test`, { force_live: true });
  return String(payload.message || (payload.status === 'ok' ? 'Connection verified' : 'Connection test complete')).trim();
}

export async function removeChannelConfig(name: string): Promise<string> {
  await requestJson('DELETE', `/api/channels/${encodeURIComponent(name)}/configure`);
  return 'Channel removed';
}

export async function startWhatsappQr(): Promise<DashboardWhatsappQrState> {
  const payload = await requestJson<JsonRecord>('POST', '/api/channels/whatsapp/qr/start', {});
  return normalizeQrState(payload);
}

export async function readWhatsappQrStatus(sessionId: string): Promise<DashboardWhatsappQrState> {
  const payload = await requestJson<JsonRecord>('GET', `/api/channels/whatsapp/qr/status?session_id=${encodeURIComponent(sessionId)}`);
  return normalizeQrState(payload);
}
