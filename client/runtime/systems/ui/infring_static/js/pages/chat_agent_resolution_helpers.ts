'use strict';

function infringChatAgentResolutionMethods() {
  return {
    pickDefaultAgent(agents) {
      if (!Array.isArray(agents) || !agents.length) return null;
      // Prefer the master/default agent when present; otherwise first running agent.
      var i;
      for (i = 0; i < agents.length; i++) {
        var a = agents[i] || {};
        if (this.isSystemThreadAgent(a)) continue;
        var text = ((a.id || '') + ' ' + (a.name || '') + ' ' + (a.role || '')).toLowerCase();
        if (text.indexOf('master') >= 0 || text.indexOf('default') >= 0 || text.indexOf('primary') >= 0) {
          return a;
        }
      }
      for (i = 0; i < agents.length; i++) {
        var b = agents[i] || {};
        if (this.isSystemThreadAgent(b)) continue;
        if (String(b.state || '').toLowerCase() === 'running') return b;
      }
      for (i = 0; i < agents.length; i++) {
        if (!this.isSystemThreadAgent(agents[i])) return agents[i];
      }
      return null;
    },

    isSystemThreadId(agentId) {
      var target = String(agentId || '').trim().toLowerCase();
      var systemId = String(this.systemThreadId || 'system').trim().toLowerCase();
      if (!target) return false;
      return target === systemId;
    },

    isSystemThreadAgent(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread === true) return true;
      return this.isSystemThreadId(agent.id);
    },

    isSystemThreadActive() {
      return this.isSystemThreadAgent(this.currentAgent);
    },

    isReservedSystemEmoji(rawEmoji) {
      var normalized = String(rawEmoji || '').replace(/\uFE0F/g, '').trim();
      return normalized === '⚙';
    },

    sanitizeAgentEmojiForDisplay(agentRef, rawEmoji) {
      var emoji = String(rawEmoji || '').trim();
      var isSystem = this.isSystemThreadAgent(agentRef);
      if (isSystem) {
        return String(this.systemThreadEmoji || '\u2699\ufe0f').trim() || '\u2699\ufe0f';
      }
      if (this.isReservedSystemEmoji(emoji)) return '';
      return emoji;
    },

    displayAgentEmoji(agentRef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : this.currentAgent;
      if (!agent) return '';
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      return this.sanitizeAgentEmojiForDisplay(agent, emoji);
    },

    isArchivedAgentRecord(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var isArchivedLikeAgent = bridge && typeof bridge.method === 'function'
        ? bridge.method('isArchivedLikeAgent')
        : null;
      if (typeof isArchivedLikeAgent === 'function' && isArchivedLikeAgent(agent)) return true;
      if (agent.archived === true) return true;
      var state = String(agent.state || '').trim().toLowerCase();
      if (state.indexOf('archived') >= 0 || state.indexOf('inactive') >= 0 || state.indexOf('terminated') >= 0) return true;
      var contract = agent.contract && typeof agent.contract === 'object' ? agent.contract : null;
      var contractStatus = String(contract && contract.status ? contract.status : '').trim().toLowerCase();
      return contractStatus.indexOf('archived') >= 0 || contractStatus.indexOf('inactive') >= 0 || contractStatus.indexOf('terminated') >= 0;
    },

    isCurrentAgentArchived() {
      return this.isArchivedAgentRecord(this.currentAgent);
    },

    makeSystemThreadAgent() {
      var id = String(this.systemThreadId || 'system').trim() || 'system';
      var name = String(this.systemThreadName || 'System').trim() || 'System';
      var emoji = String(this.systemThreadEmoji || '\u2699\ufe0f').trim() || '\u2699\ufe0f';
      return {
        id: id,
        name: name,
        state: 'running',
        role: 'system',
        is_system_thread: true,
        model_provider: 'system',
        model_name: 'terminal',
        runtime_model: 'terminal',
        identity: { emoji: emoji },
        created_at: new Date(0).toISOString(),
        updated_at: new Date().toISOString(),
        auto_terminate_allowed: false,
      };
    },

    resolveAgent(agentOrId) {
      if (!agentOrId) return null;
      var id = typeof agentOrId === 'string' ? agentOrId : agentOrId.id;
      if (!id) return null;
      if (this.isSystemThreadId(id)) return this.makeSystemThreadAgent();
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
      var list = (store && store.agents) || [];
      for (var i = 0; i < list.length; i++) {
        if (list[i] && String(list[i].id) === String(id)) return list[i];
      }
      if (store && store.pendingAgent && String(store.pendingFreshAgentId || '') === String(id)) {
        return store.pendingAgent;
      }
      if (store && store.pendingAgent && String((store.pendingAgent && store.pendingAgent.id) || '') === String(id) && this.isArchivedAgentRecord(store.pendingAgent)) {
        return store.pendingAgent;
      }
      if (typeof agentOrId === 'object' && agentOrId.id && this.isArchivedAgentRecord(agentOrId)) {
        return agentOrId;
      }
      // Only trust stale object references while the store has no live agent list yet.
      if (!list.length && typeof agentOrId === 'object' && agentOrId.id) return agentOrId;
      return null;
    },

    ensureValidCurrentAgent: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
      if (this.currentAgent && this.isSystemThreadAgent(this.currentAgent)) {
        this.currentAgent = this.makeSystemThreadAgent();
        return this.currentAgent;
      }
      var rows = store && Array.isArray(store.agents) ? store.agents : [];
      var currentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var currentLive = currentId ? this.resolveAgent(currentId) : null;
      if (!currentLive && this.currentAgent && this.isArchivedAgentRecord(this.currentAgent)) {
        return this.currentAgent;
      }
      if (currentLive) {
        if (!this.currentAgent || String(this.currentAgent.id || '') !== String(currentLive.id || '')) {
          this.currentAgent = currentLive;
        } else {
          this.syncCurrentAgentFromStore(currentLive);
        }
        return this.currentAgent;
      }
      var preferred = null;
      if (store && store.activeAgentId) preferred = this.resolveAgent(store.activeAgentId);
      if (!preferred && rows.length) preferred = this.pickDefaultAgent(rows);
      if (preferred) {
        this.selectAgent(preferred);
        return this.resolveAgent(preferred.id || preferred) || preferred;
      }
      if (opts.clear_when_missing) this.currentAgent = null;
      return null;
    },

    refreshAgentRosterAuthoritative: async function() {
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
      var rows = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');
      var list = (Array.isArray(rows) ? rows : []).filter((row) => {
        if (!row || !row.id) return false;
        return !(this.isArchivedAgentRecord && this.isArchivedAgentRecord(row));
      });
      if (store) {
        if (bridge && typeof bridge.assign === 'function') {
          bridge.assign({
            agents: list,
            agentsHydrated: true,
            agentsLoading: false,
            agentsLastError: '',
            agentCount: list.length,
            _lastAgentsRefreshAt: Date.now()
          });
        } else {
          Object.assign(store, {
            agents: list,
            agentsHydrated: true,
            agentsLoading: false,
            agentsLastError: '',
            agentCount: list.length,
            _lastAgentsRefreshAt: Date.now()
          });
        }
        if (store.activeAgentId) {
          var stillActive = list.some(function(row) {
            return !!(row && String(row.id || '') === String(store.activeAgentId || ''));
          });
          if (!stillActive && store.pendingAgent && String((store.pendingAgent && store.pendingAgent.id) || '') === String(store.activeAgentId || '') && this.isArchivedAgentRecord(store.pendingAgent)) {
            stillActive = true;
          }
          if (!stillActive && String(store.activeAgentId || '').trim().toLowerCase() === String(this.systemThreadId || 'system').trim().toLowerCase()) {
            stillActive = true;
          }
          if (!stillActive) {
            var setActiveAgentId = bridge && typeof bridge.method === 'function'
              ? bridge.method('setActiveAgentId')
              : null;
            if (typeof setActiveAgentId === 'function') setActiveAgentId(null);
            else if (bridge && typeof bridge.set === 'function') bridge.set('activeAgentId', null);
            else store.activeAgentId = null;
          }
        }
      }
      return list;
    },

    rebindCurrentAgentAuthoritative: async function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var preferredId = String(opts.preferred_id || '').trim();
      var clearWhenMissing = opts.clear_when_missing !== false;
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
      var rows = [];
      try {
        rows = await this.refreshAgentRosterAuthoritative();
      } catch (_) {
        rows = store && Array.isArray(store.agents) ? store.agents : [];
      }

      var rebound = null;
      if (preferredId) {
        rebound = this.resolveAgent(preferredId);
        if (!rebound && Array.isArray(rows)) {
          var lowerPreferred = preferredId.toLowerCase();
          for (var i = 0; i < rows.length; i++) {
            var row = rows[i];
            if (!row || !row.id) continue;
            if (String(row.id).toLowerCase() === lowerPreferred) {
              rebound = row;
              break;
            }
          }
        }
      }
      if (!rebound && store && store.activeAgentId) rebound = this.resolveAgent(store.activeAgentId);
      if (!rebound && Array.isArray(rows) && rows.length) rebound = this.pickDefaultAgent(rows);

      if (rebound && rebound.id) {
        var resolved = this.resolveAgent(rebound.id) || rebound;
        if (this.isSystemThreadAgent(resolved)) {
          InfringAPI.wsDisconnect();
          this._wsAgent = null;
          this.currentAgent = this.makeSystemThreadAgent();
          this.setStoreActiveAgentId(this.currentAgent.id || null);
          return this.currentAgent;
        }
        this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
        this.setStoreActiveAgentId(this.currentAgent.id || null);
        if (this.currentAgent && this.currentAgent.id) {
          var reboundId = String(this.currentAgent.id);
          if (String(this._wsAgent || '') !== reboundId || !InfringAPI.isWsConnected()) {
            this._wsAgent = null;
            this.connectWs(reboundId);
          }
        }
        return this.currentAgent;
      }

      if (clearWhenMissing) {
        this.currentAgent = null;
        this.setStoreActiveAgentId(null);
      }
      return null;
    },

    applyAgentGitTreeState(targetAgent, sourceState) {
      var target = targetAgent && typeof targetAgent === 'object' ? targetAgent : null;
      var source = sourceState && typeof sourceState === 'object' ? sourceState : null;
      if (!target || !source) return target;
      if (Object.prototype.hasOwnProperty.call(source, 'git_branch')) {
        var branch = source.git_branch ? String(source.git_branch).trim() : '';
        if (branch) target.git_branch = branch;
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_kind')) {
        target.git_tree_kind = source.git_tree_kind ? String(source.git_tree_kind).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'workspace_dir')) {
        var workspace = source.workspace_dir ? String(source.workspace_dir).trim() : '';
        if (workspace) {
          target.workspace_dir = workspace;
        }
      }
      if (Object.prototype.hasOwnProperty.call(source, 'workspace_rel')) {
        target.workspace_rel = source.workspace_rel ? String(source.workspace_rel).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_ready')) {
        target.git_tree_ready = !!source.git_tree_ready;
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_error')) {
        target.git_tree_error = source.git_tree_error ? String(source.git_tree_error).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'is_master_agent')) {
        target.is_master_agent = !!source.is_master_agent;
      }
      return target;
    },

    syncCurrentAgentFromStore: function(sourceAgent) {
      var source = sourceAgent && typeof sourceAgent === 'object' ? sourceAgent : null;
      if (!source || !this.currentAgent || !this.currentAgent.id) return false;
      if (String(this.currentAgent.id) !== String(source.id)) return false;
      this.applyAgentGitTreeState(this.currentAgent, source);
      var keys = Object.keys(source);
      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];
        if (key === 'id') continue;
        this.currentAgent[key] = source[key];
      }
      return true;
    },

    setStoreActiveAgentId: function(agentId) {
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
      if (!store) return;
      var setActiveAgentId = bridge && typeof bridge.method === 'function'
        ? bridge.method('setActiveAgentId')
        : null;
      if (typeof setActiveAgentId === 'function') {
        setActiveAgentId(agentId || null);
        return;
      }
      if (bridge && typeof bridge.set === 'function') bridge.set('activeAgentId', agentId || null);
      else store.activeAgentId = agentId || null;
      try {
        if (store.activeAgentId) localStorage.setItem('infring-last-active-agent-id', String(store.activeAgentId));
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch {}
    },

    activateSystemThread: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var priorAgentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (priorAgentId && !this.isSystemThreadId(priorAgentId) && typeof this.captureConversationDraft === 'function') {
        this.captureConversationDraft(priorAgentId);
      }
      this.currentAgent = this.makeSystemThreadAgent();
      this.setStoreActiveAgentId(this.currentAgent.id || null);
      this._clearTypingTimeout();
      this._clearPendingWsRequest(this.currentAgent.id || '');
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.messageQueue = Array.isArray(this.messageQueue)
        ? this.messageQueue.filter(function(row) { return !row || !row.terminal; })
        : [];
      InfringAPI.wsDisconnect();
      this._wsAgent = null;
      this.sessions = [];
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      this.terminalMode = true;
      var restored = this.restoreAgentConversation(this.currentAgent.id);
      if (!restored && opts.preserve_if_empty !== true) {
        this.messages = [];
      }
      if (typeof this.restoreConversationDraft === 'function') {
        this.restoreConversationDraft(this.currentAgent.id, 'terminal');
      }
      this.recomputeContextEstimate();
      this.refreshContextPressure();
      this.clearPromptSuggestions();
      this.$nextTick(() => {
        var input = document.getElementById('msg-input');
        if (input) input.focus();
        this.scrollToBottomImmediate();
        this.stabilizeBottomScroll();
        this.pinToLatestOnOpen(null, { maxFrames: 20 });
        this.scheduleMessageRenderWindowUpdate();
      });
    },
  };
}

function chatActiveGitBranchLabel(vm) {
  var agentBranch = vm.currentAgent && vm.currentAgent.git_branch
    ? String(vm.currentAgent.git_branch).trim()
    : '';
  if (agentBranch) return agentBranch;
  try {
    var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
      ? InfringSharedShellServices.appStore
      : null;
    var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
    var branch = store && store.gitBranch ? String(store.gitBranch).trim() : '';
    return branch || '';
  } catch(_) {
    return '';
  }
}

function chatActiveGitBranchMenuLabel(vm) {
  var label = String(chatActiveGitBranchLabel(vm) || '').trim();
  return label || 'main';
}
