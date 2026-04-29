function infringIsChatSidebarSearchActive(page) {
  return String(page.chatSidebarQuery || '').trim().length > 0;
}

function infringClearChatSidebarSearch(page) {
  if (page._chatSidebarSearchTimer) {
    clearTimeout(page._chatSidebarSearchTimer);
    page._chatSidebarSearchTimer = 0;
  }
  page.chatSidebarSearchSeq = Number(page.chatSidebarSearchSeq || 0) + 1;
  page.chatSidebarSearchLoading = false;
  page.chatSidebarSearchError = '';
  page.chatSidebarSearchResults = [];
  page.scheduleSidebarScrollIndicators();
}

function infringOnChatSidebarQueryInput(page, value) {
  page.chatSidebarQuery = String(value || '');
  page.chatSidebarVisibleCount = Math.max(1, Math.floor(Number(page.chatSidebarVisibleBase || 7)));
  var query = String(page.chatSidebarQuery || '').trim();
  if (!query) {
    page.clearChatSidebarSearch();
    return;
  }
  page.scheduleChatSidebarSearch();
}

function infringScheduleChatSidebarSearch(page) {
  var query = String(page.chatSidebarQuery || '').trim();
  if (!query) {
    page.clearChatSidebarSearch();
    return;
  }
  if (page._chatSidebarSearchTimer) {
    clearTimeout(page._chatSidebarSearchTimer);
    page._chatSidebarSearchTimer = 0;
  }
  var seq = Number(page.chatSidebarSearchSeq || 0) + 1;
  page.chatSidebarSearchSeq = seq;
  page.chatSidebarSearchLoading = true;
  page.chatSidebarSearchError = '';
  page._chatSidebarSearchTimer = setTimeout(function() {
    page._chatSidebarSearchTimer = 0;
    page.runChatSidebarSearch(seq);
  }, 140);
}

function infringProjectChatSidebarSearchRows(page, rows) {
  var sourceRows = Array.isArray(rows) ? rows : [];
  return sourceRows.filter(function(agent) {
    return !page.isSidebarArchivedAgent(agent);
  }).map(function(agent) {
    return page.sanitizeSidebarAgentRow(agent);
  });
}

async function infringRunChatSidebarSearch(page, seq) {
  var token = Number(seq || 0);
  var currentToken = Number(page.chatSidebarSearchSeq || 0);
  if (token !== currentToken) return;
  var query = String(page.chatSidebarQuery || '').trim();
  if (!query) {
    page.clearChatSidebarSearch();
    return;
  }
  try {
    var path = '/api/search/conversations?q=' + encodeURIComponent(query) + '&limit=80';
    var payload = await InfringAPI.get(path);
    if (token !== Number(page.chatSidebarSearchSeq || 0)) return;
    var serverRows = payload && Array.isArray(payload.sidebar_rows) ? payload.sidebar_rows : null;
    if (serverRows && serverRows.length) {
      page.chatSidebarSearchResults = infringProjectChatSidebarSearchRows(page, serverRows);
      page.chatSidebarSearchError = '';
      return;
    }
    var quickRows = payload && Array.isArray(payload.quick_actions) ? payload.quick_actions : [];
    page.chatSidebarSearchResults = infringProjectChatSidebarSearchRows(page, quickRows);
    page.chatSidebarSearchError = '';
  } catch (e) {
    if (token !== Number(page.chatSidebarSearchSeq || 0)) return;
    page.chatSidebarSearchResults = [];
    page.chatSidebarSearchError = String(e && e.message ? e.message : 'search_failed');
  } finally {
    if (token === Number(page.chatSidebarSearchSeq || 0)) {
      page.chatSidebarSearchLoading = false;
    }
    page.scheduleSidebarScrollIndicators();
  }
}
