// Canonical Shell helper source: dashboard runtime status projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

async function infringCheckStatus(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.booting || target.connectionState === 'disconnected') {
    target.connectionState = 'connecting';
  }
  try {
    var startedAt = Date.now();
    var results = await Promise.all([
      InfringAPI.get('/api/status'),
      InfringAPI.get('/api/version').catch(function() { return null; })
    ]);
    var latencyMs = Math.max(0, Date.now() - startedAt);
    var statusPayload = results[0];
    var versionPayload = results[1];
    var statusObj = (statusPayload && typeof statusPayload === 'object') ? statusPayload : {};
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
    target.connected = true;
    target.booting = false;
    target.statusFailureStreak = 0;
    target.connectionState = 'connected';
    target.statusDegraded = degraded;
    target.bootStage = bootStage || 'ready';
    target.lastStatusLatencyMs = latencyMs;
    target.lastStatusAt = new Date().toISOString();
    target.lastError = degraded ? String(statusObj.error || statusObj.warning || '') : '';
    target.lastErrorCode = normalizeDashboardOptionalString(statusObj.error_code || statusObj.warning_code || '');
    var liveVersion = String(versionObj.version || versionObj.tag || '').trim().replace(/^[vV]/, '');
    target.version = liveVersion || statusObj.version || target.version || window.__INFRING_APP_VERSION || '0.0.0';
    target.gitBranch = statusObj.git_branch ? String(statusObj.git_branch) : (target.gitBranch || '');
    target.agentCount = statusObj.agent_count || 0;
    target.runtimeSync = (statusObj.runtime_sync && typeof statusObj.runtime_sync === 'object') ? statusObj.runtime_sync : null;
    if (typeof target.applyBootstrapRuntimeState === 'function') {
      target.applyBootstrapRuntimeState(statusObj, versionObj);
    }
    if (typeof target.pollSessionActivity === 'function') {
      Promise.resolve(target.pollSessionActivity(false)).catch(function() {});
    }
  } catch(e) {
    var streak = Number(target.statusFailureStreak || 0) + 1;
    target.connected = false;
    target.booting = false;
    target.statusFailureStreak = streak;
    target.statusDegraded = false;
    target.connectionState = streak >= 3 ? 'disconnected' : 'reconnecting';
    target.bootStage = streak >= 3 ? 'status_unreachable' : 'status_retrying';
    target.lastStatusLatencyMs = 0;
    target.lastStatusAt = new Date().toISOString();
    target.lastError = e.message || 'Unknown error';
    target.lastErrorCode = normalizeDashboardOptionalString((e && (e.code || e.name)) || '');
    target.runtimeSync = null;
    console.warn('[Infring] Status check failed:', e.message);
  }
}
