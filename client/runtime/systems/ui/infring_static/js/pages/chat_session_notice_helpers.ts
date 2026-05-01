'use strict';

function chatSessionNoticeMemoryStorageKey(scopeKey) {
  var normalized = String(scopeKey || '').trim();
  if (!normalized) return '';
  return 'of-chat-session-notices-v1:' + normalized;
}

function chatLoadSessionNoticeMemory(scopeKey) {
  var storageKey = chatSessionNoticeMemoryStorageKey(scopeKey);
  if (!storageKey) return {};
  try {
    var raw = localStorage.getItem(storageKey);
    if (!raw) return {};
    var parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return {};
    var next = {};
    for (var i = 0; i < parsed.length; i += 1) {
      var key = String(parsed[i] || '').trim();
      if (key) next[key] = true;
    }
    return next;
  } catch (_) {
    return {};
  }
}

function chatSaveSessionNoticeMemory(scopeKey, nextMemory) {
  var storageKey = chatSessionNoticeMemoryStorageKey(scopeKey);
  if (!storageKey) return;
  var rows = Object.keys(nextMemory || {}).filter(function(key) {
    return !!nextMemory[key];
  });
  try {
    if (!rows.length) {
      localStorage.removeItem(storageKey);
      return;
    }
    localStorage.setItem(storageKey, JSON.stringify(rows));
  } catch (_) {}
}

function chatHasSeenSessionNotice(scopeKey, noticeKey) {
  var normalizedNoticeKey = String(noticeKey || '').trim();
  if (!normalizedNoticeKey) return false;
  var memory = chatLoadSessionNoticeMemory(scopeKey);
  return memory[normalizedNoticeKey] === true;
}

function chatMarkSeenSessionNotice(scopeKey, noticeKey) {
  var normalizedNoticeKey = String(noticeKey || '').trim();
  if (!normalizedNoticeKey) return;
  var memory = chatLoadSessionNoticeMemory(scopeKey);
  memory[normalizedNoticeKey] = true;
  chatSaveSessionNoticeMemory(scopeKey, memory);
}

function chatClearSeenSessionNotice(scopeKey, noticeKey) {
  var normalizedNoticeKey = String(noticeKey || '').trim();
  if (!normalizedNoticeKey) return;
  var memory = chatLoadSessionNoticeMemory(scopeKey);
  if (!memory[normalizedNoticeKey]) return;
  delete memory[normalizedNoticeKey];
  chatSaveSessionNoticeMemory(scopeKey, memory);
}
