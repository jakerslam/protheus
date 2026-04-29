// Legacy HTML5 bottom-dock drag/drop helpers stay isolated from pointer-drag handling.
function infringBottomDockIsDraggingVisual(page, id) {
  var key = String(id || '').trim();
  if (!key) return false;
  if (page._bottomDockRevealTargetDuringSettle) return false;
  return String(page.bottomDockDragId || '').trim() === key;
}

function infringBottomDockIsNeighbor(page, id) {
  var hoverId = String(page.bottomDockHoverId || '').trim();
  var key = String(id || '').trim();
  if (!hoverId || !key || hoverId === key) return false;
  return Math.abs(page.bottomDockOrderIndex(hoverId) - page.bottomDockOrderIndex(key)) === 1;
}

function infringBottomDockIsSecondNeighbor(page, id) {
  var hoverId = String(page.bottomDockHoverId || '').trim();
  var key = String(id || '').trim();
  if (!hoverId || !key || hoverId === key) return false;
  return Math.abs(page.bottomDockOrderIndex(hoverId) - page.bottomDockOrderIndex(key)) === 2;
}

function infringBottomDockHoverWeight(page, id) {
  var key = String(id || '').trim();
  if (!key) return 0;
  var weights = page.bottomDockHoverWeightById && typeof page.bottomDockHoverWeightById === 'object'
    ? page.bottomDockHoverWeightById
    : null;
  if (weights && Object.prototype.hasOwnProperty.call(weights, key)) {
    var exact = Number(weights[key] || 0);
    if (Number.isFinite(exact)) return Math.max(0, Math.min(1, exact));
  }
  if (key === String(page.bottomDockHoverId || '').trim()) return 1;
  if (page.bottomDockIsNeighbor(key)) return 0.33;
  if (page.bottomDockIsSecondNeighbor(key)) return 0.11;
  return 0;
}

function infringStartBottomDockDrag(page, id, ev) {
  var key = String(id || '').trim();
  if (!key) return;
  page.cleanupBottomDockDragGhost();
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
  page.bottomDockDragId = key;
  page.bottomDockDragCommitted = false;
  page.bottomDockDragStartOrder = page.normalizeBottomDockOrder(page.bottomDockOrder);
  page._bottomDockReorderLockUntil = 0;
  page.captureBottomDockDragBoundaries(key);
  if (ev && ev.dataTransfer) {
    try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
    try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
    try {
      var dragNode = ev.currentTarget;
      if (dragNode && typeof ev.dataTransfer.setDragImage === 'function') {
        var rect = dragNode.getBoundingClientRect();
        var ghost = dragNode.cloneNode(true);
        if (ghost && document && document.body) {
          ghost.classList.add('bottom-dock-drag-ghost');
          ghost.style.position = 'fixed';
          ghost.style.left = '-9999px';
          ghost.style.top = '-9999px';
          ghost.style.margin = '0';
          ghost.style.transform = 'none';
          ghost.style.pointerEvents = 'none';
          ghost.style.opacity = '1';
          document.body.appendChild(ghost);
          page._bottomDockDragGhostEl = ghost;
          ev.dataTransfer.setDragImage(
            ghost,
            Math.max(0, Math.round(Number(rect.width || 0) / 2)),
            Math.max(0, Math.round(Number(rect.height || 0) / 2))
          );
        } else {
          ev.dataTransfer.setDragImage(
            dragNode,
            Math.max(0, Math.round(Number(rect.width || 0) / 2)),
            Math.max(0, Math.round(Number(rect.height || 0) / 2))
          );
        }
      }
    } catch(_) {}
    try { ev.dataTransfer.setData('application/x-infring-dock', key); } catch(_) {}
    try { ev.dataTransfer.setData('text/plain', key); } catch(_) {}
  }
}

function infringBottomDockShouldInsertAfter(page, targetId, ev, targetEl) {
  var key = String(targetId || '').trim();
  if (!key) return false;
  if (!ev) return false;
  var clientX = Number(ev.clientX || 0);
  var clientY = Number(ev.clientY || 0);
  if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
  var node = targetEl || null;
  if (!node && typeof document !== 'undefined') {
    try {
      node = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
    } catch(_) {
      node = null;
    }
  }
  if (!node || typeof node.getBoundingClientRect !== 'function') return false;
  var rect = node.getBoundingClientRect();
  var width = Number(rect.width || 0);
  var height = Number(rect.height || 0);
  if (!Number.isFinite(width) || width <= 0) return false;
  if (!Number.isFinite(height) || height <= 0) return false;
  var basis = page.bottomDockAxisBasis();
  var centerX = Number(rect.left || 0) + (width / 2);
  var centerY = Number(rect.top || 0) + (height / 2);
  var centerProj = page.bottomDockProjectPointToAxis(centerX, centerY, basis);
  var pointerProj = page.bottomDockProjectPointToAxis(clientX, clientY, basis);
  var half = page.bottomDockAxisHalfExtent(width, height, basis).primary;
  if (!Number.isFinite(half) || half <= 0) half = Math.max(width, height) / 2;
  if (!Number.isFinite(half) || half <= 0) return false;
  var ratio = (pointerProj.primary - (centerProj.primary - half)) / (half * 2);
  return ratio >= 0.5;
}

function infringCaptureBottomDockDragBoundaries(page, dragId) {
  var key = String(dragId || '').trim();
  if (!key || typeof document === 'undefined') {
    page._bottomDockDragBoundaries = [];
    page._bottomDockLastInsertionIndex = -1;
    return [];
  }
  var dock = null;
  try {
    dock = document.querySelector('.bottom-dock');
  } catch(_) {
    dock = null;
  }
  if (!dock) {
    page._bottomDockDragBoundaries = [];
    page._bottomDockLastInsertionIndex = -1;
    return [];
  }
  var centers = [];
  var basis = page.bottomDockAxisBasis();
  try {
    var nodes = dock.querySelectorAll('.bottom-dock-btn[data-dock-id]');
    for (var i = 0; i < nodes.length; i += 1) {
      var node = nodes[i];
      if (!node || typeof node.getAttribute !== 'function') continue;
      var id = String(node.getAttribute('data-dock-id') || '').trim();
      if (!id || id === key || typeof node.getBoundingClientRect !== 'function') continue;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) continue;
      if (!Number.isFinite(height) || height <= 0) continue;
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      centers.push(page.bottomDockProjectPointToAxis(centerX, centerY, basis).primary);
    }
  } catch(_) {}
  centers.sort(function(a, b) { return a - b; });
  page._bottomDockDragBoundaries = centers;
  page._bottomDockLastInsertionIndex = -1;
  return centers;
}

function infringBottomDockAppendTargetId(page, dragId) {
  var key = String(dragId || '').trim();
  if (!key) return '';
  var order = page.normalizeBottomDockOrder(page.bottomDockOrder);
  var filtered = [];
  for (var i = 0; i < order.length; i += 1) {
    var id = String(order[i] || '').trim();
    if (!id || id === key) continue;
    filtered.push(id);
  }
  if (!filtered.length) return '';
  return String(filtered[filtered.length - 1] || '').trim();
}

function infringBottomDockShouldAppendFromPointer(page, dragId, ev) {
  var key = String(dragId || '').trim();
  if (!key || !ev || typeof document === 'undefined') return false;
  var clientX = Number(ev.clientX || 0);
  var clientY = Number(ev.clientY || 0);
  if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
  var appendTargetId = page.bottomDockAppendTargetId(key);
  if (!appendTargetId) return false;
  var node = null;
  try {
    node = document.querySelector('.bottom-dock-btn[data-dock-id="' + appendTargetId + '"]');
  } catch(_) {
    node = null;
  }
  if (!node || typeof node.getBoundingClientRect !== 'function') return false;
  var rect = node.getBoundingClientRect();
  var width = Number(rect.width || 0);
  var height = Number(rect.height || 0);
  if (!Number.isFinite(width) || width <= 0) return false;
  if (!Number.isFinite(height) || height <= 0) return false;
  var basis = page.bottomDockAxisBasis();
  var centerX = Number(rect.left || 0) + (width / 2);
  var centerY = Number(rect.top || 0) + (height / 2);
  var centerProj = page.bottomDockProjectPointToAxis(centerX, centerY, basis);
  var pointerProj = page.bottomDockProjectPointToAxis(clientX, clientY, basis);
  var extent = page.bottomDockAxisHalfExtent(width, height, basis);
  var halfPrimary = Number(extent.primary || 0);
  var halfSecondary = Number(extent.secondary || 0);
  if (!Number.isFinite(halfPrimary) || halfPrimary <= 0) halfPrimary = Math.max(width, height) / 2;
  if (!Number.isFinite(halfSecondary) || halfSecondary <= 0) halfSecondary = Math.min(width, height) / 2;
  var secondaryPad = Math.max(18, halfSecondary * 0.75);
  if (Math.abs(pointerProj.secondary - centerProj.secondary) > (halfSecondary + secondaryPad)) return false;
  var threshold = centerProj.primary + halfPrimary - Math.min(18, halfPrimary * 0.7);
  return pointerProj.primary >= threshold;
}
