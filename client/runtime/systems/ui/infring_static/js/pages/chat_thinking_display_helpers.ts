// Chat thinking bubble and trace display helpers.
'use strict';

function infringChatThinkingDisplayMethods() {
  return {
    ensureLiveThinkingRow: function(data) {
      var incomingStatus = String(
        data && (data.thinking_status || data.status_text) ? (data.thinking_status || data.status_text) : ''
      ).trim();
      if (incomingStatus && typeof this.normalizeThinkingStatusCandidate === 'function') {
        incomingStatus = this.normalizeThinkingStatusCandidate(incomingStatus);
      }
      var row = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (row && (row.thinking || row.streaming)) {
        row.thinking = true;
        row.streaming = true;
        if (!Number.isFinite(Number(row._stream_started_at))) row._stream_started_at = Date.now();
        row._stream_updated_at = Date.now();
        if (
          incomingStatus &&
          (
            !String(row.thinking_status || '').trim() ||
            (
              typeof this.isThinkingPlaceholderText === 'function' &&
              this.isThinkingPlaceholderText(row.thinking_status)
            )
          )
        ) {
          row.thinking_status = incomingStatus;
        }
        this.syncActiveChatMessages();
        return row;
      }
      row = {
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: '',
        thinking: true,
        streaming: true,
        thinking_status: incomingStatus,
        tools: [],
        _stream_started_at: Date.now(),
        _stream_updated_at: Date.now(),
        ts: Date.now(),
        agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
        agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
      };
      this.messages.push(row);
      this.syncActiveChatMessages();
      return row;
    },

    isThinkingPlaceholderText(input) {
      var value = String(input || '').replace(/<[^>]*>/g, ' ').replace(/\*+/g, '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!value) return true;
      if (/^(thinking|processing|working|preparing response|reasoning through context)(\.\.\.|…)?$/.test(value)) return true;
      if (/^waiting for (workflow completion|runtime response)(\.\.\.|…)?$/.test(value)) return true;
      if (/^reconnected\. syncing response(\.\.\.|…)?$/.test(value)) return true;
      if (/^(using|calling)\b.+(\.\.\.|…)?$/.test(value)) return true;
      var stripped = value.replace(/[.,!?;:…-]+/g, ' ').replace(/\s+/g, ' ').trim();
      if (stripped) {
        var words = stripped.split(' ').filter(function(part) { return !!part; });
        var placeholderLexicon = {
          thinking: true,
          processing: true,
          working: true,
          preparing: true,
          response: true,
          reasoning: true,
          through: true,
          context: true,
          waiting: true,
          workflow: true,
          completion: true,
          runtime: true,
          reconnected: true,
          syncing: true
        };
        if (words.length > 0 && words.length <= 24) {
          var allPlaceholder = words.every(function(word) {
            return !!placeholderLexicon[word];
          });
          if (allPlaceholder) return true;
        }
      }
      return false;
    },

    normalizeThinkingStatusCandidate(rawStatus) {
      var value = String(rawStatus || '').replace(/\r/g, '\n').trim();
      if (!value) return '';
      var lines = value
        .split('\n')
        .map(function(line) { return String(line || '').replace(/\s+/g, ' ').trim(); })
        .filter(function(line) { return !!line; });
      if (!lines.length) return '';
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').trim();
        if (!line) continue;
        if (this.isThinkingPlaceholderText(line)) continue;
        line = line.replace(/\[(?:end|done|start)\]/ig, '').replace(/\s+/g, ' ').trim();
        if (!line) continue;
        var lowered = line.toLowerCase();
        if (/^(active|idle|running)$/.test(lowered)) continue;
        if (/^phase[:\s]/.test(lowered)) {
          line = line.replace(/^phase[:\s]*/i, '').trim();
          lowered = line.toLowerCase();
        }
        if (/web[_\s-]?search|searching (the )?(web|internet)|duckduckgo|serp/.test(lowered)) {
          line = 'Searching internet';
        } else if (/web[_\s-]?fetch|reading web|browse|browsing/.test(lowered)) {
          line = 'Reading web pages';
        } else if (/read(_|\s)?file|file read|reading files?/.test(lowered)) {
          line = 'Scanning files';
        } else if (/folder|directory|filesystem scan|scan folders?/.test(lowered)) {
          line = 'Scanning folders';
        } else if (/terminal|shell|command execution|run command/.test(lowered)) {
          line = 'Running terminal command';
        } else if (/spawn_subagents|spawn_swarm|subagents?|swarm|parallel workers?/.test(lowered)) {
          line = 'Summoning agents';
        } else if (/memory.*query|semantic memory|vector search/.test(lowered)) {
          line = 'Searching memory';
        } else if (/context warning|context limit|context window/.test(lowered)) {
          line = 'Context window warning';
        }
        line = String(line || '').replace(/\s+/g, ' ').trim();
        if (!line || this.isThinkingPlaceholderText(line)) continue;
        if (line.length > 220) line = line.slice(0, 217) + '...';
        return line;
      }
      return '';
    },

    isThinkingShimmerText: function(msg) {
      if (!msg || !msg.thinking) return false;
      var status = typeof this.thinkingStatusText === 'function'
        ? String(this.thinkingStatusText(msg) || '').trim()
        : String(msg.thinking_status || msg.status_text || '').trim();
      if (!status) return true;
      if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(status)) return true;
      return true;
    },

    thinkingPhaseText: function(msg) {
      if (!msg || !msg.thinking) return '';
      var primary = typeof this.thinkingStatusText === 'function'
        ? String(this.thinkingStatusText(msg) || '').trim()
        : '';
      var primaryNorm = primary.toLowerCase().replace(/\s+/g, ' ').trim();
      var summary = this.thinkingToolStatusSummary(msg);
      if (summary && summary.text) {
        var summaryText = String(summary.text || '').trim();
        var summaryNorm = summaryText.toLowerCase().replace(/\s+/g, ' ').trim();
        if (
          summaryNorm &&
          primaryNorm &&
          (summaryNorm === primaryNorm || summaryNorm.indexOf(primaryNorm) >= 0 || primaryNorm.indexOf(summaryNorm) >= 0)
        ) {
          return '';
        }
        return summaryText;
      }
      if (primaryNorm && primaryNorm !== 'thinking') {
        // Prevent duplicate waiting/workflow status lines.
        return '';
      }
      if (this._pendingWsRequest && this._pendingWsRequest.agent_id) return 'Waiting for runtime response...';
      return 'Analyzing next step...';
    },

    thinkingTraceSummary: function(msg) {
      if (!msg || !msg.thinking) return '';
      var rows = this.messageToolTraceRows(msg);
      if (!rows.length) return '';
      var running = rows.filter(function(row) { return row.state === 'running'; });
      if (running.length) {
        return running.slice(0, 2).map(function(row) { return row.label; }).join(' · ');
      }
      var failed = rows.filter(function(row) { return row.state === 'error' || row.state === 'blocked'; });
      if (failed.length) {
        return failed.slice(0, 2).map(function(row) { return row.label + ' (' + row.state + ')'; }).join(' · ');
      }
      return rows.slice(0, 2).map(function(row) { return row.label; }).join(' · ');
    },

    thinkingWorkflowStatusLine: function(msg) {
      if (!msg || !msg.thinking) return '';
      var toolDialog = typeof this.currentToolDialogLabel === 'function'
        ? String(this.currentToolDialogLabel(msg) || '').trim()
        : '';
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        toolDialog = this.normalizeThinkingStatusCandidate(toolDialog);
      }
      if (toolDialog) return toolDialog;
      var explicitStatus = String(msg.thinking_status || msg.status_text || '').trim();
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        explicitStatus = this.normalizeThinkingStatusCandidate(explicitStatus);
      }
      if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(explicitStatus)) {
        return '';
      }
      return explicitStatus;
    },

    thinkingInnerDialogLine: function(msg) {
      if (!msg || !msg.thinking) return '';
      var thought = typeof this.thinkingDisplayText === 'function'
        ? String(this.thinkingDisplayText(msg) || '').trim()
        : '';
      if (!thought) {
        thought = String(msg._reasoning || msg._thoughtText || '').trim();
      }
      if (!thought && msg && msg.thoughtStreaming) {
        thought = String(msg._thought_latest_chunk || '').trim();
      }
      if (typeof this.normalizeThinkingStatusCandidate === 'function') {
        thought = this.normalizeThinkingStatusCandidate(thought);
      }
      if (!thought) return '';
      var lowered = thought.toLowerCase().replace(/\s+/g, ' ').trim();
      if (!lowered || lowered === 'thinking') return '';
      if (thought.length > 180) thought = thought.slice(0, 177).trim() + '...';
      return thought;
    },

    thinkingBubbleLineText: function(msg) {
      if (!msg || !msg.thinking) return '';
      var primary = typeof this.thinkingWorkflowStatusLine === 'function'
        ? String(this.thinkingWorkflowStatusLine(msg) || '').trim()
        : '';
      var primaryNorm = primary.toLowerCase().replace(/\s+/g, ' ').trim();
      var thought = typeof this.thinkingInnerDialogLine === 'function'
        ? String(this.thinkingInnerDialogLine(msg) || '').trim()
        : '';
      var thoughtNorm = thought.toLowerCase().replace(/\s+/g, ' ').trim();
      if (primary && primaryNorm && primaryNorm !== 'thinking') {
        if (
          thought &&
          thoughtNorm &&
          thoughtNorm !== primaryNorm &&
          thoughtNorm.indexOf(primaryNorm) === -1 &&
          primaryNorm.indexOf(thoughtNorm) === -1
        ) {
          var composedPrimary = primary.replace(/(\.\.\.|…)+$/g, '').trim();
          if (composedPrimary && !/[.!?:]$/.test(composedPrimary)) composedPrimary += '...';
          else if (composedPrimary && /[.!?:]$/.test(composedPrimary) && !/(\.\.\.|…)$/.test(composedPrimary)) composedPrimary += ' ';
          return (composedPrimary + ' ' + thought).replace(/\s+/g, ' ').trim();
        }
        return primary;
      }
      if (thought) return thought;
      var phase = typeof this.thinkingPhaseText === 'function'
        ? String(this.thinkingPhaseText(msg) || '').trim()
        : '';
      if (phase) return phase;
      var trace = typeof this.thinkingTraceSummary === 'function'
        ? String(this.thinkingTraceSummary(msg) || '').trim()
        : '';
      if (trace) return trace;
      if (primary) return primary;
      return 'Thinking';
    },
  };
}
