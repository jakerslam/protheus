'use strict';

function infringChatFreshInitPermissionMethods() {
  return {
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
  };
}
