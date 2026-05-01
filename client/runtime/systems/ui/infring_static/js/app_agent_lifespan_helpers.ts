function infringAgentAutoTerminateEnabled(page, agent) {
  if (!agent || typeof agent !== 'object') return false;
  if (typeof agent.auto_terminate_allowed === 'boolean') {
    return agent.auto_terminate_allowed;
  }
  // Server contract should provide explicit policy; default fail-closed.
  return false;
}

function infringAgentContractRemainingMs(page, agent) {
  // Force recompute every second for live countdown updates.
  var _tick = Number(page.clockTick || 0);
  void _tick;
  if (!page.agentAutoTerminateEnabled(agent)) return null;
  var store = page.getAppStore();
  var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
  var ageDriftMs =
    Number.isFinite(lastRefreshAt) && lastRefreshAt > 0
      ? Math.max(0, Date.now() - lastRefreshAt)
      : 0;
  if (!agent || typeof agent !== 'object') return null;
  var directRemaining = Number(agent.contract_remaining_ms);
  if (Number.isFinite(directRemaining) && directRemaining >= 0) {
    return Math.max(0, Math.floor(directRemaining - ageDriftMs));
  }
  return null;
}

function infringAgentContractHasFiniteExpiry(page, agent) {
  if (!agent || typeof agent !== 'object') return false;
  if (agent.revive_recommended === true) return true;
  if (typeof agent.contract_finite_expiry === 'boolean') {
    return agent.contract_finite_expiry;
  }
  var directRemaining = Number(agent.contract_remaining_ms);
  if (Number.isFinite(directRemaining) && directRemaining >= 0) return true;
  var totalMs = Number(agent.contract_total_ms);
  return Number.isFinite(totalMs) && totalMs > 0;
}

function infringAgentContractTerminationGraceMs(page) {
  return 10000;
}

function infringIsAgentPendingTermination(page, agent) {
  if (!page.agentAutoTerminateEnabled(agent)) return false;
  if (!page.agentContractHasFiniteExpiry(agent)) return false;
  var remainingMs = page.agentContractRemainingMs(agent);
  if (remainingMs == null || remainingMs > 0) return false;
  var store = page.getAppStore();
  var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
  if (!Number.isFinite(lastRefreshAt) || lastRefreshAt <= 0) return true;
  var refreshAgeMs = Math.max(0, Date.now() - lastRefreshAt);
  return refreshAgeMs < page.agentContractTerminationGraceMs();
}

function infringShouldShowInfinityLifespan(page, agent) {
  if (!agent || typeof agent !== 'object') return false;
  if (agent.revive_recommended === true) return false;
  if (typeof agent.contract_finite_expiry === 'boolean') {
    if (agent.contract_finite_expiry) return false;
    return !page.agentAutoTerminateEnabled(agent);
  }
  if (!page.agentAutoTerminateEnabled(agent)) return true;
  // Unknown contract timing should not be rendered as explicit infinity.
  return false;
}

function infringShouldShowExpiryCountdown(page, agent) {
  if (agent && agent.revive_recommended === true) return true;
  if (!page.agentAutoTerminateEnabled(agent)) return false;
  if (!page.agentContractHasFiniteExpiry(agent)) return false;
  var remainingMs = page.agentContractRemainingMs(agent);
  if (remainingMs == null) return false;
  if (remainingMs <= 0) return page.isAgentPendingTermination(agent);
  return true;
}

function infringExpiryCountdownLabel(page, agent) {
  if (agent && agent.revive_recommended === true) return 'timed out';
  var remainingMs = page.agentContractRemainingMs(agent);
  if (remainingMs == null) return '';

  if (remainingMs <= 0) return page.isAgentPendingTermination(agent) ? '0m' : '';
  var totalMin = Math.max(1, Math.ceil(remainingMs / 60000));
  var monthMin = 30 * 24 * 60;
  if (totalMin >= monthMin) {
    return Math.max(1, Math.ceil(totalMin / monthMin)) + 'm';
  }
  if (totalMin >= 1440) {
    return Math.max(1, Math.ceil(totalMin / 1440)) + 'd';
  }
  if (totalMin >= 60) {
    return Math.max(1, Math.ceil(totalMin / 60)) + 'h';
  }
  return totalMin + 'm';
}

function infringExpiryCountdownCritical(page, agent) {
  if (agent && agent.revive_recommended === true) return false;
  if (page.isAgentPendingTermination(agent)) return true;
  var remainingMs = page.agentContractRemainingMs(agent);
  if (remainingMs == null) return false;
  var totalMs = page.agentContractTotalMs(agent);
  if (!Number.isFinite(totalMs) || totalMs <= 0) return false;
  var thresholdMs = Math.min(3600000, Math.max(1, Math.floor(totalMs * 0.2)));
  return remainingMs > 0 && remainingMs <= thresholdMs;
}

function infringAgentContractTotalMs(page, agent) {
  if (!agent || typeof agent !== 'object') return null;
  var durationMs = Number(agent.contract_total_ms);
  if (Number.isFinite(durationMs) && durationMs > 0) return Math.floor(durationMs);
  return null;
}

function infringAgentHeartStates(page, agent) {
  var totalHearts = 5;
  var hearts = [true, true, true, true, true];
  if (!agent || typeof agent !== 'object') return hearts;
  if (agent.is_system_thread) return hearts;
  if (agent.revive_recommended === true) return [false, false, false, false, false];
  if (!page.agentAutoTerminateEnabled(agent) || !page.agentContractHasFiniteExpiry(agent)) return [true];
  var remainingMs = page.agentContractRemainingMs(agent);
  if (remainingMs == null) return [true];
  if (remainingMs <= 0 && page.isAgentPendingTermination(agent)) return [false, false, false, false, false];
  var totalMs = page.agentContractTotalMs(agent);
  if (!Number.isFinite(totalMs) || totalMs <= 0) return [true];
  var ratio = Math.max(0, Math.min(1, remainingMs / totalMs));
  var filled = Math.ceil(ratio * totalHearts);
  if (remainingMs <= 0 && page.isAgentPendingTermination(agent)) filled = 0;
  if (filled < 0) filled = 0;
  if (filled > totalHearts) filled = totalHearts;
  for (var i = 0; i < totalHearts; i++) {
    hearts[i] = i < filled;
  }
  return hearts;
}

function infringAgentHeartShowsInfinity(page, agent) {
  if (!agent || typeof agent !== 'object') return false;
  if (agent.is_system_thread) return false;
  if (agent.revive_recommended === true) return false;
  return !page.agentAutoTerminateEnabled(agent) || !page.agentContractHasFiniteExpiry(agent);
}

function infringAgentHeartMeterLabel(page, agent) {
  if (!agent || typeof agent !== 'object' || agent.is_system_thread) return '';
  if (agent.revive_recommended === true) return 'Time limit: timed out';
  if (!page.agentAutoTerminateEnabled(agent) || !page.agentContractHasFiniteExpiry(agent)) {
    return 'Time limit: unlimited';
  }
  var label = page.expiryCountdownLabel(agent);
  if (label) return 'Time remaining: ' + label;
  return 'Time limit active';
}
