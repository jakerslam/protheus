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
      var compact = function(value, maxLen) {
        var cap = Number(maxLen || 220);
        var text = String(value == null ? '' : value)
          .replace(/^\s*(?:agent|assistant|system|user|jarvis)\s*:\s*/i, '')
          .replace(/\s+/g, ' ')
          .trim();
        if (!text) return '';
        if (text.length > cap) return text.substring(0, Math.max(8, cap - 3)) + '...';
        return text;
      };
      var stopWords = {
        a: true, about: true, all: true, an: true, and: true, are: true, as: true, at: true, be: true, can: true,
        compare: true, confirm: true, confirmed: true, continue: true, could: true, current: true, did: true, do: true, does: true,
        explain: true, finish: true, for: true, from: true, help: true, how: true, i: true, implement: true,
        in: true, into: true, is: true, it: true, kill: true, list: true, me: true, my: true, now: true, of: true, ok: true, okay: true,
        on: true, or: true, please: true, respond: true, should: true, show: true, so: true, status: true,
        sure: true, tell: true, test: true, that: true, the: true, then: true, this: true, to: true, us: true,
        validate: true, verify: true, we: true, what: true, when: true, where: true, why: true, with: true,
        would: true, yeah: true, yep: true, yes: true, you: true, your: true, blocker: true, blockers: true,
        remove: true, delete: true, archive: true, cleanup: true, clear: true, drop: true, disable: true, enable: true,
        extra: true, works: true, working: true
      };
      var trimTrailingJoiners = function(text) {
        var words = String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean);
        while (words.length > 1) {
          var tail = String(words[words.length - 1] || '').replace(/[^a-z0-9_-]+/gi, '').toLowerCase();
          if (!tail || /^(and|or|to|with|for|from|via|then|than|versus|vs)$/i.test(tail)) {
            words.pop();
            continue;
          }
          break;
        }
        return words.join(' ');
      };
      var clampWords = function(value, maxWords) {
        var cap = Number(maxWords || 10);
        if (!Number.isFinite(cap) || cap < 3) cap = 10;
        var words = String(value == null ? '' : value).trim().split(/\s+/g).filter(Boolean);
        if (!words.length) return '';
        if (words.length > cap) words = words.slice(0, cap);
        return trimTrailingJoiners(words.join(' '));
      };
      var topicFromText = function(value, maxWords) {
        var tokens = String(value == null ? '' : value)
          .toLowerCase()
          .replace(/[^a-z0-9_:-]+/g, ' ')
          .split(/\s+/g)
          .filter(function(token) {
            return !!(token && token.length >= 3 && !stopWords[token]);
          })
          .slice(0, Number(maxWords || 3));
        return trimTrailingJoiners(tokens.join(' '));
      };
      var styleSuggestion = function(body) {
        var text = clampWords(compact(body || '', 240), 10);
        if (!text) return '';
        text = clampWords(text, 10);
        if (!text) return '';
        text = text.replace(/[.!?]+$/g, '').trim();
        if (!text) return '';
        text = text.charAt(0).toUpperCase() + text.slice(1);
        if (!/[?!]$/.test(text)) text += '?';
        return text;
      };
      var addTemplateRows = function(topic) {
        var seed = topicFromText(topic, 4);
        if (!seed) return;
        rows.push('What is the fastest next step for ' + seed);
        rows.push('What should we do next for ' + seed);
        rows.push('What still needs to be resolved for ' + seed);
        rows.push('What should we verify before closing ' + seed);
      };
      var _hint = hint;

      var context = this.collectPromptSuggestionContext();
      var history = Array.isArray(context.history) ? context.history.slice(-7) : [];
      var userRows = history
        .filter(function(entry) { return String(entry && entry.role || '').toLowerCase() === 'user'; })
        .map(function(entry) { return compact(entry && entry.text || '', 220); })
        .filter(Boolean)
        .slice(-5);

      var keywordCounts = {};
      history.forEach(function(entry) {
        var text = compact(entry && entry.text || '', 320).toLowerCase();
        text.split(/[^a-z0-9_-]+/g).forEach(function(word) {
          if (!word || word.length < 4 || stopWords[word]) return;
          keywordCounts[word] = (keywordCounts[word] || 0) + 1;
        });
      });
      var keywords = Object.keys(keywordCounts).sort(function(a, b) {
        var dc = Number(keywordCounts[b] || 0) - Number(keywordCounts[a] || 0);
        if (dc !== 0) return dc;
        return a.localeCompare(b);
      }).slice(0, 5);

      var primaryTopic = '';
      if (keywords.length) primaryTopic = topicFromText(keywords.slice(0, 3).join(' '), 4);
      if (!primaryTopic && userRows.length) primaryTopic = topicFromText(userRows[userRows.length - 1], 4);
      if (!primaryTopic && history.length) {
        primaryTopic = topicFromText((history[history.length - 1] && history[history.length - 1].text) || '', 4);
      }
      if (!primaryTopic) return [];

      addTemplateRows(primaryTopic);
      if (keywords.length >= 2) rows.push('Compare ' + keywords[0] + ' versus ' + keywords[1]);
      if (userRows.length >= 2) {
        var priorTopic = topicFromText(userRows[userRows.length - 2], 4);
        if (priorTopic && priorTopic !== primaryTopic) {
          rows.push('Compare ' + primaryTopic + ' versus ' + priorTopic);
        }
      }

      var styledRows = rows.map(styleSuggestion).filter(Boolean);
      var normalized = this.normalizePromptSuggestions(
        styledRows,
        String(gateContext || String(context.signature || ''))
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
	        var normalizedRole = compact(row.role || '', 16).toLowerCase();
	        if (!normalizedRole) {
	          normalizedRole = row.user ? 'user' : row.assistant ? 'agent' : 'agent';
	        }
	        if (normalizedRole === 'system') continue;
	        var text = compact(row.text, 240);
	        if (!text) continue;
	        if (/^\[runtime-task\]/i.test(text)) continue;
	        if (/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(text)) continue;
	        if (/the user wants exactly 3 actionable next user prompts/i.test(text)) continue;
	        if (String(text || '').toLowerCase() === 'heartbeat_ok') continue;
	        if (out.history.length < 7) {
	          out.history.unshift({
	            role: normalizedRole,
	            text: text
	          });
	        }
	        if (!out.lastUser && normalizedRole === 'user') {
	          out.lastUser = text;
	          continue;
	        }
	        if (!out.lastAgent && (normalizedRole === 'agent' || normalizedRole === 'assistant')) {
	          out.lastAgent = text;
	        }
	        if (out.lastUser && out.lastAgent && out.history.length >= 7) break;
	      }
      if (out.history.length > 7) out.history = out.history.slice(-7);
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
        if (out.length >= 7) break;
      }
      return out;
    },

    hasConversationSuggestionSeed() {
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return false;
      var context = this.collectPromptSuggestionContext();
      var count = Array.isArray(context && context.history) ? context.history.length : 0;
      return count >= 7;
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
