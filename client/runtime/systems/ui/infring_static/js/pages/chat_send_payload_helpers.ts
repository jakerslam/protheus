// Chat send payload transport helpers.
'use strict';

function infringChatSendPayloadMethods() {
  return {
    async _sendPayload(finalText, uploadedFiles, msgImages, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var ensuredAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!ensuredAgent && !opts.agent_id) {
        this.sending = false;
        this._responseStartedAt = 0;
        return;
      }
      this.sending = true;
      var targetAgentId = String(
        opts.agent_id || (ensuredAgent && ensuredAgent.id) || (this.currentAgent && this.currentAgent.id) || ''
      ).trim();
      if (!targetAgentId) {
        this.sending = false;
        this._responseStartedAt = 0;
        return;
      }
      this.setAgentLiveActivity(targetAgentId, 'typing');
      var safeFiles = Array.isArray(uploadedFiles) ? uploadedFiles.slice() : [];
      var safeImages = Array.isArray(msgImages) ? msgImages.slice() : [];
      if (
        !opts.retry_from_failover ||
        !this._inflightPayload ||
        String(this._inflightPayload.agent_id || '') !== targetAgentId
      ) {
        this._inflightPayload = {
          agent_id: targetAgentId,
          final_text: String(finalText || ''),
          uploaded_files: safeFiles,
          msg_images: safeImages,
          failover_attempted: !!opts.retry_from_failover,
          created_at: Date.now()
        };
      } else {
        this._inflightPayload.final_text = String(finalText || '');
        this._inflightPayload.uploaded_files = safeFiles;
        this._inflightPayload.msg_images = safeImages;
        this._inflightPayload.retry_started_at = Date.now();
      }
      this._pendingAutoModelSwitchBaseline = this.captureAutoModelSwitchBaseline();
      var preflightMeta = '';
      if (!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) {
        this.connectWs(targetAgentId);
        var waitStarted = Date.now();
        while ((!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) && (Date.now() - waitStarted) < 1500) {
          await new Promise(function(resolve) { setTimeout(resolve, 75); });
        }
      }
      var wsPayload = { type: 'message', content: finalText };
      if (uploadedFiles && uploadedFiles.length) wsPayload.attachments = uploadedFiles;
      if (InfringAPI.wsSend(wsPayload)) {
        this._setPendingWsRequest(targetAgentId, finalText);
        this._responseStartedAt = Date.now();
        this.messages.push({
          id: ++msgId,
          role: 'agent',
          text: '',
          meta: preflightMeta || '',
          thinking: true,
          streaming: true,
          tools: [],
          ts: Date.now()
        });
        this.scrollToBottom();
        this.scheduleConversationPersist();
        return;
      }
      this._clearPendingWsRequest(targetAgentId);
      if (!InfringAPI.isWsConnected()) {
        InfringToast.info('Using HTTP mode (no streaming)');
      }
      this.messages.push({
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: preflightMeta || '',
        thinking: true,
        tools: [],
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
      var httpStartedAt = Date.now();
      var handedOffToRecovery = false;

      try {
        var httpBody = { message: finalText };
        if (uploadedFiles && uploadedFiles.length) httpBody.attachments = uploadedFiles;
        var httpAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();
        if (!httpAutoSwitchPrevious) httpAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
        var res = await InfringAPI.post('/api/agents/' + targetAgentId + '/message', httpBody);
        this.applyContextTelemetry(res);
        var httpRoute = this.applyAutoRouteTelemetry(res);
        this.clearTransientThinkingRowsCompat({ force: true, thinking_only: true });
        var httpMeta = (res.input_tokens || 0) + ' in / ' + (res.output_tokens || 0) + ' out';
        if (res.cost_usd != null) httpMeta += ' | $' + res.cost_usd.toFixed(4);
        if (res.iterations) httpMeta += ' | ' + res.iterations + ' iter';
        var httpDurationMs = Math.max(0, Date.now() - httpStartedAt);
        var httpDuration = this.formatResponseDuration(httpDurationMs);
        if (httpDuration) httpMeta += ' | ' + httpDuration;
        var httpRouteMeta = this.formatAutoRouteMeta(httpRoute);
        if (httpRouteMeta) httpMeta += ' | ' + httpRouteMeta;
        var httpTools = typeof this.responseToolRowsFromPayload === 'function'
          ? this.responseToolRowsFromPayload(res, 'http-tool')
          : [];
        var httpHasToolCompletion = typeof this.responseHasAuthoritativeToolCompletion === 'function'
          ? this.responseHasAuthoritativeToolCompletion(res, httpTools)
          : httpTools.length > 0;
        var httpMessageMetadata = typeof this.assistantTurnMetadataFromPayload === 'function' ? this.assistantTurnMetadataFromPayload(res, httpTools) : {};
        var httpPayloadText = typeof this.assistantTextFromPayload === 'function'
          ? this.assistantTextFromPayload(res)
          : String(res.response || '');
        var httpText = this.stripModelPrefix(this.sanitizeToolText(httpPayloadText || ''));
        var httpArtifactDirectives = this.extractArtifactDirectives(httpText);
        var httpSplit = this.extractThinkingLeak(httpText);
        if (httpSplit.thought) {
          httpTools.unshift(this.makeThoughtToolCard(httpSplit.thought, httpDurationMs));
          httpText = httpSplit.content || '';
        }
        httpText = this.stripArtifactDirectivesFromText(httpText);
        var httpCompact = String(httpText || '').replace(/\s+/g, ' ').trim();
        if (
          typeof this.isThinkingPlaceholderText === 'function' &&
          this.isThinkingPlaceholderText(httpCompact)
        ) {
          httpText = '';
        }
        if (!String(httpText || '').trim()) {
          // Policy: do not inject system-authored fallback text into chat.
          this.maybeAddAutoModelSwitchNotice(httpAutoSwitchPrevious, httpRoute || preflightRoute);
          this._pendingAutoModelSwitchBaseline = '';
          this._clearPendingWsRequest(targetAgentId);
          this._inflightPayload = null;
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this.scheduleConversationPersist();
          return;
        }
        var httpFailure = httpHasToolCompletion ? null : this.extractRecoverableBackendFailure(httpText);
        if (httpFailure) {
          this._clearPendingWsRequest(targetAgentId);
          this._pendingAutoModelSwitchBaseline = '';
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          handedOffToRecovery = await this.attemptAutomaticFailoverRecovery('http_response', httpText, {
            remove_last_agent_failure: false
          });
          if (handedOffToRecovery) {
            this.scheduleConversationPersist();
            return;
          }
        }
        var httpMessage = Object.assign({
          id: ++msgId,
          role: 'agent',
          text: httpText,
          meta: httpMeta,
          tools: httpTools,
          ts: Date.now(),
          agent_id: res && res.agent_id ? String(res.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
          agent_name: res && res.agent_name ? String(res.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
        }, httpMessageMetadata || {});
        var pushedHttpMessage = this.pushAgentMessageDeduped(httpMessage, { dedupe_window_ms: 90000 }) || httpMessage;
        this.markAgentMessageComplete(pushedHttpMessage);
        if (pushedHttpMessage && typeof this._queueFinalWordTypingRender === 'function') {
          this._queueFinalWordTypingRender(pushedHttpMessage, String(pushedHttpMessage.text || ''), 10);
        }
        this.maybeAddAutoModelSwitchNotice(httpAutoSwitchPrevious, httpRoute || preflightRoute);
        this._pendingAutoModelSwitchBaseline = '';
        this._clearPendingWsRequest(targetAgentId);
        this._inflightPayload = null;
        if (httpArtifactDirectives && httpArtifactDirectives.length) {
          this.resolveArtifactDirectives(httpArtifactDirectives);
        }
        this.scheduleConversationPersist();
      } catch(e) {
        this.clearTransientThinkingRowsCompat({ force: true, thinking_only: true });
        this._clearPendingWsRequest(targetAgentId);
        this._pendingAutoModelSwitchBaseline = '';
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._clearTypingTimeout();
        this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
        var rawHttpError = String(e && e.message ? e.message : e || '');
        var lowerHttpError = rawHttpError.toLowerCase();
        var isAbortError =
          (e && String(e.name || '').toLowerCase() === 'aborterror') ||
          lowerHttpError.indexOf('this operation was aborted') >= 0 ||
          lowerHttpError.indexOf('operation was aborted') >= 0;
        if (isAbortError) {

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          this._inflightPayload = null;
          this.refreshPromptSuggestions(true, 'post-http-abort');
          this.scheduleConversationPersist();
          return;
        }
        if (
          !opts.retry_from_agent_rebind &&
          (lowerHttpError.indexOf('agent_not_found') >= 0 || lowerHttpError.indexOf('agent not found') >= 0)
        ) {
          var reboundAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
          if (!reboundAgent || String(reboundAgent.id || '') === String(targetAgentId || '')) {
            reboundAgent = await this.rebindCurrentAgentAuthoritative({
              preferred_id: targetAgentId,
              clear_when_missing: true
            });
          }
          var reboundAgentId = reboundAgent && reboundAgent.id ? String(reboundAgent.id) : '';
          if (reboundAgentId && reboundAgentId !== targetAgentId) {
            this.addNoticeEvent({
              notice_label:
                'Active agent reference expired. Switched to ' +
                String(reboundAgent.name || reboundAgent.id || reboundAgentId) +
                ' and retried.',
              notice_type: 'warn',
              ts: Date.now(),
            });
            await this._sendPayload(finalText, uploadedFiles, msgImages, {
              agent_id: reboundAgentId,
              retry_from_agent_rebind: true,
            });
            return;
          }
        }
        var noModelsError =
          lowerHttpError.indexOf('no_models_available') >= 0 ||
          lowerHttpError.indexOf('no models available') >= 0;
        if (noModelsError) {
          this.injectNoModelsGuidance('send_error');
          this._inflightPayload = null;
          this.scheduleConversationPersist();
          return;
        }
        handedOffToRecovery = await this.attemptAutomaticFailoverRecovery(
          'http_error',
          rawHttpError,
          { remove_last_agent_failure: false }
        );
        if (!handedOffToRecovery) {
          var rawSendErrorText = String(rawHttpError || (e && e.message) || '').replace(/\s+/g, ' ').trim();
          var lowerSendErrorText = rawSendErrorText.toLowerCase();
          var isTransientDisconnectError =
            lowerSendErrorText === 'fetch failed' ||
            lowerSendErrorText === 'failed to fetch' ||
            lowerSendErrorText === 'connect failed' ||
            lowerSendErrorText.indexOf('gateway connect failed') >= 0;
          var normalizedSendErrorText = (function(message) {
            var raw = String(message || '').replace(/\s+/g, ' ').trim();
            var lower = raw.toLowerCase();
            if (!raw || lower === 'unknown error') return 'Connection failed before the runtime returned a usable response. Try again after the gateway is reachable.';
            if (lower.indexOf('pairing required') >= 0) return 'Gateway pairing is required. Open Settings, pair this dashboard with the gateway, then try again.';
            if (
              lower.indexOf('device identity required') >= 0 ||
              lower.indexOf('secure context') >= 0 ||
              lower.indexOf('https/localhost') >= 0
            ) return 'This action requires HTTPS or localhost. Reopen the dashboard from a trusted origin, then try again.';
            if (
              lower.indexOf('unauthorized') >= 0 ||
              lower.indexOf('token mismatch') >= 0 ||
              lower.indexOf('token missing') >= 0 ||
              lower.indexOf('auth failed') >= 0 ||
              lower.indexOf('authentication') >= 0
            ) return 'Gateway authentication failed. Verify the API token or password in Settings, then retry.';
            if (
              lower === 'fetch failed' ||
              lower === 'failed to fetch' ||
              lower === 'connect failed' ||
              lower.indexOf('gateway connect failed') >= 0
            ) return 'Gateway connect failed. Check runtime availability, pairing, and auth settings, then retry.';
            return 'Connection error: ' + raw;
          })(rawHttpError || (e && e.message) || '');
          if (!isTransientDisconnectError) {
            console.warn('[chat http error]', normalizedSendErrorText);
            InfringToast.error(normalizedSendErrorText);
          }
          this._inflightPayload = null;
        } else {
          return;

        }
      }
      if (handedOffToRecovery) return;
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._responseStartedAt = 0;
      this.sending = false;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input'); if (el) el.focus();
        self._processQueue();
      });
    },
  };
}
