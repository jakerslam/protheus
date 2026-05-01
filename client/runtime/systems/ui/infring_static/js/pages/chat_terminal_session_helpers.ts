// Chat terminal session and send helpers.
'use strict';

function infringChatTerminalSessionMethods() {
  return {
    async ensureSystemTerminalSession() {
      var existing = String(this.systemTerminalSessionId || '').trim();
      if (existing) return existing;
      var preferredId = String(this.systemThreadId || 'system').trim() || 'system';
      this.systemTerminalSessionId = preferredId;
      return preferredId;
    },

    async _sendSystemTerminalPayload(command) {
      await this._sendTerminalPayload(command, this.systemThreadId || 'system');
    },

    async sendTerminalMessage() {
      if (this.showFreshArchetypeTiles) {
        InfringToast.info('Launch agent initialization before running terminal commands.');
        return;
      }
      var activeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!activeAgent || !this.inputText.trim()) return;
      if (!this.isSystemThreadAgent(activeAgent) && this.isArchivedAgentRecord && this.isArchivedAgentRecord(activeAgent)) {
        InfringToast.info('This agent is archived. Revive it to run commands.');
        return;
      }
      this.showFreshArchetypeTiles = false;
      var command = this.inputText.trim();
      this.pushInputHistoryEntry('terminal', command);
      this.inputText = '';
      this.terminalSelectionStart = 0;

      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';

      if (this.sending) {
        this._reconcileSendingState();
      }
      if (this.sending) {
        this.messageQueue.push({
          queue_id: this.nextPromptQueueId(),
          queue_kind: 'terminal',
          queued_at: Date.now(),
          terminal: true,
          command: command
        });
        return;
      }

      this._sendTerminalPayload(command, activeAgent.id);
    },

    async _sendTerminalPayload(command, agentIdOverride) {
      var targetAgentId = String(agentIdOverride || (this.currentAgent && this.currentAgent.id) || '').trim();
      if (!targetAgentId) return;
      var cmd = String(command || '').trim();
      if (!cmd) return;
      this.terminalMode = false;
      this.inputText = 'Use the terminal tool route for agent ' + targetAgentId + ' in cwd ' + this.terminalPromptPath + ' with command: ' + cmd;
      await this.sendMessage();
    },
  };
}
