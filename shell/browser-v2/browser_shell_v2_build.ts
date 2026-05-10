#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { compile } from 'svelte/compiler';

const OUT_DIR = 'core/local/artifacts/browser_shell_v2_app';
const COMPONENT_PATH = 'shell/browser-v2/BrowserShellV2.svelte';
const CSS_PATH = 'shell/browser-v2/browser_shell_v2.css';

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

function write(filePath: string, body: string): void {
  const abs = path.resolve(process.cwd(), filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, body, 'utf8');
}

function browserRuntimeSource(): string {
  return `const DEFAULT_GATEWAY_URL = 'http://127.0.0.1:5173';
const MESSAGE_WINDOW_LIMIT = 40;

function clean(value, max = 1000) {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function gatewayBaseUrl() {
  const params = new URLSearchParams(location.search);
  return clean(params.get('gateway') || DEFAULT_GATEWAY_URL, 300).replace(/\\/+$/, '');
}

async function socketRequest(capability, path, options = {}) {
  const response = await fetch(gatewayBaseUrl() + path, {
    method: options.method || 'GET',
    headers: {
      accept: 'application/json',
      ...(options.body ? { 'content-type': 'application/json' } : {}),
    },
    body: options.body ? JSON.stringify(options.body) : undefined,
  });
  const text = await response.text();
  const payload = text ? JSON.parse(text) : {};
  if (!response.ok) throw new Error('browser_shell_v2_socket_request_failed:' + capability + ':' + response.status);
  return payload;
}

function rowsFromMessageWindow(payload) {
  const rows = Array.isArray(payload?.message_window?.rows) ? payload.message_window.rows : [];
  return rows.slice(0, MESSAGE_WINDOW_LIMIT).map((row, index) => ({
    id: clean(row?.id || 'row-' + index, 160),
    role: clean(row?.role || 'assistant', 40),
    text: clean(row?.text || row?.preview || '', 12000),
    status: clean(row?.status || '', 80),
    detail_ref: clean(row?.detail_ref || row?.detailRef || '', 240),
  }));
}

function rowsFromAgents(payload) {
  const agents = Array.isArray(payload?.agents) ? payload.agents : [];
  const agentIds = Array.isArray(payload?.agent_ids) ? payload.agent_ids : [];
  if (agents.length) {
    return agents.slice(0, 40).map((agent, index) => {
      const id = clean(agent?.id || agent?.agent_id || agentIds[index] || 'agent-' + index, 160);
      return { id, label: clean(agent?.label || agent?.name || id, 160), state: clean(agent?.state || agent?.status || '', 80) };
    }).filter((agent) => agent.id);
  }
  return agentIds.slice(0, 40).map((id) => {
    const cleanId = clean(id, 160);
    return { id: cleanId, label: cleanId };
  }).filter((agent) => agent.id);
}

function rowsFromSessions(payload) {
  const sessions = Array.isArray(payload?.sessions) ? payload.sessions : [];
  const sessionIds = Array.isArray(payload?.session_ids) ? payload.session_ids : [];
  if (sessions.length) {
    return sessions.slice(0, 40).map((session, index) => {
      const id = clean(session?.id || session?.session_id || sessionIds[index] || 'session-' + index, 240);
      const count = Number(session?.message_count || session?.messageCount || 0);
      return { id, label: clean(session?.label || session?.title || id, 160), message_count: Number.isFinite(count) && count > 0 ? count : 0 };
    }).filter((session) => session.id);
  }
  return sessionIds.slice(0, 40).map((id) => {
    const cleanId = clean(id, 240);
    return { id: cleanId, label: cleanId };
  }).filter((session) => session.id);
}

function rowsFromEvents(payload) {
  const rows = Array.isArray(payload?.events) ? payload.events : Array.isArray(payload?.rows) ? payload.rows : [];
  return rows.slice(0, 20).map((row, index) => ({
    id: clean(row?.id || row?.event_id || 'event-' + index, 160),
    label: clean(row?.label || row?.summary || row?.type || row?.message || 'Event projection', 240),
    status: clean(row?.status || row?.state || '', 80),
    cursor: clean(row?.cursor || row?.event_cursor || '', 160),
  })).filter((event) => event.id);
}

function rowsFromSearch(payload) {
  const rows = Array.isArray(payload?.results) ? payload.results : Array.isArray(payload?.rows) ? payload.rows : [];
  return rows.slice(0, 20).map((row, index) => ({
    id: clean(row?.id || row?.result_id || 'search-' + index, 160),
    label: clean(row?.label || row?.title || row?.summary || 'Search result', 240),
    snippet: clean(row?.snippet || row?.preview || row?.text || '', 500),
    detail_ref: clean(row?.detail_ref || row?.detailRef || '', 240),
  })).filter((result) => result.id);
}

function firstAgentId(payload) {
  if (typeof payload?.active_agent_id === 'string' && payload.active_agent_id.trim()) return clean(payload.active_agent_id, 160);
  if (Array.isArray(payload?.agent_ids) && payload.agent_ids.length) return clean(payload.agent_ids[0], 160);
  if (Array.isArray(payload?.agents) && payload.agents.length) return clean(payload.agents[0]?.id || payload.agents[0]?.agent_id, 160);
  return '';
}

function firstSessionId(payload) {
  if (typeof payload?.active_session_id === 'string' && payload.active_session_id.trim()) return clean(payload.active_session_id, 240);
  if (Array.isArray(payload?.session_ids) && payload.session_ids.length) return clean(payload.session_ids[0], 240);
  if (Array.isArray(payload?.sessions) && payload.sessions.length) return clean(payload.sessions[0]?.id || payload.sessions[0]?.session_id, 240);
  return '';
}

function escapeHtml(value) {
  return clean(value, 12000).replace(/[&<>"']/g, (ch) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[ch]));
}

let selectedAgentId = '';
let selectedSessionId = '';
let agentRows = [];
let sessionRows = [];
let eventRows = [];
let eventCursor = '';
let searchQuery = '';
let searchRows = [];
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

function render(state) {
  const root = document.querySelector('#browser-shell-v2-root');
  if (!root) throw new Error('browser_shell_v2_root_missing');
  const messages = state.messages || [];
  root.innerHTML = \`
    <main class="browser-shell-v2" aria-label="Browser Shell V2">
      <section class="browser-shell-v2__topbar" aria-label="Runtime status">
        <div>
          <p class="browser-shell-v2__eyebrow">Infring Shell V2</p>
          <h1>Gateway Projection</h1>
        </div>
        <div class="browser-shell-v2__status" data-state="\${escapeHtml(state.runtimeState)}">
          <span>\${escapeHtml(state.runtimeState)}</span>
          <small>\${escapeHtml(state.runtimeLabel)}</small>
        </div>
      </section>
      <section class="browser-shell-v2__workspace" aria-label="Selected session">
        <aside class="browser-shell-v2__rail">
          <p class="browser-shell-v2__label">Agent</p>
          <strong>\${escapeHtml(selectedAgentId || 'none selected')}</strong>
          <div class="browser-shell-v2__selector-list" aria-label="Agent selector">
            \${agentRows.map((agent) => \`
              <button type="button" data-agent-id="\${escapeHtml(agent.id)}" class="\${agent.id === selectedAgentId ? 'active' : ''}" \${state.disabled ? 'disabled' : ''}>
                <span>\${escapeHtml(agent.label || agent.id)}</span>\${agent.state ? \`<small>\${escapeHtml(agent.state)}</small>\` : ''}
              </button>
            \`).join('')}
          </div>
          <p class="browser-shell-v2__label">Session</p>
          <strong>\${escapeHtml(selectedSessionId || 'none selected')}</strong>
          <div class="browser-shell-v2__selector-list" aria-label="Session selector">
            \${sessionRows.map((session) => \`
              <button type="button" data-session-id="\${escapeHtml(session.id)}" class="\${session.id === selectedSessionId ? 'active' : ''}" \${state.disabled ? 'disabled' : ''}>
                <span>\${escapeHtml(session.label || session.id)}</span>\${session.message_count ? \`<small>\${escapeHtml(session.message_count)} msgs</small>\` : ''}
              </button>
            \`).join('')}
          </div>
        </aside>
        <section class="browser-shell-v2__messages" aria-label="Message window">
          \${messages.length ? messages.map((message) => \`
            <article class="browser-shell-v2__message \${message.role === 'user' ? 'browser-shell-v2__message--user' : ''}">
              <header><span>\${escapeHtml(message.role)}</span>\${message.status ? \`<small>\${escapeHtml(message.status)}</small>\` : ''}</header>
              <p>\${escapeHtml(message.text)}</p>
              \${message.detail_ref ? \`<button class="browser-shell-v2__detail-button" type="button" data-detail-ref="\${escapeHtml(message.detail_ref)}" \${state.disabled ? 'disabled' : ''}>View detail</button>\` : ''}
            </article>
          \`).join('') : '<article class="browser-shell-v2__empty">No bounded message projection loaded yet.</article>'}
        </section>
      </section>
      \${activeDetailRef ? \`
        <section class="browser-shell-v2__detail" aria-label="Lazy message detail">
          <p class="browser-shell-v2__label">Lazy Detail</p>
          <strong>\${escapeHtml(activeDetailRef)}</strong>
          <p>\${escapeHtml(activeDetailPreview || 'Detail projection loaded.')}</p>
        </section>
      \` : ''}
      <section class="browser-shell-v2__events" aria-label="Gateway event projection">
        <div class="browser-shell-v2__events-header">
          <div>
            <p class="browser-shell-v2__label">Event Projection</p>
            <small>\${escapeHtml(eventCursor || 'no cursor')}</small>
          </div>
          <button type="button" data-refresh-events="1" \${state.disabled || !selectedSessionId ? 'disabled' : ''}>Refresh</button>
        </div>
        <div class="browser-shell-v2__event-list">
          \${eventRows.length ? eventRows.map((event) => \`
            <article>
              <span>\${escapeHtml(event.label)}</span>\${event.status ? \`<small>\${escapeHtml(event.status)}</small>\` : ''}
            </article>
          \`).join('') : '<article><span>No event projection loaded yet.</span></article>'}
        </div>
      </section>
      <section class="browser-shell-v2__search" aria-label="Bounded Gateway search">
        <form class="browser-shell-v2__search-form">
          <label>
            <span class="browser-shell-v2__label">Bounded Search</span>
            <input name="search" value="\${escapeHtml(searchQuery)}" \${state.disabled ? 'disabled' : ''} placeholder="Search via Gateway..." aria-label="Search query" />
          </label>
          <button type="submit" \${state.disabled ? 'disabled' : ''}>Search</button>
        </form>
        <div class="browser-shell-v2__search-results">
          \${searchRows.length ? searchRows.map((result) => \`
            <article>
              <strong>\${escapeHtml(result.label)}</strong>
              \${result.snippet ? \`<p>\${escapeHtml(result.snippet)}</p>\` : ''}
              \${result.detail_ref ? \`<button type="button" data-detail-ref="\${escapeHtml(result.detail_ref)}" \${state.disabled ? 'disabled' : ''}>View detail</button>\` : ''}
            </article>
          \`).join('') : '<article><strong>No search projection loaded.</strong></article>'}
        </div>
      </section>
      <section class="browser-shell-v2__issue" aria-label="Gateway issue evaluation request">
        <form class="browser-shell-v2__issue-form">
          <label>
            <span class="browser-shell-v2__label">Issue / Eval Request</span>
            <input name="issue" value="\${escapeHtml(issueNote)}" \${state.disabled ? 'disabled' : ''} placeholder="Ask Gateway to inspect this context..." aria-label="Issue note" />
          </label>
          <button type="submit" \${state.disabled || !selectedSessionId ? 'disabled' : ''}>Submit</button>
        </form>
        \${issueStatus || issueReceiptRef ? \`<p class="browser-shell-v2__issue-status"><strong>\${escapeHtml(issueStatus || 'submitted')}</strong>\${issueReceiptRef ? \`<span>\${escapeHtml(issueReceiptRef)}</span>\` : ''}</p>\` : ''}
      </section>
      <section class="browser-shell-v2__approval" aria-label="Gateway approval decision request">
        <form class="browser-shell-v2__approval-form">
          <label>
            <span class="browser-shell-v2__label">Approval Decision</span>
            <input name="approval-id" value="\${escapeHtml(approvalId)}" \${state.disabled ? 'disabled' : ''} placeholder="approval ref..." aria-label="Approval ID" />
          </label>
          <label>
            <span class="browser-shell-v2__label">Decision</span>
            <select name="approval-decision" \${state.disabled ? 'disabled' : ''} aria-label="Approval decision">
              \${['approve', 'deny', 'defer'].map((decision) => \`<option value="\${escapeHtml(decision)}" \${decision === approvalDecision ? 'selected' : ''}>\${escapeHtml(decision)}</option>\`).join('')}
            </select>
          </label>
          <button type="submit" \${state.disabled ? 'disabled' : ''}>Submit</button>
        </form>
        \${approvalStatus || approvalReceiptRef ? \`<p class="browser-shell-v2__approval-status"><strong>\${escapeHtml(approvalStatus || 'submitted')}</strong>\${approvalReceiptRef ? \`<span>\${escapeHtml(approvalReceiptRef)}</span>\` : ''}</p>\` : ''}
      </section>
      <section class="browser-shell-v2__controls" aria-label="Gateway selection requests">
        <form class="browser-shell-v2__control-form" data-control-form="model">
          <label>
            <span class="browser-shell-v2__label">Model Request</span>
            <input name="model" value="\${escapeHtml(modelSelection)}" \${state.disabled ? 'disabled' : ''} placeholder="auto, gpt-5.4, ..." aria-label="Model selection" />
          </label>
          <button type="submit" \${state.disabled || !selectedAgentId ? 'disabled' : ''}>Submit</button>
        </form>
        \${modelStatus || modelReceiptRef ? \`<p class="browser-shell-v2__control-status"><strong>\${escapeHtml(modelStatus || 'submitted')}</strong>\${modelReceiptRef ? \`<span>\${escapeHtml(modelReceiptRef)}</span>\` : ''}</p>\` : ''}
        <form class="browser-shell-v2__control-form" data-control-form="git-tree">
          <label>
            <span class="browser-shell-v2__label">Git Tree Request</span>
            <input name="git-tree" value="\${escapeHtml(gitTreeSelection)}" \${state.disabled ? 'disabled' : ''} placeholder="workspace, branch, tree ref..." aria-label="Git tree selection" />
          </label>
          <button type="submit" \${state.disabled || !selectedAgentId ? 'disabled' : ''}>Submit</button>
        </form>
        \${gitTreeStatus || gitTreeReceiptRef ? \`<p class="browser-shell-v2__control-status"><strong>\${escapeHtml(gitTreeStatus || 'submitted')}</strong>\${gitTreeReceiptRef ? \`<span>\${escapeHtml(gitTreeReceiptRef)}</span>\` : ''}</p>\` : ''}
      </section>
      <form class="browser-shell-v2__input">
        <input name="message" \${state.disabled ? 'disabled' : ''} placeholder="Send through Shell Socket..." aria-label="Shell input" />
        <button \${state.disabled ? 'disabled' : ''} type="submit">Send</button>
      </form>
    </main>\`;
  const form = root.querySelector('.browser-shell-v2__input');
  const searchForm = root.querySelector('.browser-shell-v2__search-form');
  const issueForm = root.querySelector('.browser-shell-v2__issue-form');
  const approvalForm = root.querySelector('.browser-shell-v2__approval-form');
  const modelForm = root.querySelector('[data-control-form="model"]');
  const gitTreeForm = root.querySelector('[data-control-form="git-tree"]');
  root.querySelectorAll('[data-agent-id]').forEach((button) => button.addEventListener('click', () => selectAgent(button.getAttribute('data-agent-id') || '')));
  root.querySelectorAll('[data-session-id]').forEach((button) => button.addEventListener('click', () => selectSession(button.getAttribute('data-session-id') || '')));
  root.querySelectorAll('[data-detail-ref]').forEach((button) => button.addEventListener('click', () => openMessageDetail(button.getAttribute('data-detail-ref') || '', state)));
  root.querySelectorAll('[data-refresh-events]').forEach((button) => button.addEventListener('click', () => refreshEvents(state)));
  searchForm?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = searchForm.querySelector('input');
    await search(input?.value || '', state);
  });
  issueForm?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = issueForm.querySelector('input');
    await submitIssue(input?.value || '', state);
  });
  approvalForm?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const idInput = approvalForm.querySelector('input[name="approval-id"]');
    const decisionInput = approvalForm.querySelector('select[name="approval-decision"]');
    await submitApprovalDecision(idInput?.value || '', decisionInput?.value || 'approve', state);
  });
  modelForm?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = modelForm.querySelector('input');
    await setModel(input?.value || '', state);
  });
  gitTreeForm?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = gitTreeForm.querySelector('input');
    await setGitTree(input?.value || '', state);
  });
  form?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = form.querySelector('input');
    const message = clean(input?.value || '', 24000);
    if (!message || !selectedAgentId) return;
    render({ ...state, disabled: true, runtimeLabel: 'Submitting through Shell Socket...' });
    await socketRequest('submit_input', '/api/shell-socket/input', {
      method: 'POST',
      body: { agent_id: selectedAgentId, message, source: 'browser_shell_v2', medium: 'browser' },
    });
    await hydrate();
  });
}

async function setModel(modelId, state) {
  modelSelection = clean(modelId, 160);
  if (!modelSelection || !selectedAgentId) return;
  render({ ...state, disabled: true, runtimeLabel: 'Submitting model request through Gateway...' });
  const result = await socketRequest('set_model', '/api/shell-socket/agents/' + encodeURIComponent(selectedAgentId) + '/model', {
    method: 'POST',
    body: { source: 'browser_shell_v2', medium: 'browser', model_id: modelSelection },
  });
  modelStatus = clean(result.status || result.reason_code || 'submitted', 120);
  modelReceiptRef = clean(result.receipt_ref || result.receiptRef || '', 300);
  render({ ...state, disabled: !selectedAgentId });
}

async function submitApprovalDecision(nextApprovalId, nextDecision, state) {
  approvalId = clean(nextApprovalId, 240);
  approvalDecision = clean(nextDecision || 'approve', 80);
  if (!approvalId) return;
  render({ ...state, disabled: true, runtimeLabel: 'Submitting approval decision through Gateway...' });
  const result = await socketRequest('submit_approval_decision', '/api/shell-socket/approvals/' + encodeURIComponent(approvalId) + '/decision', {
    method: 'POST',
    body: {
      source: 'browser_shell_v2',
      medium: 'browser',
      agent_id: selectedAgentId,
      session_id: selectedSessionId,
      decision: approvalDecision,
    },
  });
  approvalStatus = clean(result.status || result.reason_code || 'submitted', 120);
  approvalReceiptRef = clean(result.receipt_ref || result.receiptRef || '', 300);
  render({ ...state, disabled: !selectedAgentId });
}

async function setGitTree(treeId, state) {
  gitTreeSelection = clean(treeId, 240);
  if (!gitTreeSelection || !selectedAgentId) return;
  render({ ...state, disabled: true, runtimeLabel: 'Submitting git tree request through Gateway...' });
  const result = await socketRequest('set_git_tree', '/api/shell-socket/agents/' + encodeURIComponent(selectedAgentId) + '/git-tree', {
    method: 'POST',
    body: { source: 'browser_shell_v2', medium: 'browser', tree_id: gitTreeSelection },
  });
  gitTreeStatus = clean(result.status || result.reason_code || 'submitted', 120);
  gitTreeReceiptRef = clean(result.receipt_ref || result.receiptRef || '', 300);
  render({ ...state, disabled: !selectedAgentId });
}

async function submitIssue(note, state) {
  issueNote = clean(note || 'Browser Shell V2 issue/eval request.', 1000);
  if (!selectedSessionId) return;
  render({ ...state, disabled: true, runtimeLabel: 'Submitting issue/eval request through Gateway...' });
  const result = await socketRequest('submit_issue', '/api/shell-socket/issues', {
    method: 'POST',
    body: {
      source: 'browser_shell_v2',
      medium: 'browser',
      agent_id: selectedAgentId,
      session_id: selectedSessionId,
      note: issueNote,
      context_window: { event_ids: eventRows.slice(-8).map((row) => row.id) },
    },
  });
  issueStatus = clean(result.status || result.reason_code || 'submitted', 120);
  issueReceiptRef = clean(result.receipt_ref || result.receiptRef || '', 300);
  render({ ...state, disabled: !selectedAgentId });
}

async function search(query, state) {
  searchQuery = clean(query, 240);
  if (!searchQuery) {
    searchRows = [];
    render({ ...state, disabled: !selectedAgentId });
    return;
  }
  render({ ...state, disabled: true, runtimeLabel: 'Running bounded Gateway search...' });
  const results = await socketRequest('search', '/api/shell-socket/search?q=' + encodeURIComponent(searchQuery) + '&scope=session&limit=20');
  searchRows = rowsFromSearch(results);
  render({ ...state, disabled: !selectedAgentId });
}

async function refreshEvents(state) {
  if (!selectedSessionId) return;
  render({ ...state, disabled: true, runtimeLabel: 'Refreshing event projection...' });
  const events = await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=' + encodeURIComponent(eventCursor));
  const nextRows = rowsFromEvents(events);
  eventRows = eventRows.concat(nextRows).slice(-20);
  eventCursor = clean(events.next_cursor || events.cursor || (nextRows[nextRows.length - 1] || {}).cursor || eventCursor, 160);
  render({ ...state, disabled: !selectedAgentId });
}

async function openMessageDetail(detailRef, state) {
  const cleanDetailRef = clean(detailRef, 300);
  if (!cleanDetailRef) return;
  render({ ...state, disabled: true, runtimeLabel: 'Loading lazy detail projection...' });
  const detail = await socketRequest('get_message_detail', '/api/shell-socket/details/' + encodeURIComponent(cleanDetailRef) + '?view=summary&limit=1');
  activeDetailRef = cleanDetailRef;
  activeDetailPreview = clean(detail.preview || detail.summary || detail.text || 'Detail projection loaded.', 2000);
  render({ ...state, disabled: !selectedAgentId });
}

async function selectAgent(agentId) {
  const cleanAgentId = clean(agentId, 160);
  if (!cleanAgentId) return;
  render({ runtimeState: 'loading', runtimeLabel: 'Loading selected agent projection...', messages: [], disabled: true });
  selectedAgentId = cleanAgentId;
  const sessions = await socketRequest('list_sessions', '/api/shell-socket/agents/' + encodeURIComponent(selectedAgentId) + '/sessions?limit=40');
  sessionRows = rowsFromSessions(sessions);
  selectedSessionId = firstSessionId(sessions) || sessionRows[0]?.id || '';
  const messages = selectedSessionId ? await socketRequest('get_message_window', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/messages?limit=' + MESSAGE_WINDOW_LIMIT) : {};
  const events = selectedSessionId ? await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=') : {};
  eventRows = rowsFromEvents(events);
  eventCursor = clean(events.next_cursor || events.cursor || (eventRows[eventRows.length - 1] || {}).cursor || '', 160);
  render({ runtimeState: 'ready', runtimeLabel: 'Selected agent projection loaded.', messages: rowsFromMessageWindow(messages), disabled: !selectedAgentId });
}

async function selectSession(sessionId) {
  const cleanSessionId = clean(sessionId, 240);
  if (!cleanSessionId) return;
  render({ runtimeState: 'loading', runtimeLabel: 'Loading selected session projection...', messages: [], disabled: true });
  selectedSessionId = cleanSessionId;
  const messages = await socketRequest('get_message_window', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/messages?limit=' + MESSAGE_WINDOW_LIMIT);
  const events = await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=');
  eventRows = rowsFromEvents(events);
  eventCursor = clean(events.next_cursor || events.cursor || (eventRows[eventRows.length - 1] || {}).cursor || '', 160);
  render({ runtimeState: 'ready', runtimeLabel: 'Selected session projection loaded.', messages: rowsFromMessageWindow(messages), disabled: !selectedAgentId });
}

async function hydrate() {
  render({ runtimeState: 'loading', runtimeLabel: 'Hydrating from Shell Socket Gateway projection...', messages: [], disabled: true });
  try {
    const runtime = await socketRequest('get_runtime_status', '/api/shell-socket/runtime-status');
    const agents = await socketRequest('list_agents', '/api/shell-socket/agents?limit=40');
    agentRows = rowsFromAgents(agents);
    selectedAgentId = firstAgentId(agents) || agentRows[0]?.id || '';
    const sessions = selectedAgentId ? await socketRequest('list_sessions', '/api/shell-socket/agents/' + encodeURIComponent(selectedAgentId) + '/sessions?limit=40') : {};
    sessionRows = rowsFromSessions(sessions);
    selectedSessionId = firstSessionId(sessions) || sessionRows[0]?.id || '';
    const messages = selectedSessionId ? await socketRequest('get_message_window', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/messages?limit=' + MESSAGE_WINDOW_LIMIT) : {};
    const events = selectedSessionId ? await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=' + encodeURIComponent(eventCursor)) : {};
    eventRows = rowsFromEvents(events);
    eventCursor = clean(events.next_cursor || events.cursor || (eventRows[eventRows.length - 1] || {}).cursor || '', 160);
    render({
      runtimeState: clean(runtime.state || 'unknown', 80),
      runtimeLabel: clean(runtime.label || 'Runtime projection received.', 240),
      messages: rowsFromMessageWindow(messages),
      disabled: !selectedAgentId,
    });
  } catch (error) {
    render({ runtimeState: 'unavailable', runtimeLabel: clean(error instanceof Error ? error.message : error, 240), messages: [], disabled: true });
  }
}

hydrate();
`;
}

export function buildBrowserShellV2App(outDir = OUT_DIR): Record<string, unknown> {
  const componentSource = fs.readFileSync(path.resolve(process.cwd(), COMPONENT_PATH), 'utf8');
  const compiled = compile(componentSource, { generate: 'client', dev: false, css: 'external' });
  const warnings = compiled.warnings.map((warning) => warning.message);
  const css = fs.readFileSync(path.resolve(process.cwd(), CSS_PATH), 'utf8');
  const targetDir = path.resolve(process.cwd(), outDir);
  fs.rmSync(targetDir, { recursive: true, force: true });
  fs.mkdirSync(targetDir, { recursive: true });
  write(path.join(outDir, 'index.html'), `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Infring Browser Shell V2</title>
    <link rel="stylesheet" href="./browser_shell_v2.css" />
  </head>
  <body>
    <div id="browser-shell-v2-root"></div>
    <script type="module" src="./browser_shell_v2_app.js"></script>
  </body>
</html>
`);
  write(path.join(outDir, 'browser_shell_v2.css'), css);
  write(path.join(outDir, 'browser_shell_v2_app.js'), browserRuntimeSource());
  write(path.join(outDir, 'svelte_component_preflight.js'), compiled.js.code);
  return {
    ok: warnings.length === 0,
    type: 'browser_shell_v2_build',
    out_dir: outDir,
    files: ['index.html', 'browser_shell_v2.css', 'browser_shell_v2_app.js', 'svelte_component_preflight.js'],
    svelte_warnings: warnings,
  };
}

if (process.argv.some((arg) => arg === '--build=1' || arg === '--build')) {
  const argv = process.argv.slice(2);
  const outDir = readFlag(argv, 'out-dir', OUT_DIR);
  const result = buildBrowserShellV2App(outDir);
  const outJson = readFlag(argv, 'out-json', 'core/local/artifacts/browser_shell_v2_build_current.json');
  const outMarkdown = readFlag(argv, 'out-markdown', 'local/workspace/reports/BROWSER_SHELL_V2_BUILD_CURRENT.md');
  write(outJson, `${JSON.stringify(result, null, 2)}\n`);
  write(outMarkdown, `# Browser Shell V2 Build\n\nok: \`${result.ok}\`\nout_dir: \`${outDir}\`\nfiles: \`${(result.files as string[]).join(', ')}\`\n`);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  process.exitCode = result.ok ? 0 : 1;
}
