      var parts = msg.tools.map(function(tool) {
        if (!tool) return '';
        var name = self.toolDisplayName(tool);
        var status = self.toolStatusText(tool);
        var summary = status ? (name + ' [' + status + ']') : name;
        var inputPreview = compactToolText(tool.input, 96);
        var resultPreview = compactToolText(tool.result, 120);
        var detail = '';
        if (inputPreview && resultPreview) {
          detail = inputPreview + ' -> ' + resultPreview;
        } else {
          detail = inputPreview || resultPreview;
        }
        if (detail) summary += ': ' + detail;
        return summary;
      }).filter(function(part) { return !!part; });

      if (!parts.length) return 'Tool call';
      var preview = parts.join(' | ');
      if (preview.length > 220) return preview.slice(0, 217) + '...';
      return preview;
    },

    isLongMessagePreview: function(msg) {
      if (!msg) return false;
      var raw = '';
      if (typeof msg.text === 'string' && msg.text.trim()) {
        raw = msg.text;
      } else if (Array.isArray(msg.tools) && msg.tools.length) {
        raw = msg.tools.map(function(tool) {
          return tool && tool.name ? tool.name : 'tool';
        }).join(', ');
      }
      if (!raw) return false;
      var compact = raw.replace(/\s+/g, ' ').trim();
      if (compact.length >= 220) return true;
      if (raw.indexOf('\n\n') >= 0) return true;
      return false;
    },

    isSelectedMessage: function(msg, idx) {
      if (!this.selectedMessageDomId) return false;
      return this.selectedMessageDomId === this.messageDomId(msg, idx);
    },

    truncateActorLabel: function(label, maxChars) {
      var text = String(label || '').replace(/\s+/g, ' ').trim();
      if (!text) return '';
      var limitRaw = Number(maxChars || 0);
      var limit = Number.isFinite(limitRaw) && limitRaw > 0 ? Math.max(8, Math.floor(limitRaw)) : 24;
      if (text.length <= limit) return text;
      return text.slice(0, limit - 1) + '\u2026';
    },

    messageAgentLabel: function(msg) {
      var name = '';
      if (msg && msg.agent_name) name = String(msg.agent_name || '');
      if (!name && msg && msg.agent_id) {
        var resolved = this.resolveAgent(msg.agent_id);
        if (resolved && resolved.name) name = String(resolved.name || '');
      }
      if (!name && this.currentAgent && this.currentAgent.name) {
        name = String(this.currentAgent.name || '');
      }
      var shortName = this.truncateActorLabel(name, 28);
      return shortName || 'Agent';
    },

    messageActorLabel: function(msg) {
      if (!msg) return 'Message';
      if (msg.is_notice) {
        if (this.normalizeNoticeType(msg.notice_type, 'model') === 'info') return '\u24d8 Info';
        return 'Model';
      }
      if (msg.terminal) return 'Terminal';
      if (Array.isArray(msg.tools) && msg.tools.length && (!msg.text || !String(msg.text).trim())) {
        return 'Tool';
      }
      if (msg.role === 'user') return 'You';
      if (msg.role === 'system') return 'System';
      if (msg.role === 'agent') {
        var name = '';
        if (msg && msg.agent_name) name = String(msg.agent_name || '');
        if (!name && msg && msg.agent_id) {
          var resolved = this.resolveAgent(msg.agent_id);
          if (resolved && resolved.name) name = String(resolved.name || '');
        }
        if (!name && this.currentAgent && this.currentAgent.name) {
          name = String(this.currentAgent.name || '');
        }
        var shortName = this.truncateActorLabel(name, 24);
        if (shortName) return shortName;
      }
      return 'Agent';
    },

    isMessageMetaReserveSpace: function(msg, idx, rows) {
      if (!msg || msg.is_notice || msg.thinking) return false;
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list)) return false;
      if (this.isDirectHoveredMessage(msg, idx)) return false;
      return this.isLastInSourceRun(idx, list);
    },

    isRenameNotice: function(msg) {
      if (!msg || !msg.is_notice) return false;
      return /^changed name from /i.test(String(msg.notice_label || '').trim());
    },

    messageMapToolOutcome: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
      var hasError = false;
      var hasWarning = false;
      for (var i = 0; i < msg.tools.length; i++) {
        var tool = msg.tools[i] || {};
        if (tool.running || this.isBlockedTool(tool)) {
          hasWarning = true;
          continue;
        }
        if (tool.is_error) {
          hasError = true;
        }
      }
      if (hasError) return 'error';
      if (hasWarning) return 'warning';
      return 'success';
    },

    messageMapMarkerType: function(msg) {
      if (!msg) return '';
      if (msg.is_notice) {
        return this.normalizeNoticeType(msg.notice_type, 'model') === 'info' ? 'info' : 'model';
      }
      if (msg.terminal) return 'terminal';
      if (Array.isArray(msg.tools) && msg.tools.length) return 'tool';
      return '';
    },

    messageMapShowMarker: function(msg) {
      return this.messageMapMarkerType(msg) !== '';
    },

    messageMapMarkerTitle: function(msg) {
      var type = this.messageMapMarkerType(msg);
      if (type === 'model') {
        return msg && msg.notice_label ? String(msg.notice_label) : 'Model switched';
      }
      if (type === 'info') {
        return msg && msg.notice_label ? String(msg.notice_label) : 'Info';
      }
      if (type === 'tool') {
        var outcome = this.messageMapToolOutcome(msg) || 'success';
        if (outcome === 'error') return 'Tool call error';
        if (outcome === 'warning') return 'Tool call warning';
        return 'Tool call success';
      }
      if (type === 'terminal') {
        return 'Terminal activity';
      }
      return '';
    },

    messageDayKey: function(msg) {
      if (!msg || !msg.ts) return '';
      var d = new Date(msg.ts);
      if (Number.isNaN(d.getTime())) return '';
      var y = d.getFullYear();
      var m = String(d.getMonth() + 1).padStart(2, '0');
      var day = String(d.getDate()).padStart(2, '0');
      return y + '-' + m + '-' + day;
    },

    messageDayLabel: function(msg) {
      if (!msg || !msg.ts) return 'Unknown day';
      var d = new Date(msg.ts);
      if (Number.isNaN(d.getTime())) return 'Unknown day';
      return d.toLocaleDateString(undefined, { weekday: 'long', month: 'short', day: 'numeric', year: 'numeric' });
    },

    messageDayDomId: function(msg) {
      var key = this.messageDayKey(msg);
      return key ? ('chat-day-' + key) : '';
    },

    messageDayCollapseKey: function(msg) {
      var dayKey = this.messageDayKey(msg);
      if (!dayKey) return '';
      var agentKey = String((this.currentAgent && this.currentAgent.id) || (msg && msg.agent_id) || 'global').trim();
      return (agentKey || 'global') + '::' + dayKey;
    },

    isMessageDayCollapsed: function(msg) {
      var key = this.messageDayCollapseKey(msg);
      if (!key) return false;
      return !!(this.collapsedMessageDays && this.collapsedMessageDays[key]);
    },

    toggleMessageDayCollapse: function(msg) {
      var key = this.messageDayCollapseKey(msg);
      if (!key) return;
      if (!this.collapsedMessageDays) this.collapsedMessageDays = {};
      this.collapsedMessageDays[key] = !this.collapsedMessageDays[key];
    },

    isNewMessageDay: function(list, idx) {
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      if (idx === 0) return true;
      var curr = this.messageDayKey(list[idx]);
      var prev = this.messageDayKey(list[idx - 1]);
      if (!curr) return false;
      return curr !== prev;
    },

    jumpToMessage: function(msg, idx) {
      var id = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      var target = document.getElementById(id);
      if (!target) return;
      this.selectedMessageDomId = id;
      this.hoveredMessageDomId = id;
      target.scrollIntoView({ behavior: 'smooth', block: 'center' });
      this.mapStepIndex = idx;
      this.centerChatMapOnMessage(id);
    },

    jumpToMessageDay: function(msg) {
      var key = this.messageDayKey(msg);
      if (!key) return;
      var target = document.querySelector('.chat-day-anchor[data-day="' + key + '"]');
      if (!target) return;
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    },

    normalizeNoticeAction: function(action) {
      if (!action || typeof action !== 'object') return null;
      var kind = String(action.kind || action.type || '').trim().toLowerCase();
      if (!kind || kind !== 'system_update') return null;
      var label = String(action.label || '').trim() || 'Update';
      return {
        kind: kind,
        label: label,
        latest_version: String(action.latest_version || '').trim(),
        current_version: String(action.current_version || '').trim(),
        busy: !!action.busy
      };
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
      if (!action || action.kind !== 'system_update') return;
      if (this.systemUpdateBusy || this.noticeActionBusy(msg)) return;
      if (msg && msg.notice_action) msg.notice_action.busy = true;
      this.systemUpdateBusy = true;
      this.scheduleConversationPersist();
      try {
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
      } catch (e) {
        var reason = e && e.message ? String(e.message) : 'unknown_error';
        InfringToast.error('Failed to start system update: ' + reason);
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

    addNoticeEvent: function(notice) {
      if (!notice || typeof notice !== 'object') return;
      var label = String(notice.notice_label || notice.label || '').trim();
      if (!label) return;
      var type = this.normalizeNoticeType(
        notice.notice_type || notice.type,
        this.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
      );
      var icon = String(notice.notice_icon || notice.icon || '').trim();
      if (type === 'info' && /^changed name from /i.test(label)) {
        icon = '';
      }
      var tsRaw = Number(notice.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var action = this.normalizeNoticeAction(notice.notice_action || notice.noticeAction || null);
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: '',
        meta: '',
        tools: [],
        system_origin: 'notice:' + type,
        is_notice: true,
        notice_label: label,
        notice_type: type,
        notice_icon: icon,
        notice_action: action,
        ts: ts
      });
      if (this.currentAgent && this.currentAgent.id) {
        this.rememberModelNotice(this.currentAgent.id, label, ts, type, icon);
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    pushSystemMessage: function(entry) {
      var payload = entry && typeof entry === 'object' ? entry : { text: entry };
      var text = String(payload && payload.text ? payload.text : '').trim();
      if (!text) return null;
      var origin = String(payload.system_origin || payload.systemOrigin || '').trim();
      var canonicalText = text.replace(/\s+/g, ' ').trim().toLowerCase();
      var tsRaw = Number(payload.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      var dedupeWindowMs = Number(payload.dedupe_window_ms || payload.dedupeWindowMs || 8000);
      if (!Number.isFinite(dedupeWindowMs) || dedupeWindowMs < 0) dedupeWindowMs = 8000;
      if (dedupeWindowMs > 60000) dedupeWindowMs = 60000;
      var dedupeScope = Number(payload.dedupe_scope || payload.dedupeScope || 12);
      if (!Number.isFinite(dedupeScope) || dedupeScope < 1) dedupeScope = 12;
      if (dedupeScope > 24) dedupeScope = 24;
      var canDedupe = payload.dedupe !== false;
      if (canDedupe && Array.isArray(this.messages) && this.messages.length > 0) {
        var scannedSystemRows = 0;
        for (var idx = this.messages.length - 1; idx >= 0; idx -= 1) {
          var row = this.messages[idx];
          if (!row || row.thinking || row.streaming) continue;
          var role = String(row.role || '').toLowerCase();
          if (role !== 'system' || row.is_notice) {
            if (scannedSystemRows > 0) break;
            continue;
          }
          scannedSystemRows += 1;
          var rowText = String(row.text || '').trim();
          var rowCanonicalText = rowText.replace(/\s+/g, ' ').trim().toLowerCase();
          var rowOrigin = String(row.system_origin || '').trim();
          var rowTs = Number(row.ts || 0);
          var ageMs = Number.isFinite(rowTs) && rowTs > 0 ? Math.abs(ts - rowTs) : Number.POSITIVE_INFINITY;
          var sameText = rowCanonicalText === canonicalText;
          var sameOrigin = rowOrigin === origin || !rowOrigin || !origin;
          var isErrorLine = /^error:/i.test(canonicalText) || /^error:/i.test(rowCanonicalText);
          if (sameText && ageMs <= dedupeWindowMs && (sameOrigin || isErrorLine)) {
            var repeatCount = Number(row._repeat_count || 1);
            if (!Number.isFinite(repeatCount) || repeatCount < 1) repeatCount = 1;
            repeatCount += 1;
            row._repeat_count = repeatCount;
            var priorMeta = String(row.meta || '').trim().replace(/\s*\|\s*repeated x\d+\s*$/i, '').trim();
            row.meta = (priorMeta ? (priorMeta + ' | ') : '') + 'repeated x' + repeatCount;
            row.ts = ts;
            this.scheduleConversationPersist();
            return row;
          }
          if (scannedSystemRows >= dedupeScope) break;
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
      this.messages.push(message);
      if (payload.auto_scroll) this.scrollToBottom();
      this.scheduleConversationPersist();
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
