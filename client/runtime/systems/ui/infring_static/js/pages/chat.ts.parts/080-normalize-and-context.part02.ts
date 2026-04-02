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

    modelUsageTs: function(modelId) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key || !this.modelUsageCache || typeof this.modelUsageCache !== 'object') return 0;
      var ts = Number(this.modelUsageCache[key] || 0);
      return Number.isFinite(ts) && ts > 0 ? ts : 0;
    },

    touchModelUsage: function(modelId, ts) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key) return;
      if (!this.modelUsageCache || typeof this.modelUsageCache !== 'object') {
        this.modelUsageCache = {};
      }
      var stamp = Number(ts || Date.now());
      this.modelUsageCache[key] = Number.isFinite(stamp) && stamp > 0 ? stamp : Date.now();
      this.persistModelUsageCache();
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
          role: 'system',
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

    normalizeSessionMessages(data) {
      var source = [];
      if (data && Array.isArray(data.messages)) {
        source = data.messages;
      } else if (data && Array.isArray(data.turns)) {
        var turns = data.turns;
        var turnRows = [];
        turns.forEach(function(turn) {
          var ts = turn && turn.ts ? turn.ts : Date.now();
          if (turn && typeof turn.user === 'string' && turn.user.trim()) {
            turnRows.push({ role: 'User', content: turn.user, ts: ts });
          }
          if (turn && typeof turn.assistant === 'string' && turn.assistant.trim()) {
            turnRows.push({ role: 'Agent', content: turn.assistant, ts: ts });
          }
        });
        source = turnRows;
      } else {
        source = [];
      }
      var self = this;
      return source.map(function(m) {
        var roleRaw = String((m && (m.role || m.type)) || '').toLowerCase();
        var isTerminal = roleRaw.indexOf('terminal') >= 0 || !!(m && m.terminal);
        var role = isTerminal
          ? 'terminal'
          : (roleRaw.indexOf('user') >= 0 ? 'user' : (roleRaw.indexOf('system') >= 0 ? 'system' : 'agent'));
        var textSource = m && (m.content != null ? m.content : (m.text != null ? m.text : m.message));
        if (role === 'user' && m && m.user != null) textSource = m.user;
        if (role !== 'user' && !isTerminal && m && m.assistant != null) textSource = m.assistant;
        var text = typeof textSource === 'string' ? textSource : JSON.stringify(textSource || '');
        text = self.sanitizeToolText(text);
        if (role === 'agent') text = self.stripModelPrefix(text);
        var derivedSystemOrigin = '';
        if (role === 'user' && /^\s*protheus(?:-ops)?\s+/i.test(String(text || ''))) {
          role = 'system';
          derivedSystemOrigin = 'runtime:ops_command';
        }
        if (role === 'user' && /^\s*\[runtime-task\]/i.test(String(text || ''))) {
          role = 'system';
          if (!derivedSystemOrigin) derivedSystemOrigin = 'runtime:task';
        }

        var tools = (m && Array.isArray(m.tools) ? m.tools : []).map(function(t, idx) {
          return {
            id: (t.name || 'tool') + '-hist-' + idx,
            name: t.name || 'unknown',
            running: false,
            expanded: false,
            input: t.input || '',
            result: t.result || '',
            is_error: !!t.is_error
          };
        });
        var images = (m && Array.isArray(m.images) ? m.images : []).map(function(img) {
          return { file_id: img.file_id, filename: img.filename || 'image' };
        });
        var tsRaw = m && (m.ts || m.timestamp || m.created_at || m.createdAt) ? (m.ts || m.timestamp || m.created_at || m.createdAt) : null;
        var ts = null;
        if (typeof tsRaw === 'number') {
          ts = tsRaw;
        } else if (typeof tsRaw === 'string') {
          var parsedTs = Date.parse(tsRaw);
          ts = Number.isNaN(parsedTs) ? null : parsedTs;
        }
        var meta = typeof (m && m.meta) === 'string' ? m.meta : '';
        if (!meta && m && (m.input_tokens || m.output_tokens)) {
          meta = (m.input_tokens || 0) + ' in / ' + (m.output_tokens || 0) + ' out';
        }
        var isNotice = false;
        var noticeLabel = '';
        var noticeType = '';
        var noticeIcon = '';
        var noticeAction = null;
        if (m && (m.is_notice || m.notice_label || m.notice_type)) {
          var explicitLabel = String(m.notice_label || '').trim();
          var inferredLabel = typeof text === 'string' ? text.trim() : '';
          noticeLabel = explicitLabel || inferredLabel;
          if (noticeLabel) {
            isNotice = true;
            text = '';
            noticeType = self.normalizeNoticeType(
              m.notice_type,
              self.isModelSwitchNoticeLabel(noticeLabel) ? 'model' : 'info'
            );
            noticeIcon = String(m.notice_icon || '').trim();
            noticeAction = self.normalizeNoticeAction(m.notice_action || m.noticeAction || null);
          }
        }
        if (!isNotice && role === 'system' && typeof text === 'string') {
          var compact = text.trim();
          if (self.isModelSwitchNoticeLabel(compact)) {
            isNotice = true;
            noticeLabel = compact;
            text = '';
            noticeType = 'model';
          }
        }
        var systemOrigin = m && m.system_origin ? String(m.system_origin) : derivedSystemOrigin;
        var compactText = typeof text === 'string' ? text.trim() : '';
        if (
          role === 'system' &&
          !isNotice &&
          !systemOrigin &&
          (
            /^\[runtime-task\]/i.test(compactText) ||
            /^task accepted\.\s*report findings in this thread with receipt-backed evidence\.?$/i.test(compactText)
          )
        ) {
          // Legacy synthetic runtime-task chatter (no origin tag) is noise; skip rendering.
          return null;
        }
        if (
          role === 'system' &&
          !isNotice &&
          self.isSystemNotificationGlobalToWorkspace &&
          self.isSystemNotificationGlobalToWorkspace(systemOrigin, compactText) &&
          !(self.isSystemThreadAgent && self.isSystemThreadAgent(self.currentAgent))
        ) {
          // Keep global/system-wide notices out of non-system chats.
          return null;
        }
        return {
          id: ++msgId,
          role: role,
          text: text,
          meta: meta,
          tools: tools,
          images: images,
          ts: ts,
          is_notice: isNotice,
          notice_label: noticeLabel,
          notice_type: noticeType,
          notice_icon: noticeIcon,
          notice_action: noticeAction,
          terminal: isTerminal,
          terminal_source: m && m.terminal_source ? String(m.terminal_source).toLowerCase() : (isTerminal ? 'user' : ''),
          cwd: m && m.cwd ? String(m.cwd) : '',
          agent_id: m && m.agent_id ? String(m.agent_id) : '',
          agent_name: m && m.agent_name ? String(m.agent_name) : '',
          source_agent_id: m && m.source_agent_id ? String(m.source_agent_id) : '',
          agent_origin: m && m.agent_origin ? String(m.agent_origin) : '',
          system_origin: systemOrigin,
          actor_id: m && m.actor_id ? String(m.actor_id) : '',
          actor: m && m.actor ? String(m.actor) : ''
        };
      }).filter(function(row) { return !!row; });
    },

    isSystemNotificationGlobalToWorkspace: function(systemOrigin, text) {
      var origin = String(systemOrigin || '').trim().toLowerCase();
      var msg = String(text || '').trim().toLowerCase();
      if (!origin && !msg) return false;
      if (
        origin.indexOf('telemetry:') === 0 ||
        origin.indexOf('continuity:') === 0 ||
        origin === 'slash:alerts' ||
        origin === 'slash:next' ||
        origin === 'slash:memory' ||
        origin === 'slash:continuity' ||
        origin === 'slash:opt'
      ) {
        return true;
      }
      if (
        msg.indexOf('memory-backed session context') >= 0 ||
        msg.indexOf('stale memory context') >= 0 ||
        msg.indexOf('continuity cleanup') >= 0 ||
        msg.indexOf('cross-channel continuity') >= 0
      ) {
        return true;
      }
      return false;
    },

    init() {
      var self = this;

      if (typeof window !== 'undefined') {
        window.__infringChatCache = window.__infringChatCache || {};
        var persistedCache = this.loadConversationCache();
        var runtimeCache = window.__infringChatCache || {};
