// Bottom-dock tile pointer drag/click helpers keep drag lifecycle wiring out of app.ts.
function infringBindBottomDockPointerListeners(page) {
  if (page._bottomDockPointerMoveHandler || page._bottomDockPointerUpHandler) return;
  page._bottomDockPointerMoveHandler = function(ev) { page.handleBottomDockPointerMove(ev); };
  page._bottomDockPointerUpHandler = function(ev) { page.endBottomDockPointerDrag(ev); };
  window.addEventListener('pointermove', page._bottomDockPointerMoveHandler, true);
  window.addEventListener('pointerup', page._bottomDockPointerUpHandler, true);
  window.addEventListener('pointercancel', page._bottomDockPointerUpHandler, true);
  window.addEventListener('mousemove', page._bottomDockPointerMoveHandler, true);
  window.addEventListener('mouseup', page._bottomDockPointerUpHandler, true);
}

function infringUnbindBottomDockPointerListeners(page) {
  if (page._bottomDockPointerMoveHandler) {
    try { window.removeEventListener('pointermove', page._bottomDockPointerMoveHandler, true); } catch(_) {}
    try { window.removeEventListener('mousemove', page._bottomDockPointerMoveHandler, true); } catch(_) {}
  }
  if (page._bottomDockPointerUpHandler) {
    try { window.removeEventListener('pointerup', page._bottomDockPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('pointercancel', page._bottomDockPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('mouseup', page._bottomDockPointerUpHandler, true); } catch(_) {}
  }
  page._bottomDockPointerMoveHandler = null;
  page._bottomDockPointerUpHandler = null;
}

function infringStartBottomDockPointerDrag(page, id, ev) {
  if (!ev || Number(ev.button) !== 0) return;
  if (page.bottomDockContainerDragActive || page._bottomDockContainerPointerActive) return;
  var key = String(id || '').trim();
  if (!key) return;
  var hostEl = ev && ev.currentTarget ? ev.currentTarget : null;
  if (hostEl && typeof hostEl.getBoundingClientRect === 'function') {
    try {
      var rect = hostEl.getBoundingClientRect();
      var width = Number(rect.width || 32);
      var height = Number(rect.height || 32);
      var baseWidth = Number(hostEl && hostEl.offsetWidth ? hostEl.offsetWidth : width || 32);
      var baseHeight = Number(hostEl && hostEl.offsetHeight ? hostEl.offsetHeight : height || 32);
      if (!Number.isFinite(width) || width <= 0) width = 32;
      if (!Number.isFinite(height) || height <= 0) height = 32;
      if (!Number.isFinite(baseWidth) || baseWidth <= 0) baseWidth = width;
      if (!Number.isFinite(baseHeight) || baseHeight <= 0) baseHeight = height;
      var expandedScale = page.bottomDockExpandedScale();
      var expandedWidth = baseWidth * expandedScale;
      var expandedHeight = baseHeight * expandedScale;
      page._bottomDockDragGhostWidth = Math.max(20, Math.min(112, Math.max(width, expandedWidth)));
      page._bottomDockDragGhostHeight = Math.max(20, Math.min(112, Math.max(height, expandedHeight)));
      var offsetX = Number(ev.clientX || 0) - Number(rect.left || 0);
      var offsetY = Number(ev.clientY || 0) - Number(rect.top || 0);
      var relX = Number.isFinite(offsetX) && width > 0 ? (offsetX / width) : 0.5;
      var relY = Number.isFinite(offsetY) && height > 0 ? (offsetY / height) : 0.5;
      relX = Math.max(0, Math.min(1, relX));
      relY = Math.max(0, Math.min(1, relY));
      page._bottomDockPointerGrabOffsetX = relX * page._bottomDockDragGhostWidth;
      page._bottomDockPointerGrabOffsetY = relY * page._bottomDockDragGhostHeight;
    } catch(_) {
      page._bottomDockPointerGrabOffsetX = 16;
      page._bottomDockPointerGrabOffsetY = 16;
      page._bottomDockDragGhostWidth = 32;
      page._bottomDockDragGhostHeight = 32;
    }
  } else {
    page._bottomDockPointerGrabOffsetX = 16;
    page._bottomDockPointerGrabOffsetY = 16;
    page._bottomDockDragGhostWidth = 32;
    page._bottomDockDragGhostHeight = 32;
  }
  try {
    if (hostEl && typeof hostEl.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
      hostEl.setPointerCapture(ev.pointerId);
    }
  } catch(_) {}
  page._bottomDockPointerActive = true;
  page._bottomDockPointerMoved = false;
  page._bottomDockPointerCandidateId = key;
  page._bottomDockPointerStartX = Number(ev.clientX || 0);
  page._bottomDockPointerStartY = Number(ev.clientY || 0);
  page._bottomDockPointerLastX = Number(ev.clientX || 0);
  page._bottomDockPointerLastY = Number(ev.clientY || 0);
  page._bottomDockReorderLockUntil = 0;
  page.bindBottomDockPointerListeners();
}

function infringActivateBottomDockPointerDrag(page, ev) {
  if (page._bottomDockPointerMoved) return;
  var dragId = String(page._bottomDockPointerCandidateId || '').trim();
  if (!dragId) return;
  page._bottomDockPointerMoved = true;
  page.bottomDockHoverId = '';
  page.bottomDockHoverWeightById = {};
  page.bottomDockPointerX = 0;
  page.bottomDockPointerY = 0;
  page.bottomDockPreviewVisible = false;
  page.bottomDockPreviewText = '';
  page.bottomDockPreviewMorphFromText = '';
  page.bottomDockPreviewLabelMorphing = false;
  page.bottomDockPreviewWidth = 0;
  page.cancelBottomDockPreviewReflow();
  page._bottomDockRevealTargetDuringSettle = false;
  page.bottomDockDragId = dragId;
  page.bottomDockDragCommitted = false;
  page.bottomDockDragStartOrder = page.normalizeBottomDockOrder(page.bottomDockOrder);
  page.cleanupBottomDockDragGhost();
  page.captureBottomDockDragBoundaries(dragId);
  var originNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + dragId + '"]');
  if (!originNode || !document || !document.body) return;
  var dockEl = document.querySelector('.bottom-dock');
  if (dockEl && dockEl.style && typeof dockEl.style.setProperty === 'function') {
    dockEl.style.setProperty('--bottom-dock-drag-scale', String(page.readBottomDockScale(dockEl)));
  }
  var ghost = document.createElement('div');
  ghost.className = 'bottom-dock-drag-ghost bottom-dock-btn dock-tile';
  var tone = '';
  var iconKind = '';
  try {
    tone = String(originNode.getAttribute('data-dock-tone') || '').trim();
    iconKind = String(originNode.getAttribute('data-dock-icon') || '').trim();
  } catch(_) {
    tone = '';
    iconKind = '';
  }
  if (tone) ghost.setAttribute('data-dock-tone', tone);
  if (iconKind) ghost.setAttribute('data-dock-icon', iconKind);
  if (originNode.classList && typeof originNode.classList.contains === 'function') {
    if (originNode.classList.contains('active')) ghost.classList.add('active');
  }
  ghost.setAttribute('aria-hidden', 'true');
  ghost.innerHTML = String(originNode.innerHTML || '');
  ghost.style.position = 'fixed';
  ghost.style.width = Math.round(Number(page._bottomDockDragGhostWidth || 32)) + 'px';
  ghost.style.height = Math.round(Number(page._bottomDockDragGhostHeight || 32)) + 'px';
  ghost.style.borderRadius = Math.round((Number(page._bottomDockDragGhostWidth || 32) / 32) * 11) + 'px';
  ghost.style.setProperty(
    '--dock-ghost-scale',
    String(Math.max(0.8, Math.min(4, Number(page._bottomDockDragGhostWidth || 32) / 32)))
  );
  var ghostUpDeg = Number(page.bottomDockUpDegForSide(page.bottomDockActiveSide()) || 0);
  var ghostTileRotation = Math.round(ghostUpDeg) + 'deg';
  var ghostIconRotation = '0deg';
  ghost.style.setProperty('--bottom-dock-tile-rotation-deg', ghostTileRotation);
  ghost.style.setProperty('--bottom-dock-icon-rotation-deg', ghostIconRotation);
  var ghostX = Number(ev.clientX || 0) - Number(page._bottomDockPointerGrabOffsetX || 16);
  var ghostY = Number(ev.clientY || 0) - Number(page._bottomDockPointerGrabOffsetY || 16);
  page._bottomDockGhostCurrentX = ghostX;
  page._bottomDockGhostCurrentY = ghostY;
  ghost.style.left = Math.round(ghostX) + 'px';
  ghost.style.top = Math.round(ghostY) + 'px';
  ghost.style.margin = '0';
  ghost.style.pointerEvents = 'none';
  ghost.style.opacity = '1';
  document.body.appendChild(ghost);
  page._bottomDockDragGhostEl = ghost;
  page.setBottomDockGhostTarget(ghostX, ghostY);
}

function infringHandleBottomDockPointerMove(page, ev) {
  if (!page._bottomDockPointerActive) return;
  page._bottomDockPointerLastX = Number(ev.clientX || 0);
  page._bottomDockPointerLastY = Number(ev.clientY || 0);
  var movedX = Math.abs(Number(ev.clientX || 0) - Number(page._bottomDockPointerStartX || 0));
  var movedY = Math.abs(Number(ev.clientY || 0) - Number(page._bottomDockPointerStartY || 0));
  if (!page._bottomDockPointerMoved) {
    if (movedX < 5 && movedY < 5) return;
    page.activateBottomDockPointerDrag(ev);
  }
  if (!page._bottomDockPointerMoved) return;
  if (ev && typeof ev.preventDefault === 'function' && ev.cancelable) ev.preventDefault();
  var ghost = page._bottomDockDragGhostEl;
  if (ghost) {
    page.setBottomDockGhostTarget(
      Number(ev.clientX || 0) - Number(page._bottomDockPointerGrabOffsetX || 16),
      Number(ev.clientY || 0) - Number(page._bottomDockPointerGrabOffsetY || 16)
    );
  }
  var dragId = String(page.bottomDockDragId || '').trim();
  if (!dragId) return;
  var insertionIndex = page.bottomDockInsertionIndexFromPointer(dragId, ev);
  if (Number.isFinite(insertionIndex)) {
    var normalizedIndex = Math.max(0, Math.round(Number(insertionIndex || 0)));
    var nowMs = Date.now();
    var lockUntil = Number(page._bottomDockReorderLockUntil || 0);
    if (
      normalizedIndex !== Number(page._bottomDockLastInsertionIndex || -1) &&
      (!Number.isFinite(lockUntil) || lockUntil <= nowMs)
    ) {
      var changed = page.applyBottomDockReorderByIndex(dragId, normalizedIndex, true);
      page._bottomDockLastInsertionIndex = normalizedIndex;
      if (changed) {
        var moveDuration = page.bottomDockMoveDurationMs();
        var lockMs = Math.max(220, Math.min(420, Math.round(moveDuration * 0.55)));
        page._bottomDockReorderLockUntil = nowMs + lockMs;
      }
    }
    return;
  }
  var targetId = '';
  var targetEl = null;
  try {
    var pointerEl = typeof document !== 'undefined' && typeof document.elementFromPoint === 'function'
      ? document.elementFromPoint(Number(ev.clientX || 0), Number(ev.clientY || 0))
      : null;
    targetEl = pointerEl && typeof pointerEl.closest === 'function'
      ? pointerEl.closest('.bottom-dock-btn[data-dock-id]')
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

function infringEndBottomDockPointerDrag(page) {
  if (!page._bottomDockPointerActive) return;
  page._bottomDockPointerActive = false;
  page.unbindBottomDockPointerListeners();
  if (!page._bottomDockPointerMoved) {
    page._bottomDockPointerCandidateId = '';
    return;
  }
  var dragId = String(page.bottomDockDragId || page._bottomDockPointerCandidateId || '').trim();
  if (dragId) {
    var finalPointerEvent = {
      clientX: Number(page._bottomDockPointerLastX || 0),
      clientY: Number(page._bottomDockPointerLastY || 0)
    };
    var finalInsertionIndex = page.bottomDockInsertionIndexFromPointer(dragId, finalPointerEvent);
    if (Number.isFinite(finalInsertionIndex)) {
      page.applyBottomDockReorderByIndex(dragId, finalInsertionIndex, false);
    } else if (page.bottomDockShouldAppendFromPointer(dragId, finalPointerEvent)) {
      var appendTargetId = page.bottomDockAppendTargetId(dragId);
      if (appendTargetId) {
        page.handleBottomDockDragOver(appendTargetId, finalPointerEvent, true);
      }
    }
  }
  var current = page.normalizeBottomDockOrder(page.bottomDockOrder);
  var start = page.normalizeBottomDockOrder(page.bottomDockDragStartOrder);
  if (JSON.stringify(current) !== JSON.stringify(start)) {
    page.bottomDockOrder = current;
    page.persistBottomDockOrder();
    page.bottomDockDragCommitted = true;
  }
  page._bottomDockSuppressClickUntil = Date.now() + 220;
  var finalizeDrag = function() {
    var dockEl = document.querySelector('.bottom-dock');
    if (dockEl && dockEl.style && typeof dockEl.style.removeProperty === 'function') {
      dockEl.style.removeProperty('--bottom-dock-drag-scale');
    }
    var dropX = Number(page._bottomDockPointerLastX || 0);
    var dropY = Number(page._bottomDockPointerLastY || 0);
    page.bottomDockDragId = '';
    page.bottomDockHoverId = '';
    page.bottomDockDragStartOrder = [];
    page._bottomDockPointerGrabOffsetX = 16;
    page._bottomDockPointerGrabOffsetY = 16;
    page._bottomDockDragGhostWidth = 32;
    page._bottomDockDragGhostHeight = 32;
    page._bottomDockPointerCandidateId = '';
    page._bottomDockPointerMoved = false;
    page._bottomDockDragBoundaries = [];
    page._bottomDockLastInsertionIndex = -1;
    page.reviveBottomDockHoverFromPoint(dropX, dropY);
    page._bottomDockPointerLastX = 0;
    page._bottomDockPointerLastY = 0;
  };
  page.settleBottomDockDragGhost(dragId, finalizeDrag);
}

function infringShouldSuppressBottomDockClick(page) {
  var until = Number(page._bottomDockSuppressClickUntil || 0);
  return Number.isFinite(until) && until > Date.now();
}

function infringClearBottomDockClickAnimation(page) {
  if (page._bottomDockClickAnimTimer) {
    try { clearTimeout(page._bottomDockClickAnimTimer); } catch(_) {}
  }
  page._bottomDockClickAnimTimer = 0;
  page.bottomDockClickAnimId = '';
}

function infringTriggerBottomDockClickAnimation(page, id, durationOverrideMs) {
  var key = String(id || '').trim();
  if (!key || typeof window === 'undefined' || typeof window.setTimeout !== 'function') return;
  page.clearBottomDockClickAnimation();
  page.bottomDockClickAnimId = key;
  var durationMs = Number(durationOverrideMs);
  if (!Number.isFinite(durationMs) || durationMs < 120) {
    durationMs = Number(page._bottomDockClickAnimDurationMs || 980);
  }
  if (!Number.isFinite(durationMs) || durationMs < 120) durationMs = 980;
  if (typeof document !== 'undefined') {
    try {
      var tileNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
      if (tileNode && tileNode.style && typeof tileNode.style.setProperty === 'function') {
        tileNode.style.setProperty('--dock-click-duration', Math.round(durationMs) + 'ms');
      }
    } catch(_) {}
  }
  page._bottomDockClickAnimTimer = window.setTimeout(function() {
    if (typeof document !== 'undefined') {
      try {
        var activeNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
        if (activeNode && activeNode.style && typeof activeNode.style.removeProperty === 'function') {
          activeNode.style.removeProperty('--dock-click-duration');
        }
      } catch(_) {}
    }
    page._bottomDockClickAnimTimer = 0;
    page.bottomDockClickAnimId = '';
  }, durationMs);
}

function infringBottomDockIsClickAnimating(page, id) {
  var key = String(id || '').trim();
  if (!key) return false;
  return String(page.bottomDockClickAnimId || '').trim() === key;
}

function infringHandleBottomDockTileClick(page, id, targetPage, ev) {
  if (page.shouldSuppressBottomDockClick()) return;
  var key = String(id || '').trim();
  var pageKey = String(targetPage || '').trim();
  var clickAnimation = '';
  var clickDurationMs = 0;
  try {
    var triggerEl = ev && ev.currentTarget ? ev.currentTarget : null;
    clickAnimation = String(
      triggerEl && typeof triggerEl.getAttribute === 'function'
        ? (triggerEl.getAttribute('data-dock-click-animation') || '')
        : ''
    ).trim();
    clickDurationMs = Number(
      triggerEl && typeof triggerEl.getAttribute === 'function'
        ? (triggerEl.getAttribute('data-dock-click-duration-ms') || '')
        : ''
    );
  } catch(_) {
    clickAnimation = '';
    clickDurationMs = 0;
  }
  if (!Number.isFinite(clickDurationMs) || clickDurationMs < 120) clickDurationMs = 0;
  if (key && clickAnimation && clickAnimation !== 'none') {
    page.triggerBottomDockClickAnimation(key, clickDurationMs);
  }
  if (pageKey) page.navigate(pageKey);
}
