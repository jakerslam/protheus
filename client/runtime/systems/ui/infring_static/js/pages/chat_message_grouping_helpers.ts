// Chat message grouping, metadata visibility, and tail display helpers.
'use strict';

function infringChatMessageGroupingMethods() {
  return {
    showMessageTitle(msg, idx, rows) {
      if (!msg || msg.is_notice) return false;
      if (msg.terminal) return this.isFirstInSourceRun(idx, rows);
      var role = String(msg.role || '').toLowerCase();
      if (role !== 'agent' && role !== 'system' && role !== 'user') return false;
      return this.isFirstInSourceRun(idx, rows);
    },
    messageMetaVisible(msg, idx, rows) {
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      if (service && typeof service.visible === 'function') {
        return service.visible(msg, this.isMessageMetaCollapsed(msg, idx, rows));
      }
      return !!(msg && !msg.is_notice && !msg.thinking && !this.isMessageMetaCollapsed(msg, idx, rows));
    },
    isMessageMetaCollapsed(msg, idx, rows) {
      if (!msg || msg.is_notice || msg.thinking) return true;
      return !this.isDirectHoveredMessage(msg, idx);
    },
    isGrouped(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx <= 0 || idx >= list.length) return false;
      var prev = list[idx - 1];
      var curr = list[idx];
      if (!prev || !curr || prev.is_notice || curr.is_notice) return false;
      if (curr.thinking || prev.thinking) return false;
      return !this.isFirstInSourceRun(idx, list);
    },
    messageHasTailBlockingBox(msg) {
      if (!msg || typeof msg !== 'object') return false;
      if (this.messageHasTools(msg)) return true;
      if (msg.file_output && msg.file_output.path) return true;
      if (msg.folder_output && msg.folder_output.path) return true;
      if (this.messageProgress(msg)) return true;
      return false;
    },
    showMessageTail(msg, idx, rows) {
      if (!msg || msg.is_notice) return false;
      var role = this.messageGroupRole(msg);
      if (role !== 'user' && role !== 'agent' && role !== 'system') return false;
      // Tail only shows when this bubble is the terminal visible item in its source run.
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return true;
      return this.isLastInSourceRun(idx, list);
    },
  };
}
