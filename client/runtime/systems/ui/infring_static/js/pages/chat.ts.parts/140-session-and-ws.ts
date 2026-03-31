        }
      } catch(e) {
        if (!loadStillCurrent()) return;
        var restoredFromCache = false;
        try {
          restoredFromCache = self.restoreAgentConversation(agentId);
        } catch(_) {
          restoredFromCache = false;
        }
        if (!restoredFromCache && !keepCurrent && (!Array.isArray(self.messages) || !self.messages.length)) {
          var errText = String(e && e.message ? e.message : 'session_load_failed').trim();
          self.messages = [{
            id: ++msgId,
            role: 'system',
            text: 'Unable to load this agent session right now (' + errText + ').',
            meta: '',
            tools: [],
            system_origin: 'session:load_error',
            ts: Date.now()
          }];
        }
      }
      finally {
        if (self._sessionLoadSeq === loadSeq) {
          await new Promise(function(resolve) {
            self.$nextTick(function() {
              self.scrollToBottomImmediate();
              self.stabilizeBottomScroll();
              self.pinToLatestOnOpen(null, { maxFrames: 22 });
              self.scheduleMessageRenderWindowUpdate();
              resolve();
            });
          });
          await self.waitForSessionRender(agentId, loadSeq);
          if (self._sessionLoadSeq === loadSeq) {
            self.pinToLatestOnOpen(null, { maxFrames: 24 });
            self.sessionLoading = false;
          }
          self._reconcileSendingState();
          if (!self.showFreshArchetypeTiles) {
            self.refreshPromptSuggestions(false);
          }
        }
      }
    },

    waitForAnimationFrame() {
      return new Promise(function(resolve) {
        if (typeof requestAnimationFrame === 'function') {
          requestAnimationFrame(function() { resolve(); });
        } else {
          setTimeout(resolve, 16);
        }
      });
    },

    async waitForSessionRender(agentId, loadSeq) {
      var self = this;
      var expectedAgent = String(agentId || '');
      var hasSessionMessages = Array.isArray(this.messages) && this.messages.length > 0;
      var minFrames = hasSessionMessages ? 2 : 1;
      var maxFrames = hasSessionMessages ? 42 : 6;
      var messagesEl = null;

      // Let Alpine commit template updates before probing for rendered blocks.
      await this.waitForAnimationFrame();
      await this.waitForAnimationFrame();

      for (var frame = 0; frame < maxFrames; frame++) {
        if (self._sessionLoadSeq !== loadSeq) return;
        if (!self.currentAgent || String(self.currentAgent.id || '') !== expectedAgent) return;
        if (!messagesEl) messagesEl = self.resolveMessagesScroller();
        if (!messagesEl) {
          await self.waitForAnimationFrame();
          continue;
        }

        self.scheduleMessageRenderWindowUpdate(messagesEl);

        if (!hasSessionMessages) {
          if (frame >= minFrames) return;
          await self.waitForAnimationFrame();
          continue;
        }

        var blockCount = messagesEl.querySelectorAll('.chat-message-block').length;
        var renderedCount = messagesEl.querySelectorAll('.chat-message-block .message, .chat-message-block .message-placeholder-shell, .chat-day-anchor, .chat-day-divider').length;
        if (blockCount > 0 && renderedCount > 0 && frame >= minFrames) {
          return;
        }

        await self.waitForAnimationFrame();
      }
    },

    // Multi-session: load session list for current agent
    async loadSessions(agentId) {
      try {
        var data = await InfringAPI.get('/api/agents/' + agentId + '/sessions');
        this.sessions = data.sessions || [];
      } catch(e) { this.sessions = []; }
    },

    // Multi-session: create a new session
    async createSession() {
      if (!this.currentAgent) return;
      this.cacheCurrentConversation();
      var label = prompt('Session name (optional):');
      if (label === null) return; // cancelled
      try {
        await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/sessions', {
          label: label.trim() || undefined
        });
        await this.loadSessions(this.currentAgent.id);
        await this.loadSession(this.currentAgent.id);
        if (typeof InfringToast !== 'undefined') InfringToast.success('New session created');
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to create session');
      }
    },

    // Multi-session: switch to an existing session
    async switchSession(sessionId) {
      if (!this.currentAgent) return;
      this.cacheCurrentConversation();
      try {
        await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/sessions/' + sessionId + '/switch', {});
        await this.loadSession(this.currentAgent.id);
        await this.loadSessions(this.currentAgent.id);
        // Reconnect WebSocket for new session
        this._wsAgent = null;
        this.connectWs(this.currentAgent.id);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to switch session');
      }
    },

    connectWs(agentId) {
      var targetAgentId = String(agentId || '').trim();
      if (!targetAgentId) return;
      if (this._wsAgent === targetAgentId && InfringAPI.isWsConnected()) return;
      var connectSeq = Number(this._wsConnectSeq || 0) + 1;
      this._wsConnectSeq = connectSeq;
      this._wsAgent = targetAgentId;
      var self = this;
      var isLiveConnection = function(eventAgentId) {
        if (Number(self._wsConnectSeq || 0) !== connectSeq) return false;
        if (String(self._wsAgent || '').trim() !== targetAgentId) return false;
        var eventId = String(eventAgentId || '').trim();
        return !eventId || eventId === targetAgentId;
      };

      InfringAPI.wsConnect(targetAgentId, {
        onOpen: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = true;
          self.requestContextTelemetry(true);
        },
        onMessage: function(data) {
          var dataAgentId = data && data.agent_id ? data.agent_id : '';
          if (!isLiveConnection(dataAgentId)) return;
          self.handleWsMessage(data);
        },
        onClose: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = false;
          self._wsAgent = null;
          var pending = self._pendingWsRequest;
          if (self.sending && pending && pending.agent_id) {
            self._clearTypingTimeout();
            self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            self.sending = false;
            self._responseStartedAt = 0;
            self._recoverPendingWsRequest('ws_close');
          }
          if (self.currentAgent && self.currentAgent.id) {
            Alpine.store('app').refreshAgents().then(function() {
              var stillLive = self.resolveAgent(self.currentAgent.id);
              if (!stillLive && !self.shouldSuppressAgentInactive(self.currentAgent.id)) {
                self.handleAgentInactive(self.currentAgent.id, 'inactive');
              }
            }).catch(function() {});
          }
        },
        onError: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = false;
          self._wsAgent = null;
          var pending = self._pendingWsRequest;
          if (self.sending && pending && pending.agent_id) {
            self._clearTypingTimeout();
            self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            self.sending = false;
            self._responseStartedAt = 0;
            self._recoverPendingWsRequest('ws_error');
          }
        }
      });
    },

    formatInactiveReason: function(reason) {
      var raw = String(reason || '').trim();
      if (!raw) return 'inactive';
      raw = raw.replace(/^agent_contract_/, '');
      raw = raw.replace(/^rogue_/, '');
      raw = raw.replace(/_/g, ' ').trim();
      return raw || 'inactive';
    },

    handleAgentInactive: function(agentId, reason, options) {
      var opts = options || {};
      var targetId = String(agentId || (this.currentAgent && this.currentAgent.id) || '').trim();
      if (!opts.force && this.shouldSuppressAgentInactive(targetId)) {
        return;
      }
      var reasonLabel = this.formatInactiveReason(reason || 'inactive');
      var noticeKey = targetId + '|' + reasonLabel;
      var self = this;

      this._clearTypingTimeout();
      this._clearPendingWsRequest(targetId);
      this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._inflightPayload = null;
      this.setAgentLiveActivity(targetId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle');

      if (!opts.silentNotice && noticeKey !== this._lastInactiveNoticeKey) {
        var noticeText = opts.noticeText || '';
        if (!noticeText) {
          noticeText = targetId
            ? ('Agent ' + targetId + ' is now inactive (' + reasonLabel + ').')
            : ('Agent is now inactive (' + reasonLabel + ').');
        }
        this.messages.push({ id: ++msgId, role: 'system', text: noticeText, meta: '', tools: [], system_origin: 'agent:inactive', ts: Date.now() });
        this._lastInactiveNoticeKey = noticeKey;
      }

      if (targetId && this._wsAgent && String(this._wsAgent) === targetId) {
        InfringAPI.wsDisconnect();
        this._wsAgent = null;
      }

      if (this.currentAgent && this.currentAgent.id && (!targetId || String(this.currentAgent.id) === targetId)) {
        this.currentAgent = null;
        this.setStoreActiveAgentId(null);
        this.showAgentDrawer = false;
      }

      this.scrollToBottom();
      this.$nextTick(function() { self._processQueue(); });

      try { Alpine.store('app').refreshAgents(); } catch(_) {}
    },

    setAgentLiveActivity(agentId, state) {
      var id = String(agentId || '').trim();
      if (!id) return;
      try {
        var store = Alpine.store('app');
        if (store && typeof store.setAgentLiveActivity === 'function') {
          store.setAgentLiveActivity(id, state);
        }
      } catch(_) {}
    },

    handleStopResponse: function(agentId, payload) {
      var result = payload && typeof payload === 'object' ? payload : {};
      var reasonRaw = String(result.reason || result.error || '').trim();
      var reason = reasonRaw || (result.contract_terminated ? 'contract_terminated' : '');
      var state = String(result.state || '').trim().toLowerCase();
      var reasonLower = reason.toLowerCase();
      var isInactive =
        !!result.archived ||
        !!result.contract_terminated ||
        state === 'inactive' ||
        state === 'archived' ||
        state === 'terminated' ||
        String(result.type || '').toLowerCase() === 'agent_archived' ||
        reasonLower.indexOf('inactive') >= 0 ||
        reasonLower.indexOf('terminated') >= 0;

      if (isInactive) {
        this.handleAgentInactive(
          agentId,
          reason || (result.contract_terminated ? 'contract_terminated' : 'inactive'),
          result.message ? { noticeText: String(result.message) } : {}
        );
        return;
      }

      this.setAgentLiveActivity(agentId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle');
      this._clearTypingTimeout();
      this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
      this.messages.push({ id: ++msgId, role: 'system', text: result.message || 'Run cancelled', meta: '', tools: [], system_origin: 'agent:stop', ts: Date.now() });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() { self._processQueue(); });
      try { Alpine.store('app').refreshAgents(); } catch(_) {}
    },

    handleWsMessage(data) {
      var eventAgentId = String(data && data.agent_id ? data.agent_id : '').trim();
      var activeWsAgentId = String(this._wsAgent || '').trim();
      if (eventAgentId && activeWsAgentId && eventAgentId !== activeWsAgentId) {
        return;
      }
      switch (data.type) {
        case 'connected':
          var connectedAgentId = String(data && data.agent_id ? data.agent_id : '').trim();
          if (connectedAgentId) {
            var selectedAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
            if (activeWsAgentId && connectedAgentId !== activeWsAgentId) break;
            if (selectedAgentId && connectedAgentId !== selectedAgentId) break;
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
          }
          break;

        case 'context_state':
          this.applyContextTelemetry(data);
          break;

        // Legacy thinking event (backward compat)
        case 'thinking':
          if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
            var thinkLabel = data.level ? 'Thinking (' + data.level + ')...' : 'Processing...';
            this.messages.push({
              id: ++msgId,
              role: 'agent',
              text: '*' + thinkLabel + '*',
              meta: '',
              thinking: true,
              streaming: true,
              tools: [],
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            });
            this.scrollToBottom();
            this._resetTypingTimeout();
          } else if (data.level) {
            var lastThink = this.messages[this.messages.length - 1];
            if (lastThink && lastThink.thinking) lastThink.text = '*Thinking (' + data.level + ')...*';
          }
          break;

        // New typing lifecycle
        case 'typing':
          if (data.state === 'start') {
            this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
            if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '*Processing...*',
                meta: '',
                thinking: true,
                streaming: true,
                thinking_status: 'Preparing response...',
                tools: [],
                agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
                agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
              });
              this.scrollToBottom();
            }
            this._resetTypingTimeout();
          } else if (data.state === 'tool') {
            this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working');
            var typingMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
            if (typingMsg && (typingMsg.thinking || typingMsg.streaming)) {
              var typingTool = String(data.tool || 'tool').trim() || 'tool';
              typingMsg.text = '*Using ' + typingTool + '...*';
              typingMsg.thinking_status = 'Calling ' + typingTool + '...';
            }
            this._resetTypingTimeout();
          } else if (data.state === 'stop') {
            this._clearTypingTimeout();
          }
          break;

        case 'phase':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working');
          // Show tool/phase progress so the user sees the agent is working
          var phaseMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (phaseMsg && (phaseMsg.thinking || phaseMsg.streaming)) {
            var phaseName = String(data && data.phase ? data.phase : '').trim().toLowerCase();
            var phaseDetailText = String(data && data.detail ? data.detail : '').trim();
            var phasePercent = Number(
              data && data.progress_percent != null
                ? data.progress_percent
                : (data && data.percent != null ? data.percent : NaN)
            );
            if (Number.isFinite(phasePercent)) {
              phaseMsg.progress = {
                percent: Math.max(0, Math.min(100, Math.round(phasePercent))),
                label: data && data.phase ? ('Progress · ' + String(data.phase)) : 'Progress'
              };
            }
            // Skip phases that have no user-meaningful display text — "streaming"
            // and "done" are lifecycle signals, not status to show in the chat bubble.
            if (data.phase === 'streaming' || data.phase === 'done') {
              break;
            }
            // Context warning: show prominently as a separate system message
            if (data.phase === 'context_warning') {
              var cwDetail = data.detail || 'Context limit reached.';
              this.messages.push({ id: ++msgId, role: 'system', text: cwDetail, meta: '', tools: [], system_origin: 'context:warning' });
              phaseMsg.thinking_status = 'Context window warning';
            } else if (data.phase === 'thinking') {
              var thoughtChunk = String(data.detail || '').trim();
              if (thoughtChunk) {
                phaseMsg._thoughtText = this.appendThoughtChunk(phaseMsg._thoughtText, thoughtChunk);
                phaseMsg._reasoning = phaseMsg._thoughtText;
                phaseMsg.isHtml = true;
                phaseMsg.thoughtStreaming = true;
                phaseMsg.text = this.renderLiveThoughtHtml(phaseMsg._thoughtText);
                phaseMsg.thinking_status = 'Reasoning through context...';
              } else if (phaseMsg.thinking) {
                phaseMsg.text = 'Thinking...';
                if (!phaseMsg.thinking_status) phaseMsg.thinking_status = 'Reasoning through context...';
              }
            } else if (phaseMsg.thinking) {
              // Only update text on messages still in thinking state (not yet
              // receiving streamed content) to avoid overwriting accumulated text.
              var phaseDetail;
              if (data.phase === 'tool_use') {
                phaseDetail = 'Using ' + (data.detail || 'tool') + '...';
              } else if (data.phase === 'thinking') {
                phaseDetail = 'Thinking...';
              } else {
                phaseDetail = data.detail || 'Working...';
              }
              phaseMsg.text = phaseDetail;
              if (phaseName === 'tool_use') {
                var toolPhaseName = phaseDetail || String(data && data.tool ? data.tool : '').trim() || 'tool';
                phaseMsg.thinking_status = 'Calling ' + toolPhaseName + '...';
              } else if (phaseDetail) {
                phaseMsg.thinking_status = phaseDetail;
              } else if (phaseName) {
                phaseMsg.thinking_status = phaseName.replace(/[_-]+/g, ' ');
              }
            }
            if (!phaseMsg.thinking_status && phaseDetailText) {
              phaseMsg.thinking_status = phaseDetailText;
            }
          }
          this.scrollToBottom();
          break;
