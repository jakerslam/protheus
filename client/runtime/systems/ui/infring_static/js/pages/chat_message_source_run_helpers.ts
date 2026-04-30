// Chat message source-run, preview, and agent terminal transcript helpers.
'use strict';

function infringChatMessageSourceRunMethods() {
  return {
    isStackBoundaryNoticeMessage(msg) {
      if (!msg || msg.terminal) return false;
      if (msg.is_notice) return true;
      if (msg.notice_label || msg.notice_type || msg.notice_action) return true;
      var role = String(msg.role || '').trim().toLowerCase();
      if (role !== 'system') return false;
      var text = String(msg.text || '').trim();
      if (!text) return false;
      if (this.isModelSwitchNoticeLabel(text)) return true;
      if (/^changed name from\s+/i.test(text)) return true;
      if (/^initialized\s+.+\s+as\s+/i.test(text)) return true;
      return false;
    },
    messageSourceKey(msg) {
      if (!msg || msg.is_notice) return '';
      if (this.isStackBoundaryNoticeMessage(msg)) {
        var noticeLabel = String(msg.notice_label || msg.text || '').trim().toLowerCase();
        var noticeTs = Number(msg.ts || 0) || 0;
        return 'notice:' + noticeLabel + ':' + noticeTs;
      }
      if (msg.terminal) {
        var terminalSource = this.terminalMessageSource(msg);
        if (terminalSource === 'user') return 'terminal:user';
        if (terminalSource === 'system') return 'terminal:system';
        var terminalAgentId = String((msg && msg.agent_id) || (this.currentAgent && this.currentAgent.id) || '').trim();
        return terminalAgentId ? ('terminal:agent:' + terminalAgentId.toLowerCase()) : 'terminal:agent';
      }
      var role = String(msg.role || '').trim().toLowerCase();
      if (!role) return '';
      if (role === 'user') return 'user';
      if (role === 'system') {
        // System rows should stack as one source-run when consecutive, regardless
        // of internal origin tags (inject:test, runtime:error, slash:status, etc).
        // This keeps UI grouping consistent for user-facing system narration.
        return 'system';
      }
      if (role === 'agent') {
        var agentOrigin = String(
          (msg && msg.agent_origin) ||
          (msg && msg.source_agent_id) ||
          (msg && msg.agent_id) ||
          (msg && msg.actor_id) ||
          (msg && msg.actor) ||
          (msg && msg.agent_name) ||
          ''
        ).trim();
        if (!agentOrigin && this.currentAgent && this.currentAgent.id) {
          agentOrigin = String(this.currentAgent.id || '').trim();
        }
        return agentOrigin ? ('agent:' + agentOrigin.toLowerCase()) : 'agent';
      }
      var genericOrigin = String(
        (msg && msg.agent_id) ||
        (msg && msg.actor_id) ||
        (msg && msg.actor) ||
        ''
      ).trim();
      return genericOrigin ? (role + ':' + genericOrigin.toLowerCase()) : role;
    },
    isFirstInSourceRun(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx === 0) return true;
      var prev = list[idx - 1];
      if (!prev || this.isStackBoundaryNoticeMessage(prev)) return true;
      var prevKey = this.messageSourceKey(prev);
      return prevKey !== currKey;
    },
    isLastInSourceRun(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx >= list.length - 1) return true;
      var next = list[idx + 1];
      if (!next || this.isStackBoundaryNoticeMessage(next)) return true;
      var nextKey = this.messageSourceKey(next);
      return nextKey !== currKey;
    },
    messagePreview(msg) {
      if (!msg) return '';
      if (msg.is_notice && msg.notice_label) {
        return String(msg.notice_label);
      }
      var raw = '';
      if (typeof msg.text === 'string' && msg.text.trim()) {
        raw = msg.text;
      } else if (Array.isArray(msg.tools) && msg.tools.length) {
        raw = 'Tool calls: ' + msg.tools.map(function(tool) {
          return tool && tool.name ? tool.name : 'tool';
        }).join(', ');
      } else {
        raw = '[' + (msg.role || 'message') + ']';
      }
      var compact = raw.replace(/\s+/g, ' ').trim();
      if (compact.length > 140) return compact.slice(0, 137) + '...';
      return compact;
    },
    messageMapPreview(msg) {
      if (this.messageMapMarkerType(msg) === 'tool') {
        return this.messageToolPreview(msg);
      }
      return this.messagePreview(msg);
    },
    appendAgentTerminalTranscript(rows) {
      if (!Array.isArray(rows) || !rows.length || typeof this._appendTerminalMessage !== 'function') return false;
      var appended = false;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var cwd = row.cwd ? String(row.cwd) : this.terminalPromptPath;
        var command = row.command ? String(row.command).trim() : '';
        var output = row.output ? String(row.output).trim() : '';
        if (command) {
          this._appendTerminalMessage({ role: 'terminal', text: this._terminalPromptLine(cwd, command), meta: cwd, tools: [], ts: Date.now(), terminal_source: 'agent', cwd: cwd });
          appended = true;
        }
        if (output) {
          this._appendTerminalMessage({ role: 'terminal', text: output, meta: row.is_error ? 'command failed' : 'command output', tools: [], ts: Date.now(), terminal_source: 'system', cwd: cwd, _terminal_compact: output.length > 500 });
          appended = true;
        }
      }
      return appended;
    },
  };
}
