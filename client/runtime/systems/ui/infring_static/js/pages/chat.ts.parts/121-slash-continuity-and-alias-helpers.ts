    runSlashAlerts: async function() {
      try {
        var alertsPayload = await this.fetchProactiveTelemetryAlerts(false);
        var alertsRows = Array.isArray(alertsPayload && alertsPayload.alerts) ? alertsPayload.alerts : [];
        if (!alertsRows.length) {
          this.messages.push({
            id: ++msgId,
            role: 'system',
            text: 'No proactive telemetry alerts right now.',
            meta: '',
            tools: [],
            system_origin: 'slash:alerts',
            ts: Date.now()
          });
        } else {
          var alertText = alertsRows.map(function(row) {
            var sev = String((row && row.severity) || 'info').toUpperCase();
            var msg = String((row && row.message) || '').trim();
            var cmd = String((row && row.recommended_command) || '').trim();
            return '- [' + sev + '] ' + msg + (cmd ? ('\n  ↳ `' + cmd + '`') : '');
          }).join('\n');
          this.messages.push({
            id: ++msgId,
            role: 'system',
            text: '**Telemetry Alerts**\n' + alertText,
            meta: '',
            tools: [],
            system_origin: 'slash:alerts',
            ts: Date.now()
          });
        }
        this.scrollToBottom();
      } catch (_) {}
    },

    runSlashContinuity: async function() {
      try {
        var continuity = await InfringAPI.get('/api/continuity/pending');
        var taskPending = Number((((continuity || {}).tasks || {}).pending) || 0);
        var staleSessions = Number(((((continuity || {}).sessions) || {}).stale_48h_count) || 0);
        var channelAttention = Number(((((continuity || {}).channels) || {}).attention_needed_count) || 0);
        var continuityRows = [];
        continuityRows.push('**Cross-Channel Continuity**');
        continuityRows.push('- Pending tasks: ' + taskPending);
        continuityRows.push('- Stale sessions (48h+): ' + staleSessions);
        continuityRows.push('- Channel attention needed: ' + channelAttention);
        var stale = (((continuity || {}).sessions) || {}).stale_48h || [];
        if (Array.isArray(stale) && stale.length) {
          var stalePreview = stale.slice(0, 3).map(function(row) {
            return '- `' + String((row && row.agent_id) || '?') + '` — ' + Number((row && row.age_hours) || 0).toFixed(1) + 'h';
          });
          continuityRows.push('');
          continuityRows.push('Stale session previews:');
          continuityRows = continuityRows.concat(stalePreview);
        }
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: continuityRows.join('\n'),
          meta: '',
          tools: [],
          system_origin: 'slash:continuity',
          ts: Date.now()
        });
        this.scrollToBottom();
      } catch (_) {}
    },

    runSlashAliases: function() {
      this.loadSlashAliases();
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: '**Slash Aliases**\n' + (this.formatSlashAliasRows() || '_No aliases configured_'),
        meta: '',
        tools: [],
        system_origin: 'slash:aliases',
        ts: Date.now()
      });
      this.scrollToBottom();
    },

    runSlashAliasCommand: function(cmdArgs) {
      var aliasTokens = String(cmdArgs || '').trim().split(/\s+/).filter(Boolean);
      if (aliasTokens.length < 2) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: 'Usage: `/alias /shortcut /target [extra args]`',
          meta: '',
          tools: [],
          system_origin: 'slash:alias',
          ts: Date.now()
        });
        this.scrollToBottom();
        return;
      }
      var aliasKey = String(aliasTokens[0] || '').trim().toLowerCase();
      var aliasTarget = String(aliasTokens.slice(1).join(' ') || '').trim().toLowerCase();
      if (!aliasKey.startsWith('/') || !aliasTarget.startsWith('/')) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: 'Alias and target must both start with `/`.',
          meta: '',
          tools: [],
          system_origin: 'slash:alias',
          ts: Date.now()
        });
        this.scrollToBottom();
        return;
      }
      this.loadSlashAliases();
      this.slashAliasMap[aliasKey] = aliasTarget;
      this.saveSlashAliases();
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: 'Saved alias `' + aliasKey + '` → `' + aliasTarget + '`',
        meta: '',
        tools: [],
        system_origin: 'slash:alias',
        ts: Date.now()
      });
      this.scrollToBottom();
    },

    runSlashOptimizeWorkers: async function() {
      try {
        var optimization = await InfringAPI.get('/api/continuity/pending');
        var pending = Number((((optimization || {}).tasks || {}).pending) || 0);
        var activeWorkers = Number((((optimization || {}).workers || {}).active_workers) || 0);
        var recommendation = pending > 0
          ? 'Queue has pending tasks. Keep workers in service mode:\n`infring task worker --service=1 --wait-ms=125 --idle-hibernate-ms=15000`'
          : 'Queue is empty. Workers can hibernate safely:\n`infring task worker --service=1 --idle-hibernate-ms=15000`';
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: '**Worker Optimization**\n- Pending tasks: ' + pending + '\n- Active workers: ' + activeWorkers + '\n\n' + recommendation,
          meta: '',
          tools: [],
          system_origin: 'slash:opt',
          ts: Date.now()
        });
        this.scrollToBottom();
      } catch (_) {}
    },
