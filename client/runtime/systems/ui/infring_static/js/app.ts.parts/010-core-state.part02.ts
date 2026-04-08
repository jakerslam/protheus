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
