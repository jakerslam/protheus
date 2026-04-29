// Canonical Shell helper source: dashboard agent roster refresh projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

async function infringRefreshAgents(page, opts) {
  // Alpine can invoke store methods through different call paths; guard against lost `this`.
  var store = (page && typeof page === 'object' && Object.prototype.hasOwnProperty.call(page, 'agentsHydrated'))
    ? page
    : infringShellAppStoreCurrent();
  if (!store) return;
  var options = opts || {};
  var force = options.force === true;
  var currentTimeMs = Date.now();
  if (!force && store._lastAgentsRefreshAt && (currentTimeMs - store._lastAgentsRefreshAt) < 1200) {
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
      for (var agentIndex = 0; agentIndex < nextAgents.length; agentIndex++) {
        var row = nextAgents[agentIndex];
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
}
