function infringChatSidebarSnapDefinitions() {
  return [
    { id: 'left-top', x: 0, y: 0 },
    { id: 'left-middle', x: 0, y: 0.5 },
    { id: 'left-bottom', x: 0, y: 1 }
  ];
}

function infringChatSidebarSnapDefinitionById(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(id || '').trim().toLowerCase();
  var defs = target.chatSidebarSnapDefinitions();
  for (var i = 0; i < defs.length; i += 1) {
    var row = defs[i];
    if (!row || row.id !== key) continue;
    return row;
  }
  return defs[1] || defs[0] || { id: 'left-middle', x: 0, y: 0.5 };
}

function infringChatSidebarAnchorForSnapId(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var snap = target.chatSidebarSnapDefinitionById(id);
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatSidebarWidth() - target.readChatSidebarPulltabWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatSidebarHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var nx = Number(snap && snap.x);
  var ny = Number(snap && snap.y);
  if (!Number.isFinite(nx)) nx = 0;
  if (!Number.isFinite(ny)) ny = 0.5;
  nx = Math.max(0, Math.min(1, nx));
  ny = Math.max(0, Math.min(1, ny));
  return {
    id: String(snap && snap.id || 'left-middle'),
    left: target.chatSidebarClampLeft(minLeft + ((maxLeft - minLeft) * nx)),
    top: target.chatSidebarClampTop(minTop + ((maxTop - minTop) * ny))
  };
}

function infringChatSidebarNearestSnapId(page, leftRaw, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var defs = target.chatSidebarSnapDefinitions();
  if (!defs.length) return 'left-middle';
  var left = target.chatSidebarClampLeft(leftRaw);
  var top = target.chatSidebarClampTop(topRaw);
  var bestId = String(defs[0].id || 'left-middle');
  var bestDist = Number.POSITIVE_INFINITY;
  for (var i = 0; i < defs.length; i += 1) {
    var row = defs[i];
    if (!row) continue;
    var anchor = target.chatSidebarAnchorForSnapId(row.id);
    var dx = Number(left || 0) - Number(anchor.left || 0);
    var dy = Number(top || 0) - Number(anchor.top || 0);
    var dist = (dx * dx) + (dy * dy);
    if (!Number.isFinite(dist) || dist >= bestDist) continue;
    bestDist = dist;
    bestId = String(row.id || bestId);
  }
  return bestId || 'left-middle';
}

function infringChatSidebarResolvedLeftFromRatio(page) {
  var target = page && typeof page === 'object' ? page : {};
  var ratio = 0;
  try {
    var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-x'));
    if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
  } catch(_) {}
  if (Number.isFinite(target.chatSidebarPlacementX)) ratio = Math.max(0, Math.min(1, Number(target.chatSidebarPlacementX)));
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatSidebarWidth() - target.readChatSidebarPulltabWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  return target.chatSidebarClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
}

function infringChatSidebarResolvedTopFromRatio(page) {
  var target = page && typeof page === 'object' ? page : {};
  var topPx = Number(target.chatSidebarPlacementTopPx);
  if (!Number.isFinite(topPx)) {
    try {
      var rawTop = Number(localStorage.getItem('infring-chat-sidebar-placement-top-px'));
      if (Number.isFinite(rawTop)) topPx = rawTop;
    } catch(_) {}
  }
  if (Number.isFinite(topPx)) return target.chatSidebarClampTop(topPx);
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatSidebarHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var ratio = Number(target.chatSidebarPlacementY);
  if (!Number.isFinite(ratio)) ratio = 0.5;
  ratio = Math.max(0, Math.min(1, ratio));
  return target.chatSidebarClampTop(minTop + ((maxTop - minTop) * ratio));
}

function infringChatSidebarActiveSnapId(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.chatSidebarDragActive) {
    return target.chatSidebarNearestSnapId(target.chatSidebarDragLeft, target.chatSidebarDragTop);
  }
  var storedId = String(target.chatSidebarPlacementAnchorId || '').trim().toLowerCase();
  if (!storedId) {
    try {
      var raw = String(localStorage.getItem('infring-chat-sidebar-placement-anchor') || '').trim().toLowerCase();
      if (raw) storedId = raw;
    } catch(_) {}
  }
  if (storedId) return target.chatSidebarSnapDefinitionById(storedId).id;
  var fallbackLeft = target.chatSidebarClampLeft(target.chatSidebarResolvedLeftFromRatio());
  var fallbackTop = target.chatSidebarClampTop(target.chatSidebarResolvedTopFromRatio());
  return target.chatSidebarNearestSnapId(fallbackLeft, fallbackTop);
}

function infringChatSidebarPersistSnapId(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var snap = target.chatSidebarSnapDefinitionById(id);
  target.chatSidebarPlacementAnchorId = String(snap && snap.id || 'left-middle');
  try {
    localStorage.setItem('infring-chat-sidebar-placement-anchor', target.chatSidebarPlacementAnchorId);
  } catch(_) {}
}

function infringReadChatSidebarElement() {
  if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
  try { return document.querySelector('.sidebar'); } catch(_) {}
  return null;
}

function infringReadChatSidebarHeight(page) {
  var target = page && typeof page === 'object' ? page : {};
  var node = target.readChatSidebarElement();
  var height = Number(node && node.offsetHeight || 0);
  if (!Number.isFinite(height) || height <= 0) {
    height = Math.max(180, Math.round(target.taskbarReadViewportHeight() * 0.52));
  }
  return height;
}

function infringReadChatSidebarWidth(page) {
  var target = page && typeof page === 'object' ? page : {};
  var node = target.readChatSidebarElement();
  var width = Number(node && node.offsetWidth || 0);
  if (Number.isFinite(width) && width > 0) return width;
  var fallback = target.sidebarCollapsed ? 72 : 248;
  if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
    try {
      var key = target.sidebarCollapsed ? '--sidebar-collapsed' : '--sidebar-width';
      var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue(key) || '').trim();
      var parsed = parseFloat(raw);
      if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
    } catch(_) {}
  }
  return Math.max(1, Math.round(fallback));
}

function infringReadChatSidebarPulltabWidth() {
  var fallback = 22;
  if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
    try {
      var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue('--sidebar-pulltab-width') || '').trim();
      var parsed = parseFloat(raw);
      if (Number.isFinite(parsed) && parsed > 0) fallback = parsed;
    } catch(_) {}
  }
  return Math.max(1, Math.round(fallback));
}

function infringChatSidebarClampLeft(page, leftRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatSidebarWidth() - target.readChatSidebarPulltabWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  var left = Number(leftRaw);
  if (!Number.isFinite(left)) left = minLeft;
  return Math.max(minLeft, Math.min(maxLeft, left));
}

function infringChatSidebarHardBounds(page) {
  var target = page && typeof page === 'object' ? page : {};
  return target.dragSurfaceHardBounds(target.readChatSidebarWidth(), target.readChatSidebarHeight());
}

function infringChatSidebarWallLockNormalized(page) {
  var target = page && typeof page === 'object' ? page : {};
  var wall = target.dragSurfaceNormalizeWall(target.chatSidebarWallLock);
  return wall === 'left' || wall === 'right' ? wall : '';
}

function infringChatSidebarSetWallLock(page, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  if (wall !== 'left' && wall !== 'right') wall = '';
  target.chatSidebarWallLock = wall;
  try {
    if (wall) localStorage.setItem('infring-chat-sidebar-wall-lock', wall);
    else localStorage.removeItem('infring-chat-sidebar-wall-lock');
    localStorage.removeItem('infring-chat-sidebar-smash-wall');
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.chatBar.wallLock = wall;
  });
  return wall;
}

function infringChatSidebarResolvedLeft(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.chatSidebarDragActive) return Number(target.chatSidebarDragLeft || 0);
  var left = target.chatSidebarClampLeft(target.chatSidebarResolvedLeftFromRatio());
  var top = target.chatSidebarClampTop(target.chatSidebarResolvedTopFromRatio());
  var wall = target.chatSidebarWallLockNormalized();
  if (!wall) return left;
  return target.dragSurfaceApplyWallLock(target.chatSidebarHardBounds(), left, top, wall).left;
}

function infringChatSidebarPersistPlacementFromLeft(page, leftRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatSidebarWidth() - target.readChatSidebarPulltabWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  var left = target.chatSidebarClampLeft(leftRaw);
  var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 0;
  ratio = Math.max(0, Math.min(1, ratio));
  target.chatSidebarPlacementX = ratio;
  try {
    localStorage.setItem('infring-chat-sidebar-placement-x', String(ratio));
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.chatBar.placementX = ratio;
  });
}

function infringChatSidebarClampTop(page, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatSidebarHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var top = Number(topRaw);
  if (!Number.isFinite(top)) top = minTop + ((maxTop - minTop) * 0.5);
  return Math.max(minTop, Math.min(maxTop, top));
}

function infringChatSidebarResolvedTop(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.chatSidebarDragActive) return Number(target.chatSidebarDragTop || 0);
  var left = target.chatSidebarClampLeft(target.chatSidebarResolvedLeftFromRatio());
  var top = target.chatSidebarClampTop(target.chatSidebarResolvedTopFromRatio());
  var wall = target.chatSidebarWallLockNormalized();
  if (!wall) return top;
  return target.dragSurfaceApplyWallLock(target.chatSidebarHardBounds(), left, top, wall).top;
}

function infringChatSidebarPersistPlacementFromTop(page, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatSidebarHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var top = target.chatSidebarClampTop(topRaw);
  target.chatSidebarPlacementTopPx = top;
  try {
    localStorage.setItem('infring-chat-sidebar-placement-top-px', String(top));
  } catch(_) {}
  var ratio = maxTop > minTop ? (top - minTop) / (maxTop - minTop) : 0.5;
  ratio = Math.max(0, Math.min(1, ratio));
  target.chatSidebarPlacementY = ratio;
  try {
    localStorage.setItem('infring-chat-sidebar-placement-y', String(ratio));
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.chatBar.placementTopPx = top;
    config.chatBar.placementY = ratio;
  });
}

function infringChatSidebarContainerStyle(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.page !== 'chat') return '';
  var top = target.chatSidebarResolvedTop();
  var left = target.chatSidebarResolvedLeft();
  var durationMs = target.chatSidebarDragActive ? 0 : target.dragSurfaceMoveDurationMs(target._chatSidebarMoveDurationMs, 280);
  var wall = target.chatSidebarWallLockNormalized();
  var lockCss = target.dragSurfaceLockVisualCssVars('chat-sidebar', wall, {
    transformMs: target._dragSurfaceLockTransformMs
  });
  return (
    'position:fixed;' +
    'left:' + Math.round(left) + 'px;' +
    'top:' + Math.round(top) + 'px;' +
    'bottom:auto;' +
    'height:fit-content;' +
    'min-height:calc(56px * 3);' +
    'max-height:80vh;' +
    'transform:none;' +
    '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;' +
    lockCss
  );
}

function infringChatSidebarNavShellStyle(page) {
  var target = page && typeof page === 'object' ? page : {};
  return target.page === 'chat'
    ? 'flex:0 1 auto;min-height:0;max-height:calc(80vh - 16px);'
    : '';
}

function infringChatSidebarNavStyle(page) {
  var target = page && typeof page === 'object' ? page : {};
  return target.page === 'chat'
    ? 'height:auto;flex:0 1 auto;max-height:calc(80vh - 16px);'
    : '';
}

function infringChatSidebarPulltabStyle(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.page !== 'chat') return '';
  var durationMs = target.chatSidebarDragActive ? 0 : target.dragSurfaceMoveDurationMs(target._chatSidebarMoveDurationMs, 280);
  var wall = target.chatSidebarWallLockNormalized();
  var service = target.dragbarService();
  if (service && typeof service.pulltabStyle === 'function') {
    return service.pulltabStyle({
      active: target.page === 'chat',
      dragging: target.chatSidebarDragActive,
      durationMs: durationMs,
      fallbackMs: 280,
      transitionVar: '--sidebar-position-transition',
      wall: wall
    });
  }
  var dockRight = wall === 'right';
  return [
    'position:absolute;',
    'left:' + (dockRight ? 'auto' : '100%') + ';',
    'right:' + (dockRight ? '100%' : 'auto') + ';',
    'top:50%;',
    'transform:translateY(-50%);',
    '--sidebar-position-transition:' + Math.max(0, Math.round(durationMs)) + 'ms;'
  ].join('');
}

function infringShouldIgnoreChatSidebarDragTarget(page, dragTarget) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.shouldIgnoreTarget === 'function') {
    return service.shouldIgnoreTarget(dragTarget, {
      ignoreSelector: 'input,textarea,select,[contenteditable="true"],button,a,[role="button"],.sidebar-pulltab,.nav-item,.nav-agent-row,[data-agent-id]'
    });
  }
  var node = dragTarget;
  if (node && typeof node.closest !== 'function' && node.parentElement) {
    node = node.parentElement;
  }
  if (!node || typeof node.closest !== 'function') return false;
  if (node.closest('.sidebar-pulltab')) return true;
  return Boolean(
    node.closest(
      'input,textarea,select,[contenteditable="true"],button,a,[role="button"],.nav-item,.nav-agent-row,[data-agent-id]'
    )
  );
}
