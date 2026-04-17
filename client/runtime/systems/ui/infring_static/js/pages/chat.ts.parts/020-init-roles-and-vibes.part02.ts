
    resetInputHistoryNavigation: function(explicitMode) {
      var mode = this.inputHistoryMode(explicitMode);
      if (mode === 'terminal') {
        this.terminalInputHistoryCursor = -1;
        this.terminalInputHistoryDraft = '';
        return;
      }
      this.chatInputHistoryCursor = -1;
      this.chatInputHistoryDraft = '';
    },

    pushInputHistoryEntry: function(explicitMode, rawText) {
      var text = this.normalizeInputHistoryEntry(rawText);
      if (!text) return;
      var mode = this.inputHistoryMode(explicitMode);
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows)) return;
      if (rows.length && String(rows[rows.length - 1] || '') === text) {
        this.resetInputHistoryNavigation(mode);
        return;
      }
      var nextRows = this.normalizeInputHistoryRows(rows.concat([text]));
      rows.splice(0, rows.length);
      for (var i = 0; i < nextRows.length; i += 1) rows.push(nextRows[i]);
      this.syncInputHistoryToCache(mode);
      this.resetInputHistoryNavigation(mode);
    },

    navigateInputHistory: function(direction, event) {
      var step = Number(direction || 0);
      if (!Number.isFinite(step) || step === 0) return false;
      var mode = this.inputHistoryMode();
      var rows = this.inputHistoryEntries(mode);
      if (!Array.isArray(rows) || !rows.length) return false;
      var cursor = mode === 'terminal' ? Number(this.terminalInputHistoryCursor || -1) : Number(this.chatInputHistoryCursor || -1);
      if (!Number.isFinite(cursor)) cursor = -1;
      var draft = mode === 'terminal'
        ? String(this.terminalInputHistoryDraft || '')
        : String(this.chatInputHistoryDraft || '');

      var nextText = '';
      if (step < 0) {
        if (cursor < 0) {
          draft = String(this.inputText || '');
          cursor = rows.length - 1;
        } else {
          cursor = Math.max(0, cursor - 1);
        }
        nextText = String(rows[cursor] || '');
      } else {
        if (cursor < 0) {
          return false;
        } else if (cursor >= rows.length - 1) {
          cursor = -1;
          nextText = draft;
        } else {
          cursor += 1;
          nextText = String(rows[cursor] || '');
        }
      }

      if (mode === 'terminal') {
        this.terminalInputHistoryCursor = cursor;
        this.terminalInputHistoryDraft = draft;
      } else {
        this.chatInputHistoryCursor = cursor;
        this.chatInputHistoryDraft = draft;
      }

      this._inputHistoryApplying = true;
      this.inputText = nextText;
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) {
          var pos = String(self.inputText || '').length;
          if (typeof el.setSelectionRange === 'function') {
            try { el.setSelectionRange(pos, pos); } catch(_) {}
          }
          el.style.height = 'auto';
          el.style.height = Math.min(el.scrollHeight, 150) + 'px';
        }
        if (self.terminalMode) self.updateTerminalCursor({ target: el });
        self._inputHistoryApplying = false;
      });
      if (event && typeof event.preventDefault === 'function') event.preventDefault();
      return true;
    },

    freshInitOtherInputPlaceholder: function() {
      var label = String(
        this.freshInitName ||
        (this.currentAgent && (this.currentAgent.name || this.currentAgent.id)) ||
        'agent'
      ).trim() || 'agent';
      return 'Tell ' + label + ' who they are...';
    },

    toggleFreshInitAdvanced: function() {
      this.freshInitAdvancedOpen = !this.freshInitAdvancedOpen;
    },

    defaultFreshInitPermissionChecked: function(permissionDef) {
      return !!(permissionDef && permissionDef.default_checked);
    },

    isFreshInitPermissionChecked: function(permissionDef) {
      var key = String(permissionDef && permissionDef.key ? permissionDef.key : '').trim();
      if (!key) return false;
      var overrides = this.freshInitPermissionOverrides && typeof this.freshInitPermissionOverrides === 'object'
        ? this.freshInitPermissionOverrides
        : {};
      if (Object.prototype.hasOwnProperty.call(overrides, key)) return !!overrides[key];
      return this.defaultFreshInitPermissionChecked(permissionDef);
    },

    setFreshInitPermissionChecked: function(permissionDef, checked) {
      var key = String(permissionDef && permissionDef.key ? permissionDef.key : '').trim();
      if (!key) return;
      if (!this.freshInitPermissionOverrides || typeof this.freshInitPermissionOverrides !== 'object') {
        this.freshInitPermissionOverrides = {};
      }
      this.freshInitPermissionOverrides[key] = !!checked;
    },

    setFreshInitPermissionCategory: function(categoryId, checked) {
      var category = String(categoryId || '').trim().toLowerCase();
      var rows = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] || {};
        if (String(row.category || '').trim().toLowerCase() !== category) continue;
        var perms = Array.isArray(row.permissions) ? row.permissions : [];
        for (var j = 0; j < perms.length; j += 1) this.setFreshInitPermissionChecked(perms[j], checked);
      }
    },

    resetFreshInitPermissions: function() {
      this.freshInitPermissionOverrides = {};
    },

    resolveFreshInitPermissionManifest: function() {
      var rows = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      var grants = {};
      var categories = { agent: 'inherit', web: 'inherit', file: 'inherit', github: 'inherit', terminal: 'inherit', memory: 'inherit' };
      for (var i = 0; i < rows.length; i += 1) {
        var row = rows[i] || {};
        var perms = Array.isArray(row.permissions) ? row.permissions : [];
        for (var j = 0; j < perms.length; j += 1) {
          var permission = perms[j] || {};
          var key = String(permission.key || '').trim();
          if (!key) continue;
          grants[key] = this.isFreshInitPermissionChecked(permission) ? 'allow' : 'inherit';
        }
      }
      grants['web.search.basic'] = 'allow';
      return {
        version: 1,
        trit: { deny: -1, inherit: 0, allow: 1 },
        category_defaults: categories,
        grants: grants
      };
    },

    freshInitRoleKey: function(templateDef) {
      var template = templateDef || this.freshInitTemplateDef || {};
      var raw = String(template.archetype || template.name || '').trim().toLowerCase();
      if (!raw) return 'general';
      if (raw.indexOf('coder') >= 0 || raw.indexOf('devops') >= 0 || raw.indexOf('builder') >= 0 || raw.indexOf('api') >= 0) return 'coding';
      if (raw.indexOf('research') >= 0 || raw.indexOf('analyst') >= 0 || raw.indexOf('tutor') >= 0 || raw.indexOf('teacher') >= 0) return 'reasoning';
      if (raw.indexOf('writer') >= 0 || raw.indexOf('creative') >= 0) return 'creative';
      if (raw.indexOf('support') >= 0 || raw.indexOf('assistant') >= 0) return 'support';
      if (raw.indexOf('custom') >= 0 || raw.indexOf('other') >= 0) return 'general';
      return 'general';
    },

    freshInitModelName: function(model) {
      var row = model || {};
      var display = String(row.display_name || '').trim();
      var id = String(row.id || '').trim();
      if (display) return display;
      if (!id) return 'model';
      if (id.indexOf('/') >= 0) return id.split('/').slice(-1)[0];
      return id;
    },

    normalizeFreshInitModelRef: function(model) {
      var row = model || {};
      var id = String(row.id || '').trim();
      var provider = String(row.provider || '').trim().toLowerCase();
      if (id && id.toLowerCase() === 'auto') return '';
      if (id && id.indexOf('/') >= 0) return id;
      var name = this.freshInitModelName(row);
      if (provider && name) return provider + '/' + name;
      return id || name;
    },

    isFreshInitModelSuggestionSelected: function(model) {
      return this.normalizeFreshInitModelRef(model) === String(this.freshInitModelSelection || '').trim();
    },

    selectFreshInitModelSuggestion: function(model) {
      var ref = this.normalizeFreshInitModelRef(model);
      if (!ref) return;
      this.freshInitModelSelection = ref;
      this.freshInitModelManual = true;
      this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitModelSuggestion: function() {
      var selected = String(this.freshInitModelSelection || '').trim();
      var rows = Array.isArray(this.freshInitModelSuggestions) ? this.freshInitModelSuggestions : [];
