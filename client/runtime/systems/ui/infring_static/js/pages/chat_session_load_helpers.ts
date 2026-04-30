// Chat session load, pagination, and render stabilization helpers.
'use strict';

function infringChatSessionLoadMethods() {
  return {
    async loadSession(agentId, keepCurrent) {
      var self = this;
      var loadSeq = ++this._sessionLoadSeq;
      this.sessionLoading = true;
      var targetAgentId = String(agentId || '');
      var loadStillCurrent = function() {
        if (self._sessionLoadSeq !== loadSeq) return false;
        if (!self.currentAgent || !self.currentAgent.id) return true;
        return String(self.currentAgent.id || '') === targetAgentId;
      };
      try {
        var preserveFreshInit = self.isFreshInitInProgressFor(agentId);
        var data = await InfringAPI.get('/api/agents/' + agentId + '/session?limit=80');
        if (!loadStillCurrent()) return;
        self.rebuildInputHistoryFromSessionPayload(data);
        if (self.currentAgent && String(self.currentAgent.id || '') === String(agentId || '')) {
          self.applyAgentGitTreeState(self.currentAgent, data || {});
        }
        var messageWindow = data && data.message_window && typeof data.message_window === 'object'
          ? data.message_window
          : {};
        var normalized = self.mergeModelNoticesForAgent(agentId, self.normalizeSessionMessages(data, { requireWindow: true }));
        var shouldApplyAuthoritativeMessages = true;
        var pendingRequest = self._pendingWsRequest && self._pendingWsRequest.agent_id
          ? self._pendingWsRequest
          : null;
        if (pendingRequest && String(pendingRequest.agent_id || '') === String(agentId || '')) {
          var pendingStartedAt = Number(pendingRequest.started_at || 0);
          var observedPendingReply = false;
          if (typeof self._pendingRequestReplyObserved === 'function') {
            observedPendingReply = self._pendingRequestReplyObserved(normalized, pendingRequest, pendingStartedAt);
          }
          if (!observedPendingReply && typeof self._recentAgentReplyObserved === 'function') {
            observedPendingReply = self._recentAgentReplyObserved(normalized, pendingStartedAt);
          }
          if (!observedPendingReply) {
            // Keep optimistic local rows (user prompt + live thinking) visible
            // until authoritative session state catches up for this pending turn.
            shouldApplyAuthoritativeMessages = false;
          }
        }
        if (!loadStillCurrent()) return;
        if (normalized.length) {
          if (!preserveFreshInit) {
            self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
            self.freshInitRevealMenu = false;
            self.showFreshArchetypeTiles = false;
          }
          if (shouldApplyAuthoritativeMessages) {
            // Always prefer server-authoritative session state over potentially stale cache.
            self.messages = normalized;
            self._hasMoreMessages = !!(messageWindow && messageWindow.has_more);
            self._messagePageOffset = normalized.length;
            self.clearHoveredMessageHard();
            self.recomputeContextEstimate();
            self.cacheAgentConversation(agentId);
            self.$nextTick(function() {
              self.scrollToBottomImmediate();
              self.stabilizeBottomScroll();
              self.pinToLatestOnOpen(null, { maxFrames: 20 });
            });
          } else {
            self.recomputeContextEstimate();
            self.cacheAgentConversation(agentId);
            self._reconcileSendingState();
            self.$nextTick(function() {
              self.scrollToBottom();
              self.stabilizeBottomScroll();
            });
          }
        } else if (!keepCurrent) {
          if (!preserveFreshInit) {
            self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
            self.freshInitRevealMenu = false;
            self.showFreshArchetypeTiles = false;
            self.messages = [];
            self.clearHoveredMessageHard();
            self.recomputeContextEstimate();
            self.recoverEmptySessionRender(agentId, data || null);
          }

        }
      } catch(e) {
        if (!loadStillCurrent()) return;
        if (!keepCurrent && (!Array.isArray(self.messages) || !self.messages.length)) {
          var errText = String(e && e.message ? e.message : 'session_load_failed').trim();
          self.messages = [{
            id: ++msgId,
            role: 'notice',
            text: '',
            meta: '',
            tools: [],
            is_notice: true,
            notice_label: 'Unable to load this agent session right now (' + errText + ').',
            notice_type: 'warn',
            system_origin: 'session:load_error',
            ts: Date.now()
          }];
        }
      }
      finally {
        if (self._sessionLoadSeq === loadSeq) {
          await new Promise(function(resolve) {
            self.$nextTick(function() {
              self.scrollToBottomImmediate();
              self.stabilizeBottomScroll();
              self.pinToLatestOnOpen(null, { maxFrames: 22 });
              self.scheduleMessageRenderWindowUpdate();
              resolve();
            });
          });
          await self.waitForSessionRender(agentId, loadSeq);
          if (self._sessionLoadSeq === loadSeq) {
            self.enforceLatestViewportDeterminism();
            self.pinToLatestOnOpen(null, { maxFrames: 24 });
            self.sessionLoading = false;
          }
          self._reconcileSendingState();
          if (!self.showFreshArchetypeTiles) {
            self.refreshPromptSuggestions(false);
          }
        }
      }
    },

    async loadOlderMessages() {
      var self = this;
      if (!self._hasMoreMessages || self._olderMessagesLoading) return;
      var agentId = self.currentAgent && self.currentAgent.id;
      if (!agentId) return;
      self._olderMessagesLoading = true;
      try {
        var offset = Number(self._messagePageOffset || 0);
        var data = await InfringAPI.get('/api/agents/' + agentId + '/session?limit=80&offset=' + offset);
        if (!data || !data.ok) return;
        var messageWindow = data && data.message_window && typeof data.message_window === 'object'
          ? data.message_window
          : {};
        var older = self.normalizeSessionMessages(data, { requireWindow: true });
        if (!older.length) {
          self._hasMoreMessages = false;
          return;
        }
        self._hasMoreMessages = !!(messageWindow && messageWindow.has_more);
        self._messagePageOffset = offset + older.length;
        var el = self.resolveMessagesScroller(null);
        var prevScrollHeight = el ? el.scrollHeight : 0;
        self.messages = older.concat(Array.isArray(self.messages) ? self.messages : []);
        self.$nextTick(function() {
          if (el) el.scrollTop += (el.scrollHeight - prevScrollHeight);
        });
      } catch(_) {
      } finally {
        self._olderMessagesLoading = false;
      }
    },

    waitForAnimationFrame() {
      return new Promise(function(resolve) {
        if (typeof requestAnimationFrame === 'function') {
          requestAnimationFrame(function() { resolve(); });
        } else {
          setTimeout(resolve, 16);
        }
      });
    },

    async waitForSessionRender(agentId, loadSeq) {
      var self = this;
      var expectedAgent = String(agentId || '');
      var hasSessionMessages = Array.isArray(this.messages) && this.messages.length > 0;
      var minFrames = hasSessionMessages ? 2 : 1;
      var maxFrames = hasSessionMessages ? 42 : 6;
      var messagesEl = null;
      // Let Alpine commit template updates before probing for rendered blocks.
      await this.waitForAnimationFrame();
      await this.waitForAnimationFrame();
      for (var frame = 0; frame < maxFrames; frame++) {
        if (self._sessionLoadSeq !== loadSeq) return;
        if (!self.currentAgent || String(self.currentAgent.id || '') !== expectedAgent) return;
        if (!messagesEl) messagesEl = self.resolveMessagesScroller();
        if (!messagesEl) {
          await self.waitForAnimationFrame();
          continue;
        }
        self.scheduleMessageRenderWindowUpdate(messagesEl);
        if (!hasSessionMessages) {
          if (frame >= minFrames) return;
          await self.waitForAnimationFrame();
          continue;
        }

        var blockCount = messagesEl.querySelectorAll('.chat-message-block').length;
        var renderedCount = messagesEl.querySelectorAll('.chat-message-block .message, .chat-message-block .message-placeholder-shell, .chat-day-anchor, .chat-day-divider').length;
        if (blockCount > 0 && renderedCount > 0 && frame >= minFrames) {
          return;
        }
        await self.waitForAnimationFrame();
      }
    },

    enforceLatestViewportDeterminism() {
      var el = this.resolveMessagesScroller();
      if (!el) return false;
      if (!Array.isArray(this.messages) || this.messages.length < 1) return false;
      var blocks = el.querySelectorAll('.chat-message-block[data-msg-idx], .chat-message-block');
      if (!blocks || !blocks.length) {
        this.scrollToBottomImmediate({ force: true });
        return true;
      }
      var viewportTop = Number(el.scrollTop || 0);
      var viewportBottom = viewportTop + Math.max(0, Number(el.clientHeight || 0));
      var lastBottom = 0;
      for (var i = 0; i < blocks.length; i++) {
        var block = blocks[i];
        if (!block || block.offsetParent === null) continue;
        var bottom = Number(block.offsetTop || 0) + Math.max(0, Number(block.offsetHeight || 0));
        if (bottom > lastBottom) lastBottom = bottom;
      }
      if (!(lastBottom > 0)) return false;
      if (viewportTop > (lastBottom + 24) || viewportBottom < 24) {
        this.scrollToBottomImmediate({ force: true, tolerancePx: 999999 });
        this.stabilizeBottomScroll();
        return true;
      }
      return false;
    },
  };
}
