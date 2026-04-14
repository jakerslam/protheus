
    async ensureSystemTerminalSession() {
      var existing = String(this.systemTerminalSessionId || '').trim();
      if (existing) return existing;
      var preferredId = String(this.systemThreadId || 'system').trim() || 'system';
      try {
        var created = await InfringAPI.post('/api/terminal/sessions', {
          id: preferredId,
          cwd: this.terminalPromptPath
        });
        var sid = String(created && created.session && created.session.id ? created.session.id : preferredId).trim() || preferredId;
        this.systemTerminalSessionId = sid;
        return sid;
      } catch (_) {
        this.systemTerminalSessionId = preferredId;
        return preferredId;
      }
    },

    async _sendSystemTerminalPayload(command) {
      var cmd = String(command || '').trim();
      if (!cmd) return;
      this.sending = true;
      this.setAgentLiveActivity(this.systemThreadId || 'system', 'working');
      this._responseStartedAt = Date.now();
      this._appendTerminalMessage({
        role: 'terminal',
        text: this._terminalPromptLine(this.terminalPromptPath, cmd),
        meta: this.terminalPromptPath,
        tools: [],
        ts: Date.now(),
        terminal_source: 'user',
        cwd: this.terminalPromptPath
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
      try {
        var response = null;
        for (var attempt = 0; attempt < 2; attempt += 1) {
          var sessionId = await this.ensureSystemTerminalSession();
          response = await InfringAPI.post('/api/terminal/queue', {
            session_id: sessionId,
            command: cmd,
            cwd: this.terminalPromptPath
          });
          if (response && String(response.error || '').trim() === 'session_not_found') {
            this.systemTerminalSessionId = '';
            continue;
          }
          break;
        }
        if (!response || response.ok === false) {
          throw new Error(String((response && response.error) || 'terminal_exec_failed'));
        }
        this.handleWsMessage({
          type: 'terminal_output',
          stdout: response && response.stdout ? String(response.stdout) : '',
          stderr: response && response.stderr ? String(response.stderr) : '',
          exit_code: Number(response && response.exit_code != null ? response.exit_code : 1),
          duration_ms: 0,
          cwd: this.terminalPromptPath,
          terminal_source: 'system',
          requested_command: response && response.requested_command ? String(response.requested_command) : '',
          executed_command: response && response.executed_command ? String(response.executed_command) : '',
          command_translated: !!(response && response.command_translated),
          translation_reason: response && response.translation_reason ? String(response.translation_reason) : '',
          suggestions: response && Array.isArray(response.suggestions) ? response.suggestions : [],
          permission_gate: response && response.permission_gate ? response.permission_gate : null,
          filter_events: response && Array.isArray(response.filter_events) ? response.filter_events : [],
          low_signal_output: !!(response && response.low_signal_output),
          recovery_hints: response && Array.isArray(response.recovery_hints) ? response.recovery_hints : [],
          tool_summary: response && response.tool_summary ? response.tool_summary : null,
          tracking: response && response.tracking ? response.tracking : null
        });
      } catch (error) {
        this.handleWsMessage({
          type: 'terminal_error',
          message: error && error.message ? error.message : 'command failed',
          terminal_source: 'system'
        });
      }
    },

