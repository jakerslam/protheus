        case 'text_delta':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
          var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (last && last.streaming) {
            if (!Number.isFinite(Number(last._stream_started_at))) last._stream_started_at = Date.now();
            if (last._toolTextDetected) break;
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
              this._queueStreamTypingRender(last, visibleText);
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
                var leakTool = this.ensureStreamingToolCard(
                  last,
                  toolMatch[1],
                  inputMatch ? inputMatch[1].replace(/<\/function>?\s*$/, '').trim() : '',
                  { running: true }
                );
                var leakLabel = typeof this.toolThinkingActionLabel === 'function'
                  ? this.toolThinkingActionLabel(leakTool || { name: toolMatch[1], input: '' })
                  : String(toolMatch[1] || 'tool');
                if (leakLabel) last.thinking_status = leakLabel;
              }
            }
            this.tokenCount = Math.round(String(last._cleanText || '').length / 4);
          } else {
            var firstChunk = this.stripModelPrefix(data.content || '');
            var firstSplit = this.extractThinkingLeak(firstChunk);
            var firstVisible = firstSplit.content || '';
            var firstMessage = {
              id: ++msgId,
              role: 'agent',
              text: '',
              meta: '',
              thinking: true,
              streaming: true,
              thinking_status: '',
              tools: [],
              _streamRawText: firstChunk,
              _cleanText: firstVisible,
              _thoughtText: firstSplit.thought || '',
              _stream_started_at: Date.now(),
              _stream_updated_at: Date.now(),
              thoughtStreaming: false,
              ts: Date.now(),
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
              this._queueStreamTypingRender(firstMessage, firstVisible);
            }
          }
          this.scrollToBottom();
          break;
        case 'tool_start':
          var lastMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (!lastMsg || !(lastMsg.thinking || lastMsg.streaming)) {
            lastMsg = {
              id: ++msgId,
              role: 'agent',
              text: '',
              meta: '',
              thinking: true,
              streaming: true,
              thinking_status: '',
              tools: [],
              _stream_started_at: Date.now(),
              _stream_updated_at: Date.now(),
              ts: Date.now(),
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            };
            this.messages.push(lastMsg);
          }
          lastMsg.thinking = true;
          lastMsg.streaming = true;
          this.ensureStreamingToolCard(lastMsg, data.tool, data.input || '', { running: true });
          lastMsg._stream_updated_at = Date.now();
          if (!Number.isFinite(Number(lastMsg._stream_started_at))) lastMsg._stream_started_at = Date.now();
          var receiptStartLabel = String(data && data.tool_status ? data.tool_status : '').trim();
          if (receiptStartLabel && typeof this.normalizeThinkingStatusCandidate === 'function') receiptStartLabel = this.normalizeThinkingStatusCandidate(receiptStartLabel);
          var startLabel = receiptStartLabel || (typeof this.toolThinkingActionLabel === 'function' ? this.toolThinkingActionLabel({ name: data.tool, input: data.input || '' }) : String(data.tool || 'tool'));
          if (startLabel) lastMsg.thinking_status = startLabel;
          this._resetTypingTimeout();
          this.scrollToBottom();
          break;
        case 'tool_end':
          var lastMsg2 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg2) {
            var runningTool = this.ensureStreamingToolCard(lastMsg2, data.tool, data.input || '', { running: true });
            var receiptRunningLabel = String(data && data.tool_status ? data.tool_status : '').trim();
            if (receiptRunningLabel && typeof this.normalizeThinkingStatusCandidate === 'function') receiptRunningLabel = this.normalizeThinkingStatusCandidate(receiptRunningLabel);
            var runningLabel = receiptRunningLabel || (typeof this.toolThinkingActionLabel === 'function' ? this.toolThinkingActionLabel(runningTool || { name: data.tool, input: data.input || '' }) : String(data.tool || 'tool'));
            if (runningLabel) lastMsg2.thinking_status = runningLabel;
            lastMsg2._stream_updated_at = Date.now();
            if (!Number.isFinite(Number(lastMsg2._stream_started_at))) lastMsg2._stream_started_at = Date.now();
          }
          this._resetTypingTimeout();
          this.scrollToBottom();
          break;
        case 'tool_result':
          var lastMsg3 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg3) {
            var resultTool = this.ensureStreamingToolCard(lastMsg3, data.tool, data.input || '', { running: true });
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
            var statusSummary = typeof this.thinkingToolStatusSummary === 'function'
              ? this.thinkingToolStatusSummary(lastMsg3)
              : null;
            var summaryText = String((statusSummary && statusSummary.text) || '').trim();
            if (summaryText) lastMsg3.thinking_status = summaryText;
          }
          this._resetTypingTimeout();
          this.scrollToBottom();
          break;
        case 'response':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this.applyContextTelemetry(data);
          var wsAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim(); if (!wsAutoSwitchPrevious) wsAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
          var wsRoute = this.applyAutoRouteTelemetry(data);
          var envelope = this.collectStreamedAssistantEnvelope();
          var streamedText = envelope.text;
          var streamedTools = envelope.tools;
          var streamedThought = envelope.thought;
          var responseTools = Array.isArray(data.tools) ? data.tools.map(function(t, idx) {
            return {
              id: (t && t.id) || ('ws-tool-' + Date.now() + '-' + idx),
              name: (t && t.name) || (t && t.tool) || 'tool',
              running: false,
              expanded: false,
              input: (t && t.input) || (t && t.arguments) || '',
              result: (t && t.result) || (t && t.output) || '',
              is_error: !!(t && (t.is_error || t.error || t.blocked))
            };
          }) : [];
          if ((!Array.isArray(streamedTools) || !streamedTools.length) && responseTools.length) streamedTools = responseTools;
          if (!streamedThought && responseTools.length) {
            var thoughtTool = responseTools.find(function(rtool) {
              return !!(rtool && String(rtool.name || '').toLowerCase() === 'thought_process');
            });
            if (thoughtTool) streamedThought = String(thoughtTool.input || thoughtTool.result || '').trim();
          }
          streamedTools.forEach(function(t) {
            t.running = false;
            if (t.id && t.id.indexOf('-txt-') !== -1 && !t.result) {
              t.result = 'Model attempted this call as text (not executed via tool system)';
              t.is_error = true;
            }
          });
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          var meta = (data.input_tokens || 0) + ' in / ' + (data.output_tokens || 0) + ' out';
          if (data.cost_usd != null) meta += ' | $' + data.cost_usd.toFixed(4);
          if (data.iterations) meta += ' | ' + data.iterations + ' iter';
          if (data.fallback_model) meta += ' | fallback: ' + data.fallback_model;
          var wsDurationMs = Number(data.duration_ms || data.elapsed_ms || data.response_ms || 0);
          if (!wsDurationMs && this._responseStartedAt) {
            wsDurationMs = Math.max(0, Date.now() - this._responseStartedAt);
          }
          var wsDuration = this.formatResponseDuration(wsDurationMs); if (wsDuration) meta += ' | ' + wsDuration;
          var wsRouteMeta = this.formatAutoRouteMeta(wsRoute);
          if (wsRouteMeta) meta += ' | ' + wsRouteMeta;
          var finalText = (data.content && data.content.trim()) ? data.content : streamedText;
          finalText = this.stripModelPrefix(finalText);
          var artifactDirectives = this.extractArtifactDirectives(finalText);
          var finalSplit = this.extractThinkingLeak(finalText);
          if (finalSplit.thought) {
            if (!streamedThought) {
              streamedThought = finalSplit.thought;
            } else if (streamedThought.indexOf(finalSplit.thought) === -1) {
              streamedThought += '\n' + finalSplit.thought;
            }
            finalText = finalSplit.content || '';
          }
          finalText = this.sanitizeToolText(finalText);
          finalText = this.stripArtifactDirectivesFromText(finalText);
          var collapsedThought = String(streamedThought || '').trim();
          var compactFinal = String(finalText || '').replace(/\s+/g, ' ').trim();
          var maybePlaceholder = /^(thinking|processing|working)\.\.\.$/i.test(compactFinal);
          if (
            typeof this.isThinkingPlaceholderText === 'function' &&
            this.isThinkingPlaceholderText(compactFinal)
          ) {
            maybePlaceholder = true;
          }
          if (maybePlaceholder) {
            finalText = '';
          }
          if (collapsedThought && !streamedTools.some(function(tool) { return !!(tool && String(tool.name || '').toLowerCase() === 'thought_process'); })) {
            streamedTools.unshift(this.makeThoughtToolCard(collapsedThought, wsDurationMs));
          }
          var usedFallback = false;
          if (!finalText.trim()) {
            finalText = this.defaultAssistantFallback(collapsedThought, streamedTools);
            usedFallback = true;
          }
          var finalMessage = {
            id: ++msgId,
            role: 'agent',
            text: finalText,
            meta: meta,
            tools: streamedTools,
            ts: Date.now(),
            _auto_fallback: usedFallback,
            agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
            agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
          };
          var renderedFinalMessage = finalMessage;
          var lastStable = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (!usedFallback && lastStable && lastStable.role === 'agent' && lastStable._auto_fallback) {
            this.messages[this.messages.length - 1] = finalMessage;
            renderedFinalMessage = finalMessage;
          } else {
            renderedFinalMessage = this.pushAgentMessageDeduped(finalMessage, { dedupe_window_ms: 90000 }) || finalMessage;
          }
          this.markAgentMessageComplete(renderedFinalMessage);
          var wsFailure = this.extractRecoverableBackendFailure(finalText);
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this.scrollToBottom();
          this.requestContextTelemetry(false);
          this.maybeAddAutoModelSwitchNotice(wsAutoSwitchPrevious, wsRoute);
          this._pendingAutoModelSwitchBaseline = '';
          if (artifactDirectives && artifactDirectives.length) {
            this.resolveArtifactDirectives(artifactDirectives);
          }
          var self3 = this;
          if (wsFailure) {
            this.attemptAutomaticFailoverRecovery('ws_response', finalText, {
              remove_last_agent_failure: true
            }).then(function(recovered) {
              if (recovered) return;
              self3._inflightPayload = null;
              self3.refreshPromptSuggestions(true, 'post-response-failed-recover');
              self3.$nextTick(function() {
                var el = document.getElementById('msg-input'); if (el) el.focus();
                self3._processQueue();
              });
            });
          } else {
            this._inflightPayload = null;
            this.refreshPromptSuggestions(true, 'post-response');
            this.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              self3._processQueue();
            });
          }
          break;

        case 'silent_complete':
          // Agent intentionally chose not to reply (NO_REPLY)
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this._inflightPayload = null;
          this._pendingAutoModelSwitchBaseline = '';
          var nowTs = Date.now();
          var hasRecentSubstantiveAgentReply = false;
          for (var si = this.messages.length - 1; si >= 0; si--) {
            var stable = this.messages[si];
            if (!stable) continue;
            if (stable.thinking || stable.streaming) continue;
            if (String(stable.role || '').toLowerCase() !== 'agent') continue;
            var stableText = String(stable.text || '').trim();
            if (!stableText) continue;
            if (stable._auto_fallback) continue;
            var stableAge = Math.max(0, nowTs - Number(stable.ts || nowTs));
            if (stableAge <= 20000) {
              hasRecentSubstantiveAgentReply = true;
            }
            break;
          }
          if (hasRecentSubstantiveAgentReply) {
            this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            var selfSilentSkip = this;
            this.$nextTick(function() { selfSilentSkip._processQueue(); });
            this.refreshPromptSuggestions(true, 'post-silent-skip');
            break;
          }
          var silentEnvelope = this.collectStreamedAssistantEnvelope();
          var silentThought = String(silentEnvelope.thought || '').trim();
          var silentTools = silentEnvelope.tools || [];
          if (silentThought) {
            silentTools.unshift(this.makeThoughtToolCard(silentThought, Number(data && data.duration_ms ? data.duration_ms : 0)));
          }
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.messages.push({
            id: ++msgId,
            role: 'agent',
            text: this.defaultAssistantFallback(silentThought, silentTools),
            meta: '',
            tools: silentTools,
            ts: Date.now(),
            _auto_fallback: true,
            agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
            agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
          });
          this.markAgentMessageComplete(this.messages[this.messages.length - 1]);
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          var selfSilent = this;
          this.$nextTick(function() { selfSilent._processQueue(); });
          this.refreshPromptSuggestions(true, 'post-silent');
          break;

        case 'error':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this._pendingAutoModelSwitchBaseline = '';
          var rawError = String(data && data.content ? data.content : 'unknown_error');
          var errorText = 'Error: ' + rawError;
          var lowerError = rawError.toLowerCase();
          if (
            lowerError.indexOf('this operation was aborted') >= 0 ||
            lowerError.indexOf('operation was aborted') >= 0
          ) {
            this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            this._inflightPayload = null;
            this.refreshPromptSuggestions(true, 'post-ws-abort');
            break;
          }
          if (lowerError.indexOf('backend_http_404') >= 0) {
            // Soft-ignore noisy command-surface 404s so they do not get injected
            // into the conversation stream after a successful agent response.
            this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            this._inflightPayload = null;
            this.requestContextTelemetry(false);
            var selfSuppressed = this;
            this.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              selfSuppressed._processQueue();
            });
            this.refreshPromptSuggestions(true, 'post-suppressed-404');
            break;
          }
          if (lowerError.indexOf('agent contract terminated') !== -1 || lowerError.indexOf('agent_contract_terminated') !== -1) {
            this.handleAgentInactive(
              this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
              'contract_terminated',
              { noticeText: errorText }
            );
            break;
          }
          if (lowerError.indexOf('agent is inactive') !== -1 || lowerError.indexOf('agent_inactive') !== -1) {
            this.handleAgentInactive(
              this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
              'inactive',
              { noticeText: errorText }
            );
            break;
          }
          if (lowerError.indexOf('agent not found') !== -1 || lowerError.indexOf('agent_not_found') !== -1) {
            this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            this.sending = false;
            this._responseStartedAt = 0;
            this.tokenCount = 0;
            var priorAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
            var inflight = this._inflightPayload && typeof this._inflightPayload === 'object' ? this._inflightPayload : null;
            var rawNotFound = rawError;
            var selfRebound = this;
            Promise.resolve()
              .then(function() {
                return selfRebound.rebindCurrentAgentAuthoritative({
                  preferred_id: priorAgentId,
                  clear_when_missing: true
                });
              })
              .then(function(reboundAgent) {
                var reboundAgentId = reboundAgent && reboundAgent.id ? String(reboundAgent.id) : '';
                if (
                  reboundAgentId &&
                  reboundAgentId !== priorAgentId &&
                  inflight &&
                  !inflight._agent_rebind_attempted
                ) {
                  inflight._agent_rebind_attempted = true;
                  inflight.agent_id = reboundAgentId;
                  selfRebound.addNoticeEvent({
                    notice_label:
                      'Active agent reference expired. Switched to ' +
                      String(reboundAgent.name || reboundAgent.id || reboundAgentId) +
                      ' and retried.',
                    notice_type: 'warn',
                    ts: Date.now(),
                  });
                  return selfRebound._sendPayload(
                    inflight.final_text || '',
                    Array.isArray(inflight.uploaded_files) ? inflight.uploaded_files : [],
                    Array.isArray(inflight.msg_images) ? inflight.msg_images : [],
                    { agent_id: reboundAgentId, retry_from_agent_rebind: true }
                  );
                }
                return selfRebound
                  .attemptAutomaticFailoverRecovery('ws_error', rawNotFound, {
                    remove_last_agent_failure: false
                  })
                  .then(function(recovered) {
                    if (recovered) return;
                    selfRebound.pushSystemMessage({
                      text: 'Error: ' + rawNotFound,
                      meta: '',
                      tools: [],
                      system_origin: 'ws:error',
                      ts: Date.now(),
                      dedupe_window_ms: 12000
                    });
                    selfRebound._inflightPayload = null;
                  });
              })
              .catch(function() {});
            break;
          }
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          var self2 = this;
          this.attemptAutomaticFailoverRecovery('ws_error', rawError, {
            remove_last_agent_failure: false
          }).then(function(recovered) {
            if (recovered) return;
            self2.pushSystemMessage({
