'use strict';

function chatResolveConversationInputMode(page, agentId) {
  var key = String(agentId || '').trim();
  if (!key) return 'chat';
  if (page.isSystemThreadId(key)) return 'terminal';
  var cached = page.conversationCache && page.conversationCache[key];
  return cached && cached.default_terminal === true ? 'terminal' : 'chat';
}

function chatCurrentConversationInputMode(page, agentId) {
  if (page.isSystemThreadId(agentId)) return 'terminal';
  return page.terminalMode ? 'terminal' : 'chat';
}

function chatApplyConversationInputMode(page, agentId, options) {
  var opts = options && typeof options === 'object' ? options : {};
  var hasForced = Object.prototype.hasOwnProperty.call(opts, 'force_terminal');
  var mode = chatResolveConversationInputMode(page, agentId);
  if (hasForced) mode = opts.force_terminal === true ? 'terminal' : 'chat';
  if (page.isSystemThreadId(agentId)) mode = 'terminal';
  page.terminalMode = mode === 'terminal';
  page.showSlashMenu = false;
  page.showModelPicker = false;
  page.showModelSwitcher = false;
  page.terminalCursorFocused = false;
  if (!page.terminalMode) page.terminalSelectionStart = 0;
  if (page.terminalMode && !page.terminalCwd) page.terminalCwd = '/workspace';
  return mode;
}

function chatSanitizeConversationDraftText(rawText) {
  var text = String(rawText == null ? '' : rawText);
  if (!text) return '';
  if (text.length > 12000) text = text.slice(0, 12000);
  var trimmed = text.trim();
  if (!trimmed) return '';
  if (/^message\s+.+\.\.\.(?:\s+\(\/\s*for commands\))?$/i.test(trimmed)) return '';
  if (/^tell\s+.+\.\.\.$/i.test(trimmed)) return '';
  return text;
}

function chatConversationCacheMaxEntries() {
  return 20;
}

function chatPruneConversationCacheEntries(page) {
  if (!page.conversationCache || typeof page.conversationCache !== 'object') return;
  var keys = Object.keys(page.conversationCache || {});
  var maxEntries = Number(page.conversationCacheMaxEntries ? page.conversationCacheMaxEntries() : 20);
  if (!Number.isFinite(maxEntries) || maxEntries < 1) maxEntries = 20;
  if (keys.length <= maxEntries) return;
  keys.sort(function(left, right) {
    var a = page.conversationCache[left] && typeof page.conversationCache[left] === 'object'
      ? Number(page.conversationCache[left].saved_at || 0)
      : 0;
    var b = page.conversationCache[right] && typeof page.conversationCache[right] === 'object'
      ? Number(page.conversationCache[right].saved_at || 0)
      : 0;
    return b - a;
  });
  var next = {};
  for (var i = 0; i < keys.length && i < maxEntries; i += 1) {
    next[keys[i]] = page.conversationCache[keys[i]];
  }
  page.conversationCache = next;
}

function chatTouchConversationCacheEntry(page, agentId, patch) {
  var key = String(agentId || '').trim();
  if (!key) return null;
  if (!page.conversationCache || typeof page.conversationCache !== 'object') page.conversationCache = {};
  var prior = page.conversationCache[key] && typeof page.conversationCache[key] === 'object'
    ? page.conversationCache[key]
    : {};
  var next = Object.assign({}, prior, patch || {}, { saved_at: Date.now() });
  page.conversationCache[key] = next;
  chatPruneConversationCacheEntries(page);
  return page.conversationCache[key];
}

function chatCaptureConversationDraft(page, agentId, explicitMode) {
  var key = String(agentId || '').trim();
  if (!key) return;
  if (!page.conversationCache) page.conversationCache = {};
  var mode = String(explicitMode || chatCurrentConversationInputMode(page, key) || 'chat').trim().toLowerCase();
  if (mode !== 'terminal') mode = 'chat';
  var next = chatTouchConversationCacheEntry(page, key) || {};
  var scopeKey = typeof page.resolveConversationCacheScopeKey === 'function'
    ? page.resolveConversationCacheScopeKey(key)
    : key;
  next.session_scope_key = scopeKey;
  var sanitized = chatSanitizeConversationDraftText(page.inputText);
  if (mode === 'terminal') next.draft_terminal = sanitized;
  else next.draft_chat = sanitized;
  page.conversationCache[key] = next;
  page.persistConversationCache();
}

function chatRestoreConversationDraft(page, agentId, explicitMode) {
  var key = String(agentId || '').trim();
  if (!key || !page.conversationCache) {
    page.inputText = '';
    return '';
  }
  var cached = page.conversationCache[key];
  if (!cached || typeof cached !== 'object') {
    page.inputText = '';
    return '';
  }
  var scopeKey = typeof page.resolveConversationCacheScopeKey === 'function'
    ? page.resolveConversationCacheScopeKey(key)
    : key;
  var cachedScopeKey = String(cached.session_scope_key || '').trim();
  if (scopeKey && cachedScopeKey && scopeKey !== cachedScopeKey) {
    page.inputText = '';
    return '';
  }
  var mode = String(explicitMode || chatCurrentConversationInputMode(page, key) || 'chat').trim().toLowerCase();
  if (mode !== 'terminal') mode = 'chat';
  var raw = mode === 'terminal' ? cached.draft_terminal : cached.draft_chat;
  var nextText = chatSanitizeConversationDraftText(raw);
  chatTouchConversationCacheEntry(page, key);
  page.inputText = nextText;
  page.$nextTick(function() {
    var el = document.getElementById('msg-input');
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = Math.min(el.scrollHeight, 150) + 'px';
    if (page.terminalMode) page.updateTerminalCursor({ target: el });
  });
  return nextText;
}

function chatCacheAgentConversation(page, agentId) {
  if (!agentId) return;
  if (!page.conversationCache) page.conversationCache = {};
  try {
    var key = String(agentId);
    var scopeKey = typeof page.resolveConversationCacheScopeKey === 'function'
      ? page.resolveConversationCacheScopeKey(agentId)
      : key;
    var currentSessionRow = typeof page.resolveCurrentSessionRow === 'function'
      ? page.resolveCurrentSessionRow(agentId)
      : null;
    var cachedMessages = page.sanitizeConversationForCache(page.messages || []);
    var next = Object.assign(
      {},
      chatTouchConversationCacheEntry(page, key),
      {
        saved_at: Date.now(),
        session_scope_key: scopeKey,
        session_label: typeof page.resolveSessionRowLabel === 'function'
          ? page.resolveSessionRowLabel(currentSessionRow, agentId)
          : '',
        token_count: page.tokenCount || 0,
        default_terminal: chatCurrentConversationInputMode(page, agentId) === 'terminal',
        messages: cachedMessages,
      }
    );
    var mode = chatCurrentConversationInputMode(page, agentId);
    var draft = chatSanitizeConversationDraftText(page.inputText);
    if (mode === 'terminal') next.draft_terminal = draft;
    else next.draft_chat = draft;
    page.conversationCache[key] = next;
    var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
      ? InfringSharedShellServices.appStore
      : null;
    var saveAgentChatPreview = bridge && typeof bridge.method === 'function'
      ? bridge.method('saveAgentChatPreview')
      : null;
    if (typeof saveAgentChatPreview === 'function') saveAgentChatPreview(agentId, page.conversationCache[key].messages);
    page.persistConversationCache();
  } catch {}
}

function chatCacheCurrentConversation(page) {
  if (!page.currentAgent || !page.currentAgent.id) return;
  chatCacheAgentConversation(page, page.currentAgent.id);
}

function chatScheduleConversationPersist(page) {
  if (page._persistTimer) clearTimeout(page._persistTimer);
  page._persistTimer = setTimeout(function() {
    chatCacheCurrentConversation(page);
  }, 80);
}
