// Canonical Shell helper source: dashboard layout measurement projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringDragSurfaceMoveDurationMs(page, rawValue, fallbackMs) {
  var target = page && typeof page === 'object' ? page : {};
  var service = typeof target.dragbarService === 'function' ? target.dragbarService() : null;
  if (service && typeof service.moveDurationMs === 'function') {
    return service.moveDurationMs(rawValue, fallbackMs);
  }
  var fallback = Number(fallbackMs || 280);
  if (!Number.isFinite(fallback)) fallback = 280;
  fallback = Math.max(80, Math.round(fallback));
  var raw = Number(rawValue);
  if (!Number.isFinite(raw)) raw = fallback;
  return Math.max(80, Math.round(raw));
}

function infringReadBottomDockScale(el) {
  if (!el || typeof window === 'undefined' || typeof window.getComputedStyle !== 'function') {
    return 0.95;
  }
  try {
    var transform = String(window.getComputedStyle(el).transform || '').trim();
    if (!transform || transform === 'none') return 0.95;
    var matrix2d = transform.match(/^matrix\(([^)]+)\)$/);
    if (matrix2d && matrix2d[1]) {
      var parts2d = matrix2d[1].split(',').map(function(v) { return Number(String(v || '').trim()); });
      if (parts2d.length >= 2 && Number.isFinite(parts2d[0]) && Number.isFinite(parts2d[1])) {
        var scale2d = Math.sqrt((parts2d[0] * parts2d[0]) + (parts2d[1] * parts2d[1]));
        if (Number.isFinite(scale2d) && scale2d > 0.01) return scale2d;
      }
    }
    var matrix3d = transform.match(/^matrix3d\(([^)]+)\)$/);
    if (matrix3d && matrix3d[1]) {
      var parts3d = matrix3d[1].split(',').map(function(v) { return Number(String(v || '').trim()); });
      if (parts3d.length >= 1 && Number.isFinite(parts3d[0]) && parts3d[0] > 0.01) return parts3d[0];
    }
  } catch(_) {}
  return 0.95;
}

function infringComputeScrollHintState(el) {
  if (!el) return { above: false, below: false };
  var scrollHeight = Number(el.scrollHeight || 0);
  var clientHeight = Number(el.clientHeight || 0);
  var scrollTop = Math.max(0, Number(el.scrollTop || 0));
  var maxScroll = Math.max(0, scrollHeight - clientHeight);
  if (maxScroll <= 2) return { above: false, below: false };
  return {
    above: scrollTop > 2,
    below: (maxScroll - scrollTop) > 2
  };
}
