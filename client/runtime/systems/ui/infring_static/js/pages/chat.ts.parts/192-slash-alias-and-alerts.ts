    pushSystemMessage: function(entry) {
      var payload = entry && typeof entry === 'object' ? entry : { text: entry };
      var rawText = String(payload && payload.text ? payload.text : '');
      var text = this.normalizeSystemMessageText
        ? this.normalizeSystemMessageText(rawText)
        : rawText.trim();
      if (!text) return null;
      var canonicalText = text.replace(/\s+/g, ' ').trim().toLowerCase();
      if (/^error:\s*/i.test(canonicalText) && canonicalText.indexOf('operation was aborted') >= 0) return null;

      var origin = String(payload.system_origin || payload.systemOrigin || '').trim();
      var tsRaw = Number(payload.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var dedupeWindowMs = Number(payload.dedupe_window_ms || payload.dedupeWindowMs || 8000);
      if (!Number.isFinite(dedupeWindowMs) || dedupeWindowMs < 0) dedupeWindowMs = 8000;
      if (dedupeWindowMs > 60000) dedupeWindowMs = 60000;
      var canDedupe = payload.dedupe !== false;
      var systemThreadId = String(this.systemThreadId || 'system').trim() || 'system';
      var activeId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      var targetId = activeId || systemThreadId;
      var isGlobalNotice = !!(
        this.isSystemNotificationGlobalToWorkspace &&
        this.isSystemNotificationGlobalToWorkspace(origin, text)
      );
      var routeToSystem =
        payload.route_to_system === true ||
        (payload.route_to_system !== false && isGlobalNotice);
      if (routeToSystem) targetId = systemThreadId;
      var activeThread = !!activeId && activeId === targetId;
      if (!this._systemMessageDedupeIndex || typeof this._systemMessageDedupeIndex !== 'object') this._systemMessageDedupeIndex = {};

      var targetRows = null;
      var targetCache = null;
      if (activeThread) {
        if (!Array.isArray(this.messages)) this.messages = [];
        targetRows = this.messages;
      } else {
        if (!this.conversationCache || typeof this.conversationCache !== 'object') this.conversationCache = {};
        targetCache = this.conversationCache[targetId];
        if (!targetCache || typeof targetCache !== 'object' || !Array.isArray(targetCache.messages)) {
          targetCache = { saved_at: Date.now(), token_count: 0, messages: [] };
          this.conversationCache[targetId] = targetCache;
        }
        targetRows = targetCache.messages;
      }

      if (!Array.isArray(targetRows)) return null;
      var dedupeKey = targetId + '|' + (origin || '_') + '|' + canonicalText;
      if (canDedupe) {
        for (var idx = targetRows.length - 1, scanned = 0; idx >= 0 && scanned < 24; idx -= 1) {
          var row = targetRows[idx];
          if (!row || row.thinking || row.streaming) continue;
          if (String(row.role || '').toLowerCase() !== 'system' || row.is_notice) continue;
          scanned += 1;
          var rowText = String(row.text || '').replace(/\s+/g, ' ').trim().toLowerCase();
          if (rowText !== canonicalText) continue;
          var rowTs = Number(row.ts || 0);
          if (Number.isFinite(rowTs) && Math.abs(ts - rowTs) > dedupeWindowMs) continue;
          var rowOrigin = String(row.system_origin || '').trim();
          if (rowOrigin && origin && rowOrigin !== origin && !/^error:/i.test(canonicalText)) continue;
          var repeatCount = Number(row._repeat_count || 1);
          if (!Number.isFinite(repeatCount) || repeatCount < 1) repeatCount = 1;
          repeatCount += 1;
          row._repeat_count = repeatCount;
          var priorMeta = String(row.meta || '').trim().replace(/\s*\|\s*repeated x\d+\s*$/i, '').trim();
          row.meta = (priorMeta ? (priorMeta + ' | ') : '') + 'repeated x' + repeatCount;
          row.ts = ts;
          this._systemMessageDedupeIndex[dedupeKey] = { id: row.id, ts: ts };
          if (activeThread) this.scheduleConversationPersist();
          else this.persistConversationCache();
          return row;
        }
      }

      var message = {
        id: ++msgId,
        role: 'system',
        text: text,
        meta: String(payload.meta || ''),
        tools: Array.isArray(payload.tools) ? payload.tools : [],
        system_origin: origin,
        ts: ts
      };
      targetRows.push(message);
      if (canDedupe && canonicalText) this._systemMessageDedupeIndex[dedupeKey] = { id: message.id, ts: ts };

      var store = Alpine.store('app');
      if (store && typeof store.saveAgentChatPreview === 'function') {
        store.saveAgentChatPreview(targetId, targetRows);
      }
      if (activeThread) {
        if (payload.auto_scroll !== false) this.scrollToBottom();
        this.scheduleConversationPersist();
      } else {
        if (targetCache) {
          targetCache.saved_at = Date.now();
          targetCache.token_count = 0;
        }
        this.persistConversationCache();
      }
      return message;
    },

    activateSystemThread: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
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

    defaultSlashAliases: function() {
      return {
        '/status': '/status',
        '/opt': '/continuity',
        '/q': '/queue',
        '/ctx': '/context',
        '/mods': '/model',
        '/mem': '/compact'
      };
    },

    loadSlashAliases: function() {
      var defaults = this.defaultSlashAliases();
      var persisted = {};
      try {
        var raw = localStorage.getItem(this.slashAliasStorageKey || '');
        if (raw) {
          var parsed = JSON.parse(raw);
          if (parsed && typeof parsed === 'object') persisted = parsed;
        }
      } catch(_) {}
      var merged = {};
      Object.keys(defaults).forEach(function(key) {
        var target = String(defaults[key] || '').trim().toLowerCase();
        var alias = String(key || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      Object.keys(persisted).forEach(function(key) {
        var alias = String(key || '').trim().toLowerCase();
        var target = String(persisted[key] || '').trim().toLowerCase();
        if (!alias.startsWith('/') || !target.startsWith('/')) return;
        merged[alias] = target;
      });
      this.slashAliasMap = merged;
      return merged;
    },

    saveSlashAliases: function() {
      try {
        localStorage.setItem(
          this.slashAliasStorageKey || '',
          JSON.stringify(this.slashAliasMap || {})
        );
      } catch(_) {}
    },

    resolveSlashAlias: function(inputCmd, cmdArgs) {
      var cmd = String(inputCmd || '').trim().toLowerCase();
      var args = String(cmdArgs || '').trim();
      var aliases = this.slashAliasMap || {};
      var target = String(aliases[cmd] || '').trim();
      if (!target) return { cmd: cmd, args: args, expanded: cmd };
      var expanded = target;
      var expandedArgs = args;
      var targetParts = expanded.split(/\s+/);
      if (targetParts.length > 1) {
        expanded = targetParts[0];
        var trailing = targetParts.slice(1).join(' ').trim();
        expandedArgs = trailing ? (trailing + (args ? (' ' + args) : '')) : args;
      }
      return { cmd: expanded, args: expandedArgs.trim(), expanded: target + (args ? (' ' + args) : '') };
    },

    formatSlashAliasRows: function() {
      var aliases = this.slashAliasMap || {};
      var rows = Object.keys(aliases)
        .sort()
        .map(function(alias) {
          return '- `' + alias + '` → `' + String(aliases[alias] || '') + '`';
        });
      return rows.join('\n');
    },

    fetchProactiveTelemetryAlerts: function(notify) {
      var self = this;
      return InfringAPI.get('/api/telemetry/alerts').then(function(payload) {
        var rows = Array.isArray(payload && payload.alerts) ? payload.alerts : [];
        var nextActions = Array.isArray(payload && payload.next_actions) ? payload.next_actions : [];
        var digest = rows.map(function(row) {
          return String((row && row.id) || '') + ':' + String((row && row.message) || '');
        }).join('|');
        self._telemetrySnapshot = payload && typeof payload === 'object' ? payload : null;
        self._continuitySnapshot = payload && payload.continuity ? payload.continuity : null;
        self.telemetryNextActions = nextActions.slice(0, 6);
        if (notify && digest && digest !== String(self._lastTelemetryAlertDigest || '')) {
          var rendered = rows.map(function(row) {
            var severity = String((row && row.severity) || 'info').toUpperCase();
            var message = String((row && row.message) || '').trim();
            var command = String((row && row.recommended_command) || '').trim();
            return '- [' + severity + '] ' + message + (command ? ('\n  ↳ `' + command + '`') : '');
          }).join('\n');
          var nextRendered = nextActions.slice(0, 3).map(function(row) {
            var cmd = String((row && row.command) || '').trim();
            var reason = String((row && row.reason) || '').trim();
            return '- `' + cmd + '`' + (reason ? ('\n  ↳ ' + reason) : '');
          }).join('\n');
          if (rendered) {
            self.pushSystemMessage({
              text: '**Telemetry Alerts**\n' + rendered + (nextRendered ? ('\n\n**Suggested Next Actions**\n' + nextRendered) : ''),
              system_origin: 'telemetry:alerts',
              ts: Date.now(),
              auto_scroll: false
            });
          }
        }
        self._lastTelemetryAlertDigest = digest;
        return payload;
      }).catch(function() {
        self._telemetrySnapshot = null;
        self.telemetryNextActions = [];
        return { ok: false, alerts: [] };
      });
    },

    staleMemoryWarningText: function() {
      return '';
    },

    thinkingTraceRows: function(msg) {
      var rows = [];
      if (!msg || !msg.thinking) return rows;
      var tools = Array.isArray(msg.tools) ? msg.tools : [];
      for (var i = 0; i < tools.length; i++) {
        var tool = tools[i];
        if (!tool || this.isThoughtTool(tool)) continue;
        var state = tool.running ? 'running' : (this.isBlockedTool(tool) ? 'blocked' : (tool.is_error ? 'error' : 'done'));
        rows.push({
          id: String(tool.id || ('trace-tool-' + i)),
          label: this.toolDisplayName(tool),
          state: state,
          state_label: state === 'done' ? 'complete' : state
        });
      }
      if (!rows.length) {
        var status = String(
          typeof this.thinkingStatusText === 'function'
            ? this.thinkingStatusText(msg)
            : (msg.thinking_status || '')
        ).trim();
        if (status) {
          rows.push({
            id: 'trace-status',
            label: status,
            state: 'running',
            state_label: 'active'
          });
        }
      }
      return rows.slice(-4);
    },

    emitCommandFailureNotice: function(command, error, fallbackCommands) {
      var cmd = String(command || '').trim() || '/status';
      var message = String(error && error.message ? error.message : error || 'command_failed').trim();
      if (message.length > 220) message = message.slice(0, 217) + '...';
      var fallbacks = Array.isArray(fallbackCommands) ? fallbackCommands : [];
      var fallbackText = fallbacks
        .map(function(row) { return '`' + String(row || '').trim() + '`'; })
        .filter(Boolean)
        .join(' · ');
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text:
          'Command `' + cmd + '` failed: ' + message +
          (fallbackText ? ('\nTry recovery: ' + fallbackText) : ''),
        meta: '',
        tools: [],
        system_origin: 'slash:error',
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    get filteredSlashCommands() {
      var base = Array.isArray(this.slashCommands) ? this.slashCommands.slice() : [];
      var aliases = this.slashAliasMap || {};
      Object.keys(aliases).forEach(function(alias) {
        if (!base.some(function(c) { return c && c.cmd === alias; })) {
          base.push({
            cmd: alias,
            desc: 'Alias → ' + String(aliases[alias] || ''),
            source: 'alias'
          });
        }
      });
      if (!this.slashFilter) return base;
      var f = this.slashFilter;
      return base.filter(function(c) {
        return c.cmd.toLowerCase().indexOf(f) !== -1 || c.desc.toLowerCase().indexOf(f) !== -1;
      });
    },
