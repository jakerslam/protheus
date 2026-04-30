// Chat websocket runtime error event handlers.
'use strict';

function infringChatWebSocketErrorEventMethods() {
  return {
    handleWsErrorEvent: function(data) {
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
      this._clearTypingTimeout();
      this._clearStreamingTypewriters();
      this._pendingAutoModelSwitchBaseline = '';
      var rawError = String(data && data.content ? data.content : 'unknown_error');
      var errorText = 'Error: ' + rawError;
      var lowerError = rawError.toLowerCase();
      if (
        lowerError.indexOf('this operation was aborted') >= 0 ||
        lowerError.indexOf('operation was aborted') >= 0
      ) {
        this.clearTransientThinkingRowsCompat({ force: true });
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._inflightPayload = null;
        this.refreshPromptSuggestions(true, 'post-ws-abort');
        return;
      }
      if (lowerError.indexOf('backend_http_404') >= 0) {
        // Soft-ignore noisy command-surface 404s so they do not get injected
        // into the conversation stream after a successful agent response.
        this.clearTransientThinkingRowsCompat({ preserve_running_tools: true, preserve_pending_ws: true });
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._inflightPayload = null;
        this.requestContextTelemetry(false);
        var selfSuppressed = this;
        this.$nextTick(function() {
          var el = document.getElementById('msg-input'); if (el) el.focus();
          selfSuppressed._processQueue();
        });
        this.refreshPromptSuggestions(true, 'post-suppressed-404');
        return;
      }
      if (lowerError.indexOf('agent contract terminated') !== -1 || lowerError.indexOf('agent_contract_terminated') !== -1) {
        this.handleAgentInactive(
          this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
          'contract_terminated',
          { noticeText: errorText }
        );
        return;
      }
      if (lowerError.indexOf('agent is inactive') !== -1 || lowerError.indexOf('agent_inactive') !== -1) {
        this.handleAgentInactive(
          this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
          'inactive',
          { noticeText: errorText }
        );
        return;
      }
      if (lowerError.indexOf('agent not found') !== -1 || lowerError.indexOf('agent_not_found') !== -1) {
        this.clearTransientThinkingRowsCompat({ preserve_running_tools: true, preserve_pending_ws: true });
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        var priorAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
        var inflight = this._inflightPayload && typeof this._inflightPayload === 'object' ? this._inflightPayload : null;
        var rawNotFound = rawError;
        var selfRebound = this;
        Promise.resolve()
          .then(function() {
            return selfRebound.rebindCurrentAgentAuthoritative({
              preferred_id: priorAgentId,
              clear_when_missing: true
            });
          })
          .then(function(reboundAgent) {
            var reboundAgentId = reboundAgent && reboundAgent.id ? String(reboundAgent.id) : '';
            if (
              reboundAgentId &&
              reboundAgentId !== priorAgentId &&
              inflight &&
              !inflight._agent_rebind_attempted
            ) {
              inflight._agent_rebind_attempted = true;
              inflight.agent_id = reboundAgentId;
              selfRebound.addNoticeEvent({
                notice_label:
                  'Active agent reference expired. Switched to ' +
                  String(reboundAgent.name || reboundAgent.id || reboundAgentId) +
                  ' and retried.',
                notice_type: 'warn',
                ts: Date.now(),
              });
              return selfRebound._sendPayload(
                inflight.final_text || '',
                Array.isArray(inflight.uploaded_files) ? inflight.uploaded_files : [],
                Array.isArray(inflight.msg_images) ? inflight.msg_images : [],
                { agent_id: reboundAgentId, retry_from_agent_rebind: true }
              );
            }
            return selfRebound
              .attemptAutomaticFailoverRecovery('ws_error', rawNotFound, {
                remove_last_agent_failure: false
              })
              .then(function(recovered) {
                if (recovered) return;
                console.warn('[chat ws error]', rawNotFound);
                InfringToast.error('Runtime websocket error. See console for details.');
                selfRebound._inflightPayload = null;
              });
          })
          .catch(function() {});
        return;
      }
      this.clearTransientThinkingRowsCompat({ preserve_running_tools: true, preserve_pending_ws: true });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      var self2 = this;
      this.attemptAutomaticFailoverRecovery('ws_error', rawError, {
        remove_last_agent_failure: false
      }).then(function(recovered) {
        if (recovered) return;
        console.warn('[runtime error]', errorText);
        InfringToast.error('Runtime error. See console for details.');
        self2._inflightPayload = null;
        self2.scrollToBottom();
        self2.$nextTick(function() {
          var el = document.getElementById('msg-input'); if (el) el.focus();
          self2._processQueue();
        });
        self2.refreshPromptSuggestions(true, 'post-error');
      });
    },
  };
}
