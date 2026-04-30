// Chat notice and system-message projection helpers.
'use strict';

function infringChatNoticeMessageMethods() {
  return {
    appendChatSideResultNotice: function(payload) {
      var parsed = this.parseChatSideResult(payload);
      if (!parsed) return false;
      this.addNoticeEvent({
        notice_label: 'Background note: ' + parsed.question,
        notice_type: parsed.isError ? 'warn' : 'info',
        notice_detail: parsed.text,
        run_id: parsed.runId,
        session_key: parsed.sessionKey,
        ts: parsed.ts
      });
      return true;
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
        role: 'notice',
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
      var rawText = String(payload && payload.text ? payload.text : '');
      var text = this.normalizeSystemMessageText
        ? this.normalizeSystemMessageText(rawText)
        : rawText.trim();
      if (!text) return null;
      var canonicalText = text.replace(/\s+/g, ' ').trim().toLowerCase();
      if (/^error:\s*/i.test(canonicalText) && canonicalText.indexOf('operation was aborted') >= 0) return null;
      if (!Array.isArray(this.systemTelemetry)) this.systemTelemetry = [];
      this.systemTelemetry.push({
        text: text,
        origin: payload.system_origin || payload.systemOrigin || '',
        ts: Date.now()
      });
      return null;
    },

    emitCommandFailureNotice: function(command, error, fallbackCommands) {
      var cmd = String(command || '').trim() || '/status';
      var message = String(error && error.message ? error.message : error || 'command_failed').trim();
      if (message.length > 220) message = message.slice(0, 217) + '...';
      var fallbacks = Array.isArray(fallbackCommands) ? fallbackCommands : [];
      var fallbackText = fallbacks
        .map(function(row) { return '`' + String(row || '').trim() + '`'; })
        .filter(Boolean)
        .join(' · ');
      this.addNoticeEvent({
        notice_label:
          'Command `' + cmd + '` failed: ' + message +
          (fallbackText ? ('; try recovery: ' + fallbackText) : ''),
        notice_type: 'warn',
        ts: Date.now()
      });
    },
  };
}
