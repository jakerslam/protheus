function infringNormalizeDashboardHealthSummary(page, payload) {
  var summary = payload && typeof payload === 'object' ? payload : {};
  var agents = Array.isArray(summary.agents) ? summary.agents : [];
  return {
    ok: summary.ok === true,
    ts: Number(summary.ts || Date.now()),
    durationMs: Number(summary.durationMs != null ? summary.durationMs : summary.duration_ms || 0),
    heartbeatSeconds: Number(summary.heartbeatSeconds != null ? summary.heartbeatSeconds : summary.heartbeat_seconds || 0),
    defaultAgentId: String(summary.defaultAgentId || summary.default_agent_id || ''),
    agent_count: Number(summary.agent_count || agents.length || 0),
    agents: agents
  };
}

async function infringLoadDashboardHealthSummary(page, force) {
  var now = Date.now();
  if (!force && page._healthSummaryLoading) return page._healthSummaryLoading;
  if (!force && page._healthSummaryLoadedAt && (now - Number(page._healthSummaryLoadedAt || 0)) < 15000) {
    return page.healthSummary;
  }
  var seq = Number(page._healthSummaryLoadSeq || 0) + 1;
  page._healthSummaryLoadSeq = seq;
  page._healthSummaryLoading = (async function() {
    try {
      var payload = await InfringAPI.get('/api/health');
      if (seq !== Number(page._healthSummaryLoadSeq || 0)) return page.healthSummary;
      page.healthSummary = page.normalizeDashboardHealthSummary(payload);
      page.healthSummaryError = '';
    } catch (e) {
      if (seq !== Number(page._healthSummaryLoadSeq || 0)) return page.healthSummary;
      page.healthSummary = page.normalizeDashboardHealthSummary(null);
      page.healthSummaryError = String(e && e.message ? e.message : 'health_unavailable');
    } finally {
      if (seq === Number(page._healthSummaryLoadSeq || 0)) {
        page._healthSummaryLoadedAt = Date.now();
        page._healthSummaryLoading = null;
      }
    }
    return page.healthSummary;
  })();
  return page._healthSummaryLoading;
}

async function infringPollStatus(page, opts) {
  var force = !!(opts && opts.force);
  if (page._pollStatusInFlight) {
    page._pollStatusQueued = true;
    return page._pollStatusInFlight;
  }
  page._pollStatusInFlight = (async function() {
    var store = page.getAppStore();
    if (!store) {
      page.connected = false;
      page.connectionState = 'connecting';
      if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('status_retrying');
      return;
    }
    if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('status_requesting');
    if (typeof store.checkStatus === 'function') await store.checkStatus();
    if (typeof page.setBootProgressEvent === 'function') {
      page.setBootProgressEvent(
        store && store.connectionState === 'connected' ? 'status_connected' : 'status_retrying',
        { bootStage: store && store.bootStage }
      );
    }
    var shouldHydrateHealth = force || store.connectionState !== 'connected' || !store.runtimeSync;
    if (shouldHydrateHealth) {
      Promise.resolve(page.loadDashboardHealthSummary(store.connectionState !== 'connected')).catch(function() {});
    }
    var now = Date.now();
    var shouldRefreshAgents =
      force ||
      !store.agentsHydrated ||
      (store.connectionState !== 'connected') ||
      (now - Number(store._lastAgentsRefreshAt || 0)) >= 12000;
    if (shouldRefreshAgents) {
      if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('agents_refresh_started');
      if (typeof store.refreshAgents === 'function') await store.refreshAgents();
    }
    if (store.agentsHydrated && !store.agentsLoading) {
      if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('agents_hydrated');
    }
    if (typeof page.syncChatSidebarTopologyOrderFromAgents === 'function') {
      page.syncChatSidebarTopologyOrderFromAgents();
    }
    page.connected = store.connected;
    page.version = store.version;
    page.agentCount = store.agentCount;
    page.connectionState = store.connectionState || (store.connected ? 'connected' : 'disconnected');
    page.queueConnectionIndicatorState(page.connectionState);
    page.wsConnected = InfringAPI.isWsConnected();
    if (!page.bootSelectionApplied && store.agentsHydrated && !store.agentsLoading) {
      await page.applyBootChatSelection();
      if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('selection_applied');
    }
    page.scheduleSidebarScrollIndicators();
    if (store.booting === false && store.agentsHydrated && !store.agentsLoading) {
      if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('releasing', { bootStage: store.bootStage });
    }
    page.releaseBootSplash(false);
  })();
  try {
    await page._pollStatusInFlight;
  } finally {
    page._pollStatusInFlight = null;
    if (page._pollStatusQueued) {
      page._pollStatusQueued = false;
      window.setTimeout(function() { page.pollStatus({ force: true }); }, 0);
    }
  }
}
