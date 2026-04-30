'use strict';

function infringChatPromptQueueMethods() {
  return {
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

    nextPromptQueueId() {
      this._promptQueueSeq = Number(this._promptQueueSeq || 0) + 1;
      return 'pq-' + String(Date.now()) + '-' + String(this._promptQueueSeq);
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
      if (!text) return;
      this.inputText = text;
      if (Array.isArray(item && item.files) && item.files.length) {
        this._queuedPromptAttachmentNotice = {
          queue_id: id,
          attachment_count: item.files.length,
          ts: Date.now()
        };
      }
      this.addNoticeEvent({
        notice_label: 'Queued prompt moved to composer for manual send.',
        notice_type: 'info',
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    loadPromptSuggestionsPreference() {
      var key = String(this.promptSuggestionsStorageKey || '').trim();
      if (!key) return;
      try {
        var raw = localStorage.getItem(key);
        if (raw == null) return;
        var normalized = String(raw).trim().toLowerCase();
        this.promptSuggestionsEnabled = !(
          normalized === '0' ||
          normalized === 'false' ||
          normalized === 'off' ||
          normalized === 'no'
        );
      } catch (_) {}
    },

    persistPromptSuggestionsPreference() {
      var key = String(this.promptSuggestionsStorageKey || '').trim();
      if (!key) return;
      try {
        localStorage.setItem(key, this.promptSuggestionsEnabled ? '1' : '0');
      } catch (_) {}
    },

    setPromptSuggestionsEnabled(enabled) {
      this.promptSuggestionsEnabled = enabled !== false;
      this.persistPromptSuggestionsPreference();
      if (!this.promptSuggestionsEnabled) {
        this.clearPromptSuggestions();
        return;
      }
      this.refreshPromptSuggestions(true, 'toggle-enabled');
    },

    togglePromptSuggestionsEnabled() {
      this.setPromptSuggestionsEnabled(!this.promptSuggestionsEnabled);
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

// FILE_SIZE_EXCEPTION: reason=chat pointer fx split continuity owner=jay expires=2026-06-30
// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
        try {
          chip.classList.remove('is-resizing');
          chip._resizeBlurTimer = 0;
        } catch(_) {}
      }, 65);
    },

    onPromptSuggestionHoverOut(event) {
      if (!event || !event.currentTarget) return;
      var chip = event.currentTarget;
      if (chip._resizeBlurTimer) {
        clearTimeout(chip._resizeBlurTimer);
        chip._resizeBlurTimer = 0;
      }
      chip.classList.remove('is-resizing');
      chip.classList.remove('is-expanded');
    },

    triggerChatResizeBlurPulse(durationMs) {
      this.chatResizeBlurActive = true;
      if (this._chatResizeBlurTimer) {
        clearTimeout(this._chatResizeBlurTimer);
        this._chatResizeBlurTimer = 0;
      }
      var duration = Number(durationMs || 140);
      if (!Number.isFinite(duration) || duration < 60) duration = 140;
      var self = this;
      this._chatResizeBlurTimer = setTimeout(function() {
        self._chatResizeBlurTimer = 0;
        self.chatResizeBlurActive = false;
      }, Math.round(duration));
    },

    teardownChatResizeBlurObserver() {
      if (this._chatResizeObserver && typeof this._chatResizeObserver.disconnect === 'function') {
        try { this._chatResizeObserver.disconnect(); } catch(_) {}
      }
      this._chatResizeObserver = null;
      if (this._chatResizeBlurTimer) {
        clearTimeout(this._chatResizeBlurTimer);
        this._chatResizeBlurTimer = 0;
      }
      this.chatResizeBlurActive = false;
    },

    installChatResizeBlurObserver() {
      this.teardownChatResizeBlurObserver();
      if (typeof ResizeObserver !== 'function') return;
      var host = this.$el || null;
      if (!host || typeof host.getBoundingClientRect !== 'function') return;
      var self = this;
      this._chatResizeLastWidth = Math.round(Number(host.getBoundingClientRect().width || 0));
      this._chatResizeObserver = new ResizeObserver(function(entries) {
        var entry = entries && entries.length ? entries[0] : null;
        if (!entry) return;
        var width = Math.round(Number((entry.contentRect && entry.contentRect.width) || host.getBoundingClientRect().width || 0));
        if (!Number.isFinite(width) || width <= 0) return;
        var previous = Number(self._chatResizeLastWidth || 0);
        self._chatResizeLastWidth = width;
        if (previous <= 0) return;
        if (Math.abs(width - previous) < 2) return;
        self.triggerChatResizeBlurPulse();
      });
      this._chatResizeObserver.observe(host);
    },

    refreshChatInputOverlayMetrics() {
      var host = this.$el || null;
      if (!host || typeof host.querySelector !== 'function' || !host.style) return;
      var inputArea = host.querySelector('.input-area');
      if (!inputArea || inputArea.offsetParent === null) {
        host.style.setProperty('--chat-input-overlay-height', '0px');
        host.style.setProperty('--chat-input-bottom-reserve', '136px');
        return;
      }
      var lane = inputArea.querySelector('.chat-input-lane');
      var areaRect = typeof inputArea.getBoundingClientRect === 'function' ? inputArea.getBoundingClientRect() : null;
      var laneRect = lane && typeof lane.getBoundingClientRect === 'function' ? lane.getBoundingClientRect() : null;
      var measured = Math.max(
        Number(areaRect && areaRect.height ? areaRect.height : 0),
        Number(laneRect && laneRect.height ? laneRect.height : 0)
      );
      if (!Number.isFinite(measured) || measured < 0) measured = 0;
      var overlayHeight = Math.ceil(measured);
      var reserve = overlayHeight > 0 ? (overlayHeight + 20) : 136;
      host.style.setProperty('--chat-input-overlay-height', overlayHeight + 'px');
      host.style.setProperty('--chat-input-bottom-reserve', reserve + 'px');
    },

    teardownChatInputOverlayObserver() {
      if (this._chatInputOverlayObserver && typeof this._chatInputOverlayObserver.disconnect === 'function') {
        try { this._chatInputOverlayObserver.disconnect(); } catch(_) {}
      }
      this._chatInputOverlayObserver = null;
      if (this._chatInputOverlayResizeHandler) {
        try { window.removeEventListener('resize', this._chatInputOverlayResizeHandler); } catch(_) {}
      }
      this._chatInputOverlayResizeHandler = null;
    },

    installChatInputOverlayObserver() {
      this.teardownChatInputOverlayObserver();
      var host = this.$el || null;
      if (!host || typeof host.querySelector !== 'function') return;
      var inputArea = host.querySelector('.input-area');
      this.refreshChatInputOverlayMetrics();
      if (!inputArea) return;
      var self = this;
      if (typeof ResizeObserver === 'function') {
        this._chatInputOverlayObserver = new ResizeObserver(function() {
          self.refreshChatInputOverlayMetrics();
        });
        try { this._chatInputOverlayObserver.observe(inputArea); } catch(_) {}
        var inputLaneEl = inputArea.querySelector('.chat-input-lane');
        if (inputLaneEl) {
          try { this._chatInputOverlayObserver.observe(inputLaneEl); } catch(_) {}
        }
      }
      this._chatInputOverlayResizeHandler = function() {
        self.refreshChatInputOverlayMetrics();
      };
      try { window.addEventListener('resize', this._chatInputOverlayResizeHandler, { passive: true }); } catch(_) {
        window.addEventListener('resize', this._chatInputOverlayResizeHandler);
      }
    },

    async refreshPromptSuggestions(force, hint) {
      var agent = this.currentAgent;
      if (!agent || !agent.id) {
        this.promptSuggestions = [];
        return;
      }
      if (!this.promptSuggestionsEnabled) {
        this.promptSuggestions = [];
        this.suggestionsLoading = false;
        return;
      }
      if (this.terminalMode || this.showFreshArchetypeTiles) {
        this.promptSuggestions = [];
        return;
      }
      if (this.hasPromptQueue) {
        this.promptSuggestions = [];
        this.suggestionsLoading = false;
        return;
      }
      var now = Date.now();
      var agentId = String(agent.id);
      var suggestionScopeKey = agentId + '|main';
      if (typeof this.resolveConversationCacheScopeKey === 'function') {
        try {
          var resolvedScopeKey = String(this.resolveConversationCacheScopeKey(agentId) || '').trim();
          if (resolvedScopeKey) suggestionScopeKey = resolvedScopeKey;
        } catch (_) {}
      }
      var recentlyFetched =
        !force &&
        this._lastSuggestionsAgentId === suggestionScopeKey &&
        (now - Number(this._lastSuggestionsAt || 0)) < 12000 &&
        Array.isArray(this.promptSuggestions) &&
        this.promptSuggestions.length > 0;
      if (recentlyFetched) return;

      var seq = Number(this._suggestionFetchSeq || 0) + 1;
      this._suggestionFetchSeq = seq;
      this.suggestionsLoading = true;
	      try {
	        var payload = {};
          if (suggestionScopeKey) payload.session_scope_key = suggestionScopeKey;
	        var result = await InfringAPI.post('/api/agents/' + encodeURIComponent(agentId) + '/suggestions', payload);
	        if (this._suggestionFetchSeq !== seq) return;
	        var baseSuggestions = result && result.suggestions ? result.suggestions : [];
	        var suggestions = this.normalizePromptSuggestions(
	          Array.isArray(baseSuggestions) ? baseSuggestions : [],
	          '',
	          this.recentUserSuggestionSamples()
	        );
        this.promptSuggestions = suggestions;
        this._lastSuggestionsAt = Date.now();
        this._lastSuggestionsAgentId = suggestionScopeKey;
	      } catch (_) {
		        if (this._suggestionFetchSeq === seq) {
		          this.promptSuggestions = [];
          this._lastSuggestionsAt = Date.now();
          this._lastSuggestionsAgentId = suggestionScopeKey;
        }
      } finally {
        if (this._suggestionFetchSeq === seq) this.suggestionsLoading = false;
      }
    },
  };
}

function chatPromptQueueItems(vm) {
  var queue = Array.isArray(vm.messageQueue) ? vm.messageQueue : [];
  var out = [];
  for (var i = 0; i < queue.length; i++) {
    var row = queue[i];
    if (!row || row.terminal) continue;
    if (!row.queue_id) row.queue_id = vm.nextPromptQueueId();
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
}

function chatHasPromptQueue(vm) {
  var items = chatPromptQueueItems(vm);
  return Array.isArray(items) && items.length > 0;
}
