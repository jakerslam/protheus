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
      var compact = this.messageVisiblePreviewText(msg);
      if (!compact) return false;
      return compact.length >= 220 || compact.indexOf('\n\n') >= 0;
    },

    messageVisiblePreviewText: function(msg) {
      if (!msg) return '';
      var text = typeof this.extractMessageVisibleText === 'function' ? this.extractMessageVisibleText(msg) : '';
      if (!text && typeof msg.thinking_text === 'string') text = String(msg.thinking_text || '');
      if (!text && Array.isArray(msg.tools) && msg.tools.length) text = this.messageToolSummary(msg);
      if (!text && msg.notice_label) text = String(msg.notice_label || '');
      return String(text || '').trim();
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

    normalizeSystemMessageText: function(rawText) {
      var raw = String(rawText || '');
      if (!raw.trim()) return '';
      var lowered = raw.toLowerCase();
      var errorLike = /^\s*error:/i.test(raw) || lowered.indexOf('request_read_failed') >= 0;
      if (!errorLike) return raw.trim();

      var lines = raw.split(/\r?\n/);
      var deduped = [];
      var previousKey = '';
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').replace(/\s+/g, ' ').trim();
        if (!line) continue;
        var key = line.toLowerCase();
        if (key === previousKey) continue;
        deduped.push(line);
        previousKey = key;
      }

      if (lowered.indexOf('request_read_failed') >= 0 && deduped.length > 1) {
        var unique = [];
        var seen = {};
        for (var j = 0; j < deduped.length; j++) {
          var value = String(deduped[j] || '');
          var valueKey = value.toLowerCase();
          if (seen[valueKey]) continue;
          seen[valueKey] = true;
          unique.push(value);
        }
        deduped = unique;
      }

      return deduped.join('\n').trim();
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
      if (msg.terminal) {
        var terminalSource = this.terminalMessageSource(msg);
        if (terminalSource === 'user') return 'You';
        if (terminalSource === 'system') return 'System';
        return this.messageAgentLabel(msg);
      }
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
      var text = this.messageVisiblePreviewText(message).replace(/\s+/g, ' ').trim().toLowerCase();
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
      var mergeToolCards = function(existingTools, incomingTools) {
        var base = Array.isArray(existingTools) ? existingTools.slice() : [];
        var incoming = Array.isArray(incomingTools) ? incomingTools : [];
        if (!incoming.length) return base;
        var keyFor = function(tool) {
          if (!tool || typeof tool !== 'object') return '';
          var id = String(tool.id || '').trim();
          if (id) return 'id:' + id;
          var name = String(tool.name || '').trim().toLowerCase();
          var input = String(tool.input || '').trim();
          return 'sig:' + name + '::' + input;
        };
        var index = Object.create(null);
        for (var i = 0; i < base.length; i++) {
          var baseKey = keyFor(base[i]);
          if (!baseKey) continue;
          index[baseKey] = i;
        }
        for (var j = 0; j < incoming.length; j++) {
          var next = incoming[j];
          if (!next || typeof next !== 'object') continue;
          var nextKey = keyFor(next);
          var pos = (nextKey && Object.prototype.hasOwnProperty.call(index, nextKey))
            ? Number(index[nextKey])
            : -1;
          if (pos < 0 || pos >= base.length) {
            base.push(next);
            if (nextKey) index[nextKey] = base.length - 1;
            continue;
          }
          var prior = base[pos];
          if (!prior || typeof prior !== 'object') {
            base[pos] = next;
            continue;
          }
          if (!String(prior.result || '').trim() && String(next.result || '').trim()) prior.result = next.result;
          if (!String(prior.input || '').trim() && String(next.input || '').trim()) prior.input = next.input;
          if (!String(prior.id || '').trim() && String(next.id || '').trim()) prior.id = next.id;
          if (next.is_error) prior.is_error = true;
          if (prior.running && next.running === false) prior.running = false;
        }
        return base;
      };
      if (duplicate._auto_fallback && !payload._auto_fallback) {
        duplicate.text = payload.text;
        duplicate.tools = Array.isArray(payload.tools) ? payload.tools : [];
        duplicate._auto_fallback = false;
      } else if ((!String(duplicate.text || '').trim()) && String(payload.text || '').trim()) {
        duplicate.text = payload.text;
      }
      if (Array.isArray(payload.tools) && payload.tools.length) {
        duplicate.tools = mergeToolCards(duplicate.tools, payload.tools);
      }
      if (payload.response_finalization && typeof payload.response_finalization === 'object') {
        duplicate.response_finalization = payload.response_finalization;
      }
      if (payload.turn_transaction && typeof payload.turn_transaction === 'object') {
        duplicate.turn_transaction = payload.turn_transaction;
      }
      if (Array.isArray(payload.terminal_transcript) && payload.terminal_transcript.length) {
        duplicate.terminal_transcript = payload.terminal_transcript;
      }
      if (payload.attention_queue && typeof payload.attention_queue === 'object') {
        duplicate.attention_queue = payload.attention_queue;
      }
      if (String(payload.tool_failure_summary || '').trim()) {
        duplicate.tool_failure_summary = String(payload.tool_failure_summary || '').trim();
      }
      if (!String(duplicate.text || '').trim() && typeof this.fallbackAssistantTextFromPayload === 'function') {
        var repairedDuplicateText = String(this.fallbackAssistantTextFromPayload(duplicate, duplicate.tools || []) || '').trim();
        if (repairedDuplicateText) duplicate.text = repairedDuplicateText;
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
