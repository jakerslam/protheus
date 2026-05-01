function infringBindBottomDockContainerPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof window === 'undefined') return;
  if (target._bottomDockContainerPointerMoveHandler || target._bottomDockContainerPointerUpHandler) return;
  target._bottomDockContainerPointerMoveHandler = function(ev) { target.handleBottomDockContainerPointerMove(ev); };
  target._bottomDockContainerPointerUpHandler = function(ev) { target.endBottomDockContainerPointerDrag(ev); };
  window.addEventListener('pointermove', target._bottomDockContainerPointerMoveHandler, true);
  window.addEventListener('pointerup', target._bottomDockContainerPointerUpHandler, true);
  window.addEventListener('pointercancel', target._bottomDockContainerPointerUpHandler, true);
  window.addEventListener('mousemove', target._bottomDockContainerPointerMoveHandler, true);
  window.addEventListener('mouseup', target._bottomDockContainerPointerUpHandler, true);
}

function infringUnbindBottomDockContainerPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof window === 'undefined') return;
  if (target._bottomDockContainerPointerMoveHandler) {
    try { window.removeEventListener('pointermove', target._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
    try { window.removeEventListener('mousemove', target._bottomDockContainerPointerMoveHandler, true); } catch(_) {}
  }
  if (target._bottomDockContainerPointerUpHandler) {
    try { window.removeEventListener('pointerup', target._bottomDockContainerPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('pointercancel', target._bottomDockContainerPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('mouseup', target._bottomDockContainerPointerUpHandler, true); } catch(_) {}
  }
  target._bottomDockContainerPointerMoveHandler = null;
  target._bottomDockContainerPointerUpHandler = null;
}

function infringStartBottomDockContainerPointerDrag(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!ev || Number(ev.button) !== 0) return;
  if (String(target.bottomDockDragId || '').trim()) return;
  var eventTarget = ev && ev.target ? ev.target : null;
  if (eventTarget && typeof eventTarget.closest === 'function') {
    var tileNode = eventTarget.closest('.bottom-dock-btn[data-dock-id]');
    if (tileNode) return;
  }
  if (target._bottomDockContainerSettleTimer) {
    try { clearTimeout(target._bottomDockContainerSettleTimer); } catch(_) {}
  }
  target._bottomDockContainerSettleTimer = 0;
  target.bottomDockContainerSettling = false;
  var anchor = target.bottomDockAnchorForSnapId(target.bottomDockPlacementId);
  target._bottomDockContainerPointerActive = true;
  target._bottomDockContainerPointerMoved = false;
  target._bottomDockContainerPointerStartX = Number(ev.clientX || 0);
  target._bottomDockContainerPointerStartY = Number(ev.clientY || 0);
  target._bottomDockContainerPointerLastX = Number(ev.clientX || 0);
  target._bottomDockContainerPointerLastY = Number(ev.clientY || 0);
  target._bottomDockContainerOriginX = Number(anchor.x || 0);
  target._bottomDockContainerOriginY = Number(anchor.y || 0);
  target.bottomDockContainerDragX = Number(anchor.x || 0);
  target.bottomDockContainerDragY = Number(anchor.y || 0);
  target._bottomDockContainerDragWallLock = target.bottomDockWallLockNormalized();
  target.bindBottomDockContainerPointerListeners();
  try {
    if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
      ev.currentTarget.setPointerCapture(ev.pointerId);
    }
  } catch(_) {}
  if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringHandleBottomDockContainerPointerMove(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._bottomDockContainerPointerActive) return;
  var nextX = Number(ev.clientX || 0);
  var nextY = Number(ev.clientY || 0);
  target._bottomDockContainerPointerLastX = nextX;
  target._bottomDockContainerPointerLastY = nextY;
  var movedX = Math.abs(nextX - Number(target._bottomDockContainerPointerStartX || 0));
  var movedY = Math.abs(nextY - Number(target._bottomDockContainerPointerStartY || 0));
  if (!target._bottomDockContainerPointerMoved) {
    if (movedX < 4 && movedY < 4) return;
    target._bottomDockContainerPointerMoved = true;
    target.bottomDockContainerDragActive = true;
    target.bottomDockHoverId = '';
    target.bottomDockHoverWeightById = {};
    target.bottomDockPointerX = 0;
    target.bottomDockPointerY = 0;
    target.bottomDockPreviewVisible = false;
    target.bottomDockPreviewText = '';
    target.bottomDockPreviewMorphFromText = '';
    target.bottomDockPreviewLabelMorphing = false;
    target.bottomDockPreviewWidth = 0;
    target.cancelBottomDockPreviewReflow();
  }
  var candidateX = Number(target._bottomDockContainerOriginX || 0) + (nextX - Number(target._bottomDockContainerPointerStartX || 0));
  var candidateY = Number(target._bottomDockContainerOriginY || 0) + (nextY - Number(target._bottomDockContainerPointerStartY || 0));
  var lockedWall = target.dragSurfaceNormalizeWall(target._bottomDockContainerDragWallLock || target.bottomDockWallLockNormalized());
  if (lockedWall) {
    var lockedTopLeft = target.bottomDockTopLeftFromAnchor(candidateX, candidateY, lockedWall);
    var lockedHardBounds = target.bottomDockHardBoundsForSide(lockedWall);
    var unlockDistance = target.dragSurfaceDistanceFromWall(lockedHardBounds, lockedTopLeft.left, lockedTopLeft.top, lockedWall);
    if (unlockDistance >= target.dragSurfaceWallUnlockDistanceThreshold()) {
      lockedWall = '';
      target._bottomDockContainerDragWallLock = '';
      target.bottomDockSetWallLock('');
    } else {
      var holdTopLeft = target.bottomDockTopLeftFromAnchor(target.bottomDockContainerDragX, target.bottomDockContainerDragY, lockedWall);
      var holdLocked = target.dragSurfaceApplyWallLock(lockedHardBounds, holdTopLeft.left, holdTopLeft.top, lockedWall);
      var holdAnchor = target.bottomDockAnchorFromTopLeft(holdLocked.left, holdLocked.top, lockedWall);
      target.bottomDockContainerDragX = Number(holdAnchor.x || 0);
      target.bottomDockContainerDragY = Number(holdAnchor.y || 0);
      target.bottomDockRotationDeg = target.bottomDockResolveRotationForSide(lockedWall, target.bottomDockContainerDragX, target.bottomDockContainerDragY);
      if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
      return;
    }
  }
  var anchor = target.bottomDockClampDragAnchor(candidateX, candidateY);
  var nearestId = target.bottomDockNearestSnapId(anchor.x, anchor.y);
  var side = target.bottomDockSideForSnapId(nearestId);
  var candidateTopLeft = target.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, side);
  var hardBounds = target.bottomDockHardBoundsForSide(side);
  var clampedTopLeft = target.dragSurfaceClampWithBounds(hardBounds, candidateTopLeft.left, candidateTopLeft.top);
  var nearestWall = target.dragSurfaceNearestWall(hardBounds, clampedTopLeft.left, clampedTopLeft.top);
  var lockWall = target.dragSurfaceResolveWallLock(
    hardBounds,
    candidateTopLeft.left,
    candidateTopLeft.top,
    nearestWall,
    nextX - Number(target._bottomDockContainerPointerStartX || 0),
    nextY - Number(target._bottomDockContainerPointerStartY || 0)
  );
  if (lockWall) {
    target._bottomDockContainerDragWallLock = target.bottomDockSetWallLock(lockWall);
    var lockTopLeft = target.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, lockWall);
    var lockHardBounds = target.bottomDockHardBoundsForSide(lockWall);
    var lockClamped = target.dragSurfaceClampWithBounds(lockHardBounds, lockTopLeft.left, lockTopLeft.top);
    var snapped = target.dragSurfaceApplyWallLock(lockHardBounds, lockClamped.left, lockClamped.top, lockWall);
    var snappedAnchor = target.bottomDockAnchorFromTopLeft(snapped.left, snapped.top, lockWall);
    target.bottomDockContainerDragX = Number(snappedAnchor.x || 0);
    target.bottomDockContainerDragY = Number(snappedAnchor.y || 0);
    side = lockWall;
  } else {
    var freeAnchor = target.bottomDockAnchorFromTopLeft(clampedTopLeft.left, clampedTopLeft.top, side);
    target.bottomDockContainerDragX = Number(freeAnchor.x || 0);
    target.bottomDockContainerDragY = Number(freeAnchor.y || 0);
  }
  target.bottomDockRotationDeg = target.bottomDockResolveRotationForSide(side, target.bottomDockContainerDragX, target.bottomDockContainerDragY);
  if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringEndBottomDockContainerPointerDrag(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._bottomDockContainerPointerActive) return;
  target._bottomDockContainerPointerActive = false;
  target.unbindBottomDockContainerPointerListeners();
  if (!target._bottomDockContainerPointerMoved) {
    target.bottomDockContainerDragActive = false;
    target._bottomDockContainerPointerMoved = false;
    target._bottomDockContainerDragWallLock = '';
    return;
  }
  var lockWall = target.dragSurfaceNormalizeWall(target._bottomDockContainerDragWallLock || target.bottomDockWallLockNormalized());
  var anchor = target.bottomDockClampDragAnchor(target.bottomDockContainerDragX, target.bottomDockContainerDragY);
  if (!lockWall) {
    var freeNearest = target.bottomDockNearestSnapId(anchor.x, anchor.y);
    var freeSide = target.bottomDockSideForSnapId(freeNearest);
    var freeTopLeft = target.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, freeSide);
    var freeHardBounds = target.bottomDockHardBoundsForSide(freeSide);
    var freeNearestWall = target.dragSurfaceNearestWall(freeHardBounds, freeTopLeft.left, freeTopLeft.top);
    if (Number(freeNearestWall.distance || 0) <= target.dragSurfaceWallLockDistanceThreshold()) {
      lockWall = target.bottomDockSetWallLock(freeNearestWall.wall);
      target._bottomDockContainerDragWallLock = lockWall;
    }
  }
  if (lockWall) {
    var lockedTopLeft = target.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, lockWall);
    var lockedHardBounds = target.bottomDockHardBoundsForSide(lockWall);
    var finalLocked = target.dragSurfaceApplyWallLock(lockedHardBounds, lockedTopLeft.left, lockedTopLeft.top, lockWall);
    var finalAnchor = target.bottomDockAnchorFromTopLeft(finalLocked.left, finalLocked.top, lockWall);
    anchor = { x: Number(finalAnchor.x || 0), y: Number(finalAnchor.y || 0) };
  }
  var nearestId = target.bottomDockNearestSnapId(anchor.x, anchor.y);
  target.bottomDockPlacementId = nearestId;
  target.bottomDockRotationDeg = target.bottomDockResolveRotationForSide(target.bottomDockSideForSnapId(nearestId), anchor.x, anchor.y);
  target.persistBottomDockPlacement();
  target.bottomDockContainerDragActive = false;
  target.bottomDockContainerSettling = true;
  target._bottomDockContainerPointerMoved = false;
  target._bottomDockContainerDragWallLock = '';
  if (target._bottomDockContainerSettleTimer) {
    try { clearTimeout(target._bottomDockContainerSettleTimer); } catch(_) {}
  }
  var settleMs = target.bottomDockMoveDurationMs() + 36;
  target._bottomDockContainerSettleTimer = window.setTimeout(function() {
    target._bottomDockContainerSettleTimer = 0;
    target.bottomDockContainerSettling = false;
  }, settleMs);
}
