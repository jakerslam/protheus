// Canonical Shell helper source: bottom-dock snap and taskbar-containment projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringBottomDockSnapDefinitions(page) {
  var target = page && typeof page === 'object' ? page : {};
  var source = Array.isArray(target.bottomDockSnapPoints) ? target.bottomDockSnapPoints : [];
  var out = [];
  var seen = {};
  for (var i = 0; i < source.length; i += 1) {
    var row = source[i];
    if (!row || typeof row !== 'object') continue;
    var id = String(row.id || '').trim().toLowerCase();
    if (!id || seen[id]) continue;
    var nx = Number(row.x);
    var ny = Number(row.y);
    var side = infringBottomDockNormalizeSide(row.side);
    if (!Number.isFinite(nx)) nx = 0.5;
    if (!Number.isFinite(ny)) ny = 0.995;
    nx = Math.max(0, Math.min(1, nx));
    ny = Math.max(0, Math.min(1, ny));
    seen[id] = true;
    out.push({ id: id, x: nx, y: ny, side: side });
  }
  if (!out.length) {
    out.push({ id: 'center', x: 0.5, y: 0.995, side: 'bottom' });
  }
  return out;
}

function infringBottomDockSnapDefinitionById(page, id) {
  var key = String(id || '').trim().toLowerCase();
  var defs = infringBottomDockSnapDefinitions(page);
  if (!defs.length) return null;
  for (var i = 0; i < defs.length; i += 1) {
    if (defs[i] && defs[i].id === key) return defs[i];
  }
  for (var j = 0; j < defs.length; j += 1) {
    if (defs[j] && defs[j].id === 'center') return defs[j];
  }
  return defs[0] || null;
}

function infringBottomDockSideForSnapId(page, id) {
  var snap = infringBottomDockSnapDefinitionById(page, id);
  return infringBottomDockNormalizeSide(snap && snap.side || 'bottom');
}

function infringBottomDockActiveSnapId(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target.bottomDockContainerDragActive) {
    var anchor = typeof target.bottomDockClampDragAnchor === 'function'
      ? target.bottomDockClampDragAnchor(target.bottomDockContainerDragX, target.bottomDockContainerDragY)
      : { x: target.bottomDockContainerDragX, y: target.bottomDockContainerDragY };
    return typeof target.bottomDockNearestSnapId === 'function'
      ? target.bottomDockNearestSnapId(anchor.x, anchor.y)
      : 'center';
  }
  var snap = infringBottomDockSnapDefinitionById(target, target.bottomDockPlacementId);
  return String(snap && snap.id || 'center');
}

function infringBottomDockActiveSide(page) {
  return infringBottomDockSideForSnapId(page, infringBottomDockActiveSnapId(page));
}

function infringBottomDockWallLockNormalized(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof target.dragSurfaceNormalizeWall === 'function') {
    return target.dragSurfaceNormalizeWall(target.bottomDockContainerWallLock);
  }
  var wall = String(target.bottomDockContainerWallLock || '').trim().toLowerCase();
  return wall === 'top' || wall === 'bottom' || wall === 'left' || wall === 'right' ? wall : '';
}

function infringBottomDockTaskbarContained(page) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.dockTaskbarContained === 'function') {
    return service.dockTaskbarContained(
      infringBottomDockWallLockNormalized(target),
      target.taskbarDockEdge,
      target.taskbarDockDragActive,
      target._taskbarDockDraggingContainedBottomDock
    );
  }
  var wall = infringBottomDockWallLockNormalized(target);
  if (wall !== 'top' && wall !== 'bottom') return false;
  if (target.taskbarDockDragActive && String(target._taskbarDockDraggingContainedBottomDock || '') === wall) return true;
  if (typeof target.taskbarDockEdgeNormalized === 'function') {
    return wall === target.taskbarDockEdgeNormalized(target.taskbarDockEdge);
  }
  return wall === String(target.taskbarDockEdge || '').trim().toLowerCase();
}

function infringBottomDockHoverExpansionDisabled(page) {
  return infringBottomDockTaskbarContained(page);
}

function infringBottomDockTaskbarContainedAnchorX(page, sideHint) {
  var target = page && typeof page === 'object' ? page : {};
  var activeSide = typeof target.bottomDockActiveSide === 'function' ? target.bottomDockActiveSide() : 'bottom';
  var side = infringBottomDockNormalizeSide(sideHint || activeSide);
  var view = infringBottomDockReadViewportSize();
  var size = typeof target.bottomDockVisualSizeForSide === 'function'
    ? target.bottomDockVisualSizeForSide(side)
    : { width: 420 };
  var dockWidth = Math.max(1, Number(size && size.width || 1));
  var left = 16;
  try {
    var textMenu = document.querySelector('.global-taskbar .taskbar-text-menus');
    var rect = textMenu && typeof textMenu.getBoundingClientRect === 'function' ? textMenu.getBoundingClientRect() : null;
    if (rect && Number.isFinite(Number(rect.right))) left = Number(rect.right) + 8;
  } catch(_) {}
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.dockTaskbarContainedAnchorX === 'function') {
    return service.dockTaskbarContainedAnchorX({
      side: side,
      viewportWidth: Number(view.width || 0),
      dockWidth: dockWidth,
      leftAnchor: left
    });
  }
  var minX = dockWidth / 2;
  var maxX = Math.max(minX, Number(view.width || 0) - minX - 10);
  return Math.max(minX, Math.min(maxX, left + (dockWidth / 2)));
}

function infringBottomDockTaskbarContainedMetrics(page) {
  var target = page && typeof page === 'object' ? page : {};
  var edge = typeof target.taskbarDockEdgeNormalized === 'function'
    ? target.taskbarDockEdgeNormalized(target.taskbarDockEdge)
    : String(target.taskbarDockEdge || '').trim().toLowerCase();
  var height = 32;
  var viewportHeight = typeof target.taskbarReadViewportHeight === 'function' ? target.taskbarReadViewportHeight() : 900;
  var centerY = edge === 'bottom' ? viewportHeight - 23 : 23;
  try {
    var group = document.querySelector('.global-taskbar .taskbar-visual-group-left');
    var rect = group && typeof group.getBoundingClientRect === 'function' ? group.getBoundingClientRect() : null;
    if (rect && Number.isFinite(Number(rect.height)) && Number(rect.height) > 0) {
      height = Number(rect.height);
      centerY = Number(rect.top || 0) + (height / 2);
    }
  } catch(_) {}
  var dragging = target.taskbarDockDragActive && String(target._taskbarDockDraggingContainedBottomDock || '');
  var dragY = typeof target.taskbarClampDragY === 'function' ? target.taskbarClampDragY(target.taskbarDockDragY) : Number(target.taskbarDockDragY || 0);
  var taskbarHeight = typeof target.taskbarReadHeight === 'function' ? target.taskbarReadHeight() : 32;
  if (dragging) {
    centerY = dragY + (taskbarHeight / 2);
  }
  var service = typeof target.taskbarDockService === 'function' ? target.taskbarDockService() : null;
  if (service && typeof service.dockTaskbarContainedMetrics === 'function') {
    return service.dockTaskbarContainedMetrics({
      edge: edge,
      viewportHeight: viewportHeight,
      fallbackHeight: 32,
      groupHeight: height,
      groupTop: centerY - (height / 2),
      dragging: dragging,
      dragY: dragY,
      taskbarHeight: taskbarHeight
    });
  }
  return { height: height, centerY: centerY };
}
