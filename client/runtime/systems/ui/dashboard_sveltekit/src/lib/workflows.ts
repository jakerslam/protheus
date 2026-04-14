import { asRecord, requestJson, type JsonRecord } from '$lib/api';

export type WorkflowStepInput = {
  name: string;
  agent_name: string;
  mode: string;
  prompt: string;
};

export type DashboardWorkflowRow = {
  id: string;
  name: string;
  description: string;
  steps: WorkflowStepInput[];
};

function normalizeStep(row: unknown): WorkflowStepInput {
  const item = asRecord(row);
  const agent = asRecord(item.agent);
  return {
    name: String(item.name || 'step').trim() || 'step',
    agent_name: String(item.agent_name || agent.name || '').trim(),
    mode: String(item.mode || 'sequential').trim() || 'sequential',
    prompt: String(item.prompt || item.prompt_template || '{{input}}').trim() || '{{input}}',
  };
}

function normalizeWorkflow(row: unknown): DashboardWorkflowRow {
  const item = asRecord(row);
  const rawSteps = Array.isArray(item.steps) ? item.steps : [];
  return {
    id: String(item.id || item.name || '').trim(),
    name: String(item.name || item.id || 'workflow').trim(),
    description: String(item.description || '').trim(),
    steps: rawSteps.map(normalizeStep),
  };
}

export async function readWorkflows(): Promise<DashboardWorkflowRow[]> {
  const payload = await requestJson<unknown>('GET', '/api/workflows');
  const rows = Array.isArray(payload) ? payload : (Array.isArray(asRecord(payload).workflows) ? asRecord(payload).workflows as unknown[] : []);
  return rows.map(normalizeWorkflow).filter((row) => row.id);
}

export async function readWorkflow(id: string): Promise<DashboardWorkflowRow> {
  const payload = await requestJson<unknown>('GET', `/api/workflows/${encodeURIComponent(id)}`);
  return normalizeWorkflow(payload);
}

export async function createWorkflow(input: { name: string; description: string; steps: WorkflowStepInput[] }): Promise<string> {
  await requestJson('POST', '/api/workflows', input);
  return `Created ${input.name}`;
}

export async function updateWorkflow(id: string, input: { name: string; description: string; steps: WorkflowStepInput[] }): Promise<string> {
  await requestJson('PUT', `/api/workflows/${encodeURIComponent(id)}`, input);
  return `Updated ${input.name}`;
}

export async function deleteWorkflow(id: string): Promise<string> {
  await requestJson('DELETE', `/api/workflows/${encodeURIComponent(id)}`);
  return 'Workflow deleted';
}

export async function runWorkflow(id: string, input: string): Promise<string> {
  const payload = await requestJson<JsonRecord>('POST', `/api/workflows/${encodeURIComponent(id)}/run`, { input });
  return String(payload.output || JSON.stringify(payload, null, 2)).trim();
}

export async function readWorkflowRuns(id: string): Promise<string> {
  const payload = await requestJson<unknown>('GET', `/api/workflows/${encodeURIComponent(id)}/runs`);
  return JSON.stringify(payload, null, 2);
}
