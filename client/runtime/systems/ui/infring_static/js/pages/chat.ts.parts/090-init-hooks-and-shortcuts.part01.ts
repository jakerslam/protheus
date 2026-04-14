        this.conversationCache = Object.assign({}, persistedCache, runtimeCache);
        window.__infringChatCache = this.conversationCache;
      }
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

      this.$watch('messages.length', function() {
        self.$nextTick(function() {
          self.scrollToBottom({ force: false });
        });
      });

      // Check for pending agent from Agents page (set before chat mounted)
      var store = Alpine.store('app');
      if (store.pendingAgent) {
        self.selectAgent(store.pendingAgent);
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
        if (self.currentAgent && self.isSystemThreadAgent && self.isSystemThreadAgent(self.currentAgent)) {
          self._agentMissingAgentId = '';
          self._agentMissingSince = 0;
          self.currentAgent = self.makeSystemThreadAgent();
          if (!store || !store.activeAgentId || !self.isSystemThreadId || !self.isSystemThreadId(store.activeAgentId)) {
            self.setStoreActiveAgentId(self.systemThreadId || 'system');
          }
          return;
        }
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
        if (!self._inputHistoryApplying) {
          self.resetInputHistoryNavigation(self.terminalMode ? 'terminal' : 'chat');
        }
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
          }, 1000);
        } else if (self._agentTrailListening) {
          // Keep the "listening" pulse alive briefly after typing stops so
          // the transition feels intentional instead of abrupt.
          self._agentTrailListenTimer = setTimeout(function() {
            self._agentTrailListenTimer = 0;
            self._agentTrailListening = false;
            if (self._agentTrailOrbEl && self._agentTrailOrbEl.classList) self._agentTrailOrbEl.classList.remove('agent-listening');
            if (!self._agentTrailRaf) self.startAgentTrailLoop();
          }, 1000);
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
            InfringAPI.post('/api/models/discover', { input: '__auto__' })
              .catch(function() { return null; })
              .then(function() { return InfringAPI.get('/api/models'); })
              .then(function(data) {
              self.modelPickerList = self.sanitizeModelCatalogRows((data && data.models) || []);
              if (self.availableModelRowsCount(self.modelPickerList) === 0) {
                self.injectNoModelsGuidance('slash_model');
              }
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
        self.installChatResizeBlurObserver();
      });

      InfringAPI.get('/api/status').then(function(status) {
        var suggested = status && (status.workspace_dir || status.root_dir || status.home_dir)
          ? String(status.workspace_dir || status.root_dir || status.home_dir)
          : '';
        if (suggested) self.terminalCwd = suggested;
      }).catch(function() {});

      this.refreshModelCatalogAndGuidance({ discover: true, guidance: true }).catch(function() {});

      if (this._contextTelemetryTimer) clearInterval(this._contextTelemetryTimer);
      this._contextTelemetryTimer = setInterval(function() {
        self.requestContextTelemetry(false);
      }, 8000);
      if (this._telemetryAlertsTimer) clearInterval(this._telemetryAlertsTimer);
      this._telemetryAlertsTimer = setInterval(function() {
        self.fetchProactiveTelemetryAlerts(true);
      }, 15000);
    },

    toggleTerminalMode() {
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) {
        this.terminalMode = true;
        this.showSlashMenu = false;
        this.showModelPicker = false;
        this.showModelSwitcher = false;
        this.terminalCursorFocused = false;
        this.$nextTick(function() {
          var input = document.getElementById('msg-input');
          if (input) input.focus();
        });
        return;
      }
      this.terminalMode = !this.terminalMode;
      this.resetInputHistoryNavigation('chat');
      this.resetInputHistoryNavigation('terminal');
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
      var self = this;
      var maps = document.querySelectorAll('.chat-map-scroll');
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
      var scrollers = document.querySelectorAll('.messages#messages');
      for (var si = 0; si < scrollers.length; si++) {
        var scroller = scrollers[si];
        if (!scroller || scroller.__ofBottomWheelLock) continue;
        scroller.__ofBottomWheelLock = true;
        scroller.addEventListener('wheel', function(ev) {
          self._lastMessagesWheelAt = Date.now();
          if (Number(ev.deltaY || 0) <= 0) return;
          self._stickToBottom = true;
        }, { passive: true });
      }
    },
    anchorAgentTrailToThinking(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelectorAll !== 'function') return false;
      var self = this;
      var pinToLastThinkingAnchor = function() {
        var s = self._agentTrailState || null;
        if (!self.freshInitLaunching || !s || String(s.anchorMode || '') !== 'thinking') return false;
        var x = Number(s.anchorTargetX);
        var y = Number(s.anchorTargetY);
        if (!Number.isFinite(x) || !Number.isFinite(y)) {
          x = Number(s.x);
          y = Number(s.y);
        }
        if (!Number.isFinite(x) || !Number.isFinite(y)) return false;
        x = Math.max(pad + 1, Math.min(w - (pad + 1), x));
        y = Math.max(pad + 1, Math.min(h - (pad + 1), y));
        s.x = x; s.y = y; s.vx = 0; s.vy = 0; s.trailX = x; s.trailY = y; s.anchorLastAt = now;
        self._agentTrailState = s;
        self.ensureAgentTrailOrb(host, x, y);
        if (self._agentTrailOrbEl && self._agentTrailOrbEl.classList) self._agentTrailOrbEl.classList.add('agent-listening');
        host.style.setProperty('--chat-agent-grid-active', '1');
        host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
        host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
        return true;
      };
      var bubbles = host.querySelectorAll('.message.thinking .message-bubble.message-bubble-thinking');
      if (!bubbles || !bubbles.length) {
        if (pinToLastThinkingAnchor()) return true;
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
        // Pin the autonomous agent orb outside the bottom-left edge of
        // the active thinking dialog while the agent is working.
        // Keep a 1.5rem diagonal offset so the orb stays closer while thinking.
        var remPx = 16;
        try {
          var root = document && document.documentElement
            ? window.getComputedStyle(document.documentElement)
            : null;
          var rootFont = root ? parseFloat(String(root.fontSize || '16')) : 16;
          if (Number.isFinite(rootFont) && rootFont > 0) remPx = rootFont;
        } catch (_) {}
        var orbOffset = remPx * 1.5;
        anchor = { x: (bubbleRect.left - rect.left) - orbOffset, y: (bubbleRect.bottom - rect.top) + orbOffset };
        break;
      }
      if (!anchor) {
        if (pinToLastThinkingAnchor()) return true;
        if (this._agentTrailOrbEl && this._agentTrailOrbEl.classList && !this._agentTrailListening) this._agentTrailOrbEl.classList.remove('agent-listening');
        return false;
      }
      var targetX = Math.max(pad + 1, Math.min(w - (pad + 1), Number(anchor.x || 0)));
      var targetY = Math.max(pad + 1, Math.min(h - (pad + 1), Number(anchor.y || 0)));
      var s = this._agentTrailState;
      var x = NaN;
      var y = NaN;
      if (s && Number.isFinite(Number(s.x)) && Number.isFinite(Number(s.y))) {
        x = Number(s.x);
        y = Number(s.y);
      } else if (this._agentTrailOrbEl && this._agentTrailOrbEl.isConnected && this._agentTrailOrbEl.parentNode === host) {
        x = Number(parseFloat(String(this._agentTrailOrbEl.style.left || 'NaN')));
        y = Number(parseFloat(String(this._agentTrailOrbEl.style.top || 'NaN')));
        if (!Number.isFinite(x)) x = Number(this._agentTrailOrbEl.offsetLeft || NaN);
        if (!Number.isFinite(y)) y = Number(this._agentTrailOrbEl.offsetTop || NaN);
      }
      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        x = targetX;
        y = targetY;
      }
      if (!s) {
        s = { x: x, y: y, vx: 0, vy: 0, dir: 0, target: 0, turnAt: now + 1000 };
      }
      var lastAnchorAt = Number(s.anchorLastAt || 0);
      var dt = lastAnchorAt > 0 ? Math.min(0.08, Math.max(0.001, (now - lastAnchorAt) / 1000)) : (1 / 60);
      var dx = targetX - x;
      var dy = targetY - y;
      var dist = Math.sqrt((dx * dx) + (dy * dy));
      if (dist > 0.001) {
        // Move in a straight line into the thinking anchor, never teleport.
        var maxStep = 1480 * dt;
        if (dist <= maxStep) {
          x = targetX;
          y = targetY;
        } else {
          x += (dx / dist) * maxStep;
          y += (dy / dist) * maxStep;
        }
