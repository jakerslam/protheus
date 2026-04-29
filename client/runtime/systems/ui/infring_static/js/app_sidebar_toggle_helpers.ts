function infringResolveMessagesHost() {
  var nodes = document.querySelectorAll('#messages');
  for (var ni = 0; ni < nodes.length; ni++) {
    if (nodes[ni] && nodes[ni].offsetParent !== null) return nodes[ni];
  }
  return nodes && nodes.length ? nodes[0] : null;
}

function infringSidebarMessageAlignY(host) {
  var hostRect = host.getBoundingClientRect();
  var input = document.getElementById('msg-input');
  var alignY = hostRect.bottom;
  if (input && input.offsetParent !== null) {
    var inputRect = input.getBoundingClientRect();
    if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
  }
  return {
    hostRect: hostRect,
    alignY: alignY
  };
}

function infringCaptureSidebarMessageBottomAnchor() {
  var host = infringResolveMessagesHost();
  if (!host || host.offsetParent === null) return null;
  var projection = infringSidebarMessageAlignY(host);
  var hostRect = projection.hostRect;
  var alignY = projection.alignY;
  var rows = host.querySelectorAll('.chat-message-block[id], .chat-message-block .message[id]');
  var best = null;
  var bestDiff = Number.POSITIVE_INFINITY;
  for (var i = 0; i < rows.length; i++) {
    var row = rows[i];
    if (!row || row.offsetParent === null) continue;
    var rect = row.getBoundingClientRect();
    if (rect.bottom < (hostRect.top - 40) || rect.top > (hostRect.bottom + 40)) continue;
    var diff = Math.abs(rect.bottom - alignY);
    if (diff < bestDiff) {
      bestDiff = diff;
      best = row;
    }
  }
  return best && best.id ? { id: String(best.id) } : null;
}

function infringRestoreSidebarMessageAnchor(anchor) {
  var passes = 4;
  var restoreAnchor = function() {
    var host = infringResolveMessagesHost();
    if (!host || host.offsetParent === null || !anchor || !anchor.id) return;
    var row = document.getElementById(anchor.id);
    if (!row || !host.contains(row) || row.offsetParent === null) return;
    var projection = infringSidebarMessageAlignY(host);
    var hostRect = projection.hostRect;
    var alignY = projection.alignY;
    var alignOffset = Math.max(0, Math.min(Math.max(0, Number(host.clientHeight || 0)), Math.round(alignY - hostRect.top)));
    var rowBottom = Number(row.offsetTop || 0) + Math.max(0, Number(row.offsetHeight || 0));
    var maxTop = Math.max(0, Number(host.scrollHeight || 0) - Math.max(0, Number(host.clientHeight || 0)));
    var nextTop = Math.max(0, Math.min(maxTop, Math.round(rowBottom - alignOffset)));
    host.scrollTop = nextTop;
    if (passes-- > 1 && typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
    try {
      host.dispatchEvent(new Event('scroll'));
    } catch (_) {}
  };
  if (typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
  else setTimeout(restoreAnchor, 0);
}

function infringToggleSidebar(page) {
  if (typeof page.shouldSuppressSidebarToggle === 'function' && page.shouldSuppressSidebarToggle()) return;
  var nextCollapsed = !page.sidebarCollapsed;
  if (nextCollapsed) page._sidebarChatAnchorForExpand = infringCaptureSidebarMessageBottomAnchor();
  page.sidebarCollapsed = nextCollapsed;
  localStorage.setItem('infring-sidebar', page.sidebarCollapsed ? 'collapsed' : 'expanded');
  page.hideDashboardPopupBySource('sidebar');
  if (!nextCollapsed) {
    var anchor = (page._sidebarChatAnchorForExpand && page._sidebarChatAnchorForExpand.id)
      ? page._sidebarChatAnchorForExpand
      : infringCaptureSidebarMessageBottomAnchor();
    page._sidebarChatAnchorForExpand = null;
    infringRestoreSidebarMessageAnchor(anchor);
  }
  page.scheduleSidebarScrollIndicators();
}
