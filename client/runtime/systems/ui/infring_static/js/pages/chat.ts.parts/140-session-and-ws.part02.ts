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
	            var phaseStatusCandidate = phaseDetailText;
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
            var phaseFingerprint = phaseKey + '|' + phaseDetailText + '|' + (Number.isFinite(phasePercent) ? String(Math.round(phasePercent)) : '');
            if (phaseMsg._phase_update_fingerprint === phaseFingerprint) {
              phaseMsg._stream_updated_at = Date.now();
              this._resetTypingTimeout();
              this.scrollToBottom();
              break;
            }
            phaseMsg._phase_update_fingerprint = phaseFingerprint;
            // Skip phases that have no user-meaningful display text — "streaming"
            // and "done" are lifecycle signals, not status to show in the chat bubble.
            if (phaseKey === 'streaming' || phaseKey === 'done') {
              break;
            }
            if (phaseStatusCandidate && typeof this._setPendingWsStatusText === 'function') {
              this._setPendingWsStatusText(data && data.agent_id ? String(data.agent_id) : '', phaseStatusCandidate);
            }
            // Context warning: show prominently as a separate system message
            if (phaseKey === 'context_warning') {
              var cwDetail = data.detail || 'Context limit reached.';
              this.messages.push({ id: ++msgId, role: 'system', text: cwDetail, meta: '', tools: [], system_origin: 'context:warning' });
              if (phaseMsg.thinking_status !== 'Context window warning') phaseMsg.thinking_status = 'Context window warning';
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
                if (!phaseMsg.thinking_status && phaseStatusCandidate) {
                  if (phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
                }
              }
            } else if (phaseMsg.thinking) {
              if (phaseStatusCandidate && phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
            }
            if (!phaseMsg.thinking_status && phaseStatusCandidate) {
              if (phaseMsg.thinking_status !== phaseStatusCandidate) phaseMsg.thinking_status = phaseStatusCandidate;
            }
	          }
	          this._resetTypingTimeout();
	          this.scrollToBottom();
	          break;
