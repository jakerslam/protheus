                ? data.progress_percent
                : (data && data.percent != null ? data.percent : NaN)
            );
            if (Number.isFinite(phasePercent)) {
              phaseMsg.progress = {
                percent: Math.max(0, Math.min(100, Math.round(phasePercent))),
                label: data && data.phase ? ('Progress · ' + String(data.phase)) : 'Progress'
              };
            }
            // Skip phases that have no user-meaningful display text — "streaming"
            // and "done" are lifecycle signals, not status to show in the chat bubble.
            if (data.phase === 'streaming' || data.phase === 'done') {
              break;
            }
            // Context warning: show prominently as a separate system message
            if (data.phase === 'context_warning') {
              var cwDetail = data.detail || 'Context limit reached.';
              this.messages.push({ id: ++msgId, role: 'system', text: cwDetail, meta: '', tools: [], system_origin: 'context:warning' });
              phaseMsg.thinking_status = 'Context window warning';
            } else if (data.phase === 'thinking') {
              var thoughtChunk = String(data.detail || '').trim();
              if (thoughtChunk) {
                phaseMsg._thoughtText = this.appendThoughtChunk(phaseMsg._thoughtText, thoughtChunk);
                phaseMsg._reasoning = phaseMsg._thoughtText;
                phaseMsg.isHtml = true;
                phaseMsg.thoughtStreaming = true;
                phaseMsg.text = this.renderLiveThoughtHtml(phaseMsg._thoughtText);
                phaseMsg.thinking_status = 'Reasoning through context...';
              } else if (phaseMsg.thinking) {
                phaseMsg.text = 'Thinking...';
                if (!phaseMsg.thinking_status) phaseMsg.thinking_status = 'Reasoning through context...';
              }
            } else if (phaseMsg.thinking) {
              // Only update text on messages still in thinking state (not yet
              // receiving streamed content) to avoid overwriting accumulated text.
              var phaseDetail;
              if (data.phase === 'tool_use') {
                phaseDetail = 'Using ' + (data.detail || 'tool') + '...';
              } else if (data.phase === 'thinking') {
                phaseDetail = 'Thinking...';
              } else {
                phaseDetail = data.detail || 'Working...';
              }
              phaseMsg.text = phaseDetail;
              if (phaseName === 'tool_use') {
                var toolPhaseName = phaseDetail || String(data && data.tool ? data.tool : '').trim() || 'tool';
                phaseMsg.thinking_status = 'Calling ' + toolPhaseName + '...';
              } else if (phaseDetail) {
                phaseMsg.thinking_status = phaseDetail;
              } else if (phaseName) {
                phaseMsg.thinking_status = phaseName.replace(/[_-]+/g, ' ');
              }
            }
            if (!phaseMsg.thinking_status && phaseDetailText) {
              phaseMsg.thinking_status = phaseDetailText;
            }
          }
          this.scrollToBottom();
          break;
