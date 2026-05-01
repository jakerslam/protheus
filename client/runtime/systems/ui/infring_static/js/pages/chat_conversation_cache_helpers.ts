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
    chatSaveAgentChatPreview(agentId, page.conversationCache[key].messages);
    page.persistConversationCache();
  } catch {}
}

function chatSaveAgentChatPreview(agentId, messages) {
  var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
    ? InfringSharedShellServices.appStore
    : null;
  var saveAgentChatPreview = bridge && typeof bridge.method === 'function'
    ? bridge.method('saveAgentChatPreview')
    : null;
  if (typeof saveAgentChatPreview === 'function') saveAgentChatPreview(agentId, messages);
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

function infringChatConversationCacheDelegateMethods() {
  return {
    resolveConversationInputMode(agentId) {
      return chatResolveConversationInputMode(this, agentId);
    },

    currentConversationInputMode(agentId) {
      return chatCurrentConversationInputMode(this, agentId);
    },

    applyConversationInputMode(agentId, options) {
      return chatApplyConversationInputMode(this, agentId, options);
    },

    sanitizeConversationDraftText(rawText) {
      return chatSanitizeConversationDraftText(rawText);
    },

    conversationCacheMaxEntries: function() {
      return chatConversationCacheMaxEntries();
    },

    pruneConversationCacheEntries: function() {
      chatPruneConversationCacheEntries(this);
    },

    touchConversationCacheEntry: function(agentId, patch) {
      return chatTouchConversationCacheEntry(this, agentId, patch);
    },

    captureConversationDraft(agentId, explicitMode) {
      chatCaptureConversationDraft(this, agentId, explicitMode);
    },

    restoreConversationDraft(agentId, explicitMode) {
      return chatRestoreConversationDraft(this, agentId, explicitMode);
    },

    cacheAgentConversation(agentId) {
      chatCacheAgentConversation(this, agentId);
    },

    saveAgentChatPreview(agentId, messages) {
      return chatSaveAgentChatPreview(agentId, messages);
    },

    cacheCurrentConversation() {
      chatCacheCurrentConversation(this);
    },

    scheduleConversationPersist() {
      chatScheduleConversationPersist(this);
    },
  };
}

function infringChatConversationCachePersistenceMethods() {
  return {
    sanitizeConversationForCache(messages) {
      var source = Array.isArray(messages) ? messages : [];
      var out = [];
      var compactPreviewText = function(rawText, limit) {
        var max = Number(limit || 0);
        if (!Number.isFinite(max) || max < 40) max = 320;
        var text = rawText == null ? '' : String(rawText);
        text = text.replace(/\s+/g, ' ').trim();
        if (!text) return '';
        if (text.length > max) return text.slice(0, Math.max(0, max - 1)).trimEnd() + '\u2026';
        return text;
      };
      var detailRefFor = function(row) {
        if (!row || typeof row !== 'object') return '';
        return String(
          row.detail_ref ||
          row.message_detail_ref ||
          row.tool_detail_ref ||
          row.artifact_detail_ref ||
          row.receipt_ref ||
          row.receipt_id ||
          row.ref ||
          row.id ||
          ''
        ).trim();
      };
      var detailRefsFor = function(rows) {
        var list = Array.isArray(rows) ? rows : [];
        var refs = [];
        var seen = Object.create(null);
        for (var j = 0; j < list.length && refs.length < 8; j += 1) {
          var row = list[j] && typeof list[j] === 'object' ? list[j] : {};
          var ref = String(
            row.detail_ref ||
            row.tool_detail_ref ||
            row.artifact_detail_ref ||
            row.receipt_ref ||
            row.input_ref ||
            row.result_ref ||
            row.id ||
            ''
          ).trim();
          if (!ref || seen[ref]) continue;
          seen[ref] = true;
          refs.push(ref);
        }
        return refs;
      };
      for (var i = 0; i < source.length; i++) {
        var msg = source[i];
        if (!msg || typeof msg !== 'object') continue;
        if (msg.thinking || msg.streaming || (msg.terminal && msg.thinking)) continue;
        var roleRaw = String(msg.role || msg.type || '').trim().toLowerCase();
        if (roleRaw.indexOf('assistant') >= 0) roleRaw = 'agent';
        else if (roleRaw.indexOf('user') >= 0) roleRaw = 'user';
        else if (roleRaw.indexOf('system') >= 0) roleRaw = 'system';
        else if (msg.terminal) roleRaw = 'terminal';
        else roleRaw = roleRaw || 'agent';
        var rawText = msg.content_preview;
        if (rawText == null) rawText = msg.text;
        if (rawText == null) rawText = msg.message;
        if (rawText == null) rawText = msg.assistant;
        if (rawText == null && roleRaw === 'user') rawText = msg.user;
        var contentPreview = compactPreviewText(rawText, 320);
        var rawLineText = rawText == null ? '' : String(rawText);
        var lineCount = rawLineText ? rawLineText.split(/\r?\n/).length : 0;
        if (lineCount > 99) lineCount = 99;
        var tools = Array.isArray(msg.tools) ? msg.tools : [];
        var artifactRows = [];
        if (msg.file_output && typeof msg.file_output === 'object') artifactRows.push(msg.file_output);
        if (msg.folder_output && typeof msg.folder_output === 'object') artifactRows.push(msg.folder_output);
        if (Array.isArray(msg.artifacts)) artifactRows = artifactRows.concat(msg.artifacts);
        var progress = msg.progress && typeof msg.progress === 'object' ? msg.progress : null;
        var preview = {
          id: msg.id,
          role: roleRaw,
          status: String(msg.status || msg.receipt_status || msg.display_state || ''),
          content_preview: contentPreview,
          text: contentPreview,
          line_count: lineCount,
          detail_ref: detailRefFor(msg),
          ts: Number(msg.ts || 0) || Date.now(),
          agent_id: msg.agent_id,
          agent_name: msg.agent_name,
          terminal: msg.terminal === true,
          is_notice: msg.is_notice === true,
          notice_label: compactPreviewText(msg.notice_label, 160),
          notice_type: String(msg.notice_type || ''),
          notice_icon: String(msg.notice_icon || ''),
          notice_action: msg.notice_action,
          progress_percent: progress ? (Number(progress.percent || 0) || 0) : 0,
          progress_label: progress ? compactPreviewText(progress.label, 120) : '',
          tool_summary_count: tools.length,
          tool_detail_refs: detailRefsFor(tools),
          artifact_summary_count: artifactRows.length,
          artifact_detail_refs: detailRefsFor(artifactRows),
        };
        var hasNotice = !!(preview.is_notice || preview.notice_label || preview.notice_type || preview.notice_action);
        var hasText = typeof preview.content_preview === 'string' && preview.content_preview.trim().length > 0;
        var hasTools = preview.tool_summary_count > 0;
        var hasArtifacts = preview.artifact_summary_count > 0;
        var hasProgress = !!(preview.progress_label || preview.progress_percent);
        var hasTerminal = !!preview.terminal;
        if (!hasNotice && !hasText && !hasTools && !hasArtifacts && !hasProgress && !hasTerminal) continue;
        out.push(preview);
      }
      return out;
    },
    restoreAgentConversation(agentId) {
      if (!agentId || !this.conversationCache) return false;
      if (!(typeof this.isSystemThreadId === 'function' && this.isSystemThreadId(agentId))) {
        return false;
      }
      const cached = this.conversationCache[String(agentId)];
      if (!cached || !Array.isArray(cached.messages)) return false;
      var scopeKey = typeof this.resolveConversationCacheScopeKey === 'function'
        ? this.resolveConversationCacheScopeKey(agentId)
        : String(agentId || '').trim();
      var cachedScopeKey = String(cached.session_scope_key || '').trim();
      if (scopeKey && cachedScopeKey && scopeKey !== cachedScopeKey) return false;
      try {
        if (this.applyConversationInputMode) this.applyConversationInputMode(agentId);
        var rawCachedMessages = cached.messages || [];
        var sanitized = this.sanitizeConversationForCache(cached.messages || []);
        var cacheChanged = false;
        try {
          cacheChanged = JSON.stringify(sanitized) !== JSON.stringify(rawCachedMessages);
        } catch(_) {
          cacheChanged = sanitized.length !== rawCachedMessages.length;
        }
        this.messages = this.mergeModelNoticesForAgent(
          agentId,
          this.normalizeSessionMessages({ messages: sanitized })
        );
        this.tokenCount = Number(cached.token_count || 0);
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest();
        if (cacheChanged) {
          this.conversationCache[String(agentId)].messages = sanitized;
          this.persistConversationCache();
        }
        this.recomputeContextEstimate();
        if (typeof this.restoreConversationDraft === 'function') {
          this.restoreConversationDraft(agentId);
        }
        this.$nextTick(() => this.scrollToBottomImmediate());
        return true;

      } catch {
        return false;
      }
    },

    loadConversationCache() {
      try {
        var cacheVersion = localStorage.getItem(this.conversationCacheVersionKey);
        if (cacheVersion !== this.conversationCacheVersion) {
          localStorage.removeItem(this.conversationCacheKey);
          localStorage.setItem(this.conversationCacheVersionKey, this.conversationCacheVersion);
          return {};
        }
        var raw = localStorage.getItem(this.conversationCacheKey);
        if (!raw) return {};
        var parsed = JSON.parse(raw);
        if (!parsed || typeof parsed !== 'object') return {};
        return parsed;
      } catch {
        return {};
      }
    },

    projectConversationCacheForPersistence(cache) {
      var source = cache && typeof cache === 'object' ? cache : {};
      var projected = {};
      var keys = Object.keys(source);
      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];
        var entry = source[key] && typeof source[key] === 'object' ? source[key] : {};
        var next = {
          saved_at: Number(entry.saved_at || 0) || Date.now(),
          session_scope_key: String(entry.session_scope_key || ''),
          session_label: String(entry.session_label || ''),
          token_count: Number(entry.token_count || 0) || 0,
          default_terminal: entry.default_terminal === true,
          cache_shape: 'preview_rows_v1',
          draft_chat: this.sanitizeConversationDraftText(entry.draft_chat),
          draft_terminal: this.sanitizeConversationDraftText(entry.draft_terminal),
          messages: typeof this.sanitizeConversationForCache === 'function'
            ? this.sanitizeConversationForCache(entry.messages || [])
            : []
        };
        projected[key] = next;
      }
      return projected;
    },

    persistConversationCache() {
      try {
        localStorage.setItem(this.conversationCacheVersionKey, this.conversationCacheVersion);
        var projectedCache = this.projectConversationCacheForPersistence(this.conversationCache || {});
        localStorage.setItem(this.conversationCacheKey, JSON.stringify(projectedCache));
      } catch {}
    },
  };
}
