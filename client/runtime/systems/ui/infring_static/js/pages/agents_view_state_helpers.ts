// Agents page computed view/filter/format helpers.
'use strict';

function infringAgentsViewStateMethods() {
  return {
    get agents() {
      var store = this.shellAppStore();
      var rows = Array.isArray(store && store.agents) ? store.agents : [];
      var pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
      rows = rows.filter(function(agent) {
        if (!agent || !agent.id) return false;
        if (store && typeof store.isArchivedLikeAgent === 'function') {
          if (store.isArchivedLikeAgent(agent)) return false;
        } else {
          if (agent.archived === true) return false;
          var state = String(agent.state || '').trim().toLowerCase();
          if (state.indexOf('archived') >= 0 || state.indexOf('inactive') >= 0 || state.indexOf('terminated') >= 0) return false;
        }
        return true;
      });
      if (!pendingFreshId) return rows;
      return rows.filter(function(agent) {
        return String((agent && agent.id) || '').trim() !== pendingFreshId;
      });
    },

    get filteredAgents() {
      var f = this.filterState;
      if (f === 'all') return this.agents;
      return this.agents.filter(function(a) { return a.state.toLowerCase() === f; });
    },

    get runningCount() {
      return this.agents.filter(function(a) { return a.state === 'Running'; }).length;
    },

    get stoppedCount() {
      return this.agents.filter(function(a) { return a.state !== 'Running'; }).length;
    },

    get activeLifecycleAgents() {
      var rows = this.agentLifecycle && Array.isArray(this.agentLifecycle.active_agents)
        ? this.agentLifecycle.active_agents
        : [];
      return rows;
    },

    get terminatedAgents() {
      var rows = this.agentLifecycle && Array.isArray(this.agentLifecycle.terminated_recent)
        ? this.agentLifecycle.terminated_recent
        : [];
      return rows.slice(0, 20);
    },

    terminatedEntryKey(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      var agentId = String(row.agent_id || '').trim();
      var contractId = String(row.contract_id || '').trim();
      return agentId + '::' + contractId;
    },

    formatTerminatedReason(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      var raw = String(
        row.termination_reason
          || row.reason
          || row.archive_reason
          || row.inactive_reason
          || ''
      ).trim();
      if (!raw) return 'terminated';
      var token = raw.toLowerCase().replace(/[\s-]+/g, '_');
      if (token === 'parent_archived' || token === 'archived_by_parent_agent') {
        return 'Archived by parent agent';
      }
      if (token === 'user_archived' || token === 'user_archive' || token === 'user_archive_all') {
        return 'Archived by user';
      }
      if (token === 'archived') {
        return 'Archived';
      }
      if (token === 'contract_expired') {
        return 'Expired (contract)';
      }
      if (token === 'idle_timeout') {
        return 'Expired (idle timeout)';
      }
      if (token === 'stopped') {
        return 'Stopped by user';
      }
      if (token === 'contract_violation') {
        return 'Contract violation';
      }
      return String(raw)
        .replace(/[_-]+/g, ' ')
        .replace(/\b\w/g, function(ch) { return ch.toUpperCase(); });
    },

    setDeleteTerminatedConfirm(entry) {
      this.confirmDeleteTerminatedKey = this.terminatedEntryKey(entry);
    },

    clearDeleteTerminatedConfirm(entry) {
      var key = this.terminatedEntryKey(entry);
      if (this.confirmDeleteTerminatedKey === key) {
        this.confirmDeleteTerminatedKey = '';
      }
    },

    get idleAgentAlertText() {
      var idle = Number(this.agentLifecycle && this.agentLifecycle.idle_agents || 0);
      var threshold = Number(this.agentLifecycle && this.agentLifecycle.idle_threshold || 0);
      if (!threshold) return '';
      if (idle <= threshold) return '';
      return idle + ' idle agents above threshold ' + threshold;
    },

    // -- Templates computed --
    get categories() {
      var cats = { 'All': true };
      this.builtinTemplates.forEach(function(t) { cats[t.category] = true; });
      this.tplTemplates.forEach(function(t) { if (t.category) cats[t.category] = true; });
      return Object.keys(cats);
    },

    get filteredBuiltins() {
      var self = this;
      return this.builtinTemplates.filter(function(t) {
        if (self.selectedCategory !== 'All' && t.category !== self.selectedCategory) return false;
        if (self.searchQuery) {
          var q = self.searchQuery.toLowerCase();
          if (t.name.toLowerCase().indexOf(q) === -1 &&
              t.description.toLowerCase().indexOf(q) === -1) return false;
        }
        return true;
      });
    },

    get filteredCustom() {
      var self = this;
      return this.tplTemplates.filter(function(t) {
        if (self.searchQuery) {
          var q = self.searchQuery.toLowerCase();
          if ((t.name || '').toLowerCase().indexOf(q) === -1 &&
              (t.description || '').toLowerCase().indexOf(q) === -1) return false;
        }
        return true;
      });
    },

    isProviderConfigured(providerName) {
      if (!providerName) return false;
      var p = this.tplProviders.find(function(pr) { return pr.id === providerName; });
      return p ? p.auth_status === 'configured' : false;
    },

    contractForAgent(agent) {
      if (!agent || typeof agent !== 'object') return null;
      if (agent.contract && typeof agent.contract === 'object') return agent.contract;
      var id = String(agent.id || '');
      if (!id) return null;
      var rows = this.activeLifecycleAgents;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        if (String(row.id || '') !== id) continue;
        if (row.contract && typeof row.contract === 'object') return row.contract;
      }
      return null;
    },
    formatDurationMs(ms) {
      var raw = Number(ms || 0);
      if (!Number.isFinite(raw) || raw <= 0) return '0m';
      var totalMin = Math.max(1, Math.ceil(raw / 60000));
      var day = Math.floor(totalMin / 1440);
      var hour = Math.floor((totalMin % 1440) / 60);
      var min = totalMin % 60;
      var parts = [];
      if (day > 0) parts.push(day + 'd');
      if (hour > 0) parts.push(hour + 'h');
      parts.push(min + 'm');
      return parts.join(' ');
    },

    formatIsoTimestamp(value) {
      var ts = String(value || '').trim();
      if (!ts) return '';
      var ms = Date.parse(ts);
      if (!Number.isFinite(ms)) return '';
      return new Date(ms).toLocaleString();
    },
    formatAgentContractLine(agent) {
      var contract = this.contractForAgent(agent);
      if (!contract) return '';
      var status = String(contract.status || 'active').replace(/_/g, ' ');
      var remaining = contract.remaining_ms;
      if (remaining == null || remaining === '') {
        return 'contract ' + status;
      }
      return 'contract ' + status + ' · ' + this.formatDurationMs(remaining) + ' left';
    },

  };
}
