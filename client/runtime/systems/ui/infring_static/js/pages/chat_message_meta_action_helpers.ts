// Chat message metadata API action helpers.
'use strict';

function infringChatMessageMetaActionMethods() {
  return {
    reportIssueFromMeta: async function(msg, idx) {
      if (!this.messageCanReportIssueFromMeta(msg)) return;
      try {
        var result = await InfringAPI.post('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/eval-feedback/report-issue', {
          message_id: String(msg && msg.id || ''),
          message_index: idx
        });
        if (!result || result.ok === false) {
          throw new Error(String((result && (result.error || result.message)) || 'eval_report_failed'));
        }
        if (typeof InfringToast !== 'undefined') InfringToast.success('Eval review queued.');
      } catch (e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to queue eval review: ' + String(e && e.message ? e.message : 'unknown error'));
      }
    },

    retryMessageFromMeta: async function(msg, idx, rows) {
      if (this.sending) return;
      var allowed = this.messageCanRetryFromMeta(msg, idx, rows);
      if (!allowed) return;
      void msg;
      void idx;
      void rows;
      if (typeof InfringToast !== 'undefined') InfringToast.info('Retry requires a backend replay contract.');
    },

    forkMessageFromMeta: async function(msg, idx, rows) {
      if (!this.currentAgent || !this.currentAgent.id || this.sending) return;
      void idx;
      void rows;
      if (typeof this.messageCanForkFromMeta === 'function' && !this.messageCanForkFromMeta(msg)) return;
      var sourceAgent = this.currentAgent && typeof this.currentAgent === 'object' ? this.currentAgent : {};
      var sourceAgentId = String(sourceAgent.id || '').trim();
      if (!sourceAgentId) return;
      try {
        this.cacheCurrentConversation();
        var created = await InfringAPI.post(
          '/api/agents/' + encodeURIComponent(sourceAgentId) + '/clone',
          {}
        );
        var forkedAgentId = String(
          (created && (created.agent_id || created.id)) ||
          ''
        ).trim();
        if (!forkedAgentId) {
          throw new Error('agent_clone_failed');
        }
        var forkedAgentName = String((created && created.name) || forkedAgentId).trim();
        var appStoreBridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var refreshAgents = appStoreBridge && typeof appStoreBridge.method === 'function'
          ? appStoreBridge.method('refreshAgents')
          : null;
        if (typeof refreshAgents === 'function') {
          await refreshAgents({ force: true });
        }
        var resolvedForkedAgent = this.resolveAgent(forkedAgentId);
        if (!resolvedForkedAgent) {
          resolvedForkedAgent = {
            id: forkedAgentId,
            name: forkedAgentName,
            role: String(sourceAgent.role || 'analyst')
          };
        }
        this.selectAgent(resolvedForkedAgent);
        if (typeof InfringToast !== 'undefined') {
          InfringToast.success('Forked to new agent "' + forkedAgentName + '"');
        }
      } catch (e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to fork message: ' + (e && e.message ? e.message : 'unknown error'));
      }
    },
  };
}
