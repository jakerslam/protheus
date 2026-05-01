// Chat message preview, actor label, map marker, and day navigation helpers.
'use strict';

function infringChatMessagePreviewMapMethods() {
  return {
    messageToolPreview: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) {
        return this.messagePreview(msg);
      }
      var self = this;
      var parts = msg.tools.map(function(tool) {
        if (!tool) return '';
        var name = self.toolDisplayName(tool);
        var status = self.toolStatusText(tool);
        var summary = status ? (name + ' [' + status + ']') : name;
        var detail = String(tool.summary || tool.display_text || tool.result_ref || tool.input_ref || '').replace(/\s+/g, ' ').trim();
        if (detail && detail.length > 120) detail = detail.slice(0, 117) + '...';
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
      this.selectedMessageDomId = id;
      this.hoveredMessageDomId = id;
      this.mapStepIndex = idx;
      var chatStore = window.InfringChatStore;
      if (chatStore && typeof chatStore.setThreadProjectionCenter === 'function') {
        chatStore.setThreadProjectionCenter(idx);
      }
      this.centerChatMapOnMessage(id);
      var self = this;
      var attempts = 0;
      var scrollToTarget = function() {
        var target = document.getElementById(id);
        if (!target) {
          attempts += 1;
          if (attempts <= 4) {
            setTimeout(scrollToTarget, 28);
          }
          return;
        }
        target.scrollIntoView({ behavior: 'smooth', block: 'center' });
        if (typeof self.scheduleMessageRenderWindowUpdate === 'function') {
          self.scheduleMessageRenderWindowUpdate();
        }
      };
      scrollToTarget();
    },

    jumpToMessageDay: function(msg) {
      var key = this.messageDayKey(msg);
      if (!key) return;
      var target = document.querySelector('.chat-day-anchor[data-day="' + key + '"]');
      if (!target) return;
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    },
  };
}
