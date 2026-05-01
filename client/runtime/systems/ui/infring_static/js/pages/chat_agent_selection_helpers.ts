// Chat agent selection lifecycle helpers.
'use strict';

function infringChatAgentSelectionMethods() {
  return {
    selectAgent(agent) {
      var resolved = this.resolveAgent(agent);
      if (!resolved) return;
      var selectingSystemThread = this.isSystemThreadAgent(resolved);
      this.closeGitTreeMenu();
      var currentAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var nextAgentId = String((resolved && resolved.id) || '');
      this.maybeDiscardPendingFreshAgent(nextAgentId);
      if (currentAgentId !== nextAgentId) {
        var activeSearch = String(this.searchQuery || '').trim();
        if (activeSearch) {
          this.searchQuery = '';
          this.searchOpen = false;
        }
      }
      this._markAgentPreviewUnread(resolved.id, false);
      var pendingFreshId = this.getPendingFreshAgentIdFromShellStore();
      var forceFreshSession = pendingFreshId && String(resolved.id) === pendingFreshId;
      this.clearHoveredMessageHard();
      if (this.currentAgent && this.currentAgent.id && this.currentAgent.id !== resolved.id) {
        var switchingFrom = String(this.currentAgent.id || '');
        if (
          this.sending &&
          this._pendingWsRequest &&
          String(this._pendingWsRequest.agent_id || '') === switchingFrom
        ) {
          this._clearTypingTimeout();
          this.clearTransientThinkingRowsCompat({ force: true });
          this.sending = false;
          this._responseStartedAt = 0;
          this.setAgentLiveActivity(switchingFrom, 'working');
          this._recoverPendingWsRequest('agent_switch');
        }
        if (typeof this.captureConversationDraft === 'function') {
          this.captureConversationDraft(this.currentAgent.id);
        }
        this.cacheAgentConversation(this.currentAgent.id);
      }
      if (this.currentAgent && this.currentAgent.id === resolved.id) {
        if (selectingSystemThread) {
          this.activateSystemThread({ preserve_if_empty: true });
          return;
        }
        this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
        this.recordModelUsageTimestamp(resolved.model_name || resolved.runtime_model || '');
        if (forceFreshSession) {
          this.applyConversationInputMode(resolved.id, { force_terminal: false });
          this.messages = [];
          this.inputText = '';
          this.contextApproxTokens = 0;
          this.refreshContextPressure();
          this.resetFreshInitStateForAgent(resolved);
          if (this.conversationCache) {
            delete this.conversationCache[String(resolved.id)];
            this.persistConversationCache();
          }
          this.connectWs(resolved.id);
          this.loadSessions(resolved.id);
          this.requestContextTelemetry(true);
          this.clearPromptSuggestions();
          this.startFreshInitSequence(resolved);
          if (typeof this.restoreConversationDraft === 'function') {
            this.restoreConversationDraft(resolved.id, 'chat');
          }
          var selfFreshCurrent = this;
          this.$nextTick(function() {
            selfFreshCurrent.scrollToBottomImmediate();
            selfFreshCurrent.stabilizeBottomScroll();
            selfFreshCurrent.pinToLatestOnOpen(null, { maxFrames: 20 });
            selfFreshCurrent.installChatMapWheelLock();
            selfFreshCurrent.scheduleMessageRenderWindowUpdate();
          });
        } else {
          this.loadSession(resolved.id, false);
        }
        if (!(this.isSystemThreadAgent && this.isSystemThreadAgent(resolved))) {
          this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});
        }
        return;
      }
      if (selectingSystemThread) {
        this.activateSystemThread({ preserve_if_empty: false });
        return;
      }
      this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
      if (typeof this.setStoreActiveAgentId === 'function') this.setStoreActiveAgentId(resolved.id || null);
      this.recordModelUsageTimestamp(resolved.model_name || resolved.runtime_model || '');
      // Reset context meter on agent switch to avoid stale carry-over from prior threads.
      this.contextApproxTokens = 0;
      this.contextPressure = 'low';
      this.setContextWindowFromCurrentAgent();
      if (forceFreshSession) this.applyConversationInputMode(resolved.id, { force_terminal: false });
      else this.applyConversationInputMode(resolved.id);
      if (forceFreshSession && this.conversationCache) {
        delete this.conversationCache[String(resolved.id)];
        this.persistConversationCache();
      }
      var restored = forceFreshSession ? false : this.restoreAgentConversation(resolved.id);
      if (!restored) {
        this.messages = [];
        this.inputText = '';
        this.contextApproxTokens = 0;
        this.refreshContextPressure();
      }
      if (typeof this.restoreConversationDraft === 'function') {
        this.restoreConversationDraft(resolved.id);
      }
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      if (forceFreshSession) {
        this.resetFreshInitStateForAgent(resolved);
        this.clearPromptSuggestions();
        this.startFreshInitSequence(resolved);
      } else {
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this._freshInitThreadShownFor = '';
      }
      this._reconcileSendingState();
      this.connectWs(resolved.id);
      // Show welcome tips on first use.
      if (!restored && !this.showFreshArchetypeTiles && !this.hasSeenWelcomeTips()) {
        InfringToast.info('Type / for commands. Ctrl+/ opens the command palette.');
        this.markWelcomeTipsSeen();
      }
      if (!forceFreshSession) {
        this.loadSession(resolved.id, false);
      }
      this.loadSessions(resolved.id);
      this.requestContextTelemetry(true);
      this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});
      if (!forceFreshSession) {
        this.refreshPromptSuggestions(false);
      }
      if (this.showAgentDrawer) {
        this.openAgentDrawer();
      }
      // Focus input after agent selection.
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) el.focus();
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
      });
    },
  };
}
