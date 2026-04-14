
    hydrateInputHistoryFromCache: function(explicitMode, explicitAgentId) {
      var mode = this.inputHistoryMode(explicitMode);
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows)) return;
      var agentKey = this.inputHistoryAgentKey(explicitAgentId);
      if (!agentKey) return;
      var legacyKey = this.inputHistoryLegacyAgentKey(explicitAgentId);
      var cache = this._inputHistoryByAgent && typeof this._inputHistoryByAgent === 'object'
        ? this._inputHistoryByAgent
        : {};
      var cachedRows = this.inputHistoryBucketRows(cache, agentKey, legacyKey, mode);
      if (!Array.isArray(cachedRows) || !cachedRows.length) return;
      var merged = this.normalizeInputHistoryRows(rows.concat(cachedRows));
      if (mode === 'terminal') this.terminalInputHistory = merged;
      else this.chatInputHistory = merged;
    },

    syncInputHistoryToCache: function(explicitMode, explicitAgentId) {
      var mode = this.inputHistoryMode(explicitMode);
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows)) return;
      var agentKey = this.inputHistoryAgentKey(explicitAgentId);
      if (!agentKey) return;
      if (!this._inputHistoryByAgent || typeof this._inputHistoryByAgent !== 'object') {
        this._inputHistoryByAgent = {};
      }
      var bucket = this._inputHistoryByAgent[agentKey] && typeof this._inputHistoryByAgent[agentKey] === 'object'
        ? this._inputHistoryByAgent[agentKey]
        : {};
      var cleanRows = this.normalizeInputHistoryRows(rows);
      if (mode === 'terminal') bucket.terminal = cleanRows;
      else bucket.chat = cleanRows;
      bucket.updated_at = Date.now();
      this._inputHistoryByAgent[agentKey] = bucket;
      this.persistInputHistoryCache();
    },

    inputHistoryEntries: function(explicitMode) {
      var mode = this.inputHistoryMode(explicitMode);
      return mode === 'terminal' ? this.terminalInputHistory : this.chatInputHistory;
    },

