      this.loadModelNoticeCache();
      this.loadModelUsageCache();
      this.loadInputHistoryCache();

      // Start tip cycle
      this.startTipCycle();

      // Fetch dynamic commands from server
      this.fetchCommands();
      this.loadSlashAliases();
      this.fetchModelContextWindows();
      this.fetchProactiveTelemetryAlerts(false);
      this.refreshCurrentAgentSessionListIfStale = function(reason, maxAgeMs) {
        var agentId = String(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '').trim();
        if (!agentId) return;
        if (!self._sessionsLastLoadedAtByAgent || typeof self._sessionsLastLoadedAtByAgent !== 'object') {
          self._sessionsLastLoadedAtByAgent = {};
        }
        var normalizedAgentId = typeof self.normalizeSessionAgentId === 'function'
          ? self.normalizeSessionAgentId(agentId)
          : agentId.toLowerCase();
        var lastLoadedAt = Number(self._sessionsLastLoadedAtByAgent[normalizedAgentId] || 0);
        var ttlMs = Number(maxAgeMs || 0);
        if (!Number.isFinite(ttlMs) || ttlMs < 2000) ttlMs = 15000;
        if (lastLoadedAt > 0 && (Date.now() - lastLoadedAt) < ttlMs) return;
        Promise.resolve(self.loadSessions(agentId)).catch(function() { return []; });
      };
      this._chatFocusSessionRefreshHandler = function() {
        if (document && document.visibilityState && document.visibilityState === 'hidden') return;
        self.refreshCurrentAgentSessionListIfStale('focus', 15000);
      };
      this._chatVisibilitySessionRefreshHandler = function() {
        if (!document || document.visibilityState !== 'visible') return;
        self.refreshCurrentAgentSessionListIfStale('visibility', 15000);
      };
      window.addEventListener('focus', this._chatFocusSessionRefreshHandler);
      document.addEventListener('visibilitychange', this._chatVisibilitySessionRefreshHandler);

      // Ctrl+/ keyboard shortcut
      document.addEventListener('keydown', function(e) {
        var key = String(e && e.key ? e.key : '').toLowerCase();
        // Ctrl+T or Ctrl+\ toggles terminal compose mode.
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (key === 't' || key === '\\') && self.currentAgent) {
          e.preventDefault();
          self.toggleTerminalMode();
          return;
        }
        if ((e.ctrlKey || e.metaKey) && e.key === '/') {
          e.preventDefault();
          var input = document.getElementById('msg-input');
          if (input) { input.focus(); self.inputText = '/'; }
        }
        // Ctrl+M for model switcher
        if ((e.ctrlKey || e.metaKey) && e.key === 'm' && self.currentAgent) {
          e.preventDefault();
          self.toggleModelSwitcher();
        }
        // Ctrl+F opens file picker from chat compose.
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && key === 'f' && self.currentAgent) {
          e.preventDefault();
          if (self.terminalMode) {
            self.toggleTerminalMode();
          }
          self.showAttachMenu = true;
          self.$nextTick(function() {
            var input = self.$refs && self.$refs.fileInput ? self.$refs.fileInput : null;
            if (input && typeof input.click === 'function') input.click();
          });
          return;
        }
        // Ctrl+G for chat search
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && key === 'g' && self.currentAgent) {
          e.preventDefault();
          self.toggleSearch();
        }
      });

      if (this._sendWatchdogTimer) clearInterval(this._sendWatchdogTimer);
      this._sendWatchdogTimer = setInterval(function() {
        if (self.sending) self._reconcileSendingState();
      }, 3000);
      window.addEventListener('beforeunload', function() {
        self.handleMessagesPointerUp(null);
        if (self._sendWatchdogTimer) {
          clearInterval(self._sendWatchdogTimer);
          self._sendWatchdogTimer = null;
        }
        if (self._telemetryAlertsTimer) {
          clearInterval(self._telemetryAlertsTimer);
          self._telemetryAlertsTimer = null;
        }
        if (self._agentTrailListenTimer) {
          clearTimeout(self._agentTrailListenTimer);
          self._agentTrailListenTimer = 0;
        }
        self.teardownChatResizeBlurObserver();
        self.stopAgentTrailLoop(true);
        if (self._chatFocusSessionRefreshHandler) {
          window.removeEventListener('focus', self._chatFocusSessionRefreshHandler);
          self._chatFocusSessionRefreshHandler = null;
        }
        if (self._chatVisibilitySessionRefreshHandler) {
          document.removeEventListener('visibilitychange', self._chatVisibilitySessionRefreshHandler);
          self._chatVisibilitySessionRefreshHandler = null;
        }
      });

