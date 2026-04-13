
      this.chatInputHistory = chatRows;
      this.terminalInputHistory = terminalRows;
      this.hydrateInputHistoryFromCache('chat');
      this.hydrateInputHistoryFromCache('terminal');
      this.syncInputHistoryToCache('chat');
      this.syncInputHistoryToCache('terminal');
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
        var data = await InfringAPI.get('/api/agents/' + agentId + '/session');
        if (!loadStillCurrent()) return;
        self.rebuildInputHistoryFromSessionPayload(data);
        if (self.currentAgent && String(self.currentAgent.id || '') === String(agentId || '')) {
          self.applyAgentGitTreeState(self.currentAgent, data || {});
        }
        var normalized = self.mergeModelNoticesForAgent(agentId, self.normalizeSessionMessages(data));
        if (!loadStillCurrent()) return;
        if (normalized.length) {
          if (!preserveFreshInit) {
            self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
            self.freshInitRevealMenu = false;
            self.showFreshArchetypeTiles = false;
          }
          // Always prefer server-authoritative session state over potentially stale cache.
          self.messages = normalized;
          self.clearHoveredMessageHard();
          self.recomputeContextEstimate();
          self.cacheAgentConversation(agentId);
          self.$nextTick(function() {
            self.scrollToBottomImmediate();
            self.stabilizeBottomScroll();
            self.pinToLatestOnOpen(null, { maxFrames: 20 });
          });
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
