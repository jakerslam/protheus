    runSlashAlerts: async function() {
      try {
        var alertsPayload = await this.fetchProactiveTelemetryAlerts(false);
        var alertsRows = Array.isArray(alertsPayload && alertsPayload.alerts) ? alertsPayload.alerts : [];
        if (!alertsRows.length) {
          this.pushSystemMessage({
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
          this.pushSystemMessage({
            id: ++msgId,
            role: 'system',
            text: '**Telemetry Alerts**\n' + alertText,
            meta: '',
            tools: [],
            system_origin: 'slash:alerts',
            ts: Date.now()
          });
        }
      } catch (error) {
        this.emitCommandFailureNotice('/alerts', error, ['/status', '/continuity']);
      }
    },

    runSlashNextActions: async function() {
      try {
        var alertsPayload = await this.fetchProactiveTelemetryAlerts(false);
        var rows = Array.isArray(alertsPayload && alertsPayload.next_actions)
          ? alertsPayload.next_actions
          : [];
        if (!rows.length) {
          this.pushSystemMessage({
            id: ++msgId,
            role: 'system',
            text: 'No predicted next actions right now.',
            meta: '',
            tools: [],
            system_origin: 'slash:next',
            ts: Date.now()
          });
          return;
        }
        var rendered = rows.slice(0, 6).map(function(row) {
          var cmd = String((row && row.command) || '').trim();
          var reason = String((row && row.reason) || '').trim();
          var priority = String((row && row.priority) || 'low').toUpperCase();
          return '- [' + priority + '] `' + cmd + '`' + (reason ? ('\n  ↳ ' + reason) : '');
        }).join('\n');
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          text: '**Predicted Next Actions**\n' + rendered,
          meta: '',
          tools: [],
          system_origin: 'slash:next',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/next', error, ['/alerts', '/status']);
      }
    },

    runSlashMemoryHygiene: async function() {
      try {
        var alertsPayload = await this.fetchProactiveTelemetryAlerts(false);
        var hygiene = alertsPayload && alertsPayload.memory_hygiene ? alertsPayload.memory_hygiene : {};
        var stale48 = Number(hygiene.stale_contexts_48h || 0);
        var stale7d = Number(hygiene.stale_contexts_7d || 0);
        var bytes = Number(hygiene.snapshot_history_bytes || 0);
        var overCap = !!hygiene.snapshot_history_over_soft_cap;
        var recs = Array.isArray(hygiene.recommendations) ? hygiene.recommendations : [];
        var recText = recs.slice(0, 4).map(function(row) {
          var cmd = String((row && row.command) || '').trim();
          var reason = String((row && row.reason) || '').trim();
          return '- `' + cmd + '`' + (reason ? ('\n  ↳ ' + reason) : '');
        }).join('\n');
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          text:
            '**Memory Hygiene**\n' +
            '- Stale contexts (48h+): ' + stale48 + '\n' +
            '- Stale contexts (7d+): ' + stale7d + '\n' +
            '- Snapshot history bytes: ' + bytes + '\n' +
            '- Over soft cap: ' + (overCap ? 'yes' : 'no') +
            (recText ? ('\n\nRecommended actions:\n' + recText) : ''),
          meta: '',
          tools: [],
          system_origin: 'slash:memory',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/memory', error, ['/continuity', '/alerts']);
      }
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
        var activeAgentRows = ((((continuity || {}).active_agents) || {}).rows) || [];
        if (Array.isArray(activeAgentRows) && activeAgentRows.length) {
          continuityRows.push('');
          continuityRows.push('Active agent markers:');
          var markers = activeAgentRows.slice(0, 4).map(function(row) {
            var id = String((row && row.agent_id) || '?');
            var objective = String((row && row.objective) || '').trim();
            if (objective.length > 70) objective = objective.slice(0, 67) + '...';
            var completion = Number((row && row.completion_percent) || 0);
            return '- `' + id + '` — ' + objective + ' (' + completion + '%)';
          });
          continuityRows = continuityRows.concat(markers);
        }
        var stale = (((continuity || {}).sessions) || {}).stale_48h || [];
        if (Array.isArray(stale) && stale.length) {
          var stalePreview = stale.slice(0, 3).map(function(row) {
            return '- `' + String((row && row.agent_id) || '?') + '` — ' + Number((row && row.age_hours) || 0).toFixed(1) + 'h';
          });
          continuityRows.push('');
          continuityRows.push('Stale session previews:');
          continuityRows = continuityRows.concat(stalePreview);
        }
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          text: continuityRows.join('\n'),
          meta: '',
          tools: [],
          system_origin: 'slash:continuity',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/continuity', error, ['/status', '/alerts']);
      }
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
        this.pushSystemMessage({
          id: ++msgId,
          role: 'system',
          text: '**Worker Optimization**\n- Pending tasks: ' + pending + '\n- Active workers: ' + activeWorkers + '\n\n' + recommendation,
          meta: '',
          tools: [],
          system_origin: 'slash:opt',
          ts: Date.now()
        });
      } catch (error) {
        this.emitCommandFailureNotice('/opt', error, ['/status', '/continuity']);
      }
    },
