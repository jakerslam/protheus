export type DashboardAgentRow = {
  id: string;
  name?: string;
  state?: string;
  runtime_model?: string;
  model_name?: string;
  model_provider?: string;
  created_at?: string;
  updated_at?: string;
  last_activity_at?: string;
  archived?: boolean;
  draft?: boolean;
  identity?: {
    emoji?: string;
  };
};

export type DashboardChatToolRow = {
  id: string;
  name: string;
  input: string;
  result: string;
  status: string;
  isError: boolean;
  blocked: boolean;
};

export type DashboardChatMessageRole = 'user' | 'agent' | 'system' | 'terminal';

export type DashboardChatMessage = {
  id: string;
  role: DashboardChatMessageRole;
  text: string;
  meta: string;
  ts: number;
  tools: DashboardChatToolRow[];
  pending?: boolean;
};

export type DashboardChatSession = {
  messages: DashboardChatMessage[];
  raw: unknown;
};

type JsonRecord = Record<string, unknown>;

function asRecord(value: unknown): JsonRecord {
  return value && typeof value === 'object' ? (value as JsonRecord) : {};
}

function textValue(value: unknown, limit = 24000): string {
  if (typeof value === 'string') return value.slice(0, limit);
  if (value == null) return '';
  try {
    return JSON.stringify(value).slice(0, limit);
  } catch {
    return String(value).slice(0, limit);
  }
}

function placeholderText(value: string): boolean {
  const normalized = String(value || '').trim().toLowerCase();
  if (!normalized) return true;
  return (
    normalized.includes("i don't have usable tool findings") ||
    normalized.includes('no useful comparison findings') ||
    normalized.includes('retry with a narrower query') ||
    normalized.includes('no usable findings were extracted in this turn')
  );
}

function structuredBlocks(payload: JsonRecord): unknown[] {
  const blocks: unknown[] = [];
  const push = (value: unknown) => {
    if (Array.isArray(value)) blocks.push(...value);
  };
  push(payload.content);
  push(payload.response);
  const message = asRecord(payload.message);
  push(message.content);
  return blocks;
}

function workflowResponseTextFromPayload(payload: JsonRecord): string {
  const workflow = asRecord(payload.response_workflow);
  const finalResponse = asRecord(workflow.final_llm_response);
  const status = String(finalResponse.status || '').trim().toLowerCase();
  const response = String(workflow.response || '').trim();
  if (status !== 'synthesized' || !response || placeholderText(response)) return '';
  return response;
}

function assistantTextFromPayload(payload: JsonRecord): string {
  const workflowText = workflowResponseTextFromPayload(payload);
  if (workflowText) return workflowText;
  if (typeof payload.response === 'string') return String(payload.response || '');
  if (typeof payload.content === 'string') return String(payload.content || '');
  const parts: string[] = [];
  for (const block of structuredBlocks(payload)) {
    if (typeof block === 'string') {
      if (block.trim()) parts.push(block);
      continue;
    }
    const row = asRecord(block);
    const type = String(row.type || '').trim().toLowerCase();
    if (type === 'toolcall' || type === 'tool_call' || type === 'tooluse' || type === 'tool_use') continue;
    if (type === 'toolresult' || type === 'tool_result' || type === 'tool_result_error') continue;
    const text = typeof row.text === 'string' ? row.text : (typeof row.content === 'string' ? row.content : '');
    if (String(text || '').trim()) parts.push(String(text));
  }
  return parts.join('\n\n').trim();
}

function normalizeToolRows(payload: JsonRecord, prefix: string): DashboardChatToolRow[] {
  const rows = Array.isArray(payload.tools) ? payload.tools : [];
  return rows.slice(0, 12).map((tool, index) => {
    const item = asRecord(tool);
    const name = String(item.name || item.tool || 'tool').trim() || 'tool';
    return {
      id: String(item.id || `${prefix}-${name}-${index + 1}`),
      name,
      input: textValue(item.input ?? item.arguments ?? item.args ?? '', 16000),
      result: textValue(item.result ?? item.output ?? item.summary ?? '', 24000),
      status: String(item.status || '').trim().toLowerCase(),
      isError: Boolean(item.is_error || item.error),
      blocked: Boolean(item.blocked),
    };
  });
}

function fallbackAssistantTextFromPayload(payload: JsonRecord, tools: DashboardChatToolRow[]): string {
  const finalization = asRecord(payload.response_finalization);
  const failureSummary = String(finalization.failure_summary || '').trim();
  if (failureSummary) return failureSummary;
  const completion = asRecord(finalization.tool_completion);
  const completionSummary = String(completion.summary || '').trim();
  if (completionSummary) return completionSummary;
  if (!tools.length) return '';
  return tools
    .slice(0, 3)
    .map((tool) => {
      const tail = String(tool.result || tool.status || '').trim();
      return tail ? `${tool.name}: ${tail}` : tool.name;
    })
    .join('\n');
}

function normalizeRole(payload: JsonRecord): DashboardChatMessageRole {
  const roleRaw = String(payload.role || payload.type || '').trim().toLowerCase();
  if (roleRaw.includes('terminal') || payload.terminal === true) return 'terminal';
  if (roleRaw.includes('user')) return 'user';
  if (roleRaw.includes('system')) return 'system';
  return 'agent';
}

function normalizeMessage(payload: JsonRecord, index: number): DashboardChatMessage | null {
  const role = normalizeRole(payload);
  let textSource = payload.content ?? payload.text ?? payload.message;
  if (role === 'user' && payload.user != null) textSource = payload.user;
  if (role !== 'user' && role !== 'terminal' && payload.assistant != null) textSource = payload.assistant;
  if (role !== 'user' && role !== 'terminal') {
    const assistantText = assistantTextFromPayload(payload);
    if (assistantText || typeof textSource !== 'string') textSource = assistantText;
  }
  const tools = normalizeToolRows(payload, `hist-tool-${index + 1}`);
  let text = typeof textSource === 'string' ? textSource : textValue(textSource, 24000);
  text = String(text || '').replace(/\r\n/g, '\n').replace(/\r/g, '\n').trim();
  if (role === 'agent' && (!text || placeholderText(text))) {
    text = String(fallbackAssistantTextFromPayload(payload, tools) || '').trim();
  }
  if (!text && !tools.length && role !== 'user') return null;
  const ts = Number(payload.ts || Date.now());
  const meta = String(payload.meta || payload.notice_label || '').trim();
  return {
    id: String(payload.id || `${role}-${index + 1}-${ts}`),
    role,
    text,
    meta,
    ts: Number.isFinite(ts) ? ts : Date.now(),
    tools,
  };
}

function normalizeSessionMessages(payload: JsonRecord): DashboardChatMessage[] {
  const source = Array.isArray(payload.messages)
    ? payload.messages
    : (Array.isArray(payload.turns) ? payload.turns : []);
  return source
    .map((entry, index) => normalizeMessage(asRecord(entry), index))
    .filter((entry): entry is DashboardChatMessage => Boolean(entry));
}

async function requestJson<T>(method: string, url: string, body?: unknown): Promise<T> {
  const response = await fetch(url, {
    method,
    cache: 'no-store',
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
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
  return (await response.json()) as T;
}

export async function readSidebarAgents(): Promise<DashboardAgentRow[]> {
  const payload = await requestJson<unknown>('GET', '/api/agents?view=sidebar&authority=runtime');
  return Array.isArray(payload)
    ? payload
        .map((row) => asRecord(row))
        .filter((row) => String(row.id || '').trim())
        .map((row) => ({
          id: String(row.id || '').trim(),
          name: String(row.name || row.id || '').trim(),
          state: String(row.state || '').trim(),
          runtime_model: String(row.runtime_model || '').trim(),
          model_name: String(row.model_name || '').trim(),
          model_provider: String(row.model_provider || '').trim(),
          created_at: String(row.created_at || '').trim(),
          updated_at: String(row.updated_at || '').trim(),
          last_activity_at: String(row.last_activity_at || row.ts || '').trim(),
          archived: row.archived === true,
          draft: row.draft === true,
          identity: asRecord(row.identity) as DashboardAgentRow['identity'],
        }))
    : [];
}

export async function readAgentSession(agentId: string): Promise<DashboardChatSession> {
  const payload = await requestJson<JsonRecord>('GET', `/api/agents/${encodeURIComponent(agentId)}/session`);
  return {
    messages: normalizeSessionMessages(payload),
    raw: payload,
  };
}

export async function sendAgentMessage(agentId: string, message: string): Promise<void> {
  await requestJson<JsonRecord>('POST', `/api/agents/${encodeURIComponent(agentId)}/message`, {
    message,
  });
}

export async function createDraftAgent(): Promise<DashboardAgentRow> {
  const payload = await requestJson<JsonRecord>('POST', '/api/agents', {
    role: 'analyst',
    contract: {
      mission: 'Fresh chat initialization',
      termination_condition: 'task_or_timeout',
      expiry_seconds: 3600,
      auto_terminate_allowed: false,
      idle_terminate_allowed: false,
      conversation_hold: true,
    },
  });
  const createdId = String(payload.id || payload.agent_id || '').trim();
  if (!createdId) throw new Error('spawn_failed');
  return {
    id: createdId,
    name: String(payload.name || payload.id || payload.agent_id || '').trim() || createdId,
    state: String(payload.state || 'running').trim(),
    runtime_model: String(payload.runtime_model || '').trim(),
    model_name: String(payload.model_name || '').trim(),
    model_provider: String(payload.model_provider || '').trim(),
    created_at: String(payload.created_at || new Date().toISOString()),
    draft: true,
    identity: { emoji: '∞' },
  };
}
