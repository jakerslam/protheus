// Chat websocket tool lifecycle event handlers.
'use strict';

function infringChatWebSocketToolEventMethods() {
  return {
    handleWsToolStartEvent: function(data) {
      var toolStartAgentId = String(data && data.agent_id ? data.agent_id : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '')).trim();
      if (toolStartAgentId) this.setAgentLiveActivity(toolStartAgentId, 'working');
      var lastMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (!lastMsg || !(lastMsg.thinking || lastMsg.streaming)) {
        lastMsg = {
          id: ++msgId, role: 'agent', text: '', meta: '', thinking: true, streaming: true, thinking_status: '', tools: [],
          _stream_started_at: Date.now(), _stream_updated_at: Date.now(), ts: Date.now(),
          agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
          agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
        };
        this.messages.push(lastMsg);
      }
      lastMsg.thinking = true;
      lastMsg.streaming = true;
      this.ensureStreamingToolCard(lastMsg, data.tool, data.input || '', { running: true, attempt_id: data.attempt_id, attempt_sequence: data.attempt_sequence });
      lastMsg._stream_updated_at = Date.now();
      if (!Number.isFinite(Number(lastMsg._stream_started_at))) lastMsg._stream_started_at = Date.now();
      var receiptStartLabel = String(data && data.tool_status ? data.tool_status : '').trim();
      if (receiptStartLabel && typeof this.normalizeThinkingStatusCandidate === 'function') receiptStartLabel = this.normalizeThinkingStatusCandidate(receiptStartLabel);
      var startLabel = receiptStartLabel || (typeof this.toolThinkingActionLabel === 'function' ? this.toolThinkingActionLabel({ name: data.tool, input: data.input || '' }) : String(data.tool || 'tool'));
      if (startLabel && lastMsg.thinking_status !== startLabel) lastMsg.thinking_status = startLabel;
      if (startLabel && typeof this._setPendingWsStatusText === 'function') this._setPendingWsStatusText(toolStartAgentId, startLabel);
      this._resetTypingTimeout();
      this.scrollToBottom();
    },

    handleWsToolEndEvent: function(data) {
      var toolEndAgentId = String(data && data.agent_id ? data.agent_id : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '')).trim();
      if (toolEndAgentId) this.setAgentLiveActivity(toolEndAgentId, 'working');
      var lastMsg2 = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (lastMsg2) {
        var endedTool = this.ensureStreamingToolCard(lastMsg2, data.tool, data.input || '', { running: false, no_create: true, attempt_id: data.attempt_id, attempt_sequence: data.attempt_sequence });
        if (endedTool) endedTool.running = false;
        var activeToolLabel = typeof this.currentToolDialogLabel === 'function' ? String(this.currentToolDialogLabel(lastMsg2) || '').trim() : '';
        if (activeToolLabel && lastMsg2.thinking_status !== activeToolLabel) {
          lastMsg2.thinking_status = activeToolLabel;
        } else if (!activeToolLabel) {
          lastMsg2.thinking_status = 'Thinking';
        }
        if (typeof this._setPendingWsStatusText === 'function') {
          this._setPendingWsStatusText(toolEndAgentId, lastMsg2.thinking_status || activeToolLabel || 'Thinking');
        }
        lastMsg2._stream_updated_at = Date.now();
        if (!Number.isFinite(Number(lastMsg2._stream_started_at))) lastMsg2._stream_started_at = Date.now();
      }
      this._resetTypingTimeout();
      this.scrollToBottom();
    },

    handleWsToolResultEvent: function(data) {
      var toolResultAgentId = String(data && data.agent_id ? data.agent_id : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '')).trim();
      if (toolResultAgentId) this.setAgentLiveActivity(toolResultAgentId, 'working');
      var lastMsg3 = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (lastMsg3) {
        var resultTool = this.ensureStreamingToolCard(lastMsg3, data.tool, data.input || '', { running: true, attempt_id: data.attempt_id, attempt_sequence: data.attempt_sequence });
        if (resultTool) {
          resultTool.running = false;
          resultTool.result = data.result || '';
          resultTool.is_error = !!data.is_error;
          if ((data.tool === 'image_generate' || data.tool === 'browser_screenshot') && !data.is_error) {
            try {
              var parsed = JSON.parse(data.result);
              if (parsed.image_urls && parsed.image_urls.length) resultTool._imageUrls = parsed.image_urls;
            } catch(e) {}
          }
          if (data.tool === 'text_to_speech' && !data.is_error) {
            try {
              var ttsResult = JSON.parse(data.result);
              if (ttsResult.saved_to) {
                resultTool._audioFile = ttsResult.saved_to;
                resultTool._audioDuration = ttsResult.duration_estimate_ms;
              }
            } catch(e) {}
          }
        }
        lastMsg3._stream_updated_at = Date.now();
        if (!Number.isFinite(Number(lastMsg3._stream_started_at))) lastMsg3._stream_started_at = Date.now();
        var nextActiveToolLabel = typeof this.currentToolDialogLabel === 'function' ? String(this.currentToolDialogLabel(lastMsg3) || '').trim() : '';
        if (nextActiveToolLabel && lastMsg3.thinking_status !== nextActiveToolLabel) {
          lastMsg3.thinking_status = nextActiveToolLabel;
        } else if (!nextActiveToolLabel) {
          lastMsg3.thinking_status = 'Thinking';
        }
        if (typeof this._setPendingWsStatusText === 'function') {
          this._setPendingWsStatusText(toolResultAgentId, lastMsg3.thinking_status || nextActiveToolLabel || 'Thinking');
        }
      }
      this._resetTypingTimeout();
      this.scrollToBottom();
    },
  };
}
