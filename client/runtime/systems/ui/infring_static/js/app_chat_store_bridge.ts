// Canonical Shell helper source: chat page projection store bridge.
// Loaded before app.ts by the dashboard asset router.

function infringEnsureChatStoreBridge() {
  if (typeof window === 'undefined') return null;
  if (window.InfringChatStore && typeof window.InfringChatStore.syncMessages === 'function') return window.InfringChatStore;
  function writable(initialValue) {
    var value = initialValue;
    var subscribers = [];
    return {
      subscribe: function(run) {
        if (typeof run !== 'function') return function() {};
        subscribers.push(run);
        try { run(value); } catch (_) {}
        return function() { subscribers = subscribers.filter(function(row) { return row !== run; }); };
      },
      set: function(next) {
        value = next;
        subscribers.slice().forEach(function(run) { try { run(value); } catch (_) {} });
      },
      update: function(fn) { if (typeof fn === 'function') this.set(fn(value)); },
      get: function() { return value; }
    };
  }
  function chatPage() { return window.InfringChatPage || null; }
  function callPage(fn) {
    var page = chatPage();
    if (!page || typeof page[fn] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return page[fn].apply(page, args); } catch (_) { return undefined; }
  }
  function fallbackDayKey(msg) {
    if (!msg || !msg.ts) return '';
    var day = new Date(msg.ts);
    if (Number.isNaN(day.getTime())) return '';
    return day.getFullYear() + '-' + String(day.getMonth() + 1).padStart(2, '0') + '-' + String(day.getDate()).padStart(2, '0');
  }
  function mapMarkerType(msg) {
    var fromPage = callPage('messageMapMarkerType', msg);
    if (fromPage != null) return String(fromPage || '');
    if (msg && msg.is_notice) return String(msg.notice_type || '').toLowerCase() === 'info' ? 'info' : 'model';
    if (msg && msg.terminal) return 'terminal';
    if (msg && Array.isArray(msg.tools) && msg.tools.length) return 'tool';
    return '';
  }
  function mapToolOutcome(msg) {
    var fromPage = callPage('messageMapToolOutcome', msg);
    if (fromPage != null) return String(fromPage || '');
    if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
    for (var i = 0; i < msg.tools.length; i += 1) {
      var tool = msg.tools[i] || {};
      if (tool.is_error) return 'error';
      if (tool.running) return 'warning';
    }
    return 'success';
  }
  function buildMapRows(rows) {
    var list = Array.isArray(rows) ? rows : [];
    var out = [];
    for (var i = 0; i < list.length; i += 1) {
      var msg = list[i] || {};
      var dayKey = String(callPage('messageDayKey', msg) || fallbackDayKey(msg) || '');
      var prevDayKey = i > 0 ? String(callPage('messageDayKey', list[i - 1]) || fallbackDayKey(list[i - 1]) || '') : '';
      out.push({
        index: i,
        key: String(callPage('messageRenderKey', msg, i) || msg.id || msg.ts || i),
        domId: String(callPage('messageDomId', msg, i) || ('message-' + i)),
        role: String(msg.role || 'agent').trim() || 'agent',
        isNotice: !!msg.is_notice,
        noticeIcon: String(msg.notice_icon || 'i'),
        newDay: i === 0 || (!!dayKey && dayKey !== prevDayKey),
        dayKey: dayKey,
        dayLabel: String(callPage('messageDayLabel', msg) || dayKey || 'Unknown day'),
        dayCollapsed: !!callPage('isMessageDayCollapsed', msg),
        markerType: mapMarkerType(msg),
        markerTitle: String(callPage('messageMapMarkerTitle', msg) || ''),
        toolOutcome: mapToolOutcome(msg),
        longMessage: !!callPage('isLongMessagePreview', msg)
      });
    }
    return out;
  }
  var queuedMessageSync = false;
  var pendingMessages = [];
  var pendingFilteredMessages = [];
  var lastFilteredMessageSource = [];
  var threadProjectionCenterIndex = -1;
  var threadProjectionLimit = 80;
  function scheduleMessageStoreFlush(store) {
    if (queuedMessageSync) return;
    queuedMessageSync = true;
    var flush = function() {
      queuedMessageSync = false;
      store.messages.set(pendingMessages);
      store.filteredMessages.set(pendingFilteredMessages);
    };
    if (typeof queueMicrotask === 'function') return queueMicrotask(flush);
    Promise.resolve().then(flush).catch(function() { setTimeout(flush, 0); });
  }
  function projectThreadMessages(rows) {
    var list = Array.isArray(rows) ? rows : [];
    if (list.length <= threadProjectionLimit) return list;
    var center = Number(threadProjectionCenterIndex);
    if (!Number.isFinite(center) || center < 0) center = list.length - 1;
    center = Math.max(0, Math.min(list.length - 1, Math.round(center)));
    var before = Math.floor(threadProjectionLimit * 0.45);
    var start = Math.max(0, center - before);
    var end = Math.min(list.length, start + threadProjectionLimit);
    start = Math.max(0, end - threadProjectionLimit);
    return list.slice(start, end);
  }
  var store = {
    messages: writable([]),
    filteredMessages: writable([]),
    currentAgent: writable(null),
    agents: writable([]),
    sidebarAgents: writable([]),
    sessionLoading: writable(false),
    sending: writable(false),
    tokenCount: writable(0),
    inputText: writable(''),
    wsConnected: writable(false),
    showScrollDown: writable(false),
    stickToBottom: writable(true),
    mapStepIndex: writable(-1),
    mapRows: writable([]),
    renderWindowVersion: writable(0),
    focusMode: writable(false),
    connectionState: writable(''),
    theme: writable(''),
    sessions: writable([])
  };
  store.syncMessages = function(messages, filteredMessages) {
    store.mapRows.set(buildMapRows(messages));
    lastFilteredMessageSource = Array.isArray(filteredMessages) ? filteredMessages : [];
    pendingFilteredMessages = projectThreadMessages(lastFilteredMessageSource);
    pendingMessages = pendingFilteredMessages;
    scheduleMessageStoreFlush(store);
  };
  store.refreshMapRows = function(messages) { store.mapRows.set(buildMapRows(messages)); };
  store.setThreadProjectionCenter = function(index) {
    var next = Number(index);
    if (!Number.isFinite(next)) next = -1;
    next = Math.round(next);
    if (next === threadProjectionCenterIndex) return;
    threadProjectionCenterIndex = next;
    pendingFilteredMessages = projectThreadMessages(lastFilteredMessageSource);
    pendingMessages = pendingFilteredMessages;
    scheduleMessageStoreFlush(store);
  };
  store.bumpRenderWindowVersion = function() {
    store.renderWindowVersion.update(function(value) {
      var next = Number(value || 0) + 1;
      return Number.isFinite(next) ? next : 1;
    });
  };
  window.InfringChatStore = store;
  return store;
}

infringEnsureChatStoreBridge();
