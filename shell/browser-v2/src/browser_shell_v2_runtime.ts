import BrowserShellV2Component from '../BrowserShellV2.svelte';
import '../browser_shell_v2.css';
import { ShellSocketGatewayClient } from '../../socket/client/shell_socket_gateway_client.ts';

const DEFAULT_GATEWAY_URL = 'http://127.0.0.1:5173';
const MESSAGE_WINDOW_LIMIT = 40;

type MessageRow = {
  id: string;
  role: string;
  text: string;
  status?: string;
  timestamp?: string;
  detail_ref?: string;
};

type AgentRow = { id: string; label: string; state?: string };
type SessionRow = { id: string; label: string; message_count?: number };
type EventRow = { id: string; label: string; status?: string; cursor?: string };
type SearchRow = { id: string; label: string; snippet?: string; detail_ref?: string };

function clean(value: unknown, max = 1000): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function gatewayBaseUrl(): string {
  const params = new URLSearchParams(location.search);
  return clean(params.get('gateway') || DEFAULT_GATEWAY_URL, 300);
}

function rowsFromMessageWindow(payload: any): MessageRow[] {
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

function rowsFromAgents(payload: any): AgentRow[] {
  const agents = Array.isArray(payload?.agents) ? payload.agents : [];
  const agentIds = Array.isArray(payload?.agent_ids) ? payload.agent_ids : [];
  if (agents.length) {
    return agents.slice(0, 40).map((agent: any, index: number) => {
      const id = clean(agent?.id || agent?.agent_id || agentIds[index] || `agent-${index}`, 160);
      return { id, label: clean(agent?.label || agent?.name || id, 160), state: clean(agent?.state || agent?.status || '', 80) || undefined };
    }).filter((agent: AgentRow) => Boolean(agent.id));
  }
  return agentIds.slice(0, 40).map((id: unknown) => {
    const cleanId = clean(id, 160);
    return { id: cleanId, label: cleanId };
  }).filter((agent: AgentRow) => Boolean(agent.id));
}

function rowsFromSessions(payload: any): SessionRow[] {
  const sessions = Array.isArray(payload?.sessions) ? payload.sessions : [];
  const sessionIds = Array.isArray(payload?.session_ids) ? payload.session_ids : [];
  if (sessions.length) {
    return sessions.slice(0, 40).map((session: any, index: number) => {
      const id = clean(session?.id || session?.session_id || sessionIds[index] || `session-${index}`, 240);
      const count = Number(session?.message_count || session?.messageCount || 0);
      return { id, label: clean(session?.label || session?.title || id, 160), message_count: Number.isFinite(count) && count > 0 ? count : undefined };
    }).filter((session: SessionRow) => Boolean(session.id));
  }
  return sessionIds.slice(0, 40).map((id: unknown) => {
    const cleanId = clean(id, 240);
    return { id: cleanId, label: cleanId };
  }).filter((session: SessionRow) => Boolean(session.id));
}

function rowsFromEvents(payload: any): EventRow[] {
  const rows = Array.isArray(payload?.events) ? payload.events : Array.isArray(payload?.rows) ? payload.rows : [];
  return rows.slice(0, 20).map((row: any, index: number) => ({
    id: clean(row?.id || row?.event_id || `event-${index}`, 160),
    label: clean(row?.label || row?.summary || row?.type || row?.message || 'Event projection', 240),
    status: clean(row?.status || row?.state || '', 80) || undefined,
    cursor: clean(row?.cursor || row?.event_cursor || '', 160) || undefined,
  })).filter((event: EventRow) => Boolean(event.id));
}

function rowsFromSearch(payload: any): SearchRow[] {
  const rows = Array.isArray(payload?.results) ? payload.results : Array.isArray(payload?.rows) ? payload.rows : [];
  return rows.slice(0, 20).map((row: any, index: number) => ({
    id: clean(row?.id || row?.result_id || `search-${index}`, 160),
    label: clean(row?.label || row?.title || row?.summary || 'Search result', 240),
    snippet: clean(row?.snippet || row?.preview || row?.text || '', 500) || undefined,
    detail_ref: clean(row?.detail_ref || row?.detailRef || '', 240) || undefined,
  })).filter((result: SearchRow) => Boolean(result.id));
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

const root = document.querySelector('#browser-shell-v2-root');
if (!root) throw new Error('browser_shell_v2_root_missing');

const client = new ShellSocketGatewayClient({ baseUrl: gatewayBaseUrl() });
let selectedAgentId = '';
let selectedSessionId = '';
let agentRows: AgentRow[] = [];
let sessionRows: SessionRow[] = [];
let eventRows: EventRow[] = [];
let eventCursor = '';
let searchQuery = '';
let searchRows: SearchRow[] = [];
let activeDetailRef = '';
let activeDetailPreview = '';
let issueNote = '';
let issueStatus = '';
let issueReceiptRef = '';
let approvalId = '';
let approvalDecision = 'approve';
let approvalStatus = '';
let approvalReceiptRef = '';
let modelSelection = '';
let modelStatus = '';
let modelReceiptRef = '';
let gitTreeSelection = '';
let gitTreeStatus = '';
let gitTreeReceiptRef = '';
let receiptRefs: string[] = [];

const app = new BrowserShellV2Component({
  target: root,
  props: {
    runtimeState: 'loading',
    runtimeLabel: 'Hydrating from Shell Socket Gateway projection...',
    selectedAgentId: '',
    selectedSessionId: '',
    agentRows: [],
    sessionRows: [],
    eventRows: [],
    eventCursor: '',
    searchQuery: '',
    searchRows: [],
    activeDetailRef: '',
    activeDetailPreview: '',
    issueNote: '',
    issueStatus: '',
    issueReceiptRef: '',
    approvalId: '',
    approvalDecision: 'approve',
    approvalStatus: '',
    approvalReceiptRef: '',
    modelSelection: '',
    modelStatus: '',
    modelReceiptRef: '',
    gitTreeSelection: '',
    gitTreeStatus: '',
    gitTreeReceiptRef: '',
    receiptRefs: [],
    messages: [],
    inputValue: '',
    disabled: true,
    onSubmitInput: submitInput,
    onSelectAgent: selectAgent,
    onSelectSession: selectSession,
    onOpenMessageDetail: openMessageDetail,
    onRefreshEvents: refreshEvents,
    onSearch: search,
    onSubmitIssue: submitIssue,
    onSubmitApprovalDecision: submitApprovalDecision,
    onSetModel: setModel,
    onSetGitTree: setGitTree,
  },
});

function rememberReceiptRefs(...payloads: any[]): void {
  const nextRefs = new Set(receiptRefs);
  for (const payload of payloads) {
    const ref = clean(payload?.receipt_ref || payload?.receiptRef || '', 300);
    if (ref) nextRefs.add(ref);
  }
  receiptRefs = Array.from(nextRefs).slice(-20);
}

async function hydrate() {
  try {
    const runtime = (await client.getRuntimeStatus<Record<string, unknown>>()) || {};
    const agents = (await client.listAgents<Record<string, unknown>>({ limit: 40 })) || {};
    agentRows = rowsFromAgents(agents);
    selectedAgentId = firstAgentId(agents) || agentRows[0]?.id || '';
    const sessions = selectedAgentId ? (await client.listSessions<Record<string, unknown>>(selectedAgentId, { limit: 40 })) || {} : {};
    sessionRows = rowsFromSessions(sessions);
    selectedSessionId = firstSessionId(sessions) || sessionRows[0]?.id || '';
    const messages = selectedSessionId ? (await client.getMessageWindow<Record<string, unknown>>(selectedSessionId, { limit: MESSAGE_WINDOW_LIMIT })) || {} : {};
    const events = selectedSessionId ? (await client.subscribeEvents<Record<string, unknown>>(selectedSessionId, { cursor: eventCursor })) || {} : {};
    eventRows = rowsFromEvents(events);
    eventCursor = clean((events as any).next_cursor || (events as any).cursor || eventRows[eventRows.length - 1]?.cursor || '', 160);
    rememberReceiptRefs(runtime, agents, sessions, messages, events);
    app.$set({
      runtimeState: clean(runtime.state || 'unknown', 80),
      runtimeLabel: clean(runtime.label || 'Runtime projection received.', 240),
      selectedAgentId,
      selectedSessionId,
      agentRows,
      sessionRows,
      eventRows,
      eventCursor,
      searchQuery,
      searchRows,
      activeDetailRef,
      activeDetailPreview,
      issueNote,
      issueStatus,
      issueReceiptRef,
      approvalId,
      approvalDecision,
      approvalStatus,
      approvalReceiptRef,
      modelSelection,
      modelStatus,
      modelReceiptRef,
      gitTreeSelection,
      gitTreeStatus,
      gitTreeReceiptRef,
      receiptRefs,
      messages: rowsFromMessageWindow(messages),
      disabled: !selectedAgentId,
    });
  } catch (error) {
    app.$set({
      runtimeState: 'unavailable',
      runtimeLabel: clean(error instanceof Error ? error.message : error, 240),
      disabled: true,
    });
  }
}

async function setModel(modelId: string) {
  modelSelection = clean(modelId, 160);
  if (!modelSelection || !selectedAgentId) return;
  app.$set({ disabled: true, modelSelection });
  try {
    const result = await client.setModel<Record<string, unknown>>(selectedAgentId, {
      source: 'browser_shell_v2',
      medium: 'browser',
      model_id: modelSelection,
    });
    modelStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    modelReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    rememberReceiptRefs(result);
    app.$set({ modelStatus, modelReceiptRef, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'model_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function submitApprovalDecision(nextApprovalId: string, nextDecision: string) {
  approvalId = clean(nextApprovalId, 240);
  approvalDecision = clean(nextDecision || 'approve', 80);
  if (!approvalId) return;
  app.$set({ disabled: true, approvalId, approvalDecision });
  try {
    const result = await client.submitApprovalDecision<Record<string, unknown>>(approvalId, {
      source: 'browser_shell_v2',
      medium: 'browser',
      agent_id: selectedAgentId,
      session_id: selectedSessionId,
      decision: approvalDecision,
    });
    approvalStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    approvalReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    rememberReceiptRefs(result);
    app.$set({ approvalStatus, approvalReceiptRef, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'approval_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function setGitTree(treeId: string) {
  gitTreeSelection = clean(treeId, 240);
  if (!gitTreeSelection || !selectedAgentId) return;
  app.$set({ disabled: true, gitTreeSelection });
  try {
    const result = await client.setGitTree<Record<string, unknown>>(selectedAgentId, {
      source: 'browser_shell_v2',
      medium: 'browser',
      tree_id: gitTreeSelection,
    });
    gitTreeStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    gitTreeReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    rememberReceiptRefs(result);
    app.$set({ gitTreeStatus, gitTreeReceiptRef, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'git_tree_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function submitIssue(note: string) {
  issueNote = clean(note || 'Browser Shell V2 issue/eval request.', 1000);
  if (!selectedSessionId) return;
  app.$set({ disabled: true, issueNote });
  try {
    const result = await client.submitIssue<Record<string, unknown>>({
      source: 'browser_shell_v2',
      medium: 'browser',
      agent_id: selectedAgentId,
      session_id: selectedSessionId,
      note: issueNote,
      context_window: {
        message_ids: [],
        event_ids: eventRows.slice(-8).map((row) => row.id),
      },
    });
    issueStatus = clean((result as any).status || (result as any).reason_code || 'submitted', 120);
    issueReceiptRef = clean((result as any).receipt_ref || (result as any).receiptRef || '', 300);
    rememberReceiptRefs(result);
    app.$set({ issueStatus, issueReceiptRef, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'issue_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function search(query: string) {
  const cleanQuery = clean(query, 240);
  searchQuery = cleanQuery;
  if (!cleanQuery) {
    searchRows = [];
    app.$set({ searchQuery, searchRows });
    return;
  }
  app.$set({ disabled: true, searchQuery });
  try {
    const results = await client.search<Record<string, unknown>>({ q: cleanQuery, scope: 'session', limit: 20 });
    searchRows = rowsFromSearch(results);
    rememberReceiptRefs(results);
    app.$set({ searchRows, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'search_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function refreshEvents() {
  if (!selectedSessionId) return;
  app.$set({ disabled: true });
  try {
    const events = await client.subscribeEvents<Record<string, unknown>>(selectedSessionId, { cursor: eventCursor });
    const nextRows = rowsFromEvents(events);
    eventRows = [...eventRows, ...nextRows].slice(-20);
    eventCursor = clean((events as any).next_cursor || (events as any).cursor || nextRows[nextRows.length - 1]?.cursor || eventCursor, 160);
    rememberReceiptRefs(events);
    app.$set({ eventRows, eventCursor, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'events_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function openMessageDetail(detailRef: string) {
  const cleanDetailRef = clean(detailRef, 300);
  if (!cleanDetailRef) return;
  app.$set({ disabled: true });
  try {
    const detail = await client.getMessageDetail<Record<string, unknown>>(cleanDetailRef, { view: 'summary', limit: 1 });
    activeDetailRef = cleanDetailRef;
    activeDetailPreview = clean((detail as any).preview || (detail as any).summary || (detail as any).text || 'Detail projection loaded.', 2000);
    rememberReceiptRefs(detail);
    app.$set({ activeDetailRef, activeDetailPreview, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'detail_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function selectAgent(agentId: string) {
  const cleanAgentId = clean(agentId, 160);
  if (!cleanAgentId) return;
  app.$set({ disabled: true });
  try {
    selectedAgentId = cleanAgentId;
    const sessions = (await client.listSessions<Record<string, unknown>>(cleanAgentId, { limit: 40 })) || {};
    sessionRows = rowsFromSessions(sessions);
    selectedSessionId = firstSessionId(sessions) || sessionRows[0]?.id || '';
    const messages = selectedSessionId ? (await client.getMessageWindow<Record<string, unknown>>(selectedSessionId, { limit: MESSAGE_WINDOW_LIMIT })) || {} : {};
    const events = selectedSessionId ? (await client.subscribeEvents<Record<string, unknown>>(selectedSessionId, { cursor: '' })) || {} : {};
    eventRows = rowsFromEvents(events);
    eventCursor = clean((events as any).next_cursor || (events as any).cursor || eventRows[eventRows.length - 1]?.cursor || '', 160);
    rememberReceiptRefs(sessions, messages, events);
    app.$set({ selectedAgentId, selectedSessionId, sessionRows, messages: rowsFromMessageWindow(messages), eventRows, eventCursor, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'select_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function selectSession(sessionId: string) {
  const cleanSessionId = clean(sessionId, 240);
  if (!cleanSessionId) return;
  app.$set({ disabled: true });
  try {
    selectedSessionId = cleanSessionId;
    const messages = await client.getMessageWindow<Record<string, unknown>>(cleanSessionId, { limit: MESSAGE_WINDOW_LIMIT });
    const events = await client.subscribeEvents<Record<string, unknown>>(cleanSessionId, { cursor: '' });
    eventRows = rowsFromEvents(events);
    eventCursor = clean((events as any).next_cursor || (events as any).cursor || eventRows[eventRows.length - 1]?.cursor || '', 160);
    rememberReceiptRefs(messages, events);
    app.$set({ selectedSessionId, messages: rowsFromMessageWindow(messages), eventRows, eventCursor, receiptRefs, disabled: !selectedAgentId });
  } catch (error) {
    app.$set({ runtimeState: 'select_failed', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), disabled: false });
  }
}

async function submitInput(value: string) {
  const message = clean(value, 24000);
  if (!message || !selectedAgentId) return;
  app.$set({ disabled: true });
  try {
    const result = await client.submitInput({
      agent_id: selectedAgentId,
      message,
      source: 'browser_shell_v2',
      medium: 'browser',
    });
    rememberReceiptRefs(result);
    app.$set({ inputValue: '' });
    await hydrate();
  } catch (error) {
    app.$set({
      runtimeState: 'submit_failed',
      runtimeLabel: clean(error instanceof Error ? error.message : error, 240),
    });
  } finally {
    app.$set({ disabled: !selectedAgentId });
  }
}

hydrate();
