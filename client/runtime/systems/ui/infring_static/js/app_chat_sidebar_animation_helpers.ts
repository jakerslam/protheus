// Canonical Shell helper source: chat sidebar row animation projection.
// Loaded before app.ts by the dashboard asset router.
'use strict';

function infringChatSidebarFlipDurationMs(page) {
  var target = page && typeof page === 'object' ? page : {};
  var raw = Number(target._chatSidebarFlipDurationMs || 240);
  if (!Number.isFinite(raw)) raw = 240;
  return Math.max(120, Math.min(420, Math.round(raw)));
}

function infringReadChatSidebarSnapshot(page) {
  var target = page && typeof page === 'object' ? page : {};
  var refs = target.$refs || {};
  var nav = refs.sidebarNav;
  if (!nav || typeof nav.querySelectorAll !== 'function') return null;
  var nodes = nav.querySelectorAll('.nav-agent-row[data-agent-id]');
  var rects = {};
  var ids = [];
  for (var i = 0; i < nodes.length; i += 1) {
    var node = nodes[i];
    if (!node) continue;
    var id = String(node.getAttribute('data-agent-id') || '').trim();
    if (!id || Object.prototype.hasOwnProperty.call(rects, id)) continue;
    var rect = node.getBoundingClientRect();
    rects[id] = {
      left: Number(rect.left || 0),
      top: Number(rect.top || 0)
    };
    ids.push(id);
  }
  return {
    order: ids.join('|'),
    scrollTop: Number(nav.scrollTop || 0),
    rects: rects
  };
}

function infringAnimateChatSidebarFromSnapshot(page, snapshot) {
  var target = page && typeof page === 'object' ? page : {};
  if (!snapshot || typeof snapshot !== 'object') return;
  if (typeof requestAnimationFrame !== 'function') return;
  var refs = target.$refs || {};
  var nav = refs.sidebarNav;
  if (!nav || typeof nav.querySelectorAll !== 'function') return;
  var durationMs = infringChatSidebarFlipDurationMs(target);
  requestAnimationFrame(function() {
    var nodes = nav.querySelectorAll('.nav-agent-row[data-agent-id]');
    for (var i = 0; i < nodes.length; i += 1) {
      var node = nodes[i];
      if (!node || (node.classList && node.classList.contains('dragging'))) continue;
      var id = String(node.getAttribute('data-agent-id') || '').trim();
      if (!id || !Object.prototype.hasOwnProperty.call(snapshot.rects || {}, id)) continue;
      var from = snapshot.rects[id] || {};
      var rect = node.getBoundingClientRect();
      var dx = Number(from.left || 0) - Number(rect.left || 0);
      var dy = Number(from.top || 0) - Number(rect.top || 0);
      if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
      node.style.transition = 'none';
      node.style.transform = 'translate(' + Math.round(dx) + 'px,' + Math.round(dy) + 'px)';
      void node.offsetHeight;
      node.style.transition = 'transform ' + durationMs + 'ms var(--ease-smooth)';
      node.style.transform = 'translate(0px, 0px)';
      (function(el) {
        window.setTimeout(function() {
          if (!el.classList.contains('dragging')) {
            el.style.transform = '';
          }
          el.style.transition = '';
        }, durationMs + 24);
      })(node);
    }
  });
}

function infringMaybeAnimateChatSidebarRows(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (String(target.chatSidebarDragAgentId || '').trim()) {
    target._chatSidebarLastSnapshot = infringReadChatSidebarSnapshot(target);
    return;
  }
  if (target._chatSidebarFlipRaf) return;
  target._chatSidebarFlipRaf = requestAnimationFrame(function() {
    target._chatSidebarFlipRaf = 0;
    var current = infringReadChatSidebarSnapshot(target);
    if (!current) {
      target._chatSidebarLastSnapshot = null;
      return;
    }
    var previous = target._chatSidebarLastSnapshot;
    target._chatSidebarLastSnapshot = current;
    if (!previous) return;
    if (Math.abs(Number(current.scrollTop || 0) - Number(previous.scrollTop || 0)) > 1) return;
    if (String(current.order || '') === String(previous.order || '')) return;
    infringAnimateChatSidebarFromSnapshot(target, previous);
  });
}
