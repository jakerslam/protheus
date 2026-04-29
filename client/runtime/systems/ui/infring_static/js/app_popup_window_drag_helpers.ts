// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringBindPopupWindowPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target._popupWindowPointerMoveHandler || target._popupWindowPointerUpHandler) return;
  target._popupWindowPointerMoveHandler = function(ev) { target.handlePopupWindowPointerMove(ev); };
  target._popupWindowPointerUpHandler = function() { target.endPopupWindowPointerDrag(); };
  window.addEventListener('pointermove', target._popupWindowPointerMoveHandler, true);
  window.addEventListener('pointerup', target._popupWindowPointerUpHandler, true);
  window.addEventListener('pointercancel', target._popupWindowPointerUpHandler, true);
  window.addEventListener('mousemove', target._popupWindowPointerMoveHandler, true);
  window.addEventListener('mouseup', target._popupWindowPointerUpHandler, true);
}

function infringUnbindPopupWindowPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target._popupWindowPointerMoveHandler) {
    try { window.removeEventListener('pointermove', target._popupWindowPointerMoveHandler, true); } catch(_) {}
    try { window.removeEventListener('mousemove', target._popupWindowPointerMoveHandler, true); } catch(_) {}
  }
  if (target._popupWindowPointerUpHandler) {
    try { window.removeEventListener('pointerup', target._popupWindowPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('pointercancel', target._popupWindowPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('mouseup', target._popupWindowPointerUpHandler, true); } catch(_) {}
  }
  target._popupWindowPointerMoveHandler = null;
  target._popupWindowPointerUpHandler = null;
}

function infringStartPopupWindowPointerDrag(page, kind, ev) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(kind || '').trim().toLowerCase();
  if (!ev || !key || !target.popupWindowOpenState(key)) return;
  var button = Number(ev.button);
  if (Number.isFinite(button) && button !== 0) return;
  var eventTarget = ev && ev.target ? ev.target : null;
  if (eventTarget && typeof eventTarget.closest === 'function') {
    if (eventTarget.closest('button, input, textarea, select, a, [contenteditable="true"]')) return;
  }
  target._popupWindowPointerActive = true;
  target._popupWindowPointerMoved = false;
  target.popupWindowDragKind = key;
  target._popupWindowPointerStartX = Number(ev.clientX || 0);
  target._popupWindowPointerStartY = Number(ev.clientY || 0);
  target._popupWindowPointerOriginLeft = target.popupWindowResolvedLeft(key);
  target._popupWindowPointerOriginTop = target.popupWindowResolvedTop(key);
  target._popupWindowPointerLastX = target._popupWindowPointerStartX;
  target._popupWindowPointerLastY = target._popupWindowPointerStartY;
  target._popupWindowPointerLastAt = Date.now();
  target._popupWindowPointerVelocity = 0;
  target.popupWindowDragLeft = target._popupWindowPointerOriginLeft;
  target.popupWindowDragTop = target._popupWindowPointerOriginTop;
  target.popupWindowDragWallLock = '';
  target.bindPopupWindowPointerListeners();
  try {
    if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
      ev.currentTarget.setPointerCapture(ev.pointerId);
    }
  } catch(_) {}
}

function infringHandlePopupWindowPointerMove(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._popupWindowPointerActive) return;
  var key = String(target.popupWindowDragKind || '').trim().toLowerCase();
  if (!key || !target.popupWindowOpenState(key)) return;
  var nextX = Number(ev.clientX || 0);
  var nextY = Number(ev.clientY || 0);
  var now = Date.now();
  var prevX = Number(target._popupWindowPointerLastX || nextX);
  var prevY = Number(target._popupWindowPointerLastY || nextY);
  var prevAt = Number(target._popupWindowPointerLastAt || now);
  var dt = Math.max(1, now - prevAt);
  var stepDx = nextX - prevX;
  var stepDy = nextY - prevY;
  target._popupWindowPointerVelocity = Math.sqrt((stepDx * stepDx) + (stepDy * stepDy)) / dt;
  target._popupWindowPointerLastX = nextX;
  target._popupWindowPointerLastY = nextY;
  target._popupWindowPointerLastAt = now;
  var movedX = Math.abs(nextX - Number(target._popupWindowPointerStartX || 0));
  var movedY = Math.abs(nextY - Number(target._popupWindowPointerStartY || 0));
  if (!target._popupWindowPointerMoved) {
    if (movedX < 4 && movedY < 4) return;
    target._popupWindowPointerMoved = true;
    target.popupWindowDragActive = true;
  }
  var dragDx = nextX - Number(target._popupWindowPointerStartX || 0);
  var dragDy = nextY - Number(target._popupWindowPointerStartY || 0);
  var candidateLeft = Number(target._popupWindowPointerOriginLeft || 0) + dragDx;
  var candidateTop = Number(target._popupWindowPointerOriginTop || 0) + dragDy;
  var hardBounds = target.popupWindowHardBounds(key);
  var clamped = target.dragSurfaceClampWithBounds(hardBounds, candidateLeft, candidateTop);
  target.popupWindowDragLeft = clamped.left;
  target.popupWindowDragTop = clamped.top;
  if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringEndPopupWindowPointerDrag(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._popupWindowPointerActive) return;
  var key = String(target.popupWindowDragKind || '').trim().toLowerCase();
  var moved = !!target._popupWindowPointerMoved;
  target._popupWindowPointerActive = false;
  target._popupWindowPointerMoved = false;
  target.unbindPopupWindowPointerListeners();
  if (key && moved) {
    var hardBounds = target.popupWindowHardBounds(key);
    var finalPlacement = target.dragSurfaceClampWithBounds(hardBounds, target.popupWindowDragLeft, target.popupWindowDragTop);
    target.popupWindowDragLeft = finalPlacement.left;
    target.popupWindowDragTop = finalPlacement.top;
    target.popupWindowPersistPlacement(key, target.popupWindowDragLeft, target.popupWindowDragTop);
  }
  target.popupWindowDragActive = false;
  target.popupWindowDragWallLock = '';
  target.popupWindowDragKind = '';
}
