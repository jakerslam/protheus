// Chat message thinking status, origin, and stat label helpers.
'use strict';

function infringChatMessageStatusStatMethods() {
  return {
    thinkingToolStatusSummary: function(msg) {
      var summary = { text: '', hasRunning: false };
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return summary;
      var runningNames = [];
      var completed = 0;
      var errors = 0;
      var blocked = 0;
      var lastFinishedName = '';
      for (var ri = msg.tools.length - 1; ri >= 0; ri--) {
        var recent = msg.tools[ri];
        if (!recent || recent.running || this.isThoughtTool(recent)) continue;
        var recentName = this.toolDisplayName(recent);
        if (recentName) { lastFinishedName = recentName; break; }
      }
      for (var i = 0; i < msg.tools.length; i++) {
        var tool = msg.tools[i];
        if (!tool || this.isThoughtTool(tool)) continue;
        if (tool.running) {
          var runningName = typeof this.toolThinkingActionLabel === 'function'
            ? this.toolThinkingActionLabel(tool)
            : this.toolDisplayName(tool);
          if (runningName) runningNames.push(runningName);
          continue;
        }
        if (this.isBlockedTool(tool)) {
          blocked += 1;
          continue;
        }
        if (tool.is_error) {
          errors += 1;
          continue;
        }
        completed += 1;
      }
      summary.hasRunning = runningNames.length > 0;
      var doneCount = completed + errors + blocked;
      if (summary.hasRunning) {
        summary.text = runningNames.length === 1
          ? (runningNames[0] + '...')
          : ('Running ' + runningNames.length + ' tools...');
        var runningBits = [];
        if (doneCount > 0) runningBits.push(doneCount + ' done');
        if (errors > 0) runningBits.push(errors + ' error');
        if (blocked > 0) runningBits.push(blocked + ' blocked');
        if (runningBits.length) summary.text += ' · ' + runningBits.join(', ');
        return summary;
      }
      if (!doneCount) return summary;
      summary.text = lastFinishedName ? ('Finished ' + lastFinishedName) : 'Tool steps complete';
      var doneBits = [];
      if (completed > 0) doneBits.push(completed + ' done');
      if (errors > 0) doneBits.push(errors + ' error');
      if (blocked > 0) doneBits.push(blocked + ' blocked');
      if (doneBits.length) summary.text += ' · ' + doneBits.join(', ');
      return summary;
    },

    thinkingStatusText: function(msg) {
      if (!msg || !msg.thinking) return '';
      var toolDialog = typeof this.currentToolDialogLabel === 'function'
        ? String(this.currentToolDialogLabel(msg) || '').trim()
        : '';
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        toolDialog = this.normalizeThinkingStatusCandidate(toolDialog);
      }
      if (toolDialog) {
        return toolDialog;
      }
      var thoughtLine = typeof this.thinkingDisplayText === 'function'
        ? String(this.thinkingDisplayText(msg) || '').trim()
        : '';
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        thoughtLine = this.normalizeThinkingStatusCandidate(thoughtLine);
      }
      if (thoughtLine) {
        return thoughtLine;
      }
      var status = typeof this.normalizeThinkingStatusCandidate === 'function'
        ? this.normalizeThinkingStatusCandidate(msg.thinking_status || msg.status_text || '')
        : String(msg.thinking_status || msg.status_text || '').trim();
      if (status) return status;
      return 'Thinking';
    },

    messageGroupRole: function(msg) {
      if (!msg) return '';
      if (msg.terminal) return 'terminal';
      return String(msg.role || '');
    },

    messageOriginKind: function(msg) {
      if (!msg || typeof msg !== 'object') return 'other';
      if (msg.terminal) {
        var terminalSource = typeof this.terminalMessageSource === 'function'
          ? this.terminalMessageSource(msg)
          : String(msg.terminal_source || '').trim().toLowerCase();
        if (terminalSource === 'user') return 'human';
        if (terminalSource === 'agent' || terminalSource === 'assistant') return 'agent';
        return 'system';
      }
      var role = String(msg.role || '').trim().toLowerCase();
      if (!role) return 'other';
      if (role === 'assistant') role = 'agent';
      if (role === 'user' || role === 'human') return 'human';
      if (role === 'agent') return 'agent';
      if (role === 'system') return 'system';
      return 'other';
    },

    messageIsAgentOrigin: function(msg) {
      return this.messageOriginKind(msg) === 'agent';
    },

    messageIsHumanOrigin: function(msg) {
      return this.messageOriginKind(msg) === 'human';
    },

    messageStatReadNumberPath: function(source, path) {
      if (!source || typeof source !== 'object') return 0;
      var keyPath = String(path || '').trim();
      if (!keyPath) return 0;
      var target = source;
      var parts = keyPath.split('.');
      for (var i = 0; i < parts.length; i += 1) {
        var key = String(parts[i] || '').trim();
        if (!key || !target || typeof target !== 'object' || !Object.prototype.hasOwnProperty.call(target, key)) return 0;
        target = target[key];
      }
      var numeric = Number(typeof target === 'string' ? target.replace(/,/g, '').trim() : target);
      if (!Number.isFinite(numeric) || numeric <= 0) return 0;
      return numeric;
    },

    messageStatReadNumberFromPaths: function(msg, paths) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var probes = Array.isArray(paths) ? paths : [];
      for (var i = 0; i < probes.length; i += 1) {
        var numeric = this.messageStatReadNumberPath(row, probes[i]);
        if (numeric > 0) return numeric;
      }
      return 0;
    },

    messageStatDurationFromMeta: function(msg) {
      var meta = String(msg && msg.meta || '').trim();
      if (!meta) return 0;
      var minuteMatch = meta.match(/(?:^|\|)\s*([0-9]{1,3})\s*m\s*([0-9]{1,2})\s*s\s*(?:\||$)/i);
      if (minuteMatch) {
        var min = Number(minuteMatch[1] || 0);
        var sec = Number(minuteMatch[2] || 0);
        if (Number.isFinite(min) && Number.isFinite(sec) && (min > 0 || sec > 0)) return (min * 60000) + (sec * 1000);
      }
      var secondMatch = meta.match(/(?:^|\|)\s*([0-9]+(?:\.[0-9]+)?)\s*s\s*(?:\||$)/i);
      if (secondMatch) {
        var seconds = Number(secondMatch[1] || 0);
        if (Number.isFinite(seconds) && seconds > 0) return Math.round(seconds * 1000);
      }
      var milliMatch = meta.match(/(?:^|\|)\s*([0-9]+(?:\.[0-9]+)?)\s*ms\s*(?:\||$)/i);
      if (milliMatch) {
        var millis = Number(milliMatch[1] || 0);
        if (Number.isFinite(millis) && millis > 0) return Math.round(millis);
      }
      return 0;
    },

    messageStatResponseTimeMs: function(msg) {
      if (!msg || typeof msg !== 'object') return 0;
      var fromPayload = this.messageStatReadNumberFromPaths(msg, [
        'duration_ms',
        'elapsed_ms',
        'response_ms',
        'response_time_ms',
        'responseTimeMs',
        'latency_ms',
        'latencyMs',
        'turn_transaction.duration_ms',
        'turn_transaction.elapsed_ms',
        'turn_transaction.response_ms',
        'turn_transaction.response_time_ms',
        'turn_transaction.responseTimeMs',
        'turn_transaction.metrics.duration_ms',
        'response_finalization.duration_ms',
        'response_finalization.elapsed_ms',
        'response_finalization.response_ms',
        'response_finalization.response_time_ms',
        'response_workflow.duration_ms'
      ]);
      if (fromPayload > 0) return fromPayload;
      return this.messageStatDurationFromMeta(msg);
    },

    messageStatResponseTimeText: function(msg) {
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      var durationMs = this.messageStatResponseTimeMs(msg);
      if (service && typeof service.responseTimeText === 'function') {
        return service.responseTimeText(msg, durationMs, typeof this.formatResponseDuration === 'function' ? this.formatResponseDuration.bind(this) : null);
      }
      if (!msg || msg.thinking || msg.is_notice || !durationMs || durationMs <= 0) return '';
      return Math.round(durationMs) + 'ms';
    },

    messageStatTokensFromMeta: function(msg) {
      var meta = String(msg && msg.meta || '').trim();
      if (!meta) return 0;
      var tokenMatch = meta.match(/([0-9][0-9,]*)\s*in\s*\/\s*([0-9][0-9,]*)\s*out/i);
      if (!tokenMatch) return 0;
      var inTokens = Number(String(tokenMatch[1] || '0').replace(/,/g, ''));
      var outTokens = Number(String(tokenMatch[2] || '0').replace(/,/g, ''));
      if (!Number.isFinite(inTokens) || inTokens < 0) inTokens = 0;
      if (!Number.isFinite(outTokens) || outTokens < 0) outTokens = 0;
      return inTokens + outTokens;
    },

    messageStatBurnTotalTokens: function(msg) {
      if (!msg || typeof msg !== 'object') return 0;
      var total = this.messageStatReadNumberFromPaths(msg, [
        'total_tokens',
        'usage.total_tokens',
        'token_usage.total_tokens',
        'turn_transaction.total_tokens',
        'turn_transaction.usage.total_tokens',
        'turn_transaction.token_usage.total_tokens',
        'response_finalization.total_tokens',
        'response_finalization.usage.total_tokens',
        'response_workflow.total_tokens'
      ]);
      if (total > 0) return total;
      var inTokens = this.messageStatReadNumberFromPaths(msg, [
        'input_tokens',
        'usage.input_tokens',
        'token_usage.input_tokens',
        'turn_transaction.input_tokens',
        'turn_transaction.usage.input_tokens',
        'turn_transaction.token_usage.input_tokens',
        'response_finalization.input_tokens',
        'response_finalization.usage.input_tokens',
        'response_workflow.input_tokens'
      ]);
      var outTokens = this.messageStatReadNumberFromPaths(msg, [
        'output_tokens',
        'usage.output_tokens',
        'token_usage.output_tokens',
        'turn_transaction.output_tokens',
        'turn_transaction.usage.output_tokens',
        'turn_transaction.token_usage.output_tokens',
        'response_finalization.output_tokens',
        'response_finalization.usage.output_tokens',
        'response_workflow.output_tokens'
      ]);
      var combined = inTokens + outTokens;
      if (combined > 0) return combined;
      return this.messageStatTokensFromMeta(msg);
    },

    messageStatBurnLabelText: function(msg) {
      var total = this.messageStatBurnTotalTokens(msg);
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      if (service && typeof service.burnLabelText === 'function') {
        return service.burnLabelText(msg, total, typeof this.formatTokenK === 'function' ? this.formatTokenK.bind(this) : null);
      }
      if (!msg || msg.thinking || msg.is_notice || !Number.isFinite(total) || total <= 0) return '';
      return total < 1000 ? String(Math.round(total)) : ((Math.round((total / 1000) * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'k');
    },
  };
}
