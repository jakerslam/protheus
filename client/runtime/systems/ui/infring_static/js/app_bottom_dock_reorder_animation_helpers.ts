// Bottom-dock reorder animation helpers stay separate from app.ts to keep drag math isolated.
function infringBottomDockButtonRects() {
  var out = {};
  var root = document.querySelector('.bottom-dock');
  if (!root) return out;
  var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
  for (var i = 0; i < nodes.length; i++) {
    var node = nodes[i];
    if (!node) continue;
    var id = String(node.getAttribute('data-dock-id') || '').trim();
    if (!id) continue;
    var rect = node.getBoundingClientRect();
    var width = Number(rect.width || 0);
    var height = Number(rect.height || 0);
    var left = Number(rect.left || 0);
    var top = Number(rect.top || 0);
    out[id] = {
      left: left,
      top: top,
      width: width,
      height: height,
      cx: left + (width / 2),
      cy: top + (height / 2)
    };
  }
  return out;
}

function infringAnimateBottomDockFromRects(page, beforeRects) {
  if (!beforeRects || typeof beforeRects !== 'object') return;
  if (typeof requestAnimationFrame !== 'function') return;
  var durationMs = page.bottomDockMoveDurationMs();
  requestAnimationFrame(function() {
    var root = document.querySelector('.bottom-dock');
    if (!root) return;
    var rootScale = page.readBottomDockScale(root);
    if (!Number.isFinite(rootScale) || rootScale <= 0.01) rootScale = 1;
    var side = page.bottomDockActiveSide();
    var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
    for (var i = 0; i < nodes.length; i++) {
      var node = nodes[i];
      if (!node || node.classList.contains('dragging')) continue;
      var id = String(node.getAttribute('data-dock-id') || '').trim();
      if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
      var from = beforeRects[id] || {};
      var rect = node.getBoundingClientRect();
      var fromCx = Number(from.cx);
      var fromCy = Number(from.cy);
      if (!Number.isFinite(fromCx)) fromCx = Number(from.left || 0) + (Number(from.width || 0) / 2);
      if (!Number.isFinite(fromCy)) fromCy = Number(from.top || 0) + (Number(from.height || 0) / 2);
      var toCx = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
      var toCy = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
      var screenDx = Number(fromCx || 0) - Number(toCx || 0);
      var screenDy = Number(fromCy || 0) - Number(toCy || 0);
      if (Math.abs(screenDx) < 0.5 && Math.abs(screenDy) < 0.5) continue;
      var localDelta = page.bottomDockScreenDeltaToLocal(screenDx, screenDy, side);
      var tx = Number(localDelta.x || 0) / rootScale;
      var ty = Number(localDelta.y || 0) / rootScale;
      if (Math.abs(tx) < 0.25 && Math.abs(ty) < 0.25) continue;
      node.style.setProperty('--dock-reorder-transition', '0ms');
      node.style.setProperty('--dock-reorder-translate-x', Math.round(tx) + 'px');
      node.style.setProperty('--dock-reorder-translate-y', Math.round(ty) + 'px');
      void node.offsetHeight;
      node.style.setProperty('--dock-reorder-transition', Math.max(0, Math.round(durationMs)) + 'ms');
      node.style.setProperty('--dock-reorder-translate-x', '0px');
      node.style.setProperty('--dock-reorder-translate-y', '0px');
      (function(el) {
        window.setTimeout(function() {
          if (
            !el.classList.contains('dragging') &&
            !el.classList.contains('hovered') &&
            !el.classList.contains('neighbor-hover') &&
            !el.classList.contains('second-neighbor-hover')
          ) {
            el.style.removeProperty('--dock-reorder-translate-x');
            el.style.removeProperty('--dock-reorder-translate-y');
          }
          el.style.removeProperty('--dock-reorder-transition');
        }, durationMs + 30);
      })(node);
    }
  });
}
