function infringChatModelUsageNoticeMethods() {
  return {
    markAgentMessageComplete(msg) {
      if (!msg || msg.role !== 'agent') return;
      msg._finish_bounce = true;
      setTimeout(function() {
        try { msg._finish_bounce = false; } catch(_) {}
      }, 300);
    },

    fetchModelContextWindows(force) {
      var now = Date.now();
      if (!force && this._contextModelsFetchedAt && (now - this._contextModelsFetchedAt) < 300000) {
        this.setContextWindowFromCurrentAgent();
        return Promise.resolve();
      }
      var self = this;
      return InfringAPI.get('/api/models').then(function(data) {
        self.refreshContextWindowMap(data && data.models ? data.models : []);
        self._contextModelsFetchedAt = Date.now();
        self.setContextWindowFromCurrentAgent();
      }).catch(function() {});
    },

    requestContextTelemetry(force) {
      if (!this.currentAgent || !InfringAPI.isWsConnected()) return false;
      var now = Date.now();
      if (!force && (now - Number(this._lastContextRequestAt || 0)) < 2500) return false;
      this._lastContextRequestAt = now;
      return !!InfringAPI.wsSend({ type: 'command', command: 'context', silent: true });
    },

    normalizeModelUsageKey: function(modelId) {
      return String(modelId || '').trim().toLowerCase();
    },

    loadModelUsageCache: function() {
      try {
        var raw = localStorage.getItem(this.modelUsageCacheKey);
        if (!raw) {
          this.modelUsageCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelUsageCache = parsed && typeof parsed === 'object' ? parsed : {};
      } catch {
        this.modelUsageCache = {};
      }
    },

    persistModelUsageCache: function() {
      try {
        localStorage.setItem(this.modelUsageCacheKey, JSON.stringify(this.modelUsageCache || {}));
      } catch {}
    },

    modelUsageTimestamp: function(modelId) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key || !this.modelUsageCache || typeof this.modelUsageCache !== 'object') return 0;
      var ts = Number(this.modelUsageCache[key] || 0);
      return Number.isFinite(ts) && ts > 0 ? ts : 0;
    },

    // Backward-compat shim for legacy callers during naming migration.
    modelUsageTs: function(modelId) {
      return this.modelUsageTimestamp(modelId);
    },

    recordModelUsageTimestamp: function(modelId, ts) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key) return;
      if (!this.modelUsageCache || typeof this.modelUsageCache !== 'object') {
        this.modelUsageCache = {};
      }
      var stamp = Number(ts || Date.now());
      this.modelUsageCache[key] = Number.isFinite(stamp) && stamp > 0 ? stamp : Date.now();
      this.persistModelUsageCache();
    },

    // Backward-compat shim for legacy callers during naming migration.
    touchModelUsage: function(modelId, ts) {
      this.recordModelUsageTimestamp(modelId, ts);
    },

    loadModelNoticeCache: function() {
      try {
        var raw = localStorage.getItem(this.modelNoticeCacheKey);
        if (!raw) {
          this.modelNoticeCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelNoticeCache = (parsed && typeof parsed === 'object') ? parsed : {};
      } catch {
        this.modelNoticeCache = {};
      }
    },

    persistModelNoticeCache: function() {
      try {
        localStorage.setItem(this.modelNoticeCacheKey, JSON.stringify(this.modelNoticeCache || {}));
      } catch {}
    },

    normalizeNoticeType: function(value, fallbackType) {
      var fallback = String(fallbackType || 'info').toLowerCase();
      if (fallback !== 'model' && fallback !== 'info') fallback = 'info';
      var raw = String(value || '').toLowerCase().trim();
      if (raw === 'model' || raw === 'info') return raw;
      return fallback;
    },

    isModelSwitchNoticeLabel: function(label) {
      var text = String(label || '').trim();
      if (!text) return false;
      return /^Model switched (?:to\b|from\b)/i.test(text);
    },

    rememberModelNotice: function(agentId, label, ts, noticeType, noticeIcon) {
      if (!agentId || !label) return;
      if (!this.modelNoticeCache || typeof this.modelNoticeCache !== 'object') {
        this.modelNoticeCache = {};
      }
      var key = String(agentId);
      if (!Array.isArray(this.modelNoticeCache[key])) this.modelNoticeCache[key] = [];
      var list = this.modelNoticeCache[key];
      var tsNum = Number(ts || Date.now());
      var normalizedType = this.normalizeNoticeType(
        noticeType,
        this.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
      );
      var normalizedIcon = String(noticeIcon || '').trim();
      var exists = list.some(function(entry) {
        return (
          entry &&
          entry.label === label &&
          Number(entry.ts || 0) === tsNum &&
          String(entry.type || '') === normalizedType
        );
      });
      if (!exists) list.push({ label: label, ts: tsNum, type: normalizedType, icon: normalizedIcon });
      if (list.length > 120) this.modelNoticeCache[key] = list.slice(list.length - 120);
      this.persistModelNoticeCache();
    },

    mergeModelNoticesForAgent: function(agentId, rows) {
      var list = Array.isArray(rows) ? rows.slice() : [];
      if (!agentId || !this.modelNoticeCache) return list;
      var notices = this.modelNoticeCache[String(agentId)];
      if (!Array.isArray(notices) || !notices.length) return list;
      var existing = {};
      var self = this;
      list.forEach(function(msg) {
        if (!msg) return;
        var label = msg.notice_label || '';
        if (!label && msg.role === 'system' && typeof msg.text === 'string' && self.isModelSwitchNoticeLabel(msg.text.trim())) {
          label = msg.text.trim();
        }
        if (!label) return;
        var type = self.normalizeNoticeType(
          msg.notice_type,
          self.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
        );
        existing[type + '|' + label + '|' + Number(msg.ts || 0)] = true;
      });
      for (var i = 0; i < notices.length; i++) {
        var n = notices[i] || {};
        var nLabel = String(n.label || '').trim();
        if (!nLabel) continue;
        var nTs = Number(n.ts || 0) || Date.now();
        var nType = this.normalizeNoticeType(
          n.type || n.notice_type,
          this.isModelSwitchNoticeLabel(nLabel) ? 'model' : 'info'
        );
        var nIcon = String(n.icon || n.notice_icon || '').trim();
        var nKey = nType + '|' + nLabel + '|' + nTs;
        if (existing[nKey]) continue;
        list.push({
          id: ++msgId,
          role: 'notice',
          text: '',
          meta: '',
          tools: [],
          system_origin: 'notice:' + nType,
          is_notice: true,
          notice_label: nLabel,
          notice_type: nType,
          notice_icon: nIcon,
          ts: nTs
        });
      }
      list.sort(function(a, b) {
        return Number((a && a.ts) || 0) - Number((b && b.ts) || 0);
      });
      return list;
    },

  };
}
