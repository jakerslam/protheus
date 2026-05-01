
      this.chatInputHistory = chatRows;
      this.terminalInputHistory = terminalRows;
      this.hydrateInputHistoryFromCache('chat', fallbackAgentId);
      this.hydrateInputHistoryFromCache('terminal', fallbackAgentId);
      this.syncInputHistoryToCache('chat', fallbackAgentId);
      this.syncInputHistoryToCache('terminal', fallbackAgentId);
      this.resetInputHistoryNavigation('chat');
      this.resetInputHistoryNavigation('terminal');
    },

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
        var normalized = self.mergeModelNoticesForAgent(agentId, self.normalizeSessionMessages(data));
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
            self._hasMoreMessages = !!(data && data.has_more);
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
