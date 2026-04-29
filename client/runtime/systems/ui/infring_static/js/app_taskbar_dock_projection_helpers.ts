function infringTaskbarDockEdgeNormalized(page, raw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.normalizeTaskbarEdge === 'function') return service.normalizeTaskbarEdge(raw);
  var key = String(raw || '').trim().toLowerCase();
  return key === 'bottom' ? 'bottom' : 'top';
}

function infringTaskbarPersistDockEdge(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.taskbarDockEdge = target.taskbarDockEdgeNormalized(target.taskbarDockEdge);
  try {
    localStorage.setItem('infring-taskbar-dock-edge', target.taskbarDockEdge);
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.taskbar.edge = target.taskbarDockEdge;
  });
}

function infringTaskbarReadHeight() {
  if (typeof document === 'undefined') return 46;
  try {
    var node = document.querySelector('.global-taskbar');
    var height = Number(node && node.offsetHeight || 0);
    if (Number.isFinite(height) && height > 0) return height;
  } catch(_) {}
  return 46;
}

function infringTaskbarReadViewportHeight() {
  var h = 0;
  try { h = Number(window && window.innerHeight || 0); } catch(_) { h = 0; }
  if (!Number.isFinite(h) || h <= 0) {
    h = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
  }
  if (!Number.isFinite(h) || h <= 0) h = 900;
  return h;
}

function infringChatOverlayViewportWidth() {
  var w = 0;
  try { w = Number(window && window.innerWidth || 0); } catch(_) { w = 0; }
  if (!Number.isFinite(w) || w <= 0) {
    w = Number(document && document.documentElement && document.documentElement.clientWidth || 1440);
  }
  if (!Number.isFinite(w) || w <= 0) w = 1440;
  return w;
}

function infringTaskbarAnchorForDockEdge(page, edgeRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.normalizeTaskbarEdge === 'function') {
    var edgeFromService = service.normalizeTaskbarEdge(edgeRaw);
    if (edgeFromService === 'bottom') return Math.max(0, target.taskbarReadViewportHeight() - target.taskbarReadHeight());
    return 0;
  }
  var edge = target.taskbarDockEdgeNormalized(edgeRaw);
  if (edge === 'bottom') {
    return Math.max(0, target.taskbarReadViewportHeight() - target.taskbarReadHeight());
  }
  return 0;
}

function infringTaskbarClampDragY(page, yRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.normalizeTaskbarEdge === 'function') {
    var yFromService = Number(yRaw);
    if (!Number.isFinite(yFromService)) yFromService = target.taskbarAnchorForDockEdge(target.taskbarDockEdge);
    var maxFromService = Math.max(0, target.taskbarReadViewportHeight() - target.taskbarReadHeight());
    return Math.max(0, Math.min(maxFromService, yFromService));
  }
  var y = Number(yRaw);
  if (!Number.isFinite(y)) y = target.taskbarAnchorForDockEdge(target.taskbarDockEdge);
  var maxY = Math.max(0, target.taskbarReadViewportHeight() - target.taskbarReadHeight());
  return Math.max(0, Math.min(maxY, y));
}

function infringTaskbarNearestDockEdge(page, yRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.normalizeTaskbarEdge === 'function') {
    var yFromService = target.taskbarClampDragY(yRaw);
    var topYFromService = target.taskbarAnchorForDockEdge('top');
    var bottomYFromService = target.taskbarAnchorForDockEdge('bottom');
    return Math.abs(yFromService - bottomYFromService) < Math.abs(yFromService - topYFromService) ? 'bottom' : 'top';
  }
  var y = target.taskbarClampDragY(yRaw);
  var topY = target.taskbarAnchorForDockEdge('top');
  var bottomY = target.taskbarAnchorForDockEdge('bottom');
  var topDist = Math.abs(y - topY);
  var bottomDist = Math.abs(y - bottomY);
  return bottomDist < topDist ? 'bottom' : 'top';
}

function infringTaskbarContainerStyle(page) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.taskbarContainerStyle === 'function') {
    return service.taskbarContainerStyle({
      page: target.page,
      edge: target.taskbarDockEdge,
      dragging: target.taskbarDockDragActive,
      dragY: target.taskbarClampDragY(target.taskbarDockDragY),
      transitionMs: 220
    });
  }
  var styles = [];
  if (target.page !== 'chat') {
    styles.push('background:transparent;border-bottom:none;box-shadow:none;-webkit-backdrop-filter:none;backdrop-filter:none;');
  }
  var transitionMs = target.taskbarDockDragActive ? 0 : 220;
  styles.push('--taskbar-dock-transition:' + Math.max(0, Math.round(Number(transitionMs || 0))) + 'ms;');
  if (target.taskbarDockDragActive) {
    var y = target.taskbarClampDragY(target.taskbarDockDragY);
    styles.push('top:' + Math.round(Number(y || 0)) + 'px;bottom:auto;');
  } else if (target.taskbarDockEdgeNormalized(target.taskbarDockEdge) === 'bottom') {
    styles.push('top:auto;bottom:0;');
  } else {
    styles.push('top:0;bottom:auto;');
  }
  return styles.join('');
}

function infringShouldIgnoreTaskbarDockDragTarget(page, targetNode) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.dragbarService === 'function' ? target.dragbarService() : null;
  var ignoreSelector = 'button, a, input, textarea, select, [role="button"], [draggable="true"], .taskbar-reorder-item, .taskbar-hero-menu-anchor, .taskbar-hero-menu, .theme-switcher, .notif-wrap, .taskbar-search-popup, .taskbar-search-popup-anchor, .taskbar-clock';
  if (service && typeof service.shouldIgnoreTarget === 'function') {
    return service.shouldIgnoreTarget(targetNode, { ignoreSelector: ignoreSelector });
  }
  if (!targetNode || typeof targetNode.closest !== 'function') return false;
  return Boolean(targetNode.closest(ignoreSelector));
}
