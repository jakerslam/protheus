            await store.refreshAgents({ force: true });
          }
        }
        var resolved = this.resolveAgent(agentId);
        if (resolved) {
          this.currentAgent = Object.assign({}, resolved, {
            archived: false,
            state: String(resolved.state || 'running')
          });
        } else if (this.currentAgent && String((this.currentAgent && this.currentAgent.id) || '') === agentId) {
          this.currentAgent = Object.assign({}, this.currentAgent, { archived: false, state: 'running' });
        }
        this.showAgentDrawer = false;
        this.showFreshArchetypeTiles = false;
        await this.loadSessions(agentId);
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        InfringToast.success('Revived ' + (resolved && (resolved.name || resolved.id) ? (resolved.name || resolved.id) : agentId));
      } catch (e) {
        InfringToast.error('Failed to revive archived agent: ' + (e && e.message ? e.message : 'unknown_error'));
      }
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

    normalizeDrawerPermissionValue(raw) {
      if (typeof raw === 'number' && Number.isFinite(raw)) return raw < 0 ? -1 : (raw > 0 ? 1 : 0);
      if (typeof raw === 'boolean') return raw ? 1 : -1;
      var lowered = String(raw == null ? '' : raw).trim().toLowerCase();
      if (!lowered) return 0;
      if (lowered === 'allow' || lowered === 'true' || lowered === '1' || lowered === '+1') return 1;
      if (lowered === 'deny' || lowered === 'false' || lowered === '-1') return -1;
      if (lowered === 'inherit' || lowered === '0') return 0;
      return 0;
    },

    resolveDrawerPermissionsManifest() {
      var row = this.agentDrawer && typeof this.agentDrawer === 'object' ? this.agentDrawer : {};
      var contract = row.contract && typeof row.contract === 'object' ? row.contract : {};
      var source = (contract.permissions_manifest && typeof contract.permissions_manifest === 'object')
        ? contract.permissions_manifest
        : ((row.permissions_manifest && typeof row.permissions_manifest === 'object') ? row.permissions_manifest : {});
      var catalog = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      var defaultsSource = (source.category_defaults && typeof source.category_defaults === 'object')
        ? source.category_defaults
        : ((source.categories && typeof source.categories === 'object') ? source.categories : {});
      var grantsSource = (source.grants && typeof source.grants === 'object') ? source.grants : {};
      var out = {
        version: 1,
        trit: { deny: -1, inherit: 0, allow: 1 },
        category_defaults: {},
        grants: {}
      };
      for (var ci = 0; ci < catalog.length; ci += 1) {
        var category = String((catalog[ci] && catalog[ci].category) || '').trim().toLowerCase();
        if (!category) continue;
        out.category_defaults[category] = this.normalizeDrawerPermissionValue(defaultsSource[category]);
      }
      Object.keys(grantsSource || {}).forEach(function(key) {
        var permissionKey = String(key || '').trim();
        if (!permissionKey) return;
        out.grants[permissionKey] = this.normalizeDrawerPermissionValue(grantsSource[key]);
      }, this);
      Object.keys(source || {}).forEach(function(key) {
        var permissionKey = String(key || '').trim();
        if (permissionKey.indexOf('.') <= 0 || Object.prototype.hasOwnProperty.call(out.grants, permissionKey)) return;
        out.grants[permissionKey] = this.normalizeDrawerPermissionValue(source[key]);
      }, this);
      for (var i = 0; i < catalog.length; i += 1) {
        var section = catalog[i] || {};
        var permissions = Array.isArray(section.permissions) ? section.permissions : [];
        for (var j = 0; j < permissions.length; j += 1) {
          var key = String((permissions[j] && permissions[j].key) || '').trim();
          if (!key || Object.prototype.hasOwnProperty.call(out.grants, key)) continue;
          out.grants[key] = 0;
        }
      }
      out.grants['web.search.basic'] = out.grants['web.search.basic'] < 0 ? 0 : 1;
      return out;
    },

    ensureDrawerPermissionsManifest() {
      var manifest = this.resolveDrawerPermissionsManifest();
      if (!this.agentDrawer || typeof this.agentDrawer !== 'object') this.agentDrawer = {};
      if (!this.agentDrawer.contract || typeof this.agentDrawer.contract !== 'object') this.agentDrawer.contract = {};
      this.agentDrawer.permissions_manifest = manifest;
      this.agentDrawer.contract.permissions_manifest = manifest;
      return manifest;
    },

    drawerPermissionLabelForKey(permissionKey) {
      var key = String(permissionKey || '').trim();
      if (!key) return '';
      var rows = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      for (var i = 0; i < rows.length; i += 1) {
        var perms = Array.isArray(rows[i] && rows[i].permissions) ? rows[i].permissions : [];
        for (var j = 0; j < perms.length; j += 1) {
          if (String((perms[j] && perms[j].key) || '').trim() === key) {
            return String((perms[j] && perms[j].label) || key).trim() || key;
          }
        }
      }
      return key;
    },

    drawerPermissionRows() {
      var manifest = this.resolveDrawerPermissionsManifest();
      var grants = (manifest && manifest.grants && typeof manifest.grants === 'object') ? manifest.grants : {};
      var out = [];
      var byCategory = {};
      var catalog = Array.isArray(this.freshInitPermissionCatalog) ? this.freshInitPermissionCatalog : [];
      for (var i = 0; i < catalog.length; i += 1) {
        var categoryId = String((catalog[i] && catalog[i].category) || '').trim().toLowerCase();
        if (!categoryId || byCategory[categoryId]) continue;
        var name = String((catalog[i] && catalog[i].name) || categoryId).trim() || categoryId;
        byCategory[categoryId] = { category: categoryId, name: name, permissions: [] };
        out.push(byCategory[categoryId]);
      }
      var keys = Object.keys(grants || {}).sort(function(left, right) {
        return String(left || '').localeCompare(String(right || ''));
      });
      for (var k = 0; k < keys.length; k += 1) {
        var key = String(keys[k] || '').trim();
        if (!key) continue;
        var category = key.split('.')[0] || 'other';
        if (!byCategory[category]) {
          byCategory[category] = {
            category: category,
            name: category.charAt(0).toUpperCase() + category.slice(1),
            permissions: []
          };
          out.push(byCategory[category]);
        }
        byCategory[category].permissions.push({ key: key, label: this.drawerPermissionLabelForKey(key) });
      }
      return out.filter(function(section) {
        return section && Array.isArray(section.permissions) && section.permissions.length > 0;
      });
    },

    drawerPermissionState(permissionKey) {
      var key = String(permissionKey || '').trim();
      if (!key) return 0;
      var manifest = this.resolveDrawerPermissionsManifest();
      var grants = manifest && manifest.grants && typeof manifest.grants === 'object' ? manifest.grants : {};
      return this.normalizeDrawerPermissionValue(grants[key]);
    },

    drawerPermissionStateLabel(rawValue) {
      var value = this.normalizeDrawerPermissionValue(rawValue);
      if (value > 0) return 'Allowed';
      if (value < 0) return 'No access';
      return 'Inherited';
    },

    drawerPermissionStateClass(rawValue) {
      var value = this.normalizeDrawerPermissionValue(rawValue);
      if (value > 0) return 'perm-state-allow';
      if (value < 0) return 'perm-state-deny';
      return 'perm-state-inherit';
    },

    drawerPermissionDescriptionForKey(permissionKey) {
      var key = String(permissionKey || '').trim();
      if (!key) return '';
      var tokens = key.split('.').map(function(part) { return String(part || '').trim().toLowerCase(); }).filter(Boolean);
      if (tokens.length < 2) return 'Scope key: ' + key;
      var verb = tokens[1];
      var subjectTokens = tokens.slice(2).map(function(part) {
        return part.replace(/_/g, ' ');
      });
      var subject = subjectTokens.join(' ').trim() || 'this scope';
      if (verb === 'read') return 'Read access to ' + subject + '.';
      if (verb === 'write') return 'Write access to ' + subject + '.';
      if (verb === 'delete') return 'Delete access to ' + subject + '.';
      if (verb === 'search') return 'Search access for ' + subject + '.';
      if (verb === 'fetch') return 'Fetch access for ' + subject + '.';
      if (verb === 'create') return 'Create access for ' + subject + '.';
      if (verb === 'exec') return 'Execution access for ' + subject + '.';
      if (verb === 'spawn') return 'Can spawn child agents.';
      if (verb === 'manage') return 'Can manage ' + subject + '.';
      return 'Scope key: ' + key;
    },

    drawerPermissionCategoryState(section) {
      var perms = Array.isArray(section && section.permissions) ? section.permissions : [];
      var allow = 0;
      var inherit = 0;
      var deny = 0;
      for (var i = 0; i < perms.length; i += 1) {
        var value = this.drawerPermissionState(perms[i] && perms[i].key);
        if (value > 0) allow += 1;
        else if (value < 0) deny += 1;
        else inherit += 1;
      }
      return {
        allow: allow,
        inherit: inherit,
        deny: deny,
        total: perms.length
      };
    },

    setDrawerPermissionState(permissionKey, nextValue) {
      var key = String(permissionKey || '').trim();
      if (!key) return;
      var manifest = this.ensureDrawerPermissionsManifest();
      manifest.grants[key] = this.normalizeDrawerPermissionValue(nextValue);
      this.agentDrawer.permissions_manifest = manifest;
      this.agentDrawer.contract.permissions_manifest = manifest;
    },

    setDrawerPermissionCategoryState(categoryId, nextValue) {
      var category = String(categoryId || '').trim().toLowerCase();
      if (!category) return;
      var manifest = this.ensureDrawerPermissionsManifest();
      var rows = this.drawerPermissionRows();
      for (var i = 0; i < rows.length; i += 1) {
        if (String((rows[i] && rows[i].category) || '').trim().toLowerCase() !== category) continue;
        var perms = Array.isArray(rows[i].permissions) ? rows[i].permissions : [];
        for (var j = 0; j < perms.length; j += 1) {
          var key = String((perms[j] && perms[j].key) || '').trim();
          if (!key) continue;
          manifest.grants[key] = this.normalizeDrawerPermissionValue(nextValue);
        }
      }
      this.agentDrawer.permissions_manifest = manifest;
      this.agentDrawer.contract.permissions_manifest = manifest;
    },

    drawerPermissionChecked(permissionKey) {
      return this.drawerPermissionState(permissionKey) >= 0;
    },

    setDrawerPermissionChecked(permissionKey, checked) {
      var key = String(permissionKey || '').trim();
      if (!key) return;
      var current = this.drawerPermissionState(key);
      this.setDrawerPermissionState(key, checked ? (current < 0 ? 0 : current) : -1);
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
        configPayload.permissions_manifest = this.resolveDrawerPermissionsManifest();
        if (this.drawerEditingFallback && String(this.drawerNewFallbackValue || '').trim()) {
          var fallbackParts = String(this.drawerNewFallbackValue || '').trim().split('/');
          var fallbackProvider = fallbackParts.length > 1 ? fallbackParts[0] : this.agentDrawer.model_provider;
          var fallbackModel = fallbackParts.length > 1 ? fallbackParts.slice(1).join('/') : fallbackParts[0];
