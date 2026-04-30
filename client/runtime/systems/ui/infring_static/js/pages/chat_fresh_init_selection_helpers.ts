// Chat fresh-init template, identity, personality, lifespan, and contract helpers.
'use strict';

function infringChatFreshInitSelectionMethods() {
  return {
    isFreshInitTemplateSelected(templateDef) {
      if (!templateDef) return false;
      var key = String(templateDef.name || '').trim();
      return !!key && key === String(this.freshInitTemplateName || '').trim();
    },

    freshInitTemplateDescription: function(templateDef) {
      if (!templateDef) return '';
      if (templateDef.is_other) {
        var typed = String(this.freshInitOtherPrompt || '').trim();
        if (typed) return this.truncateFreshInitSummary(typed, 86);
      }
      return String(templateDef.description || '').trim();
    },

    truncateFreshInitSummary: function(text, limit) {
      var clean = String(text || '').replace(/\s+/g, ' ').trim();
      if (!clean) return '';
      var max = Number(limit || 0);
      if (!Number.isFinite(max) || max < 12) max = 80;
      if (clean.length <= max) return clean;
      return clean.slice(0, Math.max(8, max - 1)).trimEnd() + '…';
    },

    filteredFreshInitEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.freshInitEmojiSearch || '').trim().toLowerCase();
      var self = this;
      var rows = source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        if (self.isReservedSystemEmoji && self.isReservedSystemEmoji(emoji)) return false;
        return true;
      });
      if (!query) return rows.slice(0, 24);
      return rows.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    toggleFreshInitEmojiPicker: function() {
      this.freshInitEmojiPickerOpen = !this.freshInitEmojiPickerOpen;
      if (!this.freshInitEmojiPickerOpen) {
        this.freshInitEmojiSearch = '';
      }
    },

    selectFreshInitEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      var sanitized = this.sanitizeAgentEmojiForDisplay
        ? this.sanitizeAgentEmojiForDisplay(this.currentAgent, emoji)
        : emoji;
      if (!sanitized) {
        InfringToast.info('The gear icon is reserved for the System thread.');
        return;
      }
      this.freshInitEmoji = sanitized;
      this.freshInitAvatarUrl = '';
      this.freshInitEmojiPickerOpen = false;
      this.freshInitEmojiSearch = '';
    },

    openFreshInitAvatarPicker: function() {
      if (this.$refs && this.$refs.freshInitAvatarInput) {
        this.$refs.freshInitAvatarInput.click();
      }
    },

    uploadFreshInitAvatar: async function(fileList) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.freshInitAvatarUploading = true;
      this.freshInitAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/avatar', {
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
          throw new Error(String(payload && payload.error ? payload.error : 'avatar_upload_failed'));
        }
        this.freshInitAvatarUrl = String(payload.avatar_url || '').trim();
        this.freshInitEmojiPickerOpen = false;
        this.freshInitEmojiSearch = '';
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.freshInitAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.freshInitAvatarUploading = false;
      }
    },

    clearFreshInitAvatar: function() {
      this.freshInitAvatarUrl = '';
      this.freshInitAvatarUploadError = '';
    },

    isFreshInitPersonalitySelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitPersonalityId || '');
    },

    selectFreshInitPersonality: function(card) {
      var id = String(card && card.id ? card.id : 'none').trim() || 'none';
      this.freshInitPersonalityId = id;
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitPersonality: function() {
      var cards = Array.isArray(this.freshInitPersonalityCards) ? this.freshInitPersonalityCards : [];
      var selectedId = String(this.freshInitPersonalityId || 'none');
      for (var i = 0; i < cards.length; i += 1) {
        if (String(cards[i] && cards[i].id ? cards[i].id : '') === selectedId) return cards[i];
      }
      return cards.length ? cards[0] : null;
    },

    isFreshInitLifespanSelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitLifespanId || '');
    },

    selectFreshInitLifespan: function(card) {
      var id = String(card && card.id ? card.id : '1h').trim() || '1h';
      this.freshInitLifespanId = id;
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    selectedFreshInitLifespan: function() {
      var cards = Array.isArray(this.freshInitLifespanCards) ? this.freshInitLifespanCards : [];
      var selectedId = String(this.freshInitLifespanId || '1h');
      var fallback = null;
      for (var i = 0; i < cards.length; i += 1) {
        var cardId = String(cards[i] && cards[i].id ? cards[i].id : '');
        if (cardId === '1h') fallback = cards[i];
        if (cardId === selectedId) return cards[i];
      }
      return fallback || (cards.length ? cards[0] : null);
    },

    async applyChatArchetypeTemplate(templateDef) {
      if (!templateDef) return;
      this.freshInitTemplateDef = templateDef;
      this.freshInitTemplateName = String(templateDef.name || '').trim();
      this.freshInitModelManual = false;
      this.freshInitModelSelection = '';
      this.refreshFreshInitModelSuggestions(templateDef);
      if (templateDef.is_other) {
        this.freshInitAwaitingOtherPrompt = true;
        this.focusChatComposerFromInit(String(this.freshInitOtherPrompt || '').trim());
      } else {
        this.freshInitAwaitingOtherPrompt = false;
      }
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor();
    },

    captureFreshInitOtherPrompt: function() {
      if (!this.showFreshArchetypeTiles || !this.freshInitAwaitingOtherPrompt) return false;
      if (Array.isArray(this.attachments) && this.attachments.length > 0) {
        InfringToast.info('Init prompt does not support file attachments.');
        return false;
      }
      var text = String(this.inputText || '').trim();
      if (!text) {
        InfringToast.info('Describe the special purpose first.');
        this.focusChatComposerFromInit('');
        return false;
      }
      this.freshInitOtherPrompt = text;
      this.freshInitAwaitingOtherPrompt = false;
      this.inputText = '';
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';
      if (typeof this.scheduleFreshInitProgressAnchor === 'function') this.scheduleFreshInitProgressAnchor('lifespan');
      return true;
    },

    resolveFreshInitSystemPrompt: function(templateDef, agentName, personalityCard, vibeCard) {
      if (!templateDef) return '';
      var basePrompt = '';
      if (templateDef.is_other) {
        var purpose = String(this.freshInitOtherPrompt || '').trim();
        basePrompt = [
          'You are ' + String(agentName || 'the assistant') + '.',
          'Special purpose: ' + purpose,
          'Act as a focused specialist for this purpose. Stay concise, practical, and reliable.',
        ].join('\n');
      } else {
        basePrompt = String(templateDef.system_prompt || '').trim();
      }
      var personalitySuffix = String(personalityCard && personalityCard.system_suffix ? personalityCard.system_suffix : '').trim();
      var vibeSuffix = String(vibeCard && vibeCard.system_suffix ? vibeCard.system_suffix : '').trim();
      var suffixes = [];
      if (personalitySuffix) suffixes.push(personalitySuffix);
      if (vibeSuffix) suffixes.push(vibeSuffix);
      if (suffixes.length) {
        return (basePrompt ? (basePrompt + '\n\n') : '') + suffixes.join('\n');
      }
      return basePrompt;
    },

    resolveFreshInitRole: function(templateDef) {
      var currentRole = String((this.currentAgent && this.currentAgent.role) || '').trim().toLowerCase();
      if (!templateDef) return currentRole || 'analyst';
      var hint = String(
        templateDef.role || templateDef.archetype || templateDef.profile || templateDef.name || ''
      ).trim().toLowerCase();
      if (!hint) return currentRole || 'analyst';
      if (hint.indexOf('teacher') >= 0 || hint.indexOf('tutor') >= 0 || hint.indexOf('mentor') >= 0 || hint.indexOf('coach') >= 0 || hint.indexOf('instructor') >= 0) {
        return 'tutor';
      }
      if (hint.indexOf('code') >= 0 || hint.indexOf('coder') >= 0 || hint.indexOf('engineer') >= 0 || hint.indexOf('developer') >= 0 || hint.indexOf('devops') >= 0 || hint.indexOf('api') >= 0 || hint.indexOf('build') >= 0) {
        return 'engineer';
      }
      if (hint.indexOf('research') >= 0 || hint.indexOf('investig') >= 0) {
        return 'researcher';
      }
      if (hint.indexOf('analyst') >= 0 || hint.indexOf('analysis') >= 0 || hint.indexOf('data') >= 0 || hint.indexOf('meeting') >= 0) {
        return 'analyst';
      }
      if (hint.indexOf('writer') >= 0 || hint.indexOf('editor') >= 0 || hint.indexOf('content') >= 0) {
        return 'writer';
      }
      if (hint.indexOf('design') >= 0 || hint.indexOf('ui') >= 0 || hint.indexOf('ux') >= 0) {
        return 'designer';
      }
      if (hint.indexOf('support') >= 0) {
        return 'support';
      }
      return currentRole || 'analyst';
    },

    resolveFreshInitContractPayload: function(agentName) {
      var selected = this.selectedFreshInitLifespan();
      var mission = 'Initialize and run as ' + String(agentName || 'agent') + '.';
      if (!selected) {
        return {
          mission: mission,
          termination_condition: 'task_or_timeout',
          expiry_seconds: 60 * 60,
          indefinite: false,
          auto_terminate_allowed: true,
          idle_terminate_allowed: true,
        };
      }
      var terminationCondition = String(selected.termination_condition || 'task_or_timeout');
      var expirySeconds = selected.expiry_seconds == null ? null : Number(selected.expiry_seconds);
      var indefinite = selected.indefinite === true;
      var supportsTimeout = terminationCondition === 'timeout' || terminationCondition === 'task_or_timeout';
      return {
        mission: mission,
        termination_condition: terminationCondition,
        expiry_seconds: expirySeconds,
        indefinite: indefinite,
        auto_terminate_allowed: !indefinite && supportsTimeout && expirySeconds != null,
        idle_terminate_allowed: !indefinite && supportsTimeout && expirySeconds != null,
      };
    },

  };
}

function chatFreshInitCanLaunch(vm) {
  var selectedTemplate = vm.freshInitTemplateDef;
  var hasTemplate = !!selectedTemplate;
  if (selectedTemplate && selectedTemplate.is_other) {
    hasTemplate = !!String(vm.freshInitOtherPrompt || '').trim() && !vm.freshInitAwaitingOtherPrompt;
  }
  return !!(
    vm.showFreshArchetypeTiles &&
    !vm.freshInitLaunching &&
    !vm.freshInitAvatarUploading &&
    hasTemplate
  );
}
