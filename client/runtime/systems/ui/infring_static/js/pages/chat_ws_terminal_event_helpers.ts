// Chat websocket terminal output/error event handlers.
'use strict';

function infringChatWebSocketTerminalEventMethods() {
  return {
    handleWsTerminalOutputEvent: function(data) {
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._clearTypingTimeout();
      this.clearTerminalThinkingRows();
      var stdout = typeof data.stdout === 'string' ? data.stdout : '';
      var stderr = typeof data.stderr === 'string' ? data.stderr : '';
      var cleanStdout = stdout.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/^(?:[ \t]*\n)+|(?:\n[ \t]*)+$/g, '');
      var cleanStderr = stderr.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/^(?:[ \t]*\n)+|(?:\n[ \t]*)+$/g, '');
      var termText = '';
      if (cleanStdout.trim()) termText += cleanStdout;
      if (cleanStderr.trim()) termText += (termText ? '\n' : '') + cleanStderr;
      if (!termText.trim()) termText = '(no output)';
      var termMeta = 'exit ' + (Number.isFinite(Number(data.exit_code)) ? String(Number(data.exit_code)) : '1');
      var termDuration = this.formatResponseDuration(Number(data.duration_ms || 0));
      if (termDuration) termMeta += ' | ' + termDuration;
      var termCwd = this.terminalPromptPath;
      if (data.cwd) {
        termCwd = String(data.cwd);
        this.terminalCwd = termCwd;
        termMeta += ' | ' + termCwd;
      }
      var invokedBy = data && data.terminal_source ? String(data.terminal_source).trim().toLowerCase() : '';
      if (invokedBy === 'assistant') invokedBy = 'agent';
      if (invokedBy !== 'user' && invokedBy !== 'agent') invokedBy = '';
      var invokedCommand = String(
        (data && (data.command || data.requested_command || data.executed_command)) || ''
      ).trim();
      if (invokedBy === 'agent' && invokedCommand) {
        this._appendTerminalMessage({
          role: 'terminal',
          text: this._terminalPromptLine(termCwd, invokedCommand),
          meta: termCwd,
          tools: [],
          ts: Date.now(),
          terminal_source: 'agent',
          cwd: termCwd,
          agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
          agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
        });
      }
      var toolSummary = data && data.tool_summary && typeof data.tool_summary === 'object' ? data.tool_summary : null;
      if (toolSummary) {
        try { console.info('[terminal_tool_summary]', toolSummary); } catch (_) {}
      }
      this._appendTerminalMessage({
        role: 'terminal',
        text: termText,
        meta: termMeta,
        tools: [],
        ts: Date.now(),
        terminal_source: 'system',
        cwd: termCwd,
        agent_id: data && data.agent_id ? String(data.agent_id) : '',
        agent_name: data && data.agent_name ? String(data.agent_name) : ''
      });
      var terminalRecoveryHints = data && Array.isArray(data.recovery_hints) ? data.recovery_hints : [];
      if ((data && data.low_signal_output) || terminalRecoveryHints.length) {
        if (typeof this.addNoticeEvent === 'function') {
          this.addNoticeEvent({
            notice_label: terminalRecoveryHints.length ? 'Terminal recovery telemetry available.' : 'Terminal output was low-signal.',
            notice_type: 'info',
            ts: Date.now()
          });
        }
        try { console.info('[terminal_recovery_telemetry]', terminalRecoveryHints); } catch (_) {}
      }
      this.sending = false;
      this._responseStartedAt = 0;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() { self._processQueue(); });
      this.refreshPromptSuggestions(true, 'post-terminal');
    },

    handleWsTerminalErrorEvent: function(data) {
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._clearTypingTimeout();
      this.clearTerminalThinkingRows();
      var terminalErrorSource = data && data.terminal_source ? String(data.terminal_source).trim().toLowerCase() : '';
      if (terminalErrorSource === 'assistant') terminalErrorSource = 'agent';
      if (terminalErrorSource !== 'user' && terminalErrorSource !== 'agent') terminalErrorSource = '';
      var terminalErrorCommand = String(
        (data && (data.command || data.requested_command || data.executed_command)) || ''
      ).trim();
      var terminalErrorCwd = data && data.cwd ? String(data.cwd) : this.terminalPromptPath;
      if (terminalErrorSource === 'agent' && terminalErrorCommand) {
        this._appendTerminalMessage({
          role: 'terminal',
          text: this._terminalPromptLine(terminalErrorCwd, terminalErrorCommand),
          meta: terminalErrorCwd,
          tools: [],
          ts: Date.now(),
          terminal_source: 'agent',
          cwd: terminalErrorCwd,
          agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
          agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
        });
      }
      this._appendTerminalMessage({
        role: 'terminal',
        text: 'Terminal error: ' + (data && data.message ? data.message : 'command failed'),
        meta: '',
        tools: [],
        ts: Date.now(),
        terminal_source: 'system',
        cwd: terminalErrorCwd
      });
      var errorHints = data && Array.isArray(data.recovery_hints) ? data.recovery_hints : [];
      if (errorHints.length) {
        if (typeof this.addNoticeEvent === 'function') {
          this.addNoticeEvent({
            notice_label: 'Terminal error recovery telemetry available.',
            notice_type: 'warn',
            ts: Date.now()
          });
        }
        try { console.info('[terminal_error_recovery_telemetry]', errorHints); } catch (_) {}
      }
      this.sending = false;
      this._responseStartedAt = 0;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() { self._processQueue(); });
    },
  };
}
