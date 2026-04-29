function infringDragSurfaceWallLockContactThreshold(page) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(target.overlayWallGapPx()).contact;
  return Math.max(2, Math.round(target.overlayWallGapPx() * 0.12));
}

function infringDragSurfaceWallLockDistanceThreshold(page) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(target.overlayWallGapPx()).distance;
  return Math.max(8, Math.round(target.overlayWallGapPx() * 0.7));
}

function infringDragSurfaceWallUnlockDistanceThreshold(page) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(target.overlayWallGapPx()).unlock;
  return Math.max(42, Math.round(target.overlayWallGapPx() * 2.6));
}

function infringDragSurfaceWallLockOvershootThreshold(page) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.wallLockThresholds === 'function') return service.wallLockThresholds(target.overlayWallGapPx()).overshoot;
  return Math.max(5, Math.round(target.overlayWallGapPx() * 0.34));
}

function infringDragSurfaceResolveWallLock(page, bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.resolveWallLock === 'function') {
    return service.resolveWallLock(bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw, {
      wallGap: target.overlayWallGapPx()
    });
  }
  var walls = ['left', 'right', 'top', 'bottom'];
  var overshootThreshold = target.dragSurfaceWallLockOvershootThreshold();
  var contactThreshold = target.dragSurfaceWallLockContactThreshold();
  var distanceThreshold = target.dragSurfaceWallLockDistanceThreshold();
  var overshootWall = '';
  var overshootValue = 0;
  for (var i = 0; i < walls.length; i += 1) {
    var wall = walls[i];
    var overshoot = target.dragSurfaceWallLockOvershoot(bounds, candidateLeft, candidateTop, wall);
    if (overshoot >= overshootThreshold && overshoot > overshootValue) {
      overshootValue = overshoot;
      overshootWall = wall;
    }
  }
  if (overshootWall) return overshootWall;
  var clamped = target.dragSurfaceClampWithBounds(bounds, candidateLeft, candidateTop);
  var touchedWalls = [];
  if (Math.abs(clamped.left - Number(bounds.minLeft || 0)) <= contactThreshold) touchedWalls.push('left');
  if (Math.abs(Number(bounds.maxLeft || 0) - clamped.left) <= contactThreshold) touchedWalls.push('right');
  if (Math.abs(clamped.top - Number(bounds.minTop || 0)) <= contactThreshold) touchedWalls.push('top');
  if (Math.abs(Number(bounds.maxTop || 0) - clamped.top) <= contactThreshold) touchedWalls.push('bottom');
  if (touchedWalls.length === 1) return touchedWalls[0];
  if (touchedWalls.length > 1) {
    var motionDx = Number(motionDxRaw || 0);
    var motionDy = Number(motionDyRaw || 0);
    var absDx = Math.abs(motionDx);
    var absDy = Math.abs(motionDy);
    if (absDx > absDy + 0.25) {
      if (motionDx >= 0 && touchedWalls.indexOf('right') >= 0) return 'right';
      if (motionDx < 0 && touchedWalls.indexOf('left') >= 0) return 'left';
    } else if (absDy > absDx + 0.25) {
      if (motionDy >= 0 && touchedWalls.indexOf('bottom') >= 0) return 'bottom';
      if (motionDy < 0 && touchedWalls.indexOf('top') >= 0) return 'top';
    }
    var nearestWall = nearest && typeof nearest.wall === 'string' ? target.dragSurfaceNormalizeWall(nearest.wall) : '';
    if (nearestWall && touchedWalls.indexOf(nearestWall) >= 0) return nearestWall;
    return touchedWalls[0];
  }
  var edgeDistance = nearest && Number.isFinite(Number(nearest.distance)) ? Number(nearest.distance) : Number.POSITIVE_INFINITY;
  if (!Number.isFinite(edgeDistance) || edgeDistance > distanceThreshold) return '';
  return target.dragSurfaceNormalizeWall(nearest && nearest.wall ? nearest.wall : '');
}

function infringDragSurfaceRadiusByWall(page, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.radiusByWall === 'function') {
    return service.radiusByWall(wallRaw);
  }
  var r = 'var(--overlay-shared-surface-radius, var(--overlay-surface-radius, 18px))';
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  if (wall === 'left') return '0 ' + r + ' ' + r + ' 0';
  if (wall === 'right') return r + ' 0 0 ' + r;
  if (wall === 'top') return '0 0 ' + r + ' ' + r;
  if (wall === 'bottom') return r + ' ' + r + ' 0 0';
  return r;
}

function infringDragSurfaceLockTransformTimeMs(page, rawValue) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.lockTransformTimeMs === 'function') {
    return service.lockTransformTimeMs(rawValue, target._dragSurfaceLockTransformMs || 500);
  }
  var fallback = Number(target._dragSurfaceLockTransformMs || 500);
  if (!Number.isFinite(fallback)) fallback = 500;
  var raw = Number(rawValue);
  if (!Number.isFinite(raw)) raw = fallback;
  return Math.max(120, Math.round(raw));
}

function infringDragSurfaceLockBorderFadeDurationMs(page, transformMsRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.lockBorderFadeDurationMs === 'function') {
    return service.lockBorderFadeDurationMs(transformMsRaw);
  }
  var transformMs = target.dragSurfaceLockTransformTimeMs(transformMsRaw);
  return Math.max(80, Math.round(transformMs * 0.24));
}

function infringDragSurfaceVisualStateStore(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._dragSurfaceVisualStates || typeof target._dragSurfaceVisualStates !== 'object') {
    target._dragSurfaceVisualStates = {};
  }
  return target._dragSurfaceVisualStates;
}

function infringDragSurfaceLockVisualCssVars(page, surfaceKeyRaw, wallRaw, optionsRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.lockVisualCssVars === 'function') {
    return service.lockVisualCssVars(surfaceKeyRaw, wallRaw, optionsRaw, target.dragSurfaceVisualStateStore());
  }
  var key = String(surfaceKeyRaw || 'drag-surface').trim().toLowerCase(); if (!key) key = 'drag-surface';
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
  var transformMs = target.dragSurfaceLockTransformTimeMs(options.transformMs);
  var fadeMs = target.dragSurfaceLockBorderFadeDurationMs(transformMs);
  var delayMs = 0; var durationMs = 0;
  var store = target.dragSurfaceVisualStateStore();
  var prev = store[key] && typeof store[key] === 'object' ? store[key] : { initialized: false, wall: wall };
  var initialized = prev.initialized === true;
  var previousWall = target.dragSurfaceNormalizeWall(prev.wall); if (!initialized) previousWall = wall;
  var wallChanged = previousWall !== wall;
  if (wall && wallChanged) { delayMs = transformMs; durationMs = fadeMs; }
  store[key] = { initialized: true, wall: wall };
  var baseBorder = 'var(--drag-bar-border)';
  var borderTop = baseBorder; var borderRight = baseBorder; var borderBottom = baseBorder; var borderLeft = baseBorder;
  if (wall === 'left') borderLeft = 'transparent';
  else if (wall === 'right') borderRight = 'transparent';
  else if (wall === 'top') borderTop = 'transparent';
  else if (wall === 'bottom') borderBottom = 'transparent';
  var shellPaddingInline = Object.prototype.hasOwnProperty.call(options, 'shellPaddingInline') ? String(options.shellPaddingInline || '') : '';
  var shellPaddingInlineLocked = Object.prototype.hasOwnProperty.call(options, 'shellPaddingInlineLocked') ? String(options.shellPaddingInlineLocked || '') : '';
  var shellPaddingBlock = Object.prototype.hasOwnProperty.call(options, 'shellPaddingBlock') ? String(options.shellPaddingBlock || '') : '';
  var shellPaddingBlockLocked = Object.prototype.hasOwnProperty.call(options, 'shellPaddingBlockLocked') ? String(options.shellPaddingBlockLocked || '') : '';
  var shellAlignItems = Object.prototype.hasOwnProperty.call(options, 'shellAlignItems') ? String(options.shellAlignItems || '') : '';
  var shellAlignItemsLocked = shellAlignItems;
  if (wall === 'left' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsLeft')) shellAlignItemsLocked = String(options.shellAlignItemsLeft || shellAlignItemsLocked || '');
  else if (wall === 'right' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsRight')) shellAlignItemsLocked = String(options.shellAlignItemsRight || shellAlignItemsLocked || '');
  else if (wall === 'top' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsTop')) shellAlignItemsLocked = String(options.shellAlignItemsTop || shellAlignItemsLocked || '');
  else if (wall === 'bottom' && Object.prototype.hasOwnProperty.call(options, 'shellAlignItemsBottom')) shellAlignItemsLocked = String(options.shellAlignItemsBottom || shellAlignItemsLocked || '');
  var surfaceMarginInline = Object.prototype.hasOwnProperty.call(options, 'surfaceMarginInline') ? String(options.surfaceMarginInline || '') : '';
  var surfaceMarginInlineLocked = Object.prototype.hasOwnProperty.call(options, 'surfaceMarginInlineLocked') ? String(options.surfaceMarginInlineLocked || '') : '';
  var resolvedSurfaceMarginInline = wall ? (surfaceMarginInlineLocked || surfaceMarginInline) : surfaceMarginInline;
  var radius = target.dragSurfaceRadiusByWall(wall);
  var css = '';
  css += '--drag-bar-lock-wall:' + (wall || 'none') + ';';
  css += '--drag-bar-lock-state:' + (wall ? '1' : '0') + ';';
  css += '--drag-bar-transform-time:' + transformMs + 'ms;';
  css += '--drag-bar-radius-transition:' + transformMs + 'ms var(--ease-smooth);';
  css += '--drag-bar-radius-override:' + radius + ';';
  css += '--drag-bar-border-top-color:' + borderTop + ';';
  css += '--drag-bar-border-right-color:' + borderRight + ';';
  css += '--drag-bar-border-bottom-color:' + borderBottom + ';';
  css += '--drag-bar-border-left-color:' + borderLeft + ';';
  css += '--drag-bar-border-transition-duration:' + Math.max(0, Math.round(durationMs)) + 'ms;';
  css += '--drag-bar-border-transition-delay:' + Math.max(0, Math.round(delayMs)) + 'ms;';
  if (shellPaddingInline || shellPaddingInlineLocked) {
    css += '--drag-bar-shell-padding-inline:' + (wall ? (shellPaddingInlineLocked || shellPaddingInline || '0px') : (shellPaddingInline || '0px')) + ';';
  }
  if (shellPaddingBlock || shellPaddingBlockLocked) {
    css += '--drag-bar-shell-padding-block:' + (wall ? (shellPaddingBlockLocked || shellPaddingBlock || '0px') : (shellPaddingBlock || '0px')) + ';';
  }
  if (shellAlignItems || shellAlignItemsLocked) {
    css += '--drag-bar-shell-align-items:' + (wall ? (shellAlignItemsLocked || shellAlignItems || 'stretch') : (shellAlignItems || 'stretch')) + ';';
  }
  if (resolvedSurfaceMarginInline) {
    css += '--drag-bar-surface-margin-inline:' + resolvedSurfaceMarginInline + ';';
  }
  return css;
}

function infringDragSurfaceLockRadiusCssVars(page, wallRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  if (service && typeof service.lockRadiusCssVars === 'function') {
    return service.lockRadiusCssVars(wallRaw);
  }
  var wall = target.dragSurfaceNormalizeWall(wallRaw);
  if (!wall) return '';
  var radius = target.dragSurfaceRadiusByWall(wall);
  return '--drag-bar-radius-override:' + radius + ';';
}
