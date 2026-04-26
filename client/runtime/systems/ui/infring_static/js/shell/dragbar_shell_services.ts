'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};
  var walls = ['left', 'right', 'top', 'bottom'];

  function numericOr(value, fallback) {
    var numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : fallback;
  }

  function normalizeWall(wallRaw) {
    var wall = String(wallRaw || '').trim().toLowerCase();
    return walls.indexOf(wall) >= 0 ? wall : '';
  }

  function normalizeBounds(bounds) {
    var box = bounds && typeof bounds === 'object' ? bounds : {};
    var minLeft = numericOr(box.minLeft, 0);
    var maxLeft = numericOr(box.maxLeft, minLeft);
    var minTop = numericOr(box.minTop, 0);
    var maxTop = numericOr(box.maxTop, minTop);
    if (maxLeft < minLeft) maxLeft = minLeft;
    if (maxTop < minTop) maxTop = minTop;
    return { minLeft: minLeft, maxLeft: maxLeft, minTop: minTop, maxTop: maxTop };
  }

  function moveDurationMs(rawValue, fallbackMs) {
    var fallback = numericOr(fallbackMs, 280);
    fallback = Math.max(80, Math.round(fallback));
    var raw = numericOr(rawValue, fallback);
    return Math.max(80, Math.round(raw));
  }

  function hardBounds(input) {
    var source = input && typeof input === 'object' ? input : {};
    var width = Math.max(1, numericOr(source.width, 1));
    var height = Math.max(1, numericOr(source.height, 1));
    var viewportWidth = Math.max(1, numericOr(source.viewportWidth, width));
    var viewportHeight = Math.max(1, numericOr(source.viewportHeight, height));
    var minTop = numericOr(source.minTop, 0);
    var maxBottom = numericOr(source.maxBottom, viewportHeight);
    if (maxBottom < minTop) maxBottom = minTop;
    return {
      minLeft: 0,
      maxLeft: Math.max(0, viewportWidth - width),
      minTop: minTop,
      maxTop: Math.max(minTop, maxBottom - height)
    };
  }

  function softBounds(input) {
    var source = input && typeof input === 'object' ? input : {};
    var width = Math.max(1, numericOr(source.width, 1));
    var height = Math.max(1, numericOr(source.height, 1));
    var wallGap = Math.max(0, numericOr(source.wallGap, 0));
    var viewportWidth = Math.max(1, numericOr(source.viewportWidth, width));
    var minTop = numericOr(source.minTop, wallGap);
    var maxBottom = numericOr(source.maxBottom, minTop + height);
    var minLeft = wallGap;
    var maxLeft = Math.max(minLeft, viewportWidth - wallGap - width);
    var maxTop = Math.max(minTop, maxBottom - height);
    return { minLeft: minLeft, maxLeft: maxLeft, minTop: minTop, maxTop: maxTop };
  }

  function clampWithBounds(bounds, leftRaw, topRaw) {
    var box = normalizeBounds(bounds);
    var left = numericOr(leftRaw, box.minLeft);
    var top = numericOr(topRaw, box.minTop);
    return {
      left: Math.max(box.minLeft, Math.min(box.maxLeft, left)),
      top: Math.max(box.minTop, Math.min(box.maxTop, top))
    };
  }

  function nearestWall(bounds, leftRaw, topRaw) {
    var box = normalizeBounds(bounds);
    var clamped = clampWithBounds(box, leftRaw, topRaw);
    var distances = {
      left: Math.max(0, clamped.left - box.minLeft),
      right: Math.max(0, box.maxLeft - clamped.left),
      top: Math.max(0, clamped.top - box.minTop),
      bottom: Math.max(0, box.maxTop - clamped.top)
    };
    var winner = 'left';
    var distance = distances.left;
    for (var i = 1; i < walls.length; i += 1) {
      var wall = walls[i];
      var next = distances[wall];
      if (next < distance) {
        winner = wall;
        distance = next;
      }
    }
    return { wall: winner, distance: Math.max(0, distance), distances: distances, left: clamped.left, top: clamped.top };
  }

  function applyWallLock(bounds, leftRaw, topRaw, wallRaw) {
    var box = normalizeBounds(bounds);
    var wall = normalizeWall(wallRaw);
    var clamped = clampWithBounds(box, leftRaw, topRaw);
    if (!wall) return { left: clamped.left, top: clamped.top, wall: '' };
    if (wall === 'left') clamped.left = box.minLeft;
    else if (wall === 'right') clamped.left = box.maxLeft;
    else if (wall === 'top') clamped.top = box.minTop;
    else if (wall === 'bottom') clamped.top = box.maxTop;
    return { left: clamped.left, top: clamped.top, wall: wall };
  }

  function distanceFromWall(bounds, leftRaw, topRaw, wallRaw) {
    var box = normalizeBounds(bounds);
    var wall = normalizeWall(wallRaw);
    if (!wall) return Number.POSITIVE_INFINITY;
    var clamped = clampWithBounds(box, leftRaw, topRaw);
    if (wall === 'left') return Math.max(0, clamped.left - box.minLeft);
    if (wall === 'right') return Math.max(0, box.maxLeft - clamped.left);
    if (wall === 'top') return Math.max(0, clamped.top - box.minTop);
    return Math.max(0, box.maxTop - clamped.top);
  }

  function wallLockOvershoot(bounds, leftRaw, topRaw, wallRaw) {
    var box = normalizeBounds(bounds);
    var wall = normalizeWall(wallRaw);
    if (!wall) return 0;
    var left = numericOr(leftRaw, box.minLeft);
    var top = numericOr(topRaw, box.minTop);
    if (wall === 'left') return Math.max(0, box.minLeft - left);
    if (wall === 'right') return Math.max(0, left - box.maxLeft);
    if (wall === 'top') return Math.max(0, box.minTop - top);
    return Math.max(0, top - box.maxTop);
  }

  function centeredPoint(bounds) {
    var box = normalizeBounds(bounds);
    return {
      left: box.minLeft + ((box.maxLeft - box.minLeft) * 0.5),
      top: box.minTop + ((box.maxTop - box.minTop) * 0.5)
    };
  }

  function wallLockThresholds(wallGapRaw) {
    var wallGap = Math.max(0, numericOr(wallGapRaw, 0));
    return {
      contact: Math.max(2, Math.round(wallGap * 0.12)),
      distance: Math.max(8, Math.round(wallGap * 0.7)),
      unlock: Math.max(42, Math.round(wallGap * 2.6)),
      overshoot: Math.max(5, Math.round(wallGap * 0.34))
    };
  }

  function resolveWallLock(bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw, optionsRaw) {
    var box = normalizeBounds(bounds);
    var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
    var thresholds = wallLockThresholds(options.wallGap);
    var overshootWall = '';
    var overshootValue = 0;
    for (var i = 0; i < walls.length; i += 1) {
      var wall = walls[i];
      var overshoot = wallLockOvershoot(box, candidateLeft, candidateTop, wall);
      if (overshoot >= thresholds.overshoot && overshoot > overshootValue) {
        overshootValue = overshoot;
        overshootWall = wall;
      }
    }
    if (overshootWall) return overshootWall;
    var clamped = clampWithBounds(box, candidateLeft, candidateTop);
    var touchedWalls = [];
    if (Math.abs(clamped.left - box.minLeft) <= thresholds.contact) touchedWalls.push('left');
    if (Math.abs(box.maxLeft - clamped.left) <= thresholds.contact) touchedWalls.push('right');
    if (Math.abs(clamped.top - box.minTop) <= thresholds.contact) touchedWalls.push('top');
    if (Math.abs(box.maxTop - clamped.top) <= thresholds.contact) touchedWalls.push('bottom');
    if (touchedWalls.length === 1) return touchedWalls[0];
    if (touchedWalls.length > 1) {
      var motionDx = numericOr(motionDxRaw, 0);
      var motionDy = numericOr(motionDyRaw, 0);
      var absDx = Math.abs(motionDx);
      var absDy = Math.abs(motionDy);
      if (absDx > absDy + 0.25) {
        if (motionDx >= 0 && touchedWalls.indexOf('right') >= 0) return 'right';
        if (motionDx < 0 && touchedWalls.indexOf('left') >= 0) return 'left';
      } else if (absDy > absDx + 0.25) {
        if (motionDy >= 0 && touchedWalls.indexOf('bottom') >= 0) return 'bottom';
        if (motionDy < 0 && touchedWalls.indexOf('top') >= 0) return 'top';
      }
      var nearestTouchedWall = nearest && typeof nearest.wall === 'string' ? normalizeWall(nearest.wall) : '';
      if (nearestTouchedWall && touchedWalls.indexOf(nearestTouchedWall) >= 0) return nearestTouchedWall;
      return touchedWalls[0];
    }
    var edgeDistance = nearest && Number.isFinite(Number(nearest.distance)) ? Number(nearest.distance) : Number.POSITIVE_INFINITY;
    if (!Number.isFinite(edgeDistance) || edgeDistance > thresholds.distance) return '';
    return normalizeWall(nearest && nearest.wall ? nearest.wall : '');
  }

  function radiusByWall(wallRaw) {
    var r = 'var(--overlay-shared-surface-radius, var(--overlay-surface-radius, 18px))';
    var wall = normalizeWall(wallRaw);
    if (wall === 'left') return '0 ' + r + ' ' + r + ' 0';
    if (wall === 'right') return r + ' 0 0 ' + r;
    if (wall === 'top') return '0 0 ' + r + ' ' + r;
    if (wall === 'bottom') return r + ' ' + r + ' 0 0';
    return r;
  }

  function lockTransformTimeMs(rawValue, fallbackMs) {
    var fallback = numericOr(fallbackMs, 500);
    var raw = numericOr(rawValue, fallback);
    return Math.max(120, Math.round(raw));
  }

  function lockBorderFadeDurationMs(transformMsRaw) {
    return Math.max(80, Math.round(lockTransformTimeMs(transformMsRaw, 500) * 0.24));
  }

  function lockVisualCssVars(surfaceKeyRaw, wallRaw, optionsRaw, visualStoreRaw) {
    var key = String(surfaceKeyRaw || 'drag-surface').trim().toLowerCase() || 'drag-surface';
    var wall = normalizeWall(wallRaw);
    var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
    var transformMs = lockTransformTimeMs(options.transformMs, 500);
    var fadeMs = lockBorderFadeDurationMs(transformMs);
    var store = visualStoreRaw && typeof visualStoreRaw === 'object' ? visualStoreRaw : {};
    var prev = store[key] && typeof store[key] === 'object' ? store[key] : { initialized: false, wall: wall };
    var initialized = prev.initialized === true;
    var previousWall = initialized ? normalizeWall(prev.wall) : wall;
    var wallChanged = previousWall !== wall;
    var delayMs = wall && wallChanged ? transformMs : 0;
    var durationMs = wall && wallChanged ? fadeMs : 0;
    store[key] = { initialized: true, wall: wall };
    var baseBorder = 'var(--drag-bar-border)';
    var borderTop = wall === 'top' ? 'transparent' : baseBorder;
    var borderRight = wall === 'right' ? 'transparent' : baseBorder;
    var borderBottom = wall === 'bottom' ? 'transparent' : baseBorder;
    var borderLeft = wall === 'left' ? 'transparent' : baseBorder;
    var css = '';
    css += '--drag-bar-lock-wall:' + (wall || 'none') + ';';
    css += '--drag-bar-lock-state:' + (wall ? '1' : '0') + ';';
    css += '--drag-bar-transform-time:' + transformMs + 'ms;';
    css += '--drag-bar-radius-transition:' + transformMs + 'ms var(--ease-smooth);';
    css += '--drag-bar-radius-override:' + radiusByWall(wall) + ';';
    css += '--drag-bar-border-top-color:' + borderTop + ';';
    css += '--drag-bar-border-right-color:' + borderRight + ';';
    css += '--drag-bar-border-bottom-color:' + borderBottom + ';';
    css += '--drag-bar-border-left-color:' + borderLeft + ';';
    css += '--drag-bar-border-transition-duration:' + Math.max(0, Math.round(durationMs)) + 'ms;';
    css += '--drag-bar-border-transition-delay:' + Math.max(0, Math.round(delayMs)) + 'ms;';
    return css + optionalDragbarCss(options, wall);
  }

  function optionalDragbarCss(options, wall) {
    var css = '';
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
    if (shellPaddingInline || shellPaddingInlineLocked) css += '--drag-bar-shell-padding-inline:' + (wall ? (shellPaddingInlineLocked || shellPaddingInline || '0px') : (shellPaddingInline || '0px')) + ';';
    if (shellPaddingBlock || shellPaddingBlockLocked) css += '--drag-bar-shell-padding-block:' + (wall ? (shellPaddingBlockLocked || shellPaddingBlock || '0px') : (shellPaddingBlock || '0px')) + ';';
    if (shellAlignItems || shellAlignItemsLocked) css += '--drag-bar-shell-align-items:' + (wall ? (shellAlignItemsLocked || shellAlignItems || 'stretch') : (shellAlignItems || 'stretch')) + ';';
    if (surfaceMarginInline || surfaceMarginInlineLocked) css += '--drag-bar-surface-margin-inline:' + (wall ? (surfaceMarginInlineLocked || surfaceMarginInline) : surfaceMarginInline) + ';';
    return css;
  }

  function pulltabStyle(optionsRaw) {
    var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
    if (options.active === false) return '';
    var wall = normalizeWall(options.wall);
    var dockRight = wall === 'right';
    var durationMs = options.dragging ? 0 : moveDurationMs(options.durationMs, options.fallbackMs || 280);
    var transitionVar = String(options.transitionVar || '--sidebar-position-transition');
    return [
      'position:absolute;',
      'left:' + (dockRight ? 'auto' : '100%') + ';',
      'right:' + (dockRight ? '100%' : 'auto') + ';',
      'top:50%;',
      'transform:translateY(-50%);',
      transitionVar + ':' + Math.max(0, Math.round(durationMs)) + 'ms;'
    ].join('');
  }

  function shouldIgnoreTarget(target, optionsRaw) {
    var options = optionsRaw && typeof optionsRaw === 'object' ? optionsRaw : {};
    var selector = String(options.ignoreSelector || '').trim();
    var node = target;
    if (node && typeof node.closest !== 'function' && node.parentElement) node = node.parentElement;
    if (!selector || !node || typeof node.closest !== 'function') return false;
    return Boolean(node.closest(selector));
  }

  services.dragbar = Object.assign({}, services.dragbar || {}, {
    normalizeWall: normalizeWall,
    moveDurationMs: moveDurationMs,
    hardBounds: hardBounds,
    softBounds: softBounds,
    clampWithBounds: clampWithBounds,
    nearestWall: nearestWall,
    applyWallLock: applyWallLock,
    distanceFromWall: distanceFromWall,
    wallLockOvershoot: wallLockOvershoot,
    centeredPoint: centeredPoint,
    wallLockThresholds: wallLockThresholds,
    resolveWallLock: resolveWallLock,
    radiusByWall: radiusByWall,
    lockTransformTimeMs: lockTransformTimeMs,
    lockBorderFadeDurationMs: lockBorderFadeDurationMs,
    lockVisualCssVars: lockVisualCssVars,
    lockRadiusCssVars: function(wallRaw) {
      var wall = normalizeWall(wallRaw);
      return wall ? '--drag-bar-radius-override:' + radiusByWall(wall) + ';' : '';
    },
    pulltabStyle: pulltabStyle,
    shouldIgnoreTarget: shouldIgnoreTarget
  });

  return services;
})(typeof InfringSharedShellServices === 'object' ? InfringSharedShellServices : null);

if (typeof window !== 'undefined') {
  window.InfringSharedShellServices = InfringSharedShellServices;
}
