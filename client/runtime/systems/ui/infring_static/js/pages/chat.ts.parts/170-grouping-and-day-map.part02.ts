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
    stepMessageMap: function(list, dir) {
      if (!Array.isArray(list) || !list.length) return;
      this.suppressMapPreview = true;
      this.activeMapPreviewDomId = '';
      this.activeMapPreviewDayKey = '';
      if (this._mapPreviewSuppressTimer) clearTimeout(this._mapPreviewSuppressTimer);
      var visibleIndexes = [];
      for (var i = 0; i < list.length; i++) {
        if (!this.isMessageDayCollapsed(list[i])) visibleIndexes.push(i);
