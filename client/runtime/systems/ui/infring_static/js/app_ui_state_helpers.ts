// Canonical Shell helper source: small dashboard UI state helpers.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringIsArchivedLikeAgent(agent) {
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
  var lifecycleText = [agent.status, agent.state, agent.lifecycle_state, agent.agent_state, agent.runtime_state]
    .map(function(value) { return String(value || '').trim().toLowerCase(); })
    .filter(Boolean)
    .join(' ');
  var hasLiveActiveSignal = /\b(active|running|ready|connected)\b/.test(lifecycleText);
  var hasLiveInactiveSignal = hardInactivePattern.test(lifecycleText);
  if (hasLiveInactiveSignal && !hasLiveActiveSignal) return true;
  var reasonText = [agent.termination_reason, agent.archive_reason, agent.inactive_reason]
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
  var contractFiniteExpiry = (contract && contract.finite_expiry != null) ? truthy(contract.finite_expiry) : truthy(agent.contract_finite_expiry);
  return !!(contractFiniteExpiry && Number.isFinite(contractRemaining) && contractRemaining <= 0);
}

function infringFocusTaskbarSearchInput(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target._taskbarSearchFocusTimer) {
    clearTimeout(target._taskbarSearchFocusTimer);
    target._taskbarSearchFocusTimer = 0;
  }
  target._taskbarSearchFocusTimer = window.setTimeout(function() {
    var input = document.getElementById('taskbar-search-input');
    if (input && typeof input.focus === 'function') {
      input.focus({ preventScroll: true });
      if (typeof input.select === 'function') input.select();
    }
    target._taskbarSearchFocusTimer = 0;
  }, 40);
}

function infringOpenTaskbarSearch(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.taskbarSearchOpen = false;
}

function infringCloseTaskbarSearch(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.taskbarSearchOpen = false;
  if (target._taskbarSearchFocusTimer) {
    clearTimeout(target._taskbarSearchFocusTimer);
    target._taskbarSearchFocusTimer = 0;
  }
}

function infringToggleTaskbarSearch(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.taskbarSearchOpen = false;
}

function infringFormatNotificationTime(ts) {
  if (!ts) return '';
  var d = new Date(ts);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
}
