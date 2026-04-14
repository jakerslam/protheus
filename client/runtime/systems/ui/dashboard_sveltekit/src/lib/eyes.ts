import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type DashboardEyeRow = {
  name: string;
  status: string;
  endpoint_url: string;
  endpoint_host: string;
  api_key_present: boolean;
  updated_at: string;
};

export async function readEyes(): Promise<DashboardEyeRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/eyes');
  const rows = Array.isArray(payload.eyes) ? payload.eyes : [];
  return rows.map((row) => {
    const item = asRecord(row);
    return {
      name: String(item.name || '').trim(),
      status: String(item.status || 'active').trim(),
      endpoint_url: String(item.endpoint_url || '').trim(),
      endpoint_host: String(item.endpoint_host || '').trim(),
      api_key_present: item.api_key_present === true,
      updated_at: String(item.updated_at || '').trim(),
    };
  });
}

export async function saveEye(input: {
  name: string;
  status: string;
  url: string;
  api_key: string;
  cadence_hours: number;
  topics: string;
}): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', '/api/eyes', input);
  const eye = asRecord(payload.eye);
  return `${payload.created ? 'Added' : 'Updated'} ${String(eye.name || input.name || 'eye').trim()}`;
}
