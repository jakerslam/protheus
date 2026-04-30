// Chat websocket miscellaneous event handlers.
'use strict';

function infringChatWebSocketMiscEventMethods() {
  return {
    handleWsSilentCompleteEvent: function(data) {
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
      this._clearTypingTimeout();
      this._clearStreamingTypewriters();
      this._inflightPayload = null;
      this._pendingAutoModelSwitchBaseline = '';
      var nowTs = Date.now();
      var hasRecentSubstantiveAgentReply = false;
      for (var si = this.messages.length - 1; si >= 0; si--) {
        var stable = this.messages[si];
        if (!stable) continue;
        if (stable.thinking || stable.streaming) continue;
        if (String(stable.role || '').toLowerCase() !== 'agent') continue;
        var stableText = String(stable.text || '').trim();
        if (!stableText) continue;
        if (stable._auto_fallback) continue;
        var stableAge = Math.max(0, nowTs - Number(stable.ts || nowTs));
        if (stableAge <= 20000) {
          hasRecentSubstantiveAgentReply = true;
        }
        break;
      }
      if (hasRecentSubstantiveAgentReply) {
        this.clearTransientThinkingRowsCompat({ force: true });
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        var selfSilentSkip = this;
        this.$nextTick(function() { selfSilentSkip._processQueue(); });
        this.refreshPromptSuggestions(true, 'post-silent-skip');
        return;
      }
      var silentEnvelope = this.collectStreamedAssistantEnvelope();
      var silentThought = String(silentEnvelope.thought || '').trim();
      var silentTools = silentEnvelope.tools || [];
      if (silentThought) {
        silentTools.unshift(this.makeThoughtToolCard(silentThought, Number(data && data.duration_ms ? data.duration_ms : 0)));
      }
      this.clearTransientThinkingRowsCompat({ force: true });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      var selfSilent = this;
      this.$nextTick(function() { selfSilent._processQueue(); });
      this.refreshPromptSuggestions(true, 'post-silent-no-reply');
    },

    handleWsAgentArchivedEvent: function(data) {
      var agentId = data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
      this.setAgentLiveActivity(agentId, 'idle');
      this._clearPendingWsRequest(agentId);
      this.handleAgentInactive(agentId, data && data.reason ? String(data.reason) : 'archived');
    },

    handleWsAgentsUpdatedEvent: function(data) {
      if (data.agents) {
        this.applyAgentRosterUpdateFromWebSocket(data.agents);
      }
    },

    handleWsCommandResultEvent: function(data) {
      if (typeof this.appendChatSideResultNotice === 'function' && this.appendChatSideResultNotice(data)) {
        return;
      }
      this.applyContextTelemetry(data);
      var isContextTelemetryResult = Object.prototype.hasOwnProperty.call(data || {}, 'context_tokens') ||
        Object.prototype.hasOwnProperty.call(data || {}, 'context_window') ||
        Object.prototype.hasOwnProperty.call(data || {}, 'context_ratio') ||
        Object.prototype.hasOwnProperty.call(data || {}, 'context_pressure');
      if (!data.silent && !isContextTelemetryResult) {
        var commandResultMessage = String(data && data.message ? data.message : 'Command executed.');
        console.log('[command result]', commandResultMessage);
        InfringToast.info(commandResultMessage);
      }
    },

    handleWsCanvasEvent: function(data) {
      var canvasHtml = '<div class="canvas-panel" style="border:1px solid var(--border);border-radius:8px;margin:8px 0;overflow:hidden;">';
      canvasHtml += '<div style="padding:6px 12px;background:var(--surface);border-bottom:1px solid var(--border);font-size:0.85em;display:flex;justify-content:space-between;align-items:center;">';
      canvasHtml += '<span>' + (data.title || 'Canvas') + '</span>';
      canvasHtml += '<span style="opacity:0.5;font-size:0.8em;">' + (data.canvas_id || '').substring(0, 8) + '</span></div>';
      canvasHtml += '<iframe sandbox="allow-scripts" srcdoc="' + (data.html || '').replace(/"/g, '&quot;') + '" ';
      canvasHtml += 'style="width:100%;min-height:300px;border:none;background:#fff;" loading="lazy"></iframe></div>';
      this.messages.push({ id: ++msgId, role: 'agent', text: canvasHtml, meta: 'canvas', isHtml: true, tools: [] });
      this.scrollToBottom();
    },
  };
}
