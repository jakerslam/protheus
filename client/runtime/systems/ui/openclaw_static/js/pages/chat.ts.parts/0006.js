          var hasContextOverlap = contextKeywords.some(function(keyword) {
            return loweredRaw.indexOf(keyword) >= 0;
          });
          if (!hasContextOverlap) continue;
        }
        var key = String(raw || '').toLowerCase();
        if (seen[key]) continue;
        var duplicate = out.some(function(existing) {
          return isNearDuplicate(existing, raw);
        });
        if (duplicate) continue;
        seen[key] = true;
        out.push(raw);
        if (out.length >= 3) break;
      }
      return out;
    },

    derivePromptSuggestionFallback(agent, hint, gateContext) {
      var rows = [];
      var compact = function(value) {
        var text = String(value == null ? '' : value)
          .replace(/^\s*(?:agent|assistant|system|user|jarvis)\s*:\s*/i, '')
          .replace(/\s+/g, ' ')
          .trim();
        if (!text) return '';
        if (text.length > 180) return text.substring(0, 177) + '...';
        return text;
      };
      var sanitizeHint = function(value) {
        var text = compact(value || '');
        if (!text) return '';
        var lowered = text.toLowerCase();
        if (
          lowered === 'post-response' ||
          lowered === 'post-silent' ||
          lowered === 'post-error' ||
          lowered === 'post-terminal' ||
          lowered === 'init' ||
          lowered === 'refresh'
        ) return '';
        if (/^post-[a-z0-9_-]+$/i.test(text)) return '';
        return text;
      };
      var role = compact(agent && agent.role ? agent.role : '') || 'assistant';
      var context = this.collectPromptSuggestionContext();
      var lastUser = compact(context.lastUser || '');
      var lastAgent = compact(context.lastAgent || '');
      var cleanHint = sanitizeHint(hint || '');
      var topic = compact(cleanHint || lastUser || lastAgent || '');
      var topicWords = String(topic || '')
        .toLowerCase()
        .split(/[^a-z0-9_:-]+/g)
        .filter(function(word) {
        return word && word.length >= 4 && ['that', 'with', 'from', 'this', 'your', 'have', 'will', 'into'].indexOf(word) === -1;
      })
        .slice(0, 3);
      var topicLabel = topicWords.length ? topicWords.join(' ') : 'current task';
      var combinedLower = [cleanHint, lastUser, lastAgent].join(' ').toLowerCase();
      var rotateSeed = topicLabel + '|' + String(context.signature || '') + '|' + cleanHint;
      var rotate = 0;
      for (var ridx = 0; ridx < rotateSeed.length; ridx++) {
        rotate = (rotate + rotateSeed.charCodeAt(ridx)) % 97;
      }

      if (/\bcouldn'?t reach|failed to|timeout|lane_timeout|backend unavailable|provider-sync\b/i.test(combinedLower)) {
        rows.push('Can you auto-switch models and retry the same request');
        rows.push('What failed first, provider sync or app-plane lane');
      }
      if (/\bqueue|cockpit|conduit|latency|backpressure|reconnect|stale\b/i.test(combinedLower)) {
        rows.push('Can you reclaim stale blocks and verify queue depth after');
        rows.push('Can you scale conduit and report before and after metrics');
      }
      if (/\bupload|file|attachment\b/i.test(combinedLower)) {
        rows.push('Can you retry upload and show the failing endpoint');
      }
      if (/\bdiff|patch|commit|branch|git\b/i.test(combinedLower)) {
        rows.push('Can you show the exact diff for that change');
      }
      if (cleanHint) rows.push('Can you take the next step on ' + topicLabel);
      if (lastAgent && !/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(lastAgent)) {
        rows.push('Can you turn that into a concrete checklist');
        rows.push('Can you show the first command to run now');
      }
      if (lastUser) {
        rows.push('Can you continue this and keep the same direction');
        rows.push('Can you summarize progress in three concrete bullets');
      }
      rows.push('Can you propose the best next move from here');
      rows.push('Can you verify the latest change and report result');

      if (rows.length > 1) {
        rotate = rotate % rows.length;
        rows = rows.slice(rotate).concat(rows.slice(0, rotate));
      }

      var normalized = this.normalizePromptSuggestions(
        rows,
        String(gateContext || [cleanHint, lastUser, lastAgent, String(context.signature || '')].join(' | '))
      );
      return normalized.slice(0, 3);
    },

    collectPromptSuggestionContext() {
      var out = { lastUser: '', lastAgent: '', history: [], signature: '' };
      var history = Array.isArray(this.messages) ? this.messages : [];
      var compact = function(value, maxLen) {
        var cap = Number(maxLen || 240);
        var text = String(value == null ? '' : value).replace(/\s+/g, ' ').trim();
        if (!text) return '';
        if (text.length > cap) return text.substring(0, Math.max(8, cap - 3)) + '...';
        return text;
      };
      for (var i = history.length - 1; i >= 0; i--) {
        var row = history[i];
        if (!row || row.thinking || row.streaming || row.terminal || row.is_notice) continue;
        var text = compact(row.text, 240);
        if (!text) continue;
        if (/^\[runtime-task\]/i.test(text)) continue;
        if (/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(text)) continue;
        if (/the user wants exactly 3 actionable next user prompts/i.test(text)) continue;
        if (String(text || '').toLowerCase() === 'heartbeat_ok') continue;
        if (out.history.length < 8) {
          out.history.unshift({
            role: compact(row.role || '', 16).toLowerCase() || (row.user ? 'user' : row.assistant ? 'agent' : 'agent'),
            text: text
          });
        }
        if (!out.lastUser && row.role === 'user') {
          out.lastUser = text;
          continue;
        }
        if (!out.lastAgent && row.role === 'agent') {
          out.lastAgent = text;
        }
        if (out.lastUser && out.lastAgent && out.history.length >= 8) break;
      }
      if (out.history.length > 8) out.history = out.history.slice(-8);
      out.signature = compact(
        out.history
          .map(function(entry) {
            return compact(entry.role || 'agent', 20) + ':' + compact(entry.text || '', 180);
          })
          .join(' || ') ||
          (String(out.lastUser || '') + '|' + String(out.lastAgent || '')),
        1200
      );
      return out;
    },

    recentUserSuggestionSamples() {
      var history = Array.isArray(this.messages) ? this.messages : [];
      var out = [];
      for (var i = history.length - 1; i >= 0; i--) {
        var row = history[i];
        if (!row || row.thinking || row.streaming || row.terminal || row.is_notice) continue;
        if (String(row.role || '').toLowerCase() !== 'user') continue;
        var text = String(row.text == null ? '' : row.text).replace(/\s+/g, ' ').trim();
        if (!text) continue;
        out.unshift(text);
        if (out.length >= 8) break;
      }
      return out;
    },

    hasConversationSuggestionSeed() {
      return this.recentUserSuggestionSamples().length > 0;
    },

    nextPromptQueueId() {
      this._promptQueueSeq = Number(this._promptQueueSeq || 0) + 1;
      return 'pq-' + String(Date.now()) + '-' + String(this._promptQueueSeq);
    },

    get promptQueueItems() {
      var queue = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var out = [];
      for (var i = 0; i < queue.length; i++) {
        var row = queue[i];
        if (!row || row.terminal) continue;
        if (!row.queue_id) row.queue_id = this.nextPromptQueueId();
        if (!row.queue_kind) row.queue_kind = 'prompt';
        out.push({
          queue_id: String(row.queue_id),
          queue_index: i,
          text: String(row.text || '').trim(),
          files: Array.isArray(row.files) ? row.files : [],
          images: Array.isArray(row.images) ? row.images : [],
          queued_at: Number(row.queued_at || 0) || Date.now()
        });
      }
      return out;
    },

    get hasPromptQueue() {
      return Array.isArray(this.promptQueueItems) && this.promptQueueItems.length > 0;
    },

    queuePromptPreview(item) {
      var text = String(item && item.text ? item.text : '').replace(/\s+/g, ' ').trim();
      if (!text) return '(queued prompt)';
      return text.length > 140 ? text.substring(0, 137) + '...' : text;
    },

    removePromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      if (!this.hasPromptQueue && !this.sending && this.currentAgent) {
        this.refreshPromptSuggestions(false, 'queue-cleared');
      }
      this.scheduleConversationPersist();
    },

    movePromptQueueItem(sourceId, targetId) {
      var src = String(sourceId || '').trim();
      var dst = String(targetId || '').trim();
      if (!src || !dst || src === dst) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue.slice() : [];
      var srcIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === src); });
      var dstIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === dst); });
      if (srcIdx < 0 || dstIdx < 0) return;
      var moving = rows[srcIdx];
      rows.splice(srcIdx, 1);
      if (dstIdx > srcIdx) dstIdx -= 1;
      rows.splice(dstIdx, 0, moving);
      this.messageQueue = rows;
      this.scheduleConversationPersist();
    },

    onPromptQueueDragStart(queueId, event) {
      var id = String(queueId || '').trim();
      if (!id) return;
      this.promptQueueDragId = id;
      if (event && event.dataTransfer) {
        event.dataTransfer.effectAllowed = 'move';
        try { event.dataTransfer.setData('text/plain', id); } catch(_) {}
      }
    },

    onPromptQueueDrop(targetId, event) {
      if (event && typeof event.preventDefault === 'function') event.preventDefault();
      var sourceId = String(this.promptQueueDragId || '').trim();
      if (!sourceId && event && event.dataTransfer) {
        try {
          sourceId = String(event.dataTransfer.getData('text/plain') || '').trim();
        } catch(_) {}
      }
      var destinationId = String(targetId || '').trim();
      if (sourceId && destinationId) {
        this.movePromptQueueItem(sourceId, destinationId);
      }
      this.promptQueueDragId = '';
    },

    onPromptQueueDragEnd() {
      this.promptQueueDragId = '';
    },

    async steerPromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      var item = rows[idx];
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      var text = String(item && item.text ? item.text : '').trim();
      var files = Array.isArray(item && item.files) ? item.files : [];
      var images = Array.isArray(item && item.images) ? item.images : [];
      if (!text) return;
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: 'Steer injected into active workflow.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();

      if (!this.sending) {
        var liveAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!liveAgent || !liveAgent.id) return;
        this.appendUserChatMessage(text, images, { deferPersist: true });
        this.scheduleConversationPersist();
        this._sendPayload(text, files, images, {
          agent_id: liveAgent.id,
          steer_injected: true,
          from_queue: true,
          queue_id: id
        });
        return;
      }

      var wsPayload = { type: 'message', content: text, steer: true, priority: 'steer' };
      if (files.length) wsPayload.attachments = files;
      if (InfringAPI.wsSend(wsPayload)) {
        this.appendUserChatMessage(text, images, { deferPersist: true });
        this.scheduleConversationPersist();
        return;
      }

      if (this.currentAgent && this.currentAgent.id) {
        var reboundAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
        if (!reboundAgent || !reboundAgent.id) return;
        try {
          await InfringAPI.post('/api/agents/' + reboundAgent.id + '/message', {
            message: text,
            attachments: files,
            steer: true,
            priority: 'steer',
          });
          this.appendUserChatMessage(text, images, { deferPersist: true });
          this.scheduleConversationPersist();
          return;
        } catch(_) {}
      }

      this.messageQueue.unshift({
        queue_id: id,
        queue_kind: 'prompt',
        text: text,
        files: files,
        images: images,
        queued_at: Number(item && item.queued_at ? item.queued_at : Date.now()),
      });
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: 'Steer injection failed; prompt returned to queue.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    clearPromptSuggestions() {
      this.promptSuggestions = [];
      this.suggestionsLoading = false;
      this._lastSuggestionsAt = 0;
      this._lastSuggestionsAgentId = '';
    },

    async applyPromptSuggestion(suggestion) {
      var text = String(suggestion == null ? '' : suggestion).trim();
      if (!text) return;
      this.inputText = text;
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showAttachMenu = false;
      await this.sendMessage();
    },

    promptSuggestionNeedsResize(chip) {
      if (!chip) return false;
      var wasExpanded = chip.classList.contains('is-expanded');
      var wasResizing = chip.classList.contains('is-resizing');
      if (wasExpanded) chip.classList.remove('is-expanded');
      if (wasResizing) chip.classList.remove('is-resizing');
      var needs = false;
      try {
        var text = chip.querySelector('.prompt-suggestion-chip-text');
        if (text) needs = (Number(text.scrollWidth || 0) - Number(text.clientWidth || 0)) > 1;
        if (!needs) needs = (Number(chip.scrollWidth || 0) - Number(chip.clientWidth || 0)) > 1;
      } catch(_) {}
      if (wasExpanded) chip.classList.add('is-expanded');
      if (wasResizing) chip.classList.add('is-resizing');
      return !!needs;
    },

    onPromptSuggestionHoverIn(event) {
      if (!event || !event.currentTarget) return;
      var chip = event.currentTarget;
      if (chip._resizeBlurTimer) {
        clearTimeout(chip._resizeBlurTimer);
        chip._resizeBlurTimer = 0;
      }
      if (!this.promptSuggestionNeedsResize(chip)) {
        chip.classList.remove('is-expanded');
        chip.classList.remove('is-resizing');
        return;
      }
      chip.classList.add('is-expanded');
      chip.classList.add('is-resizing');
      chip._resizeBlurTimer = setTimeout(function() {
