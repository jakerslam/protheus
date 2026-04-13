        }
      }
      if (handedOffToRecovery) return;
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._responseStartedAt = 0;
      this.sending = false;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input'); if (el) el.focus();
        self._processQueue();
      });
    },
    stopAgent: function() {
      if (!this.currentAgent) return;
      var self = this;
      InfringAPI.post('/api/agents/' + this.currentAgent.id + '/stop', {}).then(function(res) {
        self.handleStopResponse(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', res || {});
      }).catch(function(e) {
        var raw = String(e && e.message ? e.message : 'stop_failed');
        var lower = raw.toLowerCase();
        if (lower.indexOf('agent_inactive') >= 0 || lower.indexOf('inactive') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'inactive',
            { noticeText: 'Agent is now inactive.' }
          );
          return;
        }
        if (lower.indexOf('agent_contract_terminated') >= 0 || lower.indexOf('contract terminated') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'contract_terminated',
            { noticeText: 'Agent contract terminated.' }
          );
          return;
        }
        InfringToast.error('Stop failed: ' + raw);
      });
    },
    killAgent() {
      if (!this.currentAgent) return;
      var self = this;
      var name = this.currentAgent.name;
      InfringToast.confirm('Stop Agent', 'Stop agent "' + name + '"? The agent will be shut down.', async function() {
        try {
          self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id, 'idle');
          await InfringAPI.del('/api/agents/' + self.currentAgent.id);
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          InfringToast.success('Agent "' + name + '" stopped');
          Alpine.store('app').refreshAgents();
        } catch(e) {
          InfringToast.error('Failed to stop agent: ' + e.message);
        }
      });
    },
    _latexTimer: null,
    resolveMessagesScroller: function(preferred) {
      var candidate = preferred || null;
      if (candidate && candidate.id === 'messages' && candidate.offsetParent !== null) return candidate;
      var refNode = this.$refs && this.$refs.messagesEl ? this.$refs.messagesEl : null;
      if (refNode && refNode.offsetParent !== null) return refNode;
      var nodes = document.querySelectorAll('#messages');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (node && node.offsetParent !== null) return node;
      }
      return candidate && candidate.id === 'messages' ? candidate : null;
    },
    syncMapSelectionToScroll: function(container) {
      var el = this.resolveMessagesScroller(container);
      if (!el || !this.currentAgent || !Array.isArray(this.messages) || !this.messages.length) return;
      var nodes = el.querySelectorAll('.chat-message-block[id^="chat-msg-"]');
      if (!nodes || !nodes.length) return;
      var viewport = el.getBoundingClientRect();
      var viewportCenterY = viewport.top + (viewport.height / 2);
      var bestNode = null;
      var bestDiff = Number.POSITIVE_INFINITY;
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node || node.offsetParent === null) continue;
        var rect = node.getBoundingClientRect();
        if (rect.height <= 0) continue;
        if (rect.bottom < viewport.top || rect.top > viewport.bottom) continue;
        var nodeCenter = rect.top + (rect.height / 2);
        var diff = Math.abs(nodeCenter - viewportCenterY);
        if (diff < bestDiff) {
          bestDiff = diff;
          bestNode = node;
        }
      }
      if (!bestNode || !bestNode.id) return;
      var domId = String(bestNode.id);
      if (this.selectedMessageDomId !== domId) this.selectedMessageDomId = domId;
      var popup = typeof this.activeDashboardPopupOrigin === 'function'
        ? (this.activeDashboardPopupOrigin() || {})
        : {};
      if (String(popup.source || '').trim() !== 'chat-map') this.hoveredMessageDomId = domId;
      for (var idx = 0; idx < this.messages.length; idx++) {
        if (this.messageDomId(this.messages[idx], idx) === domId) { this.mapStepIndex = idx; break; }
      }
      this.centerChatMapOnMessage(domId, { immediate: true });
    },

    scrollToBottom(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var self = this;
      self.$nextTick(function() {
        if (opts.buttonAnimated) {
          self.scrollToBottomFromButton(opts);
          if (opts.stabilize) self.stabilizeBottomScroll();
          return;
        }
        self.scrollToBottomImmediate(opts);
        if (opts.stabilize) self.stabilizeBottomScroll();
      });
    },

    scrollToBottomFromButton(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var el = this.resolveMessagesScroller(opts.container || null);
      if (!el) return;
      var force = opts.force !== false;
      if (!force && !this._stickToBottom && !isNearLatestMessageBottom(this, el, opts.tolerancePx)) return;
      var startTop = Number(el.scrollTop || 0);
      var targetTop = resolveLatestMessageScrollTop(this, el);
      if (!(targetTop > startTop + 1)) {
        this.scrollToBottomImmediate({ container: el, force: true });
        return;
      }
      if (this._scrollToBottomButtonRaf) {
        try { cancelAnimationFrame(this._scrollToBottomButtonRaf); } catch (_) {}
        this._scrollToBottomButtonRaf = 0;
      }
      this._stickToBottom = true;
      this.showScrollDown = false;
      if (typeof this.triggerChatResizeBlurPulse === 'function') {
        this.triggerChatResizeBlurPulse(1000);
      } else {
        this.chatResizeBlurActive = true;
      }
      var self = this;
      var duration = 1000;
      var startedAt = 0;
      var easeOut = function(t) {
        var x = Math.max(0, Math.min(1, Number(t || 0)));
        return 1 - Math.pow(1 - x, 3);
      };
      var step = function(ts) {
        if (!startedAt) startedAt = Number(ts || 0);
        var elapsed = Math.max(0, Number(ts || 0) - startedAt);
        var progress = Math.max(0, Math.min(1, elapsed / duration));
        var eased = easeOut(progress);
        var top = startTop + ((targetTop - startTop) * eased);
        el.scrollTop = top;
        self.syncGridBackgroundOffset(el);
        if (progress < 1) {
          self._scrollToBottomButtonRaf = requestAnimationFrame(step);
          return;
        }
        self._scrollToBottomButtonRaf = 0;
        // Preserve current "blink" completion semantics, but only after the
        // staged 1s glide has completed.
        self.scrollToBottomImmediate({ container: el, force: true });
      };
      this._scrollToBottomButtonRaf = requestAnimationFrame(step);
    },

    scrollToBottomImmediate(options) {
      var opts = options && typeof options === 'object' ? options : {};
      var el = this.resolveMessagesScroller(opts.container || null);
      if (!el) return;
      var force = opts.force !== false;
      if (!force && !this._stickToBottom && !isNearLatestMessageBottom(this, el, opts.tolerancePx)) return;
      el.scrollTop = resolveLatestMessageScrollTop(this, el);
      this.syncGridBackgroundOffset(el);
      this.showScrollDown = false;
      this._stickToBottom = true;
      this.syncMapSelectionToScroll(el);
      this.scheduleMessageRenderWindowUpdate(el);
      if (this._latexTimer) clearTimeout(this._latexTimer);
      this._latexTimer = setTimeout(function() { renderLatex(el); }, 150);
    },

    stabilizeBottomScroll: function() {
      var self = this;
      var tries = 3;
      var tick = function() {
        var el = self.resolveMessagesScroller();
        if (!el) return;
        el.scrollTop = resolveLatestMessageScrollTop(self, el);
        self.syncGridBackgroundOffset(el);
        if (--tries > 0) {
          if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
          else setTimeout(tick, 16);
        }
      };
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
      else setTimeout(tick, 0);
    },
    cancelPinToLatestOnOpen: function() {
      cancelPinToLatestOnOpenJob(this);
    },
    pinToLatestOnOpen: function(container, options) {
      runPinToLatestOnOpenJob(this, container, options);
    },
    handleMessagesScroll(e) {
      var el = this.resolveMessagesScroller(e && e.target ? e.target : null);
      if (!el) return;
      this._lastMessagesScrollAt = Date.now();
      var targetTop = resolveLatestMessageScrollTop(this, el);
      scheduleBottomHardCapClamp(this, el, targetTop, 128);
      this.startAgentTrailLoop(el);
      this.syncGridBackgroundOffset(el);
      this.syncDirectHoverAfterScroll(el);
      var hiddenBottom = Math.max(0, targetTop - Number(el.scrollTop || 0));
      this._stickToBottom = hiddenBottom <= resolveBottomFollowTolerancePx(this);
      this.showScrollDown = hiddenBottom > 120;
      var self = this;
      if (typeof requestAnimationFrame === 'function') {
        if (this._scrollSyncFrame) cancelAnimationFrame(this._scrollSyncFrame);
        this._scrollSyncFrame = requestAnimationFrame(function() {
          self._scrollSyncFrame = 0;
          self.syncMapSelectionToScroll(el);
        });
      } else {
        self.syncMapSelectionToScroll(el);
      }
      this.scheduleMessageRenderWindowUpdate(el);
    },
    resolveHoveredMessageDomIdFromPoint(container, clientX, clientY) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return '';
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!(x > 0 && y > 0)) return '';
      var currentId = String(this.directHoveredMessageDomId || '').trim();
      var pickFromNode = function(node) {
        if (!node || typeof node.closest !== 'function') return '';
        var messageEl = node.closest('.message');
        if (!messageEl || !host.contains(messageEl)) return '';
        return String(messageEl.id || '').trim();
      };
      var candidateId = '';
      try {
        candidateId = pickFromNode(document.elementFromPoint(x, y));
      } catch (_) {
        candidateId = '';
      }
      if (!candidateId && typeof document.elementsFromPoint === 'function') {
        try {
          var stack = document.elementsFromPoint(x, y) || [];
          for (var i = 0; i < stack.length; i++) {
            candidateId = pickFromNode(stack[i]);
            if (candidateId) break;
          }
        } catch (_) {
          candidateId = '';
        }
      }
      if (candidateId && currentId && candidateId !== currentId) {
        var candidateEl = document.getElementById(candidateId);
        if (candidateEl) {
          var cRect = candidateEl.getBoundingClientRect();
          // Require pointer to move slightly inside the new row to avoid
          // boundary thrash on the split line between adjacent messages.
          if (y <= (cRect.top + 2) || y >= (cRect.bottom - 2)) {
            return currentId;
          }
        }
      }
      if (!candidateId && currentId) {
        var stickyEl = document.getElementById(currentId);
        if (stickyEl && host.contains(stickyEl)) {
          var sRect = stickyEl.getBoundingClientRect();
          var inStickyBand =
            x >= (sRect.left - 2) &&
            x <= (sRect.right + 2) &&
            y >= (sRect.top - 2) &&
            y <= (sRect.bottom + 2);
          if (inStickyBand) return currentId;
        }
      }
      return candidateId;
    },

    syncDirectHoverFromPointer(event) {
      if (!event || !event.currentTarget) return;
      this._lastPointerClientX = Number(event.clientX || 0);
      this._lastPointerClientY = Number(event.clientY || 0);
      var host = this.resolveMessagesScroller(event.currentTarget);
      if (!host) return;
      var domId = this.resolveHoveredMessageDomIdFromPoint(
        host,
        this._lastPointerClientX,
        this._lastPointerClientY
      );
      if (domId) {
        if (this._hoverClearTimer) {
          clearTimeout(this._hoverClearTimer);
          this._hoverClearTimer = 0;
        }
        this.directHoveredMessageDomId = domId;
        this.hoveredMessageDomId = domId;
        return;
      }
    },

    syncDirectHoverAfterScroll(container) {
      var host = this.resolveMessagesScroller(container || null);
      if (!host) return;
      var px = Number(this._lastPointerClientX || 0);
      var py = Number(this._lastPointerClientY || 0);
      if (!(px > 0 && py > 0)) {
        this.directHoveredMessageDomId = '';
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      var domId = this.resolveHoveredMessageDomIdFromPoint(host, px, py);
      if (!domId) {
        this.directHoveredMessageDomId = '';
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      this.directHoveredMessageDomId = domId;
      this.hoveredMessageDomId = domId;
    },

    addFiles(files) {
      var self = this;
      var allowed = ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'text/plain', 'application/pdf',
                      'text/markdown', 'application/json', 'text/csv'];
      var allowedExts = ['.txt', '.pdf', '.md', '.json', '.csv'];
      for (var i = 0; i < files.length; i++) {
        var file = files[i];
        if (file.size > 10 * 1024 * 1024) {
          InfringToast.warn('File "' + file.name + '" exceeds 10MB limit');
          continue;
        }
        var typeOk = allowed.indexOf(file.type) !== -1;
        if (!typeOk) {
          var ext = file.name.lastIndexOf('.') !== -1 ? file.name.substring(file.name.lastIndexOf('.')).toLowerCase() : '';
          typeOk = allowedExts.indexOf(ext) !== -1 || file.type.startsWith('image/');
        }
        if (!typeOk) {
          InfringToast.warn('File type not supported: ' + file.name);
          continue;
        }
        var preview = null;
        if (file.type.startsWith('image/')) {
          preview = URL.createObjectURL(file);
        }
        self.attachments.push({ file: file, preview: preview, uploading: false });
      }
    },
    removeAttachment(idx) {
      var att = this.attachments[idx];
      if (att && att.preview) URL.revokeObjectURL(att.preview);
      this.attachments.splice(idx, 1);
    },
    handleDrop(e) {
      e.preventDefault();
      if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length) {
        this.addFiles(e.dataTransfer.files);
      }
    },
    showMessageTitle(msg, idx, rows) {
      if (!msg || msg.is_notice) return false;
      if (msg.terminal) return this.isFirstInSourceRun(idx, rows);
      var role = String(msg.role || '').toLowerCase();
      if (role !== 'agent' && role !== 'system' && role !== 'user') return false;
      return this.isFirstInSourceRun(idx, rows);
    },
    messageMetaVisible(msg, idx, rows) {
      if (!msg || msg.is_notice || msg.thinking) return false;
      return !this.isMessageMetaCollapsed(msg, idx, rows);
    },
    isMessageMetaCollapsed(msg, idx, rows) {
      if (!msg || msg.is_notice || msg.thinking) return true;
      return !this.isDirectHoveredMessage(msg, idx);
    },
    isGrouped(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx <= 0 || idx >= list.length) return false;
      var prev = list[idx - 1];
      var curr = list[idx];
      if (!prev || !curr || prev.is_notice || curr.is_notice) return false;
      if (curr.thinking || prev.thinking) return false;
      return !this.isFirstInSourceRun(idx, list);
    },
    messageHasTailBlockingBox(msg) {
      if (!msg || typeof msg !== 'object') return false;
      if (this.messageHasTools(msg)) return true;
      if (msg.file_output && msg.file_output.path) return true;
      if (msg.folder_output && msg.folder_output.path) return true;
      if (this.messageProgress(msg)) return true;
      return false;
    },
    showMessageTail(msg, idx, rows) {
      if (!msg || msg.is_notice) return false;
      var role = this.messageGroupRole(msg);
      if (role !== 'user' && role !== 'agent' && role !== 'system') return false;
      // Tail only shows when this bubble is the terminal visible item in its source run.
      if (!msg.thinking && this.messageHasTailBlockingBox(msg)) return false;
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return true;
      return this.isLastInSourceRun(idx, list);
    },

    // Strip raw function-call text that some models (Llama, Groq, etc.) leak into output.
    // These models don't use proper tool_use blocks — they output function calls as plain text.
    sanitizeToolText: function(text) {
      if (!text) return text;
      // Pattern: <function=tool_name>{...}</function> (including hyphenated names)
      text = text.replace(/<function=[^>]+>[\s\S]*?<\/function>/gi, '');
      // Pattern: stray opening/closing function tags
      text = text.replace(/<\/?function[^>]*>/gi, '');
      // Pattern: leaked cache-control metadata tags
      text = text.replace(/<cache_control[^>]*\/>/gi, '');
      text = text.replace(/<cache_control[^>]*>[\s\S]*?<\/cache_control>/gi, '');
      text = text.replace(/<\/?cache_control[^>]*>/gi, '');
      // Pattern: plaintext cache telemetry lines
      text = text
        .split('\n')
        .filter(function(line) {
          var lowered = String(line || '').toLowerCase();
          return !(lowered.includes('stable_hash=') && (lowered.includes('cache_control') || lowered.includes('cache control')));
        })
        .join('\n');
      // Pattern: tool_name</function={"key":"value"} or tool_name</function,{...}
      text = text.replace(/\s*\w+<\/function[=,]?\s*\{[\s\S]*$/gmi, '');
      // Pattern: dangling <function=...>{... after truncation
      text = text.replace(/\s*<function=[^>]*>\s*\{[\s\S]*$/gmi, '');
      // Pattern: tool_name{"type":"function",...}
      text = text.replace(/\s*\w+\{"type"\s*:\s*"function"[\s\S]*$/gmi, '');
      // Pattern: <|python_tag|> or similar special tokens
      text = text.replace(/<\|[\w_:-]+\|>/g, '');
      text = text.replace(/\n{3,}/g, '\n\n');
      return text.trim();
    },
    collectStreamedAssistantEnvelope: function() {
      var streamedText = '';
      var streamedTools = [];
      var streamedThought = '';
      var appendThought = function(value) {
        var clean = String(value || '').trim();
        if (!clean) return;
        if (streamedThought) streamedThought += '\n';
        streamedThought += clean;
      };
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || row.role !== 'agent' || (!row.streaming && !row.thinking)) continue;
        if (!row.thinking) {
          streamedText += (typeof row._cleanText === 'string') ? row._cleanText : (row.text || '');
        }
        if (row._thoughtText) appendThought(row._thoughtText);
        if (row._reasoning) appendThought(row._reasoning);
        if (row.thinking && row.text) {
          var pendingThought = String(row.text || '').replace(/^\*+|\*+$/g, '').trim();
          if (pendingThought && pendingThought.toLowerCase() !== 'thinking...') appendThought(pendingThought);
