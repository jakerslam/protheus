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

function chatFallbackModelCatalogRows(page) {
  var seeds = [
    ['openai', 'gpt-5.5', 'GPT-5.5'],
    ['openai', 'gpt-5.4', 'GPT-5.4'],
    ['openai', 'gpt-5.4-mini', 'GPT-5.4 Mini'],
    ['openai', 'gpt-5.3-codex', 'GPT-5.3 Codex'],
    ['openai', 'gpt-5.3-codex-spark', 'GPT-5.3 Codex Spark'],
    ['anthropic', 'claude-4.2', 'Claude 4.2'],
    ['anthropic', 'claude-opus-4-6', 'Claude Opus 4.6'],
    ['google', 'gemini-3', 'Gemini 3'],
    ['deepseek', 'deepseek-chat', 'DeepSeek Chat'],
    ['deepseek', 'deepseek-reasoner', 'DeepSeek Reasoner'],
    ['ollama', 'qwen2.5:3b-instruct', 'Qwen 2.5 3B Instruct']
  ];
  return page.sanitizeModelCatalogRows(seeds.map(function(seed) {
    var provider = seed[0];
    var model = seed[1];
    return {
      id: provider + '/' + model,
      provider: provider,
      model: model,
      model_name: model,
      runtime_model: model,
      display_name: seed[2],
      available: true,
      shell_catalog_seed: true
    };
  }));
}

function chatLoadProviderModelCatalogSafely(page, options) {
  var opts = options && typeof options === 'object' ? options : {};
  var cachedRows = page.sanitizeModelCatalogRows(page._modelCache || page.modelPickerList || []);
  var useRows = function(rows) {
    var models = page.sanitizeModelCatalogRows(rows);
    page._modelCache = models;
    page._modelCacheTime = Date.now();
    page.modelPickerList = models;
    return models;
  };
  var timeoutMs = Number(opts.timeout_ms || 2000);
  var timeoutFallback = new Promise(function(resolve) {
    setTimeout(function() { resolve(null); }, timeoutMs > 0 ? timeoutMs : 2000);
  });
  return Promise.race([
    InfringAPI.get('/api/providers'),
    timeoutFallback
  ]).then(function(providersPayload) {
    if (!providersPayload) {
      return useRows(cachedRows.length ? cachedRows : chatFallbackModelCatalogRows(page));
    }
    var providerRows = page.sanitizeModelCatalogRows(
      page.providerPayloadToModelCatalogRows(providersPayload)
    );
    if (!providerRows.length) {
      return useRows(cachedRows.length ? cachedRows : chatFallbackModelCatalogRows(page));
    }
    var existingRows = opts.merge_existing === false
      ? []
      : cachedRows;
    var models = page.mergeModelCatalogRows(existingRows, providerRows);
    return useRows(models);
  }).catch(function() {
    return useRows(cachedRows.length ? cachedRows : chatFallbackModelCatalogRows(page));
  });
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
