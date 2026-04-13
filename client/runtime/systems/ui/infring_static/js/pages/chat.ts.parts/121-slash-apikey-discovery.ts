    runSlashApiKeyDiscovery: async function(cmdArgs) {
      if (!cmdArgs || !String(cmdArgs).trim()) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: 'Usage: `/apikey <api-key-or-local-model-path>`',
          meta: '',
          tools: [],
          system_origin: 'slash:apikey'
        });
        this.scrollToBottom();
        return;
      }
      try {
        var discoveryInput = String(cmdArgs || '').trim();
        var discovery = await InfringAPI.post('/api/models/discover', {
          input: discoveryInput,
          api_key: discoveryInput
        });
        var catalogRows = typeof this.loadModelCatalogSafely === 'function'
          ? await this.loadModelCatalogSafely({
            prefer_cached: true,
            suppress_errors: true
          })
          : this.sanitizeModelCatalogRows(this._modelCache || []);
        if (this.availableModelRowsCount(catalogRows) === 0) {
          this.injectNoModelsGuidance('apikey_discover');
        }
        var statusLine = typeof this.describeModelDiscoveryResult === 'function'
          ? this.describeModelDiscoveryResult(discovery, catalogRows)
          : 'Model discovery updated.';
        var providerName = String((discovery && discovery.provider) || '').trim();
        var inputKind = String((discovery && discovery.input_kind) || '').trim().toLowerCase();
        var guidanceLine = inputKind === 'local_path'
          ? 'Local model path indexed and ready for `/model`.'
          : (providerName
            ? ('Provider `' + providerName + '` is now available in the model switcher.')
            : 'Refresh the model switcher to use the new entries.');
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: statusLine + '\n' + guidanceLine,
          meta: '',
          tools: [],
          system_origin: 'slash:apikey'
        });
        this.scrollToBottom();
        this.scheduleConversationPersist();
      } catch (eApikey) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: 'API key/model path discovery failed: ' + (eApikey && eApikey.message ? eApikey.message : eApikey),
          meta: '',
          tools: [],
          system_origin: 'slash:apikey'
        });
        this.scrollToBottom();
      }
    },
    exportCurrentChatMarkdown: function() {
      var assistantName = String(
        (this.currentAgent && (this.currentAgent.name || this.currentAgent.id)) || 'infring'
      ).trim() || 'infring';
      return exportChatMarkdown(this.messages, assistantName);
    },
