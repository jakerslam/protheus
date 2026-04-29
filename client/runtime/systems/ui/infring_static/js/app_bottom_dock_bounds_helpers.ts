// Canonical Shell helper source: bottom-dock bounds, anchor, and wall-radius projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringBottomDockBoundsScaleForSide(page, sideHint) {
  var target = page && typeof page === 'object' ? page : {};
  var activeSide = typeof target.bottomDockActiveSide === 'function' ? target.bottomDockActiveSide() : 'bottom';
  var side = infringBottomDockNormalizeSide(sideHint || activeSide);
  if (infringBottomDockTaskbarContained(target)) return 1;
  if (side === 'left' || side === 'right') return 1;
  var expandedScale = infringBottomDockExpandedScale(target);
  var baseScale = 0.95;
  var dragging = !!target.bottomDockContainerDragActive
    || !!target.bottomDockContainerSettling
    || !!String(target.bottomDockDragId || '').trim();
  var hovering = !!String(target.bottomDockHoverId || '').trim();
  if (infringBottomDockHoverExpansionDisabled(target)) hovering = false;
  if (dragging || hovering) baseScale = expandedScale;
  if (!Number.isFinite(baseScale) || baseScale <= 0.01) baseScale = 0.95;
  return baseScale;
}

function infringBottomDockVisualSizeForSide(page, sideHint) {
  var target = page && typeof page === 'object' ? page : {};
  var activeSide = typeof target.bottomDockActiveSide === 'function' ? target.bottomDockActiveSide() : 'bottom';
  var side = infringBottomDockNormalizeSide(sideHint || activeSide);
  var dock = infringBottomDockReadBaseSize();
  var scale = infringBottomDockBoundsScaleForSide(target, side);
  var baseWidth = Math.max(20, Number(dock.width || 0) * scale);
  var baseHeight = Math.max(20, Number(dock.height || 0) * scale);
  var visualWidth = infringBottomDockIsVerticalSide(side) ? baseHeight : baseWidth;
  var visualHeight = infringBottomDockIsVerticalSide(side) ? baseWidth : baseHeight;
  return { side: side, width: visualWidth, height: visualHeight };
}

function infringBottomDockHardBoundsForSide(page, sideHint) {
  var size = infringBottomDockVisualSizeForSide(page, sideHint);
  var view = infringBottomDockReadViewportSize();
  var width = Number(size && size.width || 0);
  var height = Number(size && size.height || 0);
  if (!Number.isFinite(width) || width < 1) width = 1;
  if (!Number.isFinite(height) || height < 1) height = 1;
  var viewportWidth = Number(view && view.width || 0);
  var viewportHeight = Number(view && view.height || 0);
  if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) viewportWidth = 1440;
  if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) viewportHeight = 900;
  return {
    minLeft: 0,
    maxLeft: Math.max(0, viewportWidth - width),
    minTop: 0,
    maxTop: Math.max(0, viewportHeight - height)
  };
}

function infringBottomDockTopLeftFromAnchor(page, anchorX, anchorY, sideHint) {
  var size = infringBottomDockVisualSizeForSide(page, sideHint);
  var x = Number(anchorX);
  var y = Number(anchorY);
  if (!Number.isFinite(x)) x = Number(infringBottomDockReadViewportSize().width || 0) * 0.5;
  if (!Number.isFinite(y)) y = Number(infringBottomDockReadViewportSize().height || 0) * 0.5;
  var side = infringBottomDockNormalizeSide(size && size.side);
  var top = y - (Number(size.height || 0) / 2);
  if (side === 'top') top = y;
  else if (side === 'bottom') top = y - Number(size.height || 0);
  return {
    left: x - (Number(size.width || 0) / 2),
    top: top,
    side: side
  };
}

function infringBottomDockAnchorFromTopLeft(page, leftRaw, topRaw, sideHint) {
  var size = infringBottomDockVisualSizeForSide(page, sideHint);
  var left = Number(leftRaw);
  var top = Number(topRaw);
  if (!Number.isFinite(left)) left = Number(size.width || 0) / -2;
  if (!Number.isFinite(top)) top = Number(size.height || 0) / -2;
  var side = infringBottomDockNormalizeSide(size && size.side);
  var y = top + (Number(size.height || 0) / 2);
  if (side === 'top') y = top;
  else if (side === 'bottom') y = top + Number(size.height || 0);
  return {
    x: left + (Number(size.width || 0) / 2),
    y: y,
    side: side
  };
}

function infringBottomDockLocalWallForRotation(page, wallRaw, rotationDegRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wall = typeof target.dragSurfaceNormalizeWall === 'function'
    ? target.dragSurfaceNormalizeWall(wallRaw)
    : String(wallRaw || '').trim().toLowerCase();
  if (!wall) return '';
  var rotationDeg = Number(rotationDegRaw);
  if (!Number.isFinite(rotationDeg)) rotationDeg = 0;
  var theta = (infringBottomDockNormalizeRotationDeg(rotationDeg) * Math.PI) / 180;
  var vx = 0;
  var vy = 0;
  if (wall === 'left') vx = -1;
  else if (wall === 'right') vx = 1;
  else if (wall === 'top') vy = -1;
  else vy = 1;
  var localX = (vx * Math.cos(theta)) + (vy * Math.sin(theta));
  var localY = (-vx * Math.sin(theta)) + (vy * Math.cos(theta));
  if (Math.abs(localX) >= Math.abs(localY)) {
    return localX >= 0 ? 'right' : 'left';
  }
  return localY >= 0 ? 'bottom' : 'top';
}

function infringBottomDockLockRadiusCssVars(page, wallRaw, rotationDegRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var wall = typeof target.dragSurfaceNormalizeWall === 'function'
    ? target.dragSurfaceNormalizeWall(wallRaw)
    : String(wallRaw || '').trim().toLowerCase();
  if (!wall) return '';
  var localWall = infringBottomDockLocalWallForRotation(target, wall, rotationDegRaw);
  var radius = typeof target.dragSurfaceRadiusByWall === 'function' ? target.dragSurfaceRadiusByWall(localWall) : '';
  return '--bottom-dock-radius-override:' + radius + ';';
}

function infringBottomDockClampDragAnchor(anchorX, anchorY) {
  var view = infringBottomDockReadViewportSize();
  var margin = 8;
  var minX = margin;
  var maxX = Number(view.width || 0) - margin;
  var minY = margin;
  var maxY = Number(view.height || 0) - margin;
  var x = Number(anchorX);
  var y = Number(anchorY);
  if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
  if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
  x = Math.max(minX, Math.min(maxX, x));
  y = Math.max(minY, Math.min(maxY, y));
  return { x: x, y: y };
}

function infringBottomDockAnchorForSnapId(page, id) {
  var snap = infringBottomDockSnapDefinitionById(page, id);
  var view = infringBottomDockReadViewportSize();
  var x = Number(view.width || 0) * Number(snap && snap.x || 0.5);
  var y = Number(view.height || 0) * Number(snap && snap.y || 0.995);
  var side = infringBottomDockNormalizeSide(snap && snap.side || 'bottom');
  return infringBottomDockClampDragAnchor(x, y, side);
}

function infringBottomDockNearestSnapId(page, anchorX, anchorY) {
  var defs = infringBottomDockSnapDefinitions(page);
  if (!defs.length) return 'center';
  var anchor = infringBottomDockClampDragAnchor(anchorX, anchorY);
  var bestId = defs[0].id;
  var bestDist = Number.POSITIVE_INFINITY;
  for (var i = 0; i < defs.length; i += 1) {
    var row = defs[i];
    if (!row) continue;
    var snapAnchor = infringBottomDockAnchorForSnapId(page, row.id);
    var dx = Number(anchor.x || 0) - Number(snapAnchor.x || 0);
    var dy = Number(anchor.y || 0) - Number(snapAnchor.y || 0);
    var dist = (dx * dx) + (dy * dy);
    if (!Number.isFinite(dist)) continue;
    if (dist >= bestDist) continue;
    bestDist = dist;
    bestId = row.id;
  }
  return String(bestId || 'center');
}
