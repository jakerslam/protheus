      } else {
        x = targetX;
        y = targetY;
      }
      s.x = x;
      s.y = y;
      s.vx = 0;
      s.vy = 0;
      s.trailX = x;
      s.trailY = y;
      s.anchorMode = 'thinking';
      s.anchorTargetX = targetX;
      s.anchorTargetY = targetY;
      s.anchorLastAt = now;
      this._agentTrailState = s;
      this._agentTrailSeeded = true;
      this._agentTrailLastDotAt = now;
      var orb = this.ensureAgentTrailOrb(host, x, y);
      if (orb && orb.classList) orb.classList.add('agent-listening');
      host.style.setProperty('--chat-agent-grid-active', '1');
      host.style.setProperty('--chat-agent-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-agent-grid-y', Math.round(y) + 'px');
      this._agentTrailLastAt = now;
      return true;
    },
    anchorAgentTrailToFreshInit(host, hostRect, now, pad, w, h) {
      if (!host || typeof host.querySelector !== 'function') return false;
      if (!this.showFreshArchetypeTiles || !this.freshInitRevealMenu) return false;
      // Never override active thinking positioning during init.
      var activeThinking = host.querySelector('.message.thinking .message-bubble.message-bubble-thinking');
      if (activeThinking && activeThinking.offsetParent !== null) return false;
      var panel = host.querySelector('.chat-init-panel');
      if (!panel || panel.offsetParent === null) return false;
      var rect = hostRect && Number.isFinite(Number(hostRect.width || 0)) ? hostRect : host.getBoundingClientRect();
      var panelRect = panel.getBoundingClientRect();
      if (!(Number(panelRect.width || 0) > 0 && Number(panelRect.height || 0) > 0)) return false;
      if (panelRect.bottom < rect.top || panelRect.top > rect.bottom || panelRect.right < rect.left || panelRect.left > rect.right) return false;
      // During agent initialization, pin the orb to the initial agent chat panel.
      // Keep it 1rem outside the panel's bottom-left corner.
      var anchor = {
        x: (panelRect.left - rect.left) - 16,
        y: (panelRect.bottom - rect.top) + 16,
      };
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
      var cached = self.sanitizeModelCatalogRows(self._modelCache || []);
      if (cached.length) {
        self._modelCache = cached;
        self.modelPickerList = cached;
      }
      this.modelSwitcherFilter = '';
      this.modelSwitcherProviderFilter = '';
      this.modelSwitcherIdx = 0;
      this.showModelSwitcher = true;
      this.$nextTick(function() {
        var el = document.getElementById('model-switcher-search');
        if (el) el.focus();
      });

      var cacheFresh = Array.isArray(this._modelCache) && (now - this._modelCacheTime) < 300000;
      if (cacheFresh) return;
      InfringAPI.post('/api/models/discover', { input: '__auto__' })
        .catch(function() { return null; })
        .then(function() { return InfringAPI.get('/api/models'); })
        .then(function(data) {
        var models = self.sanitizeModelCatalogRows((data && data.models) || []);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
      }).catch(function(e) {
        if (!self.modelPickerList || !self.modelPickerList.length) {
          var active = self.resolveActiveSwitcherModel([]);
          self.modelPickerList = active ? [active] : [];
        }
        self.modelApiKeyStatus = 'Unable to refresh model list (showing cached entries)';
        InfringToast.error('Failed to refresh models: ' + e.message);
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
        var models = self.sanitizeModelCatalogRows((data && data.models) || []);
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
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
