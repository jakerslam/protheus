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

    get modelDisplayName() {
      if (!this.currentAgent) return '';
      var selected = String(this.currentAgent.model_name || '').trim();
      var runtime = String(this.currentAgent.runtime_model || '').trim();
      if (selected.toLowerCase() === 'auto') {
        var resolved = runtime ? runtime.replace(/-\d{8}$/, '') : '';
        var autoLabel = resolved ? ('Auto: ' + resolved) : 'Auto';
        return autoLabel.length > 24 ? autoLabel.substring(0, 22) + '\u2026' : autoLabel;
      }
      var short = selected.replace(/-\d{8}$/, '');
      return short.length > 24 ? short.substring(0, 22) + '\u2026' : short;
    },

    get switcherProviders() {
      var seen = {};
      (this._modelCache || []).forEach(function(m) { seen[m.provider] = true; });
      return Object.keys(seen).sort();
    },

    get filteredSwitcherModels() {
      var models = this._modelCache || [];
      var provFilter = this.modelSwitcherProviderFilter;
      var textFilter = this.modelSwitcherFilter ? this.modelSwitcherFilter.toLowerCase() : '';
      var filtered = models.filter(function(m) {
        if (provFilter && m.provider !== provFilter) return false;
        if (textFilter) {
          return m.id.toLowerCase().indexOf(textFilter) !== -1 ||
                 (m.display_name || '').toLowerCase().indexOf(textFilter) !== -1 ||
                 m.provider.toLowerCase().indexOf(textFilter) !== -1;
        }
        return true;
      });
      var self = this;
      filtered.sort(function(a, b) {
        var aId = String((a && a.id) || '').trim();
        var bId = String((b && b.id) || '').trim();
        var aUsage = self.modelUsageTs(aId);
        var bUsage = self.modelUsageTs(bId);
        if (bUsage !== aUsage) return bUsage - aUsage;
        var activeIds = self.activeModelCandidateIds();
        var aActive = aId && activeIds.indexOf(aId) >= 0 ? 1 : 0;
        var bActive = bId && activeIds.indexOf(bId) >= 0 ? 1 : 0;
        if (bActive !== aActive) return bActive - aActive;
        var aProvider = String((a && a.provider) || '').toLowerCase();
        var bProvider = String((b && b.provider) || '').toLowerCase();
        if (aProvider !== bProvider) return aProvider.localeCompare(bProvider);
        return aId.toLowerCase().localeCompare(bId.toLowerCase());
      });
      return filtered;
    },

    activeModelCandidateIds: function() {
      var out = [];
      var seen = {};
      var add = function(value) {
        var id = String(value || '').trim();
        if (!id || seen[id]) return;
        seen[id] = true;
        out.push(id);
      };
      var agent = this.currentAgent || {};
      var selected = String(agent.model_name || '').trim();
      var runtime = String(agent.runtime_model || '').trim();
      var provider = String(agent.model_provider || '').trim().toLowerCase();
      if (selected) add(selected);
      if (runtime) add(runtime);
      if (selected && provider && provider !== 'ollama' && selected.indexOf('/') < 0) add(provider + '/' + selected);
      if (runtime && provider && provider !== 'ollama' && runtime.indexOf('/') < 0) add(provider + '/' + runtime);
      return out;
    },

    isSwitcherModelActive: function(model) {
      var id = String(model && model.id ? model.id : '').trim();
      if (!id) return false;
      return this.activeModelCandidateIds().indexOf(id) >= 0;
    },

    resolveActiveSwitcherModel: function(filtered) {
      var rows = Array.isArray(filtered) ? filtered : [];
      var activeIds = this.activeModelCandidateIds();
      for (var i = 0; i < activeIds.length; i++) {
        var id = activeIds[i];
        for (var j = 0; j < rows.length; j++) {
          var row = rows[j];
          if (row && String(row.id || '').trim() === id) return row;
        }
      }
      var agent = this.currentAgent || null;
      if (!agent) return null;
      var selected = String(agent.model_name || '').trim();
      var runtime = String(agent.runtime_model || '').trim();
      var provider = String(agent.model_provider || '').trim();
      var activeId = selected.toLowerCase() === 'auto' && runtime ? runtime : (selected || runtime);
      if (!activeId) return null;
      return {
        id: activeId,
        provider: provider || (activeId.indexOf('/') >= 0 ? activeId.split('/')[0] : 'unknown'),
        display_name: activeId.indexOf('/') >= 0 ? activeId.split('/').slice(-1)[0] : activeId,
        tier: 'Active',
        context_window: Number(agent.context_window || 0) || null,
        is_local: provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp',
        deployment: (provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp') ? 'local' : (provider.toLowerCase() === 'cloud' ? 'cloud' : 'api'),
        power_rating: 3,
        cost_rating: provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp' ? 1 : 3,
        specialty: 'general',
        specialty_tags: ['general'],
        local_download_path: '',
        download_available: false,
      };
    },

    get groupedSwitcherModels() {
      var filtered = this.filteredSwitcherModels;
      var groups = [];
      var active = this.resolveActiveSwitcherModel(filtered);
      if (active) groups.push({ provider: 'Active', models: [active] });
      var activeId = active ? String(active.id || '').trim() : '';
      var recent = filtered.filter(function(m) {
        var id = String((m && m.id) || '').trim();
        return !activeId || id !== activeId;
      });
      if (recent.length) {
        groups.push({ provider: 'Recent', models: recent });
      } else if (!groups.length && filtered.length) {
        groups.push({ provider: 'Recent', models: filtered });
      }
      return groups;
    },

    modelSwitcherItemName: function(m) {
      var model = m || {};
      var provider = String(model.provider || '').trim();
      var id = String(model.id || '').trim();
      var display = String(model.display_name || id).trim();
      var isAutoRow = provider.toLowerCase() === 'auto' || id.toLowerCase() === 'auto';
      if (!isAutoRow) return display || id || 'model';
      var activeAuto = this.currentAgent && String(this.currentAgent.model_name || '').trim().toLowerCase() === 'auto';
      var runtime = activeAuto ? String(this.currentAgent.runtime_model || '').trim() : '';
      if (!runtime) return 'Auto';
      var short = runtime.replace(/-\d{8}$/, '');
      return short ? ('Auto: ' + short) : 'Auto';
    },

    modelDeploymentKind: function(model) {
      var row = model || {};
      var deployment = String(row.deployment || '').trim().toLowerCase();
      if (deployment === 'local' || deployment === 'cloud' || deployment === 'api') return deployment;
      if (row.is_local === true) return 'local';
      var provider = String(row.provider || '').trim().toLowerCase();
      if (provider === 'ollama' || provider === 'llama.cpp') return 'local';
      if (provider === 'cloud') return 'cloud';
      return 'api';
    },

    modelDeploymentLabel: function(model) {
      var kind = this.modelDeploymentKind(model);
      if (kind === 'local') return 'Local model';
      if (kind === 'api') return 'API model';
      return 'Cloud model';
    },

    normalizeModelRating: function(value, fallback) {
      var level = Number(value);
      var base = Number(fallback);
      if (!Number.isFinite(base)) base = 3;
      if (!Number.isFinite(level)) level = base;
      level = Math.round(level);
      if (level < 1) level = 1;
      if (level > 5) level = 5;
      return level;
    },

    modelPowerLevel: function(model) {
      return this.normalizeModelRating(model && model.power_rating, 3);
    },

    modelCostLevel: function(model) {
      return this.normalizeModelRating(model && model.cost_rating, 3);
    },

    modelContextWindowLabel: function(model) {
      var raw = Number(model && model.context_window != null ? model.context_window : 0);
      if (!Number.isFinite(raw) || raw <= 0) return '? ctx';
      return this.formatTokenK(raw) + ' ctx';
    },

    inferModelParamsFromId: function(model) {
      var id = String((model && (model.display_name || model.id)) || '').toLowerCase();
      if (!id) return 0;
      var pair = id.match(/([0-9]+(?:\.[0-9]+)?)x([0-9]+(?:\.[0-9]+)?)b/i);
      if (pair && pair[1] && pair[2]) {
        var left = Number(pair[1]);
        var right = Number(pair[2]);
        if (Number.isFinite(left) && Number.isFinite(right) && left > 0 && right > 0) return left * right;
      }
      var bMatch = id.match(/(?:^|[^a-z0-9])([0-9]+(?:\.[0-9]+)?)b(?:[^a-z0-9]|$)/i);
      if (bMatch && bMatch[1]) {
        var b = Number(bMatch[1]);
        if (Number.isFinite(b) && b > 0) return b;
      }
      var mMatch = id.match(/(?:^|[^a-z0-9])([0-9]{3,5})m(?:[^a-z0-9]|$)/i);
      if (mMatch && mMatch[1]) {
        var m = Number(mMatch[1]);
        if (Number.isFinite(m) && m > 0) return m / 1000;
      }
      return 0;
    },

    modelParamCountB: function(model) {
      var raw = Number(model && model.param_count_billion != null ? model.param_count_billion : 0);
      if (Number.isFinite(raw) && raw > 0) return raw;
      return this.inferModelParamsFromId(model);
    },

    modelParamLabel: function(model) {
      var params = this.modelParamCountB(model);
      if (!Number.isFinite(params) || params <= 0) return '? params';
      if (params >= 100) return Math.round(params) + 'B';
      if (params >= 10) return (Math.round(params * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'B';
      if (params >= 1) return (Math.round(params * 100) / 100).toFixed(2).replace(/0$/, '').replace(/\.$/, '') + 'B';
      return Math.max(1, Math.round(params * 1000)) + 'M';
    },

    modelSpecialtyLabel: function(model) {
      var raw = String(model && model.specialty ? model.specialty : '').trim().toLowerCase();
      if (!raw) return 'General';
      if (raw === 'coding') return 'Coding';
      if (raw === 'reasoning') return 'Reasoning';
      if (raw === 'vision') return 'Vision';
      if (raw === 'speed') return 'Fast';
      return raw.charAt(0).toUpperCase() + raw.slice(1);
    },

    modelDownloadProgressValue: function(model) {
      var key = this.modelDownloadKey(model);
      if (!key || !this.modelDownloadProgress) return 0;
      var raw = Number(this.modelDownloadProgress[key] || 0);
      if (!Number.isFinite(raw) || raw <= 0) return 0;
      if (raw >= 100) return 100;
      return Math.max(1, Math.min(99, Math.round(raw)));
    },

    modelDownloadProgressStyle: function(model) {
      return 'width:' + this.modelDownloadProgressValue(model) + '%';
    },

    setModelDownloadProgress: function(key, value) {
      if (!key) return;
