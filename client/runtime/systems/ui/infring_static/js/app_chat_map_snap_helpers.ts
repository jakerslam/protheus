// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringReadChatMapWidth(page) {
  var target = page && typeof page === 'object' ? page : {};
  var lockedWall = target.chatMapWallLockNormalized();
  if (lockedWall) {
    var surface = null;
    if (typeof document !== 'undefined' && typeof document.querySelector === 'function') {
      try { surface = document.querySelector('.chat-map .chat-map-surface'); } catch(_) {}
    }
    var lockedWidth = Number(surface && surface.offsetWidth || 0);
    if (Number.isFinite(lockedWidth) && lockedWidth > 0) return lockedWidth;
    return 60;
  }
  var node = target.readChatMapElement();
  var width = Number(node && node.offsetWidth || 0);
  if (Number.isFinite(width) && width > 0) return width;
  return 76;
}

function infringChatMapSnapDefinitions() {
  return [
    { id: 'right-top', x: 1, y: 0 },
    { id: 'right-middle', x: 1, y: 0.5 },
    { id: 'right-bottom', x: 1, y: 1 }
  ];
}

function infringChatMapSnapDefinitionById(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(id || '').trim().toLowerCase();
  var defs = target.chatMapSnapDefinitions();
  for (var i = 0; i < defs.length; i += 1) {
    var row = defs[i];
    if (!row || row.id !== key) continue;
    return row;
  }
  return defs[1] || defs[0] || { id: 'right-middle', x: 1, y: 0.5 };
}

function infringChatMapAnchorForSnapId(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var snap = target.chatMapSnapDefinitionById(id);
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatMapWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatMapHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var nx = Number(snap && snap.x);
  var ny = Number(snap && snap.y);
  if (!Number.isFinite(nx)) nx = 1;
  if (!Number.isFinite(ny)) ny = 0.5;
  nx = Math.max(0, Math.min(1, nx));
  ny = Math.max(0, Math.min(1, ny));
  return {
    id: String(snap && snap.id || 'right-middle'),
    left: target.chatMapClampLeft(minLeft + ((maxLeft - minLeft) * nx)),
    top: target.chatMapClampTop(minTop + ((maxTop - minTop) * ny))
  };
}

function infringChatMapNearestSnapId(page, leftRaw, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var defs = target.chatMapSnapDefinitions();
  if (!defs.length) return 'right-middle';
  var left = target.chatMapClampLeft(leftRaw);
  var top = target.chatMapClampTop(topRaw);
  var bestId = String(defs[0].id || 'right-middle');
  var bestDist = Number.POSITIVE_INFINITY;
  for (var i = 0; i < defs.length; i += 1) {
    var row = defs[i];
    if (!row) continue;
    var anchor = target.chatMapAnchorForSnapId(row.id);
    var dx = Number(left || 0) - Number(anchor.left || 0);
    var dy = Number(top || 0) - Number(anchor.top || 0);
    var dist = (dx * dx) + (dy * dy);
    if (!Number.isFinite(dist) || dist >= bestDist) continue;
    bestDist = dist;
    bestId = String(row.id || bestId);
  }
  return bestId || 'right-middle';
}

function infringChatMapResolvedLeftFromRatio(page) {
  var target = page && typeof page === 'object' ? page : {};
  var ratio = 1;
  try {
    var raw = Number(localStorage.getItem('infring-chat-map-placement-x'));
    if (Number.isFinite(raw)) ratio = Math.max(0, Math.min(1, raw));
  } catch(_) {}
  if (Number.isFinite(target.chatMapPlacementX)) ratio = Math.max(0, Math.min(1, Number(target.chatMapPlacementX)));
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatMapWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  return target.chatMapClampLeft(minLeft + ((maxLeft - minLeft) * ratio));
}

function infringChatMapResolvedTopFromRatio(page) {
  var target = page && typeof page === 'object' ? page : {};
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatMapHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var ratio = Number(target.chatMapPlacementY);
  if (!Number.isFinite(ratio)) ratio = 0.38;
  ratio = Math.max(0, Math.min(1, ratio));
  return target.chatMapClampTop(minTop + ((maxTop - minTop) * ratio));
}

function infringChatMapActiveSnapId(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.chatMapDragActive) {
    return target.chatMapNearestSnapId(target.chatMapDragLeft, target.chatMapDragTop);
  }
  var storedId = String(target.chatMapPlacementAnchorId || '').trim().toLowerCase();
  if (!storedId) {
    try {
      var raw = String(localStorage.getItem('infring-chat-map-placement-anchor') || '').trim().toLowerCase();
      if (raw) storedId = raw;
    } catch(_) {}
  }
  if (storedId) return target.chatMapSnapDefinitionById(storedId).id;
  var fallbackLeft = target.chatMapClampLeft(target.chatMapResolvedLeftFromRatio());
  var fallbackTop = target.chatMapClampTop(target.chatMapResolvedTopFromRatio());
  return target.chatMapNearestSnapId(fallbackLeft, fallbackTop);
}

function infringChatMapPersistSnapId(page, id) {
  var target = page && typeof page === 'object' ? page : {};
  var snap = target.chatMapSnapDefinitionById(id);
  target.chatMapPlacementAnchorId = String(snap && snap.id || 'right-middle');
  try {
    localStorage.setItem('infring-chat-map-placement-anchor', target.chatMapPlacementAnchorId);
  } catch(_) {}
}

function infringChatMapClampLeft(page, leftRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatMapWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  var left = Number(leftRaw);
  if (!Number.isFinite(left)) left = maxLeft;
  return Math.max(minLeft, Math.min(maxLeft, left));
}

function infringChatMapHardBounds(page) {
  var target = page && typeof page === 'object' ? page : {};
  return target.dragSurfaceHardBounds(target.readChatMapWidth(), target.readChatMapHeight());
}

function infringChatMapWallLockNormalized(page) {
  var target = page && typeof page === 'object' ? page : {};
  var wall = target.dragSurfaceNormalizeWall(target.chatMapWallLock);
  return wall === 'left' || wall === 'right' ? wall : '';
}

function infringChatMapSetWallLock(page, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  if (wall !== 'left' && wall !== 'right') wall = '';
  target.chatMapWallLock = wall;
  try {
    if (wall) localStorage.setItem('infring-chat-map-wall-lock', wall);
    else localStorage.removeItem('infring-chat-map-wall-lock');
    localStorage.removeItem('infring-chat-map-smash-wall');
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.chatMap.wallLock = wall;
  });
  return wall;
}

function infringChatMapResolvedLeft(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.chatMapDragActive) return Number(target.chatMapDragLeft || 0);
  var left = target.chatMapClampLeft(target.chatMapResolvedLeftFromRatio());
  var top = target.chatMapClampTop(target.chatMapResolvedTopFromRatio());
  var wall = target.chatMapWallLockNormalized();
  if (!wall) return left;
  return target.dragSurfaceApplyWallLock(target.chatMapHardBounds(), left, top, wall).left;
}

function infringChatMapResolvedTop(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.chatMapDragActive) return Number(target.chatMapDragTop || 0);
  var left = target.chatMapClampLeft(target.chatMapResolvedLeftFromRatio());
  var top = target.chatMapClampTop(target.chatMapResolvedTopFromRatio());
  var wall = target.chatMapWallLockNormalized();
  if (!wall) return top;
  return target.dragSurfaceApplyWallLock(target.chatMapHardBounds(), left, top, wall).top;
}

function infringChatMapPersistPlacementFromLeft(page, leftRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = target.chatOverlayViewportWidth() - wallGap - target.readChatMapWidth();
  if (!Number.isFinite(maxLeft) || maxLeft < minLeft) maxLeft = minLeft;
  var left = target.chatMapClampLeft(leftRaw);
  var ratio = maxLeft > minLeft ? (left - minLeft) / (maxLeft - minLeft) : 1;
  ratio = Math.max(0, Math.min(1, ratio));
  target.chatMapPlacementX = ratio;
  try {
    localStorage.setItem('infring-chat-map-placement-x', String(ratio));
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.chatMap.placementX = ratio;
  });
}

function infringChatMapContainerStyle(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target.chatMapPlacementEnabled()) return '';
  var top = target.chatMapResolvedTop();
  var left = target.chatMapResolvedLeft();
  var height = target.readChatMapHeight();
  var durationMs = target.chatMapDragActive ? 0 : target.dragSurfaceMoveDurationMs(target._chatMapMoveDurationMs, 280);
  var wall = target.chatMapWallLockNormalized();
  var lockCss = target.dragSurfaceLockVisualCssVars('chat-map', wall, {
    transformMs: target._dragSurfaceLockTransformMs,
    shellPaddingInline: '8px',
    shellPaddingInlineLocked: '0px',
    shellPaddingBlock: '2px',
    shellPaddingBlockLocked: '0px',
    shellAlignItems: 'flex-end',
    shellAlignItemsLeft: 'flex-start',
    shellAlignItemsRight: 'flex-end',
    surfaceMarginInline: 'auto',
    surfaceMarginInlineLocked: '0'
  });
  return (
    'left:' + Math.round(left) + 'px;' +
    'top:' + Math.round(top) + 'px;' +
    'right:auto;' +
    'bottom:auto;' +
    'height:' + Math.round(height) + 'px;' +
    lockCss +
    'transition:top ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth), left ' + Math.max(0, Math.round(durationMs)) + 'ms var(--ease-smooth);'
  );
}

function infringStartChatMapPointerDrag(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!ev || !target.chatMapPlacementEnabled()) return;
  var button = Number(ev.button);
  if (Number.isFinite(button) && button > 0) return;
  var eventTarget = ev && ev.target ? ev.target : null;
  if (target.shouldIgnoreChatMapDragTarget(eventTarget)) return;
  target._chatMapPointerActive = true;
  target._chatMapPointerMoved = false;
  target._chatMapPointerStartX = Number(ev.clientX || 0);
  target._chatMapPointerStartY = Number(ev.clientY || 0);
  target._chatMapPointerOriginLeft = target.chatMapResolvedLeft();
  target._chatMapPointerOriginTop = target.chatMapResolvedTop();
  target._chatMapPointerLastX = target._chatMapPointerStartX;
  target._chatMapPointerLastY = target._chatMapPointerStartY;
  target._chatMapPointerLastAt = Date.now();
  target._chatMapPointerVelocity = 0;
  target.chatMapDragLeft = target._chatMapPointerOriginLeft;
  target.chatMapDragTop = target._chatMapPointerOriginTop;
  target.bindChatMapPointerListeners();
  try {
    if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
      ev.currentTarget.setPointerCapture(ev.pointerId);
    }
  } catch(_) {}
}

function infringHandleChatMapPointerMove(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._chatMapPointerActive || !target.chatMapPlacementEnabled()) return;
  var nextX = Number(ev.clientX || 0);
  var nextY = Number(ev.clientY || 0);
  var now = Date.now();
  var prevX = Number(target._chatMapPointerLastX || nextX);
  var prevY = Number(target._chatMapPointerLastY || nextY);
  var prevAt = Number(target._chatMapPointerLastAt || now);
  var dt = Math.max(1, now - prevAt);
  var stepDx = nextX - prevX;
  var stepDy = nextY - prevY;
  target._chatMapPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
  target._chatMapPointerLastX = nextX;
  target._chatMapPointerLastY = nextY;
  target._chatMapPointerLastAt = now;
  var movedX = Math.abs(nextX - Number(target._chatMapPointerStartX || 0));
  var movedY = Math.abs(nextY - Number(target._chatMapPointerStartY || 0));
  if (!target._chatMapPointerMoved) {
    if (movedX < 4 && movedY < 4) return;
    target._chatMapPointerMoved = true;
    target.chatMapDragActive = true;
    target.hideDashboardPopupBySource('chat-map');
  }
  var dragDx = nextX - Number(target._chatMapPointerStartX || 0);
  var dragDy = nextY - Number(target._chatMapPointerStartY || 0);
  var candidateLeft = Number(target._chatMapPointerOriginLeft || 0) + dragDx;
  var candidateTop = Number(target._chatMapPointerOriginTop || 0) + dragDy;
  var hardBounds = target.chatMapHardBounds();
  var lockedWall = target.chatMapWallLockNormalized();
  if (lockedWall) {
    var unlockDistance = target.dragSurfaceDistanceFromWall(hardBounds, candidateLeft, candidateTop, lockedWall);
    if (unlockDistance >= target.dragSurfaceWallUnlockDistanceThreshold()) {
      lockedWall = target.chatMapSetWallLock('');
    } else {
      var holdLeft = Number.isFinite(Number(target.chatMapDragLeft))
        ? Number(target.chatMapDragLeft)
        : Number(target._chatMapPointerOriginLeft || 0);
      var holdTop = Number.isFinite(Number(target.chatMapDragTop))
        ? Number(target.chatMapDragTop)
        : Number(target._chatMapPointerOriginTop || 0);
      var stayLocked = target.dragSurfaceApplyWallLock(hardBounds, holdLeft, holdTop, lockedWall);
      target.chatMapDragLeft = stayLocked.left;
      target.chatMapDragTop = stayLocked.top;
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
      return;
    }
  }
  var clamped = target.dragSurfaceClampWithBounds(hardBounds, candidateLeft, candidateTop);
  var nearest = target.dragSurfaceNearestWall(hardBounds, clamped.left, clamped.top);
  var lockWall = target.dragSurfaceResolveWallLock(
    hardBounds,
    candidateLeft,
    candidateTop,
    nearest,
    dragDx,
    dragDy
  );
  if (lockWall) {
    var persistedLockWall = target.chatMapSetWallLock(lockWall);
    var snapped = target.dragSurfaceApplyWallLock(hardBounds, clamped.left, clamped.top, persistedLockWall);
    target.chatMapDragLeft = snapped.left;
    target.chatMapDragTop = snapped.top;
  } else {
    target.chatMapDragLeft = clamped.left;
    target.chatMapDragTop = clamped.top;
  }
  if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringEndChatMapPointerDrag(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._chatMapPointerActive) return;
  target._chatMapPointerActive = false;
  target.unbindChatMapPointerListeners();
  if (!target._chatMapPointerMoved) {
    target.chatMapDragActive = false;
    return;
  }
  target._chatMapPointerMoved = false;
  var hardBounds = target.chatMapHardBounds();
  var lockedWall = target.chatMapWallLockNormalized();
  var final;
  if (lockedWall) {
    final = target.dragSurfaceApplyWallLock(hardBounds, target.chatMapDragLeft, target.chatMapDragTop, lockedWall);
    target.chatMapPlacementAnchorId = '';
    try { localStorage.removeItem('infring-chat-map-placement-anchor'); } catch(_) {}
  } else {
    var clamped = target.dragSurfaceClampWithBounds(hardBounds, target.chatMapDragLeft, target.chatMapDragTop);
    var snapId = target.chatMapNearestSnapId(clamped.left, clamped.top);
    var snap = target.chatMapAnchorForSnapId(snapId);
    final = target.dragSurfaceClampWithBounds(hardBounds, snap.left, snap.top);
    target.chatMapPersistSnapId(snapId);
  }
  target.chatMapDragLeft = final.left;
  target.chatMapDragTop = final.top;
  target.chatMapPersistPlacementFromLeft(target.chatMapDragLeft);
  target.chatMapPersistPlacementFromTop(target.chatMapDragTop);
  target.chatMapDragActive = false;
}
