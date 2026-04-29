// Legacy bottom-dock drop/reorder helpers complement the boundary helpers.
function infringBottomDockInsertionIndexFromCoords(page, dragId, clientXRaw, clientYRaw) {
  var key = String(dragId || '').trim();
  if (!key || typeof document === 'undefined') return null;
  var clientX = Number(clientXRaw || 0);
  var clientY = Number(clientYRaw || 0);
  if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return null;
  var dock = null;
  try {
    dock = document.querySelector('.bottom-dock');
  } catch(_) {
    dock = null;
  }
  if (!dock || typeof dock.getBoundingClientRect !== 'function') return null;
  var dockRect = dock.getBoundingClientRect();
  var basis = page.bottomDockAxisBasis();
  var pointerProj = page.bottomDockProjectPointToAxis(clientX, clientY, basis);
  var dockBounds = page.bottomDockProjectedRectBounds(dockRect, basis);
  if (!dockBounds) return null;
  if (
    pointerProj.secondary < (Number(dockBounds.secondaryMin || 0) - 24) ||
    pointerProj.secondary > (Number(dockBounds.secondaryMax || 0) + 24)
  ) return null;
  var centers = page.captureBottomDockDragBoundaries(key);
  if (centers.length === 0) return null;
  var insertionIndex = 0;
  for (var c = 0; c < centers.length; c += 1) {
    if (pointerProj.primary >= centers[c]) insertionIndex += 1;
  }
  insertionIndex = Math.max(0, Math.min(centers.length, insertionIndex));
  return insertionIndex;
}

function infringBottomDockGhostCenterPoint(page) {
  var x = Number(page._bottomDockGhostTargetX || page._bottomDockGhostCurrentX || 0);
  var y = Number(page._bottomDockGhostTargetY || page._bottomDockGhostCurrentY || 0);
  var width = Number(page._bottomDockDragGhostWidth || 0);
  var height = Number(page._bottomDockDragGhostHeight || 0);
  if (!Number.isFinite(width) || width <= 0) width = 32;
  if (!Number.isFinite(height) || height <= 0) height = 32;
  return {
    x: x + (width / 2),
    y: y + (height / 2)
  };
}

function infringBottomDockInsertionIndexFromPointer(page, dragId, ev) {
  var key = String(dragId || '').trim();
  if (!key || !ev) return null;
  var center = page.bottomDockGhostCenterPoint();
  return page.bottomDockInsertionIndexFromCoords(key, center.x, center.y);
}

function infringApplyBottomDockReorderByIndex(page, dragId, insertionIndex, animate) {
  var key = String(dragId || '').trim();
  if (!key) return false;
  var current = page.normalizeBottomDockOrder(page.bottomDockOrder);
  var fromIndex = current.indexOf(key);
  if (fromIndex < 0) return false;
  var next = current.slice();
  next.splice(fromIndex, 1);
  var idx = Number(insertionIndex);
  if (!Number.isFinite(idx)) return false;
  idx = Math.max(0, Math.min(next.length, Math.round(idx)));
  next.splice(idx, 0, key);
  if (JSON.stringify(next) === JSON.stringify(current)) return false;
  var doAnimate = Boolean(animate);
  var beforeRects = doAnimate ? page.bottomDockButtonRects() : null;
  page.bottomDockOrder = next;
  if (doAnimate && beforeRects) page.animateBottomDockFromRects(beforeRects);
  return true;
}

function infringPersistBottomDockOrderIfChangedFromDragStart(page) {
  var current = page.normalizeBottomDockOrder(page.bottomDockOrder);
  var start = page.normalizeBottomDockOrder(page.bottomDockDragStartOrder);
  if (JSON.stringify(current) !== JSON.stringify(start)) {
    page.bottomDockOrder = current;
    page.persistBottomDockOrder();
    page.bottomDockDragCommitted = true;
  }
}

function infringCompleteBottomDockDropCleanup(page, ev) {
  page.bottomDockDragId = '';
  page.bottomDockDragStartOrder = [];
  page._bottomDockSuppressClickUntil = Date.now() + 220;
  page.cleanupBottomDockDragGhost();
  page.reviveBottomDockHoverFromPoint(
    Number(ev && ev.clientX || 0),
    Number(ev && ev.clientY || 0)
  );
  if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
}

function infringHandleBottomDockContainerDragOver(page, ev) {
  if (ev && ev.dataTransfer) {
    try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
  }
  var dragId = String(page.bottomDockDragId || '').trim();
  if (!dragId) return;
  var targetId = '';
  var targetEl = null;
  try {
    targetEl = ev && ev.target && typeof ev.target.closest === 'function'
      ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
      : null;
    targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
  } catch(_) {}
  if (targetId && targetId !== dragId) {
    page._bottomDockLastInsertionIndex = -1;
    var preferAfter = page.bottomDockShouldInsertAfter(targetId, ev, targetEl);
    page.handleBottomDockDragOver(targetId, ev, preferAfter);
    return;
  }
  if (!page.bottomDockShouldAppendFromPointer(dragId, ev)) return;
  var appendTargetId = page.bottomDockAppendTargetId(dragId);
  if (!appendTargetId) return;
  page._bottomDockLastInsertionIndex = -1;
  page.handleBottomDockDragOver(appendTargetId, ev, true);
}

function infringHandleBottomDockContainerDrop(page, ev) {
  var dragId = String(page.bottomDockDragId || '').trim();
  if (!dragId) return;
  var targetId = '';
  var targetEl = null;
  try {
    targetEl = ev && ev.target && typeof ev.target.closest === 'function'
      ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
      : null;
    targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
  } catch(_) {}
  if (targetId) {
    var preferAfter = page.bottomDockShouldInsertAfter(targetId, ev, targetEl);
    page.handleBottomDockDrop(targetId, ev, preferAfter);
    return;
  }
  if (page.bottomDockShouldAppendFromPointer(dragId, ev)) {
    var appendTargetId = page.bottomDockAppendTargetId(dragId);
    if (appendTargetId) {
      page.handleBottomDockDrop(appendTargetId, ev, true);
      return;
    }
  }
  page.persistBottomDockOrderIfChangedFromDragStart();
  page.completeBottomDockDropCleanup(ev);
}

function infringHandleBottomDockDragOver(page, id, ev, preferAfter) {
  var targetId = String(id || '').trim();
  var dragId = String(page.bottomDockDragId || '').trim();
  if (!targetId || !dragId || targetId === dragId) return;
  var nowMs = Date.now();
  var lockUntil = Number(page._bottomDockReorderLockUntil || 0);
  if (Number.isFinite(lockUntil) && lockUntil > nowMs) return;
  if (ev && ev.dataTransfer) {
    try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
  }
  var placeAfter = Boolean(preferAfter);
  var current = page.normalizeBottomDockOrder(page.bottomDockOrder);
  var next = current.slice();
  var fromIndex = next.indexOf(dragId);
  var toIndex = next.indexOf(targetId);
  if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return;
  next.splice(fromIndex, 1);
  if (fromIndex < toIndex) toIndex -= 1;
  if (placeAfter) toIndex += 1;
  if (toIndex < 0) toIndex = 0;
  if (toIndex > next.length) toIndex = next.length;
  next.splice(toIndex, 0, dragId);
  if (JSON.stringify(next) === JSON.stringify(current)) return;
  var beforeRects = page.bottomDockButtonRects();
  page.bottomDockOrder = next;
  page.animateBottomDockFromRects(beforeRects);
  var moveDuration = page.bottomDockMoveDurationMs();
  var lockMs = Math.max(320, Math.min(520, Math.round(moveDuration + 60)));
  page._bottomDockReorderLockUntil = nowMs + lockMs;
}

function infringHandleBottomDockDrop(page, id, ev, preferAfter) {
  var targetId = String(id || '').trim();
  var dragId = String(page.bottomDockDragId || '').trim();
  if (!targetId || !dragId) {
    page._bottomDockSuppressClickUntil = Date.now() + 220;
    page.cleanupBottomDockDragGhost();
    page.bottomDockDragId = '';
    page.bottomDockDragStartOrder = [];
    page.bottomDockDragCommitted = false;
    page.reviveBottomDockHoverFromPoint(
      Number(ev && ev.clientX || 0),
      Number(ev && ev.clientY || 0)
    );
    return;
  }
  if (targetId === dragId) {
    page.persistBottomDockOrderIfChangedFromDragStart();
    page.completeBottomDockDropCleanup(ev);
    return;
  }
  var next = page.normalizeBottomDockOrder(page.bottomDockOrder);
  var fromIndex = next.indexOf(dragId);
  var toIndex = next.indexOf(targetId);
  var placeAfter = Boolean(preferAfter);
  if (fromIndex < 0 || toIndex < 0) {
    page.bottomDockDragId = '';
    page.bottomDockDragStartOrder = [];
    page.bottomDockDragCommitted = false;
    page.reviveBottomDockHoverFromPoint(
      Number(ev && ev.clientX || 0),
      Number(ev && ev.clientY || 0)
    );
    return;
  }
  next.splice(fromIndex, 1);
  if (fromIndex < toIndex) toIndex -= 1;
  if (placeAfter) toIndex += 1;
  if (toIndex < 0) toIndex = 0;
  if (toIndex > next.length) toIndex = next.length;
  next.splice(toIndex, 0, dragId);
  page.bottomDockOrder = next;
  page.persistBottomDockOrder();
  page.bottomDockDragCommitted = true;
  page.completeBottomDockDropCleanup(ev);
}

function infringEndBottomDockDrag(page) {
  if (!page.bottomDockDragCommitted && Array.isArray(page.bottomDockDragStartOrder) && page.bottomDockDragStartOrder.length) {
    var current = page.normalizeBottomDockOrder(page.bottomDockOrder);
    var start = page.normalizeBottomDockOrder(page.bottomDockDragStartOrder);
    if (JSON.stringify(current) !== JSON.stringify(start)) {
      page.bottomDockOrder = current;
      page.persistBottomDockOrder();
      page.bottomDockDragCommitted = true;
    } else {
      var beforeRects = page.bottomDockButtonRects();
      page.bottomDockOrder = start;
      page.animateBottomDockFromRects(beforeRects);
    }
  }
  page.bottomDockDragId = '';
  page.bottomDockHoverId = '';
  page.bottomDockDragStartOrder = [];
  page.bottomDockDragCommitted = false;
  page._bottomDockSuppressClickUntil = Date.now() + 220;
  page.cleanupBottomDockDragGhost();
}
