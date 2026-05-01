'use strict';

function infringChatModelVisualMethods() {
  return {
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
    modelLogoFamilyKey: function(model) {
      var row = model && typeof model === 'object' ? model : {};
      var combined = String(
        row.id || row.display_name || row.model_name || row.name || ''
      ).toLowerCase();
      var provider = String(row.provider || row.model_provider || '').toLowerCase();
      var haystack = (provider + ' ' + combined).trim();
      if (!haystack) return 'unknown';
      if (haystack.indexOf('openai') >= 0 || haystack.indexOf('chatgpt') >= 0 || haystack.indexOf('gpt') >= 0) return 'openai';
      if (haystack.indexOf('anthropic') >= 0 || haystack.indexOf('claude') >= 0 || haystack.indexOf('frontier_provider') >= 0) return 'anthropic';
      if (haystack.indexOf('gemini') >= 0 || haystack.indexOf('google') >= 0) return 'gemini';
      if (haystack.indexOf('qwen') >= 0) return 'qwen';
      if (haystack.indexOf('deepseek') >= 0) return 'deepseek';
      if (haystack.indexOf('kimi') >= 0 || haystack.indexOf('moonshot') >= 0) return 'kimi';
      if (haystack.indexOf('llama') >= 0 || haystack.indexOf('meta') >= 0) return 'llama';
      if (haystack.indexOf('mistral') >= 0 || haystack.indexOf('mixtral') >= 0) return 'mistral';
      if (haystack.indexOf('grok') >= 0 || haystack.indexOf('xai') >= 0) return 'xai';
      return 'unknown';
    },
    modelLogoSimpleIconUrl: function(slug) {
      var key = String(slug || '').trim().toLowerCase();
      if (!key) return '';
      return 'https://cdn.simpleicons.org/' + encodeURIComponent(key);
    },
    modelLogoClearbitUrl: function(domain) {
      var value = String(domain || '').trim().toLowerCase();
      if (!value) return '';
      return 'https://logo.clearbit.com/' + encodeURIComponent(value) + '?size=64&format=png';
    },
    pushUniqueLogoCandidate: function(list, url) {
      var value = String(url || '').trim();
      if (!value) return;
      if (list.indexOf(value) >= 0) return;
      list.push(value);
    },
    modelLogoCandidates: function(model) {
      var key = this.modelLogoFamilyKey(model);
      var out = [];
      if (key === 'openai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('openai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('openai.com'));
      } else if (key === 'anthropic') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('anthropic'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('anthropic.com'));
      } else if (key === 'gemini') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('googlegemini'));
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('google'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('google.com'));
      } else if (key === 'qwen') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('qwen'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('alibabacloud.com'));
      } else if (key === 'deepseek') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('deepseek'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('deepseek.com'));
      } else if (key === 'kimi') {
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.ai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.cn'));
      } else if (key === 'llama') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('meta'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('meta.com'));
      } else if (key === 'mistral') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('mistralai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('mistral.ai'));
      } else if (key === 'xai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('x'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('x.ai'));
      }
      return out;
    },
    modelLogoFailMap: function(kind) {
      var scope = String(kind || '').trim().toLowerCase() === 'source' ? 'source' : 'model';
      if (scope === 'source') {
        if (!this._modelSourceLogoFailIndex || typeof this._modelSourceLogoFailIndex !== 'object') {
          this._modelSourceLogoFailIndex = {};
        }
        return this._modelSourceLogoFailIndex;
      }
      if (!this._modelLogoFailIndex || typeof this._modelLogoFailIndex !== 'object') {
        this._modelLogoFailIndex = {};
      }
      return this._modelLogoFailIndex;
    },
    modelLogoUrl: function(model) {
      var key = this.modelLogoFamilyKey(model);
      if (!key || key === 'unknown') return '';
      var candidates = this.modelLogoCandidates(model);
      if (!candidates.length) return '';
      var map = this.modelLogoFailMap('model');
      var index = Number(map[key] || 0);
      if (!Number.isFinite(index) || index < 0) index = 0;
      if (index >= candidates.length) return '';
      return String(candidates[index] || '');
    },
    modelLogoTooltip: function(model) {
      var key = this.modelLogoFamilyKey(model);
      if (key === 'unknown') return 'Model family';
      if (key === 'openai') return 'Model family: OpenAI';
      if (key === 'anthropic') return 'Model family: Anthropic';
      if (key === 'gemini') return 'Model family: Gemini';
      if (key === 'qwen') return 'Model family: Qwen';
      if (key === 'deepseek') return 'Model family: DeepSeek';
      if (key === 'kimi') return 'Model family: Kimi';
      if (key === 'llama') return 'Model family: Llama';
      if (key === 'mistral') return 'Model family: Mistral';
      if (key === 'xai') return 'Model family: xAI';
      return 'Model family';
    },
    modelSourceLogoKey: function(model) {
      var row = model && typeof model === 'object' ? model : {};
      var provider = String(row.provider || row.model_provider || '').trim().toLowerCase();
      if (!provider) {
        var deployment = this.modelDeploymentKind(row);
        if (deployment === 'local') return 'local';
        if (deployment === 'cloud') return 'cloud';
        if (deployment === 'api') return 'direct';
        return 'unknown';
      }
      if (provider.indexOf('ollama') >= 0) return 'ollama';
      if (provider.indexOf('huggingface') >= 0 || provider === 'hf') return 'huggingface';
      if (provider.indexOf('openrouter') >= 0) return 'openrouter';
      if (provider.indexOf('openai') >= 0) return 'openai';
      if (provider.indexOf('frontier_provider') >= 0 || provider.indexOf('anthropic') >= 0) return 'anthropic';
      if (provider.indexOf('google') >= 0 || provider.indexOf('gemini') >= 0) return 'google';
      if (provider.indexOf('moonshot') >= 0 || provider.indexOf('kimi') >= 0) return 'moonshot';
      if (provider.indexOf('deepseek') >= 0) return 'deepseek';
      if (provider.indexOf('groq') >= 0) return 'groq';
      if (provider.indexOf('xai') >= 0) return 'xai';
      if (provider.indexOf('cloud') >= 0) return 'cloud';
      return 'direct';
    },
    modelSourceLogoCandidates: function(model) {
      var key = this.modelSourceLogoKey(model);
      var out = [];
      if (key === 'ollama') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('ollama'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('ollama.com'));
      } else if (key === 'huggingface') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('huggingface'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('huggingface.co'));
      } else if (key === 'openrouter') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('openrouter'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('openrouter.ai'));
      } else if (key === 'openai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('openai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('openai.com'));
      } else if (key === 'anthropic') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('anthropic'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('anthropic.com'));
      } else if (key === 'google') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('google'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('google.com'));
      } else if (key === 'moonshot') {
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.ai'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('moonshot.cn'));
      } else if (key === 'deepseek') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('deepseek'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('deepseek.com'));
      } else if (key === 'groq') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('groq'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('groq.com'));
      } else if (key === 'xai') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('x'));
        this.pushUniqueLogoCandidate(out, this.modelLogoClearbitUrl('x.ai'));
      } else if (key === 'local') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('docker'));
      } else if (key === 'cloud') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('icloud'));
      } else if (key === 'direct') {
        this.pushUniqueLogoCandidate(out, this.modelLogoSimpleIconUrl('chainlink'));
      }
      return out;
    },
    modelSourceLogoUrl: function(model) {
      var key = this.modelSourceLogoKey(model);
      if (!key || key === 'unknown') return '';
      var candidates = this.modelSourceLogoCandidates(model);
      if (!candidates.length) return '';
      var map = this.modelLogoFailMap('source');
      var index = Number(map[key] || 0);
      if (!Number.isFinite(index) || index < 0) index = 0;
      if (index >= candidates.length) return '';
      return String(candidates[index] || '');
    },
    onModelLogoLoad: function(event) {
      var target = event && event.target ? event.target : null;
      if (!target || !target.style) return;
      target.style.visibility = '';
    },
    onModelLogoError: function(kind, model, event) {
      var scope = String(kind || '').trim().toLowerCase() === 'source' ? 'source' : 'model';
      var key = scope === 'source' ? this.modelSourceLogoKey(model) : this.modelLogoFamilyKey(model);
      var candidates = scope === 'source' ? this.modelSourceLogoCandidates(model) : this.modelLogoCandidates(model);
      var target = event && event.target ? event.target : null;
      if (!key || !candidates.length) {
        if (target && target.style) target.style.visibility = 'hidden';
        return;
      }
      var map = this.modelLogoFailMap(scope);
      var current = Number(map[key] || 0);
      if (!Number.isFinite(current) || current < 0) current = 0;
      var next = current + 1;
      map[key] = next;
      var replacement = next < candidates.length ? String(candidates[next] || '') : '';
      if (!target || !target.style) return;
      if (replacement) {
        target.style.visibility = '';
        target.src = replacement;
        return;
      }
      target.style.visibility = 'hidden';
    },
    modelSourceLogoTooltip: function(model) {
      var key = this.modelSourceLogoKey(model);
      if (key === 'unknown') return 'Model source';
      if (key === 'ollama') return 'Source: Ollama';
      if (key === 'huggingface') return 'Source: Hugging Face';
      if (key === 'openrouter') return 'Source: OpenRouter';
      if (key === 'openai') return 'Source: OpenAI direct';
      if (key === 'anthropic') return 'Source: Anthropic direct';
      if (key === 'google') return 'Source: Google direct';
      if (key === 'moonshot') return 'Source: Moonshot direct';
      if (key === 'deepseek') return 'Source: DeepSeek direct';
      if (key === 'groq') return 'Source: Groq';
      if (key === 'xai') return 'Source: xAI direct';
      if (key === 'local') return 'Source: Local runtime';
      if (key === 'cloud') return 'Source: Cloud runtime';
      if (key === 'direct') return 'Source: Direct provider';
      return 'Model source';
    },
    modelDeploymentKind: function(model) {
      var row = model || {};
      var deployment = String(row.deployment || row.deployment_kind || '').trim().toLowerCase();
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

      if (!this.modelDownloadProgress) this.modelDownloadProgress = {};
      var raw = Number(value);
      if (!Number.isFinite(raw)) raw = 0;
      raw = Math.max(0, Math.min(100, Math.round(raw)));
      if (raw <= 0) {
        delete this.modelDownloadProgress[key];
      } else {
        this.modelDownloadProgress[key] = raw;
      }
    },

    clearModelDownloadProgressTimer: function(key) {
      if (!key) return;
      if (!this._modelDownloadProgressTimers) this._modelDownloadProgressTimers = {};
      var timer = this._modelDownloadProgressTimers[key];
      if (timer) {
        clearInterval(timer);
      }
      delete this._modelDownloadProgressTimers[key];
    },

    startModelDownloadProgressTimer: function(key) {
      if (!key) return;
      this.clearModelDownloadProgressTimer(key);
      var self = this;
      var seeded = Number(self.modelDownloadProgress && self.modelDownloadProgress[key] ? self.modelDownloadProgress[key] : 0);
      if (!Number.isFinite(seeded) || seeded <= 0) seeded = 2;
      self.setModelDownloadProgress(key, seeded);
      self._modelDownloadProgressTimers[key] = setInterval(function() {
        var current = Number(self.modelDownloadProgress[key] || 0);
        if (!Number.isFinite(current) || current <= 0) current = 2;
        if (current >= 94) return;
        var bump = current < 30 ? 7 : (current < 60 ? 4 : 2);
        self.setModelDownloadProgress(key, Math.min(94, current + bump));
      }, 520);
    },

    modelPowerIcons: function(model) {
      return 'ϟ'.repeat(this.modelPowerLevel(model));
    },

    modelCostIcons: function(model) {
      return '$'.repeat(this.modelCostLevel(model));
    },

    modelDownloadKey: function(model) {
      var row = model || {};
      var provider = String(row.provider || '').trim().toLowerCase();
      var id = String(row.id || row.display_name || '').trim().toLowerCase();
      return provider + '::' + id;
    },

    isModelDownloadable: function(model) {
      var row = model || {};
      var id = String(row.id || row.model || '').trim();
      var provider = String(row.provider || '').trim().toLowerCase();
      return !!(
        row &&
        (
          row.download_available === true ||
          String(row.local_download_path || '').trim() ||
          (id && provider && provider !== 'auto')
        )
      );
    },

    isModelDownloadBusy: function(model) {
      var key = this.modelDownloadKey(model);
      return !!(key && this.modelDownloadBusy && this.modelDownloadBusy[key] === true);
    },

    downloadModelToLocal: function(model) {
      var self = this;
      var row = model || {};
      if (!self.isModelDownloadable(row)) {
        InfringToast.error('No local download path is available for this model');
        return;
      }
      var key = self.modelDownloadKey(row);
      if (!key) return;
      if (!self.modelDownloadBusy) self.modelDownloadBusy = {};
      if (self.modelDownloadBusy[key]) return;
      self.modelDownloadBusy[key] = true;
      self.setModelDownloadProgress(key, 2);
      self.startModelDownloadProgressTimer(key);
      var modelRef = String(row.id || row.display_name || '').trim();
      var provider = String(row.provider || '').trim();
      InfringAPI.post('/api/models/download', {
        model: modelRef,
        provider: provider
      }).then(function(resp) {
        var method = String((resp && resp.method) || '').trim();
        var localPath = String((resp && resp.download_path) || '').trim();
        self.setModelDownloadProgress(key, 100);
        if (method === 'ollama_pull') {
          InfringToast.success('Model downloaded locally: ' + localPath);
          self.addNoticeEvent({
            notice_label: 'Downloaded ' + (String(row.display_name || row.id || 'model').trim()) + ' locally',
            notice_type: 'info',
            notice_icon: '⬇',
            ts: Date.now(),
          });
        } else {
          InfringToast.success('Local download path prepared: ' + localPath);
          self.addNoticeEvent({
            notice_label: 'Prepared local download path for ' + (String(row.display_name || row.id || 'model').trim()),
            notice_type: 'info',
            notice_icon: '⬇',
            ts: Date.now(),
          });
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
        InfringToast.error('Model download failed: ' + (e && e.message ? e.message : e));
        self.setModelDownloadProgress(key, 0);
      }).finally(function() {
        self.modelDownloadBusy[key] = false;
        self.clearModelDownloadProgressTimer(key);
        if (self.modelDownloadProgress && self.modelDownloadProgress[key] >= 100) {
          setTimeout(function() {
            self.setModelDownloadProgress(key, 0);
          }, 900);
        } else {
          self.setModelDownloadProgress(key, 0);
        }
      });
    },
  };
}
