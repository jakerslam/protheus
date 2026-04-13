          if (typeof this.addNotification !== 'function') continue;

          var label = agentId === 'system' ? 'System' : ('Agent ' + agentId);
          if (Array.isArray(this.agents)) {
            var agent = this.agents.find(function(entry) {
              return entry && String(entry.id || '').trim() === agentId;
            });
            if (agent) {
              var agentName = String(agent.name || '').trim();
              if (agentName) label = agentName;
            }
          }
          var preview = this.agentChatPreviews && this.agentChatPreviews[agentId]
            ? this.agentChatPreviews[agentId]
            : null;
          var previewText = preview && typeof preview.text === 'string'
            ? preview.text.replace(/\s+/g, ' ').trim()
            : '';
          if (previewText.length > 120) previewText = previewText.slice(0, 117) + '...';
          var summary = previewText || 'posted a new update.';
          var message = previewText ? (label + ': ' + previewText) : (label + ' posted a new update.');

          this.addNotification({
            type: agentId === 'system' ? 'warn' : 'info',
            message: message,
            ts: now + noticesEmitted,
            source: 'session_activity',
            page: 'chat',
            agent_id: agentId,
            summary: summary
          });
          noticesEmitted += 1;
        }
        this._sessionActivityByAgent = nextMap;
        this._sessionActivityBootstrapped = true;
      } catch(_) {}
    },

    normalizeDashboardAssistantIdentity(payload) {
      var source = payload && typeof payload === 'object' ? payload : {};
      var name = normalizeDashboardOptionalString(
        source.name ||
        source.assistant_name ||
        source.display_name ||
        source.label
      );
      var avatar = normalizeDashboardOptionalString(
        source.avatar ||
        source.avatar_url ||
        source.assistant_avatar
      );
      var agentId = normalizeDashboardOptionalString(
        source.agent_id ||
        source.assistant_agent_id ||
        source.id
      );
      return {
        name: name || 'Assistant',
        avatar: avatar || '',
        agentId: agentId || ''
      };
    },

    applyBootstrapRuntimeState(statusObj, versionObj) {
      var status = statusObj && typeof statusObj === 'object' ? statusObj : {};
      var version = versionObj && typeof versionObj === 'object' ? versionObj : {};
      var assistantPayload =
        (status.assistant_identity && typeof status.assistant_identity === 'object' && status.assistant_identity) ||
        (status.assistant && typeof status.assistant === 'object' && status.assistant) ||
        (version.assistant_identity && typeof version.assistant_identity === 'object' && version.assistant_identity) ||
        (version.assistant && typeof version.assistant === 'object' && version.assistant) ||
        {
          name: status.assistant_name || version.assistant_name || '',
          avatar: status.assistant_avatar || version.assistant_avatar || '',
          agent_id: status.assistant_agent_id || version.assistant_agent_id || ''
        };
      var assistantIdentity = this.normalizeDashboardAssistantIdentity(assistantPayload);
      this.assistantName = assistantIdentity.name || this.assistantName || 'Assistant';
      this.assistantAvatar = assistantIdentity.avatar || this.assistantAvatar || null;
      this.assistantAgentId = assistantIdentity.agentId || this.assistantAgentId || null;

      var serverVersion = normalizeDashboardOptionalString(version.version || version.tag || status.version).replace(/^[vV]/, '');
      if (serverVersion) this.serverVersion = serverVersion;

      var previewRoots = status.local_media_preview_roots || version.local_media_preview_roots;
      if (!Array.isArray(previewRoots) && status.media && typeof status.media === 'object') {
        previewRoots = status.media.local_preview_roots;
      }
      if (!Array.isArray(previewRoots) && version.media && typeof version.media === 'object') {
        previewRoots = version.media.local_preview_roots;
      }
      if (Array.isArray(previewRoots)) {
        this.localMediaPreviewRoots = previewRoots
          .map(function(root) { return normalizeDashboardOptionalString(root); })
          .filter(function(root) { return !!root; });
      }

      var sandboxMode = normalizeDashboardOptionalString(
        status.embed_sandbox_mode ||
        (status.embed && status.embed.sandbox_mode) ||
        version.embed_sandbox_mode ||
        (version.embed && version.embed.sandbox_mode)
      );
      if (sandboxMode) this.embedSandboxMode = sandboxMode;

      var allowExternal = status.allow_external_embed_urls;
      if (typeof allowExternal !== 'boolean' && status.embed && typeof status.embed === 'object') {
        allowExternal = status.embed.allow_external_urls;
      }
      if (typeof allowExternal !== 'boolean') {
        allowExternal = version.allow_external_embed_urls;
      }
      if (typeof allowExternal !== 'boolean' && version.embed && typeof version.embed === 'object') {
        allowExternal = version.embed.allow_external_urls;
      }
      if (typeof allowExternal === 'boolean') this.allowExternalEmbedUrls = allowExternal;
    },

    focusTopbarSearchInput() {
      var self = this;
      if (this._topbarSearchFocusTimer) {
        clearTimeout(this._topbarSearchFocusTimer);
        this._topbarSearchFocusTimer = 0;
      }
      this._topbarSearchFocusTimer = window.setTimeout(function() {
        var input = document.getElementById('topbar-search-input');
        if (input && typeof input.focus === 'function') {
          input.focus({ preventScroll: true });
          if (typeof input.select === 'function') input.select();
        }
        self._topbarSearchFocusTimer = 0;
      }, 40);
    },

    openTopbarSearch() {
      this.topbarSearchOpen = false;
    },

    closeTopbarSearch() {
      this.topbarSearchOpen = false;
      if (this._topbarSearchFocusTimer) {
        clearTimeout(this._topbarSearchFocusTimer);
        this._topbarSearchFocusTimer = 0;
      }
    },

    toggleTopbarSearch() {
      this.topbarSearchOpen = false;
    },

    async checkOnboarding() {
      if (localStorage.getItem('infring-onboarded')) return;
      try {
        var config = await InfringAPI.get('/api/config');
        var apiKey = config && config.api_key;
        var noKey = !apiKey || apiKey === 'not set' || apiKey === '';
        if (noKey && this.agentCount === 0) {
          this.showOnboarding = true;
        }
      } catch(e) {
        // If config endpoint fails, still show onboarding if no agents
        if (this.agentCount === 0) this.showOnboarding = true;
      }
    },

    dismissOnboarding() {
      this.showOnboarding = false;
      localStorage.setItem('infring-onboarded', 'true');
    },

    async checkAuth() {
      try {
        // First check if session-based auth is configured
        var authInfo = await InfringAPI.get('/api/auth/check');
        if (authInfo.mode === 'none') {
          // No session auth — fall back to API key detection
          this.authMode = 'apikey';
          this.sessionUser = null;
        } else if (authInfo.mode === 'session') {
          this.authMode = 'session';
          if (authInfo.authenticated) {
            this.sessionUser = authInfo.username;
            this.showAuthPrompt = false;
            return;
          }
          // Session auth enabled but not authenticated — show login prompt
          this.showAuthPrompt = true;
          return;
        }
      } catch(e) { /* ignore — fall through to API key check */ }
