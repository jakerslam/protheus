        this.conversationCache = Object.assign({}, persistedCache, runtimeCache);
        window.__infringChatCache = this.conversationCache;
      }
      this.loadModelNoticeCache();
      this.loadModelUsageCache();

      // Start tip cycle
      this.startTipCycle();

      // Fetch dynamic commands from server
      this.fetchCommands();
      this.fetchModelContextWindows();

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
        if (self._agentTrailListenTimer) {
          clearTimeout(self._agentTrailListenTimer);
          self._agentTrailListenTimer = 0;
        }
        self.stopAgentTrailLoop(true);
      });

      // Load session + session list when agent changes
      this.$watch('currentAgent', function(agent) {
        if (agent) {
          self.loadSessions(agent.id);
          self.setContextWindowFromCurrentAgent();
          self.requestContextTelemetry(true);
          self.refreshPromptSuggestions(false);
          self.checkForSystemReleaseUpdate(false);
        } else {
          self.clearPromptSuggestions();
        }
      });

      // Check for pending agent from Agents page (set before chat mounted)
      var store = Alpine.store('app');
      if (store.pendingAgent) {
        self.selectAgent(store.pendingAgent);
        store.pendingAgent = null;
      } else if (store.activeAgentId) {
        self.selectAgent(store.activeAgentId);
      } else {
        var preferred = self.pickDefaultAgent(store.agents || []);
        if (preferred) self.selectAgent(preferred);
      }

      // Watch for future pending agent selections (e.g., user clicks agent while on chat)
      this.$watch('$store.app.pendingAgent', function(agent) {
        if (agent) {
          self.selectAgent(agent);
          Alpine.store('app').pendingAgent = null;
        }
      });

      // Keep chat selection synced when an explicit active agent is set globally.
      this.$watch('$store.app.activeAgentId', function(agentId) {
        if (!agentId) return;
        if (!self.currentAgent || self.currentAgent.id !== agentId) {
          self.selectAgent(agentId);
        }
      });

      // Auto-select the first available agent in chat mode.
      this.$watch('$store.app.agents', function(agents) {
        var store = Alpine.store('app');
        var rows = Array.isArray(agents) ? agents : [];
        self.fetchModelContextWindows();
        if (self.currentAgent && self.currentAgent.id) {
          var currentLive = null;
          for (var ai = 0; ai < rows.length; ai++) {
            if (rows[ai] && String(rows[ai].id) === String(self.currentAgent.id)) {
              currentLive = rows[ai];
              break;
            }
          }
          if (!currentLive) {
            if (self.shouldSuppressAgentInactive(self.currentAgent.id)) return;
            var connectionState = String((store && store.connectionState) || '').toLowerCase();
            if (connectionState && connectionState !== 'connected') return;
            var currentId = String(self.currentAgent.id);
            var now = Date.now();
            if (self._agentMissingAgentId !== currentId) {
              self._agentMissingAgentId = currentId;
              self._agentMissingSince = now;
              return;
            }
            var missingForMs = self._agentMissingSince > 0 ? now - self._agentMissingSince : 0;
            var graceMs = Number(self._agentMissingGraceMs || 0);
            if (graceMs > 0 && missingForMs < graceMs) return;
            self._agentMissingAgentId = '';
            self._agentMissingSince = 0;
            self.handleAgentInactive(self.currentAgent.id, 'inactive', { silentNotice: true });
          } else {
            self._agentMissingAgentId = '';
            self._agentMissingSince = 0;
            if (!self.syncCurrentAgentFromStore(currentLive)) {
              self.currentAgent = currentLive;
            }
          }
        }
        if (store.activeAgentId) {
          var resolved = self.resolveAgent(store.activeAgentId);
          if (resolved) {
            if (!self.currentAgent || self.currentAgent.id !== resolved.id) {
              self.selectAgent(resolved);
            } else {
              // Refresh visible metadata without resetting the thread.
              self.syncCurrentAgentFromStore(resolved);
            }
            return;
          }
        }
        if (!self.currentAgent) {
          var preferred = self.pickDefaultAgent(agents || []);
          if (preferred) self.selectAgent(preferred);
        }
      });

      // Watch for slash commands + model autocomplete
      this.$watch('inputText', function(val) {
        var hasTyping = String(val == null ? '' : val).length > 0;
        if (self._agentTrailListenTimer) {
          clearTimeout(self._agentTrailListenTimer);
          self._agentTrailListenTimer = 0;
        }
        if (hasTyping) {
          self._agentTrailListening = true;
          if (self._agentTrailOrbEl && self._agentTrailOrbEl.classList) self._agentTrailOrbEl.classList.add('agent-listening');
          if (self._agentTrailRaf) {
            try { cancelAnimationFrame(self._agentTrailRaf); } catch(_) {}
            self._agentTrailRaf = 0;
          }
          self._agentTrailListenTimer = setTimeout(function() {
            self._agentTrailListenTimer = 0;
            self._agentTrailListening = false;
            if (self._agentTrailOrbEl && self._agentTrailOrbEl.classList) self._agentTrailOrbEl.classList.remove('agent-listening');
            self.startAgentTrailLoop();
          }, 420);
        } else if (self._agentTrailListening) {
          self._agentTrailListening = false;
          if (self._agentTrailOrbEl && self._agentTrailOrbEl.classList) self._agentTrailOrbEl.classList.remove('agent-listening');
          if (!self._agentTrailRaf) self.startAgentTrailLoop();
        }
        if (self.terminalMode) {
          self.updateTerminalCursor();
          self.showSlashMenu = false;
          self.showModelPicker = false;
          return;
        }
        var modelMatch = val.match(/^\/model\s+(.*)$/i);
        if (modelMatch) {
          self.showSlashMenu = false;
          self.modelPickerFilter = modelMatch[1].toLowerCase();
          if (!self.modelPickerList.length) {
            InfringAPI.get('/api/models').then(function(data) {
              self.modelPickerList = (data.models || []).filter(function(m) { return m.available; });
              self.showModelPicker = true;
              self.modelPickerIdx = 0;
            }).catch(function() {});
          } else {
            self.showModelPicker = true;
          }
        } else if (val.startsWith('/')) {
          self.showModelPicker = false;
          self.slashFilter = val.slice(1).toLowerCase();
          self.showSlashMenu = true;
          self.slashIdx = 0;
        } else {
          self.showSlashMenu = false;
          self.showModelPicker = false;
        }
      });

      this.$nextTick(function() {
        self.handleMessagesScroll();
        self.startAgentTrailLoop();
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
      });

      InfringAPI.get('/api/status').then(function(status) {
        var suggested = status && (status.workspace_dir || status.root_dir || status.home_dir)
          ? String(status.workspace_dir || status.root_dir || status.home_dir)
          : '';
        if (suggested) self.terminalCwd = suggested;
      }).catch(function() {});

      if (this._contextTelemetryTimer) clearInterval(this._contextTelemetryTimer);
      this._contextTelemetryTimer = setInterval(function() {
        self.requestContextTelemetry(false);
      }, 8000);
    },

    toggleTerminalMode() {
      this.terminalMode = !this.terminalMode;
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showModelSwitcher = false;
      this.terminalCursorFocused = false;
      if (!this.terminalMode) this.terminalSelectionStart = 0;
      if (this.terminalMode && !this.terminalCwd) {
        this.terminalCwd = '/workspace';
      }
      if (this.terminalMode && this.currentAgent) {
        this.connectWs(this.currentAgent.id);
      }
      if (this.terminalMode && Array.isArray(this.attachments) && this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          if (this.attachments[i] && this.attachments[i].preview) {
            try { URL.revokeObjectURL(this.attachments[i].preview); } catch(_) {}
          }
        }
        this.attachments = [];
      }
      var self = this;
      this.$nextTick(function() {
        var input = document.getElementById('msg-input');
        if (input) {
          input.focus();
          if (self.terminalMode) {
            self.setTerminalCursorFocus(true, { target: input });
            self.updateTerminalCursor({ target: input });
          }
        }
        self.scheduleConversationPersist();
      });
    },

    setTerminalCursorFocus(active, event) {
      if (!this.terminalMode) {
        this.terminalCursorFocused = false;
        return;
      }
      this.terminalCursorFocused = !!active;
      if (this.terminalCursorFocused) this.updateTerminalCursor(event);
    },

    updateTerminalCursor(event) {
      if (!this.terminalMode) {
        this.terminalSelectionStart = 0;
        return;
      }
      var text = String(this.inputText || '');
      var active = (typeof document !== 'undefined' && document.activeElement && document.activeElement.id === 'msg-input')
        ? document.activeElement
        : null;
      var el = event && event.target ? event.target : (active || document.getElementById('msg-input'));
      var pos = text.length;
      if (el && Number.isFinite(Number(el.selectionStart))) pos = Number(el.selectionStart);
      if (!Number.isFinite(pos) || pos < 0) pos = text.length;
      if (pos > text.length) pos = text.length;
      this.terminalSelectionStart = Math.floor(pos);
    },

    installChatMapWheelLock() {
      var maps = document.querySelectorAll('.chat-map-scroll');
      if (!maps || !maps.length) return;
      for (var i = 0; i < maps.length; i++) {
        var map = maps[i];
        if (!map || map.__ofWheelLock) continue;
        map.__ofWheelLock = true;
        map.addEventListener('wheel', function(ev) {
          var target = ev.currentTarget;
          if (!target) return;
          if (!target.matches(':hover')) return;
          // Keep wheel behavior scoped to chat map so the page does not scroll beneath it.
          var delta = Number(ev.deltaY || 0);
          if (delta !== 0) {
            target.scrollTop += delta;
          }
          ev.preventDefault();
        }, { passive: false });
      }
    },

    anchorAgentTrailToThinking(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelectorAll !== 'function') return false;
      var bubbles = host.querySelectorAll('.message.thinking .message-bubble.message-bubble-thinking');
      if (!bubbles || !bubbles.length) {
        if (this._agentTrailOrbEl && this._agentTrailOrbEl.classList && !this._agentTrailListening) this._agentTrailOrbEl.classList.remove('agent-listening');
        return false;
      }
      var rect = hostRect && Number.isFinite(Number(hostRect.width || 0)) ? hostRect : host.getBoundingClientRect();
      var anchor = null;
      for (var i = bubbles.length - 1; i >= 0; i--) {
        var bubble = bubbles[i];
        if (!bubble || bubble.offsetParent === null) continue;
        var bubbleRect = bubble.getBoundingClientRect();
        if (!(Number(bubbleRect.width || 0) > 0 && Number(bubbleRect.height || 0) > 0)) continue;
        if (bubbleRect.bottom < rect.top || bubbleRect.top > rect.bottom || bubbleRect.right < rect.left || bubbleRect.left > rect.right) continue;
        // Pin the autonomous agent orb ~1rem from the bottom-left of the
        // active thinking dialog while the agent is working.
        anchor = { x: (bubbleRect.left - rect.left) + 16, y: (bubbleRect.bottom - rect.top) - 16 };
        break;
      }
      if (!anchor) {
        if (this._agentTrailOrbEl && this._agentTrailOrbEl.classList && !this._agentTrailListening) this._agentTrailOrbEl.classList.remove('agent-listening');
        return false;
      }
      var x = Math.max(pad + 1, Math.min(w - (pad + 1), Number(anchor.x || 0)));
      var y = Math.max(pad + 1, Math.min(h - (pad + 1), Number(anchor.y || 0)));
      var orb = this.ensureAgentTrailOrb(host, x, y);
      if (orb && orb.classList) orb.classList.add('agent-listening');
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailState = { x: x, y: y, vx: 0, vy: 0, dir: 0, target: 0, turnAt: now + 1000 };
      this._agentTrailSeeded = false;
      this._agentTrailLastAt = now;
      return true;
    },

    get filteredModelPicker() {
      if (!this.modelPickerFilter) return this.modelPickerList.slice(0, 15);
      var f = this.modelPickerFilter;
      return this.modelPickerList.filter(function(m) {
        return m.id.toLowerCase().indexOf(f) !== -1 || (m.display_name || '').toLowerCase().indexOf(f) !== -1 || m.provider.toLowerCase().indexOf(f) !== -1;
      }).slice(0, 15);
    },

    pickModel(modelId) {
      this.showModelPicker = false;
      this.inputText = '/model ' + modelId;
      this.sendMessage();
    },

    toggleModelSwitcher() {
      if (this.showModelSwitcher) { this.showModelSwitcher = false; return; }
      var self = this;
      var now = Date.now();
      this.modelApiKeyStatus = '';
      if (this._modelCache && (now - this._modelCacheTime) < 300000) {
        this.modelSwitcherFilter = '';
        this.modelSwitcherProviderFilter = '';
        this.modelSwitcherIdx = 0;
        this.showModelSwitcher = true;
        this.$nextTick(function() {
          var el = document.getElementById('model-switcher-search');
          if (el) el.focus();
        });
        return;
      }
      InfringAPI.get('/api/models').then(function(data) {
        var models = (data.models || []).filter(function(m) { return m.available; });
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
        self.modelSwitcherFilter = '';
        self.modelSwitcherProviderFilter = '';
        self.modelSwitcherIdx = 0;
        self.showModelSwitcher = true;
        self.$nextTick(function() {
          var el = document.getElementById('model-switcher-search');
          if (el) el.focus();
        });
      }).catch(function(e) {
        InfringToast.error('Failed to load models: ' + e.message);
      });
    },

    discoverModelsFromApiKey: function() {
      var self = this;
      var entry = String(this.modelApiKeyInput || '').trim();
      if (!entry) {
        InfringToast.error('Enter an API key or local model path first');
        return;
      }
      this.modelApiKeySaving = true;
      this.modelApiKeyStatus = 'Detecting...';
      InfringAPI.post('/api/models/discover', {
        input: entry,
        api_key: entry
      }).then(function(resp) {
        var provider = String((resp && resp.provider) || '').trim();
        var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
        var count = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
        self.modelApiKeyInput = '';
        if (inputKind === 'local_path') {
          self.modelApiKeyStatus = provider
            ? ('Indexed local path to ' + provider + ' (' + count + ' models)')
            : ('Indexed local path (' + count + ' models)');
        } else {
          self.modelApiKeyStatus = provider ? ('Added ' + provider + ' (' + count + ' models)') : 'API key saved';
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return InfringAPI.get('/api/models');
      }).then(function(data) {
        var models = (data && data.models) || [];
        self._modelCache = models.filter(function(m) { return m.available; });
        self._modelCacheTime = Date.now();
        self.modelPickerList = self._modelCache;
      }).catch(function(e) {
        self.modelApiKeyStatus = '';
        InfringToast.error('Model discovery failed: ' + (e && e.message ? e.message : e));
      }).finally(function() {
        self.modelApiKeySaving = false;
      });
    },

    resolveModelContextWindowForSwitch: function(targetModelRef) {
      var modelId = '';
      var explicitWindow = 0;
      if (targetModelRef && typeof targetModelRef === 'object') {
        modelId = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
        explicitWindow = Number(
          targetModelRef.context_window || targetModelRef.context_window_tokens || 0
        );
      } else {
        modelId = String(targetModelRef || '').trim();
      }
      if (Number.isFinite(explicitWindow) && explicitWindow > 0) {
        return Math.round(explicitWindow);
      }
      var map = this._contextWindowByModel || {};
      var candidates = [];
      if (modelId) {
