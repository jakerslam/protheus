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
        var providerName = String((discovery && discovery.provider) || '').trim();
        var discoveredCount = Number((discovery && discovery.model_count) || 0);
        var refreshed = await InfringAPI.get('/api/models');
        var catalogRows = this.sanitizeModelCatalogRows((refreshed && refreshed.models) || []);
        this._modelCache = catalogRows;
        this._modelCacheTime = Date.now();
        this.modelPickerList = catalogRows;
        if (this.availableModelRowsCount(catalogRows) === 0) {
          this.injectNoModelsGuidance('apikey_discover');
        }
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text:
            'Model discovery updated' +
            (providerName ? (' for `' + providerName + '`') : '') +
            '. Added/updated ' + discoveredCount + ' model entries.',
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
