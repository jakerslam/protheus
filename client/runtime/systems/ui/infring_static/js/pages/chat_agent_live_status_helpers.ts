// Chat agent live-status and websocket naming shims.
'use strict';

function infringChatAgentLiveStatusMethods() {
  return {
    // Backward-compat shim for legacy callers during naming migration.
    connectWs(agentId) {
      this.connectChatWebSocket(agentId);
    },

    formatInactiveReason: function(reason) {
      var raw = String(reason || '').trim();
      if (!raw) return 'inactive';
      raw = raw.replace(/^agent_contract_/, '');
      raw = raw.replace(/^rogue_/, '');
      raw = raw.replace(/_/g, ' ').trim();
      return raw || 'inactive';
    },

    setAgentLiveActivity(agentId, state) {
      var id = String(agentId || '').trim();
      if (!id) return;
      try {
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var setLiveActivity = bridge && typeof bridge.method === 'function'
          ? bridge.method('setAgentLiveActivity')
          : null;
        if (typeof setLiveActivity === 'function') setLiveActivity(id, state);
      } catch(_) {}
    },

    refreshAgentRosterFromShellStore: function() {
      try {
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var refreshAgents = bridge && typeof bridge.method === 'function'
          ? bridge.method('refreshAgents')
          : null;
        if (typeof refreshAgents === 'function') return refreshAgents();
      } catch(_) {}
      return null;
    },

    setChatWebSocketConnectedState: function(connected) {
      var isConnected = connected === true;
      try {
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        if (bridge && typeof bridge.set === 'function') {
          bridge.set('wsConnected', isConnected);
        }
      } catch(_) {}
      var chatStore = window.InfringChatStore;
      if (chatStore && chatStore.wsConnected) chatStore.wsConnected.set(isConnected);
    },

    isCurrentChatWebSocketConnection: function(connectSeq, targetAgentId, eventAgentId) {
      if (Number(this._wsConnectSeq || 0) !== Number(connectSeq || 0)) return false;
      var targetId = String(targetAgentId || '').trim();
      if (String(this._wsAgent || '').trim() !== targetId) return false;
      var eventId = String(eventAgentId || '').trim();
      return !eventId || eventId === targetId;
    },

    applyAgentRosterUpdateFromWebSocket: function(agents) {
      var nextAgents = (Array.isArray(agents) ? agents : []).filter((row) => {
        if (!row || !row.id) return false;
        return !(this.isArchivedAgentRecord && this.isArchivedAgentRecord(row));
      });
      try {
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        if (bridge && typeof bridge.assign === 'function') {
          bridge.assign({ agents: nextAgents, agentCount: nextAgents.length });
        }
      } catch(_) {}
      return nextAgents;
    },

    stopAgent: function() {
      if (!this.currentAgent) return;
      var self = this;
      InfringAPI.post('/api/agents/' + this.currentAgent.id + '/stop', {}).then(function(res) {
        self.handleStopResponse(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', res || {});
      }).catch(function(e) {
        var raw = String(e && e.message ? e.message : 'stop_failed');
        var lower = raw.toLowerCase();
        if (lower.indexOf('agent_inactive') >= 0 || lower.indexOf('inactive') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'inactive',
            { noticeText: 'Agent is now inactive.' }
          );
          return;
        }
        if (lower.indexOf('agent_contract_terminated') >= 0 || lower.indexOf('contract terminated') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'contract_terminated',
            { noticeText: 'Agent contract terminated.' }
          );
          return;
        }
        InfringToast.error('Stop failed: ' + raw);
      });
    },

    killAgent() {
      if (!this.currentAgent) return;
      var self = this;
      var name = this.currentAgent.name;
      InfringToast.confirm('Stop Agent', 'Stop agent "' + name + '"? The agent will be shut down.', async function() {
        try {
          self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id, {
            activity: 'idle',
            display_label: 'Idle',
            source: 'shell_optimistic',
            optimistic: true
          });
          await InfringAPI.del('/api/agents/' + self.currentAgent.id);
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          InfringToast.success('Agent "' + name + '" stopped');
          self.refreshAgentRosterFromShellStore();
        } catch(e) {
          InfringToast.error('Failed to stop agent: ' + e.message);
        }
      });
    },

    // Preferred naming for websocket event entrypoint.
    handleChatWebSocketMessage(data) {
      this.handleWsMessage(data);
    },
  };
}
