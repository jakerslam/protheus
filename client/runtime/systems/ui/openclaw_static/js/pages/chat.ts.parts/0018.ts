      }
      if (!visibleIndexes.length) return;

      var activePos = -1;
      var anchorDomId = String(this.selectedMessageDomId || '');
      if (anchorDomId) {
        for (var p = 0; p < visibleIndexes.length; p++) {
          var vi = visibleIndexes[p];
          if (this.messageDomId(list[vi], vi) === anchorDomId) {
            activePos = p;
            break;
          }
        }
      }
      if (activePos < 0) {
        for (var p2 = 0; p2 < visibleIndexes.length; p2++) {
          if (visibleIndexes[p2] === this.mapStepIndex) {
            activePos = p2;
            break;
          }
        }
      }

      if (activePos < 0) {
        activePos = dir > 0 ? 0 : (visibleIndexes.length - 1);
      } else {
        activePos = activePos + (dir > 0 ? 1 : -1);
        if (activePos < 0) activePos = 0;
        if (activePos > visibleIndexes.length - 1) activePos = visibleIndexes.length - 1;
      }

      var next = visibleIndexes[activePos];
      var msg = list[next];
      if (!msg) return;
      this.setHoveredMessage(msg, next);
      this.jumpToMessage(msg, next);
      this.centerChatMapOnMessage(this.messageDomId(msg, next));
      var self = this;
      this._mapPreviewSuppressTimer = setTimeout(function() {
        self.suppressMapPreview = false;
      }, 220);
    },

    setMapItemHover: function(msg, idx) {
      if (!msg) return;
      var domId = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      this.suppressMapPreview = false;
      this.activeMapPreviewDomId = domId;
      this.activeMapPreviewDayKey = '';
      this.selectedMessageDomId = domId;
      this.mapStepIndex = idx;
      this.setHoveredMessage(msg, idx);
    },

    clearMapItemHover: function() {
      this.activeMapPreviewDomId = '';
      this.clearHoveredMessage();
    },

    setMapDayHover: function(msg) {
      if (!msg) return;
      this.suppressMapPreview = false;
      this.activeMapPreviewDayKey = this.messageDayKey(msg);
      this.activeMapPreviewDomId = '';
    },

    clearMapDayHover: function() {
      this.activeMapPreviewDayKey = '';
    },

    isMapPreviewVisible: function(msg, idx) {
      if (this.suppressMapPreview) return false;
      if (!msg) return false;
      return this.activeMapPreviewDomId === this.messageDomId(msg, idx);
    },

    isMapDayPreviewVisible: function(msg) {
      if (this.suppressMapPreview) return false;
      if (!msg) return false;
      return this.activeMapPreviewDayKey === this.messageDayKey(msg);
    },

    setHoveredMessage: function(msg, idx) {
      if (!msg && msg !== 0) {
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      this.hoveredMessageDomId = this.messageDomId(msg, idx);
    },

    clearHoveredMessage: function() {
      this.hoveredMessageDomId = this.selectedMessageDomId || '';
    },

    clearHoveredMessageHard: function() {
      this.hoveredMessageDomId = '';
      this.selectedMessageDomId = '';
    },

    isHoveredMessage: function(msg, idx) {
      if (!this.hoveredMessageDomId) return false;
      return this.hoveredMessageDomId === this.messageDomId(msg, idx);
    },

    centerChatMapOnMessage: function(domId, options) {
      if (!domId) return;
      var immediate = !!(options && options.immediate);
      var map = null;
      var maps = document.querySelectorAll('.chat-map-scroll');
      for (var i = 0; i < maps.length; i++) {
        var candidate = maps[i];
        if (candidate && candidate.offsetParent !== null) {
          map = candidate;
          break;
        }
      }
      if (!map) return;
      var host = map.closest('.chat-map') || map;
      var item = host.querySelector('.chat-map-item[data-msg-dom-id="' + domId + '"]');
      if (!item) return;
      var topGuard = 28;
      var bottomGuard = 28;
      var viewport = Math.max(20, map.clientHeight - topGuard - bottomGuard);
      var desired = item.offsetTop + (item.offsetHeight / 2) - (viewport / 2) - topGuard;
      var max = Math.max(0, map.scrollHeight - map.clientHeight);
      var nextTop = Math.max(0, Math.min(max, desired));
      var diff = Math.abs(map.scrollTop - nextTop);
      if (diff < 3) return;
      map.scrollTo({ top: nextTop, behavior: (immediate || this.suppressMapPreview) ? 'auto' : 'smooth' });
    },

    filteredDrawerEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.drawerEmojiSearch || '').trim().toLowerCase();
      if (!query) return source.slice(0, 24);
      return source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    defaultFreshEmojiForAgent: function(agentRef) {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      if (!source.length) return '🤖';
      var key = '';
      if (agentRef && typeof agentRef === 'object') {
        key = String(agentRef.id || agentRef.name || '').trim();
      } else {
        key = String(agentRef || '').trim();
      }
      if (!key) return String((source[0] && source[0].emoji) || '🤖');
      var hash = 0;
      for (var idx = 0; idx < key.length; idx += 1) {
        hash = ((hash * 33) ^ key.charCodeAt(idx)) >>> 0;
      }
      var bucket = hash % source.length;
      return String((source[bucket] && source[bucket].emoji) || '🤖');
    },

    suggestedFreshIdentityForAgent: function(agentRef, templateDef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : {};
      var id = String(agent.id || agentRef || '').trim();
      var role = String(agent.role || '').trim().toLowerCase();
      var templateName = String(templateDef && templateDef.name ? templateDef.name : '').trim();
      var archetype = String(templateDef && templateDef.archetype ? templateDef.archetype : '').trim().toLowerCase();
      var seed = [id, role, templateName, archetype].join('|');
      var hash = 0;
      for (var idx = 0; idx < seed.length; idx += 1) {
        hash = ((hash * 31) ^ seed.charCodeAt(idx)) >>> 0;
      }
      var prefixOptions = [
        'Nimbus', 'Vector', 'Harbor', 'Atlas', 'Signal', 'Flux',
        'Forge', 'Scout', 'Axiom', 'Nova', 'Beacon', 'Cipher'
      ];
      var suffixByArchetype = {
        assistant: ['Guide', 'Assist', 'Navigator', 'Anchor'],
        coder: ['Builder', 'Compiler', 'Kernel', 'Patcher'],
        researcher: ['Analyst', 'Research', 'Surveyor', 'Trace'],
        writer: ['Draft', 'Scribe', 'Composer', 'Narrator'],
        devops: ['Ops', 'Reliability', 'Deploy', 'Runtime'],
        support: ['Support', 'Resolver', 'Bridge', 'Caretaker'],
        analyst: ['Insight', 'Signal', 'Vector', 'Ledger'],
        custom: ['Agent', 'Core', 'Node', 'Prime']
      };
      var suffixPool = suffixByArchetype[archetype] || suffixByArchetype[role] || suffixByArchetype.assistant;
      var prefix = prefixOptions[hash % prefixOptions.length];
      var suffix = suffixPool[(Math.floor(hash / 17) % suffixPool.length)];
      var emoji = this.defaultFreshEmojiForAgent(id || (prefix + '-' + suffix));
      if (templateDef && templateDef.category) {
        var category = String(templateDef.category).toLowerCase();
        if (category.indexOf('development') >= 0) emoji = '🧑\u200d💻';
        else if (category.indexOf('research') >= 0) emoji = '🔬';
        else if (category.indexOf('operations') >= 0 || category.indexOf('ops') >= 0) emoji = '🛠️';
        else if (category.indexOf('writing') >= 0) emoji = '📝';
      }
      return {
        name: (prefix + ' ' + suffix).trim(),
        emoji: String(emoji || '🤖').trim() || '🤖',
      };
    },

    toggleDrawerEmojiPicker: function() {
      this.drawerEmojiPickerOpen = !this.drawerEmojiPickerOpen;
      if (!this.drawerEmojiPickerOpen) {
        this.drawerEmojiSearch = '';
      } else {
        this.drawerEditingEmoji = true;
      }
    },

    selectDrawerEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      this.drawerConfigForm.emoji = emoji;
      // Choosing emoji explicitly switches away from image avatar mode.
      this.drawerConfigForm.avatar_url = '';
      if (this.agentDrawer && typeof this.agentDrawer === 'object') {
        this.agentDrawer.avatar_url = '';
      }
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerEditingEmoji = false;
    },

    openDrawerAvatarPicker: function() {
      if (this.$refs && this.$refs.drawerAvatarInput) {
        this.$refs.drawerAvatarInput.click();
      }
    },

    uploadDrawerAvatar: async function(fileList) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.drawerAvatarUploading = true;
      this.drawerAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.agentDrawer.id) + '/avatar', {
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
          var reason = payload && payload.error ? payload.error : 'avatar_upload_failed';
          throw new Error(String(reason));
        }
        if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
          this.drawerConfigForm = {};
        }
        this.drawerConfigForm.avatar_url = String(payload.avatar_url || '').trim();
        this.agentDrawer.avatar_url = String(payload.avatar_url || '').trim();
        this.drawerEditingEmoji = false;
        this.drawerEmojiPickerOpen = false;
        InfringToast.success('Avatar uploaded');
        await this.saveDrawerIdentity('avatar');
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.drawerAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.drawerAvatarUploading = false;
      }
    },

    async openAgentDrawer() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      this.showAgentDrawer = true;
      this.agentDrawerLoading = true;
      this.drawerTab = 'info';
      this.drawerEditingModel = false;
      this.drawerEditingProvider = false;
      this.drawerEditingFallback = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUploading = false;
      this.drawerAvatarUploadError = '';
      this.drawerIdentitySaving = false;
      this.drawerSavePending = false;
      this.drawerNewModelValue = '';
      this.drawerNewProviderValue = '';
      this.drawerNewFallbackValue = '';
      var base = this.resolveAgent(this.currentAgent) || this.currentAgent;
      this.agentDrawer = Object.assign({}, base, {
        _fallbacks: Array.isArray(base && base._fallbacks) ? base._fallbacks : []
      });
      this.drawerConfigForm = {
        name: this.agentDrawer.name || '',
        system_prompt: this.agentDrawer.system_prompt || '',
        emoji: (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '',
        avatar_url: this.agentDrawer.avatar_url || '',
        color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
        archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
        vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
      };
      try {
        var full = await InfringAPI.get('/api/agents/' + this.currentAgent.id);
        this.agentDrawer = Object.assign({}, base, full || {}, {
          _fallbacks: Array.isArray(full && full.fallback_models) ? full.fallback_models : []
        });
        this.drawerConfigForm = {
          name: this.agentDrawer.name || '',
          system_prompt: this.agentDrawer.system_prompt || '',
          emoji: (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '',
          avatar_url: this.agentDrawer.avatar_url || '',
          color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
          archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
          vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
        };
      } catch(e) {
        // Keep best-effort drawer data from current agent/store.
      } finally {
        this.agentDrawerLoading = false;
      }
    },

    closeAgentDrawer() {
      this.showAgentDrawer = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUploadError = '';
    },

    toggleAgentDrawer() {
      if (this.showAgentDrawer) {
        this.closeAgentDrawer();
        return;
      }
      this.openAgentDrawer();
    },

    async syncDrawerAgentAfterChange() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      try {
        await Alpine.store('app').refreshAgents();
      } catch {}
      var refreshed = this.resolveAgent(this.agentDrawer.id);
      if (refreshed) {
        this.currentAgent = refreshed;
      }
      await this.openAgentDrawer();
    },

    async setDrawerMode(mode) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      try {
        await InfringAPI.put('/api/agents/' + this.agentDrawer.id + '/mode', { mode: mode });
        InfringToast.success('Mode set to ' + mode);
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to set mode: ' + e.message);
      }
    },

    async saveDrawerAll() {
      if (!this.agentDrawer || !this.agentDrawer.id || this.drawerSavePending) return;
      var agentId = this.agentDrawer.id;
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      var requestedName = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      var previousFallbacks = Array.isArray(this.agentDrawer._fallbacks) ? this.agentDrawer._fallbacks.slice() : [];
      var appendedFallback = false;
      this.drawerSavePending = true;
      this.drawerConfigSaving = true;
      this.drawerModelSaving = true;
      this.drawerIdentitySaving = true;
      try {
        var configPayload = Object.assign({}, this.drawerConfigForm || {});
        if (this.drawerEditingFallback && String(this.drawerNewFallbackValue || '').trim()) {
          var fallbackParts = String(this.drawerNewFallbackValue || '').trim().split('/');
          var fallbackProvider = fallbackParts.length > 1 ? fallbackParts[0] : this.agentDrawer.model_provider;
          var fallbackModel = fallbackParts.length > 1 ? fallbackParts.slice(1).join('/') : fallbackParts[0];
