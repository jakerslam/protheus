function infringOverlayWallGapPx() {
  var fallback = 16;
  if (typeof window !== 'undefined' && typeof window.getComputedStyle === 'function' && document && document.documentElement) {
    try {
      var raw = String(window.getComputedStyle(document.documentElement).getPropertyValue('--overlay-wall-gap') || '').trim();
      var parsed = parseFloat(raw);
      if (Number.isFinite(parsed) && parsed >= 0) fallback = parsed;
    } catch(_) {}
  }
  return Math.max(0, Math.round(fallback));
}

function infringChatOverlayVerticalBounds(page) {
  var target = page && typeof page === 'object' ? page : {};
  var viewportHeight = target.taskbarReadViewportHeight();
  var wallGap = target.overlayWallGapPx();
  var edge = target.taskbarDockEdgeNormalized(target.taskbarDockEdge);
  var taskbarH = target.taskbarReadHeight();
  var topInset = edge === 'top' ? taskbarH : 0;
  var bottomInset = edge === 'bottom' ? taskbarH : 0;
  return {
    minTop: topInset + wallGap,
    maxBottom: viewportHeight - bottomInset - wallGap,
    viewportHeight: viewportHeight,
    wallGap: wallGap
  };
}

function infringDragSurfaceHardBounds(page, widthRaw, heightRaw, ignoreTaskbarBoundaryRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var width = Number(widthRaw || 0);
  var height = Number(heightRaw || 0);
  if (!Number.isFinite(width) || width < 1) width = 1;
  if (!Number.isFinite(height) || height < 1) height = 1;
  var ignoreTaskbarBoundary = true;
  if (typeof ignoreTaskbarBoundaryRaw === 'boolean') {
    ignoreTaskbarBoundary = ignoreTaskbarBoundaryRaw;
  } else if (ignoreTaskbarBoundaryRaw && typeof ignoreTaskbarBoundaryRaw === 'object') {
    if (Object.prototype.hasOwnProperty.call(ignoreTaskbarBoundaryRaw, 'ignoreTaskbarBoundary')) {
      ignoreTaskbarBoundary = Boolean(ignoreTaskbarBoundaryRaw.ignoreTaskbarBoundary);
    }
  }
  var viewportWidth = target.chatOverlayViewportWidth();
  var viewportHeight = target.taskbarReadViewportHeight();
  var minTop = 0;
  var maxBottom = viewportHeight;
  if (!ignoreTaskbarBoundary) {
    var edge = target.taskbarDockEdgeNormalized(target.taskbarDockEdge);
    var taskbarH = target.taskbarReadHeight();
    minTop = edge === 'top' ? taskbarH : 0;
    maxBottom = viewportHeight - (edge === 'bottom' ? taskbarH : 0);
  }
  var service = target.dragbarService();
  if (service && typeof service.hardBounds === 'function') {
    return service.hardBounds({
      width: width,
      height: height,
      viewportWidth: viewportWidth,
      viewportHeight: viewportHeight,
      minTop: minTop,
      maxBottom: maxBottom
    });
  }
  return {
    minLeft: 0,
    maxLeft: Math.max(0, viewportWidth - width),
    minTop: minTop,
    maxTop: Math.max(minTop, maxBottom - height)
  };
}

function infringDragSurfaceSoftBounds(page, widthRaw, heightRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var width = Number(widthRaw || 0);
  var height = Number(heightRaw || 0);
  if (!Number.isFinite(width) || width < 1) width = 1;
  if (!Number.isFinite(height) || height < 1) height = 1;
  var vertical = target.chatOverlayVerticalBounds();
  var wallGap = target.overlayWallGapPx();
  var minLeft = wallGap;
  var maxLeft = Math.max(minLeft, target.chatOverlayViewportWidth() - wallGap - width);
  var minTop = Number(vertical.minTop || 0);
  var maxTop = Math.max(minTop, Number(vertical.maxBottom || 0) - height);
  var service = target.dragbarService();
  if (service && typeof service.softBounds === 'function') {
    return service.softBounds({
      width: width,
      height: height,
      wallGap: wallGap,
      viewportWidth: target.chatOverlayViewportWidth(),
      minTop: minTop,
      maxBottom: Number(vertical.maxBottom || 0)
    });
  }
  return { minLeft: minLeft, maxLeft: maxLeft, minTop: minTop, maxTop: maxTop };
}

function infringDragSurfaceClampWithBounds(page, bounds, leftRaw, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.clampWithBounds === 'function') {
    return service.clampWithBounds(bounds, leftRaw, topRaw);
  }
  var box = bounds && typeof bounds === 'object' ? bounds : { minLeft: 0, maxLeft: 0, minTop: 0, maxTop: 0 };
  var left = Number(leftRaw); if (!Number.isFinite(left)) left = Number(box.minLeft || 0);
  var top = Number(topRaw); if (!Number.isFinite(top)) top = Number(box.minTop || 0);
  return {
    left: Math.max(Number(box.minLeft || 0), Math.min(Number(box.maxLeft || 0), left)),
    top: Math.max(Number(box.minTop || 0), Math.min(Number(box.maxTop || 0), top))
  };
}

function infringDragSurfaceNearestWall(page, bounds, leftRaw, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.nearestWall === 'function') {
    return service.nearestWall(bounds, leftRaw, topRaw);
  }
  var clamped = target.dragSurfaceClampWithBounds(bounds, leftRaw, topRaw);
  var distances = {
    left: Math.max(0, clamped.left - Number(bounds.minLeft || 0)),
    right: Math.max(0, Number(bounds.maxLeft || 0) - clamped.left),
    top: Math.max(0, clamped.top - Number(bounds.minTop || 0)),
    bottom: Math.max(0, Number(bounds.maxTop || 0) - clamped.top)
  };
  var wall = 'left';
  var distance = Number(distances.left || 0);
  ['right', 'top', 'bottom'].forEach(function(key) {
    var next = Number(distances[key] || 0);
    if (next < distance) { wall = key; distance = next; }
  });
  return { wall: wall, distance: Math.max(0, distance), distances: distances, left: clamped.left, top: clamped.top };
}

function infringDragSurfaceNormalizeWall(page, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.normalizeWall === 'function') {
    return service.normalizeWall(wallRaw);
  }
  var wall = String(wallRaw || '').trim().toLowerCase();
  if (wall === 'left' || wall === 'right' || wall === 'top' || wall === 'bottom') return wall;
  return '';
}

function infringDragSurfaceApplyWallLock(page, bounds, leftRaw, topRaw, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.applyWallLock === 'function') {
    return service.applyWallLock(bounds, leftRaw, topRaw, wallRaw);
  }
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  var clamped = target.dragSurfaceClampWithBounds(bounds, leftRaw, topRaw);
  if (!wall) return { left: clamped.left, top: clamped.top, wall: '' };
  if (wall === 'left') clamped.left = Number(bounds.minLeft || 0);
  else if (wall === 'right') clamped.left = Number(bounds.maxLeft || 0);
  else if (wall === 'top') clamped.top = Number(bounds.minTop || 0);
  else if (wall === 'bottom') clamped.top = Number(bounds.maxTop || 0);
  return { left: clamped.left, top: clamped.top, wall: wall };
}

function infringDragSurfaceDistanceFromWall(page, bounds, leftRaw, topRaw, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.distanceFromWall === 'function') {
    return service.distanceFromWall(bounds, leftRaw, topRaw, wallRaw);
  }
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  if (!wall) return Number.POSITIVE_INFINITY;
  var clamped = target.dragSurfaceClampWithBounds(bounds, leftRaw, topRaw);
  if (wall === 'left') return Math.max(0, clamped.left - Number(bounds.minLeft || 0));
  if (wall === 'right') return Math.max(0, Number(bounds.maxLeft || 0) - clamped.left);
  if (wall === 'top') return Math.max(0, clamped.top - Number(bounds.minTop || 0));
  return Math.max(0, Number(bounds.maxTop || 0) - clamped.top);
}

function infringDragSurfaceWallLockOvershoot(page, bounds, leftRaw, topRaw, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.wallLockOvershoot === 'function') {
    return service.wallLockOvershoot(bounds, leftRaw, topRaw, wallRaw);
  }
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  if (!wall) return 0;
  var left = Number(leftRaw);
  var top = Number(topRaw);
  if (!Number.isFinite(left)) left = Number(bounds.minLeft || 0);
  if (!Number.isFinite(top)) top = Number(bounds.minTop || 0);
  if (wall === 'left') return Math.max(0, Number(bounds.minLeft || 0) - left);
  if (wall === 'right') return Math.max(0, left - Number(bounds.maxLeft || 0));
  if (wall === 'top') return Math.max(0, Number(bounds.minTop || 0) - top);
  return Math.max(0, top - Number(bounds.maxTop || 0));
}

function infringDragSurfaceCenteredPoint(page, bounds) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.centeredPoint === 'function') {
    return service.centeredPoint(bounds);
  }
  var box = bounds && typeof bounds === 'object' ? bounds : { minLeft: 0, maxLeft: 0, minTop: 0, maxTop: 0 };
  var left = Number(box.minLeft || 0) + ((Number(box.maxLeft || 0) - Number(box.minLeft || 0)) * 0.5);
  var top = Number(box.minTop || 0) + ((Number(box.maxTop || 0) - Number(box.minTop || 0)) * 0.5);
  return { left: left, top: top };
}
