// Chat model switcher derived view-state helpers.
'use strict';

function chatModelSwitcherViewState(vm) {
  var modelsRef = Array.isArray(vm._modelCache) ? vm._modelCache : [];
  var providerFilter = String(vm.modelSwitcherProviderFilter || '').trim();
  var textFilter = String(vm.modelSwitcherFilter || '').trim().toLowerCase();
  var cacheTime = Number(vm._modelCacheTime || 0);
  var cache = vm._modelSwitcherViewCache;
  if (
    cache &&
    cache.modelsRef === modelsRef &&
    cache.providerFilter === providerFilter &&
    cache.textFilter === textFilter &&
    cache.cacheTime === cacheTime
  ) {
    return cache.value;
  }

  var seenProviders = {};
  for (var pi = 0; pi < modelsRef.length; pi += 1) {
    var providerName = String(modelsRef[pi] && modelsRef[pi].provider ? modelsRef[pi].provider : '').trim();
    if (providerName) seenProviders[providerName] = true;
  }
  var providers = Object.keys(seenProviders).sort();

  var filtered = modelsRef.filter(function(m) {
    var row = m || {};
    var rowProvider = String(row.provider || '').trim();
    var rowId = String(row.id || '').trim();
    var rowDisplay = String(row.display_name || '').trim();
    if (providerFilter && rowProvider !== providerFilter) return false;
    if (!textFilter) return true;
    return rowId.toLowerCase().indexOf(textFilter) !== -1 ||
      rowDisplay.toLowerCase().indexOf(textFilter) !== -1 ||
      rowProvider.toLowerCase().indexOf(textFilter) !== -1;
  });

  var activeIds = vm.activeModelCandidateIds();
  var activeMap = {};
  for (var ai = 0; ai < activeIds.length; ai += 1) {
    activeMap[String(activeIds[ai] || '').trim()] = true;
  }
  var usageCache = {};
  var usageFor = function(id) {
    var key = String(id || '').trim();
    if (!key) return 0;
    if (Object.prototype.hasOwnProperty.call(usageCache, key)) return usageCache[key];
    var ts = vm.modelUsageTs(key);
    usageCache[key] = ts;
    return ts;
  };

  filtered.sort(function(a, b) {
    var aId = String((a && a.id) || '').trim();
    var bId = String((b && b.id) || '').trim();
    var aAvailable = !(a && a.available === false) ? 1 : 0;
    var bAvailable = !(b && b.available === false) ? 1 : 0;
    if (bAvailable !== aAvailable) return bAvailable - aAvailable;
    var aUsage = usageFor(aId);
    var bUsage = usageFor(bId);
    if (bUsage !== aUsage) return bUsage - aUsage;
    var aActive = aId && activeMap[aId] ? 1 : 0;
    var bActive = bId && activeMap[bId] ? 1 : 0;
    if (bActive !== aActive) return bActive - aActive;
    var aProvider = String((a && a.provider) || '').toLowerCase();
    var bProvider = String((b && b.provider) || '').toLowerCase();
    if (aProvider !== bProvider) return aProvider.localeCompare(bProvider);
    return aId.toLowerCase().localeCompare(bId.toLowerCase());
  });

  var maxVisible = (textFilter || providerFilter) ? 240 : 120;
  var rendered = filtered.length > maxVisible ? filtered.slice(0, maxVisible) : filtered.slice();
  var groups = [];
  var cursor = 0;
  var active = vm.resolveActiveSwitcherModel(rendered.length ? rendered : filtered);
  var activeId = '';
  if (active) {
    activeId = String(active.id || '').trim();
    groups.push({
      provider: 'Active',
      models: [Object.assign({}, active, { _switcherIndex: cursor++ })]
    });
  }
  var recent = rendered.filter(function(m) {
    var id = String((m && m.id) || '').trim();
    return !activeId || id !== activeId;
  });
  if (recent.length) {
    groups.push({
      provider: 'Recent',
      models: recent.map(function(row) {
        return Object.assign({}, row, { _switcherIndex: cursor++ });
      })
    });
  } else if (!groups.length && rendered.length) {
    groups.push({
      provider: 'Recent',
      models: rendered.map(function(row) {
        return Object.assign({}, row, { _switcherIndex: cursor++ });
      })
    });
  }

  var value = {
    providers: providers,
    filtered: filtered,
    rendered: rendered,
    grouped: groups,
    totalCount: filtered.length,
    truncatedCount: Math.max(0, filtered.length - rendered.length),
  };
  vm._modelSwitcherViewCache = {
    modelsRef: modelsRef,
    providerFilter: providerFilter,
    textFilter: textFilter,
    cacheTime: cacheTime,
    value: value,
  };
  return value;
}

function chatModelSwitcherProviders(vm) {
  return chatModelSwitcherViewState(vm).providers;
}

function chatFilteredSwitcherModels(vm) {
  return chatModelSwitcherViewState(vm).filtered;
}

function chatRenderedSwitcherModels(vm) {
  return chatModelSwitcherViewState(vm).rendered;
}

function chatModelSwitcherTruncatedCount(vm) {
  return chatModelSwitcherViewState(vm).truncatedCount;
}

function chatGroupedSwitcherModels(vm) {
  return chatModelSwitcherViewState(vm).grouped;
}
