      if (remainingMs <= 0) return this.isAgentPendingTermination(agent) ? '0m' : '';
      var totalMin = Math.max(1, Math.ceil(remainingMs / 60000));
      var day = Math.floor(totalMin / 1440);
      var hour = Math.floor((totalMin % 1440) / 60);
      var min = totalMin % 60;
      var parts = [];
      if (day > 0) parts.push(day + 'd');
      if (hour > 0) parts.push(hour + 'h');
      parts.push(min + 'm');
      return parts.join(' ');
    },

    expiryCountdownCritical(agent) {
      if (agent && agent._timed_out_local === true) return false;
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      return remainingMs > 0 && remainingMs <= 60000;
    },

    async pollStatus() {
      var store = this.getAppStore();
      if (!store) {
        this.connected = false;
        this.connectionState = 'connecting';
        return;
      }
      if (typeof store.checkStatus === 'function') await store.checkStatus();
      var now = Date.now();
      var shouldRefreshAgents =
        !store.agentsHydrated ||
        (store.connectionState !== 'connected') ||
        (now - Number(store._lastAgentsRefreshAt || 0)) >= 12000;
      if (shouldRefreshAgents) {
        if (typeof store.refreshAgents === 'function') await store.refreshAgents();
      }
      this.reconcileArchivedAgentIdsWithLiveAgents();
      if (typeof this.syncChatSidebarTopologyOrderFromAgents === 'function') {
        this.syncChatSidebarTopologyOrderFromAgents();
      }
      if (typeof this.sanitizeCollapsedAgentHoverState === 'function') {
        this.sanitizeCollapsedAgentHoverState();
      }
      this.connected = store.connected;
      this.version = store.version;
      this.agentCount = store.agentCount;
      this.connectionState = store.connectionState || (store.connected ? 'connected' : 'disconnected');
      this.queueConnectionIndicatorState(this.connectionState);
      this.wsConnected = InfringAPI.isWsConnected();
      if (!this.bootSelectionApplied && store.agentsHydrated && !store.agentsLoading) {
        await this.applyBootChatSelection();
      }
      this.scheduleSidebarScrollIndicators();
      this.releaseBootSplash(false);
    }
  };
}
