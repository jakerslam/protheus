function infringIsSystemSidebarThread(page, agent) {
  if (!agent || typeof agent !== 'object') return false;
  if (agent.is_system_thread === true) return true;
  var id = String(agent.id || '').trim().toLowerCase();
  if (id === 'system') return true;
  var role = String(agent.role || '').trim().toLowerCase();
  return role === 'system';
}

function infringIsSidebarArchivedAgent(page, agent) {
  if (!agent || typeof agent !== 'object') return false;
  var store = page.getAppStore();
  if (store && typeof store.isArchivedLikeAgent === 'function') return store.isArchivedLikeAgent(agent);
  if (Object.prototype.hasOwnProperty.call(agent, 'sidebar_archived')) return !!agent.sidebar_archived;
  return !!agent.archived;
}

function infringIsReservedSystemEmoji(rawEmoji) {
  var normalized = String(rawEmoji || '').replace(/\uFE0F/g, '').trim();
  return normalized === '⚙';
}

function infringSanitizeSidebarAgentRow(page, agent) {
  if (!agent || typeof agent !== 'object') return agent;
  var row = Object.assign({}, agent);
  var identity = Object.assign({}, (row.identity && typeof row.identity === 'object') ? row.identity : {});
  if (page.isSystemSidebarThread(row)) {
    row.id = 'system';
    row.name = 'System';
    row.is_system_thread = true;
    row.role = 'system';
    identity.emoji = '\u2699\ufe0f';
    row.identity = identity;
    return row;
  }
  if (page.isReservedSystemEmoji(identity.emoji)) {
    identity.emoji = '';
  }
  row.identity = identity;
  return row;
}

function infringPersistChatSidebarTopologyOrder(page) {
  var seen = {};
  var out = [];
  (page.chatSidebarTopologyOrder || []).forEach(function(id) {
    var key = String(id || '').trim();
    if (!key || seen[key]) return;
    seen[key] = true;
    out.push(key);
  });
  page.chatSidebarTopologyOrder = out;
  try {
    localStorage.setItem('infring-chat-sidebar-topology-order', JSON.stringify(out));
  } catch(_) {}
}

function infringChatSidebarCanReorderTopology(page) {
  return String(page.chatSidebarSortMode || '').toLowerCase() === 'topology';
}

function infringStartChatSidebarTopologyDrag(page, agent, ev) {
  if (!page.chatSidebarCanReorderTopology() || !agent || !agent.id) return;
  page.syncChatSidebarTopologyOrderFromAgents();
  page.chatSidebarDragAgentId = String(agent.id);
  page.chatSidebarDropTargetId = '';
  page.chatSidebarDropAfter = false;
  if (ev && ev.dataTransfer) {
    ev.dataTransfer.effectAllowed = 'move';
    ev.dataTransfer.setData('text/plain', page.chatSidebarDragAgentId);
  }
}

function infringHandleChatSidebarTopologyDragOver(page, agent, ev) {
  if (!page.chatSidebarCanReorderTopology() || !page.chatSidebarDragAgentId || !agent || !agent.id) return;
  if (ev) {
    ev.preventDefault();
    if (ev.dataTransfer) ev.dataTransfer.dropEffect = 'move';
  }
  var targetId = String(agent.id);
  var dropAfter = false;
  if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
    var rect = ev.currentTarget.getBoundingClientRect();
    dropAfter = ev.clientY > (rect.top + (rect.height / 2));
  }
  page.chatSidebarDropAfter = !!dropAfter;
  page.chatSidebarDropTargetId = targetId === page.chatSidebarDragAgentId ? '' : targetId;
}

function infringHandleChatSidebarTopologyDrop(page, agent, ev) {
  if (ev) ev.preventDefault();
  if (!page.chatSidebarCanReorderTopology() || !agent || !agent.id) return page.endChatSidebarTopologyDrag();
  var dragId = String(page.chatSidebarDragAgentId || '').trim();
  if (!dragId && ev && ev.dataTransfer) dragId = String(ev.dataTransfer.getData('text/plain') || '').trim();
  var targetId = String(agent.id).trim();
  if (!dragId || !targetId || dragId === targetId) return page.endChatSidebarTopologyDrag();
  page.syncChatSidebarTopologyOrderFromAgents();
  var order = (page.chatSidebarTopologyOrder || []).slice();
  var fromIndex = order.indexOf(dragId);
  var targetIndex = order.indexOf(targetId);
  if (fromIndex < 0 || targetIndex < 0) return page.endChatSidebarTopologyDrag();
  var dropAfter = false;
  if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
    var rect = ev.currentTarget.getBoundingClientRect();
    dropAfter = ev.clientY > (rect.top + (rect.height / 2));
  }
  order.splice(fromIndex, 1);
  if (fromIndex < targetIndex) targetIndex -= 1;
  if (dropAfter) targetIndex += 1;
  if (targetIndex < 0) targetIndex = 0;
  if (targetIndex > order.length) targetIndex = order.length;
  order.splice(targetIndex, 0, dragId);
  page.chatSidebarTopologyOrder = order;
  page.persistChatSidebarTopologyOrder();
  page.endChatSidebarTopologyDrag();
  page.scheduleSidebarScrollIndicators();
}

function infringEndChatSidebarTopologyDrag(page) {
  page.chatSidebarDragAgentId = '';
  page.chatSidebarDropTargetId = '';
  page.chatSidebarDropAfter = false;
}

function infringChatSidebarAgents(page) {
  var list = (page.agents || []).slice();
  var pendingFreshId = String((page.getAppStore() && page.getAppStore().pendingFreshAgentId) || '').trim();
  list = list.filter(function(agent) {
    if (!agent || !agent.id) return false;
    if (pendingFreshId && String(agent.id || '') === pendingFreshId) return false;
    if (page.isSidebarArchivedAgent(agent)) return false;
    return true;
  });
  list.sort(function(a, b) {
    return page.chatSidebarSortComparator(a, b);
  });
  if (page.chatSidebarCanReorderTopology() && Array.isArray(page.chatSidebarTopologyOrder) && page.chatSidebarTopologyOrder.length) {
    var rank = {};
    page.chatSidebarTopologyOrder.forEach(function(id, idx) {
      var key = String(id || '').trim();
      if (!key || rank[key] != null) return;
      rank[key] = idx;
    });
    list.sort(function(a, b) {
      var aId = String((a && a.id) || '');
      var bId = String((b && b.id) || '');
      var hasA = Object.prototype.hasOwnProperty.call(rank, aId);
      var hasB = Object.prototype.hasOwnProperty.call(rank, bId);
      if (hasA && hasB && rank[aId] !== rank[bId]) return rank[aId] - rank[bId];
      if (hasA && !hasB) return -1;
      if (!hasA && hasB) return 1;
      return page.chatSidebarSortComparator(a, b);
    });
  }
  return list.map(function(agent) {
    return page.sanitizeSidebarAgentRow(agent);
  });
}

function infringChatSidebarRows(page) {
  if (page.chatSidebarDragActive && Array.isArray(page._chatSidebarDragRowsCache)) {
    return page._chatSidebarDragRowsCache;
  }
  var query = String(page.chatSidebarQuery || '').trim();
  var rows;
  if (!query) rows = page.chatSidebarAgents || [];
  else if (Array.isArray(page.chatSidebarSearchResults) && page.chatSidebarSearchResults.length) rows = page.chatSidebarSearchResults;
  else rows = [];
  if (page.chatSidebarDragActive) {
    page._chatSidebarDragRowsCache = Array.isArray(rows) ? rows.slice() : [];
  } else {
    page._chatSidebarDragRowsCache = null;
  }
  return rows;
}

function infringChatSidebarDragRenderWindow(page, rows) {
  var sourceRows = Array.isArray(rows) ? rows : [];
  var total = sourceRows.length;
  var maxRows = Math.max(1, Math.floor(Number(page._chatSidebarDragRenderMaxRows || 10)));
  if (!page.chatSidebarDragActive || total <= maxRows) {
    return { virtualized: false, start: 0, end: total, padTop: 0, padBottom: 0 };
  }
  var refs = page.$refs || {};
  var nav = refs.sidebarNav || null;
  var rowHeight = Math.max(1, Math.floor(Number(page._chatSidebarDragRenderRowHeight || 56)));
  var scrollTop = nav ? Math.max(0, Number(nav.scrollTop || 0)) : 0;
  var start = Math.max(0, Math.floor(scrollTop / rowHeight));
  if (start > (total - maxRows)) start = Math.max(0, total - maxRows);
  var end = Math.min(total, start + maxRows);
  return {
    virtualized: true,
    start: start,
    end: end,
    padTop: start * rowHeight,
    padBottom: Math.max(0, (total - end) * rowHeight)
  };
}

function infringChatSidebarVirtualized(page) {
  var rows = Array.isArray(page.chatSidebarRows) ? page.chatSidebarRows : [];
  return page.chatSidebarDragRenderWindow(rows).virtualized;
}

function infringChatSidebarVirtualPadTop(page) {
  var rows = Array.isArray(page.chatSidebarRows) ? page.chatSidebarRows : [];
  return page.chatSidebarDragRenderWindow(rows).padTop;
}
