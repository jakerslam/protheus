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
      return !!(row && (row.download_available === true || String(row.local_download_path || '').trim()));
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
        var models = (data && data.models) || [];
        self._modelCache = models.filter(function(m) { return m.available; });
        self._modelCacheTime = Date.now();
        self.modelPickerList = self._modelCache;
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
        var text = ((a.id || '') + ' ' + (a.name || '') + ' ' + (a.role || '')).toLowerCase();
        if (text.indexOf('master') >= 0 || text.indexOf('default') >= 0 || text.indexOf('primary') >= 0) {
          return a;
        }
      }
      for (i = 0; i < agents.length; i++) {
        var b = agents[i] || {};
        if (String(b.state || '').toLowerCase() === 'running') return b;
      }
      return agents[0];
    },

    resolveAgent(agentOrId) {
      if (!agentOrId) return null;
      var id = typeof agentOrId === 'string' ? agentOrId : agentOrId.id;
      if (!id) return null;
      var list = (Alpine.store('app') && Alpine.store('app').agents) || [];
      for (var i = 0; i < list.length; i++) {
        if (list[i] && String(list[i].id) === String(id)) return list[i];
      }
      // Only trust stale object references while the store has no live agent list yet.
      if (!list.length && typeof agentOrId === 'object' && agentOrId.id) return agentOrId;
      return null;
    },

    ensureValidCurrentAgent: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var store = Alpine.store('app');
      var rows = store && Array.isArray(store.agents) ? store.agents : [];
      var currentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var currentLive = currentId ? this.resolveAgent(currentId) : null;
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
      var list = Array.isArray(rows) ? rows : [];
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

    cacheAgentConversation(agentId) {
      if (!agentId) return;
      if (!this.conversationCache) this.conversationCache = {};
      try {
        var cachedMessages = this.sanitizeConversationForCache(this.messages || []);
        this.conversationCache[String(agentId)] = {
          saved_at: Date.now(),
          token_count: this.tokenCount || 0,
          messages: cachedMessages,
        };
        var appStore = Alpine.store('app');
        if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
          appStore.saveAgentChatPreview(agentId, this.conversationCache[String(agentId)].messages);
        }
        this.persistConversationCache();
      } catch {}
    },

    cacheCurrentConversation() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      this.cacheAgentConversation(this.currentAgent.id);
    },

    scheduleConversationPersist() {
      var self = this;
      if (this._persistTimer) clearTimeout(this._persistTimer);
      this._persistTimer = setTimeout(function() {
        self.cacheCurrentConversation();
      }, 80);
    },

    sanitizeConversationForCache(messages) {
      var source = Array.isArray(messages) ? messages : [];
      var out = [];
      for (var i = 0; i < source.length; i++) {
        var msg = source[i];
        if (!msg || typeof msg !== 'object') continue;
        if (msg.thinking || msg.streaming || (msg.terminal && msg.thinking)) continue;
        var cloned = null;
        try {
          cloned = JSON.parse(JSON.stringify(msg));
        } catch(_) {
          cloned = null;
        }
        if (!cloned || typeof cloned !== 'object') continue;
        var roleRaw = String(cloned.role || cloned.type || '').trim().toLowerCase();
        if (roleRaw.indexOf('assistant') >= 0) roleRaw = 'agent';
        else if (roleRaw.indexOf('user') >= 0) roleRaw = 'user';
        else if (roleRaw.indexOf('system') >= 0) roleRaw = 'system';
        else if (cloned.terminal) roleRaw = 'terminal';
        else roleRaw = roleRaw || 'agent';
        cloned.role = roleRaw;
        var rawText = cloned.text;
        if (rawText == null && cloned.content != null) rawText = cloned.content;
        if (rawText == null && cloned.message != null) rawText = cloned.message;
        if (rawText == null && cloned.assistant != null) rawText = cloned.assistant;
        if (rawText == null && cloned.user != null && roleRaw === 'user') rawText = cloned.user;
        if (rawText == null) rawText = '';
        if (typeof rawText !== 'string') {
          try {
            rawText = JSON.stringify(rawText);
          } catch(_) {
            rawText = String(rawText || '');
          }
        }
        cloned.text = rawText;
        delete cloned.content;
        delete cloned.thinking;
        delete cloned.streaming;
        delete cloned.thoughtStreaming;
        delete cloned._streamRawText;
        delete cloned._cleanText;
        delete cloned._thoughtText;
        delete cloned._toolTextDetected;
        delete cloned._reasoning;
        if (Array.isArray(cloned.tools)) {
          for (var ti = 0; ti < cloned.tools.length; ti++) {
            if (cloned.tools[ti] && typeof cloned.tools[ti] === 'object') {
              cloned.tools[ti].running = false;
            }
          }
        }
        var hasNotice = !!(cloned.is_notice || cloned.notice_label || cloned.notice_type || cloned.notice_action);
        var hasText = typeof cloned.text === 'string' && cloned.text.trim().length > 0;
        var hasTools = Array.isArray(cloned.tools) && cloned.tools.length > 0;
        var hasArtifacts = !!(cloned.file_output || cloned.folder_output);
        var hasProgress = !!(cloned.progress && typeof cloned.progress === 'object');
        var hasTerminal = !!cloned.terminal;
        if (!hasNotice && !hasText && !hasTools && !hasArtifacts && !hasProgress && !hasTerminal) {
          continue;
        }
        out.push(cloned);
      }
      return out;
    },

    restoreAgentConversation(agentId) {
      if (!agentId || !this.conversationCache) return false;
      const cached = this.conversationCache[String(agentId)];
      if (!cached || !Array.isArray(cached.messages)) return false;
      try {
        var rawCachedMessages = cached.messages || [];
        var sanitized = this.sanitizeConversationForCache(cached.messages || []);
        var cacheChanged = false;
        try {
          cacheChanged = JSON.stringify(sanitized) !== JSON.stringify(rawCachedMessages);
        } catch(_) {
          cacheChanged = sanitized.length !== rawCachedMessages.length;
        }
        this.messages = this.mergeModelNoticesForAgent(agentId, sanitized);
        this.tokenCount = Number(cached.token_count || 0);
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest();
        if (cacheChanged) {
          this.conversationCache[String(agentId)].messages = sanitized;
          this.persistConversationCache();
        }
        this.recomputeContextEstimate();
        this.$nextTick(() => this.scrollToBottomImmediate());
        return true;
