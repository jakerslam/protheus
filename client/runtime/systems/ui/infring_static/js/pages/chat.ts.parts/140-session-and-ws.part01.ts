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
            self.enforceLatestViewportDeterminism();
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

    enforceLatestViewportDeterminism() {
      var el = this.resolveMessagesScroller();
      if (!el) return false;
      if (!Array.isArray(this.messages) || this.messages.length < 1) return false;
      var blocks = el.querySelectorAll('.chat-message-block[data-msg-idx], .chat-message-block');
      if (!blocks || !blocks.length) {
        this.scrollToBottomImmediate({ force: true });
        return true;
      }
      var viewportTop = Number(el.scrollTop || 0);
      var viewportBottom = viewportTop + Math.max(0, Number(el.clientHeight || 0));
      var lastBottom = 0;
      for (var i = 0; i < blocks.length; i++) {
        var block = blocks[i];
        if (!block || block.offsetParent === null) continue;
        var bottom = Number(block.offsetTop || 0) + Math.max(0, Number(block.offsetHeight || 0));
        if (bottom > lastBottom) lastBottom = bottom;
      }
      if (!(lastBottom > 0)) return false;
      if (viewportTop > (lastBottom + 24) || viewportBottom < 24) {
        this.scrollToBottomImmediate({ force: true, tolerancePx: 999999 });
        this.stabilizeBottomScroll();
        return true;
      }
      return false;
    },

    ensureLiveThinkingRow: function(data) {
      var incomingStatus = String(
        data && (data.thinking_status || data.status_text) ? (data.thinking_status || data.status_text) : ''
      ).trim();
      if (incomingStatus && typeof this.normalizeThinkingStatusCandidate === 'function') {
        incomingStatus = this.normalizeThinkingStatusCandidate(incomingStatus);
      }
      var row = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (row && (row.thinking || row.streaming)) {
        row.thinking = true;
        row.streaming = true;
        if (!Number.isFinite(Number(row._stream_started_at))) row._stream_started_at = Date.now();
        row._stream_updated_at = Date.now();
        if (
          incomingStatus &&
          (
            !String(row.thinking_status || '').trim() ||
            (
              typeof this.isThinkingPlaceholderText === 'function' &&
              this.isThinkingPlaceholderText(row.thinking_status)
            )
          )
        ) {
          row.thinking_status = incomingStatus;
        }
        return row;
      }
      row = {
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: '',
        thinking: true,
        streaming: true,
        thinking_status: incomingStatus,
        tools: [],
        _stream_started_at: Date.now(),
        _stream_updated_at: Date.now(),
        ts: Date.now(),
        agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
        agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
      };
      this.messages.push(row);
      return row;
    },

    // Multi-session: load session list for current agent
    async loadSessions(agentId) {
      try {
        var data = await InfringAPI.get('/api/agents/' + agentId + '/sessions');
        var normalizedAgentId = typeof this.normalizeSessionAgentId === 'function'
          ? this.normalizeSessionAgentId(agentId)
          : String(agentId || '').trim().toLowerCase();
        var rows = data && Array.isArray(data.sessions) ? data.sessions : [];
        if (typeof this.normalizeSessionsList === 'function') {
          rows = this.normalizeSessionsList(rows, normalizedAgentId);
        }
        this.sessions = rows;
        if (!this._sessionsLastLoadedAtByAgent || typeof this._sessionsLastLoadedAtByAgent !== 'object') {
          this._sessionsLastLoadedAtByAgent = {};
        }
        this._sessionsLastLoadedAtByAgent[normalizedAgentId] = Date.now();
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
      var reconnectPending = false;
      var reconnectSyncInFlight = false;
      var isLiveConnection = function(eventAgentId) {
        if (Number(self._wsConnectSeq || 0) !== connectSeq) return false;
        if (String(self._wsAgent || '').trim() !== targetAgentId) return false;
        var eventId = String(eventAgentId || '').trim();
        return !eventId || eventId === targetAgentId;
      };
      var ensurePendingThinkingRow = function(statusText) {
        var nextStatus = String(statusText || '').trim();
        if (typeof self.isThinkingPlaceholderText === 'function' && self.isThinkingPlaceholderText(nextStatus)) {
          nextStatus = '';
        }
        var pendingRow = null;
        var rows = Array.isArray(self.messages) ? self.messages : [];
        for (var i = rows.length - 1; i >= 0; i--) {
          var row = rows[i];
          if (!row) continue;
          if (row.thinking || row.streaming) {
            pendingRow = row;
            break;
          }
          if (String(row.role || '').toLowerCase() === 'agent') break;
        }
        if (!pendingRow) {
          pendingRow = {
            id: ++msgId,
            role: 'agent',
            text: '',
            meta: '',
            thinking: true,
            streaming: true,
            thinking_status: nextStatus,
            tools: [],
            agent_id: targetAgentId,
            agent_name: self.currentAgent && self.currentAgent.name ? String(self.currentAgent.name) : '',
            ts: Date.now(),
          };
          self.messages.push(pendingRow);
        } else {
          pendingRow.thinking = true;
          pendingRow.streaming = true;
          if (!String(pendingRow.text || '').trim()) pendingRow.text = '';
          if (nextStatus && pendingRow.thinking_status !== nextStatus) pendingRow.thinking_status = nextStatus;
          pendingRow._stream_updated_at = Date.now();
        }
      };
      var syncPendingAfterReconnect = function(reason) {
        if (reconnectSyncInFlight) return;
        var pending = self._pendingWsRequest;
        if (!pending || String(pending.agent_id || '').trim() !== targetAgentId) return;
        reconnectSyncInFlight = true;
        ensurePendingThinkingRow('Reconnected. Syncing response...');
        self.setAgentLiveActivity(targetAgentId, 'working');
        Promise.resolve()
          .then(function() {
            return self.loadSessions(targetAgentId);
          })
          .catch(function() { return null; })
          .then(function() {
            var isActive = !!(self.currentAgent && String(self.currentAgent.id || '').trim() === targetAgentId);
            if (!isActive) return null;
            return self.loadSession(targetAgentId, true).catch(function() { return null; });
          })
          .then(function() {
            return self._recoverPendingWsRequest(reason || 'ws_reopen');
          })
          .catch(function() { return null; })
          .finally(function() {
            reconnectSyncInFlight = false;
          });
      };

      InfringAPI.wsConnect(targetAgentId, {
        onOpen: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = true;
          self.requestContextTelemetry(true);
          if (reconnectPending) {
            reconnectPending = false;
            syncPendingAfterReconnect('ws_reopen');
          } else if (!self.sending) {
            self.$nextTick(function() { self._processQueue(); });
          }
        },
        onMessage: function(data) {
          var dataAgentId = data && data.agent_id ? data.agent_id : '';
          if (!isLiveConnection(dataAgentId)) return;
          self.handleWsMessage(data);
        },
        onReconnect: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = false;
          reconnectPending = true;
          var pending = self._pendingWsRequest;
          if (pending && pending.agent_id) {
            ensurePendingThinkingRow('Connection interrupted. Reconnecting...');
            self.setAgentLiveActivity(pending.agent_id, 'working');
          }
        },
        onClose: function() {
          if (!isLiveConnection('')) return;
          Alpine.store('app').wsConnected = false;
          self._wsAgent = null;
          var pending = self._pendingWsRequest;
          if (self.sending && pending && pending.agent_id) {
            reconnectPending = true;
            self._clearTypingTimeout();
            ensurePendingThinkingRow('Connection interrupted. Reconnecting...');
            self.setAgentLiveActivity(pending.agent_id, 'working');
            self._recoverPendingWsRequest('ws_close');
            self.scrollToBottom();
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
            reconnectPending = true;
            self._clearTypingTimeout();
            ensurePendingThinkingRow('Connection interrupted. Reconnecting...');
            self.setAgentLiveActivity(pending.agent_id, 'working');
            self._recoverPendingWsRequest('ws_error');
            self.scrollToBottom();
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
      if (
        (targetId && this.isSystemThreadId && this.isSystemThreadId(targetId)) ||
        (!targetId && this.isSystemThreadActive && this.isSystemThreadActive())
      ) {
        if (!this.currentAgent || !this.isSystemThreadAgent || !this.isSystemThreadAgent(this.currentAgent)) {
          this.activateSystemThread({ preserve_if_empty: true });
        } else {
          this.currentAgent = this.makeSystemThreadAgent();
          this.setStoreActiveAgentId(this.currentAgent.id || null);
        }
        return;
      }
      if (!opts.force && this.shouldSuppressAgentInactive(targetId)) {
        return;
      }
      var reasonLabel = this.formatInactiveReason(reason || 'inactive');
      var noticeKey = targetId + '|' + reasonLabel;
      var self = this;

      this._clearTypingTimeout();
      this._clearPendingWsRequest(targetId);
      typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
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
      typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
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
            this.ensureLiveThinkingRow(data);
            this.scrollToBottom();
            this._resetTypingTimeout();
          }
          break;

        // New typing lifecycle
        case 'typing':
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
          break;

        case 'phase':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working');
          // Show tool/phase progress so the user sees the agent is working
          var phaseMsg = this.ensureLiveThinkingRow(data);
          if (phaseMsg && (phaseMsg.thinking || phaseMsg.streaming)) {
            var phaseDetailText = String(data && data.detail ? data.detail : '').trim();
            var phasePercent = Number(
              data && data.progress_percent != null
