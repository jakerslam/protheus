      if (remainingMs <= 0) return this.isAgentPendingTermination(agent) ? '0m' : '';
      var totalMin = Math.max(1, Math.ceil(remainingMs / 60000));
      var monthMin = 30 * 24 * 60;
      if (totalMin >= monthMin) {
        return Math.max(1, Math.ceil(totalMin / monthMin)) + 'm';
      }
      if (totalMin >= 1440) {
        return Math.max(1, Math.ceil(totalMin / 1440)) + 'd';
      }
      if (totalMin >= 60) {
        return Math.max(1, Math.ceil(totalMin / 60)) + 'h';
      }
      return totalMin + 'm';
    },

    expiryCountdownCritical(agent) {
      if (agent && agent._timed_out_local === true) return false;
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      var totalMs = this.agentContractTotalMs(agent);
      var thresholdMs = 3600000;
      if (Number.isFinite(totalMs) && totalMs > 0) {
        thresholdMs = Math.min(3600000, Math.max(1, Math.floor(totalMs * 0.2)));
      }
      return remainingMs > 0 && remainingMs <= thresholdMs;
    },

    agentContractTotalMs(agent) {
      if (!agent || typeof agent !== 'object') return null;
      var contract = (agent.contract && typeof agent.contract === 'object') ? agent.contract : null;
      var durationMs = Number(agent.contract_duration_ms != null ? agent.contract_duration_ms : (contract && contract.duration_ms));
      if (Number.isFinite(durationMs) && durationMs > 0) return Math.floor(durationMs);
      var durationSeconds = Number(agent.contract_duration_seconds != null ? agent.contract_duration_seconds : (contract && contract.duration_seconds));
      if (Number.isFinite(durationSeconds) && durationSeconds > 0) return Math.floor(durationSeconds * 1000);
      var expiryMs = this.agentContractExpiryMs(agent);
      if (expiryMs <= 0) return null;
      var startedAt = String(
        agent.contract_started_at ||
        (contract && contract.started_at ? contract.started_at : '') ||
        agent.created_at ||
        (contract && contract.created_at ? contract.created_at : '') ||
        ''
      ).trim();
      var startedTs = Number(new Date(startedAt).getTime());
      if (Number.isFinite(startedTs) && startedTs > 0 && expiryMs > startedTs) {
        return Math.max(1000, Math.floor(expiryMs - startedTs));
      }
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null || remainingMs <= 0) return null;
      return Math.max(remainingMs, 3600000);
    },

    agentHeartStates(agent) {
      var totalHearts = 5;
      var hearts = [true, true, true, true, true];
      if (!agent || typeof agent !== 'object') return hearts;
      if (agent.is_system_thread) return hearts;
      if (agent._timed_out_local === true) return [false, false, false, false, false];
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) return [true];
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return hearts;
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) totalMs = Math.max(1, remainingMs);
      var ratio = Math.max(0, Math.min(1, remainingMs / totalMs));
      var filled = Math.ceil(ratio * totalHearts);
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) filled = 0;
      if (filled < 0) filled = 0;
      if (filled > totalHearts) filled = totalHearts;
      for (var i = 0; i < totalHearts; i++) {
        hearts[i] = i < filled;
      }
      return hearts;
    },

    agentHeartShowsInfinity(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread) return false;
      if (agent._timed_out_local === true) return false;
      return !this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent);
    },

    agentHeartMeterLabel(agent) {
      if (!agent || typeof agent !== 'object' || agent.is_system_thread) return '';
      if (agent._timed_out_local === true) return 'Time limit: timed out';
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) {
        return 'Time limit: unlimited';
      }
      var label = this.expiryCountdownLabel(agent);
      if (label) return 'Time remaining: ' + label;
      return 'Time limit active';
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
