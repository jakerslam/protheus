// Chat queued prompt and terminal dispatch helpers.
'use strict';

function infringChatQueueProcessingMethods() {
  return {
    _processQueue: function() {
      if (!this.messageQueue.length || this.sending || this._inflightFailoverInProgress) return;
      var next = this.messageQueue.shift();
      if (next && next.terminal) {
        this._sendTerminalPayload(next.command);
        return;
      }
      var nextText = String(next && next.text ? next.text : '');
      var nextFiles = Array.isArray(next && next.files) ? next.files : [];
      var nextImages = Array.isArray(next && next.images) ? next.images : [];
      if (!nextText.trim() && !nextFiles.length) {
        var self = this;
        this.$nextTick(function() { self._processQueue(); });
        return;
      }
      this.inputText = nextText;
      if (nextImages.length && typeof this.addNoticeEvent === 'function') {
        this.addNoticeEvent({
          notice_label: 'Queued prompt image attachments are ready for manual send.',
          notice_type: 'info',
          ts: Date.now()
        });
      }
      if (typeof this.addNoticeEvent === 'function') {
        this.addNoticeEvent({
          notice_label: 'Queued prompt moved to composer for manual send.',
          notice_type: 'info',
          ts: Date.now()
        });
      }
      this.scheduleConversationPersist();
    },
  };
}
