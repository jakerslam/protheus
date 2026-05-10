#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { ShellSocketGatewayClient } from '../socket/client/shell_socket_gateway_client.ts';

const DEFAULT_GATEWAY_URL = 'http://127.0.0.1:5173';
const DEFAULT_OUT_JSON = 'core/local/artifacts/browser_shell_v2_smoke_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BROWSER_SHELL_V2_SMOKE_CURRENT.md';
const MESSAGE_WINDOW_LIMIT = 40;

type FetchImpl = (input: string, init?: Record<string, unknown>) => Promise<any>;

export type BrowserShellV2MessageRow = {
  id: string;
  role: string;
  text: string;
  status?: string;
  timestamp?: string;
  detail_ref?: string;
};

export type BrowserShellV2AgentRow = {
  id: string;
  label: string;
  state?: string;
};

export type BrowserShellV2SessionRow = {
  id: string;
  label: string;
  message_count?: number;
};

export type BrowserShellV2EventRow = {
  id: string;
  label: string;
  status?: string;
  cursor?: string;
};

export type BrowserShellV2SearchRow = {
  id: string;
  label: string;
  snippet?: string;
  detail_ref?: string;
};

export type BrowserShellV2Snapshot = {
  ok: boolean;
  runtime_state: string;
  runtime_label: string;
  selected_agent_id: string;
  selected_session_id: string;
  agent_count: number;
  agent_rows: BrowserShellV2AgentRow[];
  session_rows: BrowserShellV2SessionRow[];
  message_count: number;
  visible_rows: BrowserShellV2MessageRow[];
  event_rows: BrowserShellV2EventRow[];
  event_cursor?: string;
  search_query?: string;
  search_rows: BrowserShellV2SearchRow[];
  active_detail_ref?: string;
  active_detail_preview?: string;
  issue_status?: string;
  issue_receipt_ref?: string;
  approval_status?: string;
  approval_receipt_ref?: string;
  model_status?: string;
  model_receipt_ref?: string;
  git_tree_status?: string;
  git_tree_receipt_ref?: string;
  receipt_refs: string[];
};

export type BrowserShellV2SmokeResult = BrowserShellV2Snapshot & {
  type: 'browser_shell_v2_socket_smoke';
  mode: 'fixture' | 'live';
  base_url: string;
  submitted: boolean;
  error?: string;
};

function clean(value: unknown, max = 1000): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function readFlag(argv: string[], name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  for (let index = 0; index < argv.length; index += 1) {
    const token = clean(argv[index], 1200);
    if (token === `--${name}`) return clean(argv[index + 1], 1200);
    if (token.startsWith(prefix)) return clean(token.slice(prefix.length), 1200);
  }
  return fallback;
}

function parseBool(value: string, fallback = false): boolean {
  const normalized = clean(value, 32).toLowerCase();
  if (!normalized) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(normalized);
}

function writeJson(filePath: string, payload: unknown): void {
  const abs = path.resolve(process.cwd(), filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeMarkdown(filePath: string, result: BrowserShellV2SmokeResult): void {
  const lines = [
    '# Browser Shell V2 Socket Smoke',
    '',
    `ok: \`${result.ok}\``,
    `mode: \`${result.mode}\``,
    `base_url: \`${result.base_url}\``,
    `runtime_state: \`${result.runtime_state}\``,
    `selected_agent_id: \`${result.selected_agent_id}\``,
    `selected_session_id: \`${result.selected_session_id}\``,
    `agent_count: \`${result.agent_count}\``,
    `message_count: \`${result.message_count}\``,
    `submitted: \`${result.submitted}\``,
  ];
  if (result.error) lines.push(`error: \`${result.error}\``);
  const abs = path.resolve(process.cwd(), filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${lines.join('\n')}\n`, 'utf8');
}

function rowsFromMessageWindow(payload: any): BrowserShellV2MessageRow[] {
  const rows = Array.isArray(payload?.message_window?.rows) ? payload.message_window.rows : [];
  return rows.slice(0, MESSAGE_WINDOW_LIMIT).map((row: any, index: number) => ({
    id: clean(row?.id || `row-${index}`, 160),
    role: clean(row?.role || 'assistant', 40),
    text: clean(row?.text || row?.preview || '', 12000),
    status: clean(row?.status || '', 80) || undefined,
    timestamp: clean(row?.timestamp || row?.ts || '', 80) || undefined,
    detail_ref: clean(row?.detail_ref || row?.detailRef || '', 240) || undefined,
  }));
}

function rowsFromAgents(payload: any): BrowserShellV2AgentRow[] {
  const agents = Array.isArray(payload?.agents) ? payload.agents : [];
  const agentIds = Array.isArray(payload?.agent_ids) ? payload.agent_ids : [];
  if (agents.length) {
    return agents.slice(0, 40).map((agent: any, index: number) => {
      const id = clean(agent?.id || agent?.agent_id || agentIds[index] || `agent-${index}`, 160);
      return {
        id,
        label: clean(agent?.label || agent?.name || id, 160),
        state: clean(agent?.state || agent?.status || '', 80) || undefined,
      };
    }).filter((agent: BrowserShellV2AgentRow) => Boolean(agent.id));
  }
  return agentIds.slice(0, 40).map((id: unknown) => {
    const cleanId = clean(id, 160);
    return { id: cleanId, label: cleanId };
  }).filter((agent: BrowserShellV2AgentRow) => Boolean(agent.id));
}

function rowsFromSessions(payload: any): BrowserShellV2SessionRow[] {
  const sessions = Array.isArray(payload?.sessions) ? payload.sessions : [];
  const sessionIds = Array.isArray(payload?.session_ids) ? payload.session_ids : [];
  if (sessions.length) {
    return sessions.slice(0, 40).map((session: any, index: number) => {
      const id = clean(session?.id || session?.session_id || sessionIds[index] || `session-${index}`, 240);
      const count = Number(session?.message_count || session?.messageCount || 0);
      return {
        id,
        label: clean(session?.label || session?.title || id, 160),
        message_count: Number.isFinite(count) && count > 0 ? count : undefined,
      };
    }).filter((session: BrowserShellV2SessionRow) => Boolean(session.id));
  }
  return sessionIds.slice(0, 40).map((id: unknown) => {
    const cleanId = clean(id, 240);
    return { id: cleanId, label: cleanId };
  }).filter((session: BrowserShellV2SessionRow) => Boolean(session.id));
}

function rowsFromEvents(payload: any): BrowserShellV2EventRow[] {
  const rows = Array.isArray(payload?.events) ? payload.events : Array.isArray(payload?.rows) ? payload.rows : [];
  return rows.slice(0, 20).map((row: any, index: number) => ({
    id: clean(row?.id || row?.event_id || `event-${index}`, 160),
    label: clean(row?.label || row?.summary || row?.type || row?.message || 'Event projection', 240),
    status: clean(row?.status || row?.state || '', 80) || undefined,
    cursor: clean(row?.cursor || row?.event_cursor || '', 160) || undefined,
  })).filter((event: BrowserShellV2EventRow) => Boolean(event.id));
}

function rowsFromSearch(payload: any): BrowserShellV2SearchRow[] {
  const rows = Array.isArray(payload?.results) ? payload.results : Array.isArray(payload?.rows) ? payload.rows : [];
  return rows.slice(0, 20).map((row: any, index: number) => ({
    id: clean(row?.id || row?.result_id || `search-${index}`, 160),
    label: clean(row?.label || row?.title || row?.summary || 'Search result', 240),
    snippet: clean(row?.snippet || row?.preview || row?.text || '', 500) || undefined,
    detail_ref: clean(row?.detail_ref || row?.detailRef || '', 240) || undefined,
  })).filter((result: BrowserShellV2SearchRow) => Boolean(result.id));
}

function firstAgentId(payload: any): string {
  if (typeof payload?.active_agent_id === 'string' && payload.active_agent_id.trim()) return clean(payload.active_agent_id, 160);
  if (Array.isArray(payload?.agent_ids) && payload.agent_ids.length) return clean(payload.agent_ids[0], 160);
  if (Array.isArray(payload?.agents) && payload.agents.length) return clean(payload.agents[0]?.id || payload.agents[0]?.agent_id, 160);
  return '';
}

function firstSessionId(payload: any): string {
  if (typeof payload?.active_session_id === 'string' && payload.active_session_id.trim()) return clean(payload.active_session_id, 240);
  if (Array.isArray(payload?.session_ids) && payload.session_ids.length) return clean(payload.session_ids[0], 240);
  if (Array.isArray(payload?.sessions) && payload.sessions.length) return clean(payload.sessions[0]?.id || payload.sessions[0]?.session_id, 240);
  return '';
}

function collectReceiptRefs(...payloads: any[]): string[] {
  const refs = new Set<string>();
  for (const payload of payloads) {
    const ref = clean(payload?.receipt_ref || payload?.receiptRef || '', 300);
    if (ref) refs.add(ref);
  }
  return Array.from(refs);
}

export class BrowserShellV2 {
  private readonly client: ShellSocketGatewayClient;
  private inputBuffer = '';
  private selectedAgentId = '';
  private selectedSessionId = '';
  private agentRows: BrowserShellV2AgentRow[] = [];
  private sessionRows: BrowserShellV2SessionRow[] = [];
  private visibleRows: BrowserShellV2MessageRow[] = [];
  private eventRows: BrowserShellV2EventRow[] = [];
  private eventCursor = '';
  private searchQuery = '';
  private searchRows: BrowserShellV2SearchRow[] = [];
  private activeDetailRef = '';
  private activeDetailPreview = '';
  private issueStatus = '';
  private issueReceiptRef = '';
  private approvalStatus = '';
  private approvalReceiptRef = '';
  private modelStatus = '';
  private modelReceiptRef = '';
  private gitTreeStatus = '';
  private gitTreeReceiptRef = '';
  private lastRuntimeState = 'unknown';
  private lastRuntimeLabel = 'Not hydrated yet.';
  private receiptRefs: string[] = [];

  constructor(options: { baseUrl?: string; fetchImpl?: FetchImpl } = {}) {
    this.client = new ShellSocketGatewayClient({
      baseUrl: clean(options.baseUrl || DEFAULT_GATEWAY_URL, 300),
      fetchImpl: options.fetchImpl,
    });
  }

  private rememberReceiptRefs(...payloads: any[]): void {
    const existing = this.receiptRefs.map((receipt_ref) => ({ receipt_ref }));
    this.receiptRefs = collectReceiptRefs(...payloads, ...existing).slice(0, 20);
  }

  setInputBuffer(value: string): void {
    this.inputBuffer = clean(value, 24000);
  }

  async hydrateInitialProjection(): Promise<BrowserShellV2Snapshot> {
    const runtime = (await this.client.getRuntimeStatus<Record<string, unknown>>()) || {};
    const agents = (await this.client.listAgents<Record<string, unknown>>({ limit: 40 })) || {};
    this.agentRows = rowsFromAgents(agents);
    const agentId = firstAgentId(agents) || this.agentRows[0]?.id || '';
    this.selectedAgentId = agentId;
    const sessions = agentId ? (await this.client.listSessions<Record<string, unknown>>(agentId, { limit: 40 })) || {} : {};
    this.sessionRows = rowsFromSessions(sessions);
    const sessionId = firstSessionId(sessions) || this.sessionRows[0]?.id || '';
    this.selectedSessionId = sessionId;
    const messages = sessionId ? (await this.client.getMessageWindow<Record<string, unknown>>(sessionId, { limit: MESSAGE_WINDOW_LIMIT })) || {} : {};
    this.visibleRows = rowsFromMessageWindow(messages);
    const events = sessionId ? (await this.client.subscribeEvents<Record<string, unknown>>(sessionId, { cursor: this.eventCursor })) || {} : {};
    this.eventRows = rowsFromEvents(events);
    this.eventCursor = clean((events as any).next_cursor || (events as any).cursor || this.eventRows[this.eventRows.length - 1]?.cursor || '', 160);
    this.lastRuntimeState = clean(runtime.state || 'unknown', 80);
    this.lastRuntimeLabel = clean(runtime.label || 'Runtime projection received.', 240);
    this.rememberReceiptRefs(runtime, agents, sessions, messages, events);
    return this.snapshot(agents);
  }

  async selectAgent(agentId: string): Promise<BrowserShellV2Snapshot> {
    const cleanAgentId = clean(agentId, 160);
    if (!cleanAgentId) return this.snapshot();
    this.selectedAgentId = cleanAgentId;
    const sessions = (await this.client.listSessions<Record<string, unknown>>(cleanAgentId, { limit: 40 })) || {};
    this.sessionRows = rowsFromSessions(sessions);
    this.selectedSessionId = firstSessionId(sessions) || this.sessionRows[0]?.id || '';
    const messages = this.selectedSessionId ? (await this.client.getMessageWindow<Record<string, unknown>>(this.selectedSessionId, { limit: MESSAGE_WINDOW_LIMIT })) || {} : {};
    this.visibleRows = rowsFromMessageWindow(messages);
    const events = this.selectedSessionId ? (await this.client.subscribeEvents<Record<string, unknown>>(this.selectedSessionId, { cursor: '' })) || {} : {};
    this.eventRows = rowsFromEvents(events);
    this.eventCursor = clean((events as any).next_cursor || (events as any).cursor || this.eventRows[this.eventRows.length - 1]?.cursor || '', 160);
    this.rememberReceiptRefs(sessions, messages, events);
    return this.snapshot();
  }

  async selectSession(sessionId: string): Promise<BrowserShellV2Snapshot> {
    const cleanSessionId = clean(sessionId, 240);
    if (!cleanSessionId) return this.snapshot();
    this.selectedSessionId = cleanSessionId;
    const messages = (await this.client.getMessageWindow<Record<string, unknown>>(cleanSessionId, { limit: MESSAGE_WINDOW_LIMIT })) || {};
    this.visibleRows = rowsFromMessageWindow(messages);
    const events = (await this.client.subscribeEvents<Record<string, unknown>>(cleanSessionId, { cursor: '' })) || {};
    this.eventRows = rowsFromEvents(events);
    this.eventCursor = clean((events as any).next_cursor || (events as any).cursor || this.eventRows[this.eventRows.length - 1]?.cursor || '', 160);
    this.rememberReceiptRefs(messages, events);
    return this.snapshot();
  }

  async refreshEvents(): Promise<BrowserShellV2Snapshot> {
    if (!this.selectedSessionId) return this.snapshot();
    const events = (await this.client.subscribeEvents<Record<string, unknown>>(this.selectedSessionId, { cursor: this.eventCursor })) || {};
    const nextRows = rowsFromEvents(events);
    this.eventRows = [...this.eventRows, ...nextRows].slice(-20);
    this.eventCursor = clean((events as any).next_cursor || (events as any).cursor || nextRows[nextRows.length - 1]?.cursor || this.eventCursor, 160);
    this.rememberReceiptRefs(events);
    return this.snapshot();
  }

  async search(query: string): Promise<BrowserShellV2Snapshot> {
    const cleanQuery = clean(query, 240);
    this.searchQuery = cleanQuery;
    if (!cleanQuery) {
      this.searchRows = [];
      return this.snapshot();
    }
    const results = (await this.client.search<Record<string, unknown>>({ q: cleanQuery, scope: 'session', limit: 20 })) || {};
    this.searchRows = rowsFromSearch(results);
    this.rememberReceiptRefs(results);
    return this.snapshot();
  }

  async submitIssue(note: string): Promise<BrowserShellV2Snapshot> {
    const cleanNote = clean(note || 'Browser Shell V2 issue/eval request.', 1000);
    const payload = {
      source: 'browser_shell_v2',
      medium: 'browser',
      agent_id: this.selectedAgentId,
      session_id: this.selectedSessionId,
      note: cleanNote,
      context_window: {
        message_ids: this.visibleRows.slice(-8).map((row) => row.id),
        event_ids: this.eventRows.slice(-8).map((row) => row.id),
      },
    };
    const result = (await this.client.submitIssue<Record<string, unknown>>(payload)) || {};
    this.issueStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    this.issueReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    this.rememberReceiptRefs(result);
    return this.snapshot();
  }

  async submitApprovalDecision(approvalId: string, decision: string): Promise<BrowserShellV2Snapshot> {
    const cleanApprovalId = clean(approvalId, 240);
    const cleanDecision = clean(decision || 'approve', 80);
    if (!cleanApprovalId) return this.snapshot();
    const result = (await this.client.submitApprovalDecision<Record<string, unknown>>(cleanApprovalId, {
      source: 'browser_shell_v2',
      medium: 'browser',
      agent_id: this.selectedAgentId,
      session_id: this.selectedSessionId,
      decision: cleanDecision,
    })) || {};
    this.approvalStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    this.approvalReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    this.rememberReceiptRefs(result);
    return this.snapshot();
  }

  async setModel(modelId: string): Promise<BrowserShellV2Snapshot> {
    const cleanModelId = clean(modelId, 160);
    if (!cleanModelId || !this.selectedAgentId) return this.snapshot();
    const result = (await this.client.setModel<Record<string, unknown>>(this.selectedAgentId, {
      source: 'browser_shell_v2',
      medium: 'browser',
      model_id: cleanModelId,
    })) || {};
    this.modelStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    this.modelReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    this.rememberReceiptRefs(result);
    return this.snapshot();
  }

  async setGitTree(treeId: string): Promise<BrowserShellV2Snapshot> {
    const cleanTreeId = clean(treeId, 240);
    if (!cleanTreeId || !this.selectedAgentId) return this.snapshot();
    const result = (await this.client.setGitTree<Record<string, unknown>>(this.selectedAgentId, {
      source: 'browser_shell_v2',
      medium: 'browser',
      tree_id: cleanTreeId,
    })) || {};
    this.gitTreeStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    this.gitTreeReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    this.rememberReceiptRefs(result);
    return this.snapshot();
  }

  async openMessageDetail(detailRef: string): Promise<BrowserShellV2Snapshot> {
    const cleanDetailRef = clean(detailRef, 300);
    if (!cleanDetailRef) return this.snapshot();
    const detail = (await this.client.getMessageDetail<Record<string, unknown>>(cleanDetailRef, { view: 'summary', limit: 1 })) || {};
    this.activeDetailRef = cleanDetailRef;
    this.activeDetailPreview = clean(
      (detail as any).preview || (detail as any).summary || (detail as any).text || 'Detail projection loaded.',
      2000,
    );
    this.rememberReceiptRefs(detail);
    return this.snapshot();
  }

  async submitInput(): Promise<Record<string, unknown>> {
    const message = clean(this.inputBuffer, 24000);
    if (!message || !this.selectedAgentId) {
      return { accepted: false, rejected: true, reason_code: 'browser_shell_v2_input_or_agent_missing' };
    }
    const response = (await this.client.submitInput({
      agent_id: this.selectedAgentId,
      message,
      source: 'browser_shell_v2',
      medium: 'browser',
    })) || {};
    this.inputBuffer = '';
    const receipt = clean((response as any).receipt_ref || '', 300);
    if (receipt) this.receiptRefs = Array.from(new Set([...this.receiptRefs, receipt]));
    return response as Record<string, unknown>;
  }

  snapshot(agentPayload: any = {}): BrowserShellV2Snapshot {
    const agents = Array.isArray(agentPayload?.agents) ? agentPayload.agents : [];
    const agentIds = Array.isArray(agentPayload?.agent_ids) ? agentPayload.agent_ids : [];
    return {
      ok: Boolean(this.lastRuntimeState && this.selectedAgentId),
      runtime_state: this.lastRuntimeState,
      runtime_label: this.lastRuntimeLabel,
      selected_agent_id: this.selectedAgentId,
      selected_session_id: this.selectedSessionId,
      agent_rows: this.agentRows.slice(0, 40),
      session_rows: this.sessionRows.slice(0, 40),
      agent_count: Math.max(agents.length, agentIds.length, this.agentRows.length),
      message_count: this.visibleRows.length,
      visible_rows: this.visibleRows.slice(0, MESSAGE_WINDOW_LIMIT),
      event_rows: this.eventRows.slice(-20),
      event_cursor: this.eventCursor || undefined,
      search_query: this.searchQuery || undefined,
      search_rows: this.searchRows.slice(0, 20),
      active_detail_ref: this.activeDetailRef || undefined,
      active_detail_preview: this.activeDetailPreview || undefined,
      issue_status: this.issueStatus || undefined,
      issue_receipt_ref: this.issueReceiptRef || undefined,
      approval_status: this.approvalStatus || undefined,
      approval_receipt_ref: this.approvalReceiptRef || undefined,
      model_status: this.modelStatus || undefined,
      model_receipt_ref: this.modelReceiptRef || undefined,
      git_tree_status: this.gitTreeStatus || undefined,
      git_tree_receipt_ref: this.gitTreeReceiptRef || undefined,
      receipt_refs: this.receiptRefs.slice(0, 20),
    };
  }
}

function fixtureFetch(): FetchImpl {
  return async (input: string, init?: Record<string, unknown>) => {
    const url = new URL(input, 'http://browser-shell-v2.fixture');
    const method = clean(init?.method || 'GET', 20).toUpperCase();
    const pathName = url.pathname;
    let payload: Record<string, unknown> = {};
    let ok = true;
    if (method === 'GET' && pathName === '/api/shell-socket/runtime-status') {
      payload = { state: 'ready', label: 'Browser Shell V2 fixture connected through Shell Socket', receipt_ref: 'receipt:browser-v2:runtime' };
    } else if (method === 'GET' && pathName === '/api/shell-socket/agents') {
      payload = { agents: [{ id: 'misty', label: 'Misty', state: 'ready' }], agent_ids: ['misty'], active_agent_id: 'misty', receipt_ref: 'receipt:browser-v2:agents' };
    } else if (method === 'GET' && pathName === '/api/shell-socket/agents/misty/sessions') {
      payload = { sessions: [{ id: 'misty::default', label: 'Default' }], session_ids: ['misty::default'], active_session_id: 'misty::default', receipt_ref: 'receipt:browser-v2:sessions' };
    } else if (method === 'GET' && pathName.startsWith('/api/shell-socket/sessions/misty%3A%3Adefault/messages')) {
      payload = {
        session_id: 'misty::default',
        total_count: 2,
        message_window: {
          rows: [
            { id: 'm1', role: 'user', text: 'Hello from fixture.' },
            { id: 'm2', role: 'assistant', text: 'Browser Shell V2 rendered a bounded message window.', detail_ref: 'detail:browser-v2:m2' },
          ],
        },
        receipt_ref: 'receipt:browser-v2:messages',
      };
    } else if (method === 'GET' && pathName === '/api/shell-socket/details/detail%3Abrowser-v2%3Am2') {
      payload = { preview: 'Lazy detail projection for fixture assistant row.', receipt_ref: 'receipt:browser-v2:detail' };
    } else if (method === 'GET' && pathName === '/api/shell-socket/sessions/misty%3A%3Adefault/events') {
      payload = {
        events: [
          { id: 'e1', label: 'Runtime connected', status: 'ready', cursor: 'cursor:e1' },
          { id: 'e2', label: 'Message window projected', status: 'bounded', cursor: 'cursor:e2' },
        ],
        next_cursor: 'cursor:e2',
        receipt_ref: 'receipt:browser-v2:events',
      };
    } else if (method === 'GET' && pathName === '/api/shell-socket/search') {
      payload = {
        results: [
          { id: 'search-1', label: 'Fixture assistant row', snippet: 'Browser Shell V2 rendered a bounded message window.', detail_ref: 'detail:browser-v2:m2' },
        ],
        receipt_ref: 'receipt:browser-v2:search',
      };
    } else if (method === 'POST' && pathName === '/api/shell-socket/issues') {
      payload = { status: 'submitted_to_gateway_eval', receipt_ref: 'receipt:browser-v2:issue' };
    } else if (method === 'POST' && pathName === '/api/shell-socket/approvals/approval%3Abrowser-v2/decision') {
      payload = { status: 'approval_decision_submitted', receipt_ref: 'receipt:browser-v2:approval' };
    } else if (method === 'POST' && pathName === '/api/shell-socket/agents/misty/model') {
      payload = { status: 'model_selection_submitted', receipt_ref: 'receipt:browser-v2:model' };
    } else if (method === 'POST' && pathName === '/api/shell-socket/agents/misty/git-tree') {
      payload = { status: 'git_tree_selection_submitted', receipt_ref: 'receipt:browser-v2:git-tree' };
    } else if (method === 'POST' && pathName === '/api/shell-socket/input') {
      const body = typeof init?.body === 'string' ? JSON.parse(init.body) : {};
      payload = {
        accepted: Boolean(clean(body.agent_id, 120) && clean(body.message, 24000)),
        rejected: !clean(body.agent_id, 120) || !clean(body.message, 24000),
        reason_code: clean(body.agent_id, 120) && clean(body.message, 24000) ? 'accepted' : 'agent_id_and_message_required',
        receipt_ref: 'receipt:browser-v2:submit-input',
      };
    } else {
      ok = false;
      payload = { error: 'browser_shell_v2_fixture_unknown_route', path: pathName };
    }
    return { ok, status: ok ? 200 : 404, text: async () => JSON.stringify(payload) };
  };
}

export async function runBrowserShellV2Smoke(options: { mode: 'fixture' | 'live'; baseUrl?: string }): Promise<BrowserShellV2SmokeResult> {
  try {
    const shell = new BrowserShellV2({
      baseUrl: clean(options.baseUrl || DEFAULT_GATEWAY_URL, 300),
      fetchImpl: options.mode === 'fixture' ? fixtureFetch() : undefined,
    });
    const snapshot = await shell.hydrateInitialProjection();
    const detailRef = snapshot.visible_rows.find((row) => row.detail_ref)?.detail_ref || '';
    const detailSnapshot = detailRef ? await shell.openMessageDetail(detailRef) : snapshot;
    const searchSnapshot = await shell.search('fixture');
    const issueSnapshot = await shell.submitIssue('Fixture issue/eval request.');
    const approvalSnapshot = await shell.submitApprovalDecision('approval:browser-v2', 'approve');
    const modelSnapshot = await shell.setModel('auto');
    const gitTreeSnapshot = await shell.setGitTree('workspace');
    shell.setInputBuffer('Browser Shell V2 socket smoke input.');
    const ack = await shell.submitInput();
    const submitted = Boolean((ack as any).accepted);
    return { ...gitTreeSnapshot, type: 'browser_shell_v2_socket_smoke', mode: options.mode, base_url: clean(options.baseUrl || DEFAULT_GATEWAY_URL, 300), submitted, ok: approvalSnapshot.ok && modelSnapshot.ok && gitTreeSnapshot.ok && submitted };
  } catch (error) {
    return {
      ok: false,
      type: 'browser_shell_v2_socket_smoke',
      mode: options.mode,
      base_url: clean(options.baseUrl || DEFAULT_GATEWAY_URL, 300),
      runtime_state: 'unavailable',
      runtime_label: 'Browser Shell V2 smoke failed.',
      selected_agent_id: '',
      selected_session_id: '',
      agent_count: 0,
      agent_rows: [],
      session_rows: [],
      message_count: 0,
      visible_rows: [],
      event_rows: [],
      event_cursor: undefined,
      search_query: undefined,
      search_rows: [],
      active_detail_ref: undefined,
      active_detail_preview: undefined,
      issue_status: undefined,
      issue_receipt_ref: undefined,
      approval_status: undefined,
      approval_receipt_ref: undefined,
      model_status: undefined,
      model_receipt_ref: undefined,
      git_tree_status: undefined,
      git_tree_receipt_ref: undefined,
      receipt_refs: [],
      submitted: false,
      error: clean(error instanceof Error ? error.message : error, 400),
    };
  }
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const mode: 'fixture' | 'live' = parseBool(readFlag(argv, 'live'), false) ? 'live' : 'fixture';
  const result = await runBrowserShellV2Smoke({ mode, baseUrl: readFlag(argv, 'base-url', DEFAULT_GATEWAY_URL) });
  writeJson(readFlag(argv, 'out-json', DEFAULT_OUT_JSON), result);
  writeMarkdown(readFlag(argv, 'out-markdown', DEFAULT_OUT_MARKDOWN), result);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  process.exitCode = result.ok ? 0 : 1;
}

if (process.argv.some((arg) => arg === '--smoke=1' || arg === '--smoke')) {
  main().catch((error) => {
    process.stderr.write(`${clean(error instanceof Error ? error.stack || error.message : error, 2000)}\n`);
    process.exitCode = 1;
  });
}
