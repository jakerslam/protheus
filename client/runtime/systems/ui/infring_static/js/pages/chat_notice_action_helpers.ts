// Chat notice action and release-update shell helpers.
'use strict';

function infringChatNoticeActionMethods() {
  return {
    addModelSwitchNotice(previousModelName, previousProviderName, modelName, providerName) {
      var legacyCall = arguments.length < 3;
      var previousModel = '';
      var model = '';
      if (legacyCall) {
        model = String(previousModelName || '').trim();
      } else {
        previousModel = String(previousModelName || '').trim();
        model = String(modelName || '').trim();
      }
      if (!model) return;
      if (!previousModel && this.currentAgent) {
        previousModel = String(this.currentAgent.runtime_model || this.currentAgent.model_name || '').trim();
      }
      if (!previousModel) previousModel = 'unknown';
      var label = 'Model switched from ' + previousModel + ' to ' + model;
      this.touchModelUsage(model);
      this.addNoticeEvent({ notice_label: label, notice_type: 'model', ts: Date.now() });
    },

    addAgentRenameNotice(previousName, nextName) {
      var fromName = String(previousName || '').trim();
      var toName = String(nextName || '').trim();
      if (!toName || fromName === toName) return;
      if (!fromName) fromName = 'Unnamed agent';
      this.addNoticeEvent({
        notice_label: 'changed name from ' + fromName + ' to ' + toName,
        notice_type: 'info',
        ts: Date.now()
      });
    },

    formatResponseDuration(ms) {
      var num = Number(ms || 0);
      if (!Number.isFinite(num) || num <= 0) return '';
      if (num < 1000) return Math.round(num) + 'ms';
      if (num < 60000) {
        return (num < 10000 ? (num / 1000).toFixed(1) : Math.round(num / 1000)) + 's';
      }
      var min = Math.floor(num / 60000);
      var sec = Math.round((num % 60000) / 1000);
      return min + 'm ' + sec + 's';
    },
    normalizeNoticeAction(action) {
      if (!action || typeof action !== 'object') return null;
      var kind = String(action.kind || action.type || '').trim().toLowerCase();
      if (!kind) return null;
      var label = String(action.label || '').trim();
      if (kind === 'system_update') {
        return {
          kind: kind,
          label: label || 'Update',
          latest_version: String(action.latest_version || '').trim(),
          current_version: String(action.current_version || '').trim(),
          busy: !!action.busy
        };
      }
      if (kind === 'model_discover') {
        return {
          kind: kind,
          label: label || 'Discover models',
          reason: String(action.reason || '').trim(),
          starter_model: String(action.starter_model || 'qwen2.5:3b-instruct').trim(),
          starter_provider: String(action.starter_provider || 'ollama').trim(),
          busy: !!action.busy
        };
      }
      if (kind === 'open_url') {
        var url = String(action.url || '').trim();
        if (!url) return null;
        return {
          kind: kind,
          label: label || 'Open link',
          url: url,
          busy: !!action.busy
        };
      }
      return null;
    },
    noticeActionVisible(msg) {
      return !!this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
    },
    noticeActionLabel(msg) {
      var action = this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
      return action ? String(action.label || 'Update') : '';
    },
    noticeActionBusy(msg) {
      return !!(msg && msg.notice_action && msg.notice_action.busy === true);
    },
    isTrustedExternalActionUrl(value) {
      var raw = String(value || '').trim();
      if (!raw) return false;
      try {
        var target = new URL(raw, window.location.href);
        var host = String(target.hostname || '').trim().toLowerCase();
        var sameHost = false;
        try {
          var local = new URL(window.location.href);
          sameHost = String(target.host || '').trim().toLowerCase() === String(local.host || '').trim().toLowerCase();
        } catch (_) {}
        if (sameHost) return true;
        return (
          host === 'localhost' ||
          host === '127.0.0.1' ||
          host === '::1' ||
          host === '[::1]' ||
          host.indexOf('127.') === 0
        );
      } catch (_) {
        return false;
      }
    },
    openNoticeActionUrl(url) {
      var target = String(url || '').trim();
      if (!target) return false;
      if (typeof window === 'undefined' || typeof window.open !== 'function') return false;
      if (this.isTrustedExternalActionUrl(target)) {
        window.open(target, '_blank', 'noopener,noreferrer');
        return true;
      }
      InfringToast.confirm(
        'Open External Link',
        'Open this external URL?\n' + target,
        function() {
          try {
            window.open(target, '_blank', 'noopener,noreferrer');
          } catch (_) {}
        }
      );
      return true;
    },
    async triggerNoticeAction(msg) {
      var action = this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
      if (!action) return;
      if (this.systemUpdateBusy || this.noticeActionBusy(msg)) return;
      if (msg && msg.notice_action) msg.notice_action.busy = true;
      this.systemUpdateBusy = true;
      this.scheduleConversationPersist();
      try {
        if (action.kind === 'system_update') {
          var payload = {};
          if (action.latest_version) payload.latest_version = action.latest_version;
          if (action.current_version) payload.current_version = action.current_version;
          var result = await InfringAPI.post('/api/system/update', payload);
          this.addNoticeEvent({
            notice_label: String(result && result.message ? result.message : 'System update started.'),
            notice_type: 'info',
            notice_icon: '\u21bb'
          });
          if (msg) msg.notice_action = null;
        } else if (action.kind === 'model_discover') {
          var models = await this.refreshModelCatalogAndGuidance({ discover: true, guidance: true });
          var available = this.availableModelRowsCount(models);
          if (available > 0) {
            this.addNoticeEvent({
              notice_label: 'Model discovery ready: ' + available + ' runnable model' + (available === 1 ? '' : 's') + ' detected.',
              notice_type: 'info',
              notice_icon: '\u2713'
            });
            if (msg) msg.notice_action = null;
          } else {
            var starterProvider = String(action.starter_provider || 'ollama').trim();
            var starterModel = String(action.starter_model || 'qwen2.5:3b-instruct').trim();
            await InfringAPI.post('/api/models/download', {
              provider: starterProvider,
              model: starterProvider + '/' + starterModel
            }).catch(function() { return null; });
            models = await this.refreshModelCatalogAndGuidance({ discover: true, guidance: true });
            available = this.availableModelRowsCount(models);
            if (available > 0) {
              this.addNoticeEvent({
                notice_label: 'Starter model ready. You can chat now.',
                notice_type: 'info',
                notice_icon: '\u2713'
              });
              if (msg) msg.notice_action = null;
            } else {
              if (msg && msg.notice_action) {
                msg.notice_action = {
                  kind: 'open_url',
                  label: 'Install Ollama',
                  url: 'https://ollama.com/download'
                };
              }
              this.addNoticeEvent({
                notice_label: 'Still no runnable models detected. Install Ollama, then retry discovery.',
                notice_type: 'warn',
                notice_icon: '\u26a0'
              });
            }
          }
        } else if (action.kind === 'open_url') {
          var opened = this.openNoticeActionUrl(action.url);
          if (opened && msg) msg.notice_action = null;
        }
      } catch (e) {
        var reason = e && e.message ? String(e.message) : 'unknown_error';
        if (action.kind === 'system_update') {
          InfringToast.error('Failed to start system update: ' + reason);
        } else if (action.kind === 'model_discover') {
          InfringToast.error('Model recovery failed: ' + reason);
        } else if (action.kind === 'open_url') {
          InfringToast.error('Failed to open link: ' + reason);
        }
        if (msg && msg.notice_action) msg.notice_action.busy = false;
      } finally {
        this.systemUpdateBusy = false;
        this.scheduleConversationPersist();
      }
    },
    async checkForSystemReleaseUpdate(force) {
      if (this._releaseCheckInFlight) return;
      if (!this.currentAgent || !this.currentAgent.id) return;
      this._releaseCheckInFlight = true;
      try {
        var result = await InfringAPI.get('/api/system/release-check' + (force ? '?force=1' : ''));
        if (!result || result.ok === false || !result.update_available) return;
        var latest = String(result.latest_version || '').trim();
        var current = String(result.current_version || '').trim();
        if (!latest) return;
        var noticeKey = latest + '|' + current;
        if (noticeKey && this._releaseUpdateNoticeKey === noticeKey) return;
        var label = 'Update available: ' + latest + (current ? ' (current ' + current + ')' : '');
        var existing = Array.isArray(this.messages) && this.messages.some(function(row) {
          return !!(row && row.is_notice && String(row.notice_label || '').trim() === label);
        });
        if (existing) {
          this._releaseUpdateNoticeKey = noticeKey;
          return;
        }
        this._releaseUpdateNoticeKey = noticeKey;
        this.addNoticeEvent({
          notice_label: label,
          notice_type: 'info',
          notice_icon: '\u21e7',
          notice_action: {
            kind: 'system_update',
            label: 'Update',
            latest_version: latest,
            current_version: current
          }
        });
      } catch (_) {
      } finally {
        this._releaseCheckInFlight = false;
      }
    },
  };
}
