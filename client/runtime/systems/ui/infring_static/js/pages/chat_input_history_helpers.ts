'use strict';

function chatInputHistoryMode(page, explicitMode) {
  var mode = String(explicitMode || (page.terminalMode ? 'terminal' : 'chat')).trim().toLowerCase();
  return mode === 'terminal' ? 'terminal' : 'chat';
}

function chatInputHistoryLimit(page) {
  var maxEntries = Number(page.inputHistoryMaxEntries || 0);
  if (!Number.isFinite(maxEntries) || maxEntries < 20) maxEntries = 120;
  if (maxEntries > 500) maxEntries = 500;
  return maxEntries;
}

function chatNormalizeInputHistoryEntry(value) {
  return String(value == null ? '' : value).trim();
}

function chatNormalizeInputHistoryRows(page, rows) {
  var source = Array.isArray(rows) ? rows : [];
  var clean = [];
  for (var i = 0; i < source.length; i += 1) {
    var item = chatNormalizeInputHistoryEntry(source[i]);
    if (!item) continue;
    if (clean.length && clean[clean.length - 1] === item) continue;
    clean.push(item);
  }
  var maxEntries = chatInputHistoryLimit(page);
  if (clean.length > maxEntries) clean = clean.slice(clean.length - maxEntries);
  return clean;
}

function chatInputHistoryLegacyAgentKey(page, explicitAgentId) {
  var direct = String(explicitAgentId || '').trim();
  if (direct) return direct;
  var active = page.currentAgent && page.currentAgent.id ? String(page.currentAgent.id) : '';
  return String(active || '').trim();
}

function chatInputHistorySessionScopeKey(page, explicitAgentId) {
  var agentId = chatInputHistoryLegacyAgentKey(page, explicitAgentId);
  if (!agentId) return '';
  var scopeKey = '';
  if (typeof page.resolveConversationCacheScopeKey === 'function') {
    try {
      scopeKey = String(page.resolveConversationCacheScopeKey(agentId) || '').trim();
    } catch (_) {
      scopeKey = '';
    }
  }
  if (!scopeKey) scopeKey = agentId + '|main';
  var prefix = String(page.inputHistorySessionScopePrefix || 'session:').trim() || 'session:';
  return prefix + scopeKey;
}

function chatInputHistoryAgentKey(page, explicitAgentId) {
  var scoped = chatInputHistorySessionScopeKey(page, explicitAgentId);
  if (scoped) return scoped;
  return chatInputHistoryLegacyAgentKey(page, explicitAgentId);
}

function chatInputHistoryBucketRows(page, cache, agentKey, legacyKey, mode) {
  var buckets = [];
  if (cache && agentKey && cache[agentKey] && typeof cache[agentKey] === 'object') {
    buckets.push(cache[agentKey]);
  }
  if (
    cache &&
    legacyKey &&
    legacyKey !== agentKey &&
    (!buckets.length || !Array.isArray(mode === 'terminal' ? buckets[0].terminal : buckets[0].chat) || !(mode === 'terminal' ? buckets[0].terminal : buckets[0].chat).length) &&
    cache[legacyKey] &&
    typeof cache[legacyKey] === 'object'
  ) {
    buckets.push(cache[legacyKey]);
  }
  for (var i = 0; i < buckets.length; i += 1) {
    var bucket = buckets[i];
    var rows = mode === 'terminal' ? bucket.terminal : bucket.chat;
    if (Array.isArray(rows) && rows.length) return chatNormalizeInputHistoryRows(page, rows);
  }
  return [];
}

function chatLoadInputHistoryCache(page) {
  var empty = {};
  try {
    var raw = localStorage.getItem(page.inputHistoryCacheKey);
    if (!raw) {
      page._inputHistoryByAgent = empty;
      return;
    }
    var parsed = JSON.parse(raw);
    var next = parsed && typeof parsed === 'object' ? parsed : empty;
    var normalized = {};
    var keys = Object.keys(next);
    for (var i = 0; i < keys.length; i += 1) {
      var key = String(keys[i] || '').trim();
      if (!key) continue;
      var bucket = next[key];
      if (!bucket || typeof bucket !== 'object') continue;
      normalized[key] = {
        chat: chatNormalizeInputHistoryRows(page, bucket.chat),
        terminal: chatNormalizeInputHistoryRows(page, bucket.terminal),
        updated_at: Number(bucket.updated_at || 0) || 0,
      };
    }
    page._inputHistoryByAgent = normalized;
  } catch (_) {
    page._inputHistoryByAgent = empty;
  }
}

function chatPersistInputHistoryCache(page) {
  try {
    var payload = page._inputHistoryByAgent && typeof page._inputHistoryByAgent === 'object'
      ? page._inputHistoryByAgent
      : {};
    localStorage.setItem(page.inputHistoryCacheKey, JSON.stringify(payload));
  } catch (_) {}
}

function chatInputHistoryEntries(page, explicitMode) {
  var mode = chatInputHistoryMode(page, explicitMode);
  return mode === 'terminal' ? page.terminalInputHistory : page.chatInputHistory;
}

function chatHydrateInputHistoryFromCache(page, explicitMode, explicitAgentId) {
  var mode = chatInputHistoryMode(page, explicitMode);
  var rows = chatInputHistoryEntries(page, mode);
  if (!Array.isArray(rows)) return;
  var agentKey = chatInputHistoryAgentKey(page, explicitAgentId);
  if (!agentKey) return;
  var legacyKey = chatInputHistoryLegacyAgentKey(page, explicitAgentId);
  var cache = page._inputHistoryByAgent && typeof page._inputHistoryByAgent === 'object'
    ? page._inputHistoryByAgent
    : {};
  var cachedRows = chatInputHistoryBucketRows(page, cache, agentKey, legacyKey, mode);
  if (!Array.isArray(cachedRows) || !cachedRows.length) return;
  var merged = chatNormalizeInputHistoryRows(page, rows.concat(cachedRows));
  if (mode === 'terminal') page.terminalInputHistory = merged;
  else page.chatInputHistory = merged;
}

function chatSyncInputHistoryToCache(page, explicitMode, explicitAgentId) {
  var mode = chatInputHistoryMode(page, explicitMode);
  var rows = chatInputHistoryEntries(page, mode);
  if (!Array.isArray(rows)) return;
  var agentKey = chatInputHistoryAgentKey(page, explicitAgentId);
  if (!agentKey) return;
  if (!page._inputHistoryByAgent || typeof page._inputHistoryByAgent !== 'object') {
    page._inputHistoryByAgent = {};
  }
  var bucket = page._inputHistoryByAgent[agentKey] && typeof page._inputHistoryByAgent[agentKey] === 'object'
    ? page._inputHistoryByAgent[agentKey]
    : {};
  var cleanRows = chatNormalizeInputHistoryRows(page, rows);
  if (mode === 'terminal') bucket.terminal = cleanRows;
  else bucket.chat = cleanRows;
  bucket.updated_at = Date.now();
  page._inputHistoryByAgent[agentKey] = bucket;
  chatPersistInputHistoryCache(page);
}

function chatResetInputHistoryNavigation(page, explicitMode) {
  var mode = chatInputHistoryMode(page, explicitMode);
  if (mode === 'terminal') {
    page.terminalInputHistoryCursor = -1;
    page.terminalInputHistoryDraft = '';
    return;
  }
  page.chatInputHistoryCursor = -1;
  page.chatInputHistoryDraft = '';
}

function chatPushInputHistoryEntry(page, explicitMode, rawText) {
  var text = chatNormalizeInputHistoryEntry(rawText);
  if (!text) return;
  var mode = chatInputHistoryMode(page, explicitMode);
  var rows = chatInputHistoryEntries(page, mode);
  if (!Array.isArray(rows)) return;
  if (rows.length && String(rows[rows.length - 1] || '') === text) {
    chatResetInputHistoryNavigation(page, mode);
    return;
  }
  var nextRows = chatNormalizeInputHistoryRows(page, rows.concat([text]));
  rows.splice(0, rows.length);
  for (var i = 0; i < nextRows.length; i += 1) rows.push(nextRows[i]);
  chatSyncInputHistoryToCache(page, mode);
  chatResetInputHistoryNavigation(page, mode);
}

function chatNavigateInputHistory(page, direction, event) {
  var step = Number(direction || 0);
  if (!Number.isFinite(step) || step === 0) return false;
  var mode = chatInputHistoryMode(page);
  var rows = chatInputHistoryEntries(page, mode);
  if (!Array.isArray(rows) || !rows.length) return false;
  var cursor = mode === 'terminal' ? Number(page.terminalInputHistoryCursor || -1) : Number(page.chatInputHistoryCursor || -1);
  if (!Number.isFinite(cursor)) cursor = -1;
  var draft = mode === 'terminal'
    ? String(page.terminalInputHistoryDraft || '')
    : String(page.chatInputHistoryDraft || '');

  var nextText = '';
  if (step < 0) {
    if (cursor < 0) {
      draft = String(page.inputText || '');
      cursor = rows.length - 1;
    } else {
      cursor = Math.max(0, cursor - 1);
    }
    nextText = String(rows[cursor] || '');
  } else {
    if (cursor < 0) {
      return false;
    } else if (cursor >= rows.length - 1) {
      cursor = -1;
      nextText = draft;
    } else {
      cursor += 1;
      nextText = String(rows[cursor] || '');
    }
  }

  if (mode === 'terminal') {
    page.terminalInputHistoryCursor = cursor;
    page.terminalInputHistoryDraft = draft;
  } else {
    page.chatInputHistoryCursor = cursor;
    page.chatInputHistoryDraft = draft;
  }

  page._inputHistoryApplying = true;
  page.inputText = nextText;
  page.$nextTick(function() {
    var el = document.getElementById('msg-input');
    if (el) {
      var pos = String(page.inputText || '').length;
      if (typeof el.setSelectionRange === 'function') {
        try { el.setSelectionRange(pos, pos); } catch (_) {}
      }
      el.style.height = 'auto';
      el.style.height = Math.min(el.scrollHeight, 150) + 'px';
    }
    if (page.terminalMode) page.updateTerminalCursor({ target: el });
    page._inputHistoryApplying = false;
  });
  if (event && typeof event.preventDefault === 'function') event.preventDefault();
  return true;
}
