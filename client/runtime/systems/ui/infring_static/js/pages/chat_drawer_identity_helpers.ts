// Chat agent drawer identity, emoji, and avatar helpers.
'use strict';

function infringChatDrawerIdentityMethods() {
  return {
    filteredDrawerEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.drawerEmojiSearch || '').trim().toLowerCase();
      var self = this;
      var allowSystemReserved = this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent);
      var rows = source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        if (!allowSystemReserved && self.isReservedSystemEmoji && self.isReservedSystemEmoji(emoji)) return false;
        return true;
      });
      if (!query) return rows.slice(0, 24);
      return rows.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    defaultFreshEmojiForAgent: function(agentRef) {
      void agentRef;
      return '∞';
    },

    suggestedFreshIdentityForAgent: function(agentRef, templateDef) {
      var agent = agentRef && typeof agentRef === 'object' ? agentRef : {};
      var id = String(agent.id || agentRef || '').trim();
      var name = String(agent.name || '').trim();
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      if (!emoji) {
        emoji = this.defaultFreshEmojiForAgent(id || name || 'agent');
      }
      if (templateDef && templateDef.category) {
        var category = String(templateDef.category).toLowerCase();
        if (category.indexOf('development') >= 0) emoji = '🧑\u200d💻';
        else if (category.indexOf('research') >= 0) emoji = '🔬';
        else if (category.indexOf('operations') >= 0 || category.indexOf('ops') >= 0) emoji = '🛠️';
        else if (category.indexOf('writing') >= 0) emoji = '📝';
      }
      emoji = this.sanitizeAgentEmojiForDisplay ? this.sanitizeAgentEmojiForDisplay(agent, emoji) : emoji;
      if (!emoji) emoji = '∞';
      return {
        name: name || String(id || '').trim(),
        emoji: String(emoji || '∞').trim() || '∞',
      };
    },

    toggleDrawerEmojiPicker: function() {
      this.drawerEmojiPickerOpen = !this.drawerEmojiPickerOpen;
      if (!this.drawerEmojiPickerOpen) {
        this.drawerEmojiSearch = '';
      } else {
        this.drawerAvatarUrlPickerOpen = false;
        this.drawerEditingEmoji = true;
      }
    },

    toggleDrawerAvatarUrlPicker: function() {
      this.drawerAvatarUrlPickerOpen = !this.drawerAvatarUrlPickerOpen;
      if (this.drawerAvatarUrlPickerOpen) {
        this.drawerEmojiPickerOpen = false;
        this.drawerAvatarUploadError = '';
        this.drawerAvatarUrlDraft = String(
          (this.drawerConfigForm && this.drawerConfigForm.avatar_url) ||
          (this.agentDrawer && this.agentDrawer.avatar_url) ||
          ''
        ).trim();
      } else {
        this.drawerAvatarUrlDraft = '';
      }
    },

    applyDrawerAvatarUrl: async function() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var draft = String(this.drawerAvatarUrlDraft || '').trim();
      if (!draft) {
        this.drawerAvatarUploadError = 'avatar_url_required';
        InfringToast.error('Avatar URL is required.');
        return;
      }
      var parsed = null;
      try {
        parsed = new URL(draft);
      } catch (_) {
        parsed = null;
      }
      if (!parsed || (parsed.protocol !== 'http:' && parsed.protocol !== 'https:')) {
        this.drawerAvatarUploadError = 'avatar_url_invalid';
        InfringToast.error('Avatar URL must start with http:// or https://');
        return;
      }
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      var normalized = String(parsed.toString()).trim();
      this.drawerConfigForm.avatar_url = normalized;
      if (this.agentDrawer && typeof this.agentDrawer === 'object') {
        this.agentDrawer.avatar_url = normalized;
      }
      this.drawerAvatarUploadError = '';
      this.drawerEmojiPickerOpen = false;
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerEditingEmoji = false;
      await this.saveDrawerIdentity('avatar');
    },

    selectDrawerEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      var sanitized = this.sanitizeAgentEmojiForDisplay
        ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer || this.currentAgent, emoji)
        : emoji;
      if (!sanitized) {
        InfringToast.info('The gear icon is reserved for the System thread.');
        return;
      }
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      this.drawerConfigForm.emoji = sanitized;
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
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
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
        this.drawerAvatarUrlPickerOpen = false;
        this.drawerAvatarUrlDraft = '';
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
  };
}
