    },

    normalizeNoticeAction: function(action) {
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

    noticeActionVisible: function(msg) {
      return !!this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
    },

    noticeActionLabel: function(msg) {
      var action = this.normalizeNoticeAction(msg && msg.notice_action ? msg.notice_action : null);
      return action ? String(action.label || 'Update') : '';
    },

    noticeActionBusy: function(msg) {
      return !!(msg && msg.notice_action && msg.notice_action.busy === true);
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
          if (typeof window !== 'undefined' && typeof window.open === 'function') {
            window.open(action.url, '_blank', 'noopener,noreferrer');
          }
          if (msg) msg.notice_action = null;
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

    pushSystemMessage: function(entry) {
      var payload = entry && typeof entry === 'object' ? entry : { text: entry };
      var rawText = String(payload && payload.text ? payload.text : '');
      var text = this.normalizeSystemMessageText
        ? this.normalizeSystemMessageText(rawText)
        : rawText.trim();
      if (!text) return null;
      var canonicalText = text.replace(/\s+/g, ' ').trim().toLowerCase();
      if (/^error:\s*/i.test(canonicalText) && canonicalText.indexOf('operation was aborted') >= 0) return null;

      var origin = String(payload.system_origin || payload.systemOrigin || '').trim();
      var tsRaw = Number(payload.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var dedupeWindowMs = Number(payload.dedupe_window_ms || payload.dedupeWindowMs || 8000);
      if (!Number.isFinite(dedupeWindowMs) || dedupeWindowMs < 0) dedupeWindowMs = 8000;
      if (dedupeWindowMs > 60000) dedupeWindowMs = 60000;
      var canDedupe = payload.dedupe !== false;
      var systemThreadId = String(this.systemThreadId || 'system').trim() || 'system';
      var activeId = String((this.currentAgent && this.currentAgent.id) || '').trim();
      var targetId = activeId || systemThreadId;
      var isGlobalNotice = !!(
        this.isSystemNotificationGlobalToWorkspace &&
        this.isSystemNotificationGlobalToWorkspace(origin, text)
      );
      var routeToSystem =
        payload.route_to_system === true ||
        (payload.route_to_system !== false && isGlobalNotice);
      if (routeToSystem) targetId = systemThreadId;
      var activeThread = !!activeId && activeId === targetId;
      if (!this._systemMessageDedupeIndex || typeof this._systemMessageDedupeIndex !== 'object') this._systemMessageDedupeIndex = {};

      var targetRows = null;
      var targetCache = null;
      if (activeThread) {
        if (!Array.isArray(this.messages)) this.messages = [];
        targetRows = this.messages;
      } else {
        if (!this.conversationCache || typeof this.conversationCache !== 'object') this.conversationCache = {};
        targetCache = this.conversationCache[targetId];
        if (!targetCache || typeof targetCache !== 'object' || !Array.isArray(targetCache.messages)) {
          targetCache = { saved_at: Date.now(), token_count: 0, messages: [] };
          this.conversationCache[targetId] = targetCache;
        }
        targetRows = targetCache.messages;
      }

      if (!Array.isArray(targetRows)) return null;
      var dedupeKey = targetId + '|' + (origin || '_') + '|' + canonicalText;
      if (canDedupe) {
        for (var idx = targetRows.length - 1, scanned = 0; idx >= 0 && scanned < 24; idx -= 1) {
          var row = targetRows[idx];
          if (!row || row.thinking || row.streaming) continue;
          if (String(row.role || '').toLowerCase() !== 'system' || row.is_notice) continue;
          scanned += 1;
          var rowText = String(row.text || '').replace(/\s+/g, ' ').trim().toLowerCase();
          if (rowText !== canonicalText) continue;
          var rowTs = Number(row.ts || 0);
          if (Number.isFinite(rowTs) && Math.abs(ts - rowTs) > dedupeWindowMs) continue;
          var rowOrigin = String(row.system_origin || '').trim();
          if (rowOrigin && origin && rowOrigin !== origin && !/^error:/i.test(canonicalText)) continue;
          var repeatCount = Number(row._repeat_count || 1);
          if (!Number.isFinite(repeatCount) || repeatCount < 1) repeatCount = 1;
          repeatCount += 1;
          row._repeat_count = repeatCount;
          var priorMeta = String(row.meta || '').trim().replace(/\s*\|\s*repeated x\d+\s*$/i, '').trim();
          row.meta = (priorMeta ? (priorMeta + ' | ') : '') + 'repeated x' + repeatCount;
          row.ts = ts;
          this._systemMessageDedupeIndex[dedupeKey] = { id: row.id, ts: ts };
          if (activeThread) this.scheduleConversationPersist();
          else this.persistConversationCache();
          return row;
        }
      }

      var message = {
        id: ++msgId,
        role: 'system',
        text: text,
        meta: String(payload.meta || ''),
        tools: Array.isArray(payload.tools) ? payload.tools : [],
        system_origin: origin,
        ts: ts
      };
      targetRows.push(message);
      if (canDedupe && canonicalText) this._systemMessageDedupeIndex[dedupeKey] = { id: message.id, ts: ts };
      var store = Alpine.store('app');
      if (store && typeof store.saveAgentChatPreview === 'function') {
        store.saveAgentChatPreview(targetId, targetRows);
      }
      if (activeThread) {
        if (payload.auto_scroll !== false) this.scrollToBottom();
        this.scheduleConversationPersist();
      } else {
        if (targetCache) {
          targetCache.saved_at = Date.now();
          targetCache.token_count = 0;
        }
        this.persistConversationCache();
      }
      return message;
    },
    addModelSwitchNotice: function(previousModelName, previousProviderName, modelName, providerName) {
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

    addAgentRenameNotice: function(previousName, nextName) {
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

    formatResponseDuration: function(ms) {
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
    normalizeResponseToolCard: function(tool, idx, prefix) {
      var row = tool && typeof tool === 'object' ? tool : {};
      return {
        id: (row.id || ((prefix || 'tool') + '-' + Date.now() + '-' + idx)),
        name: row.name || row.tool || 'tool',
        running: false,
        expanded: false,
        input: row.input || row.arguments || row.args || '',
        result: row.result || row.output || row.summary || '',
        is_error: !!(row.is_error || row.error || row.blocked),
        blocked: row.blocked === true || String(row.status || '').toLowerCase() === 'blocked',
        status: String(row.status || '').trim().toLowerCase(),
        tool_attempt_receipt: row.tool_attempt_receipt || null
      };
    },

    toolCardFromAttemptReceipt: function(rawAttempt, idx, prefix) {
      var envelope = rawAttempt && typeof rawAttempt === 'object' ? rawAttempt : {};
      var attempt = envelope.attempt && typeof envelope.attempt === 'object' ? envelope.attempt : envelope;
      var toolName = String(attempt.tool_name || attempt.tool || 'tool').trim() || 'tool';
      var rawStatus = String(attempt.status || attempt.outcome || '').trim().toLowerCase();
      var blocked = rawStatus === 'blocked' || rawStatus === 'policy_denied';
      var isError = !blocked && !!rawStatus && rawStatus !== 'ok';
      var normalizedArgs = envelope.normalized_result && envelope.normalized_result.normalized_args
        ? envelope.normalized_result.normalized_args
        : null;
      var input = '';
      try {
        if (normalizedArgs && typeof normalizedArgs === 'object') input = JSON.stringify(normalizedArgs);
      } catch (_) {}
      var reason = String(envelope.error || attempt.reason || rawStatus || '').trim();
      var backend = String(attempt.backend || '').trim().replace(/_/g, ' ');
      var result = reason;
      if (!result && backend) result = 'Attempted via ' + backend;
      if (!result && rawStatus === 'ok') result = 'Attempt succeeded';
      if (!result) result = 'Attempt recorded';
      return {
        id: ((prefix || 'attempt') + '-' + Date.now() + '-' + idx),
        name: toolName,
        running: false,
        expanded: false,
        input: input,
        result: result,
        is_error: isError,
        blocked: blocked,
        status: blocked ? 'blocked' : (rawStatus || (isError ? 'error' : 'ok')),
        reason_code: String(attempt.reason_code || '').trim(),
        backend: String(attempt.backend || '').trim(),
        tool_attempt_receipt: attempt
      };
    },

    responseToolRowsFromPayload: function(payload, prefix) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var base = Array.isArray(data.tools)
        ? data.tools.map(function(row, idx) { return this.normalizeResponseToolCard(row, idx, prefix || 'tool'); }, this)
        : [];
      var completion =
        data.response_finalization &&
        data.response_finalization.tool_completion &&
        typeof data.response_finalization.tool_completion === 'object'
          ? data.response_finalization.tool_completion
          : null;
      var attempts = Array.isArray(completion && completion.tool_attempts)
        ? completion.tool_attempts
        : [];
      if (!attempts.length) return base;
      var merged = base.slice();
      for (var i = 0; i < attempts.length; i++) {
        var attemptCard = this.toolCardFromAttemptReceipt(attempts[i], i, prefix || 'attempt');
        var matched = false;
        for (var j = 0; j < merged.length; j++) {
          var current = merged[j];
          if (!current || String(current.name || '').toLowerCase() !== String(attemptCard.name || '').toLowerCase()) continue;
          if (!current.input && attemptCard.input) current.input = attemptCard.input;
          if ((!current.result || !String(current.result).trim()) && attemptCard.result) current.result = attemptCard.result;
          if (attemptCard.blocked) current.blocked = true;
          if (attemptCard.status) current.status = attemptCard.status;
          if (attemptCard.is_error) current.is_error = true;
          if (!current.tool_attempt_receipt && attemptCard.tool_attempt_receipt) current.tool_attempt_receipt = attemptCard.tool_attempt_receipt;
          matched = true;
          break;
        }
        if (!matched) merged.push(attemptCard);
      }
      return merged.slice(0, 16);
    },
    stepMessageMap: function(list, dir) {
      if (!Array.isArray(list) || !list.length) return;
      this.suppressMapPreview = true;
      this.activeMapPreviewDomId = '';
      this.activeMapPreviewDayKey = '';
      if (this._mapPreviewSuppressTimer) clearTimeout(this._mapPreviewSuppressTimer);
      var visibleIndexes = [];
      for (var i = 0; i < list.length; i++) {
        if (!this.isMessageDayCollapsed(list[i])) visibleIndexes.push(i);
