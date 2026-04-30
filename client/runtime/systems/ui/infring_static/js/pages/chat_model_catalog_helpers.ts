'use strict';

function chatCountAvailableModelRows(rows) {
  var list = Array.isArray(rows) ? rows : [];
  var count = 0;
  for (var i = 0; i < list.length; i += 1) {
    var row = list[i] || {};
    if (row.available !== false) count += 1;
  }
  return count;
}

function chatProviderPayloadToModelCatalogRows(page, payload) {
  var providers = payload && Array.isArray(payload.providers) ? payload.providers : [];
  var out = [];
  for (var i = 0; i < providers.length; i += 1) {
    var providerRow = providers[i] && typeof providers[i] === 'object' ? providers[i] : {};
    var provider = String(providerRow.id || '').trim().toLowerCase();
    if (!provider || provider === 'auto') continue;
    var isLocal = providerRow.is_local === true;
    var reachable = providerRow.reachable === true;
    var supportsChat = providerRow.supports_chat !== false;
    var needsKey = providerRow.needs_key === true;
    var authStatus = String(providerRow.auth_status || '').trim().toLowerCase();
    var authConfigured = authStatus === 'configured' || authStatus === 'set' || authStatus === 'ok';
    var profiles = providerRow.model_profiles && typeof providerRow.model_profiles === 'object'
      ? providerRow.model_profiles
      : {};
    var names = Object.keys(profiles);
    for (var j = 0; j < names.length; j += 1) {
      var modelName = String(names[j] || '').trim();
      if (!modelName) continue;
      var modelRef = provider + '/' + modelName;
      if (page.isPlaceholderModelRef(modelRef)) continue;
      var profile = profiles[modelName] && typeof profiles[modelName] === 'object' ? profiles[modelName] : {};
      var deployment = String(profile.deployment_kind || '').trim().toLowerCase();
      var rowLocal = isLocal || deployment === 'local' || deployment === 'ollama';
      var available = supportsChat && (rowLocal ? reachable : (!needsKey || authConfigured || reachable));
      out.push({
        id: modelRef,
        provider: provider,
        model: modelName,
        model_name: modelName,
        runtime_model: modelName,
        display_name: String(profile.display_name || modelName).trim() || modelName,
        available: !!available,
        reachable: !!reachable,
        supports_chat: supportsChat,
        needs_key: !!needsKey,
        auth_status: authStatus || 'unknown',
        is_local: rowLocal,
        deployment_kind: deployment || (rowLocal ? 'local' : 'api'),
        context_window: Number(profile.context_window || profile.context_size || profile.context_tokens || 0) || 0,
        context_window_tokens: Number(profile.context_window || profile.context_size || profile.context_tokens || 0) || 0,
        power_rating: Number(profile.power_rating || 3) || 3,
        cost_rating: Number(profile.cost_rating || (rowLocal ? 1 : 3)) || (rowLocal ? 1 : 3),
        specialty: String(profile.specialty || 'general').trim().toLowerCase() || 'general',
        specialty_tags: Array.isArray(profile.specialty_tags) ? profile.specialty_tags : ['general'],
        param_count_billion: Number(profile.param_count_billion || 0) || 0,
        download_available: profile.download_available === true || rowLocal,
        local_download_path: String(profile.local_download_path || '').trim(),
        max_output_tokens: Number(profile.max_output_tokens || 0) || 0,
      });
    }
  }
  return out;
}

function chatMergeModelCatalogRows(primaryRows, fallbackRows) {
  var merged = [];
  var seen = {};
  var add = function(row) {
    var id = String(row && row.id ? row.id : '').trim();
    if (!id) return;
    var key = id.toLowerCase();
    if (seen[key]) return;
    seen[key] = true;
    merged.push(row);
  };
  var primary = Array.isArray(primaryRows) ? primaryRows : [];
  var fallback = Array.isArray(fallbackRows) ? fallbackRows : [];
  for (var i = 0; i < primary.length; i += 1) add(primary[i]);
  for (var j = 0; j < fallback.length; j += 1) add(fallback[j]);
  return merged;
}

function chatModelCatalogRows(page, rows) {
  var list = Array.isArray(rows) && rows.length
    ? rows
    : (
      Array.isArray(page.modelPickerList) && page.modelPickerList.length
        ? page.modelPickerList
        : (Array.isArray(page._modelCache) ? page._modelCache : [])
    );
  return page.sanitizeModelCatalogRows(list);
}

function chatResolveModelCatalogOption(page, value, providerHint, rows) {
  var list = chatModelCatalogRows(page, rows);
  var raw = value && typeof value === 'object'
    ? String(value.id || value.model || value.model_name || value.runtime_model || '').trim()
    : String(value || '').trim();
  var provider = value && typeof value === 'object'
    ? String(value.provider || value.model_provider || providerHint || '').trim().toLowerCase()
    : String(providerHint || '').trim().toLowerCase();
  if (!raw || page.isPlaceholderModelRef(raw)) return null;

  var candidates = [];
  var seen = {};
  var addCandidate = function(candidate) {
    var next = String(candidate || '').trim();
    if (!next) return;
    var key = next.toLowerCase();
    if (seen[key]) return;
    seen[key] = true;
    candidates.push(next);
  };
  addCandidate(raw);
  if (provider && raw.indexOf('/') < 0) addCandidate(provider + '/' + raw);
  if (raw.indexOf('/') >= 0) addCandidate(raw.split('/').slice(-1)[0]);

  var fallbackMatches = [];
  for (var i = 0; i < list.length; i += 1) {
    var row = list[i] || {};
    var rowId = String(row.id || '').trim();
    var rowProvider = String(row.provider || row.model_provider || '').trim().toLowerCase();
    var rowModel = String(row.model || row.model_name || row.runtime_model || '').trim();
    var rowDisplay = String(row.display_name || '').trim();
    for (var j = 0; j < candidates.length; j += 1) {
      var candidate = candidates[j];
      var candidateLower = candidate.toLowerCase();
      if (rowId && rowId.toLowerCase() === candidateLower) return row;
      if (rowModel && rowModel.toLowerCase() === candidateLower) {
        if (!provider || rowProvider === provider) return row;
        fallbackMatches.push(row);
      }
      if (rowDisplay && rowDisplay.toLowerCase() === candidateLower) {
        if (!provider || rowProvider === provider) return row;
        fallbackMatches.push(row);
      }
    }
  }
  if (provider) {
    for (var k = 0; k < fallbackMatches.length; k += 1) {
      var fallback = fallbackMatches[k] || {};
      if (String(fallback.provider || fallback.model_provider || '').trim().toLowerCase() === provider) {
        return fallback;
      }
    }
  }
  return fallbackMatches.length ? fallbackMatches[0] : null;
}

function chatResolveProviderScopedModelCatalogOption(page, providerValue, modelValue, rows) {
  var provider = String(providerValue || '').trim().toLowerCase();
  var list = chatModelCatalogRows(page, rows);
  if (!provider) return chatResolveModelCatalogOption(page, modelValue, '', list);
  var resolved = chatResolveModelCatalogOption(page, modelValue, provider, list);
  if (resolved && String(resolved.provider || resolved.model_provider || '').trim().toLowerCase() === provider) {
    return resolved;
  }
  var rawModel = String(modelValue || '').trim();
  var targetModel = rawModel.indexOf('/') >= 0 ? rawModel.split('/').slice(-1)[0] : rawModel;
  var matches = [];
  for (var i = 0; i < list.length; i += 1) {
    var row = list[i] || {};
    var rowProvider = String(row.provider || row.model_provider || '').trim().toLowerCase();
    if (rowProvider !== provider) continue;
    if (!targetModel) {
      matches.push(row);
      continue;
    }
    var rowModel = String(row.model || row.model_name || row.runtime_model || '').trim();
    var rowId = String(row.id || '').trim();
    var exactId = rowId && rowId.toLowerCase() === (provider + '/' + targetModel).toLowerCase();
    var exactModel = rowModel && rowModel.toLowerCase() === targetModel.toLowerCase();
    if (exactId || exactModel) return row;
    matches.push(row);
  }
  if (!matches.length) return resolved || null;
  for (var j = 0; j < matches.length; j += 1) {
    if (matches[j] && matches[j].available !== false) return matches[j];
  }
  return matches[0];
}

function chatDedupeFallbackModelList(page, entries, options) {
  var list = Array.isArray(entries) ? entries : [];
  var opts = options && typeof options === 'object' ? options : {};
  var rows = chatModelCatalogRows(page, opts.rows);
  var primary = chatResolveModelCatalogOption(page, opts.primary_id || '', opts.primary_provider || '', rows);
  var primaryId = String(primary && primary.id ? primary.id : '').trim().toLowerCase();
  var out = [];
  var seen = {};
  for (var i = 0; i < list.length; i += 1) {
    var entry = list[i];
    var raw = entry && typeof entry === 'object' ? entry : { model: entry };
    var provider = String(raw.provider || raw.model_provider || '').trim();
    var model = String(raw.model || raw.model_name || raw.runtime_model || raw.id || '').trim();
    if (!model || page.isPlaceholderModelRef(model)) continue;
    var resolved = provider
      ? chatResolveProviderScopedModelCatalogOption(page, provider, model, rows)
      : chatResolveModelCatalogOption(page, model, '', rows);
    var normalizedProvider = String(
      (resolved && (resolved.provider || resolved.model_provider)) || provider || ''
    ).trim();
    var normalizedModel = String(
      (resolved && (resolved.model || resolved.model_name || resolved.runtime_model)) || model
    ).trim();
    var normalizedId = String(
      (resolved && resolved.id) ||
      (normalizedProvider && normalizedModel ? (normalizedProvider + '/' + normalizedModel) : normalizedModel)
    ).trim();
    if (!normalizedId || page.isPlaceholderModelRef(normalizedId)) continue;
    var dedupeKey = normalizedId.toLowerCase();
    if (primaryId && dedupeKey === primaryId) continue;
    if (seen[dedupeKey]) continue;
    seen[dedupeKey] = true;
    out.push({
      provider: normalizedProvider || String(provider || '').trim(),
      model: normalizedModel
    });
  }
  return out;
}

function chatFilteredModelPicker(page) {
  if (!page.modelPickerFilter) return page.modelPickerList.slice(0, 15);
  var filter = page.modelPickerFilter;
  return page.modelPickerList.filter(function(model) {
    return model.id.toLowerCase().indexOf(filter) !== -1 ||
      (model.display_name || '').toLowerCase().indexOf(filter) !== -1 ||
      model.provider.toLowerCase().indexOf(filter) !== -1;
  }).slice(0, 15);
}

function infringChatModelCatalogDelegateMethods() {
  return {
    isPlaceholderModelRef: function(value) {
      var id = String(value || '').trim().toLowerCase();
      if (!id) return true;
      if (id === 'model' || id === '<model>' || id === '(model)') return true;
      if (id.indexOf('/') >= 0) {
        var tail = String(id.split('/').slice(-1)[0] || '').trim();
        if (!tail) return true;
        return tail === 'model' || tail === '<model>' || tail === '(model)';
      }
      return false;
    },
    buildQualifiedModelRef: function(modelValue, providerValue) {
      var model = String(modelValue || '').trim();
      var provider = String(providerValue || '').trim().toLowerCase();
      if (!model || this.isPlaceholderModelRef(model)) return '';
      if (!provider) return model;
      var normalizedPrefix = provider + '/';
      if (model.toLowerCase().indexOf(normalizedPrefix) === 0) return model;
      if (model.indexOf('/') >= 0) return model;
      return provider + '/' + model;
    },
    normalizeModelOverrideValue: function(modelValue) {
      if (!modelValue || typeof modelValue !== 'object') {
        return {
          kind: '',
          value: String(modelValue || '').trim()
        };
      }
      var kind = String(modelValue.kind || '').trim().toLowerCase();
      var value = String(
        modelValue.value ||
        modelValue.model ||
        modelValue.id ||
        ''
      ).trim();
      return {
        kind: kind === 'qualified' || kind === 'raw' ? kind : '',
        value: value
      };
    },
    normalizeQualifiedModelRef: function(modelValue, providerValue, rows) {
      var override = this.normalizeModelOverrideValue(modelValue);
      var raw = String(override.value || '').trim();
      if (!raw || this.isPlaceholderModelRef(raw)) return '';
      if (override.kind === 'qualified') return raw;
      if (typeof this.resolveModelCatalogOption === 'function') {
        var resolved = this.resolveModelCatalogOption(raw, providerValue || '', rows);
        var resolvedId = String(resolved && resolved.id ? resolved.id : '').trim();
        if (resolvedId) return resolvedId;
      }
      return this.buildQualifiedModelRef(raw, providerValue);
    },
    formatQualifiedModelDisplay: function(value) {
      var ref = String(value || '').trim();
      if (!ref || this.isPlaceholderModelRef(ref)) return '';
      if (ref.indexOf('/') < 0) return ref;
      var parts = ref.split('/');
      var provider = String(parts[0] || '').trim();
      var model = String(parts.slice(1).join('/') || '').trim();
      if (!model) return provider || ref;
      if (!provider) return model;
      return model + ' · ' + provider;
    },
    truncateModelLabel: function(value) {
      var raw = this.normalizeQualifiedModelRef(value, '', this._modelCache || []);
      if (!raw || this.isPlaceholderModelRef(raw)) return '';
      var compact = raw;
      if (raw.indexOf('/') >= 0) {
        var parts = raw.split('/');
        var tail = String(parts[parts.length - 1] || '').trim();
        var head = String(parts[0] || '').trim();
        compact = tail || head;
      }
      compact = String(compact || '').trim();
      if (!compact || this.isPlaceholderModelRef(compact)) return '';
      return compact.replace(/-\d{8}$/, '');
    },

    // Backward-compat shim for legacy callers during naming migration.
    compactModelLabel: function(value) {
      return this.truncateModelLabel(value);
    },
    sanitizeModelCatalogRows: function(rows) {
      var list = Array.isArray(rows) ? rows : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < list.length; i += 1) {
        var row = list[i] && typeof list[i] === 'object' ? list[i] : {};
        var provider = String(row.provider || row.model_provider || '').trim();
        var modelName = String(row.model || row.model_name || row.runtime_model || row.id || '').trim();
        var id = this.buildQualifiedModelRef(row.id || modelName, provider);
        if (!id || this.isPlaceholderModelRef(id)) continue;
        if (!provider && id.indexOf('/') >= 0) provider = String(id.split('/')[0] || '').trim();
        if (!provider) provider = 'unknown';
        var key = id.toLowerCase();
        if (seen[key]) continue;
        seen[key] = true;
        var normalizedModelName = modelName;
        if (!normalizedModelName && id.indexOf('/') >= 0) {
          normalizedModelName = String(id.split('/').slice(1).join('/') || '').trim();
        }
        out.push(Object.assign({}, row, {
          id: id,
          provider: provider,
          model: normalizedModelName || id,
          model_name: normalizedModelName || id,
          display_name: String(row.display_name || normalizedModelName || this.formatQualifiedModelDisplay(id) || id).trim(),
          available: row.available !== false
        }));
      }
      return out;
    },
    activeModelCandidateIds: function() {
      var out = [];
      var seen = {};
      var self = this;
      var add = function(value) {
        var id = self.normalizeQualifiedModelRef(value, provider, self._modelCache || []);
        if (!id || self.isPlaceholderModelRef(id) || seen[id]) return;
        seen[id] = true;
        out.push(id);
      };
      var agent = this.currentAgent || {};
      var selected = String(agent.model_name || '').trim();
      var runtime = String(agent.runtime_model || '').trim();
      var provider = String(agent.model_provider || '').trim().toLowerCase();
      if (selected) add(selected);
      if (runtime) add(runtime);
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
      activeId = this.normalizeQualifiedModelRef(activeId, provider, rows);
      if (!activeId || this.isPlaceholderModelRef(activeId)) return null;
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
  };
}

function infringChatModelCatalogForwarderMethods() {
  return {
    countAvailableModelRows: function(rows) {
      return chatCountAvailableModelRows(rows);
    },

    // Backward-compat shim for legacy callers during naming migration.
    availableModelRowsCount: function(rows) {
      return this.countAvailableModelRows(rows);
    },

    providerPayloadToModelCatalogRows: function(payload) {
      return chatProviderPayloadToModelCatalogRows(this, payload);
    },

    mergeModelCatalogRows: function(primaryRows, fallbackRows) {
      return chatMergeModelCatalogRows(primaryRows, fallbackRows);
    },

    modelCatalogRows: function(rows) {
      return chatModelCatalogRows(this, rows);
    },

    resolveModelCatalogOption: function(value, providerHint, rows) {
      return chatResolveModelCatalogOption(this, value, providerHint, rows);
    },

    resolveProviderScopedModelCatalogOption: function(providerValue, modelValue, rows) {
      return chatResolveProviderScopedModelCatalogOption(this, providerValue, modelValue, rows);
    },

    dedupeFallbackModelList: function(entries, options) {
      return chatDedupeFallbackModelList(this, entries, options);
    },
  };
}
