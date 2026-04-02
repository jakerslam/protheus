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

function resolveBottomBufferPx(page) {
  var raw = Number(page && page.scrollBottomBufferPx);
  if (!Number.isFinite(raw) || raw < 0) raw = 64;
  if (raw > 192) raw = 192;
  return raw;
}

function resolveBottomFollowTolerancePx(page, overridePx) {
  var raw = Number(overridePx);
  if (!Number.isFinite(raw) || raw < 1) raw = Number(page && page.scrollBottomFollowTolerancePx);
  if (!Number.isFinite(raw) || raw < 1) raw = 32;
  if (raw > 160) raw = 160;
  return raw;
}

function isNearLatestMessageBottom(page, el, tolerancePx) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return false;
  var targetTop = resolveLatestMessageScrollTop(page, host);
  var top = Math.max(0, Number(host.scrollTop || 0));
  return Math.abs(top - targetTop) <= resolveBottomFollowTolerancePx(page, tolerancePx);
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
  }, delay);
}
function resolveLatestMessageScrollTop(page, el) {
  var host = el || (page && typeof page.resolveMessagesScroller === 'function' ? page.resolveMessagesScroller() : null);
  if (!host) return 0;
  var clientHeight = Math.max(0, Number(host.clientHeight || 0));
  var maxTop = Math.max(0, Number(host.scrollHeight || 0) - clientHeight);
  var blocks = host.querySelectorAll('.chat-message-block[data-msg-idx], .chat-message-block');
  if (!blocks || !blocks.length) return maxTop;
  var bottom = 0;
  for (var i = 0; i < blocks.length; i++) {
    var block = blocks[i];
    if (!block || block.offsetParent === null) continue;
    var blockBottom = Number(block.offsetTop || 0) + Math.max(0, Number(block.offsetHeight || 0));
    if (blockBottom > bottom) bottom = blockBottom;
  }
  if (!(bottom > 0)) return maxTop;
  var bottomBuffer = resolveBottomBufferPx(page);
  var targetTop = Math.max(0, Math.round((bottom + bottomBuffer) - clientHeight));
  return targetTop > maxTop ? maxTop : targetTop;
}
