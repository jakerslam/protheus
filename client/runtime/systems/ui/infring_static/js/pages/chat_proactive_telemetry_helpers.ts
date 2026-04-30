// Chat proactive telemetry alert and thinking trace display helpers.
'use strict';

function infringChatProactiveTelemetryMethods() {
  return {
    fetchProactiveTelemetryAlerts(notify) {
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
            console.log('[telemetry alerts]', rendered, nextRendered || '');
            InfringToast.info('Telemetry alerts updated; details are in DevTools console.');
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
    staleMemoryWarningText() {
      return '';
    },
    thinkingTraceRows(msg) {
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
  };
}
