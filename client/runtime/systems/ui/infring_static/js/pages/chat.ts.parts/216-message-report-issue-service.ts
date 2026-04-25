    messageCanReportIssueFromMeta: function(msg) {
      var services = typeof InfringSharedShellServices !== 'undefined' ? InfringSharedShellServices : null;
      if (services && services.message && typeof services.message.canRequestEvalIssueReport === 'function') {
        return services.message.canRequestEvalIssueReport(msg, this.currentAgent);
      }
      if (!this.currentAgent || !this.currentAgent.id) return false;
      if (typeof this.messageIsAgentOrigin === 'function' && !this.messageIsAgentOrigin(msg)) return false;
      if (!msg || msg.thinking || msg.terminal || msg.is_notice) return false;
      return !!((msg.text && String(msg.text).trim()) || (msg.tools && msg.tools.length) || msg.meta || msg.ts);
    },

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
