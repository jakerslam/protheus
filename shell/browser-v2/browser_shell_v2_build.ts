#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { compile } from 'svelte/compiler';

const OUT_DIR = 'core/local/artifacts/browser_shell_v2_app';
const COMPONENT_PATH = 'shell/browser-v2/BrowserShellV2.svelte';
const CSS_PATH = 'shell/browser-v2/browser_shell_v2.css';
const LEGACY_CSS_DIR = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'css'].join('/');
const LEGACY_STATIC_DIR = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static'].join('/');
const LEGACY_BOTTOM_DOCK_BUNDLE = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'js', 'svelte', 'bottom_dock_shell.bundle.ts'].join('/');
const LEGACY_DASHBOARD_POPUP_OVERLAY_BUNDLE = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'js', 'svelte', 'dashboard_popup_overlay_shell.bundle.ts'].join('/');
const LEGACY_SIDEBAR_AGENT_LIST_BUNDLE = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'js', 'svelte', 'sidebar_agent_list_shell.bundle.ts'].join('/');
const LEGACY_CHAT_MAP_SHELL_BUNDLE = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'js', 'svelte', 'chat_map_shell.bundle.ts'].join('/');
const LEGACY_CHAT_MAP_RAIL_BUNDLE = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'js', 'svelte', 'chat_map_rail_shell.bundle.ts'].join('/');
const LEGACY_CHAT_MAP_VIEWPORT_BUNDLE = ['client', 'runtime', 'systems', 'ui', 'infring' + '_static', 'js', 'svelte', 'chat_map_viewport_shell.bundle.ts'].join('/');
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
      return fs.readFileSync(absPath, 'utf8');
    })
    .join('');
}

function legacyDockIconDefs(): string {
  const bodyPart = fs.readFileSync(path.resolve(process.cwd(), LEGACY_STATIC_DIR, 'index_body.html.parts/0001-body-part.part03.html'), 'utf8');
  return bodyPart.split('<infring-bottom-dock-shell')[0] || '';
}

function browserRuntimeSource(): string {
  return `const DEFAULT_GATEWAY_URL = 'http://127.0.0.1:5173';
const MESSAGE_WINDOW_LIMIT = 40;
const EVENT_POLL_INTERVAL_MS = 5000;
const DOCK_ICON_DEFS = ${JSON.stringify(legacyDockIconDefs())};
const DOCK_ICON_DEFS_MARKER = 'dock-icon-defs';

function clean(value, max = 1000) {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function gatewayBaseUrl() {
  const params = new URLSearchParams(location.search);
  return clean(params.get('gateway') || DEFAULT_GATEWAY_URL, 300).replace(/\\/+$/, '');
}

async function socketRequest(capability, path, options = {}) {
  const requestInit = {
    method: options.method || 'GET',
    headers: {
      accept: 'application/json',
      ...(options.body ? { 'content-type': 'application/json' } : {}),
    },
    body: options.body ? JSON.stringify(options.body) : undefined,
  };
  let response;
  try {
    response = await fetch(gatewayBaseUrl() + path, requestInit);
  } catch (error) {
    if (!gatewayBaseUrl()) throw error;
    response = await fetch(path, requestInit);
  }
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

const dockTileRegistry = {
  chat: { icon: 'messages', tone: 'message', tooltip: 'Messages', label: 'Messages' },
  overview: { icon: 'home', tone: 'bright', tooltip: 'Home', label: 'Home' },
  agents: { icon: 'agents', tone: 'bright', tooltip: 'Agents', label: 'Agents' },
  scheduler: { icon: 'automation', tone: 'muted', tooltip: 'Automation', label: 'Automation', animation: ['automation-gears', 1200] },
  skills: { icon: 'apps', tone: 'default', tooltip: 'Apps', label: 'Apps' },
  runtime: { icon: 'system', tone: 'bright', tooltip: 'System', label: 'System', animation: ['system-terminal', 2000] },
  settings: { icon: 'settings', tone: 'muted', tooltip: 'Settings', label: 'Settings', animation: ['spin', 4000] },
};
const browserShellV2DisplayState = {
  page: 'chat',
  bottomDockOrder: Object.keys(dockTileRegistry),
  bottomDockHoverId: '',
  bottomDockClickAnimatingId: '',
  bottomDockDragActive: false,
  bottomDockSide: 'bottom',
  bottomDockAnchorX: 0,
  bottomDockAnchorY: 0,
  bottomDockPointerMoveHandler: null,
  bottomDockPointerUpHandler: null,
  sidebarCollapsed: false,
  chatSidebarDragActive: false,
  chatSidebarLeft: 16,
  chatSidebarTop: 96,
  taskbarDockDragActive: false,
  taskbarDockEdge: 'top',
  taskbarDockY: 0,
  themeMode: 'system',
  resolvedTheme: 'light',
  dashboardPopup: {
    id: '',
    source: '',
    active: false,
    ready: false,
    side: 'top',
    inline_away: 'center',
    block_away: 'center',
    left: -9999,
    top: -9999,
    title: '',
    body: '',
    meta_origin: '',
    meta_time: '',
    unread: false,
  },
};

let lastRenderedState = null;

function overlayWallGapPx() {
  return 16;
}

function chatSidebarStyle() {
  const left = Math.round(Number(browserShellV2DisplayState.chatSidebarLeft || overlayWallGapPx()));
  const top = Math.round(Number(browserShellV2DisplayState.chatSidebarTop || 96));
  return [
    'position:fixed',
    'left:' + left + 'px',
    'top:' + top + 'px',
    'right:auto',
    'bottom:auto',
    'height:fit-content',
    'min-height:calc(56px * 3)',
    'max-height:80vh',
    'transform:none',
    '--sidebar-position-transition:' + (browserShellV2DisplayState.chatSidebarDragActive ? '0ms' : '280ms'),
  ].join(';');
}

function chatSidebarNavShellStyle() {
  return 'flex:0 1 auto;min-height:0;max-height:calc(80vh - 16px);';
}

function chatSidebarNavStyle() {
  return 'height:auto;flex:0 1 auto;max-height:calc(80vh - 16px);';
}

function sidebarPulltabStyle() {
  return [
    'position:absolute',
    'left:100%',
    'right:auto',
    'top:50%',
    'transform:translateY(-50%)',
    '--sidebar-position-transition:' + (browserShellV2DisplayState.chatSidebarDragActive ? '0ms' : '280ms'),
  ].join(';');
}

function chatMapStyle() {
  return [
    'position:fixed',
    'right:18px',
    'top:50%',
    'bottom:auto',
    'transform:translateY(-50%)',
    'max-height:70vh',
    '--chat-map-position-transition:280ms',
  ].join(';');
}

function taskbarDockStyle() {
  const edge = browserShellV2DisplayState.taskbarDockEdge === 'bottom' ? 'bottom' : 'top';
  const y = edge === 'bottom' ? Math.max(0, window.innerHeight - 46) : 0;
  browserShellV2DisplayState.taskbarDockY = y;
  return [
    '--taskbar-dock-drag-y:' + Math.round(y) + 'px',
    '--taskbar-dock-position-transition:' + (browserShellV2DisplayState.taskbarDockDragActive ? '0ms' : '220ms'),
  ].join(';');
}

function pageTitle(page) {
  return ({
    overview: 'Overview',
    agents: 'Agents',
    scheduler: 'Scheduler',
    skills: 'Apps',
    runtime: 'Runtime',
    settings: 'Settings',
  })[page] || 'Shell';
}

function renderCurrentPageShell() {
  const page = clean(browserShellV2DisplayState.page || 'chat', 80);
  if (page === 'chat') return '';
  const title = pageTitle(page);
  return \`
    <section class="chat-wrapper shell-v2-page-shell" aria-label="\${escapeHtml(title)}">
      <div class="page-header">
        <div>
          <div class="text-xs text-muted font-mono">Browser Shell V2</div>
          <h2>\${escapeHtml(title)}</h2>
        </div>
      </div>
      <div class="empty-state">
        <h4>\${escapeHtml(title)}</h4>
        <p class="hint">This page slot is wired to the shell navigation contract. The next pass should fill it with Gateway projection data only.</p>
      </div>
    </section>
  \`;
}

function createProjectionStore(initial) {
  let value = initial;
  const subscribers = new Set();
  return {
    get: () => value,
    set: (next) => {
      value = next;
      subscribers.forEach((subscriber) => {
        try { subscriber(value); } catch (_error) {}
      });
    },
    update: (updater) => {
      value = typeof updater === 'function' ? updater(value) : value;
      subscribers.forEach((subscriber) => {
        try { subscriber(value); } catch (_error) {}
      });
    },
    subscribe: (subscriber) => {
      subscribers.add(subscriber);
      try { subscriber(value); } catch (_error) {}
      return () => subscribers.delete(subscriber);
    },
  };
}

function legacySidebarAgentRows() {
  return agentRows.map((agent) => {
    const preview = clean(agent.state || 'Gateway projection', 240);
    return {
      ...agent,
      name: clean(agent.label || agent.id, 160),
      active: agent.id === selectedAgentId,
      sidebar_status_state: agent.state === 'connected' ? 'connected' : agent.state || 'unknown',
      sidebar_preview: {
        text: preview,
        ts: Date.now(),
        unread_response: false,
      },
    };
  });
}

function legacyChatMapRows(messages) {
  return messages.map((message, index) => ({
    key: clean(message.id || 'message-' + index, 180) + '-' + index,
    domId: 'browser-v2-message-' + index,
    index,
    role: message.role === 'user' ? 'user' : 'agent',
    longMessage: clean(message.text || '', 12000).length > 900,
    markerType: '',
    markerTitle: '',
    toolOutcome: '',
    newDay: index === 0,
    dayLabel: 'Current session',
    dayCollapsed: false,
  }));
}

function messageHasTail(messages, index) {
  const current = messages[index] || {};
  const next = messages[index + 1] || null;
  if (!next) return true;
  return clean(next.role || 'agent', 40) !== clean(current.role || 'agent', 40);
}

function installProjectionStores() {
  const existingStore = window.InfringChatStore || {};
  if (!existingStore.sidebarAgents) existingStore.sidebarAgents = createProjectionStore([]);
  if (!existingStore.currentAgent) existingStore.currentAgent = createProjectionStore(null);
  if (!existingStore.agents) existingStore.agents = createProjectionStore([]);
  if (!existingStore.mapRows) existingStore.mapRows = createProjectionStore([]);
  if (!existingStore.mapStepIndex) existingStore.mapStepIndex = createProjectionStore(-1);
  existingStore.refreshMapRows = (messages) => {
    existingStore.mapRows.set(legacyChatMapRows(Array.isArray(messages) ? messages : []));
  };
  window.InfringChatStore = existingStore;

  const existingPage = window.InfringChatPage || {};
  window.InfringChatPage = {
    ...existingPage,
    messages: [],
    filteredMessages: [],
    currentAgent: null,
    chatMapDragActive: false,
    page: 'chat',
    chatSidebarPreview: (agent) => agent?.sidebar_preview || { text: clean(agent?.state || 'Gateway projection', 240), ts: Date.now() },
    formatChatSidebarTime: () => '',
    agentStatusState: (agent) => clean(agent?.sidebar_status_state || agent?.state || 'unknown', 80).toLowerCase() || 'unknown',
    agentStatusLabel: (agent) => clean(agent?.state || 'Gateway projection', 200),
    sidebarDisplayEmoji: () => '',
    isAgentLiveBusy: () => false,
    shouldShowExpiryCountdown: () => false,
    shouldShowInfinityLifespan: () => false,
    chatSidebarCanReorderTopology: () => false,
    isSidebarArchivedAgent: () => false,
    normalizeSidebarPopupText: (text) => clean(text, 400),
    sidebarPopupMetaOrigin: () => 'Gateway projection',
    selectAgentChatFromSidebar: (agent) => selectAgent(clean(agent?.id || '', 160)),
    showCollapsedSidebarAgentPopup: (agent, event) => {
      const method = window.InfringSharedShellServices?.appStore?.method?.('showDashboardPopup');
      if (method) method('sidebar-agent:' + clean(agent?.id || '', 120), clean(agent?.name || agent?.id || 'Agent', 160), event, { source: 'sidebar', side: 'right', body: clean(agent?.sidebar_preview?.text || '', 400), meta_origin: 'Gateway projection' });
    },
    hideDashboardPopupBySource: (source) => window.InfringSharedShellServices?.appStore?.method?.('hideDashboardPopupBySource')?.(source),
    showDashboardPopup: (id, title, event, overrides) => window.InfringSharedShellServices?.appStore?.method?.('showDashboardPopup')?.(id, title, event, overrides),
    hideDashboardPopup: (id) => window.InfringSharedShellServices?.appStore?.method?.('hideDashboardPopup')?.(id),
    startChatMapPointerDrag: () => {},
    stepMessageMap: (messages, direction) => {
      const store = window.InfringChatStore;
      const current = Number(store?.mapStepIndex?.get?.() || 0);
      const length = Array.isArray(messages) ? messages.length : 0;
      const next = Math.max(0, Math.min(length - 1, current + Number(direction || 0)));
      store?.mapStepIndex?.set?.(next);
      const target = document.querySelector('[data-msg-index="' + next + '"]');
      if (target && typeof target.scrollIntoView === 'function') target.scrollIntoView({ block: 'center', behavior: 'smooth' });
    },
    showMapItemPopup: (message, index, event) => {
      const title = (message?.role === 'user' ? 'You' : selectedAgentId || 'Agent') + ' message';
      const body = clean(message?.text || '', 400);
      window.InfringSharedShellServices?.appStore?.method?.('showDashboardPopup')?.('chat-map:' + index, title, event, { source: 'chat-map', side: 'left', body, meta_origin: 'Message map' });
    },
    hideMapItemPopup: () => window.InfringSharedShellServices?.appStore?.method?.('hideDashboardPopupBySource')?.('chat-map'),
    jumpToMessage: (_message, index) => {
      const target = document.querySelector('[data-msg-index="' + Number(index) + '"]');
      if (target && typeof target.scrollIntoView === 'function') target.scrollIntoView({ block: 'center', behavior: 'smooth' });
    },
    toggleMessageDayCollapse: () => {},
    showMapDayPopup: (_message, event) => window.InfringSharedShellServices?.appStore?.method?.('showDashboardPopup')?.('chat-map-day', 'Current session', event, { source: 'chat-map', side: 'left', body: 'Gateway message window projection', meta_origin: 'Message map' }),
    hideMapDayPopup: () => window.InfringSharedShellServices?.appStore?.method?.('hideDashboardPopupBySource')?.('chat-map'),
  };
}

function syncLegacyDisplayProjection(state) {
  const sidebarRows = legacySidebarAgentRows();
  const currentAgent = sidebarRows.find((agent) => agent.id === selectedAgentId) || sidebarRows[0] || null;
  const messages = Array.isArray(state.messages) ? state.messages : [];
  browserShellV2DisplayState.page = clean(browserShellV2DisplayState.page || 'chat', 80);
  browserShellV2DisplayState.activeAgentId = selectedAgentId;
  browserShellV2DisplayState.chatSidebarRows = sidebarRows;
  browserShellV2DisplayState.chatSidebarVisibleRows = sidebarRows;
  const store = window.InfringChatStore;
  if (store) {
    store.sidebarAgents?.set?.(sidebarRows);
    store.agents?.set?.(sidebarRows);
    store.currentAgent?.set?.(currentAgent);
    store.mapRows?.set?.(legacyChatMapRows(messages));
  }
  if (window.InfringChatPage) {
    window.InfringChatPage.page = browserShellV2DisplayState.page;
    window.InfringChatPage.messages = messages;
    window.InfringChatPage.filteredMessages = messages;
    window.InfringChatPage.currentAgent = currentAgent;
    window.InfringChatPage.activeAgentId = selectedAgentId;
  }
}

function installDisplayOnlyShellServices() {
  const services = window.InfringSharedShellServices || {};
  const orderIndex = (id) => Math.max(0, browserShellV2DisplayState.bottomDockOrder.indexOf(id));
  const viewportSide = (x, y) => {
    const width = Math.max(1, window.innerWidth || 1);
    const height = Math.max(1, window.innerHeight || 1);
    const distances = [
      ['top', y],
      ['bottom', height - y],
      ['left', x],
      ['right', width - x],
    ].sort((a, b) => Number(a[1]) - Number(b[1]));
    return distances[0][0] || 'bottom';
  };
  const openSideForDockSide = (side) => ({ top: 'bottom', bottom: 'top', left: 'right', right: 'left' }[side] || 'top');
  const anchorForSide = (side, x, y) => {
    const width = Math.max(1, window.innerWidth || 1);
    const height = Math.max(1, window.innerHeight || 1);
    if (side === 'top') return { x: Math.max(80, Math.min(width - 80, x || width / 2)), y: 4 };
    if (side === 'left') return { x: 4, y: Math.max(80, Math.min(height - 80, y || height / 2)) };
    if (side === 'right') return { x: width - 4, y: Math.max(80, Math.min(height - 80, y || height / 2)) };
    return { x: Math.max(80, Math.min(width - 80, x || width / 2)), y: height - 4 };
  };
  const setDockAnchor = (side, x, y) => {
    const anchor = anchorForSide(side, x, y);
    browserShellV2DisplayState.bottomDockSide = side;
    browserShellV2DisplayState.bottomDockAnchorX = anchor.x;
    browserShellV2DisplayState.bottomDockAnchorY = anchor.y;
  };
  const defaultPopup = () => ({
    id: '',
    source: '',
    active: false,
    ready: false,
    side: 'top',
    inline_away: 'center',
    block_away: 'center',
    left: -9999,
    top: -9999,
    title: '',
    body: '',
    meta_origin: '',
    meta_time: '',
    unread: false,
  });
  const popupFromEvent = (id, title, event, overrides = {}) => {
    const target = event?.currentTarget || event?.target;
    const rect = target && typeof target.getBoundingClientRect === 'function'
      ? target.getBoundingClientRect()
      : { left: Number(event?.clientX || 0), right: Number(event?.clientX || 0), top: Number(event?.clientY || 0), bottom: Number(event?.clientY || 0), width: 0, height: 0 };
    const centerX = rect.left + (rect.width || 0) / 2;
    const centerY = rect.top + (rect.height || 0) / 2;
    const width = Math.max(1, window.innerWidth || 1);
    const height = Math.max(1, window.innerHeight || 1);
    const inlineAway = centerX < width / 2 ? 'right' : 'left';
    const blockAway = centerY < height / 2 ? 'bottom' : 'top';
    const side = overrides.side || (Math.min(centerY, height - centerY) < Math.min(centerX, width - centerX) ? blockAway : inlineAway);
    const left = side === 'left' ? rect.left : side === 'right' ? rect.right : centerX;
    const top = side === 'top' ? rect.top : side === 'bottom' ? rect.bottom : centerY;
    return {
      ...defaultPopup(),
      id: clean(id, 220),
      source: clean(overrides.source || id, 120),
      active: true,
      ready: true,
      side,
      inline_away: inlineAway,
      block_away: blockAway,
      left,
      top,
      title: clean(title, 220),
      body: clean(overrides.body || '', 1000),
      meta_origin: clean(overrides.meta_origin || '', 160),
      meta_time: clean(overrides.meta_time || '', 160),
      unread: !!overrides.unread,
    };
  };
  setDockAnchor(browserShellV2DisplayState.bottomDockSide);
  services.popup = {
    origin: (overrides) => ({ ...defaultPopup(), ...(overrides || {}) }),
    stateOrigin: (popup) => ({ ...defaultPopup(), ...(popup || {}) }),
    overlayClass: (popup, glassKind) => ({
      [glassKind || 'fogged-glass']: true,
      'is-visible': !!(popup && popup.active && popup.ready && popup.title),
      'is-side-top': popup?.side === 'top',
      'is-side-bottom': popup?.side === 'bottom',
      'is-side-left': popup?.side === 'left',
      'is-side-right': popup?.side === 'right',
      'is-inline-away-left': popup?.inline_away === 'left',
      'is-inline-away-right': popup?.inline_away === 'right',
      'is-inline-away-center': popup?.inline_away !== 'left' && popup?.inline_away !== 'right',
      'is-block-away-top': popup?.block_away === 'top',
      'is-block-away-bottom': popup?.block_away === 'bottom',
      'is-block-away-center': popup?.block_away !== 'top' && popup?.block_away !== 'bottom',
      'is-unread': !!popup?.unread,
    }),
    overlayStyle: (popup) => {
      if (!popup || !popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
      return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
    },
  };
  services.appStore = {
    root: () => browserShellV2DisplayState,
    current: () => browserShellV2DisplayState,
    set: (key, value) => { browserShellV2DisplayState[key] = value; },
    method: (name) => {
      const methods = {
        normalizeBottomDockOrder: (order) => {
          const defaults = Object.keys(dockTileRegistry);
          const seen = new Set();
          return (Array.isArray(order) ? order : []).concat(defaults)
            .map((id) => clean(id, 80))
            .filter((id) => dockTileRegistry[id] && !seen.has(id) && seen.add(id));
        },
        bottomDockTileData: (id, field, fallback) => (dockTileRegistry[id] && dockTileRegistry[id][field]) || fallback || '',
        bottomDockActiveSide: () => browserShellV2DisplayState.bottomDockSide,
        bottomDockOpenSide: () => openSideForDockSide(browserShellV2DisplayState.bottomDockSide),
        bottomDockWallLockNormalized: () => '',
        bottomDockTaskbarContained: () => false,
        bottomDockHoverExpansionDisabled: () => false,
        bottomDockContainerStyle: () => {
          const anchor = anchorForSide(
            browserShellV2DisplayState.bottomDockSide,
            browserShellV2DisplayState.bottomDockAnchorX,
            browserShellV2DisplayState.bottomDockAnchorY,
          );
          return [
            '--bottom-dock-anchor-x:' + Math.round(anchor.x) + 'px',
            '--bottom-dock-anchor-y:' + Math.round(anchor.y) + 'px',
            '--bottom-dock-position-transition:' + (browserShellV2DisplayState.bottomDockDragActive ? '0ms' : '220ms'),
          ].join(';');
        },
        bottomDockSlotStyle: (id) => {
          const hoverIndex = orderIndex(browserShellV2DisplayState.bottomDockHoverId);
          const index = orderIndex(id);
          const distance = browserShellV2DisplayState.bottomDockHoverId ? Math.abs(index - hoverIndex) : 99;
          const weight = distance === 0 ? 1 : distance === 1 ? 0.62 : distance === 2 ? 0.34 : 0;
          return 'order:' + index + ';--bottom-dock-hover-weight:' + weight.toFixed(4);
        },
        bottomDockTileStyle: () => '',
        bottomDockIsNeighbor: (id) => {
          if (!browserShellV2DisplayState.bottomDockHoverId) return false;
          return Math.abs(orderIndex(id) - orderIndex(browserShellV2DisplayState.bottomDockHoverId)) === 1;
        },
        bottomDockIsSecondNeighbor: (id) => {
          if (!browserShellV2DisplayState.bottomDockHoverId) return false;
          return Math.abs(orderIndex(id) - orderIndex(browserShellV2DisplayState.bottomDockHoverId)) === 2;
        },
        bottomDockIsDraggingVisual: () => browserShellV2DisplayState.bottomDockDragActive,
        bottomDockIsClickAnimating: (id) => browserShellV2DisplayState.bottomDockClickAnimatingId === id,
        bottomDockTileAnimationName: (id) => (dockTileRegistry[id]?.animation || [])[0] || '',
        bottomDockTileAnimationDurationAttr: (id) => String((dockTileRegistry[id]?.animation || [])[1] || ''),
        appsIconBottomRowFill: (index) => ['#22c55e', '#06b6d4', '#f97316'][Number(index) || 0] || '#22c55e',
        setBottomDockHover: (id) => { browserShellV2DisplayState.bottomDockHoverId = clean(id, 80); },
        clearBottomDockHover: (id) => {
          if (!id || browserShellV2DisplayState.bottomDockHoverId === id) browserShellV2DisplayState.bottomDockHoverId = '';
        },
        updateBottomDockPointer: (event) => {
          if (!browserShellV2DisplayState.bottomDockDragActive || !event) return;
          const x = Number(event.clientX || 0);
          const y = Number(event.clientY || 0);
          setDockAnchor(viewportSide(x, y), x, y);
        },
        startBottomDockContainerPointerDrag: (event) => {
          if (!event || event.button > 0) return;
          browserShellV2DisplayState.bottomDockDragActive = true;
          const move = (ev) => {
            const x = Number(ev.clientX || 0);
            const y = Number(ev.clientY || 0);
            setDockAnchor(viewportSide(x, y), x, y);
          };
          const end = (ev) => {
            move(ev || event);
            browserShellV2DisplayState.bottomDockDragActive = false;
            window.removeEventListener('pointermove', move, true);
            window.removeEventListener('pointerup', end, true);
            window.removeEventListener('pointercancel', end, true);
          };
          window.addEventListener('pointermove', move, true);
          window.addEventListener('pointerup', end, true);
          window.addEventListener('pointercancel', end, true);
        },
        startBottomDockPointerDrag: (_id, event) => {
          const target = event?.target;
          if (target && typeof target.closest === 'function' && target.closest('.dock-tile')) return;
          methods.startBottomDockContainerPointerDrag(event);
        },
        handleBottomDockTileClick: (id) => {
          browserShellV2DisplayState.page = clean(id, 80) || 'chat';
          browserShellV2DisplayState.bottomDockClickAnimatingId = clean(id, 80);
          if (lastRenderedState) render(lastRenderedState);
          window.setTimeout(() => {
            if (browserShellV2DisplayState.bottomDockClickAnimatingId === id) browserShellV2DisplayState.bottomDockClickAnimatingId = '';
          }, 900);
        },
        showDashboardPopup: (id, title, event, overrides) => {
          browserShellV2DisplayState.dashboardPopup = popupFromEvent(id, title, event, overrides || {});
        },
        hideDashboardPopup: (id) => {
          if (!id || browserShellV2DisplayState.dashboardPopup.id === id) browserShellV2DisplayState.dashboardPopup = defaultPopup();
        },
        hideDashboardPopupBySource: (source) => {
          if (!source || browserShellV2DisplayState.dashboardPopup.source === source) browserShellV2DisplayState.dashboardPopup = defaultPopup();
        },
      };
      return methods[name] || null;
    },
  };
  window.InfringSharedShellServices = services;
}

installProjectionStores();
installDisplayOnlyShellServices();

function resolveTheme(mode) {
  const cleanMode = clean(mode || browserShellV2DisplayState.themeMode || 'system', 40);
  if (cleanMode === 'light' || cleanMode === 'dark') return cleanMode;
  return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function applyDisplayTheme(mode) {
  const nextMode = ['light', 'dark', 'system'].includes(clean(mode, 40)) ? clean(mode, 40) : 'system';
  browserShellV2DisplayState.themeMode = nextMode;
  browserShellV2DisplayState.resolvedTheme = resolveTheme(nextMode);
  document.body.setAttribute('data-theme', browserShellV2DisplayState.resolvedTheme);
  document.documentElement.setAttribute('data-theme', browserShellV2DisplayState.resolvedTheme);
  document.documentElement.dataset.uiBackgroundTemplate = clean(document.documentElement.dataset.uiBackgroundTemplate || 'default-grid', 80);
}

applyDisplayTheme('system');

function startSidebarDrag(event, state) {
  if (!event || event.button > 0) return;
  const target = event.target;
  if (target && typeof target.closest === 'function' && target.closest('button,input,textarea,select,a,[role="button"]') && !target.closest('[data-dragbar-pulltab="chat-sidebar"]')) return;
  event.preventDefault();
  browserShellV2DisplayState.chatSidebarDragActive = true;
  const startX = Number(event.clientX || 0);
  const startY = Number(event.clientY || 0);
  const originLeft = Number(browserShellV2DisplayState.chatSidebarLeft || overlayWallGapPx());
  const originTop = Number(browserShellV2DisplayState.chatSidebarTop || 96);
  const move = (ev) => {
    const maxLeft = Math.max(overlayWallGapPx(), window.innerWidth - 280);
    const maxTop = Math.max(overlayWallGapPx(), window.innerHeight - 160);
    browserShellV2DisplayState.chatSidebarLeft = Math.max(overlayWallGapPx(), Math.min(maxLeft, originLeft + Number(ev.clientX || 0) - startX));
    browserShellV2DisplayState.chatSidebarTop = Math.max(overlayWallGapPx(), Math.min(maxTop, originTop + Number(ev.clientY || 0) - startY));
    render(state);
  };
  const end = (ev) => {
    move(ev || event);
    browserShellV2DisplayState.chatSidebarDragActive = false;
    window.removeEventListener('pointermove', move, true);
    window.removeEventListener('pointerup', end, true);
    window.removeEventListener('pointercancel', end, true);
    render(state);
  };
  window.addEventListener('pointermove', move, true);
  window.addEventListener('pointerup', end, true);
  window.addEventListener('pointercancel', end, true);
}

function startTaskbarDrag(event, state) {
  if (!event || event.button > 0) return;
  const target = event.target;
  if (target && typeof target.closest === 'function' && target.closest('button,input,textarea,select,a,[role="button"]')) return;
  event.preventDefault();
  browserShellV2DisplayState.taskbarDockDragActive = true;
  const move = (ev) => {
    browserShellV2DisplayState.taskbarDockEdge = Number(ev.clientY || 0) > (window.innerHeight / 2) ? 'bottom' : 'top';
    render(state);
  };
  const end = (ev) => {
    move(ev || event);
    browserShellV2DisplayState.taskbarDockDragActive = false;
    window.removeEventListener('pointermove', move, true);
    window.removeEventListener('pointerup', end, true);
    window.removeEventListener('pointercancel', end, true);
    render(state);
  };
  window.addEventListener('pointermove', move, true);
  window.addEventListener('pointerup', end, true);
  window.addEventListener('pointercancel', end, true);
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
  lastRenderedState = state;
  syncLegacyDisplayProjection(state);
  const root = document.querySelector('#browser-shell-v2-root');
  if (!root) throw new Error('browser_shell_v2_root_missing');
  const messages = state.messages || [];
  const selectedAgentLabel = selectedAgentId || 'No agent selected';
  const runtimeBadge = clean(state.runtimeState || 'unknown', 80);
  const page = clean(browserShellV2DisplayState.page || 'chat', 80);
  const isChatPage = page === 'chat';
  root.innerHTML = \`
    <div class="app-layout \${browserShellV2DisplayState.taskbarDockEdge === 'bottom' ? 'taskbar-bottom' : ''}" data-shell-plug="browser-v2" data-event-cursor="\${escapeHtml(eventCursor)}" data-receipt-ref="\${escapeHtml(issueReceiptRef || approvalReceiptRef || modelReceiptRef || gitTreeReceiptRef)}" aria-label="Browser Shell V2">
      <div class="main-pointer-fx-layer" aria-hidden="true"></div>
      <infring-sidebar-rail-shell class="sidebar drag-bar overlay-shared-surface \${isChatPage ? 'chat-sidebar-dynamic' : 'chat-only-hidden'} \${browserShellV2DisplayState.sidebarCollapsed ? 'collapsed' : ''} \${browserShellV2DisplayState.chatSidebarDragActive ? 'is-container-dragging' : ''}" dragbarsurface="chat-sidebar" parentownedmechanics="true" style="\${isChatPage ? chatSidebarStyle() : ''}" aria-label="Legacy dashboard conversation rail">
        <div class="sidebar-nav-shell" style="\${chatSidebarNavShellStyle()}">
          <div class="sidebar-nav" role="navigation" aria-label="Main navigation" style="\${chatSidebarNavStyle()}">
            <div class="sidebar-top-ghost" aria-hidden="true"></div>
            <div class="nav-section" aria-label="Agent conversations">
              <a class="nav-item sidebar-tab-item \${page === 'chat' ? 'active' : ''}" data-page-id="chat" aria-current="\${page === 'chat' ? 'page' : 'false'}">
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
              <infring-sidebar-agent-list-shell></infring-sidebar-agent-list-shell>
              <div class="chat-sidebar-list" aria-label="Session selector">
                \${sessionRows.map((session) => \`
                  <button type="button" data-session-id="\${escapeHtml(session.id)}" class="chat-sidebar-item \${session.id === selectedSessionId ? 'active' : ''}" \${state.disabled ? 'disabled' : ''}>
                    <span class="chat-sidebar-item-avatar agent-mark infring-logo"><span class="infring-logo-glyph">S</span></span>
                    <span class="chat-sidebar-item-main">
                      <span class="chat-sidebar-item-name">\${escapeHtml(session.label || session.id)}</span>
                      <span class="chat-sidebar-item-preview">\${escapeHtml(session.message_count ? String(session.message_count) + ' messages' : 'Window projection')}</span>
                    </span>
                  </button>
                \`).join('')}
              </div>
              <a class="nav-item sidebar-tab-item \${['agents','sessions','approvals'].includes(page) ? 'active' : ''}" data-page-id="agents" aria-current="\${['agents','sessions','approvals'].includes(page) ? 'page' : 'false'}">
                <span class="nav-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg></span>
                <span class="nav-label">Agents</span>
              </a>
            </div>
            <div class="nav-section sidebar-tab-section" aria-label="Automation">
              <a class="nav-item sidebar-tab-item \${['scheduler','workflows'].includes(page) ? 'active' : ''}" data-page-id="scheduler" aria-current="\${['scheduler','workflows'].includes(page) ? 'page' : 'false'}">
                <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg></span>
                <span class="nav-label">Automation</span>
              </a>
            </div>
            <div class="nav-section sidebar-tab-section" aria-label="Apps">
              <a class="nav-item sidebar-tab-item \${['skills','channels','eyes','hands'].includes(page) ? 'active' : ''}" data-page-id="skills" aria-current="\${['skills','channels','eyes','hands'].includes(page) ? 'page' : 'false'}">
                <span class="nav-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="4" y="4" width="6" height="6" rx="1.5"></rect><rect x="14" y="4" width="6" height="6" rx="1.5"></rect><rect x="4" y="14" width="6" height="6" rx="1.5"></rect><rect x="14" y="14" width="6" height="6" rx="1.5"></rect></svg></span>
                <span class="nav-label">Apps</span>
              </a>
            </div>
            <div class="nav-section sidebar-tab-section" aria-label="System">
              <a class="nav-item sidebar-tab-item \${['runtime','analytics','logs'].includes(page) ? 'active' : ''}" data-page-id="runtime" aria-current="\${['runtime','analytics','logs'].includes(page) ? 'page' : 'false'}">
                <span class="nav-icon"><svg viewBox="0 0 24 24"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/></svg></span>
                <span class="nav-label">System</span>
              </a>
            </div>
          </div>
        </div>
        <button
          class="overlay-pulltab-object sidebar-pulltab drag-bar drag-bar-pulltab overlay-shared-surface pulltab-border-top-active pulltab-border-right-active pulltab-border-bottom-active pulltab-border-left-inactive"
          data-dragbar-pulltab="chat-sidebar"
          style="\${sidebarPulltabStyle()}"
          type="button"
          aria-label="Toggle sidebar"
        >
          <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-top-left" aria-hidden="true"></span>
          <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-top-right" aria-hidden="true"></span>
          <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-bottom-left" aria-hidden="true"></span>
          <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-bottom-right" aria-hidden="true"></span>
          <svg class="overlay-pulltab-object-icon sidebar-pulltab-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m15 6-6 6 6 6"></path>
          </svg>
        </button>
      </infring-sidebar-rail-shell>
      <div class="sidebar-overlay"></div>
      <main class="main-content" aria-label="Legacy dashboard main surface">
        <div class="global-taskbar \${browserShellV2DisplayState.taskbarDockDragActive ? 'is-dock-dragging' : ''} \${browserShellV2DisplayState.taskbarDockEdge === 'bottom' ? 'is-docked-bottom' : 'is-docked-top'}" data-shell-primitive="taskbar-dock" style="\${taskbarDockStyle()}">
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
            <infring-taskbar-system-items-shell shellprimitive="taskbar-dock" wrapperrole="taskbar-system-items" parentownedmechanics="true">
            <div class="taskbar-visual-group taskbar-visual-group-right" aria-label="System taskbar items">
              <div class="taskbar-reorder-box taskbar-reorder-box-right">
                <div class="taskbar-reorder-item" data-taskbar-item="connectivity">
                  <div class="global-taskbar-controls">
                    <button class="health-indicator taskbar-agent-indicator \${state.runtimeState === 'connected' ? 'health-ok' : 'health-connecting'}" type="button" aria-label="Open agents" title="\${escapeHtml(state.runtimeLabel)}">
                      <span class="taskbar-agent-indicator-icon" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path></svg></span>
                      <span class="taskbar-agent-indicator-text">\${escapeHtml(runtimeBadge)}</span>
                    </button>
                  </div>
                </div>
                <div class="taskbar-reorder-item" data-taskbar-item="theme">
                  <div class="theme-switcher toggle-pill" data-mode="\${escapeHtml(browserShellV2DisplayState.themeMode)}" data-resolved="\${escapeHtml(browserShellV2DisplayState.resolvedTheme)}" role="group" aria-label="Theme">
                    <button class="theme-opt \${browserShellV2DisplayState.themeMode === 'light' ? 'active' : ''}" data-theme-mode="light" type="button" title="Light" aria-label="Light theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="4"></circle><path d="M12 2v2"></path><path d="M12 20v2"></path><path d="m4.93 4.93 1.41 1.41"></path><path d="m17.66 17.66 1.41 1.41"></path><path d="M2 12h2"></path><path d="M20 12h2"></path><path d="m6.34 17.66-1.41 1.41"></path><path d="m19.07 4.93-1.41 1.41"></path></svg></button>
                    <button class="theme-opt \${browserShellV2DisplayState.themeMode === 'system' ? 'active' : ''}" data-theme-mode="system" type="button" title="System" aria-label="System theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="3" width="20" height="14" rx="2"></rect><path d="M8 21h8"></path><path d="M12 17v4"></path></svg></button>
                    <button class="theme-opt \${browserShellV2DisplayState.themeMode === 'dark' ? 'active' : ''}" data-theme-mode="dark" type="button" title="Dark" aria-label="Dark theme"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.79A9 9 0 1 1 11.21 3A7 7 0 0 0 21 12.79z"></path></svg></button>
                  </div>
                </div>
                <div class="taskbar-reorder-item" data-taskbar-item="notifications">
                  <div id="taskbar-notification-menu-anchor" class="notif-wrap">
                    <button class="btn btn-ghost btn-sm taskbar-icon-btn notif-btn" type="button" title="Notifications" aria-label="Notifications">
                      <svg class="notif-bell-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M15 17h5l-1.4-1.4A2 2 0 0 1 18 14.2V11a6 6 0 1 0-12 0v3.2a2 2 0 0 1-.6 1.4L4 17h5"></path><path d="M9 17a3 3 0 0 0 6 0"></path></svg>
                    </button>
                  </div>
                </div>
                <div class="taskbar-reorder-item" data-taskbar-item="search">
                  <button class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-search-btn" type="button" aria-label="Search" aria-disabled="true"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.05" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="6"></circle><path d="m20 20-3.7-3.7"></path></svg></button>
                </div>
                <div class="taskbar-reorder-item" data-taskbar-item="auth">
                  <button class="btn btn-ghost btn-sm taskbar-icon-btn auth-key-btn" type="button" aria-label="Authentication"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5" y="11" width="14" height="10" rx="2"></rect><path d="M8 11V8a4 4 0 0 1 8 0v3"></path><circle cx="12" cy="16" r="1"></circle></svg></button>
                </div>
                <div class="taskbar-reorder-item" data-taskbar-item="clock">
                  <span class="taskbar-clock" aria-label="Clock">\${new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                </div>
              </div>
            </div>
            </infring-taskbar-system-items-shell>
          </div>
        </div>
        \${isChatPage ? \`
        <div class="chat-wrapper">
          <infring-chat-header-shell>
          <div class="chat-thread-topline">
            <div class="chat-thread-profile-center">
              <div class="chat-thread-profile warped-glass chat-thread-profile-disabled" role="button" tabindex="-1" title="Agent details">
                <div class="chat-thread-profile-avatar">
                  <span class="infring-logo infring-logo--agent-default" aria-hidden="true"><span class="infring-logo-glyph" aria-hidden="true">&infin;</span></span>
                </div>
                <div class="chat-thread-profile-info-pill">
                  <div class="chat-thread-profile-meta">
                    <span class="agent-status-dot chat-title-status-dot \${state.runtimeState === 'connected' ? 'status-connected' : ''}" aria-hidden="true"></span>
                    <div class="chat-thread-profile-name">\${escapeHtml(selectedAgentLabel)}</div>
                  </div>
                  <div class="chat-thread-heart-meter" title="\${escapeHtml(selectedSessionId || 'No session selected')}">
                    <span class="chat-thread-heart" aria-hidden="true">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M12 21s-7-4.2-9-8.4C1.5 9.5 3.3 6 6.4 6c2.2 0 3.4 1.2 3.9 2.1.5-.9 1.7-2.1 3.9-2.1 3.1 0 4.9 3.5 3.4 6.6-2 4.2-9 8.4-9 8.4z"></path></svg>
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
          </infring-chat-header-shell>
          <infring-messages-surface-shell>
          <div class="messages" id="messages" aria-label="Message window">
            <div class="chat-reflection-overlay" aria-hidden="true"></div>
            <div class="chat-grid-overlay" aria-hidden="true"></div>
            \${messages.length ? messages.map((message, index) => \`
              <article id="browser-v2-message-\${index}" class="message \${message.role === 'user' ? 'user' : 'agent'} \${messageHasTail(messages, index) ? 'has-tail' : ''} meta-collapsed" data-msg-index="\${index}">
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
          </infring-messages-surface-shell>
          <infring-chat-map-shell class="chat-map" dragbarsurface="chat-map" parentownedmechanics="true" style="\${chatMapStyle()}" aria-label="Message map"></infring-chat-map-shell>
          <infring-chat-input-footer-shell>
          <form class="input-area">
            <div class="chat-input-lane">
              <div class="composer-stack">
              <div class="input-row">
                <div class="composer-shell">
                  <div class="composer-main-row">
                    <div class="composer-display-pill" aria-label="Message input controls">
                      <div class="composer-menu-pill composer-shared-input-pill">
                        <div class="composer-plus-wrap composer-icon-left">
                          <button class="composer-icon-btn composer-hamburger-btn" type="button" aria-label="Add files and more">
                            <svg class="composer-hamburger-icon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><line x1="4" y1="7" x2="20" y2="7"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="17" x2="20" y2="17"/></svg>
                          </button>
                        </div>
                      </div>
                      <div class="composer-input-pill composer-shared-input-pill">
                        <textarea id="msg-input" name="message" rows="1" \${state.disabled ? 'disabled' : ''} placeholder="Message Infring..." aria-label="Shell input"></textarea>
                      </div>
                      <div class="composer-controls-pill composer-shared-input-pill">
                        <div class="composer-actions-right">
                          <div class="toggle-pill toggle-pill--triple input-toggle-wrapper" data-mode="text" role="group" aria-label="Voice and send controls">
                            <button type="button" class="composer-send-voice-opt composer-send-voice-opt-attach" aria-label="Add files"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/></svg></button>
                            <button type="button" class="composer-send-voice-opt composer-send-voice-opt-voice" aria-label="Toggle voice recording"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" x2="12" y1="19" y2="22"/></svg></button>
                            <button class="composer-send-voice-opt composer-send-voice-opt-send" \${state.disabled ? 'disabled' : ''} type="submit" aria-label="Send message"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"></line><polyline points="5 12 12 5 19 12"></polyline></svg></button>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              </div>
            </div>
          </form>
          </infring-chat-input-footer-shell>
        </div>
        \` : renderCurrentPageShell()}
        \${activeDetailRef ? \`
          <div class="popup-window dashboard-popup-surface" aria-label="Lazy message detail">
            <div class="popup-window-header"><h3 class="popup-window-title">\${escapeHtml(activeDetailPanel?.title || activeDetailRef)}</h3></div>
            <div class="popup-window-body">
              <p>\${escapeHtml(activeDetailPanel?.summary || activeDetailPreview || 'Detail projection loaded.')}</p>
              \${activeDetailPanel?.rows?.length ? \`<div>\${activeDetailPanel.rows.map((row) => \`<p><strong>\${escapeHtml(row.label)}</strong>\${row.meta ? \` <span>\${escapeHtml(row.meta)}</span>\` : ''}</p>\`).join('')}</div>\` : ''}
            </div>
          </div>
        \` : ''}
      </main>
      \${DOCK_ICON_DEFS}
      <infring-bottom-dock-shell shellprimitive="taskbar-dock" parentownedmechanics="true"></infring-bottom-dock-shell>
      <infring-dashboard-popup-overlay-shell></infring-dashboard-popup-overlay-shell>
    </div>\`;
  const form = root.querySelector('.input-area');
  root.querySelectorAll('[data-page-id]').forEach((button) => button.addEventListener('click', () => {
    browserShellV2DisplayState.page = clean(button.getAttribute('data-page-id') || 'chat', 80) || 'chat';
    render(state);
  }));
  root.querySelectorAll('[data-agent-id]').forEach((button) => button.addEventListener('click', () => selectAgent(button.getAttribute('data-agent-id') || '')));
  root.querySelectorAll('[data-session-id]').forEach((button) => button.addEventListener('click', () => selectSession(button.getAttribute('data-session-id') || '')));
  root.querySelectorAll('[data-detail-ref]').forEach((button) => button.addEventListener('click', () => openMessageDetail(button.getAttribute('data-detail-ref') || '', state)));
  root.querySelectorAll('[data-theme-mode]').forEach((button) => button.addEventListener('click', () => {
    applyDisplayTheme(button.getAttribute('data-theme-mode') || 'system');
    render(state);
  }));
  root.querySelector('[data-dragbar-pulltab="chat-sidebar"]')?.addEventListener('click', () => {
    browserShellV2DisplayState.sidebarCollapsed = !browserShellV2DisplayState.sidebarCollapsed;
    render(state);
  });
  root.querySelector('.sidebar')?.addEventListener('pointerdown', (event) => startSidebarDrag(event, state), true);
  root.querySelector('.global-taskbar')?.addEventListener('pointerdown', (event) => startTaskbarDrag(event, state), true);
  root.querySelectorAll('[data-map-index]').forEach((button) => button.addEventListener('click', () => {
    const index = Number(button.getAttribute('data-map-index'));
    const target = Number.isFinite(index) ? root.querySelector('[data-msg-index="' + index + '"]') : null;
    if (target && typeof target.scrollIntoView === 'function') target.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }));
  root.querySelector('.chat-map-jump-up')?.addEventListener('click', () => root.querySelector('.messages')?.scrollBy({ top: -260, behavior: 'smooth' }));
  root.querySelector('.chat-map-jump-down')?.addEventListener('click', () => root.querySelector('.messages')?.scrollBy({ top: 260, behavior: 'smooth' }));
  root.querySelectorAll('[title], [aria-label]').forEach((node) => {
    if (!node.closest('.bottom-dock')) {
      node.addEventListener('mouseenter', (event) => {
        const title = node.getAttribute('title') || node.getAttribute('aria-label') || '';
        if (!title) return;
        const method = window.InfringSharedShellServices?.appStore?.method?.('showDashboardPopup');
        if (method) method('browser-v2:' + title.toLowerCase().replace(/[^a-z0-9]+/g, '-'), title, event, { source: 'browser-v2', meta_origin: 'Browser Shell V2' });
      });
      node.addEventListener('mouseleave', () => {
        const method = window.InfringSharedShellServices?.appStore?.method?.('hideDashboardPopupBySource');
        if (method) method('browser-v2');
      });
    }
  });
  form?.addEventListener('submit', async (event) => {
    event.preventDefault();
    const input = form.querySelector('textarea, input');
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
  // No V2 CSS is appended. Visual parity means the artifact CSS is exactly the legacy dashboard CSS bundle.
  const css = legacySurfaceCss();
  const targetDir = path.resolve(process.cwd(), outDir);
  fs.rmSync(targetDir, { recursive: true, force: true });
  fs.mkdirSync(targetDir, { recursive: true });
  write(path.join(outDir, 'index.html'), `<!doctype html>
<html lang="en" data-ui-background-template="default-grid">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Infring Browser Shell V2</title>
    <link rel="stylesheet" href="./browser_shell_v2.css" />
  </head>
  <body data-theme="light">
    <div id="browser-shell-v2-root"></div>
    <script src="./sidebar_agent_list_shell.bundle.js"></script>
    <script src="./chat_map_rail_shell.bundle.js"></script>
    <script src="./chat_map_viewport_shell.bundle.js"></script>
    <script src="./chat_map_shell.bundle.js"></script>
    <script src="./bottom_dock_shell.bundle.js"></script>
    <script src="./dashboard_popup_overlay_shell.bundle.js"></script>
    <script type="module" src="./browser_shell_v2_app.js"></script>
  </body>
</html>
`);
  write(path.join(outDir, 'browser_shell_v2.css'), css);
  write(path.join(outDir, 'sidebar_agent_list_shell.bundle.js'), fs.readFileSync(path.resolve(process.cwd(), LEGACY_SIDEBAR_AGENT_LIST_BUNDLE), 'utf8'));
  write(path.join(outDir, 'chat_map_shell.bundle.js'), fs.readFileSync(path.resolve(process.cwd(), LEGACY_CHAT_MAP_SHELL_BUNDLE), 'utf8'));
  write(path.join(outDir, 'chat_map_rail_shell.bundle.js'), fs.readFileSync(path.resolve(process.cwd(), LEGACY_CHAT_MAP_RAIL_BUNDLE), 'utf8'));
  write(path.join(outDir, 'chat_map_viewport_shell.bundle.js'), fs.readFileSync(path.resolve(process.cwd(), LEGACY_CHAT_MAP_VIEWPORT_BUNDLE), 'utf8'));
  write(path.join(outDir, 'bottom_dock_shell.bundle.js'), fs.readFileSync(path.resolve(process.cwd(), LEGACY_BOTTOM_DOCK_BUNDLE), 'utf8'));
  write(path.join(outDir, 'dashboard_popup_overlay_shell.bundle.js'), fs.readFileSync(path.resolve(process.cwd(), LEGACY_DASHBOARD_POPUP_OVERLAY_BUNDLE), 'utf8'));
  write(path.join(outDir, 'browser_shell_v2_app.js'), browserRuntimeSource());
  write(path.join(outDir, 'svelte_component_preflight.js'), compiled.js.code);
  return {
    ok: warnings.length === 0,
    type: 'browser_shell_v2_build',
    out_dir: outDir,
    files: ['index.html', 'browser_shell_v2.css', 'sidebar_agent_list_shell.bundle.js', 'chat_map_rail_shell.bundle.js', 'chat_map_viewport_shell.bundle.js', 'chat_map_shell.bundle.js', 'bottom_dock_shell.bundle.js', 'dashboard_popup_overlay_shell.bundle.js', 'browser_shell_v2_app.js', 'svelte_component_preflight.js'],
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
