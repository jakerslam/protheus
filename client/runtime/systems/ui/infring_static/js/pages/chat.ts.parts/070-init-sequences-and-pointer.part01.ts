// FILE_SIZE_EXCEPTION: reason=chat pointer fx split continuity owner=jay expires=2026-06-30
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

    async refreshPromptSuggestions(force, hint) {
      var agent = this.currentAgent;
      if (!agent || !agent.id) {
        this.promptSuggestions = [];
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
      if (!this.hasConversationSuggestionSeed()) {
        this.promptSuggestions = [];
        this.suggestionsLoading = false;
        return;
      }
      var now = Date.now();
      var agentId = String(agent.id);
      var recentlyFetched =
        !force &&
        this._lastSuggestionsAgentId === agentId &&
        (now - Number(this._lastSuggestionsAt || 0)) < 12000 &&
        Array.isArray(this.promptSuggestions) &&
        this.promptSuggestions.length > 0;
      if (recentlyFetched) return;

      var seq = Number(this._suggestionFetchSeq || 0) + 1;
      this._suggestionFetchSeq = seq;
      this.suggestionsLoading = true;
	      try {
	        var payload = {};
	        var context = this.collectPromptSuggestionContext();
	        if (context.signature) payload.recent_context = String(context.signature).trim();
	        var result = await InfringAPI.post('/api/agents/' + encodeURIComponent(agentId) + '/suggestions', payload);
	        if (this._suggestionFetchSeq !== seq) return;
	        var freshContext = this.collectPromptSuggestionContext();
	        var freshHistoryCount = Array.isArray(freshContext.history) ? freshContext.history.length : 0;
	        if (freshHistoryCount < 7) {
	          this.promptSuggestions = [];
	          this._lastSuggestionsAt = Date.now();
	          this._lastSuggestionsAgentId = agentId;
	          return;
	        }
	        var gatingContext = String(context.signature || '');
	        var baseSuggestions = result && result.suggestions ? result.suggestions : [];
	        var suggestions = this.normalizePromptSuggestions(
	          Array.isArray(baseSuggestions) ? baseSuggestions : [],
	          gatingContext,
	          this.recentUserSuggestionSamples()
	        );
        this.promptSuggestions = suggestions;
        this._lastSuggestionsAt = Date.now();
        this._lastSuggestionsAgentId = agentId;
	      } catch (_) {
		        if (this._suggestionFetchSeq === seq) {
		          var fallbackContext = this.collectPromptSuggestionContext();
		          var fallbackHistoryCount = Array.isArray(fallbackContext.history) ? fallbackContext.history.length : 0;
		          if (fallbackHistoryCount < 7) {
		            this.promptSuggestions = [];
		            this._lastSuggestionsAt = Date.now();
		            this._lastSuggestionsAgentId = agentId;
		            return;
		          }
		          this.promptSuggestions = [];
          this._lastSuggestionsAt = Date.now();
          this._lastSuggestionsAgentId = agentId;
        }
      } finally {
        if (this._suggestionFetchSeq === seq) this.suggestionsLoading = false;
      }
    },

    resetFreshInitStateForAgent: function(agentRef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : {};
      var seedName = String(agent.name || agent.id || '').trim() || String(agent.id || '').trim();
      var seedEmoji = String((agent.identity && agent.identity.emoji) || '').trim();
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      this.freshInitTemplateDef = null;
      this.freshInitTemplateName = '';
      this.freshInitLaunching = false;
      this.freshInitName = '';
      this.freshInitEmoji = '';
      this.freshInitDefaultName = seedName;
      this.freshInitDefaultEmoji = seedEmoji;
      this.freshInitAvatarUrl = String(agent.avatar_url || '').trim();
      this.freshInitAvatarUploading = false;
      this.freshInitAvatarUploadError = '';
      this.freshInitEmojiPickerOpen = false;
      this.freshInitEmojiSearch = '';
      this.freshInitOtherPrompt = '';
      this.freshInitAwaitingOtherPrompt = false;
      this.freshInitPersonalityId = 'none';
      this.freshInitLifespanId = '1h';
      this.freshInitAdvancedOpen = false;
      this.freshInitVibeId = 'none';
      this.freshInitModelSuggestions = [];
      this.freshInitModelSelection = '';
      this.freshInitModelManual = false;
      this.freshInitModelSuggestLoading = false;
    },

    focusChatComposerFromInit: function(seedText) {
      var self = this;
      var text = seedText == null ? null : String(seedText);
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (!el) return;
        if (text != null) {
          self.inputText = text;
        }
        el.focus();
        try {
          var cursor = String(self.inputText || '').length;
          el.setSelectionRange(cursor, cursor);
        } catch (_) {}
        el.style.height = 'auto';
        el.style.height = Math.min(el.scrollHeight, 150) + 'px';
      });
    },

    startFreshInitSequence(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      var token = Number(this.freshInitStageToken || 0) + 1;
      this.freshInitStageToken = token;
      this._freshInitThreadShownFor = agentId;
      this.resetFreshInitStateForAgent(agent);
      this.ensureFailoverModelCache().catch(function() { return []; });
      var agentName = String(agent.name || agent.id || 'agent').trim() || 'agent';
      this.messages = [
        {
          id: ++msgId,
          role: 'agent',
          text: 'Who am I?',
          meta: '',
          tools: [],
          ts: Date.now(),
          thinking: true,
          thinking_status: 'Who am I?',
          agent_id: agentId,
          agent_name: agentName
        }
      ];
      this.recomputeContextEstimate();
      this.cacheAgentConversation(agentId);
      var self = this;
      this.$nextTick(function() {
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
      });

      setTimeout(function() {
        if (Number(self.freshInitStageToken || 0) !== token) return;
        if (!self.currentAgent || String(self.currentAgent.id || '') !== agentId) return;
          self.messages = [
            {
              id: ++msgId,
              role: 'agent',
              text: 'Who am I?',
              meta: '',
              tools: [],
              ts: Date.now(),
              thinking: true,
              thinking_status: 'Who am I?',
              agent_id: agentId,
              agent_name: agentName
            }
          ];
        self.recomputeContextEstimate();
        self.cacheAgentConversation(agentId);
        self.$nextTick(function() {
          self.scrollToBottomImmediate();
          self.stabilizeBottomScroll();
          self.pinToLatestOnOpen(null, { maxFrames: 20 });
        });

        setTimeout(function() {
          if (Number(self.freshInitStageToken || 0) !== token) return;
          if (!self.currentAgent || String(self.currentAgent.id || '') !== agentId) return;
          self.freshInitRevealMenu = true;
          self.showFreshArchetypeTiles = true;
          self.$nextTick(function() {
            self.stabilizeBottomScroll();
          });
        }, 900);
      }, 500);
    },

    ensureFreshInitThread(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if (this._freshInitThreadShownFor === agentId && Array.isArray(this.messages) && this.messages.length > 0) {
        return;
      }
      this.startFreshInitSequence(agent);
    },

    sessionHasAnyHistory: function(data) {
      if (data && Array.isArray(data.messages) && data.messages.length > 0) return true;
      var pools = [];
      if (data && Array.isArray(data.sessions)) pools = pools.concat(data.sessions);
      if (data && data.session && Array.isArray(data.session.sessions)) {
        pools = pools.concat(data.session.sessions);
      }
      for (var i = 0; i < pools.length; i++) {
        var row = pools[i] || {};
        var count = Number(row.message_count);
        if (Number.isFinite(count) && count > 0) return true;
        if (Array.isArray(row.messages) && row.messages.length > 0) return true;
      }
      return false;
    },

    recoverEmptySessionRender: function(agentId, sessionPayload) {
      var targetId = String(agentId || '').trim();
      if (!targetId) return;
      if (this.isFreshInitInProgressFor(targetId)) return;
      var resolved =
        this.resolveAgent(targetId) ||
        (this.currentAgent && String(this.currentAgent.id || '') === targetId ? this.currentAgent : null);
      if (!this.sessionHasAnyHistory(sessionPayload) && resolved && resolved.id) {
        this.ensureFreshInitThread(resolved);
        return;
      }
      this.messages = [{
        id: ++msgId,
        role: 'system',
        text: 'This session is empty. Send a message to begin.',
        meta: '',
        tools: [],
        system_origin: 'session:empty',
        ts: Date.now()
      }];
      this.recomputeContextEstimate();
      this.cacheAgentConversation(targetId);
      var self = this;
      this.$nextTick(function() {
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
      });
    },

    isFreshInitInProgressFor: function(agentId) {
      var targetId = String(agentId || '').trim();
      if (!targetId) return false;
      var currentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      if (!currentId || currentId !== targetId) return false;
      if (
        this.showFreshArchetypeTiles ||
        this.freshInitRevealMenu ||
        this.freshInitLaunching ||
        this.freshInitAwaitingOtherPrompt ||
        !!this.freshInitTemplateDef
      ) {
        return true;
      }
      var pendingFreshId = '';
      try {
        var store = Alpine.store('app');
        pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
      } catch(_) {}
      return !!pendingFreshId && pendingFreshId === targetId;
    },

    shouldSuppressAgentInactive: function(agentId) {
      var targetId = String(agentId || '').trim();
      if (!targetId) return false;
      if (this.isSystemThreadId && this.isSystemThreadId(targetId)) return true;
      if (this.isFreshInitInProgressFor(targetId)) return true;
      try {
        var store = Alpine.store('app');
        var pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
        var currentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
        if (pendingFreshId && currentId && pendingFreshId === targetId && currentId === targetId) {
          return true;
        }
      } catch(_) {}
      return false;
    },

    pointerFxThemeMode() {
      try {
        var bodyTheme = '';
        var rootTheme = '';
        if (document && document.body && document.body.dataset) {
          bodyTheme = String(document.body.dataset.theme || '').toLowerCase().trim();
        }
        if (document && document.documentElement) {
          rootTheme = String(
            (document.documentElement.dataset && document.documentElement.dataset.theme) ||
            document.documentElement.getAttribute('data-theme') ||
            ''
          ).toLowerCase().trim();
        }
        var resolved = bodyTheme || rootTheme;
        if (!resolved) {
          try {
            resolved = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
              ? 'dark'
              : 'light';
          } catch(_) {
            resolved = 'light';
          }
        }
        if (document && document.body && document.body.dataset) {
          if (!bodyTheme || bodyTheme !== resolved) {
            document.body.dataset.theme = resolved;
          }
        }
        return resolved === 'dark' ? 'dark' : 'light';
      } catch(_) {
        return 'light';
      }
    },

    pointerTrailProfile() {
      if (
        typeof window !== 'undefined' &&
        window.__INFRING_POINTER_TRAIL_PROFILE_V1 &&
        typeof window.__INFRING_POINTER_TRAIL_PROFILE_V1 === 'object'
      ) {
        return window.__INFRING_POINTER_TRAIL_PROFILE_V1;
      }
      return {
        spacing: 0.13,
        max_steps: 52,
        head_interval_ms: 28,
        segment_thickness_base: 2.05,
        segment_thickness_gain: 1.85,
        segment_opacity_base: 0.32,
        segment_opacity_gain: 0.45,
        segment_hue_base: -4,
        segment_hue_gain: 8,
        head_particles: [
          { back: 0.0, lateral: 0.0, size: 3.9, opacity: 0.58, hue: 0 },
          { back: 1.55, lateral: 0.64, size: 3.4, opacity: 0.5, hue: 2 },
          { back: 2.45, lateral: -0.58, size: 3.0, opacity: 0.44, hue: -2 },
          { back: 3.15, lateral: 0.0, size: 2.7, opacity: 0.38, hue: 1 }
        ]
      };
    },

    pointerTrailFadeDurationMs(kind, slow) {
      var base = String(kind || '') === 'segment' ? 760 : 860;
      return slow ? (base * 10) : base;
    },

    clearPointerFxCleanupTimer(node) {
      if (!node) return;
      if (node._pointerFxCleanupTimer) {
        try { clearTimeout(node._pointerFxCleanupTimer); } catch(_) {}
        node._pointerFxCleanupTimer = 0;
      }
    },

    schedulePointerFxCleanup(node, kind, slow) {
      if (!node) return;
      this.clearPointerFxCleanupTimer(node);
      var delay = this.pointerTrailFadeDurationMs(kind, !!slow);
      node._pointerFxCleanupTimer = setTimeout(function() {
        try { node.remove(); } catch(_) {}
      }, Math.max(120, delay + 120));
    },

    updatePointerTrailHoldState(container, releaseSlow) {
      var host = this.resolveMessagesScroller(container || this._pointerTrailHoldHost || null) || this.resolveMessagesScroller();
      if (!host) return;
      var layer = this.resolvePointerFxLayer(host) || host;
      var nodes = layer.querySelectorAll('.chat-pointer-trail-dot:not(.chat-pointer-agent), .chat-pointer-trail-segment:not(.chat-pointer-agent)');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        var isSegment = !!(node.classList && node.classList.contains('chat-pointer-trail-segment'));
        var kind = isSegment ? 'segment' : 'dot';
        this.clearPointerFxCleanupTimer(node);
        if (!node.classList) continue;
        if (releaseSlow) {
          node.classList.remove('chat-pointer-held');
          node.classList.remove('chat-pointer-release-slow');
          try { void node.offsetWidth; } catch(_) {}
          node.classList.add('chat-pointer-release-slow');
          this.schedulePointerFxCleanup(node, kind, true);
          continue;
        }
        node.classList.remove('chat-pointer-release-slow');
        node.classList.add('chat-pointer-held');
      }
    },

    ensurePointerTrailReleaseListener() {
      if (this._pointerTrailMouseUpHandler) return;
      var self = this;
      this._pointerTrailMouseUpHandler = function(ev) {
        self.handleMessagesPointerUp(ev || null);
      };
      document.addEventListener('mouseup', this._pointerTrailMouseUpHandler, true);
      document.addEventListener('pointerup', this._pointerTrailMouseUpHandler, true);
      window.addEventListener('blur', this._pointerTrailMouseUpHandler, true);
    },

    removePointerTrailReleaseListener() {
      if (!this._pointerTrailMouseUpHandler) return;
      try { document.removeEventListener('mouseup', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      try { document.removeEventListener('pointerup', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      try { window.removeEventListener('blur', this._pointerTrailMouseUpHandler, true); } catch(_) {}
      this._pointerTrailMouseUpHandler = null;
    },
