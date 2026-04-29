function infringReadChatMapElement() {
  if (typeof document === 'undefined' || typeof document.querySelector !== 'function') return null;
  try { return document.querySelector('.chat-map'); } catch(_) {}
  return null;
}

function infringReadChatMapHeight(page) {
  var target = page && typeof page === 'object' ? page : {};
  var node = target.readChatMapElement();
  var height = Number(node && node.offsetHeight || 0);
  if (!Number.isFinite(height) || height <= 0) {
    height = Math.max(180, target.taskbarReadViewportHeight() - 276);
  }
  return height;
}

function infringChatMapPlacementEnabled(page) {
  var target = page && typeof page === 'object' ? page : {};
  return target.page === 'chat' || (target.page === 'agents' && !!target.activeChatAgent);
}

function infringChatMapClampTop(page, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatMapHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var top = Number(topRaw);
  if (!Number.isFinite(top)) top = minTop + ((maxTop - minTop) * 0.38);
  return Math.max(minTop, Math.min(maxTop, top));
}

function infringChatMapPersistPlacementFromTop(page, topRaw) {
  var target = page && typeof page === 'object' ? page : {};
  var bounds = target.chatOverlayVerticalBounds();
  var height = target.readChatMapHeight();
  var minTop = Number(bounds.minTop || 0);
  var maxTop = Number(bounds.maxBottom || 0) - height;
  if (!Number.isFinite(maxTop) || maxTop < minTop) maxTop = minTop;
  var top = target.chatMapClampTop(topRaw);
  var ratio = maxTop > minTop ? (top - minTop) / (maxTop - minTop) : 0.38;
  ratio = Math.max(0, Math.min(1, ratio));
  target.chatMapPlacementY = ratio;
  try {
    localStorage.setItem('infring-chat-map-placement-y', String(ratio));
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    config.chatMap.placementY = ratio;
  });
}

function infringShouldIgnoreChatMapDragTarget(page, targetNode) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.dragbarService();
  var ignoreSelector = 'button, a, input, textarea, select, [role="button"], [contenteditable="true"], .chat-map-item, .chat-map-day, .chat-map-jump';
  if (service && typeof service.shouldIgnoreTarget === 'function') {
    return service.shouldIgnoreTarget(targetNode, { ignoreSelector: ignoreSelector });
  }
  var node = targetNode;
  if (node && typeof node.closest !== 'function' && node.parentElement) {
    node = node.parentElement;
  }
  if (!node || typeof node.closest !== 'function') return false;
  return Boolean(node.closest(ignoreSelector));
}

function infringBindChatMapPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof window === 'undefined') return;
  if (target._chatMapPointerMoveHandler || target._chatMapPointerUpHandler) return;
  target._chatMapPointerMoveHandler = function(ev) { target.handleChatMapPointerMove(ev); };
  target._chatMapPointerUpHandler = function() { target.endChatMapPointerDrag(); };
  window.addEventListener('pointermove', target._chatMapPointerMoveHandler, true);
  window.addEventListener('pointerup', target._chatMapPointerUpHandler, true);
  window.addEventListener('pointercancel', target._chatMapPointerUpHandler, true);
  window.addEventListener('mousemove', target._chatMapPointerMoveHandler, true);
  window.addEventListener('mouseup', target._chatMapPointerUpHandler, true);
}

function infringUnbindChatMapPointerListeners(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (typeof window === 'undefined') return;
  if (target._chatMapPointerMoveHandler) {
    try { window.removeEventListener('pointermove', target._chatMapPointerMoveHandler, true); } catch(_) {}
    try { window.removeEventListener('mousemove', target._chatMapPointerMoveHandler, true); } catch(_) {}
  }
  if (target._chatMapPointerUpHandler) {
    try { window.removeEventListener('pointerup', target._chatMapPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('pointercancel', target._chatMapPointerUpHandler, true); } catch(_) {}
    try { window.removeEventListener('mouseup', target._chatMapPointerUpHandler, true); } catch(_) {}
  }
  target._chatMapPointerMoveHandler = null;
  target._chatMapPointerUpHandler = null;
}
