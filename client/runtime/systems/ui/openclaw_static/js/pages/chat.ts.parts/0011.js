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
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text:
            'Automatic model recovery failed: ' +
            String(error && error.message ? error.message : error),
          meta: '',
          tools: [],
          system_origin: 'model:auto-recover:error',
          ts: Date.now()
        });
        this.scheduleConversationPersist();
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

    get filteredSlashCommands() {
      if (!this.slashFilter) return this.slashCommands;
      var f = this.slashFilter;
      return this.slashCommands.filter(function(c) {
        return c.cmd.toLowerCase().indexOf(f) !== -1 || c.desc.toLowerCase().indexOf(f) !== -1;
      });
    },

    // Clear any stuck typing indicator after 120s
    _resetTypingTimeout: function() {
      var self = this;
      if (self._typingTimeout) clearTimeout(self._typingTimeout);
      self._typingTimeout = setTimeout(function() {
        // Auto-clear stuck typing indicators
        var timeoutEnvelope = self.collectStreamedAssistantEnvelope();
        var timeoutThought = String(timeoutEnvelope.thought || '').trim();
        var timeoutTools = timeoutEnvelope.tools || [];
        var timeoutText = self.sanitizeToolText(String(timeoutEnvelope.text || '').trim());
        self._clearStreamingTypewriters();
        if (timeoutThought) {
          timeoutTools.unshift(self.makeThoughtToolCard(timeoutThought, Math.max(0, Date.now() - Number(self._responseStartedAt || Date.now()))));
        }
        self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; });
        if (!timeoutText) {
          timeoutText = self.defaultAssistantFallback(timeoutThought, timeoutTools);
        }
        if (timeoutText) {
          self.messages.push({
            id: ++msgId,
            role: 'agent',
            text: timeoutText,
            meta: 'transport timeout',
            tools: timeoutTools,
            ts: Date.now(),
            _auto_fallback: true
          });
        }
        self.sending = false;
        self._responseStartedAt = 0;
        self.tokenCount = 0;
        self._inflightPayload = null;
        self._clearPendingWsRequest();
        self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', 'idle');
        self.scheduleConversationPersist();
      }, 120000);
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
      var touchedPendingRows = false;
      var now = Date.now();
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          var activityAt = Number(row._stream_updated_at || row.ts || 0);
          var ageMs = activityAt > 0 ? Math.max(0, now - activityAt) : 0;
          // Keep pending rows pending while transport recovers; do not emit
          // premature fallback assistant messages for long-running thoughts.
          hasVisiblePending = true;
          if (row.thinking && !String(row.text || '').trim() && ageMs >= 12000) {
            row.text = 'Thinking...';
            touchedPendingRows = true;
          }
        }
      }
      if (touchedPendingRows) {
        this.scheduleConversationPersist();
      }
      var pending = this._pendingWsRequest && this._pendingWsRequest.agent_id ? this._pendingWsRequest : null;
      var hasPendingWs = !!pending;
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
          if (pendingAgeMs >= 30000) {
            this._clearPendingWsRequest();
            hasPendingWs = false;
          }
        }
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

    _recoverPendingWsRequest: async function(reason) {
      if (this._pendingWsRecovering) return;
      var pending = this._pendingWsRequest;
      if (!pending || !pending.agent_id) return;
      this._pendingWsRecovering = true;
      var agentId = String(pending.agent_id);
      var startedAt = Number(pending.started_at || Date.now());
      var recoverStartedAt = Date.now();
      var maxRecoverMs = 60000;
      var resolved = false;
      for (var attempt = 0; attempt < 120; attempt++) {
        if (!this._pendingWsRequest || String(this._pendingWsRequest.agent_id || '') !== agentId) {
          break;
        }
        if ((Date.now() - recoverStartedAt) > maxRecoverMs) {
          break;
        }
        try {
          var sessionData = await InfringAPI.get('/api/agents/' + encodeURIComponent(agentId) + '/session');
          var normalized = this.normalizeSessionMessages(sessionData);
          var hasFreshAgentReply = normalized.some(function(msg) {
            var role = String(msg && msg.role ? msg.role : '').toLowerCase();
            var ts = Number(msg && msg.ts ? msg.ts : 0);
            var text = String(msg && msg.text ? msg.text : '').trim();
            return role === 'agent' && text && ts >= startedAt;
          });
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
          await new Promise(function(resolve) { setTimeout(resolve, 500); });
        }
      }

      var stillActiveAgent = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
      if (!resolved && stillActiveAgent) {
        this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: 'Connection dropped before the agent reply was delivered. Please retry.',
          meta: '',
          tools: [],
          system_origin: 'transport:recovery',
          ts: Date.now()
        });
        this.scrollToBottom();
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
    },

    async executeSlashCommand(cmd, cmdArgs) {
      this.showSlashMenu = false;
      this.inputText = '';
      var self = this;
      cmdArgs = cmdArgs || '';
      switch (cmd) {
        case '/help':
          self.messages.push({ id: ++msgId, role: 'system', text: self.slashCommands.map(function(c) { return '`' + c.cmd + '` — ' + c.desc; }).join('\n'), meta: '', tools: [], system_origin: 'slash:help' });
          self.scrollToBottom();
          break;
        case '/agents':
          location.hash = 'agents';
          break;
        case '/new':
          if (self.currentAgent) {
            InfringAPI.post('/api/agents/' + self.currentAgent.id + '/session/reset', {}).then(function() {
              self.messages = [];
              InfringToast.success('Session reset');
            }).catch(function(e) { InfringToast.error('Reset failed: ' + e.message); });
          }
          break;
        case '/compact':
          if (self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Compacting session...', meta: '', tools: [], system_origin: 'slash:compact' });
            InfringAPI.post('/api/agents/' + self.currentAgent.id + '/session/compact', {}).then(function(res) {
              self.messages.push({ id: ++msgId, role: 'system', text: res.message || 'Compaction complete', meta: '', tools: [], system_origin: 'slash:compact' });
              self.scrollToBottom();
            }).catch(function(e) { InfringToast.error('Compaction failed: ' + e.message); });
          }
          break;
        case '/stop':
          self.stopAgent();
          break;
        case '/usage':
          if (self.currentAgent) {
            var approxTokens = self.messages.reduce(function(sum, m) { return sum + Math.round((m.text || '').length / 4); }, 0);
            self.messages.push({ id: ++msgId, role: 'system', text: '**Session Usage**\n- Messages: ' + self.messages.length + '\n- Approx tokens: ~' + approxTokens, meta: '', tools: [], system_origin: 'slash:usage' });
            self.scrollToBottom();
          }
          break;
        case '/think':
          if (cmdArgs === 'on') {
            self.thinkingMode = 'on';
          } else if (cmdArgs === 'off') {
            self.thinkingMode = 'off';
          } else if (cmdArgs === 'stream') {
            self.thinkingMode = 'stream';
          } else {
            // Cycle: off -> on -> stream -> off
            if (self.thinkingMode === 'off') self.thinkingMode = 'on';
            else if (self.thinkingMode === 'on') self.thinkingMode = 'stream';
            else self.thinkingMode = 'off';
          }
          var modeLabel = self.thinkingMode === 'stream' ? 'enabled (streaming reasoning)' : (self.thinkingMode === 'on' ? 'enabled' : 'disabled');
          self.messages.push({ id: ++msgId, role: 'system', text: 'Extended thinking **' + modeLabel + '**. ' +
            (self.thinkingMode === 'stream' ? 'Reasoning tokens will appear in a collapsible panel.' :
             self.thinkingMode === 'on' ? 'The agent will show its reasoning when supported by the model.' :
             'Normal response mode.'), meta: '', tools: [], system_origin: 'slash:think' });
          self.scrollToBottom();
          break;
