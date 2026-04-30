// Chat websocket lifecycle event handlers.
'use strict';

function infringChatWebSocketLifecycleEventMethods() {
  return {
    handleWsConnectedEvent: function(data, activeWsAgentId) {
      var connectedAgentId = String(data && data.agent_id ? data.agent_id : '').trim();
      if (!connectedAgentId) return;
      var selectedAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      if (activeWsAgentId && connectedAgentId !== activeWsAgentId) return;
      if (selectedAgentId && connectedAgentId !== selectedAgentId) return;
      var connectedLive = this.resolveAgent(connectedAgentId);
      if (connectedLive) {
        this.currentAgent = this.applyAgentGitTreeState(connectedLive, connectedLive) || connectedLive;
        this.setStoreActiveAgentId(connectedAgentId);
      } else {
        var selfConnected = this;
        Promise.resolve()
          .then(function() {
            return selfConnected.rebindCurrentAgentAuthoritative({
              preferred_id: connectedAgentId,
              clear_when_missing: false,
            });
          })
          .catch(function() {});
      }
    },

    handleWsContextStateEvent: function(data) {
      this.applyContextTelemetry(data);
    },

    handleWsThinkingEvent: function(data) {
      if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
        this.ensureLiveThinkingRow(data);
        this.scrollToBottom();
        this._resetTypingTimeout();
      }
    },

    handleWsTypingEvent: function(data) {
      if (typeof this.shouldReloadHistoryForFinalEventPayload === 'function' && this.shouldReloadHistoryForFinalEventPayload(data)) {
        var finalAgentId = String((data && data.agent_id) || (this.currentAgent && this.currentAgent.id) || '').trim();
        var canReloadFinalSnapshot =
          !!finalAgentId &&
          !this.sending &&
          !(typeof this.hasLivePendingResponse === 'function' && this.hasLivePendingResponse()) &&
          !(typeof this._hasActiveTypewriterVisual === 'function' && this._hasActiveTypewriterVisual());
        if (canReloadFinalSnapshot) {
          var selfFinal = this;
          Promise.resolve()
            .then(function() { return selfFinal.loadSessions(finalAgentId); })
            .catch(function() { return []; })
            .then(function() { return selfFinal.loadSession(finalAgentId, true).catch(function() { return null; }); });
        }
      }
      if (data.state === 'start') {
        this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
        if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
          this.ensureLiveThinkingRow(data);
          this.scrollToBottom();
        }
        this._resetTypingTimeout();
      } else if (data.state === 'tool') {
        this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working');
        var typingMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
        if (typingMsg && (typingMsg.thinking || typingMsg.streaming)) {
          typingMsg.text = '';
          if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(typingMsg.thinking_status)) {
            typingMsg.thinking_status = '';
          }
        }
        this._resetTypingTimeout();
      } else if (data.state === 'stop') {
        var stillPending = (this.sending === true)
          || (typeof this.hasLivePendingResponse === 'function' && this.hasLivePendingResponse());
        if (stillPending) {
          if (typeof this.ensureLiveThinkingRow === 'function') {
            var pendingMsg = this.ensureLiveThinkingRow(data);
            if (pendingMsg) {
              pendingMsg.thinking = true;
              pendingMsg.streaming = true;
              pendingMsg._stream_updated_at = Date.now();
              if (!Number.isFinite(Number(pendingMsg._stream_started_at))) {
                pendingMsg._stream_started_at = Date.now();
              }
            }
          }
          this._resetTypingTimeout();
        } else this._clearTypingTimeout();
      }
    },
  };
}
