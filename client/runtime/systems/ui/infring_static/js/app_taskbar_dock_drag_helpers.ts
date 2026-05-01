function infringBindTaskbarDockPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof window === 'undefined') return;
  if (target._taskbarDockPointerMoveHandler || target._taskbarDockPointerUpHandler) return;
  target._taskbarDockPointerMoveHandler = function(ev) { target.handleTaskbarDockPointerMove(ev); };
  target._taskbarDockPointerUpHandler = function(ev) { target.endTaskbarDockPointerDrag(ev); };
  window.addEventListener('pointermove', target._taskbarDockPointerMoveHandler, true);
  window.addEventListener('pointerup', target._taskbarDockPointerUpHandler, true);
  window.addEventListener('pointercancel', target._taskbarDockPointerUpHandler, true);
  window.addEventListener('mousemove', target._taskbarDockPointerMoveHandler, true);
  window.addEventListener('mouseup', target._taskbarDockPointerUpHandler, true);
}

function infringUnbindTaskbarDockPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof window === 'undefined') return;
  if (target._taskbarDockPointerMoveHandler) {
    try { window.removeEventListener('pointermove', target._taskbarDockPointerMoveHandler, true); } catch(_) {}
    try { window.removeEventListener('mousemove', target._taskbarDockPointerMoveHandler, true); } catch(_) {}
  }
  if (target._taskbarDockPointerUpHandler) {
    try { window.removeEventListener('pointerup', target._taskbarDockPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('pointercancel', target._taskbarDockPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('mouseup', target._taskbarDockPointerUpHandler, true); } catch(_) {}
  }
  target._taskbarDockPointerMoveHandler = null;
  target._taskbarDockPointerUpHandler = null;
}

function infringStartTaskbarDockPointerDrag(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!ev || Number(ev.button) !== 0) return;
  if (String(target.taskbarDragGroup || '').trim()) return;
  var eventTarget = ev && ev.target ? ev.target : null;
  if (target.shouldIgnoreTaskbarDockDragTarget(eventTarget)) return;
  target._taskbarDockDraggingContainedBottomDock = target.bottomDockTaskbarContained()
    ? target.bottomDockWallLockNormalized()
    : '';
  target._taskbarDockPointerActive = true;
  target._taskbarDockPointerMoved = false;
  target._taskbarDockPointerStartX = Number(ev.clientX || 0);
  target._taskbarDockPointerStartY = Number(ev.clientY || 0);
  target._taskbarDockOriginY = target.taskbarAnchorForDockEdge(target.taskbarDockEdge);
  target.taskbarDockDragY = target._taskbarDockOriginY;
  target.bindTaskbarDockPointerListeners();
  try {
    if (ev.currentTarget && typeof ev.currentTarget.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
      ev.currentTarget.setPointerCapture(ev.pointerId);
    }
  } catch(_) {}
  if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringHandleTaskbarDockPointerMove(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._taskbarDockPointerActive) return;
  var x = Number(ev.clientX || 0);
  var y = Number(ev.clientY || 0);
  var movedX = Math.abs(x - Number(target._taskbarDockPointerStartX || 0));
  var movedY = Math.abs(y - Number(target._taskbarDockPointerStartY || 0));
  if (!target._taskbarDockPointerMoved) {
    if (movedX < 4 && movedY < 4) return;
    target._taskbarDockPointerMoved = true;
    target.taskbarDockDragActive = true;
  }
  var candidateY = Number(target._taskbarDockOriginY || 0) + (y - Number(target._taskbarDockPointerStartY || 0));
  target.taskbarDockDragY = target.taskbarClampDragY(candidateY);
  if (ev.cancelable && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringEndTaskbarDockPointerDrag(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (!target._taskbarDockPointerActive) return;
  target._taskbarDockPointerActive = false;
  target.unbindTaskbarDockPointerListeners();
  if (!target._taskbarDockPointerMoved) {
    target.taskbarDockDragActive = false;
    target._taskbarDockDraggingContainedBottomDock = '';
    return;
  }
  target._taskbarDockPointerMoved = false;
  target.taskbarDockEdge = target.taskbarNearestDockEdge(target.taskbarDockDragY);
  var carriedBottomDock = String(target._taskbarDockDraggingContainedBottomDock || '');
  if (carriedBottomDock) {
    target.bottomDockSetWallLock(target.taskbarDockEdge);
    target.taskbarDockDragY = target.taskbarAnchorForDockEdge(target.taskbarDockEdge);
    target.taskbarPersistDockEdge();
    window.requestAnimationFrame(function() {
      target._taskbarDockDraggingContainedBottomDock = '';
      target.taskbarDockDragActive = false;
    });
    return;
  }
  target._taskbarDockDraggingContainedBottomDock = '';
  target.taskbarDockDragActive = false;
  target.taskbarPersistDockEdge();
}
