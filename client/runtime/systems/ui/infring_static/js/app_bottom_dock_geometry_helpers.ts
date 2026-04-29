// Canonical Shell helper source: bottom-dock geometry and orientation projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringBottomDockMoveDurationMs(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof target.dragSurfaceMoveDurationMs === 'function') {
    return target.dragSurfaceMoveDurationMs(target._bottomDockMoveDurationMs, 360);
  }
  return infringDragSurfaceMoveDurationMs(target, target._bottomDockMoveDurationMs, 360);
}

function infringBottomDockExpandedScale(page) {
  var target = page && typeof page === 'object' ? page : {};
  var raw = Number(target._bottomDockExpandedScale || 1.54);
  if (!Number.isFinite(raw) || raw <= 1) raw = 1.54;
  return raw;
}

function infringBottomDockReadViewportSize() {
  var width = 0;
  var height = 0;
  try {
    width = Number(window && window.innerWidth || 0);
    height = Number(window && window.innerHeight || 0);
  } catch(_) {
    width = 0;
    height = 0;
  }
  if (!Number.isFinite(width) || width <= 0) {
    width = Number(document && document.documentElement && document.documentElement.clientWidth || 1440);
  }
  if (!Number.isFinite(height) || height <= 0) {
    height = Number(document && document.documentElement && document.documentElement.clientHeight || 900);
  }
  if (!Number.isFinite(width) || width <= 0) width = 1440;
  if (!Number.isFinite(height) || height <= 0) height = 900;
  return { width: width, height: height };
}

function infringBottomDockReadBaseSize() {
  var width = 0;
  var height = 0;
  try {
    var node = document && typeof document.querySelector === 'function'
      ? document.querySelector('.bottom-dock')
      : null;
    if (node) {
      width = Number(node.offsetWidth || 0);
      height = Number(node.offsetHeight || 0);
    }
  } catch(_) {
    width = 0;
    height = 0;
  }
  if (!Number.isFinite(width) || width <= 0) width = 420;
  if (!Number.isFinite(height) || height <= 0) height = 54;
  return { width: width, height: height };
}

function infringBottomDockNormalizeSide(side) {
  var key = String(side || '').trim().toLowerCase();
  if (key === 'top' || key === 'left' || key === 'right') return key;
  return 'bottom';
}

function infringBottomDockIsVerticalSide(side) {
  var key = infringBottomDockNormalizeSide(side);
  return key === 'left' || key === 'right';
}

function infringBottomDockRotationDegForSide(side) {
  var key = infringBottomDockNormalizeSide(side);
  if (key === 'left') return -90;
  if (key === 'right') return 90;
  return 0;
}

function infringBottomDockIconRotationDegForSide(side) {
  var key = infringBottomDockNormalizeSide(side);
  if (key === 'left') return 90;
  if (key === 'right') return -90;
  return 0;
}

function infringBottomDockUpDegForSide(side) {
  var key = infringBottomDockNormalizeSide(side);
  if (key === 'left' || key === 'right' || key === 'top' || key === 'bottom') return 0;
  return 0;
}

function infringBottomDockOrientation(page, sideHint) {
  var target = page && typeof page === 'object' ? page : {};
  var activeSide = typeof target.bottomDockActiveSide === 'function' ? target.bottomDockActiveSide() : 'bottom';
  var side = infringBottomDockNormalizeSide(sideHint || activeSide);
  var horizontal = !infringBottomDockIsVerticalSide(side);
  var axis = horizontal ? 'x' : 'y';
  return {
    side: side,
    horizontal: horizontal,
    axis: axis,
    primarySign: 1,
    upDeg: Number(infringBottomDockUpDegForSide(side) || 0)
  };
}

function infringBottomDockOppositeSide(sideHint) {
  var side = infringBottomDockNormalizeSide(sideHint);
  if (side === 'left') return 'right';
  if (side === 'right') return 'left';
  if (side === 'top') return 'bottom';
  return 'top';
}

function infringBottomDockWallSide(page) {
  var target = page && typeof page === 'object' ? page : {};
  var activeSide = typeof target.bottomDockActiveSide === 'function' ? target.bottomDockActiveSide() : 'bottom';
  return infringBottomDockNormalizeSide(activeSide);
}

function infringBottomDockOpenSide(page) {
  return infringBottomDockOppositeSide(infringBottomDockWallSide(page));
}

function infringBottomDockRotationDegResolved(page, sideHint) {
  var target = page && typeof page === 'object' ? page : {};
  var activeSide = typeof target.bottomDockActiveSide === 'function' ? target.bottomDockActiveSide() : 'bottom';
  var side = infringBottomDockNormalizeSide(sideHint || activeSide);
  var rotationDeg = Number(target.bottomDockRotationDeg);
  if (!Number.isFinite(rotationDeg)) {
    rotationDeg = Number(infringBottomDockRotationDegForSide(side));
  }
  return Number(infringBottomDockNormalizeRotationDeg(rotationDeg) || 0);
}

function infringBottomDockScreenDeltaToLocal(page, dx, dy, sideHint) {
  var screenDx = Number(dx || 0);
  var screenDy = Number(dy || 0);
  var rotationDeg = infringBottomDockRotationDegResolved(page, sideHint);
  var theta = (rotationDeg * Math.PI) / 180;
  var cos = Math.cos(theta);
  var sin = Math.sin(theta);
  return {
    x: (screenDx * cos) + (screenDy * sin),
    y: (-screenDx * sin) + (screenDy * cos)
  };
}

function infringBottomDockCanonicalRotationCandidatesForSide(side) {
  var key = infringBottomDockNormalizeSide(side);
  if (key === 'left' || key === 'right') return [90, -90];
  return [0];
}

function infringBottomDockNormalizeRotationDeg(value) {
  var raw = Number(value);
  var canonical = [-90, 0, 90];
  if (!Number.isFinite(raw)) return 0;
  var best = canonical[0];
  var bestDist = Number.POSITIVE_INFINITY;
  for (var i = 0; i < canonical.length; i += 1) {
    var candidate = canonical[i];
    var dist = Math.abs(raw - candidate);
    if (dist < bestDist) {
      bestDist = dist;
      best = candidate;
    }
  }
  return best;
}

function infringBottomDockResolveShortestRotationDeg(currentDeg, targetDeg) {
  var current = Number(currentDeg);
  var target = Number(targetDeg);
  if (!Number.isFinite(target)) target = 0;
  if (!Number.isFinite(current)) return target;
  var best = target;
  var bestDelta = Number.POSITIVE_INFINITY;
  for (var k = -2; k <= 2; k += 1) {
    var candidate = target + (k * 360);
    var delta = Math.abs(candidate - current);
    if (delta < bestDelta) {
      bestDelta = delta;
      best = candidate;
    }
  }
  return best;
}

function infringBottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY) {
  var view = infringBottomDockReadViewportSize();
  var x = Number(anchorX);
  var y = Number(anchorY);
  if (!Number.isFinite(x)) x = Number(view.width || 0) * 0.5;
  if (!Number.isFinite(y)) y = Number(view.height || 0) * 0.5;
  var left = x < (Number(view.width || 0) * 0.5);
  var top = y < (Number(view.height || 0) * 0.5);
  // TL + BR => counterclockwise. TR + BL => clockwise.
  return (left === top) ? 'ccw' : 'cw';
}

function infringBottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction) {
  var current = Number(currentDeg);
  var target = Number(targetDeg);
  var dir = String(direction || '').trim().toLowerCase();
  if (!Number.isFinite(target)) target = 0;
  if (!Number.isFinite(current)) return target;
  if (dir !== 'cw' && dir !== 'ccw') {
    return infringBottomDockResolveShortestRotationDeg(current, target);
  }
  var best = null;
  var bestAbs = Number.POSITIVE_INFINITY;
  for (var k = -2; k <= 2; k += 1) {
    var candidate = target + (k * 360);
    var delta = candidate - current;
    if (dir === 'cw' && delta < 0) continue;
    if (dir === 'ccw' && delta > 0) continue;
    var absDelta = Math.abs(delta);
    if (absDelta < bestAbs) {
      bestAbs = absDelta;
      best = candidate;
    }
  }
  if (best === null) {
    return infringBottomDockResolveShortestRotationDeg(current, target);
  }
  return best;
}

function infringBottomDockResolveRotationForSide(page, side, anchorX, anchorY) {
  var target = page && typeof page === 'object' ? page : {};
  var current = infringBottomDockNormalizeRotationDeg(target.bottomDockRotationDeg);
  var dir = infringBottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY);
  var candidates = infringBottomDockCanonicalRotationCandidatesForSide(side);
  if (!Array.isArray(candidates) || !candidates.length) return current;
  var best = Number(candidates[0] || 0);
  var bestScore = Number.POSITIVE_INFINITY;
  var bestDeltaAbs = Number.POSITIVE_INFINITY;
  for (var i = 0; i < candidates.length; i += 1) {
    var nextTarget = Number(candidates[i] || 0);
    var delta = nextTarget - current;
    var deltaAbs = Math.abs(delta);
    var directionPenalty = 0;
    if (dir === 'cw' && delta < 0) directionPenalty = 0.35;
    if (dir === 'ccw' && delta > 0) directionPenalty = 0.35;
    var score = deltaAbs + directionPenalty;
    if (score < bestScore || (score === bestScore && deltaAbs < bestDeltaAbs)) {
      best = nextTarget;
      bestScore = score;
      bestDeltaAbs = deltaAbs;
    }
  }
  var chosenDelta = best - current;
  if (Math.abs(chosenDelta) > 90) {
    if (dir === 'cw') return current + 90;
    if (dir === 'ccw') return current - 90;
    return current + (chosenDelta > 0 ? 90 : -90);
  }
  return best;
}
