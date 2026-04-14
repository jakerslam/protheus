        this.tokenCount = 0;
        this._clearTypingTimeout();
        this._clearPendingWsRequest(agentId);
        this.setAgentLiveActivity(agentId, 'idle');
        await this._sendPayload(
          payload.final_text,
          Array.isArray(payload.uploaded_files) ? payload.uploaded_files : [],
          Array.isArray(payload.msg_images) ? payload.msg_images : [],
          { retry_from_failover: true }
        );
        return true;
      } catch (error) {
        this.pushSystemMessage({
          text:
            'Automatic model recovery failed: ' +
            String(error && error.message ? error.message : error),
          meta: '',
          tools: [],
          system_origin: 'model:auto-recover:error',
          ts: Date.now(),
          dedupe_window_ms: 15000
        });
        return false;
      } finally {
        this._inflightFailoverInProgress = false;
      }
    },

    // Fetch dynamic slash commands from server
    fetchCommands: function() {
      var self = this;
      InfringAPI.get('/api/commands').then(function(data) {
        if (data.commands && data.commands.length) {
          // Build a set of known cmds to avoid duplicates
          var existing = {};
          self.slashCommands.forEach(function(c) { existing[c.cmd] = true; });
          data.commands.forEach(function(c) {
            if (!existing[c.cmd]) {
              self.slashCommands.push({ cmd: c.cmd, desc: c.desc || '', source: c.source || 'server' });
              existing[c.cmd] = true;
            }
          });
        }
      }).catch(function() { /* silent — use hardcoded list */ });
    },

    // Keep thinking indicators alive while work is still in-flight.
    // Only hard-timeout once no pending activity remains or the request is
    // genuinely stale far beyond expected runtime.
    _resetTypingTimeout: function() {
      var self = this;
      if (self._typingTimeout) clearTimeout(self._typingTimeout);
      self._typingTimeout = setTimeout(function() {
        var hasPending = typeof self.hasLivePendingResponse === 'function'
          ? self.hasLivePendingResponse()
          : false;
        var hardStale = typeof self.pendingResponseExceededHardTimeout === 'function'
          ? self.pendingResponseExceededHardTimeout()
          : false;
        if (hasPending && !hardStale) {
          self._resetTypingTimeout();
          return;
        }
        // Transport timeout: do not fabricate assistant content.
        self._clearStreamingTypewriters();
        typeof self.clearTransientThinkingRows === 'function' ? self.clearTransientThinkingRows({ force: true }) : (self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
        self.pushSystemMessage({
          text: 'Response timed out before delivery. Please retry.',
          meta: '',
          tools: [],
          system_origin: 'transport:timeout',
          ts: Date.now(),
          dedupe_window_ms: 60000
        });
        self.sending = false;
        self._responseStartedAt = 0;
        self.tokenCount = 0;
        self._inflightPayload = null;
        self._clearPendingWsRequest();
        self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', 'idle');
        self.scheduleConversationPersist();
      }, 120000);
    },

    hasLivePendingResponse: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          return true;
        }
      }
      return !!(this._pendingWsRequest && this._pendingWsRequest.agent_id);
    },

    pendingResponseExceededHardTimeout: function() {
      var now = Date.now();
      var startedAt = Number(this._responseStartedAt || 0);
      if ((!Number.isFinite(startedAt) || startedAt <= 0) && this._pendingWsRequest) {
        startedAt = Number(this._pendingWsRequest.started_at || 0);
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) {
        var rows = Array.isArray(this.messages) ? this.messages : [];
        for (var i = rows.length - 1; i >= 0; i--) {
          var row = rows[i];
          if (!row) continue;
          if (!(row.thinking || row.streaming || (row.terminal && row.thinking))) continue;
          var rowStartedAt = Number(row._stream_started_at || row._stream_updated_at || row.ts || 0);
          if (Number.isFinite(rowStartedAt) && rowStartedAt > 0) {
            startedAt = rowStartedAt;
            break;
          }
        }
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) return false;
      return Math.max(0, now - startedAt) >= 900000;
    },

    _clearTypingTimeout: function() {
      if (this._typingTimeout) {
        clearTimeout(this._typingTimeout);
        this._typingTimeout = null;
      }
    },

    _clearMessageTypewriter: function(message) {
      if (!message || typeof message !== 'object') return;
      if (message._typewriterTimer) {
        clearTimeout(message._typewriterTimer);
        message._typewriterTimer = null;
      }
      message._typewriterRunning = false;
    },

    _clearStreamingTypewriters: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        this._clearMessageTypewriter(rows[i]);
      }
    },

    _queueStreamTypingRender: function(message, nextText) {
      if (!message || typeof message !== 'object') return;
      var targetText = String(nextText || '');
      message._streamTargetText = targetText;
      if (message._typewriterRunning) return;
      var self = this;
      message._typewriterRunning = true;

      var step = function() {
        if (!message || !message.streaming) {
          self._clearMessageTypewriter(message);
          return;
        }
        var target = String(message._streamTargetText || '');
        var current = String(message.text || '');
        if (target === current) {
          self._clearMessageTypewriter(message);
          return;
        }
        // If sanitization trims or rewrites content, snap to the newest safe value.
        if (target.length < current.length || target.indexOf(current) !== 0) {
          message.text = target;
          self._clearMessageTypewriter(message);
          self.scrollToBottom();
          return;
        }
        var remaining = target.length - current.length;
        var take = Math.max(1, Math.min(8, Math.ceil(remaining / 4)));
        message.text = target.slice(0, current.length + take);
        self.scrollToBottom();
        if (message.text.length < target.length) {
          message._typewriterTimer = setTimeout(step, 14);
          return;
        }
        self._clearMessageTypewriter(message);
      };

      step();
    },
    _reconcileSendingState: function() {
      if (!this.sending) return false;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var hasVisiblePending = false;
      var now = Date.now();
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          var activityAt = Number(row._stream_updated_at || row.ts || 0);
          var ageMs = activityAt > 0 ? Math.max(0, now - activityAt) : 0;
          hasVisiblePending = true;
        }
      }
      var pending = this._pendingWsRequest && this._pendingWsRequest.agent_id ? this._pendingWsRequest : null;
      var hasPendingWs = !!pending;
      if (!hasVisiblePending && hasPendingWs && typeof this.ensureLiveThinkingRow === 'function') {
        var keepRow = this.ensureLiveThinkingRow({
          agent_id: String(pending.agent_id || ''),
          agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : ''
        });
        if (keepRow) {
          keepRow.thinking = true;
          keepRow.streaming = true;
          if (!Number.isFinite(Number(keepRow._stream_started_at))) keepRow._stream_started_at = now;
          keepRow._stream_updated_at = now;
          if (!String(keepRow.text || '').trim()) keepRow.text = '';
          hasVisiblePending = true;
        }
      }
      if (pending) {
        var pendingAgentId = String(pending.agent_id || '');
        var currentAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
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
    _setPendingWsRequest: function(agentId, messageText) {
      var id = String(agentId || '').trim();
      if (!id) return;
      this._pendingWsRequest = {
        agent_id: id,
        message_text: String(messageText || '').trim(),
        started_at: Date.now(),
      };
      this._pendingWsRecovering = false;
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
        var store = Alpine.store('app');
        if (!store) return;
        if (typeof store.markAgentPreviewUnread === 'function') {
          store.markAgentPreviewUnread(id, unread !== false);
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

    _hasAgentReplyAfterLatestUser: function(rows) {
      var list = Array.isArray(rows) ? rows : [];
      if (!list.length) return false;
      var lastUserIndex = -1;
      for (var i = list.length - 1; i >= 0; i -= 1) {
        var user = list[i] || {};
        var userRole = String(user.role || '').toLowerCase();
        if (userRole !== 'user') continue;
        var userText = String(user.text || '').trim();
        if (!userText) continue;
        lastUserIndex = i;
        break;
      }
      if (lastUserIndex < 0) return false;
      for (var j = lastUserIndex + 1; j < list.length; j += 1) {
        var msg = list[j] || {};
        if (msg.thinking || msg.streaming || msg._auto_fallback) continue;
        var role = String(msg.role || '').toLowerCase();
        if (role !== 'agent' && role !== 'assistant') continue;
        var text = String(msg.text || '').trim();
        var hasToolPayload = Array.isArray(msg.tools) && msg.tools.length > 0;
        if (!text && !hasToolPayload) continue;
        if (text && /^thinking\.\.\.$/i.test(text)) continue;
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
          var sessionData = await InfringAPI.get('/api/agents/' + encodeURIComponent(agentId) + '/session');
          if (!recoveryStillCurrent()) break;
          var normalized = this.normalizeSessionMessages(sessionData);
          var hasFreshAgentReply =
            this._pendingRequestReplyObserved(normalized, pending, startedAt) ||
            this._recentAgentReplyObserved(normalized, startedAt) ||
            this._hasAgentReplyAfterLatestUser(normalized);
          if (!hasFreshAgentReply) {
            await new Promise(function(resolve) { setTimeout(resolve, 650); });
            continue;
          }
          if (!this.conversationCache) this.conversationCache = {};
          this.conversationCache[String(agentId)] = {
            saved_at: Date.now(),
            token_count: Number(this.contextApproxTokens || 0),
            messages: JSON.parse(JSON.stringify(normalized || [])),
          };
          try {
            var appStore = Alpine.store('app');
            if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
              appStore.saveAgentChatPreview(agentId, this.conversationCache[String(agentId)].messages);
            }
          } catch(_) {}
          var isActive = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
          if (isActive) {
            this.messages = this.mergeModelNoticesForAgent(agentId, JSON.parse(JSON.stringify(normalized || [])));
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
        if (!resolved && this._hasAgentReplyAfterLatestUser(localRows)) {
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
        this.pushSystemMessage({
          text: 'Connection dropped before the agent reply was delivered. Please retry.',
          meta: '',
          tools: [],
          system_origin: 'transport:recovery',
          ts: Date.now(),
          dedupe_window_ms: 60000,
          dedupe_scope: 40,
          auto_scroll: true
        });
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

    async executeSlashCommand(cmd, cmdArgs) {
      this.showSlashMenu = false;
