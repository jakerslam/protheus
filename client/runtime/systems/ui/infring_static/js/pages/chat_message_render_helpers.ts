// Chat message search highlighting, line-window, and bubble render helpers.
'use strict';

function infringChatMessageRenderMethods() {
  return {
    highlightSearch: function(html) {
      if (!this.searchQuery.trim() || !html) return html;
      var q = this.searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      var regex = new RegExp('(' + q + ')', 'gi');
      return html.replace(regex, '<mark style="background:var(--warning);color:var(--bg);border-radius:2px;padding:0 2px">$1</mark>');
    },

    messageVisibleLineWindow: function(msg, idx) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var text = String(row.text || '');
      if (!text || row._typingVisual || row.isHtml) return { text: text, truncated: false, key: '', shown: 0, total: 0 };
      var step = Number(this.messageLineExpandStep || 20);
      if (!Number.isFinite(step) || step < 20) step = 20;
      var key = String(row.id || '').trim() || ('line:' + String(row.ts || '') + ':' + String(idx || 0));
      if (!this.messageLineExpandState || typeof this.messageLineExpandState !== 'object') this.messageLineExpandState = {};
      var shown = Number(this.messageLineExpandState[key] || step);
      if (!Number.isFinite(shown) || shown < step) shown = step;
      var lines = text.split(/\r?\n/);
      var total = lines.length;
      if (shown >= total) return { text: text, truncated: false, key: key, shown: total, total: total };
      return { text: lines.slice(0, shown).join('\n'), truncated: true, key: key, shown: shown, total: total };
    },

    // Backward-compat shim for legacy callers during naming migration.
    messageLineWindow: function(msg, idx) {
      return this.messageVisibleLineWindow(msg, idx);
    },

    messageHasLineOverflow: function(msg, idx) {
      return !!this.messageVisibleLineWindow(msg, idx).truncated;
    },

    expandMessageLines: function(msg, idx) {
      var window = this.messageVisibleLineWindow(msg, idx);
      if (!window.truncated || !window.key) return;
      var step = Number(this.messageLineExpandStep || 20);
      if (!Number.isFinite(step) || step < 20) step = 20;
      this.messageLineExpandState[window.key] = Math.min(window.total, window.shown + step);
    },

    messageBubbleHtml: function(msg, idx) {
      if (!msg || typeof msg !== 'object') return '';
      if (msg._typingVisual) {
        if (typeof msg._typingVisualHtml === 'string' && msg._typingVisualHtml.trim()) {
          return msg._typingVisualHtml;
        }
        return this.escapeHtml(String(msg.text || ''));
      }
      var lineWindow = this.messageVisibleLineWindow(msg, idx);
      var displayText = String(lineWindow.text || '');
      var baseHtml = '';
      if (msg.isHtml) {
        baseHtml = String(displayText || '');
      } else if ((msg.role === 'agent' || msg.role === 'system') && !msg.thinking) {
        baseHtml = this.renderMarkdown(String(displayText || ''));
      } else {
        baseHtml = this.escapeHtml(String(displayText || ''));
      }
      return this.highlightSearch(baseHtml);
    },
    messageTypingReserveStyle: function(msg) {
      if (!msg || typeof msg !== 'object' || !msg._typingVisual) return '';
      var finalText = String(msg._typewriterFinalText || msg.text || '');
      if (!finalText.trim()) return '--typing-reserve-height:72px;';
      var hardLines = finalText.split(/\r?\n/).length;
      var softWrapLines = Math.ceil(Math.max(0, finalText.length - (hardLines * 20)) / 92);
      var visualLines = Math.max(1, hardLines + softWrapLines);
      var reserveHeight = 20 + (visualLines * 25);
      reserveHeight = Math.max(72, Math.min(980, Math.round(reserveHeight)));
      return '--typing-reserve-height:' + reserveHeight + 'px;';
    },
  };
}
