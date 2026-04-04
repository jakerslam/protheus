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
        this._modelCache = models;
        this._modelCacheTime = Date.now();
        this.modelPickerList = models;
        if (includeGuidance && this.availableModelRowsCount(models) === 0) {
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
