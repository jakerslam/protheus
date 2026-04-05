// Infring App — Alpine.js init, hash router, global store
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

function renderMarkdown(text) {
  if (!text) return '';
  if (typeof marked !== 'undefined') {
    // Protect LaTeX blocks from marked.js mangling (underscores, backslashes, etc.)
    var latexBlocks = [];
    var protected_ = text;
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
    // Wrap fenced code blocks in a dedicated chat code card with per-block copy.
    html = html.replace(/<pre><code([^>]*)>([\s\S]*?)<\/code><\/pre>/g, function(_, attrs, body) {
      var codeAttrs = attrs || '';
      return (
        '<div class="chat-codeblock">' +
          '<button class="message-stat-btn chat-codeblock-copy" type="button" onclick="copyCode(this)" title="Copy code" aria-label="Copy code">' +
            '<svg class="copy-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>' +
            '<svg class="copied-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" style="display:none"><path d="M20 6L9 17l-5-5"></path></svg>' +
          '</button>' +
          '<pre><code' + codeAttrs + '>' + body + '</code></pre>' +
        '</div>'
      );
    });
    // Open external links in new tab
    html = html.replace(/<a\s+href="(https?:\/\/[^"]*)"(?![^>]*target=)([^>]*)>/gi, '<a href="$1" target="_blank" rel="noopener"$2>');
    return html;
  }
  return escapeHtml(text);
}

// Alpine.js global store
document.addEventListener('alpine:init', function() {
  // Restore saved API key on load
  var savedKey = localStorage.getItem('infring-api-key');
  if (savedKey) InfringAPI.setAuthToken(savedKey);

  Alpine.store('app', {
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
    gitBranch: '',
    agentCount: 0,
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
    _notificationBubbleTimer: null,
    _notificationSeq: 0,
    topbarRefreshTurns: 0,
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
    _sessionActivityByAgent: {},
    _sessionActivityBootstrapped: false,
    _lastSessionActivityPollAt: 0,

    toggleFocusMode() {
      this.focusMode = !this.focusMode;
      localStorage.setItem('infring-focus', this.focusMode);
    },

    bumpTopbarRefreshTurn() {
      var current = Number(this.topbarRefreshTurns || 0);
      if (!Number.isFinite(current) || current < 0) current = 0;
      this.topbarRefreshTurns = (current + 1) % 4096;
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
        : Alpine.store('app');
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
          agents = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');
        } catch(e) {
          fetchError = (e && e.message) ? String(e.message) : 'agent_fetch_failed';
          try {
            await new Promise(function(resolve) { setTimeout(resolve, 250); });
            agents = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');
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

          var nextAgents = (Array.isArray(agents) ? agents : []).filter(function(row) {
            if (!row || !row.id) return false;
            if (typeof store.isArchivedLikeAgent === 'function') return !store.isArchivedLikeAgent(row);
            if (row.archived === true) return false;
            var state = String(row.state || '').trim().toLowerCase();
            var contract = row.contract && typeof row.contract === 'object' ? row.contract : null;
            var contractStatus = String(contract && contract.status ? contract.status : '').trim().toLowerCase();
            return !(
              state.indexOf('archived') >= 0 ||
              state.indexOf('inactive') >= 0 ||
              state.indexOf('terminated') >= 0 ||
              contractStatus.indexOf('archived') >= 0 ||
              contractStatus.indexOf('inactive') >= 0 ||
              contractStatus.indexOf('terminated') >= 0
            );
          });
          var nextById = {};
          for (var ni = 0; ni < nextAgents.length; ni++) {
            var nextRow = nextAgents[ni];
            if (!nextRow || !nextRow.id) continue;
            nextById[String(nextRow.id)] = true;
          }
          if (hadPriorAgents) {
            for (var pi = 0; pi < priorAgents.length; pi++) {
              var prior = priorAgents[pi];
              if (!prior || !prior.id) continue;
              var priorId = String(prior.id || '').trim();
              if (!priorId || nextById[priorId]) continue;
              if (priorId.toLowerCase() === 'system') continue;
              if (prior.archived === true) continue;
              var priorState = String(prior.state || '').toLowerCase();
              if (priorState.indexOf('archived') >= 0) continue;
              var priorContract = (prior.contract && typeof prior.contract === 'object') ? prior.contract : null;
              var priorAutoAllowed = !(prior.auto_terminate_allowed === false || (priorContract && priorContract.auto_terminate_allowed === false));
              var priorRemainingMs = Number(prior.contract_remaining_ms != null ? prior.contract_remaining_ms : (priorContract && priorContract.remaining_ms));
              var priorExpiresAt = String(
                prior.contract_expires_at ||
                (priorContract && priorContract.expires_at ? priorContract.expires_at : '') ||
                ''
              ).trim();
              var priorExpiryTs = priorExpiresAt ? Number(new Date(priorExpiresAt).getTime()) : NaN;
              var reachedTimeout = (Number.isFinite(priorRemainingMs) && priorRemainingMs <= 0)
                || (Number.isFinite(priorExpiryTs) && priorExpiryTs > 0 && priorExpiryTs <= (Date.now() + 1500));
              var timeoutHint = priorState.indexOf('terminated') >= 0
                || priorState.indexOf('timed out') >= 0
                || priorState.indexOf('timeout') >= 0
                || String((priorContract && priorContract.termination_reason) || '').toLowerCase().indexOf('timeout') >= 0;
              if (!(priorAutoAllowed && (reachedTimeout || timeoutHint))) continue;
              var ghost = Object.assign({}, prior, {
                state: 'Timed out',
                archived: false,
                _timed_out_local: true,
                _sidebar_timed_out_at: Date.now()
              });
              var ghostContract = (ghost.contract && typeof ghost.contract === 'object') ? ghost.contract : {};
              ghost.contract = Object.assign({}, ghostContract, {
                status: 'terminated',
                termination_reason: String(ghostContract.termination_reason || 'idle_timeout'),
                auto_terminate_allowed: true,
                idle_terminate_allowed: true,
                remaining_ms: 0
              });
              ghost.contract_remaining_ms = 0;
              if (!ghost.contract_expires_at && ghost.contract && ghost.contract.expires_at) {
                ghost.contract_expires_at = ghost.contract.expires_at;
              }
              nextAgents.push(ghost);
              nextById[priorId] = true;
            }
          }
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
        var s = await InfringAPI.get('/api/status');
        var latencyMs = Math.max(0, Date.now() - startedAt);
        var statusObj = (s && typeof s === 'object') ? s : {};
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
        this.version = statusObj.version || this.version || window.__INFRING_APP_VERSION || '0.0.0';
        this.gitBranch = statusObj.git_branch ? String(statusObj.git_branch) : (this.gitBranch || '');
        this.agentCount = statusObj.agent_count || 0;
        this.runtimeSync = (statusObj.runtime_sync && typeof statusObj.runtime_sync === 'object') ? statusObj.runtime_sync : null;
        await this.pollSessionActivity(false);
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
