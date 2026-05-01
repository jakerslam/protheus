// Chat agent inactive/stop lifecycle projection helpers.
'use strict';

function infringChatAgentLifecycleMethods() {
  return {
    handleAgentInactive: function(agentId, reason, options) {
      var opts = options || {};
      var targetId = String(agentId || (this.currentAgent && this.currentAgent.id) || '').trim();
      if (
        (targetId && this.isSystemThreadId && this.isSystemThreadId(targetId)) ||
        (!targetId && this.isSystemThreadActive && this.isSystemThreadActive())
      ) {
        if (!this.currentAgent || !this.isSystemThreadAgent || !this.isSystemThreadAgent(this.currentAgent)) {
          this.activateSystemThread({ preserve_if_empty: true });
        } else {
          this.currentAgent = this.makeSystemThreadAgent();
          this.setStoreActiveAgentId(this.currentAgent.id || null);
        }
        return;
      }
      if (!opts.force && this.shouldSuppressAgentInactive(targetId)) {
        return;
      }
      var reasonLabel = this.formatInactiveReason(reason || 'inactive');
      var noticeKey = targetId + '|' + reasonLabel;
      var self = this;

      this._clearTypingTimeout();
      this._clearPendingWsRequest(targetId);
      this.clearTransientThinkingRowsCompat({ force: true });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._inflightPayload = null;
      this.setAgentLiveActivity(targetId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle');

      if (!opts.silentNotice && noticeKey !== this._lastInactiveNoticeKey) {
        var noticeText = opts.noticeText || '';
        if (!noticeText) {
          noticeText = targetId
            ? ('Agent ' + targetId + ' is now inactive (' + reasonLabel + ').')
            : ('Agent is now inactive (' + reasonLabel + ').');
        }
        this.addNoticeEvent({
          notice_label: noticeText,
          notice_type: 'warn',
          ts: Date.now()
        });
        this._lastInactiveNoticeKey = noticeKey;
      }

      if (targetId && this._wsAgent && String(this._wsAgent) === targetId) {
        InfringAPI.wsDisconnect();
        this._wsAgent = null;
      }

      if (this.currentAgent && this.currentAgent.id && (!targetId || String(this.currentAgent.id) === targetId)) {
        this.currentAgent = null;
        this.setStoreActiveAgentId(null);
        this.showAgentDrawer = false;
      }

      this.scrollToBottom();
      this.$nextTick(function() { self._processQueue(); });

      this.refreshAgentRosterFromShellStore();
    },

    handleStopResponse: function(agentId, payload) {
      var result = payload && typeof payload === 'object' ? payload : {};
      var reasonRaw = String(result.reason || result.error || '').trim();
      var reason = reasonRaw || (result.contract_terminated ? 'contract_terminated' : '');
      var state = String(result.state || '').trim().toLowerCase();
      var reasonLower = reason.toLowerCase();
      var isInactive =
        !!result.archived ||
        !!result.contract_terminated ||
        state === 'inactive' ||
        state === 'archived' ||
        state === 'terminated' ||
        String(result.type || '').toLowerCase() === 'agent_archived' ||
        reasonLower.indexOf('inactive') >= 0 ||
        reasonLower.indexOf('terminated') >= 0;

      if (isInactive) {
        this.handleAgentInactive(
          agentId,
          reason || (result.contract_terminated ? 'contract_terminated' : 'inactive'),
          result.message ? { noticeText: String(result.message) } : {}
        );
        return;
      }

      this.setAgentLiveActivity(agentId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle');
      this._clearTypingTimeout();
      this.clearTransientThinkingRowsCompat({ force: true });
      this.addNoticeEvent({
        notice_label: result.message || 'Run cancelled',
        notice_type: 'info',
        ts: Date.now()
      });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() { self._processQueue(); });
      this.refreshAgentRosterFromShellStore();
    },
  };
}
