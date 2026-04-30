// Chat message workspace panel and metadata shell helpers.
'use strict';

function infringChatMessageWorkspaceMetaMethods() {
  return {
    _workspaceState: function() {
      if (!this._messageWorkspaceState || typeof this._messageWorkspaceState !== 'object') {
        this._messageWorkspaceState = {
          open: false,
          payload: null
        };
      }
      return this._messageWorkspaceState;
    },

    isWorkspacePanelOpen: function() {
      var state = this._workspaceState();
      return !!state.open && !!state.payload;
    },

    closeWorkspacePanel: function() {
      var state = this._workspaceState();
      state.open = false;
      state.payload = null;
    },

    _messageTextPreviewForWorkspace: function(msg) {
      var text = '';
      if (typeof this.extractMessageVisibleText === 'function') {
        text = String(this.extractMessageVisibleText(msg) || '').trim();
      }
      if (!text) text = String(msg && msg.text || '').trim();
      if (text.length > 420) text = text.slice(0, 417).trim() + '...';
      return text;
    },

    _messageArtifactsForWorkspace: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var out = [];
      if (row.file_output && row.file_output.path) {
        out.push({ id: 'file-' + String(row.file_output.path), type: 'File', label: String(row.file_output.path), detail: String(row.file_output.bytes || '') });
      }
      if (row.folder_output && row.folder_output.path) {
        out.push({ id: 'folder-' + String(row.folder_output.path), type: 'Folder', label: String(row.folder_output.path), detail: String(row.folder_output.entries || '') + ' entries' });
      }
      if (Array.isArray(row.images) && row.images.length) {
        out.push({ id: 'images-' + row.images.length, type: 'Images', label: String(row.images.length) + ' uploaded image(s)', detail: '' });
      }
      return out;
    },

    openWorkspacePanelForMessage: function(msg, idx, rows) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var state = this._workspaceState();
      var trace = this.messageToolTraceRows(row);
      state.payload = {
        id: String(row.id || ('msg-' + String(idx || 0))).trim(),
        actor: typeof this.messageActorLabel === 'function' ? this.messageActorLabel(row) : String(row.role || 'Message'),
        timestamp: typeof this.messageTs === 'function' ? this.messageTs(row) : '',
        preview: this._messageTextPreviewForWorkspace(row),
        sources: this.messageSourceChips(row),
        trace: trace,
        artifacts: this._messageArtifactsForWorkspace(row),
        rows_count: Array.isArray(rows) ? rows.length : 0
      };
      state.open = true;
    },

    workspacePanelPayload: function() {
      var state = this._workspaceState();
      if (state.payload && typeof state.payload === 'object') return state.payload;
      return {
        id: '',
        actor: '',
        timestamp: '',
        preview: '',
        sources: [],
        trace: [],
        artifacts: [],
        rows_count: 0
      };
    },

    messageMetadataService: function() {
      var services = typeof InfringSharedShellServices !== 'undefined' ? InfringSharedShellServices : null;
      return services && services.messageMeta ? services.messageMeta : null;
    },

    messageMetadataShellState: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      var model = service && typeof service.viewModel === 'function' ? service.viewModel({
        row: msg,
        index: idx,
        rows: list,
        agent: this.currentAgent,
        shouldRender: typeof this.shouldRenderMessageContent === 'function' ? this.shouldRenderMessageContent(msg, idx, list) : true,
        collapsed: typeof this.isMessageMetaCollapsed === 'function' ? this.isMessageMetaCollapsed(msg, idx, list) : false,
        copied: !!(msg && msg._copied),
        hasTools: typeof this.messageHasTools === 'function' ? this.messageHasTools(msg) : !!(msg && Array.isArray(msg.tools) && msg.tools.length),
        toolsCollapsed: typeof this.allToolsCollapsed === 'function' ? this.allToolsCollapsed(msg) : true,
        timestamp: typeof this.messageTs === 'function' ? this.messageTs(msg) : '',
        responseTimeMs: typeof this.messageStatResponseTimeMs === 'function' ? this.messageStatResponseTimeMs(msg) : 0,
        responseTimeFormatter: typeof this.formatResponseDuration === 'function' ? this.formatResponseDuration.bind(this) : null,
        burnTotalTokens: typeof this.messageStatBurnTotalTokens === 'function' ? this.messageStatBurnTotalTokens(msg) : 0,
        burnFormatter: typeof this.formatTokenK === 'function' ? this.formatTokenK.bind(this) : null
      }) : { shouldRender: false };
      try { return JSON.stringify(model); } catch (_) { return '{"shouldRender":false}'; }
    },

    handleMessageMetaAction: function(event, msg, idx, rows) {
      var action = String(event && event.detail && event.detail.action || '').trim();
      var handlers = {
        copy: this.copyMessage.bind(this, msg),
        report: this.reportIssueFromMeta.bind(this, msg, idx),
        'toggle-tools': this.toggleMessageTools.bind(this, msg),
        retry: this.retryMessageFromMeta.bind(this, msg, idx, rows),
        reply: this.replyToMessageFromMeta.bind(this, msg, idx, rows),
        fork: this.forkMessageFromMeta.bind(this, msg, idx, rows)
      };
      var handler = handlers[action];
      if (typeof handler === 'function') return handler();
    },

    messageCanRetryFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return !!(service && typeof service.canRetry === 'function' && service.canRetry(msg, idx, list));
    },

    _resolveMessageIndexFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return service && typeof service.resolveIndex === 'function' ? service.resolveIndex(msg, idx, list) : -1;
    },

    messageIsLatestAgentFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return !!(service && typeof service.isLatestAgent === 'function' && service.isLatestAgent(msg, idx, list));
    },

    messageCanReplyFromMeta: function(msg, idx, rows) {
      var service = this.messageMetadataService();
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      return !!(service && typeof service.canReply === 'function' && service.canReply(msg, idx, list));
    },

    replyToMessageFromMeta: function(msg, idx, rows) {
      void msg;
      void idx;
      void rows;
      if (typeof InfringToast !== 'undefined') InfringToast.info('Reply requires a backend quote-by-reference contract.');
    },

    messageCanForkFromMeta: function(msg) {
      var service = this.messageMetadataService();
      return !!(service && typeof service.canFork === 'function' && service.canFork(msg, this.currentAgent));
    },

    messageCanReportIssueFromMeta: function(msg) {
      var service = typeof this.messageMetadataService === 'function' ? this.messageMetadataService() : null;
      if (service && typeof service.canReportIssue === 'function') {
        return service.canReportIssue(msg, this.currentAgent);
      }
      return false;
    },
  };
}
