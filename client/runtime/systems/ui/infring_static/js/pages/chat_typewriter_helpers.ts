function infringChatTypewriterMethods() {
  return {
    // Keep thinking indicators alive while work is still in-flight.
    // Only hard-timeout once no pending activity remains or the request is
    // genuinely stale far beyond expected runtime.
    _resetTypingTimeout: function() {
      var self = this;
      if (self._typingTimeout) clearTimeout(self._typingTimeout);
      self._typingTimeout = setTimeout(function() {
        var hasPending = typeof self.hasLivePendingResponse === 'function'
          ? self.hasLivePendingResponse()
          : false;
        var hardStale = typeof self.pendingResponseExceededHardTimeout === 'function'
          ? self.pendingResponseExceededHardTimeout()
          : false;
        if (hasPending && !hardStale) {
          self._resetTypingTimeout();
          return;
        }
        // Transport timeout: do not fabricate assistant content.
        self._clearStreamingTypewriters();
        typeof self.clearTransientThinkingRows === 'function' ? self.clearTransientThinkingRows({ force: true }) : (self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; }));
        // Do not inject transport-authored text into the chat transcript.
        self.sending = false;
        self._responseStartedAt = 0;
        self.tokenCount = 0;
        self._inflightPayload = null;
        self._clearPendingWsRequest();
        self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', 'idle');
        self.scheduleConversationPersist();
      }, 120000);
    },

    hasLivePendingResponse: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          return true;
        }
      }
      return !!(this._pendingWsRequest && this._pendingWsRequest.agent_id);
    },

    pendingResponseExceededHardTimeout: function() {
      var now = Date.now();
      var startedAt = Number(this._responseStartedAt || 0);
      if ((!Number.isFinite(startedAt) || startedAt <= 0) && this._pendingWsRequest) {
        startedAt = Number(this._pendingWsRequest.started_at || 0);
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) {
        var rows = Array.isArray(this.messages) ? this.messages : [];
        for (var i = rows.length - 1; i >= 0; i--) {
          var row = rows[i];
          if (!row) continue;
          if (!(row.thinking || row.streaming || (row.terminal && row.thinking))) continue;
          var rowStartedAt = Number(row._stream_started_at || row._stream_updated_at || row.ts || 0);
          if (Number.isFinite(rowStartedAt) && rowStartedAt > 0) {
            startedAt = rowStartedAt;
            break;
          }
        }
      }
      if (!Number.isFinite(startedAt) || startedAt <= 0) return false;
      return Math.max(0, now - startedAt) >= 900000;
    },

    _clearTypingTimeout: function() {
      if (this._typingTimeout) {
        clearTimeout(this._typingTimeout);
        this._typingTimeout = null;
      }
    },

    _resolveLiveMessageRef: function(message) {
      if (!message || typeof message !== 'object') return null;
      var msgId = message.id;
      if (!Array.isArray(this.messages) || !this.messages.length) return message;
      if (msgId == null) return message;
      for (var i = this.messages.length - 1; i >= 0; i--) {
        var row = this.messages[i];
        if (!row || typeof row !== 'object') continue;
        if (String(row.id) === String(msgId)) return row;
      }
      return message;
    },

    _clearMessageTypewriter: function(message, options) {
      var liveMessage = this._resolveLiveMessageRef(message);
      if (!liveMessage || typeof liveMessage !== 'object') return;
      var opts = options && typeof options === 'object' ? options : {};
      var preserveTypingVisual = opts.preserveTypingVisual === true;
      var preservePartialText = opts.preservePartialText === true;
      var clearFinalText = opts.clearFinalText !== false;
      if (liveMessage._typewriterTimer) {
        clearTimeout(liveMessage._typewriterTimer);
        liveMessage._typewriterTimer = null;
      }
      if (message && message !== liveMessage && message._typewriterTimer) {
        clearTimeout(message._typewriterTimer);
        message._typewriterTimer = null;
      }
      liveMessage._typewriterRunning = false;
      if (message && message !== liveMessage) message._typewriterRunning = false;
      if (!preserveTypingVisual) {
        if (
          !preservePartialText &&
          liveMessage._typingVisual &&
          typeof liveMessage._typewriterFinalText === 'string'
        ) {
          liveMessage.text = String(liveMessage._typewriterFinalText || '');
        }
        liveMessage._typingVisual = false;
        if (message && message !== liveMessage) message._typingVisual = false;
      }
      if (clearFinalText && !preserveTypingVisual) {
        liveMessage._typewriterFinalText = '';
        if (message && message !== liveMessage) message._typewriterFinalText = '';
      }
      if (!preserveTypingVisual) {
        liveMessage._typingVisualHtml = '';
        liveMessage._typingVisualHtmlStable = '';
        liveMessage._typingVisualHtmlActive = '';
        liveMessage._typingVisualHtmlActiveStable = '';
        if (message && message !== liveMessage) message._typingVisualHtml = '';
        if (message && message !== liveMessage) message._typingVisualHtmlStable = '';
        if (message && message !== liveMessage) message._typingVisualHtmlActive = '';
        if (message && message !== liveMessage) message._typingVisualHtmlActiveStable = '';
      }
    },

    _clearStreamingTypewriters: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        this._clearMessageTypewriter(rows[i], {
          preserveTypingVisual: false,
          preservePartialText: false,
        });
      }
    },

    _hasActiveTypewriterVisual: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || typeof row !== 'object') continue;
        if (row._typingVisual || row._typewriterRunning || row._typewriterTimer) return true;
      }
      return false;
    },

    _queueStreamTypingRender: function(message, nextText) {
      if (!message || typeof message !== 'object') return;
      var targetText = String(nextText || '');
      message._streamTargetText = targetText;
      message._typewriterFinalText = '';
      if (message._typewriterRunning) return;
      var self = this;
      message._typewriterRunning = true;

      var step = function() {
        if (!message || !message.streaming) {
          self._clearMessageTypewriter(message);
          return;
        }
        var target = String(message._streamTargetText || '');
        var current = String(message.text || '');
        if (target === current) {
          self._clearMessageTypewriter(message);
          return;
        }
        // If sanitization trims or rewrites content, snap to the newest safe value.
        if (target.length < current.length || target.indexOf(current) !== 0) {
          message.text = target;
          self._clearMessageTypewriter(message);
          self.scrollToBottom();
          return;
        }
        var remaining = target.length - current.length;
        var take = Math.max(1, Math.min(8, Math.ceil(remaining / 4)));
        message.text = target.slice(0, current.length + take);
        self.scrollToBottom();
        if (message.text.length < target.length) {
          message._typewriterTimer = setTimeout(step, 1);
          return;
        }
        self._clearMessageTypewriter(message);
      };

      step();
    },

    _resolveTypingDelayForToken: function(baseDelay, emittedToken, fullText, emittedIndex) {
      var base = Number(baseDelay || 1);
      if (!Number.isFinite(base) || base < 0) base = 1;
      var token = String(emittedToken || '');
      if (!/[.!?]/.test(token)) return base;
      var source = String(fullText || '');
      var idx = Number(emittedIndex || 0);
      if (!Number.isFinite(idx) || idx < 0) idx = 0;
      var next = source.charAt(idx + 1);
      if (!next || /\s|["')\]]/.test(next)) {
        return base * 2;
      }
      return base;
    },

    // Backward-compat shim for legacy callers during naming migration.
    _typingDelayForToken: function(baseDelay, emittedToken, fullText, emittedIndex) {
      return this._resolveTypingDelayForToken(baseDelay, emittedToken, fullText, emittedIndex);
    },

    _resolveTypingWordCadenceMs: function(fallbackDelayMs) {
      var cadenceMs = Number(this.typingWordCadenceMs);
      if (!Number.isFinite(cadenceMs) || cadenceMs <= 0) cadenceMs = Number(fallbackDelayMs);
      if (!Number.isFinite(cadenceMs) || cadenceMs <= 0) cadenceMs = 1;
      cadenceMs = Math.max(1, Math.min(2000, cadenceMs));
      return cadenceMs;
    },

    _escapeTypingVisualTokenHtml: function(token) {
      var raw = String(token == null ? '' : token);
      var escaped = '';
      if (typeof this.escapeHtml === 'function') escaped = this.escapeHtml(raw);
      else escaped = raw
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
      escaped = escaped.replace(/\t/g, '    ');
      escaped = escaped.replace(/\n/g, '<br>');
      return escaped;
    },

    _queueFinalWordTypingRender: function(message, finalText, wordDelayMs) {
      var baseMessage = this._resolveLiveMessageRef(message);
      if (!baseMessage || typeof baseMessage !== 'object') return;
      var targetText = String(finalText || '');
      this._clearMessageTypewriter(baseMessage, {
        preserveTypingVisual: false,
        preservePartialText: false,
      });
      baseMessage._typingVisual = false;
      if (!targetText.trim()) {
        baseMessage._typewriterFinalText = '';
        baseMessage._typingVisualHtml = '';
        baseMessage._typingVisualHtmlStable = '';
        baseMessage._typingVisualHtmlActive = '';
        baseMessage._typingVisualHtmlActiveStable = '';
        baseMessage.text = targetText;
        if (typeof this.scheduleConversationPersist === 'function') this.scheduleConversationPersist();
        return;
      }
      var segments = [];
      var segmentPattern = /\S+\s*/g;
      var segmentMatch;
      var typingRenderText = typeof normalizeChatMarkdownListBreaks === 'function'
        ? normalizeChatMarkdownListBreaks(targetText)
        : targetText;
      while ((segmentMatch = segmentPattern.exec(typingRenderText)) !== null) {
        segments.push({
          text: String(segmentMatch[0] || ''),
          index: Number(segmentMatch.index || 0)
        });
      }
      var leadingWhitespaceMatch = typingRenderText.match(/^\s+/);
      if (leadingWhitespaceMatch && segments.length) {
        var leadingWhitespace = String(leadingWhitespaceMatch[0] || '');
        segments[0].text = leadingWhitespace + String(segments[0].text || '');
        segments[0].index = 0;
      }
      if (!Array.isArray(segments) || !segments.length) {
        baseMessage._typewriterFinalText = '';
        baseMessage._typingVisualHtml = '';
        baseMessage._typingVisualHtmlStable = '';
        baseMessage._typingVisualHtmlActive = '';
        baseMessage._typingVisualHtmlActiveStable = '';
        baseMessage.text = targetText;
        baseMessage._typingVisual = false;
        if (typeof this.scheduleConversationPersist === 'function') this.scheduleConversationPersist();
        return;
      }
      baseMessage._typewriterFinalText = targetText;
      baseMessage.text = '';
      baseMessage._typingVisualHtml = '';
      baseMessage._typingVisualHtmlStable = '';
      baseMessage._typingVisualHtmlActive = '';
      baseMessage._typingVisualHtmlActiveStable = '';
      baseMessage._typingVisual = true;
      baseMessage._typewriterRunning = true;
      var self = this;
      var index = 0;
      var markdownState = { bold: false, italic: false };
      var cadenceMs = typeof this._resolveTypingWordCadenceMs === 'function'
        ? this._resolveTypingWordCadenceMs(wordDelayMs)
        : 1;
      var maxTokensPerTick = 24;
      var nextTickAt = Date.now();
      var keepPinnedToBottom = function() {
        try {
          if (typeof self.scrollToBottomImmediate === 'function') {
            self.scrollToBottomImmediate({ force: false });
          } else {
            self.scrollToBottom();
          }
        } catch (_) {}
      };
      var step = function() {
        var liveMessage = self._resolveLiveMessageRef(baseMessage);
        if (!liveMessage) {
          self._clearMessageTypewriter(baseMessage);
          return;
        }
        if (!liveMessage._typewriterRunning) {
          self._clearMessageTypewriter(liveMessage, {
            preserveTypingVisual: false,
            preservePartialText: false,
          });
          if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
          return;
        }
        if (index >= segments.length) {
          liveMessage.text = targetText;
          liveMessage._typingVisual = false;
          liveMessage._typingVisualHtmlStable = '';
          liveMessage._typingVisualHtmlActive = '';
          liveMessage._typingVisualHtmlActiveStable = '';
          liveMessage._typingVisualHtml = '';
          if (baseMessage !== liveMessage) {
            baseMessage._typingVisual = false;
            baseMessage._typingVisualHtmlStable = '';
            baseMessage._typingVisualHtmlActive = '';
            baseMessage._typingVisualHtmlActiveStable = '';
            baseMessage._typingVisualHtml = '';
          }
          liveMessage._typewriterRunning = false;
          liveMessage._typewriterTimer = null;
          if (baseMessage !== liveMessage) {
            baseMessage._typewriterRunning = false;
            baseMessage._typewriterTimer = null;
          }
          if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
          return;
        }
        var now = Date.now();
        if (now < nextTickAt) {
          var waitMs = Math.max(1, Math.min(2000, Math.round(nextTickAt - now)));
          var waitTimer = setTimeout(step, waitMs);
          liveMessage._typewriterTimer = waitTimer;
          if (baseMessage !== liveMessage) baseMessage._typewriterTimer = waitTimer;
          return;
        }
        var emitted = 0;
        var stableHtml = String(liveMessage._typingVisualHtmlStable || '') + String(liveMessage._typingVisualHtmlActiveStable || '');
        var activeHtml = '';
        var activeStable = '';
        while (index < segments.length && emitted < maxTokensPerTick) {
          now = Date.now();
          if (now < nextTickAt) break;
          cadenceMs = typeof self._resolveTypingWordCadenceMs === 'function'
            ? self._resolveTypingWordCadenceMs(wordDelayMs)
            : cadenceMs;
          var segment = segments[index] || { text: '', index: 0 };
          var token = String(segment.text || '');
          index += 1;
          emitted += 1;
          liveMessage.text = String(liveMessage.text || '') + token;
          var tokenEndIndex = Number(segment.index || 0) + Math.max(0, token.length - 1);
          var nextDelay = typeof self._resolveTypingDelayForToken === 'function'
            ? self._resolveTypingDelayForToken(cadenceMs, token, typingRenderText, tokenEndIndex)
            : cadenceMs;
          if (!Number.isFinite(nextDelay) || nextDelay <= 0) nextDelay = cadenceMs;
          nextDelay = Math.max(1, Math.min(2000, nextDelay));
          nextTickAt += nextDelay;
          var tokenHtmlStable = '';
          var tokenHtmlActive = '';
          var tokenState = { bold: !!markdownState.bold, italic: !!markdownState.italic };
          var appendChunk = function(chunk, isActiveChunk) {
            if (!chunk) return;
            var chunkHtml = self._escapeTypingVisualTokenHtml(chunk);
            if (tokenState.bold) chunkHtml = '<strong>' + chunkHtml + '</strong>';
            if (tokenState.italic) chunkHtml = '<em>' + chunkHtml + '</em>';
            if (isActiveChunk) {
              tokenHtmlActive +=
                '<span class="typing-word-active" style="--typing-word-fade-ms:' +
                '1000ms">' +
                chunkHtml +
                '</span>';
              tokenHtmlStable += chunkHtml;
              return;
            }
            tokenHtmlStable += chunkHtml;
            tokenHtmlActive += chunkHtml;
          };
          var cursor = 0;
          while (cursor < token.length) {
            if (token.charAt(cursor) === '\\' && (cursor + 1) < token.length && token.charAt(cursor + 1) === '*') {
              appendChunk('*', true);
              cursor += 2;
              continue;
            }
            if ((cursor + 1) < token.length && token.charAt(cursor) === '*' && token.charAt(cursor + 1) === '*') {
              tokenState.bold = !tokenState.bold;
              cursor += 2;
              continue;
            }
            if (token.charAt(cursor) === '*') {
              tokenState.italic = !tokenState.italic;
              cursor += 1;
              continue;
            }
            var start = cursor;
            while (cursor < token.length) {
              if (token.charAt(cursor) === '\\' && (cursor + 1) < token.length && token.charAt(cursor + 1) === '*') break;
              if ((cursor + 1) < token.length && token.charAt(cursor) === '*' && token.charAt(cursor + 1) === '*') break;
              if (token.charAt(cursor) === '*') break;
              cursor += 1;
            }
            var chunk = token.slice(start, cursor);
            if (!chunk) continue;
            if (!/\S/.test(chunk)) {
              appendChunk(chunk, false);
              continue;
            }
            var leadMatch = chunk.match(/^\s+/);
            var trailMatch = chunk.match(/\s+$/);
            var lead = leadMatch ? String(leadMatch[0] || '') : '';
            var trail = trailMatch ? String(trailMatch[0] || '') : '';
            var coreStart = lead.length;
            var coreEnd = chunk.length - trail.length;
            if (coreEnd < coreStart) {
              coreEnd = coreStart;
              trail = '';
            }
            var core = chunk.slice(coreStart, coreEnd);
            if (lead) appendChunk(lead, false);
            if (core) appendChunk(core, true);
            if (trail) appendChunk(trail, false);
          }
          markdownState.bold = !!tokenState.bold;
          markdownState.italic = !!tokenState.italic;
          if (activeStable) stableHtml += activeStable;
          activeStable = tokenHtmlStable;
          activeHtml = tokenHtmlActive;
        }
        liveMessage._typingVisualHtmlStable = stableHtml;
        liveMessage._typingVisualHtmlActive = activeHtml;
        liveMessage._typingVisualHtmlActiveStable = activeStable;
        liveMessage._typingVisualHtml = stableHtml + activeHtml;
        if (baseMessage !== liveMessage) {
          baseMessage._typingVisualHtmlStable = liveMessage._typingVisualHtmlStable;
          baseMessage._typingVisualHtmlActive = liveMessage._typingVisualHtmlActive;
          baseMessage._typingVisualHtmlActiveStable = liveMessage._typingVisualHtmlActiveStable;
          baseMessage._typingVisualHtml = liveMessage._typingVisualHtml;
        }
        if (emitted > 0) keepPinnedToBottom();
        if (index < segments.length) {
          var timerDelay = Math.max(1, Math.min(2000, Math.round(nextTickAt - Date.now())));
          var timerId = setTimeout(step, timerDelay);
          liveMessage._typewriterTimer = timerId;
          if (baseMessage !== liveMessage) baseMessage._typewriterTimer = timerId;
          return;
        }
        liveMessage.text = targetText;
        liveMessage._typingVisual = false;
        liveMessage._typingVisualHtmlStable = '';
        liveMessage._typingVisualHtmlActive = '';
        liveMessage._typingVisualHtmlActiveStable = '';
        liveMessage._typingVisualHtml = '';
        if (baseMessage !== liveMessage) {
          baseMessage._typingVisual = false;
          baseMessage._typingVisualHtmlStable = '';
          baseMessage._typingVisualHtmlActive = '';
          baseMessage._typingVisualHtmlActiveStable = '';
          baseMessage._typingVisualHtml = '';
        }
        liveMessage._typewriterRunning = false;
        liveMessage._typewriterTimer = null;
        if (baseMessage !== liveMessage) {
          baseMessage._typewriterRunning = false;
          baseMessage._typewriterTimer = null;
        }
        keepPinnedToBottom();
        if (typeof self.scheduleConversationPersist === 'function') self.scheduleConversationPersist();
      };
      step();
    },
  };
}
