// Chat websocket streaming text delta event handlers.
'use strict';

function infringChatWebSocketTextDeltaEventMethods() {
  return {
    handleWsTextDeltaEvent: function(data) {
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
      var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (last && last.streaming) {
        if (!Number.isFinite(Number(last._stream_started_at))) last._stream_started_at = Date.now();
        if (last._toolTextDetected) return;
        var deltaText = String(data.content || '');
        last._streamRawText = String(last._streamRawText || '') + deltaText;
        last._stream_updated_at = Date.now();
        var streamingSplit = this.extractThinkingLeak(last._streamRawText);
        var visibleText = this.stripModelPrefix(streamingSplit.content || '');
        last._cleanText = visibleText;
        last._thoughtText = streamingSplit.thought || '';
        if (streamingSplit.thought && !visibleText.trim()) {
          this._clearMessageTypewriter(last);
          last.isHtml = true;
          last.thoughtStreaming = true;
          last.text = this.renderLiveThoughtHtml(streamingSplit.thought, last);
        } else {
          if (last.isHtml) last.isHtml = false;
          last.thoughtStreaming = false;
          this._clearMessageTypewriter(last);
          last._typingVisual = false;
          last.text = visibleText;
        }
        var toolScanText = String(last._cleanText || '');
        var fcIdx = toolScanText.search(/\w+<\/function[=,>]/);
        if (fcIdx === -1) fcIdx = toolScanText.search(/<function=\w+>/);
        if (fcIdx !== -1) {
          var fcPart = toolScanText.substring(fcIdx);
          var toolMatch = fcPart.match(/^(\w+)<\/function/) || fcPart.match(/^<function=(\w+)>/);
          var trimmedVisible = toolScanText.substring(0, fcIdx).trim();
          if (streamingSplit.thought && !trimmedVisible) {
            this._clearMessageTypewriter(last);
            last.isHtml = true;
            last.thoughtStreaming = true;
            last.text = this.renderLiveThoughtHtml(streamingSplit.thought, last);
          } else {
            if (last.isHtml) last.isHtml = false;
            last.thoughtStreaming = false;
            this._clearMessageTypewriter(last);
            last.text = trimmedVisible;
          }
          last._cleanText = trimmedVisible;
          last._toolTextDetected = true;
          if (toolMatch) {
            var inputMatch = fcPart.match(/[=,>]\s*(\{[\s\S]*)/);
            var leakTool = this.ensureStreamingToolCard(last, toolMatch[1], inputMatch ? inputMatch[1].replace(/<\/function>?\s*$/, '').trim() : '', { running: true });
            var leakLabel = typeof this.toolThinkingActionLabel === 'function'
              ? this.toolThinkingActionLabel(leakTool || { name: toolMatch[1], input: '' })
              : String(toolMatch[1] || 'tool');
            if (leakLabel && last.thinking_status !== leakLabel) last.thinking_status = leakLabel;
            if (leakLabel && typeof this._setPendingWsStatusText === 'function') {
              this._setPendingWsStatusText(last.agent_id || (this.currentAgent && this.currentAgent.id), leakLabel);
            }
          }
        }
        this.tokenCount = Math.round(String(last._cleanText || '').length / 4);
      } else {
        var firstChunk = this.stripModelPrefix(data.content || '');
        var firstSplit = this.extractThinkingLeak(firstChunk);
        var firstVisible = firstSplit.content || '';
        var firstMessage = {
          id: ++msgId, role: 'agent', text: '', meta: '', thinking: true, streaming: true, thinking_status: '', tools: [],
          _streamRawText: firstChunk, _cleanText: firstVisible, _thoughtText: firstSplit.thought || '',
          _stream_started_at: Date.now(), _stream_updated_at: Date.now(), thoughtStreaming: false, ts: Date.now(),
          agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
          agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
        };
        if (firstSplit.thought && !firstVisible.trim()) {
          firstMessage.isHtml = true;
          firstMessage.thoughtStreaming = true;
          firstMessage.text = this.renderLiveThoughtHtml(firstSplit.thought, firstMessage);
        }
        this.messages.push(firstMessage);
        if (!firstMessage.isHtml) {
          this._clearMessageTypewriter(firstMessage);
          firstMessage._typingVisual = false;
          firstMessage.text = firstVisible;
        }
      }
      this.scrollToBottom();
    },
  };
}
