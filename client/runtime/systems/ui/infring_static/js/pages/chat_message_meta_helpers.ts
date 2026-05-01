// Chat message timestamp, progress, identity, and terminal preview helpers.
'use strict';

function infringChatMessageMetaMethods() {
  return {
    // Format timestamp for display
    formatClockTime: function(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      var h = d.getHours();
      var m = d.getMinutes();
      var ampm = h >= 12 ? 'PM' : 'AM';
      h = h % 12 || 12;
      return h + ':' + (m < 10 ? '0' : '') + m + ' ' + ampm;
    },

    // Backward-compat shim for legacy callers during naming migration.
    formatTime: function(ts) {
      return this.formatClockTime(ts);
    },

    isSameDay: function(a, b) {
      if (!a || !b) return false;
      return (
        a.getFullYear() === b.getFullYear() &&
        a.getMonth() === b.getMonth() &&
        a.getDate() === b.getDate()
      );
    },

    // UI-safe timestamp formatter for templates
    messageTimestampLabel: function(msg) {
      if (!msg || !msg.ts) return '';
      var ts = new Date(msg.ts);
      if (Number.isNaN(ts.getTime())) return '';
      var now = new Date();
      if (this.isSameDay(ts, now)) return this.formatClockTime(ts);
      var yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      if (this.isSameDay(ts, yesterday)) {
        return 'Yesterday at ' + this.formatClockTime(ts);
      }
      var dateText = ts.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
      return dateText + ' at ' + this.formatClockTime(ts);
    },

    // Backward-compat shim for legacy callers during naming migration.
    messageTs: function(msg) {
      return this.messageTimestampLabel(msg);
    },

    parseProgressFromText: function(text) {
      var value = String(text || '');
      if (!value) return null;
      var explicit = value.match(/\[\[\s*progress\s*:\s*([0-9]{1,3})(?:\s*\/\s*([0-9]{1,3}))?\s*\]\]/i);
      if (explicit) {
        var part = Number(explicit[1] || 0);
        var total = Number(explicit[2] || 100);
        if (Number.isFinite(part) && Number.isFinite(total) && total > 0) {
          var pct = Math.max(0, Math.min(100, Math.round((part / total) * 100)));
          return { percent: pct, label: 'Progress ' + pct + '%' };
        }
      }
      var percent = value.match(/\bprogress(?:\s+is)?\s*[:=-]?\s*([0-9]{1,3})\s*%/i);
      if (percent) {
        var p = Number(percent[1] || 0);
        if (Number.isFinite(p)) {
          var clamped = Math.max(0, Math.min(100, Math.round(p)));
          return { percent: clamped, label: 'Progress ' + clamped + '%' };
        }
      }
      return null;
    },

    messageProgress: function(msg) {
      if (!msg || msg.terminal || msg.is_notice) return null;
      var key = String(msg.id || '') + '|' + String(msg.text || '').length + '|' + String(msg.meta || '').length;
      if (!this._progressCache || typeof this._progressCache !== 'object') this._progressCache = {};
      var keys = Object.keys(this._progressCache);
      if (keys.length > 4096) {
        this._progressCache = {};
      }
      if (Object.prototype.hasOwnProperty.call(this._progressCache, key)) return this._progressCache[key];

      var progress = null;
      if (msg.progress && typeof msg.progress === 'object') {
        var pct = Number(msg.progress.percent);
        if (Number.isFinite(pct)) {
          progress = {
            percent: Math.max(0, Math.min(100, Math.round(pct))),
            label: String(msg.progress.label || ('Progress ' + Math.round(pct) + '%')).trim()
          };
        }
      }
      if (!progress) progress = this.parseProgressFromText(msg.text || '');
      this._progressCache[key] = progress;
      return progress;
    },

    progressFillStyle: function(msg) {
      var progress = this.messageProgress(msg);
      if (!progress) return 'width:0%';
      return 'width:' + progress.percent + '%';
    },

    messageDomId: function(msg, idx) {
      var suffix = (msg && msg.id != null) ? String(msg.id) : String(idx || 0);
      return 'chat-msg-' + suffix;
    },

    messageRenderKey: function(msg, idx) {
      var idPart = (msg && msg.id != null) ? String(msg.id) : '';
      var tsPart = (msg && msg.ts != null) ? String(msg.ts) : '';
      var rolePart = String((msg && msg.role) || '');
      var noticePart = msg && msg.is_notice ? 'notice' : 'message';
      if (msg && (msg.thinking || msg.streaming || msg._typingVisual || msg._typewriterRunning)) {
        return noticePart + '|' + idPart + '|' + tsPart + '|' + rolePart + '|' + String(idx || 0) + '|live';
      }
      var textLen = (msg && typeof msg.text === 'string') ? msg.text.length : 0;
      return noticePart + '|' + idPart + '|' + tsPart + '|' + rolePart + '|' + String(idx || 0) + '|' + String(textLen);
    },

    messageRoleClass: function(msg) {
      if (msg && msg.terminal) {
        var source = this.terminalMessageSource(msg);
        if (source === 'user') return 'terminal terminal-user';
        if (source === 'agent') return 'terminal terminal-agent';
        return 'terminal terminal-system';
      }
      if (!msg || !msg.role) return 'agent';
      return String(msg.role);
    },

    terminalMessageSource: function(msg) {
      if (!msg || !msg.terminal) return 'agent';
      var source = String(msg.terminal_source || '').trim().toLowerCase();
      if (source === 'user' || source === 'agent' || source === 'system') return source;
      if (source === 'assistant') return 'agent';
      return 'system';
    },

    terminalToolboxSideClass: function(msg) {
      return this.terminalMessageSource(msg) === 'user' ? 'terminal-toolbox-right' : 'terminal-toolbox-left';
    },

    expandTerminalMessage: function(msg, idx, rows) {
      if (!msg || !msg.terminal || msg.thinking) return;
      if (msg._terminal_expanded === true) return;
      if (!this.terminalMessageCollapsed(msg, idx, rows)) return;
      msg._terminal_expanded = true;
      this.scheduleConversationPersist();
      this.$nextTick(() => {
        this.scheduleMessageRenderWindowUpdate();
        this.stabilizeBottomScroll();
      });
    },

    terminalMessageCollapsed: function(msg, idx, rows) {
      if (!msg || !msg.terminal || msg.thinking) return false;
      if (msg._terminal_compact !== true) return false;
      if (msg._terminal_expanded === true) return false;
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      for (var i = idx + 1; i < list.length; i++) {
        var row = list[i];
        if (!row || row.is_notice || row.terminal || row.thinking) continue;
        var hasText = typeof row.text === 'string' && row.text.trim().length > 0;
        var hasTools = Array.isArray(row.tools) && row.tools.length > 0;
        var hasArtifact = !!(row.file_output || row.folder_output);
        if (hasText || hasTools || hasArtifact) return true;
      }
      return false;
    },

    terminalToolboxPreview: function(msg) {
      if (!msg || !msg.terminal) return '';
      var text = String(msg.text || '').trim();
      if (!text) return 'Command completed';
      var first = text.split('\n')[0] || '';
      var compact = first.replace(/\s+/g, ' ').trim();
      if (!compact) return 'Command completed';
      if (compact.length > 108) return compact.slice(0, 105) + '...';
      return compact;
    },
  };
}
