// Chat websocket connection lifecycle helpers.
'use strict';

function infringChatWebSocketConnectionMethods() {
  return {
    connectChatWebSocket(agentId) {
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
        return self.isCurrentChatWebSocketConnection(connectSeq, targetAgentId, eventAgentId);
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
        self.syncActiveChatMessages();
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
          self.setChatWebSocketConnectedState(true);
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
          self.handleChatWebSocketMessage(data);
        },
        onReconnect: function() {
          if (!isLiveConnection('')) return;
          self.setChatWebSocketConnectedState(false);
          reconnectPending = true;
          var pending = self._pendingWsRequest;
          if (pending && pending.agent_id) {
            ensurePendingThinkingRow('Connection interrupted. Reconnecting...');
            self.setAgentLiveActivity(pending.agent_id, 'working');
          }
        },
        onClose: function() {
          if (!isLiveConnection('')) return;
          self.setChatWebSocketConnectedState(false);
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
            Promise.resolve(self.refreshAgentRosterFromShellStore()).then(function() {
              var stillLive = self.resolveAgent(self.currentAgent.id);
              if (!stillLive && !self.shouldSuppressAgentInactive(self.currentAgent.id)) {
                self.handleAgentInactive(self.currentAgent.id, 'inactive');
              }
            }).catch(function() {});
          }
        },
        onError: function() {
          if (!isLiveConnection('')) return;
          self.setChatWebSocketConnectedState(false);
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
  };
}
