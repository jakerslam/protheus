function infringChatModelSwitchMethods() {
  return {
    pickModel(modelId) {
      this.showModelPicker = false;
      this.inputText = '/model ' + modelId;
      this.sendMessage();
    },

    loadModelCatalogSafely: function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var preferCached = opts.prefer_cached !== false;
      var suppressErrors = opts.suppress_errors === true;
      var self = this;
      return InfringAPI.get('/api/models').then(function(data) {
        var models = self.sanitizeModelCatalogRows((data && data.models) || []);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
        return models;
      }).catch(function(error) {
        var fallback = preferCached ? self.sanitizeModelCatalogRows(self._modelCache || []) : [];
        if (fallback.length) {
          self._modelCache = fallback;
          self.modelPickerList = fallback;
          return fallback;
        }
        if (suppressErrors) return [];
        throw error;
      });
    },

    describeModelDiscoveryResult: function(resp, catalogRows) {
      var provider = String((resp && resp.provider) || '').trim();
      var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
      var discoveredCount = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
      if (!Number.isFinite(discoveredCount) || discoveredCount < 0) discoveredCount = 0;
      var availableRows = Array.isArray(catalogRows) ? catalogRows : [];
      var availableCount = this.availableModelRowsCount ? this.availableModelRowsCount(availableRows) : availableRows.length;
      var prefix = '';
      if (inputKind === 'local_path') {
        prefix = provider
          ? ('Indexed local path for `' + provider + '`')
          : 'Indexed local path';
      } else {
        prefix = provider
          ? ('Added provider `' + provider + '`')
          : 'Saved model discovery input';
      }
      prefix += ' (' + discoveredCount + ' discovered';
      if (availableCount > 0) {
        prefix += ', ' + availableCount + ' available now';
      }
      prefix += ').';
      return prefix;
    },

    toggleModelSwitcher() {
      if (this.showModelSwitcher) { this.showModelSwitcher = false; return; }
      var self = this;
      var now = Date.now();
      if (typeof this.closeComposerMenus === 'function') this.closeComposerMenus({ model: true });
      else {
        this.showAttachMenu = false;
        this.closeGitTreeMenu();
      }
      this.modelApiKeyStatus = '';
      var cached = self.sanitizeModelCatalogRows(self._modelCache || []);
      if (cached.length) {
        self._modelCache = cached;
        self.modelPickerList = cached;
      }
      this.modelSwitcherFilter = '';
      this.modelSwitcherProviderFilter = '';
      this.modelSwitcherIdx = 0;
      this.showModelSwitcher = true;
      this.$nextTick(function() {
        var el = document.getElementById('model-switcher-search');
        if (el) el.focus();
      });

      var cacheFresh = Array.isArray(this._modelCache) && (now - this._modelCacheTime) < 300000;
      var cachedAvailable = self.availableModelRowsCount ? self.availableModelRowsCount(cached) : 0;
      var shouldRefresh = !cacheFresh || cached.length < 8 || cachedAvailable < 4;
      if (!shouldRefresh) return;
      self.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function(e) {
        return self.loadModelCatalogSafely({
          prefer_cached: true,
          suppress_errors: true
        }).then(function(models) {
          if (!models.length && (!self.modelPickerList || !self.modelPickerList.length)) {
            var active = self.resolveActiveSwitcherModel([]);
            self.modelPickerList = active ? [active] : [];
          }
          self.modelApiKeyStatus = models.length
            ? 'Unable to refresh model list (showing cached entries)'
            : 'Unable to refresh model list right now';
          InfringToast.error('Failed to refresh models: ' + e.message);
        });
      });
    },

    discoverModelsFromApiKey: function() {
      var self = this;
      var entry = String(this.modelApiKeyInput || '').trim();
      if (!entry) {
        InfringToast.error('Enter an API key or local model path first');
        return;
      }
      this.modelApiKeySaving = true;
      this.modelApiKeyStatus = 'Detecting...';
      InfringAPI.post('/api/models/discover', {
        input: entry,
        api_key: entry
      }).then(function(resp) {
        var provider = String((resp && resp.provider) || '').trim();
        var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
        var count = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
        self.modelApiKeyInput = '';
        if (inputKind === 'local_path') {
          self.modelApiKeyStatus = provider
            ? ('Indexed local path to ' + provider + ' (' + count + ' models)')
            : ('Indexed local path (' + count + ' models)');
        } else {
          self.modelApiKeyStatus = provider ? ('Added ' + provider + ' (' + count + ' models)') : 'API key saved';
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return self.loadModelCatalogSafely({
          prefer_cached: false,
          suppress_errors: false
        }).then(function(models) {
          self.modelApiKeyStatus = self.describeModelDiscoveryResult(resp, models);
          return models;
        });
      }).then(function(models) {
        if (self.availableModelRowsCount(models) === 0) {
          self.injectNoModelsGuidance('discover_key');
        }
      }).catch(function(e) {
        self.modelApiKeyStatus = '';
        InfringToast.error('Model discovery failed: ' + (e && e.message ? e.message : e));
      }).finally(function() {
        self.modelApiKeySaving = false;
      });
    },

    resolveModelContextWindowForSwitch: function(targetModelRef) {
      var modelId = '';
      var explicitWindow = 0;
      if (targetModelRef && typeof targetModelRef === 'object') {
        modelId = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
        explicitWindow = Number(
          targetModelRef.context_window || targetModelRef.context_window_tokens || 0
        );
      } else {
        modelId = String(targetModelRef || '').trim();
      }
      if (Number.isFinite(explicitWindow) && explicitWindow > 0) {
        return Math.round(explicitWindow);
      }
      var map = this._contextWindowByModel || {};
      var candidates = [];
      if (modelId) {

        candidates.push(modelId);
        if (modelId.indexOf('/') >= 0) {
          candidates.push(modelId.split('/').slice(-1)[0]);
        }
      }
      var bestFromMap = 0;
      for (var i = 0; i < candidates.length; i++) {
        var candidate = String(candidates[i] || '').trim();
        if (!candidate) continue;
        var fromMap = Number(map[candidate] || 0);
        if (Number.isFinite(fromMap) && fromMap > bestFromMap) {
          bestFromMap = Math.round(fromMap);
        }
      }
      if (bestFromMap > 0) return bestFromMap;
      return 0;
    },

    ensureContextBudgetForModelSwitch: function(agentId, targetModelRef, options) {
      var self = this;
      var opts = options && typeof options === 'object' ? options : {};
      var id = String(agentId || '').trim();
      if (!id) return Promise.resolve({ compacted: false });
      var targetWindow = self.resolveModelContextWindowForSwitch(targetModelRef);
      var usedTokens = Number(self.contextApproxTokens || 0);
      if (
        !Number.isFinite(targetWindow) ||
        targetWindow <= 0 ||
        !Number.isFinite(usedTokens) ||
        usedTokens <= targetWindow
      ) {
        return Promise.resolve({
          compacted: false,
          target_context_window: targetWindow,
          before_tokens: Math.max(0, Math.round(usedTokens || 0)),
          after_tokens: Math.max(0, Math.round(usedTokens || 0))
        });
      }
      var targetRatio = Number(opts.target_ratio);
      if (!Number.isFinite(targetRatio) || targetRatio <= 0 || targetRatio >= 1) {
        targetRatio = 0.8;
      }
      var targetTokens = Math.max(1, Math.floor(targetWindow * targetRatio));
      InfringToast.info('Switching to a model with smaller context may degrade performance.');
      return InfringAPI.post('/api/agents/' + encodeURIComponent(id) + '/session/compact', {
        target_context_window: targetWindow,
        target_ratio: targetRatio,
        min_recent_messages: 12,
        max_messages: 200
      }).then(function(resp) {
        var beforeTokens = Number(
          resp && resp.before_tokens != null ? resp.before_tokens : usedTokens
        );
        var afterTokens = Number(
          resp && resp.after_tokens != null ? resp.after_tokens : Math.min(usedTokens, targetTokens)
        );
        if (Number.isFinite(afterTokens) && afterTokens >= 0) {
          self.contextApproxTokens = Math.max(0, Math.round(afterTokens));
        }
        if (Number.isFinite(targetWindow) && targetWindow > 0) {
          self.contextWindow = Math.round(targetWindow);
        }
        self.refreshContextPressure();
        self.addNoticeEvent({
          notice_label:
            'Context compacted from ' +
            self.formatTokenK(beforeTokens) +
            ' to ' +
            self.formatTokenK(afterTokens) +
            ' tokens (target ' +
            self.formatTokenK(targetTokens) +
            ')',
          notice_type: 'info',
          ts: Date.now()
        });
        return resp || {};
      });
    },

    switchAgentModelWithGuards: function(targetModelRef, options) {
      var self = this;
      var opts = options && typeof options === 'object' ? options : {};
      var reboundAgent = self.ensureValidCurrentAgent({ clear_when_missing: true });
      var agentId = String(opts.agent_id || (reboundAgent && reboundAgent.id) || '').trim();
      if (!agentId) return Promise.reject(new Error('No agent selected'));
      var requestedModel = '';
      if (targetModelRef && typeof targetModelRef === 'object') {
        requestedModel = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
      } else {
        requestedModel = String(targetModelRef || '').trim();
      }
      if (!requestedModel) return Promise.reject(new Error('Model is required'));
      var previousModel = String(
        opts.previous_model != null
          ? opts.previous_model
          : ((self.currentAgent && (self.currentAgent.runtime_model || self.currentAgent.model_name)) || '')
      ).trim();
      var previousProvider = String(
        opts.previous_provider != null
          ? opts.previous_provider
          : ((self.currentAgent && self.currentAgent.model_provider) || '')
      ).trim();
      return self
        .ensureContextBudgetForModelSwitch(agentId, targetModelRef, opts)
        .catch(function(error) {
          InfringToast.error(
            'Context compaction failed before model switch: ' +
              (error && error.message ? error.message : error)
          );
          return null;
        })
        .then(function() {
          return InfringAPI.put('/api/agents/' + encodeURIComponent(agentId) + '/model', {
            model: requestedModel
          });
        })
        .catch(function(error) {
          var message = String(error && error.message ? error.message : error || '');
          var lower = message.toLowerCase();
          var allowRetry = !opts._rebind_retry && (lower.indexOf('agent_not_found') >= 0 || lower.indexOf('agent not found') >= 0);
          if (!allowRetry) throw error;
          return self.rebindCurrentAgentAuthoritative({
            preferred_id: agentId,
            clear_when_missing: true
          }).then(function(rebound) {
            var reboundId = String(rebound && rebound.id ? rebound.id : '').trim();
            if (!reboundId || reboundId === agentId) throw error;
            self.addNoticeEvent({
              notice_label: 'Active agent reference expired. Rebound to ' + String(rebound.name || rebound.id || reboundId),
              notice_type: 'warn',
              ts: Date.now()
            });
            var retryOptions = {};
            var keys = Object.keys(opts);
            for (var k = 0; k < keys.length; k++) retryOptions[keys[k]] = opts[keys[k]];
            retryOptions.agent_id = reboundId;
            retryOptions._rebind_retry = true;
            return self.switchAgentModelWithGuards(targetModelRef, retryOptions);
          });
        })
        .then(function(resp) {
          if (self.currentAgent && String(self.currentAgent.id || '') === agentId) {
            self.currentAgent.model_name = (resp && resp.model) || requestedModel;
            self.currentAgent.model_provider =
              (resp && resp.provider) || self.currentAgent.model_provider || '';
            self.currentAgent.runtime_model =
              (resp && resp.runtime_model) ||
              self.currentAgent.runtime_model ||
              self.currentAgent.model_name;
            var resolvedContextWindow = Number(
              resp && resp.context_window != null ? resp.context_window : 0
            );
            if (Number.isFinite(resolvedContextWindow) && resolvedContextWindow > 0) {
              self.currentAgent.context_window = Math.round(resolvedContextWindow);
              self.contextWindow = Math.round(resolvedContextWindow);
              self.refreshContextPressure();
            }
            self.recordModelUsageTimestamp(requestedModel || '');
            self.recordModelUsageTimestamp(self.currentAgent.model_name || '');
            self.recordModelUsageTimestamp(self.currentAgent.runtime_model || '');
            if (self.currentAgent.model_provider && self.currentAgent.model_name) {
              self.recordModelUsageTimestamp(
                self.currentAgent.model_provider + '/' + self.currentAgent.model_name
              );
            }
            if (self.currentAgent.model_provider && self.currentAgent.runtime_model) {
              self.recordModelUsageTimestamp(
                self.currentAgent.model_provider + '/' + self.currentAgent.runtime_model
              );
            }
            self.addModelSwitchNotice(
              previousModel,
              previousProvider,
              self.currentAgent.model_name,
              self.currentAgent.model_provider
            );
          }
          return resp || {};
        });
    },

    switchModel(model) {
      var activeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!activeAgent) return;
      if (model && model.available === false) {
        InfringToast.error('This model is not ready yet. Configure its provider/API key first.');
        return;
      }
      if (model.id === this.currentAgent.model_name) {
        this.recordModelUsageTimestamp(model.id || '');
        this.showModelSwitcher = false;
        return;
      }
      var self = this;
      this.modelSwitching = true;
      self.switchAgentModelWithGuards(model, {
        agent_id: activeAgent.id
      }).then(function() {
        InfringToast.success('Switched to ' + (model.display_name || model.id));
        self.showModelSwitcher = false;
      }).catch(function(e) {
        InfringToast.error('Switch failed: ' + e.message);
      }).finally(function() {
        self.modelSwitching = false;
      });
    },

  };
}
