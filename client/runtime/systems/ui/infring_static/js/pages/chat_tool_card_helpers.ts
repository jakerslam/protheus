// Chat tool card interaction, transient thinking, and message copy helpers.
'use strict';

function infringChatToolCardMethods() {
  return {
    ensureStreamingToolCard: function(msg, toolName, toolInput, options) {
      if (!msg || typeof msg !== 'object') return null;
      if (!Array.isArray(msg.tools)) msg.tools = [];
      var name = String(toolName || '').trim();
      if (!name) name = 'tool';
      var opts = options && typeof options === 'object' ? options : {};
      var identity = typeof this.toolAttemptIdentity === 'function'
        ? this.toolAttemptIdentity({ name: name, attempt_id: opts.attempt_id || '', attempt_sequence: opts.attempt_sequence || (msg.tools.length + 1), tool_attempt_receipt: opts.tool_attempt_receipt || null }, msg.tools.length, 'stream-tool')
        : { id: name + '-' + Date.now(), attempt_id: '', attempt_sequence: (msg.tools.length + 1), identity_key: name.toLowerCase() };
      var markRunning = opts.running !== false;
      var allowCreate = opts.no_create !== true;
      for (var i = msg.tools.length - 1; i >= 0; i--) {
        var card = msg.tools[i];
        if (!card) continue;
        var matchesIdentity = String(card.identity_key || '').trim() && String(card.identity_key || '').trim() === String(identity.identity_key || '').trim();
        if (!matchesIdentity && String(card.name || '') !== name) continue;
        if (markRunning && card.running) {
          if (!card.summary) card.summary = 'Tool running';
          if (!card.input_ref && opts.input_ref) card.input_ref = String(opts.input_ref || '');
          if (identity.id) card.id = identity.id;
          if (identity.attempt_id) card.attempt_id = identity.attempt_id; if (identity.attempt_sequence) card.attempt_sequence = identity.attempt_sequence; if (identity.identity_key) card.identity_key = identity.identity_key;
          return card;
        }
        if (!markRunning && card.running) {
          if (!card.summary) card.summary = 'Tool finished';
          if (!card.input_ref && opts.input_ref) card.input_ref = String(opts.input_ref || '');
          if (identity.id) card.id = identity.id;
          if (identity.attempt_id) card.attempt_id = identity.attempt_id; if (identity.attempt_sequence) card.attempt_sequence = identity.attempt_sequence; if (identity.identity_key) card.identity_key = identity.identity_key;
          card.running = false;
          return card;
        }
      }
      if (!allowCreate) return null;
      var created = { id: identity.id, name: name, running: markRunning, expanded: false, summary: markRunning ? 'Tool running' : 'Tool recorded', input_ref: String(opts.input_ref || identity.attempt_id || identity.id || ''), result_ref: String(opts.result_ref || identity.attempt_id || identity.id || ''), is_error: false, attempt_id: identity.attempt_id, attempt_sequence: identity.attempt_sequence, identity_key: identity.identity_key };
      msg.tools.push(created);
      return created;
    },

    currentToolDialogLabel: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
      for (var i = msg.tools.length - 1; i >= 0; i--) {
        var tool = msg.tools[i];
        if (!tool || this.isThoughtTool(tool) || !tool.running) continue;
        return this.toolThinkingActionLabel(tool);
      }
      return '';
    },

    hasRunningActionableTools: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return false;
      return msg.tools.some(function(tool) { return !!(tool && !this.isThoughtTool(tool) && tool.running); }, this);
    },

    clearTransientThinkingRows: function(options) {
      var opts = options && typeof options === 'object' ? options : {}, force = opts.force === true;
      var preserveRunningTools = !force && opts.preserve_running_tools !== false;
      var pendingAgentId = !force && opts.preserve_pending_ws !== false && this._pendingWsRequest && this._pendingWsRequest.agent_id ? String(this._pendingWsRequest.agent_id || '').trim() : '';
      var rows = Array.isArray(this.messages) ? this.messages : []; if (!rows.length) return 0;
      var kept = [], now = Date.now(), keptPending = false;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || (!row.thinking && !row.streaming)) { kept.push(row); continue; }
        var rowAgentId = String(row.agent_id || '').trim();
        var keep = (preserveRunningTools && this.hasRunningActionableTools(row)) || (!!pendingAgentId && (!rowAgentId || rowAgentId === pendingAgentId));
        if (!keep) continue;
        if (pendingAgentId && (!rowAgentId || rowAgentId === pendingAgentId)) keptPending = true;
        row.thinking = true; row.streaming = true; row._stream_updated_at = now;
        if (!Number.isFinite(Number(row._stream_started_at))) row._stream_started_at = now;
        if (!String(row.thinking_status || '').trim()) {
          var label = typeof this.currentToolDialogLabel === 'function' ? String(this.currentToolDialogLabel(row) || '').trim() : '';
          if (label) row.thinking_status = label;
        }
        kept.push(row);
      }
      if (typeof this.replaceActiveChatMessages === 'function') this.replaceActiveChatMessages(kept);
      else this.messages = kept;
      if (!force && pendingAgentId && !keptPending && typeof this.ensureLiveThinkingRow === 'function') {
        var restored = this.ensureLiveThinkingRow({ agent_id: pendingAgentId, agent_name: this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '' });
        if (restored) {
          restored.thinking = true; restored.streaming = true; restored._stream_updated_at = now;
          if (!Number.isFinite(Number(restored._stream_started_at))) restored._stream_started_at = now;
        }
      }
      return Math.max(0, rows.length - this.messages.length);
    },

    thoughtToolDurationSeconds: function(tool) {
      if (!tool || typeof tool !== 'object') return 0;
      var ms = Number(tool.duration_ms || tool.durationMs || tool.elapsed_ms || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      var seconds = Math.round(ms / 1000);
      if (ms > 0 && seconds < 1) seconds = 1;
      return Math.max(0, seconds);
    },

    thoughtToolLabel: function(tool) {
      return 'Thought for ' + this.thoughtToolDurationSeconds(tool) + ' seconds';
    },

    toolStatusText: function(tool) {
      if (!tool) return '';
      var state = typeof this.toolReceiptDisplayState === 'function'
        ? this.toolReceiptDisplayState(tool)
        : String(tool.status || '').trim().toLowerCase();
      if (tool.running || state === 'running') return 'running...';
      if (this.isThoughtTool(tool)) return 'thought';
      if (this.isBlockedTool(tool)) return 'blocked';
      if (state === 'error') return 'error';
      if (state === 'low_signal') return 'low signal';
      if (state === 'no_output') return 'no output';
      if (state === 'success' || state === 'ok' || state === 'done' || state === 'ready') return 'done';
      return state || '';
    },

    // Mark chat-rendered error messages for styling
    isErrorMessage: function(msg) {
      if (!msg || !msg.text) return false;
      if (String(msg.role || '').toLowerCase() !== 'system') return false;
      var t = String(msg.text).trim().toLowerCase();
      return t.startsWith('error:');
    },

    messageHasTools: function(msg) {
      return !!(msg && Array.isArray(msg.tools) && msg.tools.length);
    },

    allToolsCollapsed: function(msg) {
      if (!this.messageHasTools(msg)) return true;
      return !msg.tools.some(function(tool) {
        return !!(tool && tool.expanded);
      });
    },

    toggleMessageTools: function(msg) {
      if (!this.messageHasTools(msg)) return;
      var expand = this.allToolsCollapsed(msg);
      msg.tools.forEach(function(tool) {
        if (tool) tool.expanded = expand;
      });
      this.scheduleConversationPersist();
    },

    formatToolOutputForClipboard: function(text) {
      var raw = String(text == null ? '' : text);
      var trimmed = raw.trim();
      if (!trimmed) return '';
      if (trimmed.charAt(0) === '{' || trimmed.charAt(0) === '[') {
        try {
          return '```json\n' + JSON.stringify(JSON.parse(trimmed), null, 2) + '\n```';
        } catch (_) {}
      }
      return raw;
    },

    truncateToolOutputPreview: function(text) {
      var raw = String(text == null ? '' : text).trim();
      if (!raw) return '';
      var allLines = raw.split('\n');
      var maxLines = Number(this.toolPreviewMaxLines || 0);
      if (!Number.isFinite(maxLines) || maxLines < 1) maxLines = 2;
      var maxChars = Number(this.toolPreviewMaxChars || 0);
      if (!Number.isFinite(maxChars) || maxChars < 24) maxChars = 100;
      var preview = allLines.slice(0, maxLines).join('\n');
      if (preview.length > maxChars) return preview.slice(0, maxChars).trimEnd() + '…';
      return allLines.length > maxLines ? preview.trimEnd() + '…' : preview;
    },

    toolProjectionSections: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var sections = [];
      var summary = String(row.summary || row.display_text || '').trim();
      if (summary) {
        sections.push({ id: 'summary', label: 'Summary', text: summary });
      }
      var inputRef = String(row.input_ref || '').trim();
      if (inputRef) {
        sections.push({ id: 'input-ref', label: 'Input reference', text: inputRef });
      }
      var resultRef = String(row.result_ref || '').trim();
      if (resultRef) {
        sections.push({ id: 'result-ref', label: 'Result reference', text: resultRef });
      }
      if (!sections.length) {
        var status = this.toolStatusText(row);
        if (status) sections.push({ id: 'status', label: 'Status', text: status });
      }
      return sections;
    },

    messageCopyMarkdown: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var parts = [];
      var label = typeof this.messageActorLabel === 'function'
        ? String(this.messageActorLabel(row) || '').trim()
        : String(row.role || 'Message').trim();
      var stamp = typeof this.messageTimestampLabel === 'function' ? String(this.messageTimestampLabel(row) || '').trim() : '';
      if (label) parts.push('**' + label + '**');
      if (stamp) parts.push('_' + stamp + '_');

      var text = '';
      if (typeof this.extractMessageVisibleText === 'function') {
        text = String(this.extractMessageVisibleText(row) || '').trim();
      }
      if (!text && typeof this.messageVisiblePreviewText === 'function') {
        text = String(this.messageVisiblePreviewText(row) || '').trim();
      }
      if (!text) text = String(row.text || '').trim();
      if (text) parts.push(text);

      if (row.notice_label) {
        var notice = String(row.notice_label || '').trim();
        if (notice) parts.push('Notice: ' + notice);
      }

      var toolLines = [];
      var tools = Array.isArray(row.tools) ? row.tools : [];
      for (var i = 0; i < tools.length; i += 1) {
        var tool = tools[i] || {};
        var toolName = this.toolDisplayName(tool);
        var status = typeof this.toolReceiptDisplayState === 'function'
          ? String(this.toolReceiptDisplayState(tool) || '').trim()
          : String(tool.status || '').trim();
        var rendered = this.formatToolOutputForClipboard(tool.summary || tool.display_text || tool.result_ref || '');
        var preview = rendered ? this.truncateToolOutputPreview(rendered) : '';
        var line = '- ' + toolName;
        if (status) line += ' (' + status + ')';
        if (preview) line += ': ' + preview;
        toolLines.push(line);
      }
      if (toolLines.length) {
        parts.push('');
        parts.push('Tools:');
        for (var j = 0; j < toolLines.length; j += 1) parts.push(toolLines[j]);
      }

      if (row.file_output && row.file_output.path) parts.push('', 'File: `' + String(row.file_output.path).trim() + '`');
      if (row.folder_output && row.folder_output.path) parts.push('', 'Folder: `' + String(row.folder_output.path).trim() + '`');

      return parts.filter(function(part, idx, arr) {
        if (part !== '') return true;
        return idx > 0 && arr[idx - 1] !== '';
      }).join('\n').trim();
    },

    // Copy message text to clipboard as markdown
    copyMessage: function(msg) {
      if (!msg || msg._copying) return;
      var text = this.messageCopyMarkdown(msg);
      if (!text || !navigator.clipboard || typeof navigator.clipboard.writeText !== 'function') {
        InfringToast.error('Copy failed.');
        return;
      }
      msg._copying = true;
      navigator.clipboard.writeText(text).then(function() {
        msg._copying = false;
        msg._copied = true;
        setTimeout(function() { msg._copied = false; }, 1500);
      }).catch(function() {
        msg._copying = false;
        InfringToast.error('Copy failed.');
      });
    },
  };
}
