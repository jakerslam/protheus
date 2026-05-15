'use strict';

function chatMessageDisplayScopeKey(page) {
  var agentId = String((page.currentAgent && page.currentAgent.id) || '').trim();
  var sessionId = '';
  if (Array.isArray(page.sessions)) {
    for (var i = 0; i < page.sessions.length; i += 1) {
      var row = page.sessions[i];
      if (row && row.active) {
        sessionId = String((row.session_id || row.id || '')).trim();
        break;
      }
    }
  }
  var search = String(page.searchQuery || '').trim().toLowerCase();
  return agentId + '|' + sessionId + '|' + search;
}

function chatEnsureMessageDisplayWindow(page, totalCount) {
  var total = Number(totalCount || 0);
  if (!Number.isFinite(total) || total < 0) total = 0;
  var key = chatMessageDisplayScopeKey(page);
  if (String(page._messageDisplayKey || '') !== key) {
    page._messageDisplayKey = key;
    page.messageDisplayCount = Number(page.messageDisplayInitialLimit || 10);
  }
  var rawQuery = String(page.searchQuery || '').trim();
  if (!rawQuery) {
    page.messageDisplayCount = total;
    return;
  }
  var base = Number(page.messageDisplayInitialLimit || 10);
  if (!Number.isFinite(base) || base < 1) base = 10;
  if (!Number.isFinite(Number(page.messageDisplayCount))) {
    page.messageDisplayCount = base;
  }
  if (page.messageDisplayCount < base) page.messageDisplayCount = base;
  if (page.messageDisplayCount > total) page.messageDisplayCount = total;
}

function chatCanExpandDisplayedMessages(page) {
  var total = Array.isArray(page.allFilteredMessages) ? page.allFilteredMessages.length : 0;
  chatEnsureMessageDisplayWindow(page, total);
  return total > Number(page.messageDisplayCount || 0);
}

function chatExpandRemainingCount(page) {
  var total = Array.isArray(page.allFilteredMessages) ? page.allFilteredMessages.length : 0;
  var visible = Number(page.messageDisplayCount || 0);
  if (!Number.isFinite(visible)) visible = 0;
  return Math.max(0, total - visible);
}

function chatExpandDisplayedMessages(page) {
  var total = Array.isArray(page.allFilteredMessages) ? page.allFilteredMessages.length : 0;
  chatEnsureMessageDisplayWindow(page, total);
  if (total <= Number(page.messageDisplayCount || 0)) return;
  var step = Number(page.messageDisplayStep || 5);
  if (!Number.isFinite(step) || step < 1) step = 5;
  page.messageDisplayCount = Math.min(total, Number(page.messageDisplayCount || 0) + step);
}

function chatAllFilteredMessages(page) {
  var query = String(page.searchQuery || '').trim();
  if (!query) return page.messages;
  if (
    typeof page.shouldUseGatewaySearch === 'function' &&
    page.shouldUseGatewaySearch(query) &&
    String(page.gatewaySearchQuery || '') === query &&
    Array.isArray(page.gatewaySearchResultMessages)
  ) {
    return page.gatewaySearchResultMessages;
  }
  var filtered = page.messages.filter(function(m) {
    if (typeof page.messageMatchesSearchQuery === 'function') return page.messageMatchesSearchQuery(m, query);
    var text = typeof (m && m.text) === 'string' ? m.text : String((m && m.text) || '');
    return text.toLowerCase().indexOf(query.toLowerCase()) !== -1;
  });
  if (filtered.length > 0) return filtered;
  if (!page.searchOpen && Array.isArray(page.messages) && page.messages.length > 0) {
    return page.messages;
  }
  return filtered;
}

function chatFilteredMessages(page) {
  var all = Array.isArray(page.allFilteredMessages) ? page.allFilteredMessages : [];
  chatEnsureMessageDisplayWindow(page, all.length);
  if (!all.length) return all;
  var visible = Number(page.messageDisplayCount || 0);
  if (!Number.isFinite(visible) || visible < 1 || visible >= all.length) return all;
  return all.slice(Math.max(0, all.length - visible));
}
