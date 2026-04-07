      if (!this.modelDownloadProgress) this.modelDownloadProgress = {};
      var raw = Number(value);
      if (!Number.isFinite(raw)) raw = 0;
      raw = Math.max(0, Math.min(100, Math.round(raw)));
      if (raw <= 0) {
        delete this.modelDownloadProgress[key];
      } else {
        this.modelDownloadProgress[key] = raw;
      }
    },

    clearModelDownloadProgressTimer: function(key) {
      if (!key) return;
      if (!this._modelDownloadProgressTimers) this._modelDownloadProgressTimers = {};
      var timer = this._modelDownloadProgressTimers[key];
      if (timer) {
        clearInterval(timer);
      }
      delete this._modelDownloadProgressTimers[key];
    },

    startModelDownloadProgressTimer: function(key) {
      if (!key) return;
      this.clearModelDownloadProgressTimer(key);
      var self = this;
      var seeded = Number(self.modelDownloadProgress && self.modelDownloadProgress[key] ? self.modelDownloadProgress[key] : 0);
      if (!Number.isFinite(seeded) || seeded <= 0) seeded = 2;
      self.setModelDownloadProgress(key, seeded);
      self._modelDownloadProgressTimers[key] = setInterval(function() {
        var current = Number(self.modelDownloadProgress[key] || 0);
        if (!Number.isFinite(current) || current <= 0) current = 2;
        if (current >= 94) return;
        var bump = current < 30 ? 7 : (current < 60 ? 4 : 2);
        self.setModelDownloadProgress(key, Math.min(94, current + bump));
      }, 520);
    },

    modelPowerIcons: function(model) {
      return 'ϟ'.repeat(this.modelPowerLevel(model));
    },

    modelCostIcons: function(model) {
      return '$'.repeat(this.modelCostLevel(model));
    },

    modelDownloadKey: function(model) {
      var row = model || {};
      var provider = String(row.provider || '').trim().toLowerCase();
      var id = String(row.id || row.display_name || '').trim().toLowerCase();
      return provider + '::' + id;
    },

    isModelDownloadable: function(model) {
      var row = model || {};
      var id = String(row.id || row.model || '').trim();
      var provider = String(row.provider || '').trim().toLowerCase();
      return !!(
        row &&
        (
          row.download_available === true ||
          String(row.local_download_path || '').trim() ||
          (id && provider && provider !== 'auto')
        )
      );
    },

    isModelDownloadBusy: function(model) {
      var key = this.modelDownloadKey(model);
      return !!(key && this.modelDownloadBusy && this.modelDownloadBusy[key] === true);
    },

    downloadModelToLocal: function(model) {
      var self = this;
      var row = model || {};
      if (!self.isModelDownloadable(row)) {
        InfringToast.error('No local download path is available for this model');
        return;
      }
      var key = self.modelDownloadKey(row);
      if (!key) return;
      if (!self.modelDownloadBusy) self.modelDownloadBusy = {};
      if (self.modelDownloadBusy[key]) return;
      self.modelDownloadBusy[key] = true;
      self.setModelDownloadProgress(key, 2);
      self.startModelDownloadProgressTimer(key);
      var modelRef = String(row.id || row.display_name || '').trim();
      var provider = String(row.provider || '').trim();
      InfringAPI.post('/api/models/download', {
        model: modelRef,
        provider: provider
      }).then(function(resp) {
        var method = String((resp && resp.method) || '').trim();
        var localPath = String((resp && resp.download_path) || '').trim();
        self.setModelDownloadProgress(key, 100);
        if (method === 'ollama_pull') {
          InfringToast.success('Model downloaded locally: ' + localPath);
          self.addNoticeEvent({
            notice_label: 'Downloaded ' + (String(row.display_name || row.id || 'model').trim()) + ' locally',
            notice_type: 'info',
            notice_icon: '⬇',
            ts: Date.now(),
          });
        } else {
          InfringToast.success('Local download path prepared: ' + localPath);
          self.addNoticeEvent({
            notice_label: 'Prepared local download path for ' + (String(row.display_name || row.id || 'model').trim()),
            notice_type: 'info',
            notice_icon: '⬇',
            ts: Date.now(),
          });
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return InfringAPI.get('/api/models');
      }).then(function(data) {
        var models = self.sanitizeModelCatalogRows((data && data.models) || []);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
      }).catch(function(e) {
        InfringToast.error('Model download failed: ' + (e && e.message ? e.message : e));
        self.setModelDownloadProgress(key, 0);
      }).finally(function() {
        self.modelDownloadBusy[key] = false;
        self.clearModelDownloadProgressTimer(key);
        if (self.modelDownloadProgress && self.modelDownloadProgress[key] >= 100) {
          setTimeout(function() {
            self.setModelDownloadProgress(key, 0);
          }, 900);
        } else {
          self.setModelDownloadProgress(key, 0);
        }
      });
    },

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
      var store = Alpine.store('app');
      if (store && typeof store.isArchivedLikeAgent === 'function' && store.isArchivedLikeAgent(agent)) return true;
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
      var store = Alpine.store('app');
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
      var store = Alpine.store('app');
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
      var store = Alpine.store('app');
      var rows = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');
      var list = (Array.isArray(rows) ? rows : []).filter((row) => {
        if (!row || !row.id) return false;
        return !(this.isArchivedAgentRecord && this.isArchivedAgentRecord(row));
      });
      if (store) {
        store.agents = list;
        store.agentsHydrated = true;
        store.agentsLoading = false;
        store.agentsLastError = '';
        store.agentCount = list.length;
        store._lastAgentsRefreshAt = Date.now();
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
            if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
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
      var store = Alpine.store('app');
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
      var store = Alpine.store('app');
      if (!store) return;
      if (typeof store.setActiveAgentId === 'function') {
        store.setActiveAgentId(agentId || null);
        return;
      }
      store.activeAgentId = agentId || null;
      try {
        if (store.activeAgentId) localStorage.setItem('infring-last-active-agent-id', String(store.activeAgentId));
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch {}
    },

    resolveConversationInputMode(agentId) {
      var key = String(agentId || '').trim();
      if (!key) return 'chat';
      if (this.isSystemThreadId(key)) return 'terminal';
      var cached = this.conversationCache && this.conversationCache[key];
      return cached && cached.default_terminal === true ? 'terminal' : 'chat';
    },

    currentConversationInputMode(agentId) {
      if (this.isSystemThreadId(agentId)) return 'terminal';
      return this.terminalMode ? 'terminal' : 'chat';
    },

    applyConversationInputMode(agentId, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var hasForced = Object.prototype.hasOwnProperty.call(opts, 'force_terminal');
      var mode = this.resolveConversationInputMode(agentId);
      if (hasForced) mode = opts.force_terminal === true ? 'terminal' : 'chat';
      if (this.isSystemThreadId(agentId)) mode = 'terminal';
      this.terminalMode = mode === 'terminal';
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showModelSwitcher = false;
      this.terminalCursorFocused = false;
      if (!this.terminalMode) this.terminalSelectionStart = 0;
      if (this.terminalMode && !this.terminalCwd) this.terminalCwd = '/workspace';
      return mode;
    },

    sanitizeConversationDraftText(rawText) {
      var text = String(rawText == null ? '' : rawText);
      if (!text) return '';
      if (text.length > 12000) text = text.slice(0, 12000);
      var trimmed = text.trim();
      if (!trimmed) return '';
      if (/^message\s+.+\.\.\.(?:\s+\(\/\s*for commands\))?$/i.test(trimmed)) return '';
      if (/^tell\s+.+\.\.\.$/i.test(trimmed)) return '';
      return text;
    },

    captureConversationDraft(agentId, explicitMode) {
      var key = String(agentId || '').trim();
      if (!key) return;
      if (!this.conversationCache) this.conversationCache = {};
      var mode = String(explicitMode || this.currentConversationInputMode(key) || 'chat').trim().toLowerCase();
      if (mode !== 'terminal') mode = 'chat';
      var prior = this.conversationCache[key] && typeof this.conversationCache[key] === 'object'
        ? this.conversationCache[key]
        : {};
      var next = { ...prior, saved_at: Date.now() };
      var sanitized = this.sanitizeConversationDraftText(this.inputText);
      if (mode === 'terminal') next.draft_terminal = sanitized;
      else next.draft_chat = sanitized;
      this.conversationCache[key] = next;
      this.persistConversationCache();
    },

    restoreConversationDraft(agentId, explicitMode) {
      var key = String(agentId || '').trim();
      if (!key || !this.conversationCache) {
        this.inputText = '';
        return '';
      }
      var cached = this.conversationCache[key];
      if (!cached || typeof cached !== 'object') {
        this.inputText = '';
        return '';
      }
      var mode = String(explicitMode || this.currentConversationInputMode(key) || 'chat').trim().toLowerCase();
      if (mode !== 'terminal') mode = 'chat';
      var raw = mode === 'terminal' ? cached.draft_terminal : cached.draft_chat;
      var nextText = this.sanitizeConversationDraftText(raw);
      this.inputText = nextText;
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (!el) return;
        el.style.height = 'auto';
        el.style.height = Math.min(el.scrollHeight, 150) + 'px';
        if (self.terminalMode) self.updateTerminalCursor({ target: el });
      });
      return nextText;
    },

    cacheAgentConversation(agentId) {
      if (!agentId) return;
      if (!this.conversationCache) this.conversationCache = {};
      try {
        var key = String(agentId);
        var prior = this.conversationCache[key] && typeof this.conversationCache[key] === 'object'
          ? this.conversationCache[key]
          : {};
        var cachedMessages = this.sanitizeConversationForCache(this.messages || []);
        var next = {
          ...prior,
          saved_at: Date.now(),
          token_count: this.tokenCount || 0,
          default_terminal: this.currentConversationInputMode(agentId) === 'terminal',
          messages: cachedMessages,
        };
        var mode = this.currentConversationInputMode(agentId);
        var draft = this.sanitizeConversationDraftText(this.inputText);
        if (mode === 'terminal') next.draft_terminal = draft;
        else next.draft_chat = draft;
        this.conversationCache[key] = next;
        var appStore = Alpine.store('app');
        if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
          appStore.saveAgentChatPreview(agentId, this.conversationCache[key].messages);
        }
        this.persistConversationCache();
      } catch {}
    },

    cacheCurrentConversation() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      this.cacheAgentConversation(this.currentAgent.id);
    },

    scheduleConversationPersist() {
