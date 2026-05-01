// Infring App — legacy reactive init, hash router, global store
'use strict';

// Marked.js configuration
if (typeof marked !== 'undefined') {
  marked.setOptions({
    breaks: true,
    gfm: true,
    highlight: function(code, lang) {
      if (typeof hljs !== 'undefined' && lang && hljs.getLanguage(lang)) {
        try { return hljs.highlight(code, { language: lang }).value; } catch(e) {}
      }
      return code;
    }
  });
}

function escapeHtml(text) {
  var div = document.createElement('div');
  div.textContent = text || '';
  return div.innerHTML;
}

function normalizeChatMarkdownListBreaks(text) {
  var source = String(text || '');
  if (!source) return '';
  var normalized = source.replace(/\r\n/g, '\n');
  normalized = normalized.replace(/[ \t]+•[ \t]+/g, '\n- ');
  normalized = normalized.replace(/(:\s+)(\*\*[^*\n]{1,120}\*\*:)/g, function(_match, prefix, marker) {
    return prefix.replace(/\s+$/, '') + '\n' + marker;
  });
  normalized = normalized.replace(/([.!?])\s+(\*\*[^*\n]{1,120}\*\*:)/g, function(_match, punctuation, marker) {
    return punctuation + '\n' + marker;
  });
  normalized = normalized.replace(/(:\n)(\*\*[^*\n]{1,120}\*\*:)/, '$1- $2');
  var out = '';
  var i = 0;
  var inCodeFence = false;
  var atLineStart = true;
  var lineHasContent = false;
  var isLikelyListMarkerAt = function(value, index) {
    var tail = String(value || '').slice(index);
    var match = tail.match(/^(\*\*\d{1,3}[.)]\s+[^*\n]+\*\*|\d{1,3}[.)]\s+|[*+-]\s+)/);
    if (!match) return null;
    var marker = String(match[1] || '');
    if (/^[*+-]\s+$/.test(marker)) {
      var nextChar = tail.charAt(marker.length);
      if (!nextChar || /\s/.test(nextChar)) return null;
    }
    if (/^\d/.test(marker)) {
      var prev = index > 0 ? value.charAt(index - 1) : '';
      if (/\d/.test(prev)) return null;
    }
    return marker;
  };
  while (i < normalized.length) {
    if ((i === 0 || normalized.charAt(i - 1) === '\n') && normalized.slice(i, i + 3) === '```') {
      inCodeFence = !inCodeFence;
    }
    if (!inCodeFence && !atLineStart) {
      var marker = isLikelyListMarkerAt(normalized, i);
      if (marker && lineHasContent) {
        var prevChar = out.length ? out.charAt(out.length - 1) : '';
        if (prevChar !== '\n') out += '\n';
        atLineStart = true;
        lineHasContent = false;
      }
    }
    var ch = normalized.charAt(i);
    out += ch;
    if (ch === '\n') {
      atLineStart = true;
      lineHasContent = false;
    } else {
      if (!/\s/.test(ch)) lineHasContent = true;
      atLineStart = false;
    }
    i += 1;
  }
  return out.replace(/\n{3,}/g, '\n\n');
}

function renderMarkdown(text) {
  if (!text) return '';
  if (typeof marked !== 'undefined') {
    // Protect LaTeX blocks from marked.js mangling (underscores, backslashes, etc.)
    var latexBlocks = [];
    var protected_ = normalizeChatMarkdownListBreaks(text);
    // Protect display math $$...$$ first (greedy across lines)
    protected_ = protected_.replace(/\$\$([\s\S]+?)\$\$/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect inline math $...$ (single line, not empty, not starting/ending with space)
    protected_ = protected_.replace(/\$([^\s$](?:[^$]*[^\s$])?)\$/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect \[...\] display math
    protected_ = protected_.replace(/\\\[([\s\S]+?)\\\]/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect \(...\) inline math
    protected_ = protected_.replace(/\\\(([\s\S]+?)\\\)/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });

    var html = marked.parse(protected_);
    // Restore LaTeX blocks
    for (var i = 0; i < latexBlocks.length; i++) {
      html = html.replace('\x00LATEX' + i + '\x00', latexBlocks[i]);
    }
    // Upgrade markdown render cards for richer code/table ergonomics.
    if (typeof dashboardWrapMarkdownCodeBlocks === 'function') {
      html = dashboardWrapMarkdownCodeBlocks(html);
    }
    if (typeof dashboardWrapMarkdownTables === 'function') {
      html = dashboardWrapMarkdownTables(html);
    }
    // Open external links in new tab
    html = html.replace(/<a\s+href="(https?:\/\/[^"]*)"(?![^>]*target=)([^>]*)>/gi, '<a href="$1" target="_blank" rel="noopener"$2>');
    return html;
  }
  return escapeHtml(text);
}

function infringEnsureChatStoreBridge() {
  if (typeof window === 'undefined') return null;
  if (window.InfringChatStore && typeof window.InfringChatStore.syncMessages === 'function') return window.InfringChatStore;
  function writable(initialValue) {
    var value = initialValue;
    var subscribers = [];
    return {
      subscribe: function(run) {
        if (typeof run !== 'function') return function() {};
        subscribers.push(run);
        try { run(value); } catch (_) {}
        return function() { subscribers = subscribers.filter(function(row) { return row !== run; }); };
      },
      set: function(next) {
        value = next;
        subscribers.slice().forEach(function(run) { try { run(value); } catch (_) {} });
      },
      update: function(fn) { if (typeof fn === 'function') this.set(fn(value)); },
      get: function() { return value; }
    };
  }
  function chatPage() { return window.InfringChatPage || null; }
  function callPage(fn) {
    var page = chatPage();
    if (!page || typeof page[fn] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return page[fn].apply(page, args); } catch (_) { return undefined; }
  }
  function fallbackDayKey(msg) {
    if (!msg || !msg.ts) return '';
    var day = new Date(msg.ts);
    if (Number.isNaN(day.getTime())) return '';
    return day.getFullYear() + '-' + String(day.getMonth() + 1).padStart(2, '0') + '-' + String(day.getDate()).padStart(2, '0');
  }
  function mapMarkerType(msg) {
    var fromPage = callPage('messageMapMarkerType', msg);
    if (fromPage != null) return String(fromPage || '');
    if (msg && msg.is_notice) return String(msg.notice_type || '').toLowerCase() === 'info' ? 'info' : 'model';
    if (msg && msg.terminal) return 'terminal';
    if (msg && Array.isArray(msg.tools) && msg.tools.length) return 'tool';
    return '';
  }
  function mapToolOutcome(msg) {
    var fromPage = callPage('messageMapToolOutcome', msg);
    if (fromPage != null) return String(fromPage || '');
    if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
    for (var i = 0; i < msg.tools.length; i += 1) {
      var tool = msg.tools[i] || {};
      if (tool.is_error) return 'error';
      if (tool.running) return 'warning';
    }
    return 'success';
  }
  function buildMapRows(rows) {
    var list = Array.isArray(rows) ? rows : [];
    var out = [];
    for (var i = 0; i < list.length; i += 1) {
      var msg = list[i] || {};
      var dayKey = String(callPage('messageDayKey', msg) || fallbackDayKey(msg) || '');
      var prevDayKey = i > 0 ? String(callPage('messageDayKey', list[i - 1]) || fallbackDayKey(list[i - 1]) || '') : '';
      out.push({
        index: i,
        key: String(callPage('messageRenderKey', msg, i) || msg.id || msg.ts || i),
        domId: String(callPage('messageDomId', msg, i) || ('message-' + i)),
        role: String(msg.role || 'agent').trim() || 'agent',
        isNotice: !!msg.is_notice,
        noticeIcon: String(msg.notice_icon || 'i'),
        newDay: i === 0 || (!!dayKey && dayKey !== prevDayKey),
        dayKey: dayKey,
        dayLabel: String(callPage('messageDayLabel', msg) || dayKey || 'Unknown day'),
        dayCollapsed: !!callPage('isMessageDayCollapsed', msg),
        markerType: mapMarkerType(msg),
        markerTitle: String(callPage('messageMapMarkerTitle', msg) || ''),
        toolOutcome: mapToolOutcome(msg),
        longMessage: !!callPage('isLongMessagePreview', msg)
      });
    }
    return out;
  }
  var queuedMessageSync = false;
  var pendingMessages = [];
  var pendingFilteredMessages = [];
  var lastFilteredMessageSource = [];
  var threadProjectionCenterIndex = -1;
  var threadProjectionLimit = 80;
  function scheduleMessageStoreFlush(store) {
    if (queuedMessageSync) return;
    queuedMessageSync = true;
    var flush = function() {
      queuedMessageSync = false;
      store.messages.set(pendingMessages);
      store.filteredMessages.set(pendingFilteredMessages);
    };
    if (typeof queueMicrotask === 'function') return queueMicrotask(flush);
    Promise.resolve().then(flush).catch(function() { setTimeout(flush, 0); });
  }
  function projectThreadMessages(rows) {
    var list = Array.isArray(rows) ? rows : [];
    if (list.length <= threadProjectionLimit) return list;
    var center = Number(threadProjectionCenterIndex);
    if (!Number.isFinite(center) || center < 0) center = list.length - 1;
    center = Math.max(0, Math.min(list.length - 1, Math.round(center)));
    var before = Math.floor(threadProjectionLimit * 0.45);
    var start = Math.max(0, center - before);
    var end = Math.min(list.length, start + threadProjectionLimit);
    start = Math.max(0, end - threadProjectionLimit);
    return list.slice(start, end);
  }
  var store = {
    messages: writable([]),
    filteredMessages: writable([]),
    currentAgent: writable(null),
    agents: writable([]),
    sidebarAgents: writable([]),
    sessionLoading: writable(false),
    sending: writable(false),
    tokenCount: writable(0),
    inputText: writable(''),
    wsConnected: writable(false),
    showScrollDown: writable(false),
    stickToBottom: writable(true),
    mapStepIndex: writable(-1),
    mapRows: writable([]),
    renderWindowVersion: writable(0),
    focusMode: writable(false),
    connectionState: writable(''),
    theme: writable(''),
    sessions: writable([])
  };
  store.syncMessages = function(messages, filteredMessages) {
    store.mapRows.set(buildMapRows(messages));
    lastFilteredMessageSource = Array.isArray(filteredMessages) ? filteredMessages : [];
    pendingFilteredMessages = projectThreadMessages(lastFilteredMessageSource);
    pendingMessages = pendingFilteredMessages;
    scheduleMessageStoreFlush(store);
  };
  store.refreshMapRows = function(messages) { store.mapRows.set(buildMapRows(messages)); };
  store.setThreadProjectionCenter = function(index) {
    var next = Number(index);
    if (!Number.isFinite(next)) next = -1;
    next = Math.round(next);
    if (next === threadProjectionCenterIndex) return;
    threadProjectionCenterIndex = next;
    pendingFilteredMessages = projectThreadMessages(lastFilteredMessageSource);
    pendingMessages = pendingFilteredMessages;
    scheduleMessageStoreFlush(store);
  };
  store.bumpRenderWindowVersion = function() {
    store.renderWindowVersion.update(function(value) {
      var next = Number(value || 0) + 1;
      return Number.isFinite(next) ? next : 1;
    });
  };
  window.InfringChatStore = store;
  return store;
}

infringEnsureChatStoreBridge();

function infringEnsureShellAppStoreBridge() {
  if (typeof window === 'undefined') return null;
  var services = window.InfringSharedShellServices && typeof window.InfringSharedShellServices === 'object'
    ? window.InfringSharedShellServices
    : {};
  if (services.appStore) return services.appStore;
  var sourceStore = null;
  var shellRoot = null;
  var listeners = [];
  function current() {
    if (sourceStore && typeof sourceStore === 'object') return sourceStore;
    if (window.InfringApp && typeof window.InfringApp === 'object') {
      sourceStore = window.InfringApp;
      return sourceStore;
    }
    return null;
  }
  function root() {
    return shellRoot && typeof shellRoot === 'object' ? shellRoot : null;
  }
  function legacySource() {
    if (sourceStore && typeof sourceStore === 'object') return sourceStore;
    if (window.InfringApp && typeof window.InfringApp === 'object') return window.InfringApp;
    return null;
  }
  function snapshot() {
    var store = current() || {};
    var rootState = root() || {};
    var legacy = legacySource() || {};
    var agents = Array.isArray(store.agents) ? store.agents : [];
    if (!agents.length && Array.isArray(legacy.agents)) agents = legacy.agents;
    return {
      page: String(rootState.page || store.page || 'chat'),
      route: String(rootState.page || store.page || 'chat'),
      theme: String(rootState.theme || store.theme || ''),
      themeMode: String(rootState.themeMode || store.themeMode || rootState.theme || store.theme || ''),
      agents: agents,
      activeAgentId: String(store.activeAgentId || legacy.activeAgentId || ''),
      focusMode: !!store.focusMode,
      wsConnected: !!(store.wsConnected || legacy.wsConnected),
      connectionState: String(store.connectionState || legacy.connectionState || ''),
      notifications: Array.isArray(store.notifications) ? store.notifications : (Array.isArray(legacy.notifications) ? legacy.notifications : []),
      unreadNotifications: Number(store.unreadNotifications || legacy.unreadNotifications || 0) || 0,
      raw: store,
      root: rootState
    };
  }
  function notify(reason) {
    var state = snapshot();
    listeners.slice().forEach(function(listener) {
      try { listener(state); } catch (_) {}
    });
    try {
      window.dispatchEvent(new CustomEvent('infring:shell-app-store-changed', {
        detail: { reason: String(reason || 'changed'), state: state }
      }));
    } catch (_) {}
    return state;
  }
  function subscribe(listener) {
    if (typeof listener !== 'function') return function() {};
    listeners.push(listener);
    try { listener(snapshot()); } catch (_) {}
    return function() {
      listeners = listeners.filter(function(row) { return row !== listener; });
    };
  }
  function registerSource(store, reason) {
    if (store && typeof store === 'object') {
      sourceStore = store;
      window.InfringApp = store;
      notify(reason || 'source_registered');
    }
    return sourceStore;
  }
  function registerShellRoot(rootState) {
    if (rootState && typeof rootState === 'object') {
      shellRoot = rootState;
      notify('root_registered');
    }
    return shellRoot;
  }
  function registerAlpineStore(runtime, name, definition) {
    var registry = runtime && typeof runtime.store === 'function' ? runtime : null;
    if (!registry) return registerSource(definition, 'fallback_source_registered');
    registry.store(name || 'app', definition || {});
    return registerSource(registry.store(name || 'app'), 'alpine_compat_registered');
  }
  services.appStore = {
    current: current,
    root: root,
    snapshot: snapshot,
    subscribe: subscribe,
    notify: notify,
    registerSource: registerSource,
    registerShellRoot: registerShellRoot,
    registerAlpineStore: registerAlpineStore,
    set: function(key, value) {
      var store = current() || root();
      var legacy = legacySource();
      if (store && key) {
        store[key] = value;
        if (legacy && legacy !== store) legacy[key] = value;
        notify('set:' + key);
      }
      return store;
    },
    assign: function(values) {
      var store = current() || root();
      var legacy = legacySource();
      if (store && values && typeof values === 'object') {
        Object.assign(store, values);
        if (legacy && legacy !== store) Object.assign(legacy, values);
        notify('assign');
      }
      return store;
    },
    method: function(name) {
      var store = current();
      var fn = store && store[name];
      return typeof fn === 'function' ? fn.bind(store) : null;
    }
  };
  window.InfringSharedShellServices = services;
  return services.appStore;
}

function infringShellAppStoreBridge() {
  var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
  return services && services.appStore ? services.appStore : infringEnsureShellAppStoreBridge();
}

function infringShellAppStoreCurrent() {
  var bridge = infringShellAppStoreBridge();
  if (bridge && typeof bridge.current === 'function') return bridge.current();
  return (typeof window !== 'undefined' && window.InfringApp && typeof window.InfringApp === 'object')
    ? window.InfringApp
    : null;
}

// Temporary Alpine compatibility registration for the canonical Shell app-store bridge.
document.addEventListener('alpine:init', function() {
  // Restore saved API key on load
  var savedKey = localStorage.getItem('infring-api-key');
  if (savedKey) InfringAPI.setAuthToken(savedKey);

  var appStoreDefinition = {
    agents: [],
    connected: false,
    booting: true,
    agentsLoading: true,
    agentsHydrated: false,
    wsConnected: false,
    connectionState: 'connecting',
    statusFailureStreak: 0,
    lastError: '',
    bootStage: 'starting',
    statusDegraded: false,
    lastStatusLatencyMs: 0,
    lastStatusAt: '',
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
    serverVersion: '',
    gitBranch: '',
    assistantName: 'Assistant',
    assistantAvatar: null,
    assistantAgentId: null,
    agentCount: 0,
    localMediaPreviewRoots: [],
    embedSandboxMode: 'scripts',
    allowExternalEmbedUrls: false,
    pendingAgent: null,
    pendingFreshAgentId: null,
    activeAgentId: (() => {
      try {
        var saved = localStorage.getItem('infring-last-active-agent-id');
        return saved ? String(saved) : null;
      } catch(_) {
        return null;
      }
    })(),
    focusMode: localStorage.getItem('infring-focus') === 'true',
    showOnboarding: false,
    showAuthPrompt: false,
    authMode: 'apikey',
    sessionUser: null,
    notifications: [],
    notificationsOpen: false,
    unreadNotifications: 0,
    notificationBubble: null,
    notificationBellPulse: false,
    _notificationBellPulseTimer: null,
    _notificationBellPulseSeq: 0,
    _notificationBubbleTimer: null,
    _notificationSeq: 0,
    taskbarRefreshTurns: 0,
    taskbarSearchOpen: false,
    taskbarSearchQuery: '',
    _taskbarSearchFocusTimer: 0,
    agentChatPreviews: {},
    agentLiveActivity: {},
    agentsEmptyResponseStreak: 0,
    agentsLastNonEmptyAt: 0,
    agentsFetchAttempts: 0,
    agentsLastError: '',
    agentTransientHoldMs: 20000,
    _refreshAgentsInFlight: null,
    _lastAgentsRefreshAt: 0,
    runtimeSync: null,
    lastErrorCode: '',
    _sessionActivityByAgent: {},
    _sessionActivityBootstrapped: false,
    _lastSessionActivityPollAt: 0,

    toggleFocusMode() {
      this.focusMode = !this.focusMode;
      localStorage.setItem('infring-focus', this.focusMode);
    },

    bumpTaskbarRefreshTurn() {
      var current = Number(this.taskbarRefreshTurns || 0);
      if (!Number.isFinite(current) || current < 0) current = 0;
      this.taskbarRefreshTurns = (current + 1) % 4096;
    },

    setActiveAgentId(agentId) {
      this.activeAgentId = agentId ? String(agentId) : null;
      if (this.activeAgentId && this.agentChatPreviews && this.agentChatPreviews[this.activeAgentId]) {
        this.agentChatPreviews[this.activeAgentId].unread_response = false;
      }
      try {
        if (this.activeAgentId) localStorage.setItem('infring-last-active-agent-id', this.activeAgentId);
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch(_) {}
    },

    isArchivedLikeAgent(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var truthy = function(value) {
        if (value === true || value === 1) return true;
        var text = String(value || '').trim().toLowerCase();
        return text === 'true' || text === '1' || text === 'yes';
      };
      if (truthy(agent.archived) || truthy(agent.sidebar_archived)) return true;
      if (truthy(agent.contract_terminated) || truthy(agent.revive_recommended)) return true;
      if (truthy(agent.is_terminated) || truthy(agent.terminated) || truthy(agent.is_archived) || truthy(agent.inactive)) return true;
      var hardInactivePattern = /\b(archived|inactive|terminated|termed|contract[_\s-]*terminated|expired|revoked|timed[_\s-]*out|timeout|stopped|killed|dead)\b/;
      var lifecycleText = [
        agent.status,
        agent.state,
        agent.lifecycle_state,
        agent.agent_state,
        agent.runtime_state
      ]
        .map(function(value) { return String(value || '').trim().toLowerCase(); })
        .filter(Boolean)
        .join(' ');
      var hasLiveActiveSignal = /\b(active|running|ready|connected)\b/.test(lifecycleText);
      var hasLiveInactiveSignal = hardInactivePattern.test(lifecycleText);
      if (hasLiveInactiveSignal && !hasLiveActiveSignal) return true;
      var reasonText = [
        agent.termination_reason,
        agent.archive_reason,
        agent.inactive_reason
      ]
        .map(function(value) { return String(value || '').trim().toLowerCase(); })
        .filter(Boolean)
        .join(' ');
      if (hardInactivePattern.test(reasonText)) return true;
      var contract = agent.contract && typeof agent.contract === 'object' ? agent.contract : null;
      var contractStatus = String(contract && (contract.status || contract.state) ? (contract.status || contract.state) : '').trim().toLowerCase();
      if (hardInactivePattern.test(contractStatus)) return true;
      var contractRemaining = Number(
        (contract && (contract.remaining_ms != null ? contract.remaining_ms : contract.contract_remaining_ms)) != null
          ? (contract.remaining_ms != null ? contract.remaining_ms : contract.contract_remaining_ms)
          : (agent.contract_remaining_ms != null ? agent.contract_remaining_ms : NaN)
      );
      var contractFiniteExpiry = (contract && contract.finite_expiry != null)
        ? truthy(contract.finite_expiry)
        : truthy(agent.contract_finite_expiry);
      if (contractFiniteExpiry && Number.isFinite(contractRemaining) && contractRemaining <= 0) return true;
      return false;
    },

    markAgentPreviewUnread(agentId, unread) {
      var id = String(agentId || '').trim();
      if (!id) return;
      if (!this.agentChatPreviews) this.agentChatPreviews = {};
      if (!this.agentChatPreviews[id]) this.agentChatPreviews[id] = { text: '', ts: Date.now(), role: 'agent' };
      this.agentChatPreviews[id].unread_response = unread !== false;
    },

    async refreshAgents(opts) {
      // Alpine can invoke store methods through different call paths; guard against lost `this`.
      var store = (this && typeof this === 'object' && Object.prototype.hasOwnProperty.call(this, 'agentsHydrated'))
        ? this
        : infringShellAppStoreCurrent();
      if (!store) return;
      var options = opts || {};
      var force = options.force === true;
      var now = Date.now();
      if (!force && store._lastAgentsRefreshAt && (now - store._lastAgentsRefreshAt) < 1200) {
        return;
      }
      if (store._refreshAgentsInFlight) {
        return store._refreshAgentsInFlight;
      }
      store._refreshAgentsInFlight = (async () => {
        if (!store.agentsHydrated) store.agentsLoading = true;
        store.agentsFetchAttempts = Number(store.agentsFetchAttempts || 0) + 1;
        var agents = null;
        var fetchError = '';
        try {
          agents = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime&compact=1');
        } catch(e) {
          fetchError = (e && e.message) ? String(e.message) : 'agent_fetch_failed';
          try {
            await new Promise(function(resolve) { setTimeout(resolve, 250); });
            agents = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime&compact=1');
          } catch(_) {
            agents = null;
          }
        }
        if (Array.isArray(agents)) {
          var priorAgents = Array.isArray(store.agents) ? store.agents.slice() : [];
          var hadPriorAgents = priorAgents.length > 0;
          var holdMs = Number(store.agentTransientHoldMs || 0);
          var statusAgentCountHint = Number(store.agentCount || 0);
          if (!Number.isFinite(statusAgentCountHint) || statusAgentCountHint < 0) {
            statusAgentCountHint = 0;
          }
          var connectionState = String(store.connectionState || '').toLowerCase();
          if (agents.length === 0 && hadPriorAgents && store.connectionState !== 'disconnected') {
            // Strict runtime authority can momentarily return an empty roster when
            // collab dashboard polling times out. Preserve known-good rows while
            // status still reports active agents so the sidebar/chat selection
            // does not flap to zero.
            if (statusAgentCountHint > 0 || connectionState === 'connecting' || connectionState === 'reconnecting') {
              store.agentsHydrated = true;
              store.agentsLoading = false;
              store.agentsLastError = fetchError || 'strict_roster_transient_empty';
              store.agentCount = Math.max(priorAgents.length, statusAgentCountHint);
              return;
            }
            store.agentsEmptyResponseStreak = Number(store.agentsEmptyResponseStreak || 0) + 1;
            var lastNonEmptyAt = Number(store.agentsLastNonEmptyAt || 0);
            var withinHoldWindow = lastNonEmptyAt > 0 && (Date.now() - lastNonEmptyAt) < holdMs;
            // Buffer transient empty responses so chat selection doesn't flap/reset.
            if (withinHoldWindow || store.agentsEmptyResponseStreak < 3) {
              store.agentsHydrated = true;
              store.agentsLoading = false;
              store.agentCount = priorAgents.length;
              return;
            }
          } else if (agents.length > 0) {
            store.agentsEmptyResponseStreak = 0;
            store.agentsLastNonEmptyAt = Date.now();
          } else {
            store.agentsEmptyResponseStreak = 0;
          }

          // First-load protection: do not finalize empty roster until repeated confirms.
          if (agents.length === 0 && !store.agentsHydrated) {
            var attempts = Number(store.agentsFetchAttempts || 0);
            if (statusAgentCountHint > 0) {
              store.agentsLoading = true;
              store.agentCount = statusAgentCountHint;
              store.agentsLastError = fetchError || 'strict_roster_waiting_for_directory';
              return;
            }
            if (connectionState !== 'connected' || attempts < 3) {
              store.agentsLoading = true;
              store.agentCount = 0;
              return;
            }
          }

          var isSidebarArchivedRow = function(row) {
            if (!row || typeof row !== 'object') return false;
            return typeof store.isArchivedLikeAgent === 'function' ? store.isArchivedLikeAgent(row) : false;
          };
          var nextAgents = (Array.isArray(agents) ? agents : []).filter(function(row) {
            if (!row || !row.id) return false;
            return !isSidebarArchivedRow(row);
          });
          store.agents = nextAgents;
          store.agentsHydrated = true;
          store.agentsLoading = false;
          store.agentsLastError = '';
          var keep = {};
          for (var ai = 0; ai < nextAgents.length; ai++) {
            var row = nextAgents[ai];
            if (row && row.id) keep[String(row.id)] = true;
          }
          var nextActivity = {};
          var now = Date.now();
          var srcActivity = store.agentLiveActivity || {};
          keep.system = true;
          Object.keys(srcActivity).forEach(function(id) {
            var entry = srcActivity[id];
            if (!keep[id] || !entry) return;
            var state = String(entry.state || '').toLowerCase();
            var ts = Number(entry.ts || 0);
            var busyState = state.indexOf('typing') >= 0 || state.indexOf('working') >= 0 || state.indexOf('processing') >= 0;
            var ttlMs = busyState ? 180000 : 20000;
            if (!Number.isFinite(ts) || (now - ts) > ttlMs) return;
            nextActivity[id] = entry;
          });
          store.agentLiveActivity = nextActivity;
          if (store.activeAgentId) {
            var activeId = String(store.activeAgentId || '');
            var pendingFreshId = String(store.pendingFreshAgentId || '');
            var stillActive = activeId === 'system' || nextAgents.some(function(agent) {
              return agent && agent.id === store.activeAgentId;
            });
            if (!stillActive && pendingFreshId && activeId && pendingFreshId === activeId) {
              stillActive = true;
            }
            if (!stillActive) {
              store.setActiveAgentId(null);
            }
          }
          store.agentCount = nextAgents.length;
        } else if (!store.agentsHydrated) {
          store.agentsLoading = true;
          store.agentsLastError = fetchError || 'agent_fetch_failed';
        }
        store._lastAgentsRefreshAt = Date.now();
      })();
      try {
        await store._refreshAgentsInFlight;
      } finally {
        store._refreshAgentsInFlight = null;
      }
    },

    async checkStatus() {
      if (this.booting || this.connectionState === 'disconnected') {
        this.connectionState = 'connecting';
      }
      try {
        var startedAt = Date.now();
        var results = await Promise.all([
          InfringAPI.get('/api/status'),
          InfringAPI.get('/api/version').catch(function() { return null; })
        ]);
        var latencyMs = Math.max(0, Date.now() - startedAt);
        var s = results[0];
        var versionPayload = results[1];
        var statusObj = (s && typeof s === 'object') ? s : {};
        var versionObj = (versionPayload && typeof versionPayload === 'object') ? versionPayload : {};
        var stateRaw = String(
          statusObj.connection_state ||
          statusObj.state ||
          (statusObj.connected === false ? 'disconnected' : 'connected')
        ).toLowerCase();
        var connectedState = stateRaw === 'connected';
        var degraded = !!statusObj.degraded || !!statusObj.warning || statusObj.ok === false;
        var bootStage = String(statusObj.boot_stage || statusObj.last_stage || (connectedState ? 'ready' : 'connecting')).trim();
        if (!connectedState) {
          throw new Error(String(statusObj.error || 'status_unavailable'));
        }
        this.connected = true;
        this.booting = false;
        this.statusFailureStreak = 0;
        this.connectionState = 'connected';
        this.statusDegraded = degraded;
        this.bootStage = bootStage || 'ready';
        this.lastStatusLatencyMs = latencyMs;
        this.lastStatusAt = new Date().toISOString();
        this.lastError = degraded ? String(statusObj.error || statusObj.warning || '') : '';
        this.lastErrorCode = normalizeDashboardOptionalString(statusObj.error_code || statusObj.warning_code || '');
        var liveVersion = String(versionObj.version || versionObj.tag || '').trim().replace(/^[vV]/, '');
        this.version = liveVersion || statusObj.version || this.version || window.__INFRING_APP_VERSION || '0.0.0';
        this.gitBranch = statusObj.git_branch ? String(statusObj.git_branch) : (this.gitBranch || '');
        this.agentCount = statusObj.agent_count || 0;
        this.runtimeSync = (statusObj.runtime_sync && typeof statusObj.runtime_sync === 'object') ? statusObj.runtime_sync : null;
        if (typeof this.applyBootstrapRuntimeState === 'function') {
          this.applyBootstrapRuntimeState(statusObj, versionObj);
        }
        if (typeof this.pollSessionActivity === 'function') {
          Promise.resolve(this.pollSessionActivity(false)).catch(function() {});
        }
      } catch(e) {
        var streak = Number(this.statusFailureStreak || 0) + 1;
        this.connected = false;
        this.booting = false;
        this.statusFailureStreak = streak;
        this.statusDegraded = false;
        this.connectionState = streak >= 3 ? 'disconnected' : 'reconnecting';
        this.bootStage = streak >= 3 ? 'status_unreachable' : 'status_retrying';
        this.lastStatusLatencyMs = 0;
        this.lastStatusAt = new Date().toISOString();
        this.lastError = e.message || 'Unknown error';
        this.lastErrorCode = normalizeDashboardOptionalString((e && (e.code || e.name)) || '');
        this.runtimeSync = null;
        console.warn('[Infring] Status check failed:', e.message);
      }
    },

    async pollSessionActivity(force) {
      var now = Date.now();
      if (!force && this._lastSessionActivityPollAt && (now - Number(this._lastSessionActivityPollAt || 0)) < 8000) {
        return;
      }
      this._lastSessionActivityPollAt = now;
      try {
        var payload = await InfringAPI.get('/api/sessions');
        var rows = Array.isArray(payload && payload.sessions)
          ? payload.sessions
          : (Array.isArray(payload && payload.rows) ? payload.rows : []);
        var priorMap = this._sessionActivityByAgent && typeof this._sessionActivityByAgent === 'object'
          ? this._sessionActivityByAgent
          : {};
        var nextMap = {};
        var activeId = String(this.activeAgentId || '').trim();
        var noticesEmitted = 0;
        for (var i = 0; i < rows.length; i++) {
          var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : null;
          if (!row) continue;
          var agentId = String(row.agent_id || '').trim();
          if (!agentId) continue;
          var messageCount = Number(row.message_count || 0);
          if (!Number.isFinite(messageCount) || messageCount < 0) messageCount = 0;
          var updatedAt = String(row.updated_at || '').trim();
          nextMap[agentId] = {
            message_count: messageCount,
            updated_at: updatedAt
          };
          if (!this._sessionActivityBootstrapped) continue;
          if (noticesEmitted >= 8) continue;
          var prior = priorMap[agentId];
          if (!prior || typeof prior !== 'object') continue;
          var priorCount = Number(prior.message_count || 0);
          if (!Number.isFinite(priorCount) || priorCount < 0) priorCount = 0;
          var priorUpdated = String(prior.updated_at || '').trim();
          var countIncreased = messageCount > priorCount;
          var updatedChanged = !!updatedAt && updatedAt !== priorUpdated;
          if (!countIncreased && !updatedChanged) continue;
          if (agentId === activeId) continue;
