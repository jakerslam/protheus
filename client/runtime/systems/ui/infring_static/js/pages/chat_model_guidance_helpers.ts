'use strict';

function infringChatModelGuidanceMethods() {
  return {
    noModelsGuidanceText: function() {
      return '';
    },

    injectNoModelsGuidance: function(reason) {
      return this.addNoModelsRecoveryNotice(reason || 'chat_send_gate', 'model_discover');
    },

    addNoModelsRecoveryNotice: function(reason, actionKind) {
      if (!this.currentAgent || (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent))) {
        return null;
      }
      if (typeof this.addNoticeEvent !== 'function') return null;
      if (!this._noModelsRecoveryNoticeByAgent || typeof this._noModelsRecoveryNoticeByAgent !== 'object') {
        this._noModelsRecoveryNoticeByAgent = {};
      }
      var agentId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      if (!agentId) return null;
      var now = Date.now();
      var prev = this._noModelsRecoveryNoticeByAgent[agentId];
      if (prev && Number(prev.ts || 0) > 0 && (now - Number(prev.ts || 0)) < 20000) {
        return null;
      }
      var desiredKind = String(actionKind || '').trim().toLowerCase();
      if (!desiredKind) desiredKind = 'model_discover';
      var action = null;
      if (desiredKind === 'open_url') {
        action = {
          kind: 'open_url',
          label: 'Install Ollama',
          url: 'https://ollama.com/download'
        };
      } else {
        action = {
          kind: 'model_discover',
          label: 'Discover models',
          reason: String(reason || 'chat_send_gate').trim()
        };
      }
      this.addNoticeEvent({
        notice_label: desiredKind === 'open_url'
          ? 'No runnable models detected. Install Ollama, then run model discovery.'
          : 'No runnable models detected. Discover models to unlock chat.',
        notice_type: 'warn',
        notice_icon: '\u26a0',
        notice_action: action,
        ts: now
      });
      this._noModelsRecoveryNoticeByAgent[agentId] = {
        ts: now,
        reason: String(reason || ''),
        action_kind: desiredKind
      };
      return true;
    },

    currentAvailableModelCount: function() {
      var rows = [];
      if (Array.isArray(this.modelPickerList) && this.modelPickerList.length) {
        rows = this.modelPickerList;
      } else if (Array.isArray(this._modelCache) && this._modelCache.length) {
        rows = this._modelCache;
      } else {
        rows = [];
      }
      rows = this.sanitizeModelCatalogRows(rows);
      return this.countAvailableModelRows(rows);
    },

    ensureUsableModelsForChatSend: async function(reason) {
      var available = this.currentAvailableModelCount();
      if (available > 0) return available;
      try {
        var models = await this.refreshModelCatalogAndGuidance({ discover: true, guidance: true });
        available = this.countAvailableModelRows(models);
      } catch (_) {
        available = this.currentAvailableModelCount();
      }
      if (available <= 0) {
        this.injectNoModelsGuidance(reason || 'chat_send_gate');
        this.addNoModelsRecoveryNotice(reason || 'chat_send_gate', 'model_discover');
      }
      return available;
    },

    refreshModelCatalogAndGuidance: async function(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var discoverFirst = opts.discover !== false;
      var includeGuidance = opts.guidance !== false;
      try {
        var _ = discoverFirst;
        var data = await InfringAPI.get('/api/models');
        var models = this.sanitizeModelCatalogRows((data && data.models) || []);
        var available = this.countAvailableModelRows(models);
        this._modelCache = models;
        this._modelCacheTime = Date.now();
        this.modelPickerList = models;
        if (includeGuidance && available === 0) {
          this.injectNoModelsGuidance('refresh');
        }
        return models;
      } catch (err) {
        if (includeGuidance && (!this.modelPickerList || !this.modelPickerList.length)) {
          this.injectNoModelsGuidance('refresh_error');
        }
        throw err;
      }
    },
  };
}
