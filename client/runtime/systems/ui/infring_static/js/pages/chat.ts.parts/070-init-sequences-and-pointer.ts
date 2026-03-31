// FILE_SIZE_EXCEPTION: reason=chat pointer fx split continuity owner=jay expires=2026-06-30
        try {
          chip.classList.remove('is-resizing');
          chip._resizeBlurTimer = 0;
        } catch(_) {}
      }, 130);
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
        var cleanHint = String(hint || '').trim();
        if (/^(post-(response|silent|error|terminal)|init|refresh)$/i.test(cleanHint)) cleanHint = '';
        if (cleanHint) payload.hint = cleanHint;
        var context = this.collectPromptSuggestionContext();
        if (context.signature) payload.recent_context = String(context.signature).trim();
        var activeModel = String(agent && (agent.runtime_model || agent.model_name) ? (agent.runtime_model || agent.model_name) : '').trim();
        if (activeModel) payload.current_model = activeModel;
        var result = await InfringAPI.post('/api/agents/' + encodeURIComponent(agentId) + '/suggestions', payload);
        if (this._suggestionFetchSeq !== seq) return;
        var gatingContext = [cleanHint, String(context.signature || '')].join(' | ');
        var suggestions = this.normalizePromptSuggestions(
          result && result.suggestions ? result.suggestions : [],
          gatingContext,
          this.recentUserSuggestionSamples()
        );
        this.promptSuggestions = suggestions;
        this._lastSuggestionsAt = Date.now();
        this._lastSuggestionsAgentId = agentId;
      } catch (_) {
        if (this._suggestionFetchSeq === seq) {
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
      var resolvedName = String(agent.name || agent.id || '').trim() || String(agent.id || '').trim();
      var resolvedEmoji = String(
        (agent.identity && agent.identity.emoji) ||
        this.defaultFreshEmojiForAgent(agentRef)
      ).trim() || this.defaultFreshEmojiForAgent(agentRef);
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      this.freshInitTemplateDef = null;
      this.freshInitTemplateName = '';
      this.freshInitLaunching = false;
      this.freshInitName = resolvedName;
      this.freshInitEmoji = resolvedEmoji;
      this.freshInitDefaultName = resolvedName;
      this.freshInitDefaultEmoji = resolvedEmoji;
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
          text: 'Thinking...',
          meta: '',
          tools: [],
          ts: Date.now(),
          thinking: true,
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
            self.scrollToBottomImmediate();
            self.stabilizeBottomScroll();
            self.pinToLatestOnOpen(null, { maxFrames: 20 });
          });
        }, 500);
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

    spawnPointerTrail(container, x, y, opts) {
      var options = opts || {};
      var layer = options.agentTrail ? this.resolveAgentFxLayer(container) : this.resolvePointerFxLayer(container);
      if (!layer) return;
      var marker = document.createElement('span');
      marker.className = options.agentTrail ? 'chat-pointer-trail-dot chat-pointer-agent' : 'chat-pointer-trail-dot';
      marker.style.left = x + 'px';
      marker.style.top = y + 'px';
      if (Number.isFinite(Number(options.size))) marker.style.setProperty('--trail-size', String(Number(options.size)));
      if (Number.isFinite(Number(options.opacity))) marker.style.setProperty('--trail-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.scale))) marker.style.setProperty('--trail-scale', String(Number(options.scale)));
      if (Number.isFinite(Number(options.hueShift))) marker.style.setProperty('--trail-hue-shift', String(Number(options.hueShift)) + 'deg');
      var holdMouseTrail = !options.agentTrail && !!this._pointerTrailMouseHeld;
      if (holdMouseTrail) marker.classList.add('chat-pointer-held');
      layer.appendChild(marker);
      if (!holdMouseTrail) this.schedulePointerFxCleanup(marker, 'dot', false);
    },

    spawnPointerTrailSegment(container, x0, y0, x1, y1, opts) {
      var options = opts || {};
      var layer = options.agentTrail ? this.resolveAgentFxLayer(container) : this.resolvePointerFxLayer(container);
      if (!layer) return;
      var dx = Number(x1 || 0) - Number(x0 || 0);
      var dy = Number(y1 || 0) - Number(y0 || 0);
      var dist = Math.sqrt(dx * dx + dy * dy);
      if (!Number.isFinite(dist) || dist < 0.75) return;
      var seg = document.createElement('span');
      seg.className = options.agentTrail ? 'chat-pointer-trail-segment chat-pointer-agent' : 'chat-pointer-trail-segment';
      var mx = Number(x0 || 0) + (dx * 0.5);
      var my = Number(y0 || 0) + (dy * 0.5);
      var angle = Math.atan2(dy, dx) * (180 / Math.PI);
      seg.style.left = mx + 'px';
      seg.style.top = my + 'px';
      seg.style.width = Math.max(2, dist + 1) + 'px';
      seg.style.transform = 'translate(-50%, -50%) rotate(' + angle + 'deg)';
      if (Number.isFinite(Number(options.thickness))) seg.style.setProperty('--trail-seg-thickness', String(Number(options.thickness)));
      if (Number.isFinite(Number(options.opacity))) seg.style.setProperty('--trail-seg-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.hueShift))) seg.style.setProperty('--trail-seg-hue-shift', String(Number(options.hueShift)) + 'deg');
      var holdMouseTrail = !options.agentTrail && !!this._pointerTrailMouseHeld;
      if (holdMouseTrail) seg.classList.add('chat-pointer-held');
      layer.appendChild(seg);
      if (!holdMouseTrail) this.schedulePointerFxCleanup(seg, 'segment', false);
    },

    spawnPointerRipple(container, x, y) {
      var layer = this.resolvePointerFxLayer(container);
      if (!layer) return;
      var ripple = document.createElement('span');
      ripple.className = 'chat-pointer-ripple';
      ripple.style.left = x + 'px';
      ripple.style.top = y + 'px';
      layer.appendChild(ripple);
      setTimeout(function() {
        try { ripple.remove(); } catch(_) {}
      }, 820);
    },

    resolvePointerFxLayer(container) {
      if (!container || typeof container.querySelector !== 'function') return null;
      return container.querySelector('.chat-grid-overlay') || container;
    },
    resolveAgentFxLayer(container) {
      if (!container || typeof container.querySelector !== 'function') return null;
      var layer = container.querySelector('.chat-agent-overlay');
      if (layer) return layer;
      layer = document.createElement('div');
      layer.className = 'chat-agent-overlay';
      container.appendChild(layer);
      return layer;
    },

    ensurePointerOrb(container, x, y) {
      var layer = this.resolvePointerFxLayer(container);
      if (!layer) return null;
      var orb = this._pointerOrbEl;
      if (!orb || !orb.isConnected || orb.parentNode !== layer) {
        if (orb) {
          try { orb.remove(); } catch(_) {}
        }
        orb = document.createElement('span');
        orb.className = 'chat-pointer-orb';
        layer.appendChild(orb);
        this._pointerOrbEl = orb;
      }
      orb.style.left = x + 'px';
      orb.style.top = y + 'px';
      return orb;
    },

    removePointerOrb() {
      var orb = this._pointerOrbEl;
      if (!orb) return;
      try { orb.remove(); } catch(_) {}
      this._pointerOrbEl = null;
    },

    handleMessagesPointerMove(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      this.startAgentTrailLoop(host);
      this.syncDirectHoverFromPointer(event);
      if (this.pointerFxThemeMode() !== 'dark') {
        this.removePointerOrb();
        return;
      }
      var now = Date.now();
      if ((now - Number(this._pointerTrailLastAt || 0)) < 8) return;
      this._pointerTrailLastAt = now;
      var rect = host.getBoundingClientRect();
      // Keep pointer FX in viewport coordinates so the mask remains visible
      // while reading scrolled chat history.
      var x = event.clientX - rect.left;
      var y = event.clientY - rect.top;
      host.style.setProperty('--chat-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-grid-y', Math.round(y) + 'px');
      host.style.setProperty('--chat-grid-active', '1');
      this.ensurePointerOrb(host, x, y);
      if (!this._pointerTrailSeeded) {
        this._pointerTrailLastX = x;
        this._pointerTrailLastY = y;
        this._pointerTrailSeeded = true;
      }
      var dx = x - Number(this._pointerTrailLastX || x);
      var dy = y - Number(this._pointerTrailLastY || y);
      var dist = Math.sqrt(dx * dx + dy * dy);
      // Denser sampling for a smoother neon trail.
      var spacing = 0.13;
      var steps = Math.max(1, Math.min(52, Math.ceil(dist / spacing)));
      for (var i = 1; i <= steps; i++) {
        var t0 = (i - 1) / steps;
        var t1 = i / steps;
        var sx0 = this._pointerTrailLastX + (dx * t0);
        var sy0 = this._pointerTrailLastY + (dy * t0);
        var sx1 = this._pointerTrailLastX + (dx * t1);
        var sy1 = this._pointerTrailLastY + (dy * t1);
        var progress = t1;
        var thickness = 2.05 + (progress * 1.85);
        var alpha = 0.32 + (progress * 0.45);
        var hueShift = -4 + (progress * 8);
        this.spawnPointerTrailSegment(host, sx0, sy0, sx1, sy1, {
