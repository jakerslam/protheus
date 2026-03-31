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
            self.messages.push({
              id: ++msgId,
              role: 'system',
              text: '**Telemetry Alerts**\n' + rendered + (nextRendered ? ('\n\n**Suggested Next Actions**\n' + nextRendered) : ''),
              meta: '',
              tools: [],
              system_origin: 'telemetry:alerts',
              ts: Date.now()
            });
            self.scrollToBottom();
            self.scheduleConversationPersist();
          }
        }
        self._lastTelemetryAlertDigest = digest;
        return payload;
      }).catch(function() {
        self.telemetryNextActions = [];
        return { ok: false, alerts: [] };
      });
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
