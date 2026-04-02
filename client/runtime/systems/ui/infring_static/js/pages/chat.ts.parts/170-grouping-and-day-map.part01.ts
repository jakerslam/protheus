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

    messageTitleLabel: function(msg) {
      if (!msg) return '';
      var role = String(msg.role || '').toLowerCase();
      if (role === 'user') return 'me';
      return this.messageActorLabel(msg);
    },

    messageTitleClass: function(msg) {
      var role = String((msg && msg.role) || '').toLowerCase();
      if (role === 'user') return 'message-agent-name-user-ghost';
      return '';
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

    agentMessageSignature: function(message) {
      if (!message || typeof message !== 'object') return '';
      var text = String(message.text || '');
      text = this.stripModelPrefix(text);
      text = this.sanitizeToolText(text);
      text = this.stripArtifactDirectivesFromText(text);
      text = text.replace(/\s+/g, ' ').trim().toLowerCase();
      var tools = Array.isArray(message.tools) ? message.tools : [];
      var toolParts = [];
      for (var i = 0; i < tools.length && i < 8; i += 1) {
        var tool = tools[i] || {};
        var name = String(tool.name || '').trim().toLowerCase();
        var result = String(tool.result || '').replace(/\s+/g, ' ').trim().toLowerCase();
        if (result.length > 180) result = result.slice(0, 180);
        var state = tool && tool.is_error ? 'error' : (tool && tool.running ? 'running' : 'ok');
        if (name || result) toolParts.push(name + ':' + state + ':' + result);
      }
      return (text || '') + '||' + toolParts.join('||');
    },

    findRecentDuplicateAgentMessage: function(candidate, dedupeWindowMs) {
      if (!candidate || typeof candidate !== 'object') return null;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      if (!rows.length) return null;
      var signature = this.agentMessageSignature(candidate);
      if (!signature) return null;
      var nowTs = Number(candidate.ts || Date.now());
      var maxAge = Number(dedupeWindowMs || 70000);
      if (!Number.isFinite(maxAge) || maxAge < 5000) maxAge = 70000;
      var checked = 0;
      for (var i = rows.length - 1; i >= 0; i -= 1) {
        var row = rows[i];
        if (!row || row.thinking || row.streaming) continue;
        var role = String(row.role || '').toLowerCase();
        if (role !== 'agent' && role !== 'assistant') continue;
        checked += 1;
        var rowTs = Number(row.ts || 0);
        var ageMs = rowTs > 0 ? Math.abs(nowTs - rowTs) : 0;
        if (ageMs > maxAge && checked > 3) break;
        if (this.agentMessageSignature(row) === signature) return row;
        if (checked >= 16) break;
      }
      return null;
    },

    pushAgentMessageDeduped: function(message, options) {
      var payload = message && typeof message === 'object' ? message : null;
      if (!payload) return null;
      var opts = options && typeof options === 'object' ? options : {};
      var dedupeWindowMs = Number(opts.dedupe_window_ms || opts.dedupeWindowMs || 70000);
      var duplicate = this.findRecentDuplicateAgentMessage(payload, dedupeWindowMs);
      if (!duplicate) {
        this.messages.push(payload);
        return payload;
      }
      if (duplicate._auto_fallback && !payload._auto_fallback) {
        duplicate.text = payload.text;
        duplicate.tools = Array.isArray(payload.tools) ? payload.tools : [];
        duplicate._auto_fallback = false;
      } else if ((!String(duplicate.text || '').trim()) && String(payload.text || '').trim()) {
        duplicate.text = payload.text;
      }
      if ((!Array.isArray(duplicate.tools) || !duplicate.tools.length) && Array.isArray(payload.tools) && payload.tools.length) {
        duplicate.tools = payload.tools;
      }
      var nextMeta = String(payload.meta || '').trim();
      if (nextMeta) {
        var priorMeta = String(duplicate.meta || '').trim();
        duplicate.meta = priorMeta ? priorMeta : nextMeta;
      }
      duplicate.ts = Number(payload.ts || Date.now());
      duplicate.agent_id = payload.agent_id || duplicate.agent_id;
      duplicate.agent_name = payload.agent_name || duplicate.agent_name;
      this.scheduleConversationPersist();
      return duplicate;
