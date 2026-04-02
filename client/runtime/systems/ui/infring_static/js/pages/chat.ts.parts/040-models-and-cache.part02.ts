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
