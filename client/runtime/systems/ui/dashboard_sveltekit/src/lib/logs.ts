import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type DashboardLogEntry = {
  seq: number;
  ts: string;
  action: string;
  actor: string;
  agent_id: string;
};

export type DashboardAuditVerification = {
  tip_hash: string;
  chain_valid: boolean | null;
};

export async function readLogEntries(): Promise<DashboardLogEntry[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/audit/recent?n=200');
  const rows = Array.isArray(payload.entries) ? payload.entries : [];
  return rows.map((row, index) => {
    const item = asRecord(row);
    return {
      seq: Number(item.seq || index + 1) || index + 1,
      ts: String(item.ts || item.created_at || '').trim(),
      action: String(item.action || 'event').trim(),
      actor: String(item.actor || 'system').trim(),
      agent_id: String(item.agent_id || '').trim(),
    };
  });
}

export async function readAuditVerification(): Promise<DashboardAuditVerification> {
  const payload = await requestJson<JsonRecord>('GET', '/api/audit/verify');
  return {
    tip_hash: String(payload.tip_hash || '').trim(),
    chain_valid: typeof payload.chain_valid === 'boolean' ? payload.chain_valid : null,
  };
}
