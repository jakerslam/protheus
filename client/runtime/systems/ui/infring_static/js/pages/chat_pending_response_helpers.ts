function infringChatPendingResponseMethods() {
  return {
    _reconcileSendingState: function() {
      if (!this.sending) return false;
      var pending = this._pendingWsRequest && this._pendingWsRequest.agent_id ? this._pendingWsRequest : null;
      var hasPendingWs = !!pending;
      var inflight = this._inflightPayload && typeof this._inflightPayload === 'object'
        ? this._inflightPayload
        : null;
      var pendingStatusText = pending && String(pending.status_text || '').trim()
        ? String(pending.status_text || '').trim()
        : 'Waiting for workflow completion...';
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var hasVisiblePending = false;
      var now = Date.now();
      var currentAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          if (
            (!String(row.thinking_status || '').trim() ||
              (
                typeof this.isThinkingPlaceholderText === 'function' &&
                this.isThinkingPlaceholderText(row.thinking_status)
              )) &&
            (!pending || !pending.agent_id || !String(row.agent_id || '').trim() || String(row.agent_id || '').trim() === String(pending.agent_id || '').trim())
          ) {
            row.thinking_status = pendingStatusText;
          }
          hasVisiblePending = true;
        }
      }
      if (!hasVisiblePending && hasPendingWs && typeof this.ensureLiveThinkingRow === 'function') {
        var keepRow = this.ensureLiveThinkingRow({
          agent_id: String(pending.agent_id || ''),
          agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
          status_text: pendingStatusText
        });
        if (keepRow) {
          keepRow.thinking = true;
          keepRow.streaming = true;
          if (!Number.isFinite(Number(keepRow._stream_started_at))) keepRow._stream_started_at = now;
          keepRow._stream_updated_at = now;
          if (!String(keepRow.text || '').trim()) keepRow.text = '';
          if (
            !String(keepRow.thinking_status || '').trim() ||
            (
              typeof this.isThinkingPlaceholderText === 'function' &&
              this.isThinkingPlaceholderText(keepRow.thinking_status)
            )
          ) {
            keepRow.thinking_status = pendingStatusText;
          }
          hasVisiblePending = true;
        }
      }
      if (pending) {
        var pendingAgentId = String(pending.agent_id || '');
        var pendingAgeMs = Math.max(0, now - Number(pending.started_at || now));
        if (currentAgentId && pendingAgentId && pendingAgentId !== currentAgentId) {
          hasPendingWs = false;
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('cross_agent_pending');
          }
        } else if (pendingAgeMs >= 12000) {
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('stale_pending');
          }
          if (pendingAgeMs >= 900000) {
            this._clearPendingWsRequest();
            hasPendingWs = false;
          }
        }
      }
      if (!hasVisiblePending && !hasPendingWs && inflight) {
        var inflightAgentId = String(inflight.agent_id || currentAgentId || '').trim();
        var inflightStartedAt = Number(
          this._responseStartedAt ||
          (pending && pending.started_at) ||
          inflight.created_at ||
          0
        );
        var hasRecentInflightReply = false;
        if (
          Number.isFinite(inflightStartedAt) &&
          inflightStartedAt > 0 &&
          typeof this._recentAgentReplyObserved === 'function'
        ) {
          hasRecentInflightReply = this._recentAgentReplyObserved(rows, inflightStartedAt);
        }
        if (inflightAgentId && !hasRecentInflightReply) {
          this._setPendingWsRequest(
            inflightAgentId,
            String(inflight.final_text || ''),
            {
              started_at: Number.isFinite(inflightStartedAt) && inflightStartedAt > 0
                ? inflightStartedAt
                : Date.now(),
              status_text: pendingStatusText
            }
          );
          pending = this._pendingWsRequest;
          hasPendingWs = !!pending;
          if (!Number.isFinite(Number(this._responseStartedAt)) || Number(this._responseStartedAt) <= 0) {
            this._responseStartedAt = Number(
              (pending && pending.started_at) ||
              inflight.created_at ||
              Date.now()
            );
          }
          if (typeof this.ensureLiveThinkingRow === 'function') {
            var restoredPending = this.ensureLiveThinkingRow({
              agent_id: inflightAgentId,
              agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '',
              status_text: pendingStatusText
            });
            if (restoredPending) {
              restoredPending.thinking = true;
              restoredPending.streaming = true;
              restoredPending._stream_updated_at = now;
              if (!Number.isFinite(Number(restoredPending._stream_started_at))) {
                restoredPending._stream_started_at = now;
              }
              hasVisiblePending = true;
            }
          }
        }
      }
      if (hasVisiblePending || hasPendingWs) {
        var keepBusyAgentId = '';
        if (pending && pending.agent_id) keepBusyAgentId = String(pending.agent_id || '').trim();
        if (!keepBusyAgentId) keepBusyAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
        if (keepBusyAgentId) this.setAgentLiveActivity(keepBusyAgentId, 'working');
      }
      if (hasVisiblePending || hasPendingWs) return false;
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._clearTypingTimeout();
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '', 'idle');
      return true;
    },
    _setPendingWsRequest: function(agentId, messageText, options) {
      var id = String(agentId || '').trim();
      if (!id) return;
      var opts = options && typeof options === 'object' ? options : {};
      var startedAt = Number(opts.started_at || 0);
      if (!Number.isFinite(startedAt) || startedAt <= 0) startedAt = Date.now();
      var statusText = String(opts.status_text || 'Waiting for workflow completion...').trim();
      if (!statusText) statusText = 'Waiting for workflow completion...';
      this._pendingWsRequest = {
        agent_id: id,
        message_text: String(messageText || '').trim(),
        status_text: statusText,
        started_at: startedAt,
      };
      this._pendingWsRecovering = false;
    },

    _setPendingWsStatusText: function(agentId, statusText) {
      if (!this._pendingWsRequest) return;
      var pendingAgentId = String(this._pendingWsRequest.agent_id || '').trim();
      var targetAgentId = String(agentId || '').trim();
      if (targetAgentId && pendingAgentId && pendingAgentId !== targetAgentId) return;
      var nextStatus = String(statusText || '').trim();
      if (!nextStatus) return;
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        var normalized = this.normalizeThinkingStatusCandidate(nextStatus);
        if (normalized) nextStatus = normalized;
      }
      if (!nextStatus) return;
      this._pendingWsRequest.status_text = nextStatus;
    },

    _clearPendingWsRequest: function(agentId) {
      if (!this._pendingWsRequest) return;
      if (!agentId) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
        return;
      }
      var current = String(this._pendingWsRequest.agent_id || '').trim();
      if (current && current === String(agentId)) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
      }
    },

    _markAgentPreviewUnread: function(agentId, unread) {
      var id = String(agentId || '').trim();
      if (!id) return;
      try {
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
        if (!store) return;
        var markAgentPreviewUnread = bridge && typeof bridge.method === 'function'
          ? bridge.method('markAgentPreviewUnread')
          : null;
        if (typeof markAgentPreviewUnread === 'function') {
          markAgentPreviewUnread(id, unread !== false);
        } else if (store.agentChatPreviews && store.agentChatPreviews[id]) {
          store.agentChatPreviews[id].unread_response = unread !== false;
        }
      } catch(_) {}
    },

    _pendingRequestReplyObserved: function(normalizedMessages, pendingRequest, startedAt) {
      var rows = Array.isArray(normalizedMessages) ? normalizedMessages : [];
      if (!rows.length) return false;

      var pendingText = this.sanitizeToolText(String(
        pendingRequest && pendingRequest.message_text ? pendingRequest.message_text : ''
      )).trim();
      var started = Number(startedAt || (pendingRequest && pendingRequest.started_at) || 0);
      var skewToleranceMs = 15000;
      var lastMatchingUserIndex = -1;

      if (pendingText) {
        for (var i = 0; i < rows.length; i++) {
          var userMsg = rows[i] || {};
          var userRole = String(userMsg.role || '').toLowerCase();
          if (userRole !== 'user') continue;
          var userText = this.sanitizeToolText(String(userMsg.text || '')).trim();
          if (userText && userText === pendingText) {
            lastMatchingUserIndex = i;
          }
        }
      }

      for (var j = 0; j < rows.length; j++) {
        var msg = rows[j] || {};
        var role = String(msg.role || '').toLowerCase();
        var text = String(msg.text || '').trim();
        var hasToolPayload = Array.isArray(msg.tools) && msg.tools.length > 0;
        var isAgentRole = role === 'agent' || role === 'assistant';
        if (!isAgentRole || (!text && !hasToolPayload)) continue;

        var ts = Number(msg.ts || 0);
        if (started > 0 && ts > 0 && (ts + skewToleranceMs) >= started) {
          return true;
        }
        if (lastMatchingUserIndex >= 0 && j > lastMatchingUserIndex) {
          return true;
        }
      }
      return false;
    },

    _recentAgentReplyObserved: function(rows, startedAt) {
      var list = Array.isArray(rows) ? rows : [];
      if (!list.length) return false;
      var started = Number(startedAt || 0);
      var skewToleranceMs = 20000;
      for (var i = list.length - 1; i >= 0; i -= 1) {
        var msg = list[i] || {};
        if (msg.thinking || msg.streaming) continue;
        var role = String(msg.role || '').toLowerCase();
        if (role !== 'agent' && role !== 'assistant') continue;
        var text = String(msg.text || '').trim();
        var hasToolPayload = Array.isArray(msg.tools) && msg.tools.length > 0;
        if (!text && !hasToolPayload) continue;
        if (msg._auto_fallback) continue;
        if (text && /^thinking\.\.\.$/i.test(text)) continue;
        var ts = Number(msg.ts || 0);
        if (started > 0 && ts > 0 && (ts + skewToleranceMs) < started) continue;
        return true;
      }
      return false;
    },

    _recoverPendingWsRequest: async function(reason) {
      if (this._pendingWsRecovering) return;
      var pending = this._pendingWsRequest;
      if (!pending || !pending.agent_id) return;
      this._pendingWsRecovering = true;
      var recoverySeq = Number(this._pendingWsRecoverySeq || 0) + 1;
      this._pendingWsRecoverySeq = recoverySeq;
      var agentId = String(pending.agent_id);
      var startedAt = Number(pending.started_at || Date.now());
      var recoverStartedAt = Date.now();
      var maxRecoverMs = 15000;
      var resolved = false;
      var self = this;
      var recoveryStillCurrent = function() {
        if (Number(self._pendingWsRecoverySeq || 0) !== recoverySeq) return false;
        if (!self._pendingWsRequest || String(self._pendingWsRequest.agent_id || '') !== agentId) return false;
        return true;
      };
      for (var attempt = 0; attempt < 30; attempt++) {
        if (!recoveryStillCurrent()) {
          break;
        }
        if ((Date.now() - recoverStartedAt) > maxRecoverMs) {
          break;
        }
        try {
          var sessionData = await InfringAPI.get('/api/agents/' + encodeURIComponent(agentId) + '/session?limit=80');
          if (!recoveryStillCurrent()) break;
          var normalized = this.normalizeSessionMessages(sessionData, { requireWindow: true });
          var hasFreshAgentReply =
            this._pendingRequestReplyObserved(normalized, pending, startedAt) ||
            this._recentAgentReplyObserved(normalized, startedAt);
          if (!hasFreshAgentReply) {
            await new Promise(function(resolve) { setTimeout(resolve, 650); });
            continue;
          }
          if (!this.conversationCache) this.conversationCache = {};
          this.conversationCache[String(agentId)] = {
            saved_at: Date.now(),
            token_count: Number(this.contextApproxTokens || 0),
            messages: this.sanitizeConversationForCache(normalized || []),
          };
          try {
            var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
              ? InfringSharedShellServices.appStore
              : null;
            var saveAgentChatPreview = bridge && typeof bridge.method === 'function'
              ? bridge.method('saveAgentChatPreview')
              : null;
            if (typeof saveAgentChatPreview === 'function') saveAgentChatPreview(agentId, this.conversationCache[String(agentId)].messages);
          } catch(_) {}
          var isActive = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
          if (isActive) {
            this.messages = this.mergeModelNoticesForAgent(agentId, this.normalizeSessionMessages({ messages: normalized || [] }));
            this.scrollToBottom();
          } else {
            this._markAgentPreviewUnread(agentId, true);
          }
          this.persistConversationCache();
          resolved = true;
          break;
        } catch(_) {
          if (!recoveryStillCurrent()) break;
          await new Promise(function(resolve) { setTimeout(resolve, 500); });
        }
      }

      if (!recoveryStillCurrent()) {
        this._pendingWsRecovering = false;
        return;
      }
      var stillActiveAgent = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
      if (!resolved && stillActiveAgent) {
        var localRows = Array.isArray(this.messages) ? this.messages : [];
        if (this._pendingRequestReplyObserved(localRows, pending, startedAt)) {
          resolved = true;
        }
        if (!resolved && this._recentAgentReplyObserved(localRows, startedAt)) {
          resolved = true;
        }
        if (!resolved && this._recentAgentReplyObserved(localRows, Math.max(0, startedAt - 120000))) {
          resolved = true;
        }
      }
      var pendingAgeMs = Math.max(0, Date.now() - Number(startedAt || Date.now()));
      if (!resolved && stillActiveAgent && pendingAgeMs < 900000) {
        this._pendingWsRecovering = false;
        return;
      }
      if (!resolved && stillActiveAgent) {
        typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
        // Do not inject transport-authored text into the chat transcript.
      }
      if (!resolved && !stillActiveAgent) {
        this._pendingWsRecovering = false;
        return;
      }
      this.setAgentLiveActivity(agentId, 'idle');
      if (stillActiveAgent) {
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('msg-input'); if (el) el.focus();
          self._processQueue();
        });
      }
      this._clearPendingWsRequest(agentId);
      this._pendingWsRecovering = false;
    },

  };
}
