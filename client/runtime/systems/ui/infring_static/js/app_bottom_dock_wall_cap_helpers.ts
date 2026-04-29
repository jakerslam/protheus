function infringSyncDragWallCapHostNode(page, node, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  if (!node || !node.classList) return;
  var wall = typeof target.dragSurfaceNormalizeWall === 'function'
    ? target.dragSurfaceNormalizeWall(wallRaw)
    : String(wallRaw || '').trim().toLowerCase();
  node.classList.add('drag-wall-cap-host');
  node.classList.remove('wall-lock-left', 'wall-lock-right', 'wall-lock-top', 'wall-lock-bottom');
  if (wall) node.classList.add('wall-lock-' + wall);
  var capA = null;
  var capB = null;
  var kids = node.children || [];
  for (var i = 0; i < kids.length; i += 1) {
    var child = kids[i];
    if (!child || !child.classList) continue;
    if (child.classList.contains('drag-bar-wall-cap--a')) capA = child;
    if (child.classList.contains('drag-bar-wall-cap--b')) capB = child;
  }
  if (!capA && typeof document !== 'undefined') {
    capA = document.createElement('span');
    capA.className = 'drag-bar-wall-cap drag-bar-wall-cap--a';
    capA.setAttribute('aria-hidden', 'true');
    node.appendChild(capA);
  }
  if (!capB && typeof document !== 'undefined') {
    capB = document.createElement('span');
    capB.className = 'drag-bar-wall-cap drag-bar-wall-cap--b';
    capB.setAttribute('aria-hidden', 'true');
    node.appendChild(capB);
  }
}

function infringSyncDragWallCaps(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof document === 'undefined') return;
  var sidebarNode = null;
  var chatMapSurfaceNode = null;
  var dockNode = null;
  try { sidebarNode = document.querySelector('.sidebar.drag-bar'); } catch(_) {}
  try { chatMapSurfaceNode = document.querySelector('.chat-map .chat-map-surface.drag-bar'); } catch(_) {}
  try { dockNode = document.querySelector('.bottom-dock.drag-bar'); } catch(_) {}
  infringSyncDragWallCapHostNode(
    target,
    sidebarNode,
    target.page === 'chat' && typeof target.chatSidebarWallLockNormalized === 'function'
      ? target.chatSidebarWallLockNormalized()
      : ''
  );
  infringSyncDragWallCapHostNode(
    target,
    chatMapSurfaceNode,
    typeof target.chatMapPlacementEnabled === 'function' && target.chatMapPlacementEnabled() && typeof target.chatMapWallLockNormalized === 'function'
      ? target.chatMapWallLockNormalized()
      : ''
  );
  infringSyncDragWallCapHostNode(
    target,
    dockNode,
    typeof target.bottomDockTaskbarContained === 'function' && target.bottomDockTaskbarContained()
      ? ''
      : (typeof target.bottomDockWallLockNormalized === 'function' ? target.bottomDockWallLockNormalized() : '')
  );
}

function infringBottomDockContainerStyle(page) {
  var target = page && typeof page === 'object' ? page : {};
  infringSyncDragWallCaps(target);
  var lockWall = typeof target.bottomDockWallLockNormalized === 'function' ? target.bottomDockWallLockNormalized() : '';
  var taskbarContained = typeof target.bottomDockTaskbarContained === 'function' ? target.bottomDockTaskbarContained() : false;
  var activeSnapId = target.bottomDockContainerDragActive && typeof target.bottomDockNearestSnapId === 'function'
    ? target.bottomDockNearestSnapId(target.bottomDockContainerDragX, target.bottomDockContainerDragY)
    : target.bottomDockPlacementId;
  var side = typeof target.bottomDockSideForSnapId === 'function' ? target.bottomDockSideForSnapId(activeSnapId) : 'bottom';
  if (lockWall) side = lockWall;
  var anchor = target.bottomDockContainerDragActive && typeof target.bottomDockClampDragAnchor === 'function'
    ? target.bottomDockClampDragAnchor(target.bottomDockContainerDragX, target.bottomDockContainerDragY)
    : (typeof target.bottomDockAnchorForSnapId === 'function' ? target.bottomDockAnchorForSnapId(target.bottomDockPlacementId) : { x: 0, y: 0 });
  if (lockWall && typeof target.bottomDockTopLeftFromAnchor === 'function' && typeof target.bottomDockHardBoundsForSide === 'function' && typeof target.dragSurfaceApplyWallLock === 'function' && typeof target.bottomDockAnchorFromTopLeft === 'function') {
    var topLeft = target.bottomDockTopLeftFromAnchor(anchor.x, anchor.y, side);
    var hardBounds = target.bottomDockHardBoundsForSide(side);
    var snapped = target.dragSurfaceApplyWallLock(hardBounds, topLeft.left, topLeft.top, lockWall);
    var lockedAnchor = target.bottomDockAnchorFromTopLeft(snapped.left, snapped.top, side);
    anchor = { x: Number(lockedAnchor.x || 0), y: Number(lockedAnchor.y || 0) };
  }
  var taskbarContainedMetrics = null;
  if (taskbarContained && typeof target.bottomDockTaskbarContainedMetrics === 'function') {
    taskbarContainedMetrics = target.bottomDockTaskbarContainedMetrics();
    if (typeof target.bottomDockTaskbarContainedAnchorX === 'function') anchor.x = target.bottomDockTaskbarContainedAnchorX(side);
    anchor.y = Number(taskbarContainedMetrics.centerY || anchor.y || 0);
  }
  var rotationDeg = Number(target.bottomDockRotationDeg);
  if (!Number.isFinite(rotationDeg)) {
    rotationDeg = typeof target.bottomDockResolveRotationForSide === 'function'
      ? target.bottomDockResolveRotationForSide(side, anchor.x, anchor.y)
      : 0;
    target.bottomDockRotationDeg = rotationDeg;
  }
  var upDeg = typeof target.bottomDockUpDegForSide === 'function' ? Number(target.bottomDockUpDegForSide(side) || 0) : 0;
  var tileRotationDeg = upDeg - Number(rotationDeg || 0);
  var iconRotationDeg = 0;
  var carriedByTaskbar = taskbarContained && target.taskbarDockDragActive;
  var durationMs = (target.bottomDockContainerDragActive || carriedByTaskbar)
    ? 0
    : (typeof target.bottomDockMoveDurationMs === 'function' ? target.bottomDockMoveDurationMs() : 0);
  var localLockWall = lockWall && !taskbarContained && typeof target.bottomDockLocalWallForRotation === 'function'
    ? target.bottomDockLocalWallForRotation(lockWall, rotationDeg)
    : '';
  var lockCss = typeof target.dragSurfaceLockVisualCssVars === 'function'
    ? target.dragSurfaceLockVisualCssVars('bottom-dock', localLockWall, { transformMs: target._dragSurfaceLockTransformMs })
    : '';
  return (
    lockCss +
    '--bottom-dock-anchor-x:' + Math.round(Number(anchor.x || 0)) + 'px;' +
    '--bottom-dock-anchor-y:' + Math.round(Number(anchor.y || 0)) + 'px;' +
    '--bottom-dock-taskbar-contained-height:' + Math.round(Number((taskbarContainedMetrics && taskbarContainedMetrics.height) || 32)) + 'px;' +
    '--bottom-dock-taskbar-contained-tile-size:' + Math.max(18, Math.round(Number((taskbarContainedMetrics && taskbarContainedMetrics.height) || 32) - 10)) + 'px;' +
    '--bottom-dock-position-transition:' + Math.max(0, Math.round(Number(durationMs || 0))) + 'ms;' +
    '--bottom-dock-up-deg:' + Math.round(Number(upDeg || 0)) + 'deg;' +
    '--bottom-dock-rotation-deg:' + Math.round(Number(rotationDeg || 0)) + 'deg;' +
    '--bottom-dock-tile-rotation-deg:' + Math.round(Number(tileRotationDeg || 0)) + 'deg;' +
    '--bottom-dock-icon-rotation-deg:' + Math.round(Number(iconRotationDeg || 0)) + 'deg;'
  );
}
