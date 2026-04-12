      var self = this;
      if (this._persistTimer) clearTimeout(this._persistTimer);
      this._persistTimer = setTimeout(function() {
        self.cacheCurrentConversation();
      }, 80);
    },

    availableModelRowsCount: function(rows) {
      var list = Array.isArray(rows) ? rows : [];
      var count = 0;
      for (var i = 0; i < list.length; i += 1) {
        var row = list[i] || {};
        if (row.available !== false) count += 1;
      }
      return count;
    },

    providerPayloadToModelCatalogRows: function(payload) {
      var providers = payload && Array.isArray(payload.providers) ? payload.providers : [];
      var out = [];
      for (var i = 0; i < providers.length; i += 1) {
        var providerRow = providers[i] && typeof providers[i] === 'object' ? providers[i] : {};
        var provider = String(providerRow.id || '').trim().toLowerCase();
        if (!provider || provider === 'auto') continue;
        var isLocal = providerRow.is_local === true;
        var reachable = providerRow.reachable === true;
        var supportsChat = providerRow.supports_chat !== false;
        var needsKey = providerRow.needs_key === true;
        var authStatus = String(providerRow.auth_status || '').trim().toLowerCase();
        var authConfigured = authStatus === 'configured' || authStatus === 'set' || authStatus === 'ok';
        var profiles = providerRow.model_profiles && typeof providerRow.model_profiles === 'object'
          ? providerRow.model_profiles
          : {};
        var names = Object.keys(profiles);
        for (var j = 0; j < names.length; j += 1) {
          var modelName = String(names[j] || '').trim();
          if (!modelName) continue;
          var modelRef = provider + '/' + modelName;
          if (this.isPlaceholderModelRef(modelRef)) continue;
          var profile = profiles[modelName] && typeof profiles[modelName] === 'object' ? profiles[modelName] : {};
          var deployment = String(profile.deployment_kind || '').trim().toLowerCase();
          var rowLocal = isLocal || deployment === 'local' || deployment === 'ollama';
          var available = supportsChat && (rowLocal ? reachable : (!needsKey || authConfigured || reachable));
          out.push({
            id: modelRef,
            provider: provider,
            model: modelName,
            model_name: modelName,
            runtime_model: modelName,
            display_name: String(profile.display_name || modelName).trim() || modelName,
            available: !!available,
            reachable: !!reachable,
            supports_chat: supportsChat,
            needs_key: !!needsKey,
            auth_status: authStatus || 'unknown',
            is_local: rowLocal,
            deployment_kind: deployment || (rowLocal ? 'local' : 'api'),
            context_window: Number(profile.context_window || profile.context_size || profile.context_tokens || 0) || 0,
            context_window_tokens: Number(profile.context_window || profile.context_size || profile.context_tokens || 0) || 0,
            power_rating: Number(profile.power_rating || 3) || 3,
            cost_rating: Number(profile.cost_rating || (rowLocal ? 1 : 3)) || (rowLocal ? 1 : 3),
            specialty: String(profile.specialty || 'general').trim().toLowerCase() || 'general',
            specialty_tags: Array.isArray(profile.specialty_tags) ? profile.specialty_tags : ['general'],
            param_count_billion: Number(profile.param_count_billion || 0) || 0,
            download_available: profile.download_available === true || rowLocal,
            local_download_path: String(profile.local_download_path || '').trim(),
            max_output_tokens: Number(profile.max_output_tokens || 0) || 0,
          });
        }
      }
      return out;
    },

    mergeModelCatalogRows: function(primaryRows, fallbackRows) {
      var merged = [];
      var seen = {};
      var add = function(row) {
        var id = String(row && row.id ? row.id : '').trim();
        if (!id) return;
        var key = id.toLowerCase();
        if (seen[key]) return;
        seen[key] = true;
        merged.push(row);
      };
      var primary = Array.isArray(primaryRows) ? primaryRows : [];
      var fallback = Array.isArray(fallbackRows) ? fallbackRows : [];
      for (var i = 0; i < primary.length; i += 1) add(primary[i]);
      for (var j = 0; j < fallback.length; j += 1) add(fallback[j]);
      return merged;
    },

    noModelsGuidanceText: function() {
      return [
        "I don't have any usable models yet.",
        '',
        'To enable models now:',
        '1. Install Ollama: https://ollama.com/download',
        '2. Start it: `ollama serve`',
        '3. Pull a model: `ollama pull qwen2.5:3b-instruct`',
        '4. Or add an API key with `/apikey <key>`',
        '',
        'Useful links:',
        '- Ollama library: https://ollama.com/library',
        '- OpenRouter keys: https://openrouter.ai/keys',
        '- OpenAI keys: https://platform.openai.com/api-keys',
        '- Anthropic keys: https://console.anthropic.com/settings/keys'
      ].join('\n');
    },

    injectNoModelsGuidance: function(reason) {
      if (!this.currentAgent || (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent))) {
        return null;
      }
      if (!this._noModelsGuidanceByAgent || typeof this._noModelsGuidanceByAgent !== 'object') {
        this._noModelsGuidanceByAgent = {};
      }
      var agentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (!agentId) return null;
      if (this._noModelsGuidanceByAgent[agentId]) return null;
      var text = this.noModelsGuidanceText();
      var row = {
        id: ++msgId,
        role: 'agent',
        text: text,
        meta: '',
        tools: [],
        ts: Date.now(),
        agent_id: agentId,
        agent_name: String((this.currentAgent && this.currentAgent.name) || 'Agent'),
        system_origin: 'models:no_models_available'
      };
      var pushed = this.pushAgentMessageDeduped(row, { dedupe_window_ms: 120000 }) || row;
      this._noModelsGuidanceByAgent[agentId] = {
        ts: Date.now(),
        reason: String(reason || ''),
        id: pushed && pushed.id ? pushed.id : row.id
      };
      this.scrollToBottom();
      this.scheduleConversationPersist();
      return pushed;
    },

    addNoModelsRecoveryNotice: function(reason, actionKind) {
      if (!this.currentAgent || (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent))) {
        return null;
      }
      if (typeof this.addNoticeEvent !== 'function') return null;
      if (!this._noModelsRecoveryNoticeByAgent || typeof this._noModelsRecoveryNoticeByAgent !== 'object') {
        this._noModelsRecoveryNoticeByAgent = {};
      }
      var agentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (!agentId) return null;
      var now = Date.now();
      var prev = this._noModelsRecoveryNoticeByAgent[agentId];
      if (prev && Number(prev.ts || 0) > 0 && (now - Number(prev.ts || 0)) < 20000) {
        return null;
      }
      var desiredKind = String(actionKind || '').trim().toLowerCase();
      if (!desiredKind) desiredKind = 'model_discover';
      var action = null;
      if (desiredKind === 'open_url') {
        action = {
          kind: 'open_url',
          label: 'Install Ollama',
          url: 'https://ollama.com/download'
        };
      } else {
        action = {
          kind: 'model_discover',
          label: 'Discover models',
          reason: String(reason || 'chat_send_gate').trim()
        };
      }
      this.addNoticeEvent({
        notice_label: desiredKind === 'open_url'
          ? 'No runnable models detected. Install Ollama, then run model discovery.'
          : 'No runnable models detected. Discover models to unlock chat.',
        notice_type: 'warn',
        notice_icon: '\u26a0',
        notice_action: action,
        ts: now
      });
      this._noModelsRecoveryNoticeByAgent[agentId] = {
        ts: now,
        reason: String(reason || ''),
        action_kind: desiredKind
      };
      return true;
    },

    currentAvailableModelCount: function() {
      var rows = [];
      if (Array.isArray(this.modelPickerList) && this.modelPickerList.length) {
        rows = this.modelPickerList;
      } else if (Array.isArray(this._modelCache) && this._modelCache.length) {
        rows = this._modelCache;
      } else {
        rows = [];
      }
      rows = this.sanitizeModelCatalogRows(rows);
      return this.availableModelRowsCount(rows);
    },

    ensureUsableModelsForChatSend: async function(reason) {
      var available = this.currentAvailableModelCount();
      if (available > 0) return available;
      try {
        var models = await this.refreshModelCatalogAndGuidance({ discover: true, guidance: true });
        available = this.availableModelRowsCount(models);
      } catch (_) {
        available = this.currentAvailableModelCount();
      }
      if (available <= 0) {
        this.injectNoModelsGuidance(reason || 'chat_send_gate');
        this.addNoModelsRecoveryNotice(reason || 'chat_send_gate', 'model_discover');
      }
      return available;
    },

    refreshModelCatalogAndGuidance: async function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var discoverFirst = opts.discover !== false;
      var includeGuidance = opts.guidance !== false;
      try {
        if (discoverFirst) {
          await InfringAPI.post('/api/models/discover', { input: '__auto__' }).catch(function() { return null; });
        }
        var data = await InfringAPI.get('/api/models');
        var models = this.sanitizeModelCatalogRows((data && data.models) || []);
        var available = this.availableModelRowsCount(models);
        // Recover from partial catalog responses by rebuilding rows from provider model_profiles.
        if (models.length < 8 || available < 4) {
          var providersPayload = await InfringAPI.get('/api/providers').catch(function() { return null; });
          if (providersPayload) {
            var providerRows = this.sanitizeModelCatalogRows(
              this.providerPayloadToModelCatalogRows(providersPayload)
            );
            if (providerRows.length) {
              models = this.mergeModelCatalogRows(models, providerRows);
              available = this.availableModelRowsCount(models);
            }
          }
        }
        this._modelCache = models;
        this._modelCacheTime = Date.now();
        this.modelPickerList = models;
        if (includeGuidance && available === 0) {
          this.injectNoModelsGuidance('refresh');
        }
        return models;
      } catch (err) {
        if (includeGuidance && (!this.modelPickerList || !this.modelPickerList.length)) {
          this.injectNoModelsGuidance('refresh_error');
        }
        throw err;
      }
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
        if (this.applyConversationInputMode) this.applyConversationInputMode(agentId);
        var rawCachedMessages = cached.messages || [];
        var sanitized = this.sanitizeConversationForCache(cached.messages || []);
        var cacheChanged = false;
        try {
          cacheChanged = JSON.stringify(sanitized) !== JSON.stringify(rawCachedMessages);
        } catch(_) {
          cacheChanged = sanitized.length !== rawCachedMessages.length;
        }
        this.messages = this.mergeModelNoticesForAgent(
          agentId,
          this.normalizeSessionMessages({ messages: sanitized })
        );
        this.tokenCount = Number(cached.token_count || 0);
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest();
        if (cacheChanged) {
          this.conversationCache[String(agentId)].messages = sanitized;
          this.persistConversationCache();
        }
        this.recomputeContextEstimate();
        if (typeof this.restoreConversationDraft === 'function') {
          this.restoreConversationDraft(agentId);
        }
        this.$nextTick(() => this.scrollToBottomImmediate());
        return true;
