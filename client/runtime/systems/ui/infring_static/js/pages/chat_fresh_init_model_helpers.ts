'use strict';

function infringChatFreshInitModelMethods() {
  return {
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

      for (var i = 0; i < rows.length; i += 1) {
        if (this.normalizeFreshInitModelRef(rows[i]) === selected) return rows[i];
      }
      return rows.length ? rows[0] : null;
    },
    isFreshInitVibeSelected: function(card) {
      if (!card) return false;
      return String(card.id || '') === String(this.freshInitVibeId || '');
    },
    selectFreshInitVibe: function(card) {
      var id = String(card && card.id ? card.id : 'none').trim() || 'none';
      this.freshInitVibeId = id;
      this.scheduleFreshInitProgressAnchor();
    },
    scheduleFreshInitProgressAnchor: function(forcedAnchor) {
      var anchor = String(forcedAnchor || '').trim();
      if (!anchor) {
        if (this.freshInitCanLaunch) anchor = 'launch';
        else if (this.freshInitTemplateDef) anchor = 'lifespan';
        else anchor = 'role';
      }
      var self = this;
      this.$nextTick(function() {
        var scroller = typeof self.resolveMessagesScroller === 'function' ? self.resolveMessagesScroller(null) : null;
        if (!scroller || typeof scroller.getBoundingClientRect !== 'function') return;
        var panel = scroller.querySelector('.chat-init-panel');
        if (!panel) return;
        var target = panel.querySelector('[data-init-anchor=\"' + anchor + '\"]');
        if (!target || typeof target.getBoundingClientRect !== 'function') return;
        var hostRect = scroller.getBoundingClientRect();
        var targetRect = target.getBoundingClientRect();
        var delta = (targetRect.bottom + 92) - hostRect.bottom;
        if (Math.abs(delta) < 2) return;
        scroller.scrollTo({ top: Math.max(0, scroller.scrollTop + delta), behavior: 'smooth' });
      });
    },
    selectedFreshInitVibe: function() {
      var cards = Array.isArray(this.freshInitVibeCards) ? this.freshInitVibeCards : [];
      var selectedId = String(this.freshInitVibeId || 'none');
      for (var i = 0; i < cards.length; i += 1) {
        if (String(cards[i] && cards[i].id ? cards[i].id : '') === selectedId) return cards[i];
      }
      return cards.length ? cards[0] : null;
    },
    modelSpecialtyTagsForScoring: function(model) {
      var tags = model && model.specialty_tags;
      if (!Array.isArray(tags)) return [];
      var seen = {};
      var out = [];
      for (var i = 0; i < tags.length; i += 1) {
        var tag = String(tags[i] || '').trim().toLowerCase();
        if (!tag || seen[tag]) continue;
        seen[tag] = true;
        out.push(tag);
      }
      return out;
    },
    scoreFreshInitModelForRole: function(model, roleKey) {
      var row = model || {};
      var role = String(roleKey || 'general').trim().toLowerCase() || 'general';
      var power = this.modelPowerLevel(row);
      var cost = this.modelCostLevel(row);
      var contextWindow = Number(row && row.context_window != null ? row.context_window : 0);
      var contextScore = 0;
      if (Number.isFinite(contextWindow) && contextWindow > 0) {
        contextScore = Math.max(0, Math.min(2.4, Math.log2(Math.max(4096, contextWindow) / 4096)));
      }
      var paramsB = this.modelParamCountB(row);
      var specialty = String(row && row.specialty ? row.specialty : '').trim().toLowerCase();
      var tags = this.modelSpecialtyTagsForScoring(row);
      var name = this.freshInitModelName(row).toLowerCase();
      var local = this.modelDeploymentKind(row) === 'local';
      var score = (power * 1.25) + ((6 - cost) * 0.7) + (contextScore * 0.45);
      if (local) score += 0.35;
      if (role === 'coding') {
        if (specialty === 'coding') score += 3.1;
        if (tags.indexOf('coding') >= 0) score += 1.6;
        if (/\b(code|coder|codex|codestral|deepseek|starcoder|qwen.*coder)\b/i.test(name)) score += 1.5;
        score += power * 0.35;
      } else if (role === 'reasoning') {
        if (specialty === 'reasoning') score += 3.0;
        if (tags.indexOf('reasoning') >= 0) score += 1.2;
        score += contextScore * 1.15;
        if (/\b(reason|o3|r1|sonnet|opus|think)\b/i.test(name)) score += 0.9;
      } else if (role === 'creative') {
        score += Math.max(0, 1.8 - Math.abs(power - 3) * 0.7);
        score += contextScore * 0.8;
        if (specialty === 'coding') score -= 0.5;
      } else if (role === 'support') {
        score += (6 - cost) * 1.05;
        if (/\b(mini|flash|instant|turbo|haiku)\b/i.test(name)) score += 1.0;
        if (Number.isFinite(paramsB) && paramsB > 60) score -= 0.8;
      } else {
        score += power * 0.35;
        score += contextScore * 0.55;
      }
      if (Number.isFinite(paramsB) && paramsB > 0) {
        if (role === 'support' && paramsB > 80) score -= 1.0;
        if (role === 'coding' && paramsB > 100) score -= 0.6;
      }
      var usageBonus = this.modelUsageTs(this.normalizeFreshInitModelRef(row)) > 0 ? 0.25 : 0;
      score += usageBonus;
      return Number(score.toFixed(6));
    },
    refreshFreshInitModelSuggestions: async function(templateDef) {
      var template = templateDef || this.freshInitTemplateDef || null;
      if (!template) {
        this.freshInitModelSuggestions = [];
        this.freshInitModelSelection = '';
        this.freshInitModelSuggestLoading = false;
        return;
      }
      this.freshInitModelSuggestLoading = true;
      try {
        var rows = await this.ensureFailoverModelCache();
        var roleKey = this.freshInitRoleKey(template);
        var ranked = (Array.isArray(rows) ? rows : [])
          .filter(function(row) {
            return !!(row && row.available !== false && String(row.id || '').trim() && String(row.id || '').trim().toLowerCase() !== 'auto');
          })
          .map((row) => ({
            ...(row && typeof row === 'object' ? row : {}),
            _fresh_role_score: this.scoreFreshInitModelForRole(row, roleKey),
          }))
          .sort((left, right) => {
            var a = Number(left && left._fresh_role_score != null ? left._fresh_role_score : 0);
            var b = Number(right && right._fresh_role_score != null ? right._fresh_role_score : 0);
            if (b !== a) return b - a;
            var lName = this.normalizeFreshInitModelRef(left).toLowerCase();
            var rName = this.normalizeFreshInitModelRef(right).toLowerCase();
            return lName.localeCompare(rName);
          })
          .slice(0, 5);
        if (!ranked.length) {
          var fallbackProvider = String(template.provider || '').trim().toLowerCase();
          var fallbackModel = String(template.model || '').trim();
          if (fallbackProvider && fallbackModel) {
            ranked = [{
              id: fallbackProvider + '/' + fallbackModel,
              display_name: fallbackModel,
              provider: fallbackProvider,
              context_window: 0,
              available: true,
              power_rating: 3,
              cost_rating: fallbackProvider === 'ollama' || fallbackProvider === 'llama.cpp' ? 1 : 3,
              specialty: 'general',
              specialty_tags: ['general'],
            }];
          }
        }
        this.freshInitModelSuggestions = ranked;
        var current = String(this.freshInitModelSelection || '').trim();
        var hasCurrent = ranked.some((row) => this.normalizeFreshInitModelRef(row) === current);
        if (!this.freshInitModelManual || !hasCurrent) {
          this.freshInitModelSelection = ranked.length ? this.normalizeFreshInitModelRef(ranked[0]) : '';
        }
      } catch (_) {
        if (!this.freshInitModelManual && template) {
          var provider = String(template.provider || '').trim();
          var model = String(template.model || '').trim();
          this.freshInitModelSelection = provider && model ? (provider.toLowerCase() + '/' + model) : '';
        }
      } finally {
        this.freshInitModelSuggestLoading = false;
      }
    },
  };
}
