// Chat scroll, bottom-follow, and direct-hover method helpers.
'use strict';

function infringChatScrollMethodHelpers() {
  return {
    _latexTimer: null,
    exportCurrentChatMarkdown: function() {
      var assistantName = String(
        (this.currentAgent && (this.currentAgent.name || this.currentAgent.id)) || 'infring'
      ).trim() || 'infring';
      return exportChatMarkdown(this.messages, assistantName);
    },

    resolveMessagesScroller: function(preferred) {
      var candidate = preferred || null;
      if (candidate && candidate.id === 'messages' && candidate.offsetParent !== null) return candidate;
      var refNode = this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : null;
      if (refNode && refNode.offsetParent !== null) return refNode;
      var nodes = document.querySelectorAll('#messages');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (node && node.offsetParent !== null) return node;
      }
      return candidate && candidate.id === 'messages' ? candidate : null;
    },
    syncMapSelectionToScroll: function(container) {
      var el = this.resolveMessagesScroller(container);
      if (!el || !this.currentAgent || !Array.isArray(this.messages) || !this.messages.length) return;
      var nodes = el.querySelectorAll('.chat-message-block[id^="chat-msg-"]');
      if (!nodes || !nodes.length) return;
      var viewport = el.getBoundingClientRect();
      var viewportCenterY = viewport.top + (viewport.height / 2);
      var bestNode = null;
      var bestDiff = Number.POSITIVE_INFINITY;
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node || node.offsetParent === null) continue;
        var rect = node.getBoundingClientRect();
        if (rect.height <= 0) continue;
        if (rect.bottom < viewport.top || rect.top > viewport.bottom) continue;
        var nodeCenter = rect.top + (rect.height / 2);
        var diff = Math.abs(nodeCenter - viewportCenterY);
        if (diff < bestDiff) {
          bestDiff = diff;
          bestNode = node;
        }
      }
      if (!bestNode || !bestNode.id) return;
      var domId = String(bestNode.id);
      if (this.selectedMessageDomId !== domId) this.selectedMessageDomId = domId;
      var popup = typeof this.activeDashboardPopupOrigin === 'function'
        ? (this.activeDashboardPopupOrigin() || {})
        : {};
      if (String(popup.source || '').trim() !== 'chat-map') this.hoveredMessageDomId = domId;
      for (var idx = 0; idx < this.messages.length; idx++) {
        if (this.messageDomId(this.messages[idx], idx) === domId) { this.mapStepIndex = idx; break; }
      }
      var chatStore = window.InfringChatStore;
      if (chatStore && typeof chatStore.setThreadProjectionCenter === 'function') {
        chatStore.setThreadProjectionCenter(this.mapStepIndex);
      }
      this.centerChatMapOnMessage(domId, { immediate: true });
    },

    scrollToBottom(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var self = this;
      self.$nextTick(function() {
        if (opts.buttonAnimated) {
          self.scrollToBottomFromButton(opts);
          if (opts.stabilize) self.stabilizeBottomScroll();
          return;
        }
        self.scrollToBottomImmediate(opts);
        if (opts.stabilize) self.stabilizeBottomScroll();
      });
    },

    scrollToBottomFromButton(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var el = this.resolveMessagesScroller(opts.container || null);
      if (!el) return;
      var force = opts.force !== false;
      if (!force && !this._stickToBottom && !isNearLatestMessageBottom(this, el, opts.tolerancePx)) return;
      var startTop = Number(el.scrollTop || 0);
      var targetTop = resolveLatestMessageScrollTop(this, el);
      if (!(targetTop > startTop + 1)) {
        this.scrollToBottomImmediate({ container: el, force: true });
        return;
      }
      if (this._scrollToBottomButtonRaf) {
        try { cancelAnimationFrame(this._scrollToBottomButtonRaf); } catch (_) {}
        this._scrollToBottomButtonRaf = 0;
      }
      this._stickToBottom = true;
      this.showScrollDown = false;
      var self = this;
      var duration = 1000;
      var startedAt = 0;
      var easeOut = function(t) {
        var x = Math.max(0, Math.min(1, Number(t || 0)));
        return 1 - Math.pow(1 - x, 3);
      };
      var step = function(ts) {
        if (!startedAt) startedAt = Number(ts || 0);
        var elapsed = Math.max(0, Number(ts || 0) - startedAt);
        var progress = Math.max(0, Math.min(1, elapsed / duration));
        var eased = easeOut(progress);
        var top = startTop + ((targetTop - startTop) * eased);
        el.scrollTop = top;
        self.syncGridBackgroundOffset(el);
        if (progress < 1) {
          self._scrollToBottomButtonRaf = requestAnimationFrame(step);
          return;
        }
        self._scrollToBottomButtonRaf = 0;
        // Preserve current "blink" completion semantics, but only after the
        // staged 1s glide has completed.
        self.scrollToBottomImmediate({ container: el, force: true });
      };
      this._scrollToBottomButtonRaf = requestAnimationFrame(step);
    },

    scrollToBottomImmediate(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var el = this.resolveMessagesScroller(opts.container || null);
      if (!el) return;
      var force = opts.force !== false;
      if (!force && !this._stickToBottom && !isNearLatestMessageBottom(this, el, opts.tolerancePx)) return;
      el.scrollTop = resolveLatestMessageScrollTop(this, el);
      this.syncGridBackgroundOffset(el);
      this.showScrollDown = false;
      this._stickToBottom = true;
      this.syncMapSelectionToScroll(el);
      this.scheduleMessageRenderWindowUpdate(el);
      if (this._latexTimer) clearTimeout(this._latexTimer);
      this._latexTimer = setTimeout(function() { renderLatex(el); }, 150);
    },

    stabilizeBottomScroll: function() {
      var self = this;
      var tries = 3;
      var tick = function() {
        var el = self.resolveMessagesScroller();
        if (!el) return;
        el.scrollTop = resolveLatestMessageScrollTop(self, el);
        self.syncGridBackgroundOffset(el);
        if (--tries > 0) {
          if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
          else setTimeout(tick, 16);
        }
      };
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
      else setTimeout(tick, 0);
    },
    cancelPinToLatestOnOpen: function() {
      cancelPinToLatestOnOpenJob(this);
    },
    pinToLatestOnOpen: function(container, options) {
      runPinToLatestOnOpenJob(this, container, options);
    },
    handleMessagesScroll(e) {
      var el = this.resolveMessagesScroller(e && e.target ? e.target : null);
      if (!el) return;
      this._lastMessagesScrollAt = Date.now();
      var targetTop = resolveLatestMessageScrollTop(this, el);
      scheduleBottomHardCapClamp(this, el, targetTop, 128);
      this.startAgentTrailLoop(el);
      this.syncGridBackgroundOffset(el);
      this.syncDirectHoverAfterScroll(el);
      var hiddenBottom = Math.max(0, targetTop - Number(el.scrollTop || 0));
      this._stickToBottom = hiddenBottom <= resolveBottomFollowTolerancePx(this);
      this.showScrollDown = hiddenBottom > 120;
      var self = this;
      if (typeof requestAnimationFrame === 'function') {
        if (this._scrollSyncFrame) cancelAnimationFrame(this._scrollSyncFrame);
        this._scrollSyncFrame = requestAnimationFrame(function() {
          self._scrollSyncFrame = 0;
          self.syncMapSelectionToScroll(el);
        });
      } else {
        self.syncMapSelectionToScroll(el);
      }
      this.scheduleMessageRenderWindowUpdate(el);
      if (Number(el.scrollTop || 0) === 0 && this._hasMoreMessages && !this._olderMessagesLoading) {
        this.loadOlderMessages();
      }
    },
    resolveHoveredMessageDomIdFromPoint(container, clientX, clientY) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return '';
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!(x > 0 && y > 0)) return '';
      var currentId = String(this.directHoveredMessageDomId || '').trim();
      var pickFromNode = function(node) {
        if (!node || typeof node.closest !== 'function') return '';
        var blockEl = node.closest('.chat-message-block[id^="chat-msg-"]');
        if (blockEl && host.contains(blockEl)) return String(blockEl.id || '').trim();
        var messageEl = node.closest('.message[id^="chat-msg-"]');
        if (messageEl && host.contains(messageEl)) return String(messageEl.id || '').trim();
        return '';
      };
      var candidateId = '';
      try {
        candidateId = pickFromNode(document.elementFromPoint(x, y));
      } catch (_) {
        candidateId = '';
      }
      if (!candidateId && typeof document.elementsFromPoint === 'function') {
        try {
          var stack = document.elementsFromPoint(x, y) || [];
          for (var i = 0; i < stack.length; i++) {
            candidateId = pickFromNode(stack[i]);
            if (candidateId) break;
          }
        } catch (_) {
          candidateId = '';
        }
      }
      if (candidateId && currentId && candidateId !== currentId) {
        var candidateEl = document.getElementById(candidateId);
        if (candidateEl) {
          var cRect = candidateEl.getBoundingClientRect();
          // Require pointer to move slightly inside the new row to avoid
          // boundary thrash on the split line between adjacent messages.
          if (y <= (cRect.top + 2) || y >= (cRect.bottom - 2)) {
            return currentId;
          }
        }
      }
      if (!candidateId && currentId) {
        var stickyEl = document.getElementById(currentId);
        if (stickyEl && host.contains(stickyEl)) {
          var sRect = stickyEl.getBoundingClientRect();
          var inStickyBand =
            x >= (sRect.left - 2) &&
            x <= (sRect.right + 2) &&
            y >= (sRect.top - 2) &&
            y <= (sRect.bottom + 2);
          if (inStickyBand) return currentId;
        }
      }
      return candidateId;
    },

    syncDirectHoverFromPointer(event) {
      if (!event || !event.currentTarget) return;
      this._lastPointerClientX = Number(event.clientX || 0);
      this._lastPointerClientY = Number(event.clientY || 0);
      var host = this.resolveMessagesScroller(event.currentTarget);
      if (!host) return;
      var domId = this.resolveHoveredMessageDomIdFromPoint(
        host,
        this._lastPointerClientX,
        this._lastPointerClientY
      );
      if (domId) {
        if (this._hoverClearTimer) {
          clearTimeout(this._hoverClearTimer);
          this._hoverClearTimer = 0;
        }
        this.directHoveredMessageDomId = domId;
        this.hoveredMessageDomId = domId;
        return;
      }
    },

    syncDirectHoverAfterScroll(container) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return;
      var px = Number(this._lastPointerClientX || 0);
      var py = Number(this._lastPointerClientY || 0);
      if (!(px > 0 && py > 0)) {
        this.directHoveredMessageDomId = '';
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      var domId = this.resolveHoveredMessageDomIdFromPoint(host, px, py);
      if (!domId) {
        this.directHoveredMessageDomId = '';
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      this.directHoveredMessageDomId = domId;
      this.hoveredMessageDomId = domId;
    },
  };
}
