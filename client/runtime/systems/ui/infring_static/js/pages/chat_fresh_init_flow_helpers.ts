'use strict';

function infringChatFreshInitFlowMethods() {
  return {
    getPendingFreshAgentIdFromShellStore: function() {
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
      return store && store.pendingFreshAgentId ? String(store.pendingFreshAgentId) : '';
    },

    maybeDiscardPendingFreshAgent: function(nextAgentId) {
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
      if (!store) return;
      var pendingId = String(store.pendingFreshAgentId || '').trim();
      if (!pendingId) return;
      var targetId = String(nextAgentId || '').trim();
      if (!targetId || targetId === pendingId) return;
      if (bridge && typeof bridge.assign === 'function') bridge.assign({ pendingFreshAgentId: null, pendingAgent: null });
      else {
        store.pendingFreshAgentId = null;
        store.pendingAgent = null;
      }
      InfringAPI.del('/api/agents/' + encodeURIComponent(pendingId)).catch(function() {});
      var refreshAgents = bridge && typeof bridge.method === 'function'
        ? bridge.method('refreshAgents')
        : null;
      if (typeof refreshAgents === 'function') {
        setTimeout(function() { refreshAgents({ force: true }).catch(function() {}); }, 0);
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
      this.freshInitModelSuggestLoading = false; if (typeof this.resetFreshInitPermissions === 'function') this.resetFreshInitPermissions();
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
      if (data && data.message_window && Array.isArray(data.message_window.rows) && data.message_window.rows.length > 0) return true;
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

    agentHasInitialContract: function(agentRef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : null;
      if (!agent) return false;
      var systemPrompt = String(agent.system_prompt || '').trim();
      if (systemPrompt) return true;
      var contract = agent.contract && typeof agent.contract === 'object' ? agent.contract : null;
      if (!contract) return false;
      var initialPrompt = String(
        contract.initial_prompt ||
        contract.initialPrompt ||
        contract.prompt ||
        ''
      ).trim();
      return !!initialPrompt;
    },

    recoverEmptySessionRender: function(agentId, sessionPayload) {
      var targetId = String(agentId || '').trim();
      if (!targetId) return;
      if (this.isFreshInitInProgressFor(targetId)) return;
      var resolved =
        this.resolveAgent(targetId) ||
        (this.currentAgent && String(this.currentAgent.id || '') === targetId ? this.currentAgent : null);
      if (
        !this.sessionHasAnyHistory(sessionPayload) &&
        resolved &&
        resolved.id &&
        !this.agentHasInitialContract(resolved)
      ) {
        this.ensureFreshInitThread(resolved);
        return;
      }
      this.messages = [{
        id: ++msgId,
        role: 'notice',
        text: '',
        meta: '',
        tools: [],
        is_notice: true,
        notice_label: 'This session is empty. Send a message to begin.',
        notice_type: 'info',
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
      var resolved =
        this.resolveAgent(targetId) ||
        (this.currentAgent && String(this.currentAgent.id || '') === targetId ? this.currentAgent : null);
      if (resolved && this.agentHasInitialContract(resolved)) {
        try {
          var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
            ? InfringSharedShellServices.appStore
            : null;
          var appStore = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
          var pendingForResolved = String(appStore && appStore.pendingFreshAgentId ? appStore.pendingFreshAgentId : '').trim();
          if (pendingForResolved === targetId) {
            if (bridge && typeof bridge.set === 'function') bridge.set('pendingFreshAgentId', '');
            else appStore.pendingFreshAgentId = '';
          }
        } catch(_) {}
        return false;
      }
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
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
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
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
        var pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
        var currentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
        if (pendingFreshId && currentId && pendingFreshId === targetId && currentId === targetId) {
          return true;
        }
      } catch(_) {}
      return false;
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
      var requestedName = String(this.freshInitName || '').trim();
      var requestedEmoji = String(this.freshInitEmoji || '').trim();
      var launchName = requestedName || 'agent';
      if (templateDef.is_other && !String(this.freshInitOtherPrompt || '').trim()) {
        InfringToast.info('Describe the special purpose for Other before launch.');
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit('');
        return;
      }
      var selectedPersonality = this.selectedFreshInitPersonality();
      var selectedVibe = this.selectedFreshInitVibe();
      var resolvedSystemPrompt = this.resolveFreshInitSystemPrompt(templateDef, launchName, selectedPersonality, selectedVibe);
      var resolvedContract = this.resolveFreshInitContractPayload(launchName);
      var resolvedPermissions = this.resolveFreshInitPermissionManifest ? this.resolveFreshInitPermissionManifest() : null;
      if (resolvedPermissions && typeof resolvedPermissions === 'object') resolvedContract.permissions_manifest = resolvedPermissions;
      this.freshInitLaunching = true;
      this.freshInitRevealMenu = false;
      this.freshInitEmojiPickerOpen = false;
      try {
        if (resolvedModelRef) {
          await InfringAPI.put('/api/agents/' + agentId + '/model', {
            model: resolvedModelRef
          });
        }
        var sanitizedRequestedEmoji = this.sanitizeAgentEmojiForDisplay
          ? this.sanitizeAgentEmojiForDisplay(this.currentAgent, requestedEmoji || '')
          : (requestedEmoji || '');
        var identityPayload = {};
        if (String(sanitizedRequestedEmoji || '').trim()) {
          identityPayload.emoji = String(sanitizedRequestedEmoji || '').trim();
        }
        var vibeValue = String(selectedVibe && selectedVibe.id ? selectedVibe.id : '').trim();
        if (vibeValue && vibeValue !== 'none') identityPayload.vibe = vibeValue;
        var configPayload = {
          role: this.resolveFreshInitRole(templateDef),
          identity: identityPayload,
          system_prompt: resolvedSystemPrompt,
          archetype: String(templateDef.archetype || '').trim(),
          profile: String(templateDef.profile || '').trim(),
          contract: resolvedContract,
          termination_condition: resolvedContract.termination_condition,
          expiry_seconds: resolvedContract.expiry_seconds,
          indefinite: resolvedContract.indefinite === true,
        };
        if (requestedName) {
          configPayload.name = requestedName;
        }
        if (!Object.keys(identityPayload).length) {
          delete configPayload.identity;
        }
        if (this.freshInitAvatarUrl) {
          configPayload.avatar_url = String(this.freshInitAvatarUrl || '').trim();
        }
        await InfringAPI.patch('/api/agents/' + agentId + '/config', {
          ...configPayload
        });
        var appliedAgentName = requestedName || String(this.currentAgent.name || this.currentAgent.id || agentId).trim() || 'agent';
        this.addNoticeEvent({
          notice_label: 'Initialized ' + appliedAgentName + ' as ' + String(templateDef.name || 'template'),
          notice_type: 'info',
          ts: Date.now()
        });
        try {
          var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
            ? InfringSharedShellServices.appStore
            : null;
          if (bridge && typeof bridge.assign === 'function') {
            bridge.assign({ pendingFreshAgentId: null, pendingAgent: null });
          }
          var refreshAgents = bridge && typeof bridge.method === 'function'
            ? bridge.method('refreshAgents')
            : null;
          if (typeof refreshAgents === 'function') {
            await refreshAgents();
          }
        } catch(_) {}
        await this.syncDrawerAgentAfterChange();
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this.showFreshArchetypeTiles = false;
        this.freshInitTemplateDef = null;
        this.freshInitTemplateName = '';
        this.freshInitLaunching = false; if (typeof this.resetFreshInitPermissions === 'function') this.resetFreshInitPermissions();
        var launchedRole = String((templateDef && (templateDef.name || templateDef.profile || templateDef.archetype)) || 'agent').trim() || 'agent';
        InfringToast.success('Launched ' + String(appliedAgentName || 'agent') + ' as ' + launchedRole);
      } catch (e) {
        this.freshInitLaunching = false;
        this.freshInitRevealMenu = true;
        InfringToast.error('Failed to initialize agent: ' + e.message);
      }
    },

  };
}
