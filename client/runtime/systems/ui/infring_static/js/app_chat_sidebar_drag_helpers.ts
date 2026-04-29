// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringBindChatSidebarPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target._chatSidebarPointerMoveHandler || target._chatSidebarPointerUpHandler) return;
  target._chatSidebarPointerMoveHandler = function(ev) { target.handleChatSidebarPointerMove(ev); };
  target._chatSidebarPointerUpHandler = function() { target.endChatSidebarPointerDrag(); };
  var supportsPointer = typeof window !== 'undefined' && ('PointerEvent' in window);
  if (supportsPointer) {
    window.addEventListener('pointermove', target._chatSidebarPointerMoveHandler, true);
    window.addEventListener('pointerup', target._chatSidebarPointerUpHandler, true);
    window.addEventListener('pointercancel', target._chatSidebarPointerUpHandler, true);
  } else {
    window.addEventListener('mousemove', target._chatSidebarPointerMoveHandler, true);
    window.addEventListener('mouseup', target._chatSidebarPointerUpHandler, true);
  }
}

function infringUnbindChatSidebarPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target._chatSidebarPointerMoveHandler) {
    try { window.removeEventListener('pointermove', target._chatSidebarPointerMoveHandler, true); } catch(_) {}
    try { window.removeEventListener('mousemove', target._chatSidebarPointerMoveHandler, true); } catch(_) {}
  }
  if (target._chatSidebarPointerUpHandler) {
    try { window.removeEventListener('pointerup', target._chatSidebarPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('pointercancel', target._chatSidebarPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('mouseup', target._chatSidebarPointerUpHandler, true); } catch(_) {}
  }
  target._chatSidebarPointerMoveHandler = null;
  target._chatSidebarPointerUpHandler = null;
}

function infringStartChatSidebarPointerDrag(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!ev || target.page !== 'chat') return;
  if (target._chatSidebarPointerActive) return;
  var button = Number(ev.button);
  if (Number.isFinite(button) && button !== 0) return;
  var eventTarget = ev && ev.target ? ev.target : null;
  if (target.shouldIgnoreChatSidebarDragTarget(eventTarget)) return;
  target._chatSidebarPointerActive = true;
  target._chatSidebarPointerMoved = false;
  target._chatSidebarPointerStartX = Number(ev.clientX || 0);
  target._chatSidebarPointerStartY = Number(ev.clientY || 0);
  target._chatSidebarPointerOriginLeft = target.chatSidebarResolvedLeft();
  target._chatSidebarPointerOriginTop = target.chatSidebarResolvedTop();
  target._chatSidebarPointerLastX = target._chatSidebarPointerStartX;
  target._chatSidebarPointerLastY = target._chatSidebarPointerStartY;
  target._chatSidebarPointerLastAt = Date.now();
  target._chatSidebarPointerVelocity = 0;
  target.chatSidebarDragLeft = target._chatSidebarPointerOriginLeft;
  target.chatSidebarDragTop = target._chatSidebarPointerOriginTop;
  target.bindChatSidebarPointerListeners();
  try {
    if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
      ev.currentTarget.setPointerCapture(ev.pointerId);
    }
  } catch(_) {}
}

function infringHandleChatSidebarPointerMove(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._chatSidebarPointerActive || target.page !== 'chat') return;
  var nextX = Number(ev.clientX || 0);
  var nextY = Number(ev.clientY || 0);
  var now = Date.now();
  var prevX = Number(target._chatSidebarPointerLastX || nextX);
  var prevY = Number(target._chatSidebarPointerLastY || nextY);
  var prevAt = Number(target._chatSidebarPointerLastAt || now);
  var dt = Math.max(1, now - prevAt);
  var stepDx = nextX - prevX;
  var stepDy = nextY - prevY;
  target._chatSidebarPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
  target._chatSidebarPointerLastX = nextX;
  target._chatSidebarPointerLastY = nextY;
  target._chatSidebarPointerLastAt = now;
  var movedX = Math.abs(nextX - Number(target._chatSidebarPointerStartX || 0));
  var movedY = Math.abs(nextY - Number(target._chatSidebarPointerStartY || 0));
  if (!target._chatSidebarPointerMoved) {
    if (movedX < 4 && movedY < 4) return;
    target._chatSidebarPointerMoved = true;
    target.chatSidebarDragActive = true;
    target.hideDashboardPopupBySource('sidebar');
  }
  var dragDx = nextX - Number(target._chatSidebarPointerStartX || 0);
  var dragDy = nextY - Number(target._chatSidebarPointerStartY || 0);
  var candidateLeft = Number(target._chatSidebarPointerOriginLeft || 0) + dragDx;
  var candidateTop = Number(target._chatSidebarPointerOriginTop || 0) + dragDy;
  var hardBounds = target.chatSidebarHardBounds();
  var lockedWall = target.chatSidebarWallLockNormalized();
  if (lockedWall) {
    var unlockDistance = target.dragSurfaceDistanceFromWall(hardBounds, candidateLeft, candidateTop, lockedWall);
    if (unlockDistance >= target.dragSurfaceWallUnlockDistanceThreshold()) {
      lockedWall = target.chatSidebarSetWallLock('');
    } else {
      var holdLeft = Number.isFinite(Number(target.chatSidebarDragLeft))
        ? Number(target.chatSidebarDragLeft)
        : Number(target._chatSidebarPointerOriginLeft || 0);
      var holdTop = Number.isFinite(Number(target.chatSidebarDragTop))
        ? Number(target.chatSidebarDragTop)
        : Number(target._chatSidebarPointerOriginTop || 0);
      var stayLocked = target.dragSurfaceApplyWallLock(hardBounds, holdLeft, holdTop, lockedWall);
      target.chatSidebarDragLeft = stayLocked.left;
      target.chatSidebarDragTop = stayLocked.top;
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
    var persistedLockWall = target.chatSidebarSetWallLock(lockWall);
    var snapped = target.dragSurfaceApplyWallLock(hardBounds, clamped.left, clamped.top, persistedLockWall);
    target.chatSidebarDragLeft = snapped.left;
    target.chatSidebarDragTop = snapped.top;
  } else {
    target.chatSidebarDragLeft = clamped.left;
    target.chatSidebarDragTop = clamped.top;
  }
  if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringEndChatSidebarPointerDrag(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._chatSidebarPointerActive) return;
  target._chatSidebarPointerActive = false;
  target.unbindChatSidebarPointerListeners();
  if (!target._chatSidebarPointerMoved) {
    target.chatSidebarDragActive = false;
    target._chatSidebarDragRowsCache = null;
    return;
  }
  target._chatSidebarPointerMoved = false;
  var hardBounds = target.chatSidebarHardBounds();
  var lockedWall = target.chatSidebarWallLockNormalized();
  var final;
  if (lockedWall) {
    final = target.dragSurfaceApplyWallLock(hardBounds, target.chatSidebarDragLeft, target.chatSidebarDragTop, lockedWall);
  } else {
    final = target.dragSurfaceClampWithBounds(hardBounds, target.chatSidebarDragLeft, target.chatSidebarDragTop);
  }
  target.chatSidebarPlacementAnchorId = '';
  try { localStorage.removeItem('infring-chat-sidebar-placement-anchor'); } catch(_) {}
  target.chatSidebarDragLeft = final.left;
  target.chatSidebarDragTop = final.top;
  target.chatSidebarPersistPlacementFromLeft(target.chatSidebarDragLeft);
  target.chatSidebarPersistPlacementFromTop(target.chatSidebarDragTop);
  target.chatSidebarDragActive = false;
  target._chatSidebarDragRowsCache = null;
  target._sidebarToggleSuppressUntil = Date.now() + 260;
}

function infringShouldSuppressSidebarToggle(page) {
  var target = page && typeof page === 'object' ? page : {};
  var until = Number(target._sidebarToggleSuppressUntil || 0);
  return Number.isFinite(until) && until > Date.now();
}
