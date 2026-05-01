// Chat multi-session list/create/switch action helpers.
'use strict';

function infringChatSessionActionMethods() {
  return {
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
        var chatStore = window.InfringChatStore;
        if (chatStore && chatStore.sessions) chatStore.sessions.set(rows);
        if (!this._sessionsLastLoadedAtByAgent || typeof this._sessionsLastLoadedAtByAgent !== 'object') {
          this._sessionsLastLoadedAtByAgent = {};
        }
        this._sessionsLastLoadedAtByAgent[normalizedAgentId] = Date.now();
      } catch(e) {
        this.sessions = [];
        var fallbackStore = window.InfringChatStore;
        if (fallbackStore && fallbackStore.sessions) fallbackStore.sessions.set([]);
      }
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
        this.connectChatWebSocket(this.currentAgent.id);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to switch session');
      }
    },
  };
}
