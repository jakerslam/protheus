#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { compile } from 'svelte/compiler';

const OUT_DIR = 'core/local/artifacts/browser_shell_v2_app';
const COMPONENT_PATH = 'shell/browser-v2/BrowserShellV2.svelte';
const CSS_PATH = 'shell/browser-v2/browser_shell_v2.css';
const LEGACY_CSS_DIR = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'css'].join('/');
const LEGACY_CSS_PATHS = [
  'theme.css',
  ...fs.readdirSync(path.resolve(process.cwd(), LEGACY_CSS_DIR, 'layout.css.parts')).sort().map((name) => `layout.css.parts/${name}`),
  ...fs.readdirSync(path.resolve(process.cwd(), LEGACY_CSS_DIR, 'components.css.parts')).sort().map((name) => `components.css.parts/${name}`),
];

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

function legacySurfaceCss(): string {
  return LEGACY_CSS_PATHS
    .map((relPath) => {
      const absPath = path.resolve(process.cwd(), LEGACY_CSS_DIR, relPath);
      return `\n/* legacy surface: ${relPath} */\n${fs.readFileSync(absPath, 'utf8')}\n`;
    })
    .join('\n');
}

function browserRuntimeSource(): string {
  return `const DEFAULT_GATEWAY_URL = 'http://127.0.0.1:5173';
const MESSAGE_WINDOW_LIMIT = 40;
const EVENT_POLL_INTERVAL_MS = 5000;

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

function rowsFromSelectorOptions(payload, keys, fallback) {
  const source = keys.map((key) => payload?.[key]).find((value) => Array.isArray(value)) || [];
  const rows = source.slice(0, 20).map((row, index) => {
    const id = clean(row?.id || row?.ref || row?.value || row?.name || row, 240);
    return {
      id: id || 'option-' + index,
      label: clean(row?.label || row?.title || row?.name || id || 'Option ' + (index + 1), 160),
      meta: clean(row?.meta || row?.summary || row?.description || row?.status || '', 220),
    };
  }).filter((row) => row.id);
  return rows.length ? rows : fallback;
}

function rowsFromDetailProjection(projection) {
  const source = Array.isArray(projection?.rows)
    ? projection.rows
    : Array.isArray(projection?.tool_summaries)
      ? projection.tool_summaries
      : Array.isArray(projection?.items)
        ? projection.items
        : [];
  return source.slice(0, 12).map((row, index) => ({
    id: clean(row?.id || row?.ref || row?.name || row?.label || 'detail-row-' + index, 160),
    label: clean(row?.label || row?.title || row?.name || row?.kind || 'Detail row ' + (index + 1), 180),
    meta: clean(row?.meta || row?.status || row?.summary || row?.text_preview || row?.preview || '', 260),
  })).filter((row) => row.id);
}

function detailRefsFromProjection(detail, projection) {
  const refs = []
    .concat(Array.isArray(detail?.detail_refs) ? detail.detail_refs : [])
    .concat(Array.isArray(projection?.detail_refs) ? projection.detail_refs : [])
    .concat(Array.isArray(projection?.refs) ? projection.refs : []);
  return refs.map((ref) => clean(ref, 240)).filter(Boolean).slice(0, 12);
}

function detailPanelFromProjection(detailRef, detail) {
  const projection = detail?.detail_projection || detail?.projection || detail || {};
  return {
    ref: detailRef,
    kind: clean(detail?.detail_kind || projection?.kind || projection?.type || 'detail', 80),
    title: clean(projection?.title || projection?.label || projection?.id || detail?.detail_id || detailRef, 180),
    summary: clean(projection?.summary || projection?.text_preview || projection?.preview || detail?.preview || detail?.summary || detail?.text || 'Detail projection loaded.', 2000),
    rows: rowsFromDetailProjection(projection),
    refs: detailRefsFromProjection(detail, projection),
    cursor: clean(detail?.next_cursor || projection?.next_cursor || '', 160),
    receipt_ref: clean(detail?.receipt_ref || detail?.receiptRef || '', 300),
  };
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
let activeDetailPanel = null;
let issueNote = '';
let issueStatus = '';
let issueReceiptRef = '';
let approvalId = '';
let approvalDecision = 'approve';
let approvalStatus = '';
let approvalReceiptRef = '';
let modelSelection = '';
let modelRows = [];
let modelStatus = '';
let modelReceiptRef = '';
let gitTreeSelection = '';
let gitTreeRows = [];
let gitTreeStatus = '';
let gitTreeReceiptRef = '';
let eventRefreshInFlight = false;
let eventPollTimer = 0;

function render(state) {
  const root = document.querySelector('#browser-shell-v2-root');
  if (!root) throw new Error('browser_shell_v2_root_missing');
  const messages = state.messages || [];
  const selectedAgentLabel = selectedAgentId || 'No agent selected';
  const runtimeBadge = clean(state.runtimeState || 'unknown', 80);
  root.innerHTML = \`
    <div class="app-layout browser-shell-v2 browser-shell-v2--legacy-surface" aria-label="Browser Shell V2">
      <aside class="sidebar drag-bar overlay-shared-surface chat-sidebar-dynamic" aria-label="Legacy dashboard conversation rail">
        <div class="sidebar-nav-shell">
          <div class="sidebar-nav" role="navigation" aria-label="Main navigation">
            <div class="nav-section" aria-label="Agent conversations">
              <a class="nav-item sidebar-tab-item active" aria-current="page">
                <span class="nav-icon">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg>
                </span>
                <span class="nav-label">Conversations</span>
              </a>
              <div class="nav-sub-search-row">
                <div class="nav-sub-search-wrap">
                  <span class="nav-sub-search-icon" aria-hidden="true">
                    <svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7"></circle><path d="m20 20-3.6-3.6"></path></svg>
                  </span>
                  <input class="nav-sub-search-input" type="text" value="\${escapeHtml(searchQuery)}" placeholder="Search conversations..." aria-label="Search conversations" readonly>
                </div>
              </div>
              <div class="nav-sub-item-controls">
                <div class="nav-sub-sort-group nav-sub-sort-pill toggle-pill" role="group" aria-label="Sort conversations">
                  <button type="button" class="nav-sub-sort-btn active" aria-label="Sort by recent activity">
                    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M12 7v6l4 2"></path></svg>
                  </button>
                  <button type="button" class="nav-sub-sort-btn" aria-label="Sort by topology">
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 7h10"></path><path d="M5 12h14"></path><path d="M5 17h8"></path></svg>
                  </button>
                </div>
              </div>
              <div class="chat-sidebar-list" aria-label="Agent selector">
                \${agentRows.map((agent) => \`
                  <button type="button" data-agent-id="\${escapeHtml(agent.id)}" class="chat-sidebar-item \${agent.id === selectedAgentId ? 'active' : ''}" \${state.disabled ? 'disabled' : ''}>
                    <span class="chat-sidebar-item-avatar agent-mark infring-logo"><span class="infring-logo-glyph">\${escapeHtml((agent.label || agent.id || 'A').slice(0, 1).toUpperCase())}</span></span>
                    <span class="chat-sidebar-item-main">
                      <span class="chat-sidebar-item-name">\${escapeHtml(agent.label || agent.id)}</span>
                      <span class="chat-sidebar-item-preview">\${escapeHtml(agent.state || 'Gateway projection')}</span>
                    </span>
                  </button>
                \`).join('')}
              </div>
            </div>
          </div>
        </div>
      </aside>
      <main class="main-content" aria-label="Legacy dashboard main surface">
        <div class="global-taskbar is-docked-top" data-shell-primitive="taskbar-dock">
          <div class="global-taskbar-left">
            <div class="taskbar-visual-group taskbar-visual-group-left" aria-label="Primary taskbar items">
              <div class="taskbar-hero-menu-anchor">
                <button class="taskbar-brand taskbar-brand-trigger" type="button" title="System actions">
                  <div class="brand-mark infring-logo" aria-hidden="true"><span class="brand-mark-glyph infring-logo-glyph">&infin;</span></div>
                  <div><div class="taskbar-brand-title">INFRING</div></div>
                </button>
              </div>
              <div class="taskbar-reorder-box taskbar-reorder-box-left">
                <div class="taskbar-reorder-item taskbar-reorder-nav-cluster taskbar-nav-pill">
                  <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn taskbar-back-btn" type="button" aria-label="Back">
                    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15 18-6-6 6-6"></path></svg>
                  </button>
                  <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn taskbar-forward-btn" type="button" aria-label="Forward">
                    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 18 6-6-6-6"></path></svg>
                  </button>
                </div>
              </div>
              <div class="taskbar-text-menus">
                <button class="taskbar-text-menu-btn" type="button" aria-label="Help menu">Help</button>
              </div>
              <div class="global-taskbar-page-slot"></div>
            </div>
          </div>
          <div class="global-taskbar-right">
            <div class="taskbar-visual-group taskbar-visual-group-right" aria-label="System taskbar items">
              <div class="taskbar-agent-indicator">
                <span class="taskbar-agent-indicator-icon" aria-hidden="true"><svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="8"></circle></svg></span>
                <span class="taskbar-agent-indicator-text">\${escapeHtml(runtimeBadge)}</span>
              </div>
              <button class="btn btn-ghost btn-sm taskbar-icon-btn" type="button" aria-label="Theme">◐</button>
              <span class="conn-badge">\${escapeHtml(state.runtimeLabel)}</span>
            </div>
          </div>
        </div>
        <div class="chat-wrapper">
          <div class="chat-thread-topline">
            <button class="chat-thread-profile chat-thread-profile-disabled" type="button" aria-label="Current agent">
              <span class="chat-thread-profile-avatar agent-mark infring-logo"><span class="infring-logo-glyph">&infin;</span></span>
              <span class="chat-thread-profile-copy">
                <span class="chat-thread-profile-name">\${escapeHtml(selectedAgentLabel)}</span>
                <span class="chat-thread-profile-subtitle">\${escapeHtml(selectedSessionId || 'No session selected')}</span>
              </span>
            </button>
          </div>
          <div class="messages" id="messages" aria-label="Message window">
            <div class="chat-reflection-overlay" aria-hidden="true"></div>
            <div class="chat-grid-overlay" aria-hidden="true"></div>
            \${messages.length ? messages.map((message) => \`
              <article class="message \${message.role === 'user' ? 'user' : 'agent'} meta-collapsed">
                <div class="message-avatar agent-mark infring-logo" aria-hidden="true"><span class="infring-logo-glyph">\${message.role === 'user' ? 'Y' : '∞'}</span></div>
                <div class="message-body">
                  <div class="message-bubble markdown-body">
                    <span class="message-agent-name"><span class="message-agent-name-label">\${escapeHtml(message.role === 'user' ? 'You' : selectedAgentLabel)}</span></span>
                    <p class="message-bubble-content">\${escapeHtml(message.text)}</p>
                    \${message.detail_ref ? \`<button class="message-stat-btn" type="button" data-detail-ref="\${escapeHtml(message.detail_ref)}" \${state.disabled ? 'disabled' : ''}>View detail</button>\` : ''}
                    \${message.status ? \`<div class="message-stats-row"><span class="message-stat-meta">\${escapeHtml(message.status)}</span></div>\` : ''}
                  </div>
                </div>
              </article>
            \`).join('') : '<div class="empty-state"><h4>No bounded message projection loaded yet.</h4><p class="hint">Select an agent from the legacy-style rail or send a message through the composer.</p></div>'}
          </div>
          <div class="chat-map" aria-label="Message map">
            <div class="chat-map-surface drag-bar overlay-shared-surface">
              <div class="chat-map-rail">
                <button class="chat-map-jump chat-map-jump-up" type="button" aria-label="Previous message"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="m18 15-6-6-6 6"></path></svg></button>
                <div class="chat-map-items-wrap"><div class="chat-map-viewport"><div class="chat-map-scroll">
                  <div class="chat-map-spacer" aria-hidden="true"></div>
                  \${messages.map((message, index) => \`<div class="chat-map-entry"><button class="chat-map-item role-\${escapeHtml(message.role === 'user' ? 'user' : 'agent')}" type="button" aria-label="Message \${index + 1}"><span class="chat-map-item-main"><span class="chat-map-bar"></span></span></button></div>\`).join('')}
                  <div class="chat-map-spacer" aria-hidden="true"></div>
                </div></div></div>
                <button class="chat-map-jump chat-map-jump-down" type="button" aria-label="Next message"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="m6 9 6 6 6-6"></path></svg></button>
              </div>
            </div>
          </div>
          <form class="input-area browser-shell-v2__input">
            <div class="chat-input-lane">
              <div class="composer-display-pill">
                <div class="composer-shell">
                  <div class="composer-main-row">
                    <button class="composer-menu-pill composer-shared-input-pill" type="button" aria-label="Menu">☰</button>
                    <div class="composer-input-pill composer-shared-input-pill">
                      <input name="message" \${state.disabled ? 'disabled' : ''} placeholder="Message Infring..." aria-label="Shell input" />
                    </div>
                    <div class="composer-controls-pill">
                      <button class="btn btn-primary btn-send" \${state.disabled ? 'disabled' : ''} type="submit">Send</button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </form>
        </div>
        \${activeDetailRef ? \`
          <div class="popup-window dashboard-popup-surface browser-shell-v2__detail" aria-label="Lazy message detail">
            <div class="popup-window-header"><h3 class="popup-window-title">\${escapeHtml(activeDetailPanel?.title || activeDetailRef)}</h3></div>
            <div class="popup-window-body"><p>\${escapeHtml(activeDetailPanel?.summary || activeDetailPreview || 'Detail projection loaded.')}</p></div>
          </div>
        \` : ''}
      </main>
      \${activeDetailRef ? \`
        <template data-v2-detail-shadow>
        <section class="browser-shell-v2__detail" aria-label="Lazy message detail" hidden>
          <div class="browser-shell-v2__detail-header">
            <div>
              <p class="browser-shell-v2__label">Lazy Detail</p>
              <strong>\${escapeHtml(activeDetailPanel?.title || activeDetailRef)}</strong>
            </div>
            \${activeDetailPanel?.kind ? \`<small>\${escapeHtml(activeDetailPanel.kind)}</small>\` : ''}
          </div>
          <p>\${escapeHtml(activeDetailPanel?.summary || activeDetailPreview || 'Detail projection loaded.')}</p>
          \${activeDetailPanel?.rows?.length ? \`
            <div class="browser-shell-v2__detail-grid" aria-label="Detail projection rows">
              \${activeDetailPanel.rows.map((row) => \`
                <article><span>\${escapeHtml(row.label)}</span>\${row.meta ? \`<small>\${escapeHtml(row.meta)}</small>\` : ''}</article>
              \`).join('')}
            </div>
          \` : ''}
          \${activeDetailPanel?.refs?.length || activeDetailPanel?.cursor || activeDetailPanel?.receipt_ref ? \`
            <div class="browser-shell-v2__detail-refs" aria-label="Detail refs">
              \${(activeDetailPanel.refs || []).map((ref) => \`<code>\${escapeHtml(ref)}</code>\`).join('')}
              \${activeDetailPanel.cursor ? \`<code>\${escapeHtml(activeDetailPanel.cursor)}</code>\` : ''}
              \${activeDetailPanel.receipt_ref ? \`<code>\${escapeHtml(activeDetailPanel.receipt_ref)}</code>\` : ''}
            </div>
          \` : ''}
        </section>
        </template>
      \` : ''}
      <section class="browser-shell-v2__events" aria-label="Gateway event projection" hidden>
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
      <section class="browser-shell-v2__search" aria-label="Bounded Gateway search" hidden>
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
      <section class="browser-shell-v2__issue" aria-label="Gateway issue evaluation request" hidden>
        <form class="browser-shell-v2__issue-form">
          <label>
            <span class="browser-shell-v2__label">Issue / Eval Request</span>
            <input name="issue" value="\${escapeHtml(issueNote)}" \${state.disabled ? 'disabled' : ''} placeholder="Ask Gateway to inspect this context..." aria-label="Issue note" />
          </label>
          <button type="submit" \${state.disabled || !selectedSessionId ? 'disabled' : ''}>Submit</button>
        </form>
        \${issueStatus || issueReceiptRef ? \`<p class="browser-shell-v2__issue-status"><strong>\${escapeHtml(issueStatus || 'submitted')}</strong>\${issueReceiptRef ? \`<span>\${escapeHtml(issueReceiptRef)}</span>\` : ''}</p>\` : ''}
      </section>
      <section class="browser-shell-v2__approval" aria-label="Gateway approval decision request" hidden>
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
      <section class="browser-shell-v2__controls" aria-label="Gateway selection requests" hidden>
        <form class="browser-shell-v2__control-form" data-control-form="model">
          <label>
            <span class="browser-shell-v2__label">Model Request</span>
            <input name="model" value="\${escapeHtml(modelSelection)}" \${state.disabled ? 'disabled' : ''} placeholder="auto, gpt-5.4, ..." aria-label="Model selection" />
          </label>
          <button type="submit" \${state.disabled || !selectedAgentId ? 'disabled' : ''}>Submit</button>
        </form>
        <div class="browser-shell-v2__control-menu" aria-label="Model selector">
          \${modelRows.length ? modelRows.map((model) => \`
            <button type="button" data-model-id="\${escapeHtml(model.id)}" class="\${model.id === modelSelection ? 'active' : ''}" \${state.disabled || !selectedAgentId ? 'disabled' : ''}>
              <span>\${escapeHtml(model.label || model.id)}</span>\${model.meta ? \`<small>\${escapeHtml(model.meta)}</small>\` : ''}
            </button>
          \`).join('') : '<span class="browser-shell-v2__control-empty">No model projection loaded.</span>'}
        </div>
        \${modelStatus || modelReceiptRef ? \`<p class="browser-shell-v2__control-status"><strong>\${escapeHtml(modelStatus || 'submitted')}</strong>\${modelReceiptRef ? \`<span>\${escapeHtml(modelReceiptRef)}</span>\` : ''}</p>\` : ''}
        <form class="browser-shell-v2__control-form" data-control-form="git-tree">
          <label>
            <span class="browser-shell-v2__label">Git Tree Request</span>
            <input name="git-tree" value="\${escapeHtml(gitTreeSelection)}" \${state.disabled ? 'disabled' : ''} placeholder="workspace, branch, tree ref..." aria-label="Git tree selection" />
          </label>
          <button type="submit" \${state.disabled || !selectedAgentId ? 'disabled' : ''}>Submit</button>
        </form>
        <div class="browser-shell-v2__control-menu" aria-label="Git tree selector">
          \${gitTreeRows.length ? gitTreeRows.map((tree) => \`
            <button type="button" data-git-tree-id="\${escapeHtml(tree.id)}" class="\${tree.id === gitTreeSelection ? 'active' : ''}" \${state.disabled || !selectedAgentId ? 'disabled' : ''}>
              <span>\${escapeHtml(tree.label || tree.id)}</span>\${tree.meta ? \`<small>\${escapeHtml(tree.meta)}</small>\` : ''}
            </button>
          \`).join('') : '<span class="browser-shell-v2__control-empty">No git tree projection loaded.</span>'}
        </div>
        \${gitTreeStatus || gitTreeReceiptRef ? \`<p class="browser-shell-v2__control-status"><strong>\${escapeHtml(gitTreeStatus || 'submitted')}</strong>\${gitTreeReceiptRef ? \`<span>\${escapeHtml(gitTreeReceiptRef)}</span>\` : ''}</p>\` : ''}
      </section>
      <form class="browser-shell-v2__input" hidden>
        <input name="message" \${state.disabled ? 'disabled' : ''} placeholder="Send through Shell Socket..." aria-label="Shell input" />
        <button \${state.disabled ? 'disabled' : ''} type="submit">Send</button>
      </form>
    </div>\`;
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
  root.querySelectorAll('[data-model-id]').forEach((button) => button.addEventListener('click', () => setModel(button.getAttribute('data-model-id') || '', state)));
  root.querySelectorAll('[data-git-tree-id]').forEach((button) => button.addEventListener('click', () => setGitTree(button.getAttribute('data-git-tree-id') || '', state)));
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
  if (!selectedSessionId || eventRefreshInFlight) return;
  eventRefreshInFlight = true;
  render({ ...state, disabled: true, runtimeLabel: 'Refreshing event projection...' });
  try {
    const events = await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=' + encodeURIComponent(eventCursor));
    applyEventProjection(events, true);
    render({ ...state, disabled: !selectedAgentId });
  } finally {
    eventRefreshInFlight = false;
  }
}

async function pollEventProjection(state) {
  if (!selectedSessionId || eventRefreshInFlight) return;
  eventRefreshInFlight = true;
  try {
    const events = await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=' + encodeURIComponent(eventCursor));
    applyEventProjection(events, true);
    render({ ...state, disabled: !selectedAgentId });
  } finally {
    eventRefreshInFlight = false;
  }
}

function applyEventProjection(events, append = false) {
  const nextRows = rowsFromEvents(events);
  eventRows = append ? eventRows.concat(nextRows).slice(-20) : nextRows;
  eventCursor = clean(events.next_cursor || events.cursor || (nextRows[nextRows.length - 1] || {}).cursor || eventCursor, 160);
}

function startEventProjectionStream(state) {
  if (eventPollTimer) window.clearInterval(eventPollTimer);
  eventPollTimer = window.setInterval(() => {
    void pollEventProjection(state);
  }, EVENT_POLL_INTERVAL_MS);
}

window.addEventListener('beforeunload', () => {
  if (eventPollTimer) window.clearInterval(eventPollTimer);
});

async function openMessageDetail(detailRef, state) {
  const cleanDetailRef = clean(detailRef, 300);
  if (!cleanDetailRef) return;
  render({ ...state, disabled: true, runtimeLabel: 'Loading lazy detail projection...' });
  const detail = await socketRequest('get_message_detail', '/api/shell-socket/details/' + encodeURIComponent(cleanDetailRef) + '?view=summary&limit=1');
  activeDetailRef = cleanDetailRef;
  activeDetailPanel = detailPanelFromProjection(cleanDetailRef, detail);
  activeDetailPreview = activeDetailPanel.summary;
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
  applyEventProjection(events, false);
  if (selectedSessionId) startEventProjectionStream({ runtimeState: 'ready', runtimeLabel: 'Selected agent projection loaded.', messages: rowsFromMessageWindow(messages), disabled: !selectedAgentId });
  render({ runtimeState: 'ready', runtimeLabel: 'Selected agent projection loaded.', messages: rowsFromMessageWindow(messages), disabled: !selectedAgentId });
}

async function selectSession(sessionId) {
  const cleanSessionId = clean(sessionId, 240);
  if (!cleanSessionId) return;
  render({ runtimeState: 'loading', runtimeLabel: 'Loading selected session projection...', messages: [], disabled: true });
  selectedSessionId = cleanSessionId;
  const messages = await socketRequest('get_message_window', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/messages?limit=' + MESSAGE_WINDOW_LIMIT);
  const events = await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=');
  applyEventProjection(events, false);
  startEventProjectionStream({ runtimeState: 'ready', runtimeLabel: 'Selected session projection loaded.', messages: rowsFromMessageWindow(messages), disabled: !selectedAgentId });
  render({ runtimeState: 'ready', runtimeLabel: 'Selected session projection loaded.', messages: rowsFromMessageWindow(messages), disabled: !selectedAgentId });
}

async function hydrate() {
  render({ runtimeState: 'loading', runtimeLabel: 'Hydrating from Shell Socket Gateway projection...', messages: [], disabled: true });
  try {
    const runtime = await socketRequest('get_runtime_status', '/api/shell-socket/runtime-status');
    modelRows = rowsFromSelectorOptions(runtime, ['model_options', 'models', 'model_rows'], [
      { id: 'auto', label: 'Auto', meta: 'Gateway chooses the admitted model.' },
    ]);
    gitTreeRows = rowsFromSelectorOptions(runtime, ['git_tree_options', 'git_trees', 'workspace_trees'], [
      { id: 'workspace', label: 'Workspace', meta: 'Current Gateway workspace tree.' },
    ]);
    modelSelection = clean(runtime.selected_model || runtime.model_id || modelSelection || modelRows[0]?.id || '', 160);
    gitTreeSelection = clean(runtime.selected_git_tree || runtime.tree_id || gitTreeSelection || gitTreeRows[0]?.id || '', 240);
    const agents = await socketRequest('list_agents', '/api/shell-socket/agents?limit=40');
    agentRows = rowsFromAgents(agents);
    selectedAgentId = firstAgentId(agents) || agentRows[0]?.id || '';
    const sessions = selectedAgentId ? await socketRequest('list_sessions', '/api/shell-socket/agents/' + encodeURIComponent(selectedAgentId) + '/sessions?limit=40') : {};
    sessionRows = rowsFromSessions(sessions);
    selectedSessionId = firstSessionId(sessions) || sessionRows[0]?.id || '';
    const messages = selectedSessionId ? await socketRequest('get_message_window', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/messages?limit=' + MESSAGE_WINDOW_LIMIT) : {};
    const events = selectedSessionId ? await socketRequest('subscribe_events', '/api/shell-socket/sessions/' + encodeURIComponent(selectedSessionId) + '/events?cursor=' + encodeURIComponent(eventCursor)) : {};
    applyEventProjection(events, false);
    const nextState = {
      runtimeState: clean(runtime.state || 'unknown', 80),
      runtimeLabel: clean(runtime.label || 'Runtime projection received.', 240),
      messages: rowsFromMessageWindow(messages),
      disabled: !selectedAgentId,
    };
    if (selectedSessionId) startEventProjectionStream(nextState);
    render(nextState);
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
  const css = `${legacySurfaceCss()}\n\n/* Browser Shell V2 surface parity adapter */\n${fs.readFileSync(path.resolve(process.cwd(), CSS_PATH), 'utf8')}`;
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
  <body data-theme="light" class="browser-shell-v2-body">
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
