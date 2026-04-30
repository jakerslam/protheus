// Chat websocket phase/progress event handlers.
'use strict';

function infringChatWebSocketPhaseEventMethods() {
  return {
    handleWsPhaseEvent: function(data) {
      var activityProjection = data && (data.agent_activity_projection || data.activity_projection || data.live_activity_projection)
        ? (data.agent_activity_projection || data.activity_projection || data.live_activity_projection)
        : null;
      var optimisticActivityProjection = {
        activity: 'working',
        display_label: 'Working',
        source: 'shell_optimistic',
        optimistic: true
      };
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, activityProjection || optimisticActivityProjection);
      // Show tool/phase progress so the user sees the agent is working.
      var phaseMsg = this.ensureLiveThinkingRow(data);
      if (phaseMsg && (phaseMsg.thinking || phaseMsg.streaming)) {
        if (data && data.workflow_visibility && typeof data.workflow_visibility === 'object') {
          phaseMsg.workflow_visibility = data.workflow_visibility;
          phaseMsg.workflow_trace = data.workflow_visibility.workflow_trace || data.workflow_trace || null;
        } else if (data && data.workflow_trace && typeof data.workflow_trace === 'object') {
          phaseMsg.workflow_trace = data.workflow_trace;
        }
        var statusProjection = data && (
          data.status_phase_projection ||
          data.thinking_bubble_projection ||
          data.context_warning_projection ||
          data.phase_projection ||
          data.workflow_phase_projection
        ) ? (
          data.status_phase_projection ||
          data.thinking_bubble_projection ||
          data.context_warning_projection ||
          data.phase_projection ||
          data.workflow_phase_projection
        ) : null;
        var phasePercent = Number(
          data && data.progress_percent != null
            ? data.progress_percent
            : (data && data.percent != null ? data.percent : NaN)
        );
        if (Number.isFinite(phasePercent)) {
          phaseMsg.progress = {
            percent: Math.max(0, Math.min(100, Math.round(phasePercent))),
            label: data && data.phase ? ('Progress · ' + String(data.phase)) : 'Progress'
          };
        }
        phaseMsg._stream_updated_at = Date.now();
        if (!Number.isFinite(Number(phaseMsg._stream_started_at))) {
          phaseMsg._stream_started_at = Date.now();
        }
        var phaseStatusCandidate = String((statusProjection && (statusProjection.display_label || statusProjection.status_text)) || (data && (data.display_label || data.status_text || data.thinking_status || data.workflow_stage || data.stage || data.phase)) || '').trim();
        var phaseKey = String(data && data.phase ? data.phase : '').trim().toLowerCase();
        if (!phaseStatusCandidate && phaseKey) {
          phaseStatusCandidate = phaseKey.replace(/[_-]+/g, ' ').trim();
        }
        if (typeof this.normalizeThinkingStatusCandidate === 'function') {
          phaseStatusCandidate = this.normalizeThinkingStatusCandidate(phaseStatusCandidate);
        }
        if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(phaseStatusCandidate)) {
          phaseStatusCandidate = '';
        }
        var phaseCurrentStatus = String(phaseMsg.thinking_status || '').trim();
        var phaseCanReplaceStatus = !!phaseStatusCandidate && (
          !phaseCurrentStatus ||
          (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(phaseCurrentStatus))
        );
        var phaseFingerprint = phaseKey + '|' + phaseStatusCandidate + '|' + (Number.isFinite(phasePercent) ? String(Math.round(phasePercent)) : '');
        if (phaseMsg._phase_update_fingerprint === phaseFingerprint) {
          phaseMsg._stream_updated_at = Date.now();
          this._resetTypingTimeout();
          this.scrollToBottom();
          return true;
        }
        phaseMsg._phase_update_fingerprint = phaseFingerprint;
        // Skip phases that have no user-meaningful display text.
        if (phaseKey === 'streaming' || phaseKey === 'done') {
          return true;
        }
        if (phaseStatusCandidate && typeof this._setPendingWsStatusText === 'function') {
          this._setPendingWsStatusText(data && data.agent_id ? String(data.agent_id) : '', phaseStatusCandidate);
        }
        if (phaseKey === 'context_warning') {
          if (phaseStatusCandidate) {
            this.addNoticeEvent({
              notice_label: phaseStatusCandidate,
              notice_type: 'warn',
              ts: Date.now()
            });
            if (phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
          }
        } else if (
          phaseKey === 'thinking' ||
          phaseKey === 'reasoning' ||
          phaseKey === 'analysis' ||
          phaseKey === 'planning' ||
          phaseKey === 'plan'
        ) {
          var thoughtChunk = String(data.detail || '').trim();
          if (thoughtChunk && typeof this.normalizeThinkingStatusCandidate === 'function') {
            thoughtChunk = this.normalizeThinkingStatusCandidate(thoughtChunk);
          }
          if (thoughtChunk) {
            var chunkChanged = phaseMsg._thought_latest_chunk !== thoughtChunk;
            phaseMsg._thought_latest_chunk = thoughtChunk;
            if (chunkChanged) {
              phaseMsg._thoughtText = this.appendThoughtChunk(phaseMsg._thoughtText, thoughtChunk);
              phaseMsg._reasoning = phaseMsg._thoughtText;
              phaseMsg.isHtml = true;
              phaseMsg.thoughtStreaming = true;
              phaseMsg.text = this.renderLiveThoughtHtml(phaseMsg._thoughtText, phaseMsg);
            }
            if (typeof this._setPendingWsStatusText === 'function') {
              this._setPendingWsStatusText(data && data.agent_id ? String(data.agent_id) : '', phaseStatusCandidate || thoughtChunk);
            }
            if (phaseCanReplaceStatus) {
              if (phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
            }
          }
        } else if (phaseMsg.thinking) {
          if (phaseStatusCandidate && phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
        }
        if (phaseStatusCandidate && phaseMsg.status_text !== phaseStatusCandidate) phaseMsg.status_text = phaseStatusCandidate;
        if (phaseCanReplaceStatus) {
          if (phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
        }
      }
      this._resetTypingTimeout();
      this.scrollToBottom();
      return false;
    },
  };
}
