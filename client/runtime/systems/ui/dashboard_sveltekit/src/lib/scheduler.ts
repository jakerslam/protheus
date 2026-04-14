import { requestJson, type JsonRecord, asRecord } from '$lib/api';

export type DashboardCronJobRow = {
  id: string;
  name: string;
  cron: string;
  agent_id: string;
  message: string;
  enabled: boolean;
  last_run: string;
  next_run: string;
};

export type DashboardTriggerRow = {
  id: string;
  pattern: unknown;
  enabled: boolean;
  fire_count: number;
  created_at: string;
};

export async function readCronJobs(): Promise<DashboardCronJobRow[]> {
  const payload = await requestJson<JsonRecord>('GET', '/api/cron/jobs');
  const rows = Array.isArray(payload.jobs) ? payload.jobs : [];
  return rows.map((row) => {
    const item = asRecord(row);
    const schedule = asRecord(item.schedule);
    const action = asRecord(item.action);
    return {
      id: String(item.id || '').trim(),
      name: String(item.name || '').trim(),
      cron: schedule.kind === 'cron' ? String(schedule.expr || '').trim() : String(schedule.kind || '').trim(),
      agent_id: String(item.agent_id || '').trim(),
      message: String(action.message || '').trim(),
      enabled: item.enabled === true,
      last_run: String(item.last_run || '').trim(),
      next_run: String(item.next_run || '').trim(),
    };
  });
}

export async function readTriggers(): Promise<DashboardTriggerRow[]> {
  const payload = await requestJson<unknown>('GET', '/api/triggers');
  const rows = Array.isArray(payload) ? payload : [];
  return rows.map((row) => {
    const item = asRecord(row);
    return {
      id: String(item.id || '').trim(),
      pattern: item.pattern,
      enabled: item.enabled !== false,
      fire_count: Number(item.fire_count || 0) || 0,
      created_at: String(item.created_at || '').trim(),
    };
  });
}

export async function createCronJob(input: { agent_id: string; name: string; cron: string; message: string; enabled: boolean }): Promise<string> {
  await requestJson('POST', '/api/cron/jobs', {
    agent_id: input.agent_id,
    name: input.name,
    schedule: { kind: 'cron', expr: input.cron },
    action: { kind: 'agent_turn', message: input.message || `Scheduled task: ${input.name}` },
    delivery: { kind: 'last_channel' },
    enabled: input.enabled,
  });
  return `Created ${input.name}`;
}

export async function setCronJobEnabled(jobId: string, enabled: boolean): Promise<string> {
  await requestJson('PUT', `/api/cron/jobs/${encodeURIComponent(jobId)}/enable`, { enabled });
  return enabled ? 'Schedule enabled' : 'Schedule paused';
}

export async function deleteCronJob(jobId: string): Promise<string> {
  await requestJson('DELETE', `/api/cron/jobs/${encodeURIComponent(jobId)}`);
  return 'Schedule deleted';
}

export async function runCronJobNow(jobId: string): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', `/api/schedules/${encodeURIComponent(jobId)}/run`, {});
  return payload.status === 'completed' ? 'Schedule executed' : String(payload.error || 'Schedule run failed');
}

export async function setTriggerEnabled(triggerId: string, enabled: boolean): Promise<string> {
  await requestJson('PUT', `/api/triggers/${encodeURIComponent(triggerId)}`, { enabled });
  return enabled ? 'Trigger enabled' : 'Trigger disabled';
}

export async function deleteTrigger(triggerId: string): Promise<string> {
  await requestJson('DELETE', `/api/triggers/${encodeURIComponent(triggerId)}`);
  return 'Trigger deleted';
}
