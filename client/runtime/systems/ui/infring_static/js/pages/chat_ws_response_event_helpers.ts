// Chat websocket final response event handlers.
'use strict';

function infringChatWebSocketResponseEventMethods() {
  return {
    handleWsResponseEvent: function(data) {
      var responsePendingRequest = this._pendingWsRequest && this._pendingWsRequest.agent_id
        ? this._pendingWsRequest
        : null;
      var responseAgentId = String(
        (data && data.agent_id) ||
        (responsePendingRequest && responsePendingRequest.agent_id) ||
        (this.currentAgent && this.currentAgent.id) ||
        ''
      ).trim();
      var responseTurnStartedAt = Number(
        this._responseStartedAt ||
        (responsePendingRequest && responsePendingRequest.started_at) ||
        Date.now()
      );
      if (!Number.isFinite(responseTurnStartedAt) || responseTurnStartedAt <= 0) {
        responseTurnStartedAt = Date.now();
      }
      this._clearTypingTimeout();
      this._clearStreamingTypewriters();
      this.applyContextTelemetry(data);
      var wsAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();
      if (!wsAutoSwitchPrevious) wsAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
      var wsRoute = this.applyAutoRouteTelemetry(data);
      var envelope = this.collectStreamedAssistantEnvelope();
      var streamedText = envelope.text;
      var streamedTools = envelope.tools;
      var streamedThought = envelope.thought;
      var responseTools = typeof this.responseToolRowsFromPayload === 'function' ? this.responseToolRowsFromPayload(data, 'ws-tool') : [];
      var responseHasToolCompletion = typeof this.responseHasAuthoritativeToolCompletion === 'function' ? this.responseHasAuthoritativeToolCompletion(data, responseTools.length ? responseTools : streamedTools) : (responseTools.length > 0 || streamedTools.length > 0);
      var hasAgentTerminalTranscript = !!(Array.isArray(data.terminal_transcript) && data.terminal_transcript.length && typeof this.appendAgentTerminalTranscript === 'function' && this.appendAgentTerminalTranscript(data.terminal_transcript));
      if (hasAgentTerminalTranscript) responseTools = responseTools.filter(function(t) { var n = String((t && t.name) || '').toLowerCase(); return !(n === 'terminal_exec' || n === 'run_terminal' || n === 'terminal' || n === 'shell_exec'); });
      if ((!Array.isArray(streamedTools) || !streamedTools.length) && responseTools.length) streamedTools = responseTools;
      var messageMetadata = typeof this.assistantTurnMetadataFromPayload === 'function' ? this.assistantTurnMetadataFromPayload(data, streamedTools) : {};
      if (!streamedThought && responseTools.length) {
        var thoughtTool = responseTools.find(function(rtool) { return !!(rtool && String(rtool.name || '').toLowerCase() === 'thought_process'); });
        if (thoughtTool) streamedThought = String(thoughtTool.input || thoughtTool.result || '').trim();
      }
      streamedTools.forEach(function(t) {
        t.running = false;
        if (t.id && t.id.indexOf('-txt-') !== -1 && !t.result) {
          t.result = 'Model attempted this call as text (not executed via tool system)';
          t.is_error = true;
        }
      });
      var meta = (data.input_tokens || 0) + ' in / ' + (data.output_tokens || 0) + ' out';
      if (data.cost_usd != null) meta += ' | $' + data.cost_usd.toFixed(4);
      if (data.iterations) meta += ' | ' + data.iterations + ' iter';
      if (data.fallback_model) meta += ' | fallback: ' + data.fallback_model;
      var wsDurationMs = Number(data.duration_ms || data.elapsed_ms || data.response_ms || 0);
      if (!wsDurationMs && this._responseStartedAt) wsDurationMs = Math.max(0, Date.now() - this._responseStartedAt);
      var wsDuration = this.formatResponseDuration(wsDurationMs);
      if (wsDuration) meta += ' | ' + wsDuration;
      var wsRouteMeta = this.formatAutoRouteMeta(wsRoute);
      if (wsRouteMeta) meta += ' | ' + wsRouteMeta;
      var payloadText = typeof this.assistantTextFromPayload === 'function'
        ? this.assistantTextFromPayload(data)
        : '';
      var finalText = (payloadText && payloadText.trim()) ? payloadText : streamedText;
      finalText = this.stripModelPrefix(finalText);
      var artifactDirectives = this.extractArtifactDirectives(finalText);
      var finalSplit = this.extractThinkingLeak(finalText);
      if (finalSplit.thought) {
        if (!streamedThought) streamedThought = finalSplit.thought;
        else if (streamedThought.indexOf(finalSplit.thought) === -1) streamedThought += '\n' + finalSplit.thought;
        finalText = finalSplit.content || '';
      }
      finalText = this.sanitizeToolText(finalText);
      finalText = this.stripArtifactDirectivesFromText(finalText);
      var collapsedThought = String(streamedThought || '').trim();
      var compactFinal = String(finalText || '').replace(/\s+/g, ' ').trim();
      var maybePlaceholder = /^(thinking|processing|working)\.\.\.$/i.test(compactFinal);
      if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(compactFinal)) maybePlaceholder = true;
      if (maybePlaceholder) finalText = '';
      if (collapsedThought && !streamedTools.some(function(tool) { return !!(tool && String(tool.name || '').toLowerCase() === 'thought_process'); })) streamedTools.unshift(this.makeThoughtToolCard(collapsedThought, wsDurationMs));
      if (!finalText.trim()) {
        // Policy: do not inject system-authored fallback text into chat.
      }
      var finalMessage = Object.assign({
        id: ++msgId,
        role: 'agent',
        text: finalText,
        meta: meta,
        tools: streamedTools,
        ts: Date.now(),
        _turn_started_at: responseTurnStartedAt,
        agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
        agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
      }, messageMetadata || {});
      var renderedFinalMessage = finalMessage;
      var lastStable = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (lastStable && lastStable.role === 'agent' && lastStable._auto_fallback) {
        this.messages[this.messages.length - 1] = finalMessage;
        renderedFinalMessage = finalMessage;
      } else {
        renderedFinalMessage = this.pushAgentMessageDeduped(finalMessage, { dedupe_window_ms: 90000 }) || finalMessage;
      }
      this.clearTransientThinkingRowsCompat({ force: true });
      this.markAgentMessageComplete(renderedFinalMessage);
      if (renderedFinalMessage && typeof this._queueFinalWordTypingRender === 'function') {
        this._queueFinalWordTypingRender(renderedFinalMessage, String(renderedFinalMessage.text || ''), 10);
      }
      var wsFailure = responseHasToolCompletion ? null : this.extractRecoverableBackendFailure(finalText);
      if (responseAgentId) this._clearPendingWsRequest(responseAgentId);
      else this._clearPendingWsRequest();
      this.setAgentLiveActivity(responseAgentId || (this.currentAgent && this.currentAgent.id), 'idle');
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.scrollToBottom();
      this.requestContextTelemetry(false);
      this.maybeAddAutoModelSwitchNotice(wsAutoSwitchPrevious, wsRoute);
      this._pendingAutoModelSwitchBaseline = '';
      if (artifactDirectives && artifactDirectives.length) {
        this.resolveArtifactDirectives(artifactDirectives);
      }
      var self3 = this;
      if (wsFailure) {
        this.attemptAutomaticFailoverRecovery('ws_response', finalText, {
          remove_last_agent_failure: true
        }).then(function(recovered) {
          if (recovered) return;
          self3._inflightPayload = null;
          self3.refreshPromptSuggestions(true, 'post-response-failed-recover');
          self3.$nextTick(function() {
            var el = document.getElementById('msg-input'); if (el) el.focus();
            self3._processQueue();
          });
        });
      } else {
        this._inflightPayload = null;
        this.refreshPromptSuggestions(true, 'post-response');
        this.$nextTick(function() {
          var el = document.getElementById('msg-input'); if (el) el.focus();
          self3._processQueue();
        });
      }
    },
  };
}
