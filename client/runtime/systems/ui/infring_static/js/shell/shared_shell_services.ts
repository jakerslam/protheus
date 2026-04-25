'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};
  var agentMessageRoles = { agent: true, assistant: true };

  function trimString(value) {
    return String(value == null ? '' : value).trim();
  }

  function normalizeRole(value) {
    var role = trimString(value).toLowerCase();
    return role === 'assistant' ? 'agent' : role;
  }

  function messageRole(row) {
    if (!row || typeof row !== 'object') return '';
    if (row.terminal) {
      var terminalSource = normalizeRole(row.terminal_source);
      if (terminalSource === 'user' || terminalSource === 'human') return 'human';
      if (terminalSource === 'agent') return 'agent';
      return 'system';
    }
    var role = normalizeRole(row.role);
    if (role === 'user') return 'human';
    return role;
  }

  function messageIsAgentAuthored(row) {
    var role = messageRole(row);
    return !!agentMessageRoles[role];
  }

  function messageHasVisibleContent(row) {
    if (!row || typeof row !== 'object') return false;
    if (trimString(row.text)) return true;
    if (Array.isArray(row.tools) && row.tools.length > 0) return true;
    if (row.meta) return true;
    if (row.ts) return true;
    return false;
  }

  function agentHasSession(agent) {
    return !!(agent && typeof agent === 'object' && trimString(agent.id));
  }

  function canRequestEvalIssueReport(row, agent) {
    if (!agentHasSession(agent)) return false;
    if (!messageIsAgentAuthored(row)) return false;
    if (!row || row.thinking || row.terminal || row.is_notice) return false;
    return messageHasVisibleContent(row);
  }

  function numericOr(value, fallback) {
    var numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : fallback;
  }

  function normalizeRect(rect, fallbackWidth, fallbackHeight) {
    var source = rect && typeof rect === 'object' ? rect : {};
    var left = numericOr(source.left, 0);
    var top = numericOr(source.top, 0);
    var width = numericOr(source.width, fallbackWidth || 0);
    var height = numericOr(source.height, fallbackHeight || 0);
    var right = numericOr(source.right, left + width);
    var bottom = numericOr(source.bottom, top + height);
    return {
      left: left,
      right: right,
      top: top,
      bottom: bottom,
      width: Math.max(0, right - left || width),
      height: Math.max(0, bottom - top || height)
    };
  }

  function resolveOverlayPlacement(originRect, viewportRect, options) {
    var viewport = normalizeRect(viewportRect, 0, 0);
    var origin = normalizeRect(originRect, 0, 0);
    var opts = options && typeof options === 'object' ? options : {};
    var horizontalCenter = origin.left + origin.width / 2;
    var verticalCenter = origin.top + origin.height / 2;
    var viewportHorizontalCenter = viewport.left + viewport.width / 2;
    var viewportVerticalCenter = viewport.top + viewport.height / 2;
    var horizontal = horizontalCenter <= viewportHorizontalCenter ? 'right' : 'left';
    var vertical = verticalCenter <= viewportVerticalCenter ? 'below' : 'above';
    if (opts.preferHorizontal === 'left' || opts.preferHorizontal === 'right') horizontal = opts.preferHorizontal;
    if (opts.preferVertical === 'above' || opts.preferVertical === 'below') vertical = opts.preferVertical;
    return {
      horizontal: horizontal,
      vertical: vertical,
      anchorX: horizontal === 'right' ? origin.left : origin.right,
      anchorY: vertical === 'below' ? origin.bottom : origin.top,
      className: 'shell-overlay-placement shell-overlay-x-' + horizontal + ' shell-overlay-y-' + vertical
    };
  }

  function glassSurfaceClass(kind) {
    var value = trimString(kind).toLowerCase().replace(/_/g, '-');
    if (value === 'fogged' || value === 'fogged-glass') return 'fogged-glass';
    if (value === 'warped' || value === 'warped-glass' || value === 'magnified-glass') return 'warped-glass';
    if (value === 'simple' || value === 'simple-glass') return 'simple-glass';
    return 'simple-glass';
  }

  services.message = Object.assign({}, services.message || {}, {
    role: messageRole,
    isAgentAuthored: messageIsAgentAuthored,
    hasVisibleContent: messageHasVisibleContent,
    canRequestEvalIssueReport: canRequestEvalIssueReport
  });
  services.overlay = Object.assign({}, services.overlay || {}, {
    resolvePlacement: resolveOverlayPlacement
  });
  services.glass = Object.assign({}, services.glass || {}, {
    surfaceClass: glassSurfaceClass
  });

  return services;
})(typeof InfringSharedShellServices === 'object' ? InfringSharedShellServices : null);

if (typeof window !== 'undefined') {
  window.InfringSharedShellServices = InfringSharedShellServices;
}
