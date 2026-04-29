// Bottom-dock axis/projection helpers live outside app.ts so layout math stays reviewable.
function infringBottomDockAxisBasis(page, sideHint) {
  var rotationDeg = page.bottomDockRotationDegResolved(sideHint);
  var theta = (Number(rotationDeg || 0) * Math.PI) / 180;
  var ux = Math.cos(theta);
  var uy = Math.sin(theta);
  if (Math.abs(ux) < 0.0001) ux = 0;
  if (Math.abs(uy) < 0.0001) uy = 0;
  return { ux: ux, uy: uy, vx: -uy, vy: ux };
}

function infringBottomDockProjectPointToAxis(page, x, y, basis) {
  var axis = basis && typeof basis === 'object'
    ? basis
    : page.bottomDockAxisBasis();
  var ux = Number(axis.ux || 0);
  var uy = Number(axis.uy || 0);
  var vx = Number(axis.vx || (-uy));
  var vy = Number(axis.vy || ux);
  var px = Number(x || 0);
  var py = Number(y || 0);
  return {
    primary: (px * ux) + (py * uy),
    secondary: (px * vx) + (py * vy)
  };
}

function infringBottomDockAxisHalfExtent(page, width, height, basis) {
  var axis = basis && typeof basis === 'object'
    ? basis
    : page.bottomDockAxisBasis();
  var w = Number(width || 0);
  var h = Number(height || 0);
  if (!Number.isFinite(w) || w < 0) w = 0;
  if (!Number.isFinite(h) || h < 0) h = 0;
  var ux = Math.abs(Number(axis.ux || 0));
  var uy = Math.abs(Number(axis.uy || 0));
  var vx = Math.abs(Number(axis.vx || 0));
  var vy = Math.abs(Number(axis.vy || 0));
  return {
    primary: ((ux * w) + (uy * h)) / 2,
    secondary: ((vx * w) + (vy * h)) / 2
  };
}

function infringBottomDockProjectedRectBounds(page, rect, basis) {
  if (!rect) return null;
  var axis = basis && typeof basis === 'object'
    ? basis
    : page.bottomDockAxisBasis();
  var left = Number(rect.left || 0);
  var top = Number(rect.top || 0);
  var right = Number(rect.right || left);
  var bottom = Number(rect.bottom || top);
  var p1 = page.bottomDockProjectPointToAxis(left, top, axis);
  var p2 = page.bottomDockProjectPointToAxis(right, top, axis);
  var p3 = page.bottomDockProjectPointToAxis(left, bottom, axis);
  var p4 = page.bottomDockProjectPointToAxis(right, bottom, axis);
  var primaryMin = Math.min(p1.primary, p2.primary, p3.primary, p4.primary);
  var primaryMax = Math.max(p1.primary, p2.primary, p3.primary, p4.primary);
  var secondaryMin = Math.min(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
  var secondaryMax = Math.max(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
  return {
    primaryMin: primaryMin,
    primaryMax: primaryMax,
    secondaryMin: secondaryMin,
    secondaryMax: secondaryMax
  };
}
