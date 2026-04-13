import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type DashboardApprovalRow = {
  id: string;
  status: string;
  title: string;
  summary: string;
  requested_by: string;
  created_at: string;
};

export async function readApprovals(): Promise<DashboardApprovalRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/approvals');
  const rows = Array.isArray(payload.approvals) ? payload.approvals : [];
  return rows
    .map((row) => asRecord(row))
    .filter((row) => String(row.id || '').trim())
    .map((row) => ({
      id: String(row.id || '').trim(),
      status: String(row.status || 'unknown').trim(),
      title: String(row.title || row.action || row.kind || row.id || '').trim(),
      summary: String(row.summary || row.description || row.message || '').trim(),
      requested_by: String(row.requested_by || row.agent_name || row.actor || 'system').trim(),
      created_at: String(row.created_at || row.ts || '').trim(),
    }));
}

export async function approveApproval(id: string): Promise<string> {
  await requestJson('POST', `/api/approvals/${encodeURIComponent(id)}/approve`, {});
  return 'Approval granted';
}

export async function rejectApproval(id: string): Promise<string> {
  await requestJson('POST', `/api/approvals/${encodeURIComponent(id)}/reject`, {});
  return 'Approval rejected';
}
