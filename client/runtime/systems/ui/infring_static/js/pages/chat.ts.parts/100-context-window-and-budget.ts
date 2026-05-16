        candidates.push(modelId);
        if (modelId.indexOf('/') >= 0) {
          candidates.push(modelId.split('/').slice(-1)[0]);
        }
      }
      var bestFromMap = 0;
      var bestInferred = 0;
      var needsFloor = false;
      for (var i = 0; i < candidates.length; i++) {
        var candidate = String(candidates[i] || '').trim();
        if (!candidate) continue;
        if (typeof this.contextWindowNeedsFloor === 'function' && this.contextWindowNeedsFloor(candidate)) {
          needsFloor = true;
        }
        var fromMap = Number(map[candidate] || 0);
        if (Number.isFinite(fromMap) && fromMap > bestFromMap) {
          bestFromMap = Math.round(fromMap);
        }
        var inferred = this.inferContextWindowFromModelId(
          candidate.indexOf('/') >= 0 ? candidate.split('/').slice(-1)[0] : candidate
        );
        if (Number.isFinite(inferred) && inferred > bestInferred) {
          bestInferred = Math.round(inferred);
        }
      }
      if (needsFloor && bestInferred > 0) {
        return Math.max(bestFromMap, bestInferred);
      }
      if (bestFromMap > 0) return bestFromMap;
      if (bestInferred > 0) return bestInferred;
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
      return InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(id) + '/compact-session', {
        target_context_window: targetWindow,
        target_ratio: targetRatio,
        min_recent_messages: 12,
        max_messages: 200
      }).then(function(resp) {
        var beforeTokens = usedTokens;
        var afterTokens = Math.min(usedTokens, targetTokens);
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
        return {
          compacted: !!(resp && resp.accepted !== false),
          receipt_ref: resp && resp.receipt_ref,
          before_tokens: beforeTokens,
          after_tokens: afterTokens
        };
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
          return InfringAPI.post('/api/shell-socket/agents/' + encodeURIComponent(agentId) + '/model', {
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

    ensureFailoverModelCache: function() {
      var now = Date.now();
      if (this._modelCache && (now - Number(this._modelCacheTime || 0)) < 180000) {
        return Promise.resolve(this._modelCache);
      }
      var self = this;
      return InfringAPI.get('/api/shell-socket/models')
        .then(function(data) {
          var models = self.sanitizeModelCatalogRows(Array.isArray(data && data.models) ? data.models : []);
          var available = models.filter(function(m) { return !!(m && m.available); });
          self._modelCache = models;
          self._modelCacheTime = Date.now();
          self.modelPickerList = models;
          return available;
        })
        .catch(function() {
          return Array.isArray(self._modelCache) ? self._modelCache : [];
        });
    },

    normalizeFailoverCandidateId: function(entry) {
      if (!entry) return '';
      if (typeof entry === 'string') return String(entry || '').trim();
      if (typeof entry !== 'object') return '';
      var model = String(entry.id || entry.model || entry.model_name || '').trim();
      var provider = String(entry.provider || entry.model_provider || '').trim();
      if (!model) return '';
      if (provider && model.indexOf('/') < 0) return provider + '/' + model;
      return model;
    },

    collectModelIdVariants: function(values) {
      var set = {};
      var add = function(value) {
        var raw = String(value || '').trim();
        if (!raw) return;
        var lower = raw.toLowerCase();
        set[lower] = true;
        if (raw.indexOf('/') >= 0) {
          var tail = String(raw.split('/').slice(-1)[0] || '').toLowerCase();
          if (tail) set[tail] = true;
        }
      };
      if (Array.isArray(values)) {
        for (var i = 0; i < values.length; i++) add(values[i]);
      } else {
        add(values);
      }
      return set;
    },

    // Backward-compat shim for legacy callers during naming migration.
    modelIdVariantSet: function(values) {
      return this.collectModelIdVariants(values);
    },

    extractRecoverableBackendFailure: function(text) {
      var raw = String(text || '').trim();
      if (!raw) return null;
      var lower = raw.toLowerCase();
      if (
        lower === 'i lost the final response handoff for this turn. context is still intact, and i can continue from exactly where this left off.' ||
        lower.indexOf('completed tool steps:') === 0
      ) {
        return null;
      }
      var markers = [
        "couldn't reach a chat model backend",
        'could not reach a chat model backend',
        'hosted_model_provider_sync_failed',
        'provider-sync',
        'switch-provider',
        'lane_timeout_1500ms',
        'start ollama',
        'configure app-plane',
        'model backend unavailable',
        'no chat model backend',
        'app_plane_chat_ui',
        'did not receive a final answer',
        'lost the final response handoff'
      ];
      var matched = false;
      for (var i = 0; i < markers.length; i++) {
        if (lower.indexOf(markers[i]) >= 0) {
          matched = true;
          break;
        }
      }
      if (!matched) return null;
      var summary = raw.replace(/\s+/g, ' ').trim();
      if (summary.length > 220) summary = summary.slice(0, 217) + '...';
      return { raw: raw, summary: summary };
    },

    attemptAutomaticFailoverRecovery: async function(source, rawFailure, options) {
      var payload = this._inflightPayload;
      if (payload && typeof payload === 'object') {
        payload.failover_skipped = true;
        payload.failover_skip_reason = 'automatic_model_failover_disabled';
        payload.failover_source = String(source || 'runtime');
      }
      void rawFailure;
      void options;
      return false;
    },
