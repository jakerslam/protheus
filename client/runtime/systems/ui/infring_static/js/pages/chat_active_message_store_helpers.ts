// Chat active-message array and store synchronization helpers.
'use strict';

function infringChatActiveMessageStoreMethods() {
  return {
    ensureActiveChatMessagesArray() {
      if (!Array.isArray(this.messages)) this.messages = [];
      return this.messages;
    },
    syncActiveChatMessages() {
      var activeStore = window.InfringChatStore;
      if (activeStore && typeof activeStore.syncMessages === 'function') {
        activeStore.syncMessages(this.messages, this.allFilteredMessages);
      }
      return this.messages;
    },
    replaceActiveChatMessages(rows) {
      this.messages = Array.isArray(rows) ? rows : [];
      this.syncActiveChatMessages();
      return this.messages;
    },
    mutateActiveChatMessages(mutator) {
      var rows = this.ensureActiveChatMessagesArray();
      var nextRows = typeof mutator === 'function' ? mutator(rows) : rows;
      if (Array.isArray(nextRows) && nextRows !== rows) this.messages = nextRows;
      this.syncActiveChatMessages();
      return this.messages;
    },
    appendActiveChatMessage(message) {
      this.ensureActiveChatMessagesArray().push(message);
      this.syncActiveChatMessages();
      return message;
    },
    clearTransientThinkingRowsFallback(options) {
      var opts = options && typeof options === 'object' ? options : {};
      this.messages = this.ensureActiveChatMessagesArray().filter(function(message) {
        if (!message) return false;
        if (opts.preserve_running_tools && message.tool_running) return true;
        if (opts.preserve_pending_ws && message._pending_ws) return true;
        if (opts.thinking_only) return !message.thinking;
        return !message.thinking && !message.streaming;
      });
      return this.messages;
    },
    clearTransientThinkingRowsCompat(options) {
      if (typeof this.clearTransientThinkingRows === 'function') {
        return this.clearTransientThinkingRows(options || { force: true });
      }
      return this.clearTransientThinkingRowsFallback(options || { force: true });
    },
    clearTerminalThinkingRows() {
      this.messages = this.ensureActiveChatMessagesArray().filter(function(message) {
        return !(message && message.terminal && message.thinking);
      });
      return this.messages;
    },
    clearSystemThinkingRows() {
      this.messages = this.ensureActiveChatMessagesArray().filter(function(message) {
        return !message.thinking || message.role !== 'system';
      });
      return this.messages;
    },
  };
}
