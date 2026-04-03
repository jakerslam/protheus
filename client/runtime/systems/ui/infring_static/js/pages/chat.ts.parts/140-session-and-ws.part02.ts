                ? data.progress_percent
                : (data && data.percent != null ? data.percent : NaN)
            );
            if (Number.isFinite(phasePercent)) {
              phaseMsg.progress = {
                percent: Math.max(0, Math.min(100, Math.round(phasePercent))),
                label: data && data.phase ? ('Progress · ' + String(data.phase)) : 'Progress'
              };
            }
            var phaseStatusCandidate = phaseDetailText;
            if (typeof this.isThinkingPlaceholderText === 'function' && this.isThinkingPlaceholderText(phaseStatusCandidate)) {
              phaseStatusCandidate = '';
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
                if (!phaseMsg.thinking_status && phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
              }
            } else if (phaseMsg.thinking) {
              if (phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
            }
            if (!phaseMsg.thinking_status && phaseStatusCandidate) {
              phaseMsg.thinking_status = phaseStatusCandidate;
            }
          }
          this.scrollToBottom();
          break;
