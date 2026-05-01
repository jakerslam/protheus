function infringChatModelFailoverMethods() {
  return {
    ensureFailoverModelCache: function() {
      var now = Date.now();
      if (this._modelCache && (now - Number(this._modelCacheTime || 0)) < 180000) {
        return Promise.resolve(this._modelCache);
      }
      var self = this;
      return InfringAPI.get('/api/models')
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

    stageModelRecoveryCoordinationRequest: function(source, failure) {
      var previousModel = String(
        (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || 'unknown'
      ).trim() || 'unknown';
      var previousProvider = String(
        (this.currentAgent && this.currentAgent.model_provider) || ''
      ).trim() || 'unknown';
      var recoveryRequest = [
        'Use the model_provider_coordination route to recover from a model backend failure.',
        'Do not switch models or retry automatically from the shell.',
        'Source: ' + String(source || 'runtime'),
        'Previous model: ' + previousModel,
        'Previous provider: ' + previousProvider,
        'Failure: ' + String(failure && failure.summary ? failure.summary : 'unknown')
      ].join('\n');
      if (!String(this.inputText || '').trim()) {
        this.inputText = recoveryRequest;
      }
      this.addNoticeEvent({
        notice_label: 'Model backend failed. A recovery request was staged in the composer for deliberate submission.',
        notice_type: 'warn',
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    attemptAutomaticFailoverRecovery: async function(source, rawFailure, options) {
      var failure = this.extractRecoverableBackendFailure(rawFailure);
      if (!failure) return false;
      if (this._inflightFailoverInProgress) return false;
      if (!this.currentAgent || !this.currentAgent.id) return false;
      var agentId = String(this.currentAgent.id || '').trim();
      if (!agentId) return false;
      var payload = this._inflightPayload;
      if (!payload || String(payload.agent_id || '') !== agentId) return false;
      if (payload.failover_attempted) return false;

      var opts = options && typeof options === 'object' ? options : {};
      this._inflightFailoverInProgress = true;
      payload.failover_attempted = true;
      payload.failover_reason = failure.summary;
      payload.failover_source = String(source || 'runtime');

      try {
        if (opts.remove_last_agent_failure) {
          var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (last && last.role === 'agent') {
            var lastText = String(last.text || '').trim();
            if (this.extractRecoverableBackendFailure(lastText)) {
              this.messages.pop();
            }
          }
        }

        this.stageModelRecoveryCoordinationRequest(source, failure);

        this.sending = false;
        this._responseStartedAt = 0;

        this.tokenCount = 0;
        this._clearTypingTimeout();
        this._clearPendingWsRequest(agentId);
        this.setAgentLiveActivity(agentId, 'idle');
        return true;
      } catch (error) {
        var modelRecoveryError = String(error && error.message ? error.message : error);
        console.warn('[model recovery staging failed]', modelRecoveryError);
        InfringToast.error('Model recovery staging failed. See console for details.');
        return false;
      } finally {
        this._inflightFailoverInProgress = false;
      }
    },

  };
}
