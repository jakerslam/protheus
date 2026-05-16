
    async ensureSystemTerminalSession() {
      var existing = String(this.systemTerminalSessionId || '').trim();
      if (existing) return existing;
      var preferredId = String(this.systemThreadId || 'system').trim() || 'system';
      this.systemTerminalSessionId = preferredId;
      return preferredId;
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
        var sessionId = await this.ensureSystemTerminalSession();
        var ack = await InfringAPI.post('/api/shell-socket/terminal/commands', {
          agent_id: sessionId,
          command: cmd,
          cwd: this.terminalPromptPath
        });
        if (!ack || ack.rejected) throw new Error(String((ack && ack.reason_code) || 'terminal_command_rejected'));
        this.sending = false;
        this._responseStartedAt = 0;
        this.setAgentLiveActivity(this.systemThreadId || 'system', 'idle', { optimistic: true, source: 'shell_socket_terminal_ack' });
      } catch (error) {
        this.sending = false;
        this._responseStartedAt = 0;
        InfringToast.error(error && error.message ? error.message : 'command failed');
      }
    },
