function infringTaskbarReorderDefaults(page, group) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.taskbarDockService();
  if (service && typeof service.taskbarOrderDefaults === 'function') return service.taskbarOrderDefaults(group);
  var key = String(group || '').trim().toLowerCase();
  if (key === 'right') return ['connectivity', 'theme', 'notifications', 'search', 'auth'];
  return ['nav_cluster'];
}

function infringTaskbarReorderStorageKey(page, group) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.taskbarDockService();
  if (service && typeof service.taskbarStorageKey === 'function') return service.taskbarStorageKey(group);
  var key = String(group || '').trim().toLowerCase();
  return key === 'right' ? 'infring-taskbar-order-right' : 'infring-taskbar-order-left';
}

function infringTaskbarReorderOrderForGroup(page, group) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  return key === 'right' ? target.taskbarReorderRight : target.taskbarReorderLeft;
}

function infringSetTaskbarReorderOrderForGroup(page, group, nextOrder) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key === 'right') {
    target.taskbarReorderRight = nextOrder;
    return;
  }
  target.taskbarReorderLeft = nextOrder;
}

function infringNormalizeTaskbarReorder(page, group, rawOrder) {
  var target = page && typeof page === 'object' ? page : {};
  var service = target.taskbarDockService();
  if (service && typeof service.normalizeOrder === 'function') return service.normalizeOrder(rawOrder, target.taskbarReorderDefaults(group));
  var defaults = target.taskbarReorderDefaults(group);
  var source = Array.isArray(rawOrder) ? rawOrder : [];
  var seen = {};
  var ordered = [];
  for (var i = 0; i < source.length; i += 1) {
    var id = String(source[i] || '').trim();
    if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
    seen[id] = true;
    ordered.push(id);
  }
  for (var j = 0; j < defaults.length; j += 1) {
    var fallbackId = defaults[j];
    if (seen[fallbackId]) continue;
    seen[fallbackId] = true;
    ordered.push(fallbackId);
  }
  return ordered;
}

function infringPersistTaskbarReorder(page, group) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  var normalized = target.normalizeTaskbarReorder(key, target.taskbarReorderOrderForGroup(key));
  target.setTaskbarReorderOrderForGroup(key, normalized);
  try {
    var service = target.taskbarDockService();
    if (service && typeof service.persistTaskbarOrder === 'function') normalized = service.persistTaskbarOrder(key, normalized);
    else localStorage.setItem(target.taskbarReorderStorageKey(key), JSON.stringify(normalized));
  } catch(_) {}
  infringUpdateShellLayoutConfig(function(config) {
    if (key === 'right') config.taskbar.orderRight = normalized.slice();
    else config.taskbar.orderLeft = normalized.slice();
  });
}

function infringTaskbarReorderOrderIndex(page, group, item) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  var itemId = String(item || '').trim();
  if (!itemId) return 999;
  var service = target.taskbarDockService();
  if (service && typeof service.orderIndex === 'function') {
    return service.orderIndex(itemId, target.taskbarReorderOrderForGroup(key), target.taskbarReorderDefaults(key));
  }
  var order = target.normalizeTaskbarReorder(key, target.taskbarReorderOrderForGroup(key));
  var idx = order.indexOf(itemId);
  if (idx >= 0) return idx;
  var fallback = target.taskbarReorderDefaults(key).indexOf(itemId);
  return fallback >= 0 ? fallback : 999;
}

function infringTaskbarReorderItemStyle(page, group, item) {
  var target = page && typeof page === 'object' ? page : {};
  return 'order:' + target.taskbarReorderOrderIndex(group, item);
}

function infringTaskbarReorderItemRects(group) {
  if (typeof document === 'undefined') return {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  var out = {};
  var box = null;
  try { box = document.querySelector('.taskbar-reorder-box-' + key); } catch(_) { box = null; }
  if (!box || typeof box.querySelectorAll !== 'function') return out;
  var nodes = box.querySelectorAll('.taskbar-reorder-item[data-taskbar-item]');
  for (var i = 0; i < nodes.length; i += 1) {
    var node = nodes[i];
    if (!node || typeof node.getBoundingClientRect !== 'function') continue;
    var id = String(node.getAttribute('data-taskbar-item') || '').trim();
    if (!id || Object.prototype.hasOwnProperty.call(out, id)) continue;
    var rect = node.getBoundingClientRect();
    out[id] = { left: Number(rect.left || 0), top: Number(rect.top || 0) };
  }
  return out;
}

function infringAnimateTaskbarReorderFromRects(group, beforeRects) {
  if (!beforeRects || typeof beforeRects !== 'object') return;
  if (typeof requestAnimationFrame !== 'function' || typeof document === 'undefined') return;
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  requestAnimationFrame(function() {
    var box = null;
    try { box = document.querySelector('.taskbar-reorder-box-' + key); } catch(_) { box = null; }
    if (!box || typeof box.querySelectorAll !== 'function') return;
    var nodes = box.querySelectorAll('.taskbar-reorder-item[data-taskbar-item]');
    for (var i = 0; i < nodes.length; i += 1) {
      var node = nodes[i];
      if (!node || node.classList.contains('dragging')) continue;
      var id = String(node.getAttribute('data-taskbar-item') || '').trim();
      if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
      var from = beforeRects[id] || {};
      var rect = node.getBoundingClientRect();
      var dx = Number(from.left || 0) - Number(rect.left || 0);
      var dy = Number(from.top || 0) - Number(rect.top || 0);
      if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
      node.style.transition = 'none';
      node.style.transform = 'translate(' + Math.round(dx) + 'px,' + Math.round(dy) + 'px)';
      void node.offsetHeight;
      node.style.transition = 'transform 220ms var(--ease-smooth)';
      node.style.transform = 'translate(0px, 0px)';
      (function(el) {
        window.setTimeout(function() {
          if (!el.classList.contains('dragging')) el.style.transform = '';
          el.style.transition = '';
        }, 250);
      })(node);
    }
  });
}

function infringApplyTaskbarReorder(page, group, dragItem, targetItem, preferAfter, animate) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  var dragId = String(dragItem || '').trim();
  var targetId = String(targetItem || '').trim();
  if (!dragId || !targetId || dragId === targetId) return false;
  var current = target.normalizeTaskbarReorder(key, target.taskbarReorderOrderForGroup(key));
  var fromIndex = current.indexOf(dragId);
  var toIndex = current.indexOf(targetId);
  if (fromIndex < 0 || toIndex < 0) return false;
  var next = current.slice();
  next.splice(fromIndex, 1);
  if (fromIndex < toIndex) toIndex -= 1;
  if (Boolean(preferAfter)) toIndex += 1;
  if (toIndex < 0) toIndex = 0;
  if (toIndex > next.length) toIndex = next.length;
  next.splice(toIndex, 0, dragId);
  if (JSON.stringify(next) === JSON.stringify(current)) return false;
  var beforeRects = Boolean(animate) ? target.taskbarReorderItemRects(key) : null;
  target.setTaskbarReorderOrderForGroup(key, next);
  if (beforeRects) target.animateTaskbarReorderFromRects(key, beforeRects);
  return true;
}

function infringHandleTaskbarReorderPointerDown(page, group, ev) {
  var target = page && typeof page === 'object' ? page : {};
  if (String(target.taskbarDragGroup || '').trim()) return;
  if (!ev || Number(ev.button) !== 0) return;
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  var eventTarget = ev && ev.target && typeof ev.target.closest === 'function'
    ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
    : null;
  var item = eventTarget ? String(eventTarget.getAttribute('data-taskbar-item') || '').trim() : '';
  if (!item) return;
  target.cancelTaskbarDragHold();
  target._taskbarDragHoldGroup = key;
  target._taskbarDragHoldItem = item;
  if (typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
    target._taskbarDragHoldTimer = window.setTimeout(function() {
      target._taskbarDragHoldTimer = 0;
      target._taskbarDragArmedGroup = key;
      target._taskbarDragArmedItem = item;
    }, 180);
  }
}

function infringCancelTaskbarDragHold(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (target._taskbarDragHoldTimer) {
    try { clearTimeout(target._taskbarDragHoldTimer); } catch(_) {}
  }
  target._taskbarDragHoldTimer = 0;
  target._taskbarDragHoldGroup = '';
  target._taskbarDragHoldItem = '';
  if (!String(target.taskbarDragGroup || '').trim()) {
    target._taskbarDragArmedGroup = '';
    target._taskbarDragArmedItem = '';
  }
}

function infringForceTaskbarMoveDragEffect(ev) {
  if (!ev || !ev.dataTransfer) return;
  try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
  try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
}

function infringSetTaskbarDragBodyActive(active) {
  if (typeof document === 'undefined' || !document.body || !document.body.classList) return;
  if (active) document.body.classList.add('taskbar-drag-active');
  else document.body.classList.remove('taskbar-drag-active');
}

function infringHandleTaskbarReorderDragStart(page, group, ev) {
  var targetPage = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  var target = ev && ev.target && typeof ev.target.closest === 'function'
    ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
    : null;
  var item = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
  if (!item || targetPage._taskbarDragArmedGroup !== key || targetPage._taskbarDragArmedItem !== item) {
    if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
    return;
  }
  targetPage.taskbarDragGroup = key;
  targetPage.taskbarDragItem = item;
  targetPage.taskbarDragStartOrder = targetPage.normalizeTaskbarReorder(key, targetPage.taskbarReorderOrderForGroup(key));
  targetPage._taskbarDragArmedGroup = '';
  targetPage._taskbarDragArmedItem = '';
  targetPage.cancelTaskbarDragHold();
  if (ev && ev.dataTransfer) {
    targetPage.forceTaskbarMoveDragEffect(ev);
    try { ev.dataTransfer.setData('application/x-infring-taskbar', key + ':' + item); } catch(_) {}
    try { ev.dataTransfer.setData('text/plain', key + ':' + item); } catch(_) {}
    try {
      if (typeof document !== 'undefined' && document.body && typeof ev.dataTransfer.setDragImage === 'function') {
        var ghost = target && typeof target.cloneNode === 'function' ? target.cloneNode(true) : document.createElement('span');
        ghost.style.position = 'fixed';
        ghost.style.left = '-9999px';
        ghost.style.top = '-9999px';
        ghost.style.margin = '0';
        ghost.style.pointerEvents = 'none';
        ghost.style.transform = 'none';
        ghost.style.opacity = '1';
        if (ghost.classList && ghost.classList.contains('dragging')) ghost.classList.remove('dragging');
        var rect = target && typeof target.getBoundingClientRect === 'function' ? target.getBoundingClientRect() : null;
        var offsetX = 0;
        var offsetY = 0;
        if (rect) {
          var width = Math.max(1, Math.round(Number(rect.width || 0)));
          var height = Math.max(1, Math.round(Number(rect.height || 0)));
          ghost.style.width = width + 'px';
          ghost.style.height = height + 'px';
          ghost.style.boxSizing = 'border-box';
          if (typeof ev.clientX === 'number') offsetX = Math.round(Math.max(0, Math.min(width, ev.clientX - rect.left)));
          if (typeof ev.clientY === 'number') offsetY = Math.round(Math.max(0, Math.min(height, ev.clientY - rect.top)));
        } else {
          ghost.style.width = '1px';
          ghost.style.height = '1px';
        }
        document.body.appendChild(ghost);
        ev.dataTransfer.setDragImage(ghost, offsetX, offsetY);
        window.setTimeout(function() {
          if (ghost.parentNode) ghost.parentNode.removeChild(ghost);
        }, 0);
      }
    } catch(_) {}
  }
  if (target && target.classList) target.classList.add('dragging');
  targetPage.setTaskbarDragBodyActive(true);
}

function infringHandleTaskbarReorderDragMove(page, ev) {
  var target = page && typeof page === 'object' ? page : {};
  target.forceTaskbarMoveDragEffect(ev);
}

function infringHandleTaskbarReorderDragEnter(page, group, ev) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  if (String(target.taskbarDragGroup || '').trim() !== key) return;
  if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
  target.forceTaskbarMoveDragEffect(ev);
}

function infringHandleTaskbarReorderDragOver(page, group, ev) {
  var targetPage = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  if (String(targetPage.taskbarDragGroup || '').trim() !== key) return;
  if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
  targetPage.forceTaskbarMoveDragEffect(ev);
  var dragItem = String(targetPage.taskbarDragItem || '').trim();
  if (!dragItem) return;
  var target = ev && ev.target && typeof ev.target.closest === 'function'
    ? ev.target.closest('.taskbar-reorder-item[data-taskbar-item]')
    : null;
  var targetItem = target ? String(target.getAttribute('data-taskbar-item') || '').trim() : '';
  if (!targetItem || targetItem === dragItem) return;
  var preferAfter = false;
  if (target && typeof target.getBoundingClientRect === 'function') {
    var rect = target.getBoundingClientRect();
    var midX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
    preferAfter = Number(ev && ev.clientX || 0) >= midX;
  }
  targetPage.applyTaskbarReorder(key, dragItem, targetItem, preferAfter, true);
}

function infringClearTaskbarReorderDraggingClass() {
  if (typeof document === 'undefined') return;
  try {
    var draggingNodes = document.querySelectorAll('.taskbar-reorder-item.dragging');
    for (var i = 0; i < draggingNodes.length; i += 1) {
      draggingNodes[i].classList.remove('dragging');
    }
  } catch(_) {}
}

function infringHandleTaskbarReorderDrop(page, group, ev) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(group || '').trim().toLowerCase();
  if (key !== 'right') key = 'left';
  if (String(target.taskbarDragGroup || '').trim() !== key) return;
  if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
  target.persistTaskbarReorder(key);
  target.taskbarDragGroup = '';
  target.taskbarDragItem = '';
  target.taskbarDragStartOrder = [];
  target.cancelTaskbarDragHold();
  target.setTaskbarDragBodyActive(false);
  target.clearTaskbarReorderDraggingClass();
}

function infringHandleTaskbarDragEnd(page) {
  var target = page && typeof page === 'object' ? page : {};
  var key = String(target.taskbarDragGroup || '').trim();
  if (key) target.persistTaskbarReorder(key);
  target.taskbarDragGroup = '';
  target.taskbarDragItem = '';
  target.taskbarDragStartOrder = [];
  target.cancelTaskbarDragHold();
  target.setTaskbarDragBodyActive(false);
  target.clearTaskbarReorderDraggingClass();
}
