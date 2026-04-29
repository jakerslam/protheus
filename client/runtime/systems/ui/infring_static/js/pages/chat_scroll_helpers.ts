// Canonical chat helper module: scroll/bottom-follow and chat markdown export helpers.

function resolveBottomFollowTolerancePx(page, overridePx) {
  var raw = Number(overridePx);
  if (!Number.isFinite(raw) || raw < 1) raw = Number(page && page.scrollBottomFollowTolerancePx);
  if (!Number.isFinite(raw) || raw < 1) raw = 32;
  if (raw > 160) raw = 160;
  return raw;
}

function extractChatMarkdownText(message) {
  var row = message && typeof message === 'object' ? message : {};
  var text = String(row.text || '').trim();
  if (!text && row.file_output && row.file_output.content) {
    text = String(row.file_output.content || '').trim();
  }
  if (!text && row.folder_output && row.folder_output.tree) {
    text = String(row.folder_output.tree || '').trim();
  }
  return text;
}

function buildChatMarkdown(messages, assistantName) {
  var rows = Array.isArray(messages) ? messages : [];
  if (!rows.length) return '';
  var assistantLabel = String(assistantName || 'Assistant').trim() || 'Assistant';
  var lines = ['# Chat with ' + assistantLabel, ''];
  for (var i = 0; i < rows.length; i++) {
    var row = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
    var role = String(row.role || '').toLowerCase();
    var label = role === 'user'
      ? 'You'
      : (role === 'agent'
        ? assistantLabel
        : (role === 'system' ? 'System' : 'Tool'));
    var content = extractChatMarkdownText(row);
    if (!content) continue;
    var ts = Number(row.ts || row.timestamp || 0);
    var tsLabel = Number.isFinite(ts) && ts > 0 ? (' (' + new Date(ts).toISOString() + ')') : '';
    lines.push('## ' + label + tsLabel, '', content, '');
  }
  return lines.join('\n').trim();
}

function exportChatMarkdown(messages, assistantName) {
  var markdown = buildChatMarkdown(messages, assistantName);
  if (!markdown) return false;
  var blob = new Blob([markdown + '\n'], { type: 'text/markdown' });
  var url = URL.createObjectURL(blob);
  var anchor = document.createElement('a');
  var label = String(assistantName || 'chat').trim().replace(/[^A-Za-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '') || 'chat';
  anchor.href = url;
  anchor.download = 'chat-' + label + '-' + Date.now() + '.md';
  anchor.click();
  URL.revokeObjectURL(url);
  return true;
}

function resolveLatestMessageScrollTop(page, el) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return 0;
  var clientHeight = Math.max(0, Number(host.clientHeight || 0));
  var maxTop = Math.max(0, Number(host.scrollHeight || 0) - clientHeight);
  void page;
  return maxTop;
}

function resolveDistanceFromLatestMessageBottom(page, el) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return Number.POSITIVE_INFINITY;
  var targetTop = resolveLatestMessageScrollTop(page, host);
  var top = Math.max(0, Number(host.scrollTop || 0));
  return Math.max(0, targetTop - top);
}

function syncLatestMessageBottomState(page, el, tolerancePx) {
  if (!page || typeof page !== 'object') return;
  var host = el || (typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return;
  var hiddenBottom = resolveDistanceFromLatestMessageBottom(page, host);
  page._stickToBottom = hiddenBottom <= resolveBottomFollowTolerancePx(page, tolerancePx);
  page.showScrollDown = hiddenBottom > 120;
}

function isNearLatestMessageBottom(page, el, tolerancePx) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return false;
  return resolveDistanceFromLatestMessageBottom(page, host) <= resolveBottomFollowTolerancePx(page, tolerancePx);
}

function clampScrollToLatestMessageBottom(page, el) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return 0;
  var targetTop = resolveLatestMessageScrollTop(page, host);
  if ((page && page.showFreshArchetypeTiles) || !host.querySelector('.chat-message-block[data-msg-idx], .chat-message-block')) return targetTop;
  var top = Number(host.scrollTop || 0), clientHeight = Math.max(0, Number(host.clientHeight || 0));
  var maxTop = Math.max(0, Number(host.scrollHeight || 0) - clientHeight);
  var hardCapTop = Math.min(maxTop, targetTop);
  var slack = Number(page && page.scrollBottomClampSlackPx);
  if (!Number.isFinite(slack) || slack < 0) slack = 16;
  if (top > (hardCapTop + slack)) {
    var wheelAt = Number(page && page._lastMessagesWheelAt || 0), recentWheel = wheelAt > 0 && ((Date.now() - wheelAt) < 120);
    if (!recentWheel) setTimeout(function() { host.scrollTop = Math.min(Number(host.scrollTop || 0), resolveLatestMessageScrollTop(page, host)); }, 24);
  }
  return hardCapTop;
}

function scheduleBottomHardCapClamp(page, el, targetTop, delayMs) {
  if (!page || typeof page !== 'object') return;
  if (page._bottomClampTimer) clearTimeout(page._bottomClampTimer);
  var host = el || (typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return;
  var hardCapTop = Number(targetTop), delay = Number(delayMs);
  if (!Number.isFinite(hardCapTop)) hardCapTop = resolveLatestMessageScrollTop(page, host);
  if (!Number.isFinite(delay) || delay < 24) delay = 120;
  page._bottomClampTimer = setTimeout(function() {
    page._bottomClampTimer = 0;
    var now = Date.now(), recentAt = Math.max(Number(page._lastMessagesWheelAt || 0), Number(page._lastMessagesScrollAt || 0));
    if (recentAt > 0 && (now - recentAt) < 96) return scheduleBottomHardCapClamp(page, host, hardCapTop, 72);
    clampScrollToLatestMessageBottom(page, host);
    if (typeof page.syncGridBackgroundOffset === 'function') page.syncGridBackgroundOffset(host);
    syncLatestMessageBottomState(page, host);
  }, delay);
}

function cancelPinToLatestOnOpenJob(page) {
  if (!page || typeof page !== 'object') return;
  page._openPinToken = Number(page._openPinToken || 0) + 1;
  if (page._openPinRaf) {
    if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(page._openPinRaf);
    page._openPinRaf = 0;
  }
  if (page._openPinTimer) {
    clearTimeout(page._openPinTimer);
  }
  page._openPinRaf = 0;
  page._openPinTimer = 0;
}

function runPinToLatestOnOpenJob(page, container, options) {
  if (!page || typeof page !== 'object') return;
  var opts = options || {};
  var maxFrames = Number(opts.maxFrames || 18);
  if (!Number.isFinite(maxFrames) || maxFrames < 4) maxFrames = 18;
  if (maxFrames > 64) maxFrames = 64;
  var stableFramesNeeded = Number(opts.stableFrames || 2);
  if (!Number.isFinite(stableFramesNeeded) || stableFramesNeeded < 1) stableFramesNeeded = 2;
  if (stableFramesNeeded > 6) stableFramesNeeded = 6;
  var token = Number(page._openPinToken || 0) + 1;
  var frame = 0;
  var stable = 0;
  var lastTop = -1;
  var lastHeight = -1;
  var lastClient = -1;
  var target = container || null;
  page._openPinToken = token;
  cancelPinToLatestOnOpenJob(page);
  var schedule = function() {
    if (Number(page._openPinToken || 0) !== token) return;
    if (typeof requestAnimationFrame === 'function') {
      page._openPinRaf = requestAnimationFrame(tick);
    } else {
      page._openPinTimer = setTimeout(tick, 16);
    }
  };
  var tick = function() {
    if (Number(page._openPinToken || 0) !== token) return;
    page._openPinRaf = 0;
    page._openPinTimer = 0;
    var el = typeof page.resolveMessagesScroller === 'function'
      ? page.resolveMessagesScroller(target)
      : null;
    if (el) {
      var scrollHeight = Math.max(0, Number(el.scrollHeight || 0));
      var clientHeight = Math.max(0, Number(el.clientHeight || 0));
      var targetTop = resolveLatestMessageScrollTop(page, el);
      el.scrollTop = targetTop;
      if (typeof page.syncGridBackgroundOffset === 'function') page.syncGridBackgroundOffset(el);
      page.showScrollDown = false;
      if (typeof page.syncMapSelectionToScroll === 'function') page.syncMapSelectionToScroll(el);
      if (typeof page.scheduleMessageRenderWindowUpdate === 'function') page.scheduleMessageRenderWindowUpdate(el);
      var top = Math.round(Number(el.scrollTop || 0));
      var height = Math.round(scrollHeight);
      var client = Math.round(clientHeight);
      var nearBottom = Math.abs(top - targetTop) <= 2 || height <= (client + 2);
      if (nearBottom && top === lastTop && height === lastHeight && client === lastClient) {
        stable += 1;
      } else if (nearBottom) {
        stable = 1;
      } else {
        stable = 0;
      }
      lastTop = top;
      lastHeight = height;
      lastClient = client;
    } else {
      stable = 0;
    }
    frame += 1;
    if (stable >= stableFramesNeeded || frame >= maxFrames) {
      cancelPinToLatestOnOpenJob(page);
      if (typeof page.scrollToBottomImmediate === 'function') page.scrollToBottomImmediate();
      return;
    }
    schedule();
  };
  schedule();
}
