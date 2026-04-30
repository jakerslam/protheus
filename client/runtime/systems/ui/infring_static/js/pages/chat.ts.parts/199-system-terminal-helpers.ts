
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
