      }
      var now = Date.now();
      var forced = this._forcedHydrateById || {};
      Object.keys(forced).forEach(function(id) {
        var until = Number(forced[id] || 0);
        if (until > now) {
          next[id] = true;
        } else {
          delete forced[id];
        }
      });
      if (this.selectedMessageDomId) next[this.selectedMessageDomId] = true;
      if (this.hoveredMessageDomId) next[this.hoveredMessageDomId] = true;
      this.messageHydration = next;
    },

    isFreshInitTemplateSelected(templateDef) {
      if (!templateDef) return false;
      var key = String(templateDef.name || '').trim();
      return !!key && key === String(this.freshInitTemplateName || '').trim();
    },

    freshInitTemplateDescription: function(templateDef) {
      if (!templateDef) return '';
      if (templateDef.is_other) {
        var typed = String(this.freshInitOtherPrompt || '').trim();
        if (typed) return this.truncateFreshInitSummary(typed, 86);
      }
      return String(templateDef.description || '').trim();
    },

    truncateFreshInitSummary: function(text, limit) {
      var clean = String(text || '').replace(/\s+/g, ' ').trim();
      if (!clean) return '';
      var max = Number(limit || 0);
      if (!Number.isFinite(max) || max < 12) max = 80;
      if (clean.length <= max) return clean;
      return clean.slice(0, Math.max(8, max - 1)).trimEnd() + '…';
    },

    filteredFreshInitEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.freshInitEmojiSearch || '').trim().toLowerCase();
      if (!query) return source.slice(0, 24);
      return source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    toggleFreshInitEmojiPicker: function() {
      this.freshInitEmojiPickerOpen = !this.freshInitEmojiPickerOpen;
      if (!this.freshInitEmojiPickerOpen) {
        this.freshInitEmojiSearch = '';
      }
    },

    selectFreshInitEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      this.freshInitEmoji = emoji;
      this.freshInitAvatarUrl = '';
      this.freshInitEmojiPickerOpen = false;
      this.freshInitEmojiSearch = '';
    },

    openFreshInitAvatarPicker: function() {
      if (this.$refs && this.$refs.freshInitAvatarInput) {
        this.$refs.freshInitAvatarInput.click();
      }
    },

    uploadFreshInitAvatar: async function(fileList) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.freshInitAvatarUploading = true;
      this.freshInitAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/avatar', {
          method: 'POST',
          headers: headers,
          body: file
        });
        var payload = null;
        try {
          payload = await response.json();
        } catch (_) {
          payload = null;
        }
        if (!response.ok || !payload || !payload.ok || !payload.avatar_url) {
          throw new Error(String(payload && payload.error ? payload.error : 'avatar_upload_failed'));
        }
        this.freshInitAvatarUrl = String(payload.avatar_url || '').trim();
        this.freshInitEmojiPickerOpen = false;
        this.freshInitEmojiSearch = '';
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.freshInitAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.freshInitAvatarUploading = false;
      }
    },

    clearFreshInitAvatar: function() {
      this.freshInitAvatarUrl = '';
      this.freshInitAvatarUploadError = '';
    },

    isFreshInitPersonalitySelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitPersonalityId || '');
    },

    selectFreshInitPersonality: function(card) {
      var id = String(card && card.id ? card.id : 'none').trim() || 'none';
      this.freshInitPersonalityId = id;
    },

    selectedFreshInitPersonality: function() {
      var cards = Array.isArray(this.freshInitPersonalityCards) ? this.freshInitPersonalityCards : [];
      var selectedId = String(this.freshInitPersonalityId || 'none');
      for (var i = 0; i < cards.length; i += 1) {
        if (String(cards[i] && cards[i].id ? cards[i].id : '') === selectedId) return cards[i];
      }
      return cards.length ? cards[0] : null;
    },

    isFreshInitLifespanSelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitLifespanId || '');
    },

    selectFreshInitLifespan: function(card) {
      var id = String(card && card.id ? card.id : '1h').trim() || '1h';
      this.freshInitLifespanId = id;
    },

    selectedFreshInitLifespan: function() {
      var cards = Array.isArray(this.freshInitLifespanCards) ? this.freshInitLifespanCards : [];
      var selectedId = String(this.freshInitLifespanId || '1h');
      var fallback = null;
      for (var i = 0; i < cards.length; i += 1) {
        var cardId = String(cards[i] && cards[i].id ? cards[i].id : '');
        if (cardId === '1h') fallback = cards[i];
        if (cardId === selectedId) return cards[i];
      }
      return fallback || (cards.length ? cards[0] : null);
    },

    async applyChatArchetypeTemplate(templateDef) {
      if (!templateDef) return;
      this.freshInitTemplateDef = templateDef;
      this.freshInitTemplateName = String(templateDef.name || '').trim();
      this.freshInitModelManual = false;
      this.freshInitModelSelection = '';
      this.refreshFreshInitModelSuggestions(templateDef);
      if (templateDef.is_other) {
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit(String(this.freshInitOtherPrompt || '').trim());
      } else {
        this.freshInitAwaitingOtherPrompt = false;
      }
    },

    captureFreshInitOtherPrompt: function() {
      if (!this.showFreshArchetypeTiles || !this.freshInitAwaitingOtherPrompt) return false;
      if (Array.isArray(this.attachments) && this.attachments.length > 0) {
        InfringToast.info('Init prompt does not support file attachments.');
        return false;
      }
      var text = String(this.inputText || '').trim();
      if (!text) {
        InfringToast.info('Describe the special purpose first.');
        this.focusChatComposerFromInit('');
        return false;
      }
      this.freshInitOtherPrompt = text;
      this.freshInitAwaitingOtherPrompt = false;
      this.inputText = '';
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';
      return true;
    },

    resolveFreshInitSystemPrompt: function(templateDef, agentName, personalityCard, vibeCard) {
      if (!templateDef) return '';
      var basePrompt = '';
      if (templateDef.is_other) {
        var purpose = String(this.freshInitOtherPrompt || '').trim();
        basePrompt = [
          'You are ' + String(agentName || 'the assistant') + '.',
          'Special purpose: ' + purpose,
          'Act as a focused specialist for this purpose. Stay concise, practical, and reliable.',
        ].join('\n');
      } else {
        basePrompt = String(templateDef.system_prompt || '').trim();
      }
      var personalitySuffix = String(personalityCard && personalityCard.system_suffix ? personalityCard.system_suffix : '').trim();
      var vibeSuffix = String(vibeCard && vibeCard.system_suffix ? vibeCard.system_suffix : '').trim();
      var suffixes = [];
      if (personalitySuffix) suffixes.push(personalitySuffix);
      if (vibeSuffix) suffixes.push(vibeSuffix);
      if (suffixes.length) {
        return (basePrompt ? (basePrompt + '\n\n') : '') + suffixes.join('\n');
      }
      return basePrompt;
    },

    resolveFreshInitContractPayload: function(agentName) {
      var selected = this.selectedFreshInitLifespan();
      var mission = 'Initialize and run as ' + String(agentName || 'agent') + '.';
      if (!selected) {
        return {
          mission: mission,
          termination_condition: 'task_or_timeout',
          expiry_seconds: 60 * 60,
          indefinite: false,
          auto_terminate_allowed: true,
        };
      }
      var terminationCondition = String(selected.termination_condition || 'task_or_timeout');
      var expirySeconds = selected.expiry_seconds == null ? null : Number(selected.expiry_seconds);
      var indefinite = selected.indefinite === true;
      var supportsTimeout = terminationCondition === 'timeout' || terminationCondition === 'task_or_timeout';
      return {
        mission: mission,
        termination_condition: terminationCondition,
        expiry_seconds: expirySeconds,
        indefinite: indefinite,
        auto_terminate_allowed: !indefinite && supportsTimeout && expirySeconds != null,
      };
    },

    async launchFreshAgentInitialization() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.freshInitLaunching) return;
      if (!this.freshInitTemplateDef) {
        InfringToast.info('Select an archetype first.');
        return;
      }
      var agentId = this.currentAgent.id;
      var templateDef = this.freshInitTemplateDef;
      var provider = String(templateDef.provider || '').trim();
      var model = String(templateDef.model || '').trim();
      var selectedModel = this.selectedFreshInitModelSuggestion();
      var selectedModelRef = this.normalizeFreshInitModelRef(selectedModel);
      if (!selectedModelRef) {
        await this.refreshFreshInitModelSuggestions(templateDef);
        selectedModel = this.selectedFreshInitModelSuggestion();
        selectedModelRef = this.normalizeFreshInitModelRef(selectedModel);
      }
      var resolvedModelRef = selectedModelRef;
      if (!resolvedModelRef && provider && model) resolvedModelRef = provider.toLowerCase() + '/' + model;
      var agentName = String(this.freshInitName || '').trim() || String(this.currentAgent.name || this.currentAgent.id || '').trim() || String(agentId);
      var agentEmoji = String(this.freshInitEmoji || '').trim() || this.defaultFreshEmojiForAgent(agentId);
      var defaultName = String(this.freshInitDefaultName || '').trim();
      var defaultEmoji = String(this.freshInitDefaultEmoji || '').trim();
      if (!agentName) {
        agentName = String(this.currentAgent.name || this.currentAgent.id || agentId).trim();
      }
      var existingAgentEmoji = String(
        (this.currentAgent && this.currentAgent.identity && this.currentAgent.identity.emoji) || ''
      ).trim();
      if (!agentEmoji) {
        agentEmoji = existingAgentEmoji || this.defaultFreshEmojiForAgent(agentId);
      }
      if (templateDef.is_other && !String(this.freshInitOtherPrompt || '').trim()) {
        InfringToast.info('Describe the special purpose for Other before launch.');
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit('');
        return;
      }
      var selectedPersonality = this.selectedFreshInitPersonality();
      var selectedVibe = this.selectedFreshInitVibe();
      var resolvedSystemPrompt = this.resolveFreshInitSystemPrompt(templateDef, agentName, selectedPersonality, selectedVibe);
      var resolvedContract = this.resolveFreshInitContractPayload(agentName);
      this.freshInitName = agentName;
      this.freshInitEmoji = agentEmoji;
      this.freshInitLaunching = true;
      try {
        if (resolvedModelRef) {
          await InfringAPI.put('/api/agents/' + agentId + '/model', {
            model: resolvedModelRef
          });
        }
        var identityPayload = {};
        if (!defaultEmoji || agentEmoji !== defaultEmoji) {
          identityPayload.emoji = agentEmoji;
        }
        var vibeValue = String(selectedVibe && selectedVibe.id ? selectedVibe.id : '').trim();
        var personalityVibe = String(selectedPersonality && selectedPersonality.vibe ? selectedPersonality.vibe : '').trim();
        if (vibeValue && vibeValue !== 'none') {
          identityPayload.vibe = vibeValue;
        } else if (personalityVibe && personalityVibe !== 'none') {
          identityPayload.vibe = personalityVibe;
        }
        var configPayload = {
          system_prompt: resolvedSystemPrompt,
          archetype: String(templateDef.archetype || '').trim(),
          profile: String(templateDef.profile || '').trim(),
          contract: resolvedContract,
          termination_condition: resolvedContract.termination_condition,
          expiry_seconds: resolvedContract.expiry_seconds,
          indefinite: resolvedContract.indefinite === true,
        };
        if (!defaultName || agentName !== defaultName) {
          configPayload.name = agentName;
        }
        if (Object.keys(identityPayload).length > 0) {
          configPayload.identity = identityPayload;
        }
        if (this.freshInitAvatarUrl) {
          configPayload.avatar_url = String(this.freshInitAvatarUrl || '').trim();
        }
        await InfringAPI.patch('/api/agents/' + agentId + '/config', {
          ...configPayload
        });
        this.addNoticeEvent({
          notice_label: 'Initialized ' + agentName + ' as ' + String(templateDef.name || 'template'),
          notice_type: 'info',
          ts: Date.now()
        });
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this.freshInitRevealMenu = false;
        this.showFreshArchetypeTiles = false;
        this.freshInitTemplateDef = null;
        this.freshInitTemplateName = '';
        this.freshInitLaunching = false;
        try {
          var store = Alpine.store('app');
          if (store) {
            store.pendingFreshAgentId = null;
            if (typeof store.refreshAgents === 'function') {
              await store.refreshAgents();
            }
          }
        } catch(_) {}
        await this.syncDrawerAgentAfterChange();
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        InfringToast.success('Launched ' + String(templateDef.name || 'agent setup'));
      } catch (e) {
        this.freshInitLaunching = false;
        InfringToast.error('Failed to initialize agent: ' + e.message);
      }
    },

    async loadSession(agentId, keepCurrent) {
      var self = this;
      var loadSeq = ++this._sessionLoadSeq;
      this.sessionLoading = true;
      var targetAgentId = String(agentId || '');
      var loadStillCurrent = function() {
        if (self._sessionLoadSeq !== loadSeq) return false;
        if (!self.currentAgent || !self.currentAgent.id) return true;
        return String(self.currentAgent.id || '') === targetAgentId;
      };
      try {
        var preserveFreshInit = self.isFreshInitInProgressFor(agentId);
        var data = await InfringAPI.get('/api/agents/' + agentId + '/session');
        if (!loadStillCurrent()) return;
        if (self.currentAgent && String(self.currentAgent.id || '') === String(agentId || '')) {
          self.applyAgentGitTreeState(self.currentAgent, data || {});
        }
        var normalized = self.mergeModelNoticesForAgent(agentId, self.normalizeSessionMessages(data));
        if (!loadStillCurrent()) return;
        if (normalized.length) {
          if (!preserveFreshInit) {
            self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
            self.freshInitRevealMenu = false;
            self.showFreshArchetypeTiles = false;
          }
          // Always prefer server-authoritative session state over potentially stale cache.
          self.messages = normalized;
          self.clearHoveredMessageHard();
          self.activeMapPreviewDomId = '';
          self.activeMapPreviewDayKey = '';
          self.recomputeContextEstimate();
          self.cacheAgentConversation(agentId);
          self.$nextTick(function() {
            self.scrollToBottomImmediate();
            self.stabilizeBottomScroll();
            self.pinToLatestOnOpen(null, { maxFrames: 20 });
          });
        } else if (!keepCurrent) {
          if (!preserveFreshInit) {
            self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
            self.freshInitRevealMenu = false;
            self.showFreshArchetypeTiles = false;
            self.messages = [];
            self.clearHoveredMessageHard();
            self.activeMapPreviewDomId = '';
            self.activeMapPreviewDayKey = '';
            self.recomputeContextEstimate();
            self.recoverEmptySessionRender(agentId, data || null);
          }
