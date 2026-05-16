function infringNormalizeNavigablePage(pageId) {
  var raw = String(pageId || '').trim().toLowerCase();
  if (!raw) return 'chat';
  var aliases = {
    'automation': 'scheduler',
    'templates': 'agents',
    'triggers': 'workflows',
    'cron': 'scheduler',
    'schedules': 'scheduler',
    'memory': 'sessions',
    'audit': 'logs',
    'security': 'settings',
    'peers': 'settings',
    'migration': 'settings',
    'usage': 'analytics',
    'approval': 'approvals'
  };
  return aliases[raw] || raw;
}

function infringIsKnownNavigablePage(page, pageId) {
  var normalized = page.normalizeNavigablePage(pageId);
  return ['chat','agents','sessions','approvals','comms','workflows','scheduler','channels','eyes','skills','hands','overview','analytics','logs','runtime','settings','wizard']
    .indexOf(normalized) >= 0;
}

function infringSyncPageHistory(page, nextPage) {
  var next = page.normalizeNavigablePage(nextPage);
  if (!page.isKnownNavigablePage(next)) return;
  var current = page.normalizeNavigablePage(page._navCurrentPage || page.page || '');
  var action = String(page._navHistoryAction || '').trim().toLowerCase();
  var back = Array.isArray(page.navBackStack) ? page.navBackStack.slice() : [];
  var forward = Array.isArray(page.navForwardStack) ? page.navForwardStack.slice() : [];
  var cap = Number(page._navHistoryCap || 48);
  if (!Number.isFinite(cap) || cap < 8) cap = 48;
  var trim = function(list) {
    return list.length > cap ? list.slice(list.length - cap) : list;
  };
  if (!current || !page.isKnownNavigablePage(current)) {
    page._navCurrentPage = next;
    page._navHistoryAction = '';
    return;
  }
  if (next === current) {
    page._navCurrentPage = next;
    page._navHistoryAction = '';
    return;
  }
  if (action === 'back') {
    if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
  } else if (action === 'forward') {
    if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
  } else if (back.length > 0 && back[back.length - 1] === next) {
    back.pop();
    if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
  } else if (forward.length > 0 && forward[forward.length - 1] === next) {
    forward.pop();
    if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
  } else {
    if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
    forward = [];
  }
  page.navBackStack = trim(back);
  page.navForwardStack = trim(forward);
  page._navCurrentPage = next;
  page._navHistoryAction = '';
}

function infringCanNavigateBack(page) {
  return Array.isArray(page.navBackStack) && page.navBackStack.length > 0;
}

function infringCanNavigateForward(page) {
  return Array.isArray(page.navForwardStack) && page.navForwardStack.length > 0;
}

function infringNavigateBackPage(page) {
  if (!page.canNavigateBack()) return;
  var back = page.navBackStack.slice();
  var target = page.normalizeNavigablePage(back.pop());
  page.navBackStack = back;
  page._navHistoryAction = 'back';
  if (!target || target === page.normalizeNavigablePage(page.page)) {
    page._navHistoryAction = '';
    return;
  }
  page.navigate(target);
}

function infringNavigateForwardPage(page) {
  if (!page.canNavigateForward()) return;
  var forward = page.navForwardStack.slice();
  var target = page.normalizeNavigablePage(forward.pop());
  page.navForwardStack = forward;
  page._navHistoryAction = 'forward';
  if (!target || target === page.normalizeNavigablePage(page.page)) {
    page._navHistoryAction = '';
    return;
  }
  page.navigate(target);
}

function infringClearPendingFreshAgentForNavigation(page, targetPage) {
  if (String(targetPage || '') === 'chat') return;
  var store = page.getAppStore();
  var pendingId = String((store && store.pendingFreshAgentId) || '').trim();
  var activeId = String((store && store.activeAgentId) || '').trim();
  if (!pendingId) return;
  if (store) {
    store.pendingFreshAgentId = null;
    store.pendingAgent = null;
    if (pendingId === activeId) {
      if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
      else store.activeAgentId = null;
    }
  }
  page.chatSidebarTopologyOrder = (page.chatSidebarTopologyOrder || []).filter(function(id) {
    return String(id || '').trim() !== pendingId;
  });
  page.persistChatSidebarTopologyOrder();
  InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(pendingId) + '/archive', { reason: 'discard_pending_fresh_agent' }).catch(function() {});
  if (store && typeof store.refreshAgents === 'function') {
    setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
  }
}

function infringNavigate(page, targetPage) {
  if (typeof page.hideDashboardPopupBySource === 'function') page.hideDashboardPopupBySource('sidebar');
  infringClearPendingFreshAgentForNavigation(page, targetPage);
  page.page = targetPage;
  if (typeof page.syncAgentChatsSectionForPage === 'function') {
    page.syncAgentChatsSectionForPage(targetPage);
  }
  if (typeof page.notifyShellAppStore === 'function') page.notifyShellAppStore('navigate');
  window.location.hash = targetPage;
  page.mobileMenuOpen = false;
}
