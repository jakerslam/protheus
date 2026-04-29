'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};

  function trimString(value) {
    return String(value == null ? '' : value).trim();
  }

  function numericOr(value, fallback) {
    var numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : fallback;
  }

  function rowRole(row) {
    if (services.message && typeof services.message.role === 'function') {
      return trimString(services.message.role(row)).toLowerCase();
    }
    var role = trimString(row && row.role).toLowerCase();
    if (role === 'user') return 'human';
    if (role === 'assistant') return 'agent';
    return role;
  }

  function isAgentAuthored(row) {
    if (services.message && typeof services.message.isAgentAuthored === 'function') {
      return services.message.isAgentAuthored(row);
    }
    return rowRole(row) === 'agent';
  }

  function isHumanAuthored(row) {
    var role = rowRole(row);
    return role === 'human' || role === 'user';
  }

  function hasVisibleContent(row) {
    if (services.message && typeof services.message.hasVisibleContent === 'function') {
      return services.message.hasVisibleContent(row);
    }
    return !!(
      row &&
      typeof row === 'object' &&
      (
        trimString(row.text) ||
        (Array.isArray(row.tools) && row.tools.length > 0) ||
        row.meta ||
        row.ts
      )
    );
  }

  function resolveIndex(row, index, rows) {
    var list = Array.isArray(rows) ? rows : [];
    if (!list.length) return -1;
    var rowId = trimString(row && row.id);
    var resolved = Number(index);
    if (!Number.isFinite(resolved) || resolved < 0 || resolved >= list.length) {
      resolved = -1;
    }
    if (resolved >= 0) {
      var probe = list[resolved];
      if (probe && rowId && trimString(probe.id) !== rowId) resolved = -1;
    }
    if (resolved >= 0) return resolved;
    for (var i = list.length - 1; i >= 0; i -= 1) {
      var candidate = list[i];
      if (!candidate) continue;
      if (candidate === row) return i;
      if (rowId && trimString(candidate.id) === rowId) return i;
    }
    return -1;
  }

  function isLatestAgent(row, index, rows) {
    var list = Array.isArray(rows) ? rows : [];
    var resolved = resolveIndex(row, index, list);
    if (resolved < 0) return false;
    for (var i = list.length - 1; i >= 0; i -= 1) {
      var candidate = list[i];
      if (!candidate || candidate.is_notice || !isAgentAuthored(candidate)) continue;
      return i === resolved;
    }
    return false;
  }

  function canRetry(row, index, rows) {
    if (!isAgentAuthored(row)) return false;
    void index;
    void rows;
    return false;
  }

  function canReply(row, index, rows) {
    void index;
    void rows;
    if (!row || row.is_notice || isHumanAuthored(row)) return false;
    return false;
  }

  function canFork(row, agent) {
    if (!agent || !trimString(agent.id)) return false;
    return isAgentAuthored(row);
  }

  function canReportIssue(row, agent) {
    if (services.message && typeof services.message.canRequestEvalIssueReport === 'function') {
      return services.message.canRequestEvalIssueReport(row, agent);
    }
    if (!agent || !trimString(agent.id)) return false;
    if (!isAgentAuthored(row)) return false;
    if (!row || row.thinking || row.terminal || row.is_notice) return false;
    return hasVisibleContent(row);
  }

  function shouldRender(row, override) {
    if (override === false) return false;
    if (!row || row.thinking) return false;
    return hasVisibleContent(row);
  }

  function metaVisible(row, collapsed) {
    if (!row || row.is_notice || row.thinking) return false;
    return !collapsed;
  }

  function responseTimeText(row, durationMs, formatter) {
    if (!row || row.thinking || row.is_notice || !isAgentAuthored(row)) return '';
    var duration = numericOr(durationMs, 0);
    if (duration <= 0) return '';
    if (typeof formatter === 'function') return trimString(formatter(duration));
    return Math.round(duration) + 'ms';
  }

  function burnLabelText(row, totalTokens, formatter) {
    if (!row || row.thinking || row.is_notice) return '';
    var total = numericOr(totalTokens, 0);
    if (total <= 0) return '';
    if (total < 1000) return String(Math.round(total));
    if (typeof formatter === 'function') return trimString(formatter(total));
    return (Math.round((total / 1000) * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'k';
  }

  function viewModel(input) {
    var source = input && typeof input === 'object' ? input : {};
    var row = source.row && typeof source.row === 'object' ? source.row : {};
    var rows = Array.isArray(source.rows) ? source.rows : [];
    var index = numericOr(source.index, -1);
    var collapsed = !!source.collapsed;
    var render = shouldRender(row, source.shouldRender);
    var visible = metaVisible(row, collapsed);
    return {
      shouldRender: render,
      visible: visible,
      copied: !!source.copied,
      hasTools: !!source.hasTools,
      toolsCollapsed: !!source.toolsCollapsed,
      canReportIssue: canReportIssue(row, source.agent),
      canRetry: canRetry(row, index, rows),
      canReply: canReply(row, index, rows),
      canFork: canFork(row, source.agent),
      timestamp: trimString(source.timestamp),
      responseTime: responseTimeText(row, source.responseTimeMs, source.responseTimeFormatter),
      burnLabel: burnLabelText(row, source.burnTotalTokens, source.burnFormatter),
      burnIconSrc: trimString(source.burnIconSrc) || '/icons/vecteezy_fire-icon-simple-vector-perfect-illustration_13821331.svg'
    };
  }

  services.messageMeta = Object.assign({}, services.messageMeta || {}, {
    resolveIndex: resolveIndex,
    isLatestAgent: isLatestAgent,
    canRetry: canRetry,
    canReply: canReply,
    canFork: canFork,
    canReportIssue: canReportIssue,
    shouldRender: shouldRender,
    visible: metaVisible,
    responseTimeText: responseTimeText,
    burnLabelText: burnLabelText,
    viewModel: viewModel
  });

  return services;
})(typeof InfringSharedShellServices === 'object' ? InfringSharedShellServices : null);

if (typeof window !== 'undefined') {
  window.InfringSharedShellServices = InfringSharedShellServices;
}
