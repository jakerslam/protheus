// Chat message render-window and placeholder virtualization helpers.
'use strict';

function infringChatMessageVirtualizationMethods() {
  return {
    isMessageVirtualizationActive(list) {
      var rows = Array.isArray(list) ? list : this.messages;
      return Array.isArray(rows) && rows.length > 80;
    },
    messageRenderMetrics(msg) {
      if (!msg || typeof msg !== 'object') return null;
      var metrics = msg._renderMetrics;
      if (!metrics || typeof metrics !== 'object') {
        metrics = {};
        msg._renderMetrics = metrics;
      }
      return metrics;
    },
    resolveMessageByDomId(domId) {
      var target = String(domId || '').trim();
      if (!target) return null;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        if (this.messageDomId(rows[i], i) === target) return rows[i];
      }
      return null;
    },
    trackRenderedMessageMetrics(blockEl) {
      if (!blockEl || typeof blockEl.querySelector !== 'function') return;
      var metricRoot = blockEl.classList && blockEl.classList.contains('chat-message-block') ? blockEl : ((typeof blockEl.closest === 'function' && blockEl.closest('.chat-message-block')) || blockEl), bubble = metricRoot.querySelector('.message:not(.message-placeholder) .message-bubble:not(.message-placeholder-bubble)');
      if (!bubble) return;
      var msg = this.resolveMessageByDomId(String(metricRoot.id || blockEl.id || '').trim());
      if (!msg) return;
      var styles = window.getComputedStyle(bubble);
      var paddingTop = parseFloat(styles.paddingTop || '0');
      var paddingBottom = parseFloat(styles.paddingBottom || '0');
      var lineHeightRaw = parseFloat(styles.lineHeight || '0');
      var fontSizeRaw = parseFloat(styles.fontSize || '14');
      var lineHeight = Number.isFinite(lineHeightRaw) && lineHeightRaw > 0
        ? lineHeightRaw
        : Math.max(20, Math.round(fontSizeRaw * 1.6));
      var bubbleHeight = Math.max(0, Math.round(bubble.getBoundingClientRect().height));
      var bubbleWidth = Math.max(0, Math.round(bubble.getBoundingClientRect().width));
      var contentHeight = Math.max(0, bubbleHeight - Math.round(paddingTop + paddingBottom));
      var lineCount = Math.max(1, Math.ceil(contentHeight / Math.max(lineHeight, 1)));
      var metrics = this.messageRenderMetrics(msg);
      if (!metrics) return;
      metrics.lineCount = lineCount;
      metrics.lineHeight = Math.max(18, Math.round(lineHeight));
      metrics.bubbleHeight = Math.max(Math.round(lineHeight + paddingTop + paddingBottom), bubbleHeight);
      metrics.bubbleWidth = bubbleWidth;
      metrics.updatedAt = Date.now();
    },
    shouldRenderMessage(msg, idx, list) { void msg; void idx; void list; return true; },
    // Gate the heavyweight bubble content on the render window. When this returns
    // false, Alpine's x-if branch in index_body.html.parts unmounts the
    // <infring-chat-bubble-render> element (markdown + code blocks + media) and
    // mounts the lightweight <infring-message-placeholder-shell> instead, which
    // is a sized stack of <span class="message-placeholder-line"> elements
    // dimensioned from msg._renderMetrics so scroll position is preserved.
    //
    // Previously this was hardcoded to `return true`, which meant the heavy
    // bubble never unmounted; the .message-text-skeletonized CSS class was used
    // as a visual fallback (transparent text + repeating-linear-gradient gray
    // lines) but every DOM node was still rendered, so markdown parsing, code
    // tokenization, and layout cost all stayed in the hot path. Flipping this
    // gate to delegate to isMessageTextInRenderWindow turns the existing
    // placeholder infrastructure into a real DOM-level virtualization.
    shouldRenderMessageContent(msg, idx, list) {
      var rows = Array.isArray(list) ? list : this.messages;
      // Virtualization only kicks in once the chat passes the threshold
      // (currently > 80 messages, see isMessageVirtualizationActive). Below
      // that, render everything to keep the small-chat path simple.
      if (typeof this.isMessageVirtualizationActive === 'function'
        && !this.isMessageVirtualizationActive(rows)) return true;
      // Always keep streaming / thinking / typing-visual / thought-streaming
      // messages fully rendered. The user is actively watching them and any
      // visual flicker from unmount/remount destroys the live-text experience.
      if (msg && (msg.streaming || msg.thinking || msg._typingVisual || msg.thoughtStreaming)) return true;
      var domId = typeof this.messageDomId === 'function'
        ? this.messageDomId(msg, idx)
        : null;
      if (domId) {
        var forced = this._forcedHydrateById && typeof this._forcedHydrateById === 'object'
          ? this._forcedHydrateById
          : null;
        if (forced && Number(forced[domId] || 0) > Date.now()) return true;
        if (this.messageHydrationReady && this.messageHydration && typeof this.messageHydration === 'object') {
          return !!this.messageHydration[domId];
        }
      }
      // Fall through to the existing render-window logic (±messageTextRenderWindowRadius
      // around the active scroll position, default 20). Active position is
      // updated on every scroll by syncMapSelectionToScroll which sets
      // mapStepIndex + selectedMessageDomId from the viewport center, so this
      // gate follows the user's current focus.
      if (typeof this.isMessageTextInRenderWindow === 'function') {
        return !!this.isMessageTextInRenderWindow(msg, idx, rows);
      }
      // Conservative fallback: if the gate plumbing is missing, render.
      return true;
    },
    isMessageTextInRenderWindow(msg, idx, list) {
      var rows = Array.isArray(list) ? list : this.messages, active = Number(this.mapStepIndex), selected = String(this.selectedMessageDomId || this.hoveredMessageDomId || this.directHoveredMessageDomId || '').trim(), windowRows = Number(this.messageTextRenderWindowRadius || 20);
      if (!this.isMessageVirtualizationActive(rows)) return true;
      var domId = typeof this.messageDomId === 'function' ? this.messageDomId(msg, idx) : '';
      if (selected && domId && selected === domId) return true;
      if (!Number.isFinite(active) || active < 0 || active >= rows.length) active = Math.max(0, rows.length - 1);
      return Math.abs(Number(idx || 0) - active) <= (Number.isFinite(windowRows) && windowRows > 0 ? windowRows : 20) || !!(msg && (msg.streaming || msg.thinking || msg._typingVisual));
    },
    messageEstimatedLineCount(msg) {
      var metrics = this.messageRenderMetrics(msg);
      if (metrics && Number.isFinite(Number(metrics.lineCount)) && Number(metrics.lineCount) > 0) {
        return Math.max(1, Math.round(Number(metrics.lineCount)));
      }
      if (!msg || typeof msg !== 'object') return 1;
      var preview = '';
      if (typeof this.messageVisiblePreviewText === 'function') {
        preview = String(this.messageVisiblePreviewText(msg) || '');
      }
      if (!preview && typeof msg.text === 'string') preview = String(msg.text || '');
      var logicalLines = preview ? preview.split(/\r?\n/) : [''];
      var charsPerLine = msg.terminal ? 72 : (String(msg.role || '').toLowerCase() === 'user' ? 46 : 54);
      var lineCount = 0;
      for (var i = 0; i < logicalLines.length; i++) {
        var segment = String(logicalLines[i] || '');
        lineCount += Math.max(1, Math.ceil(segment.length / Math.max(charsPerLine, 1)));
      }
      if (Array.isArray(msg.tools) && msg.tools.length) lineCount += Math.max(2, msg.tools.length * 2);
      if (msg.file_output && msg.file_output.path) lineCount += 4;
      if (msg.folder_output && msg.folder_output.path) lineCount += 5;
      if (Array.isArray(msg.images) && msg.images.length) lineCount += Math.max(2, msg.images.length * 2);
      if (typeof this.messageProgress === 'function' && this.messageProgress(msg)) lineCount += 2;
      if (typeof this.messageToolTraceSummary === 'function' && this.messageToolTraceSummary(msg).visible) lineCount += 1;
      return Math.max(1, Math.min(48, lineCount));
    },
    messagePlaceholderResolvedLineCount(msg, idx, list) {
      void idx;
      void list;
      return this.messageEstimatedLineCount(msg);
    },
    messagePlaceholderResolvedLineHeight(msg, idx, list) {
      void idx;
      void list;
      var metrics = this.messageRenderMetrics(msg);
      if (metrics && Number.isFinite(Number(metrics.lineHeight)) && Number(metrics.lineHeight) > 0) {
        return Math.max(18, Math.round(Number(metrics.lineHeight)));
      }
      return msg && msg.terminal ? 20 : 24;
    },
    messagePlaceholderStyle(msg, idx, list) {
      var lineCount = this.messagePlaceholderResolvedLineCount(msg, idx, list);
      var lineHeight = this.messagePlaceholderResolvedLineHeight(msg, idx, list);
      var metrics = this.messageRenderMetrics(msg);
      var bubbleHeight = metrics && Number.isFinite(Number(metrics.bubbleHeight)) && Number(metrics.bubbleHeight) > 0
        ? Math.round(Number(metrics.bubbleHeight))
        : Math.round((lineCount * lineHeight) + (msg && msg.terminal ? 20 : 28));
      var trackedWidth = metrics && Number.isFinite(Number(metrics.bubbleWidth)) ? Math.round(Number(metrics.bubbleWidth)) : 0;
      var widthValue = 'var(--message-bubble-readable-width)';
      if (msg && msg.terminal) {
        widthValue = trackedWidth > 0 ? (trackedWidth + 'px') : 'min(84ch, 90%)';
      } else if (lineCount > 1 && trackedWidth > 0) {
        widthValue = Math.max(180, trackedWidth) + 'px';
      }
      return '--message-placeholder-line-count:' + String(lineCount) + ';' +
        '--message-placeholder-line-height:' + String(lineHeight) + 'px;' +
        '--message-placeholder-bubble-height:' + String(bubbleHeight) + 'px;' +
        '--message-placeholder-width:' + widthValue + ';';
    },
    messagePlaceholderLineIndices(msg, idx, list) {
      var count = this.messagePlaceholderResolvedLineCount(msg, idx, list);
      var indices = [];
      for (var i = 0; i < count; i++) indices.push(i);
      return indices;
    },
    forceMessageRender(msg, idx, ttlMs) {
      if (!msg) return;
      if (!this._forcedHydrateById || typeof this._forcedHydrateById !== 'object') this._forcedHydrateById = {};
      var domId = this.messageDomId(msg, idx);
      if (!domId) return;
      var ttl = Number(ttlMs || 0);
      if (!Number.isFinite(ttl) || ttl < 250) ttl = 2500;
      this._forcedHydrateById[domId] = Date.now() + ttl;
      this.scheduleMessageRenderWindowUpdate();
    },
    scheduleMessageRenderWindowUpdate(container) {
      var root = container && typeof container.querySelectorAll === 'function' ? container : null;
      if (this._renderWindowRaf) window.cancelAnimationFrame(this._renderWindowRaf);
      var self = this;
      this._renderWindowRaf = window.requestAnimationFrame(function() {
        self._renderWindowRaf = 0;
        self.updateMessageRenderWindow(root);
      });
    },
    updateMessageRenderWindow(container) {
      var root = container && typeof container.querySelectorAll === 'function'
        ? container
        : (this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : document.getElementById('messages'));
      if (!root) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      var blocks = Array.prototype.slice.call(root.querySelectorAll('.chat-message-block[id]')); if (!blocks.length) blocks = Array.prototype.slice.call(root.querySelectorAll('.chat-message-block .message[id]'));
      if (!blocks.length) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      for (var i = 0; i < blocks.length; i++) this.trackRenderedMessageMetrics(blocks[i]);
      if (!this.isMessageVirtualizationActive(blocks)) {
        this.messageHydration = {};
        this.messageHydrationReady = false;
        return;
      }
      var scrollTop = Number(root.scrollTop || 0);
      var viewportHeight = Number(root.clientHeight || 0);
      var bufferPx = Math.max(viewportHeight, 320);
      var firstVisible = -1;
      var lastVisible = -1;
      for (var j = 0; j < blocks.length; j++) {
        var block = blocks[j];
        var top = Number(block.offsetTop || 0);
        var height = Number(block.offsetHeight || 0);
        var bottom = top + Math.max(height, 1);
        if (bottom >= (scrollTop - bufferPx) && top <= (scrollTop + viewportHeight + bufferPx)) {
          if (firstVisible < 0) firstVisible = j;
          lastVisible = j;
        }
      }
      if (firstVisible < 0 || lastVisible < 0) {
        firstVisible = Math.max(0, blocks.length - 20);
        lastVisible = blocks.length - 1;
      }
      var extraRows = 10;
      var start = Math.max(0, firstVisible - extraRows);
      var end = Math.min(blocks.length - 1, lastVisible + extraRows);
      var nextHydration = {};
      for (var k = start; k <= end; k++) {
        nextHydration[blocks[k].id] = true;
      }
      if (blocks.length > 0) {
        nextHydration[blocks[0].id] = true;
        nextHydration[blocks[blocks.length - 1].id] = true;
      }
      if (this.selectedMessageDomId) nextHydration[String(this.selectedMessageDomId)] = true;
      if (this.hoveredMessageDomId) nextHydration[String(this.hoveredMessageDomId)] = true;
      if (this.directHoveredMessageDomId) nextHydration[String(this.directHoveredMessageDomId)] = true;
      var retainedForced = {};
      var now = Date.now();
      var forced = this._forcedHydrateById && typeof this._forcedHydrateById === 'object' ? this._forcedHydrateById : {};
      Object.keys(forced).forEach(function(domId) {
        var expiresAt = Number(forced[domId] || 0);
        if (!Number.isFinite(expiresAt) || expiresAt <= now) return;
        retainedForced[domId] = expiresAt;
        nextHydration[domId] = true;
      });
      this._forcedHydrateById = retainedForced;
      this.messageHydration = nextHydration;
      this.messageHydrationReady = true;
      var chatStore = window.InfringChatStore;
      if (chatStore && typeof chatStore.bumpRenderWindowVersion === 'function') chatStore.bumpRenderWindowVersion();
    },
  };
}
