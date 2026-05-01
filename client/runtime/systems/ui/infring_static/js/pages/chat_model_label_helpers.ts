// Chat model display label helpers.
'use strict';

function chatReadModelField(agent, keys) {
  var row = agent && typeof agent === 'object' ? agent : null;
  if (!row) return '';
  for (var i = 0; i < keys.length; i += 1) {
    var key = String(keys[i] || '').trim();
    if (!key) continue;
    var value = String(row[key] || '').trim();
    if (value) return value;
  }
  return '';
}

function chatModelDisplayName(vm) {
  var store = typeof vm.getAppStore === 'function' ? vm.getAppStore() : null;
  var currentId = String((vm.currentAgent && vm.currentAgent.id) || '').trim();
  var storeAgent = null;
  if (store && Array.isArray(store.agents) && currentId) {
    for (var ai = 0; ai < store.agents.length; ai += 1) {
      var row = store.agents[ai];
      if (row && String(row.id || '').trim() === currentId) {
        storeAgent = row;
        break;
      }
    }
  }
  var selected = chatReadModelField(vm.currentAgent, ['model_name', 'selected_model', 'model', 'selected_model_id']);
  var runtime = chatReadModelField(vm.currentAgent, ['runtime_model', 'current_model', 'resolved_model']);
  var modelOverride = chatReadModelField(vm.currentAgent, ['model_override', 'active_model_ref']);
  var storeSelected = chatReadModelField(storeAgent, ['model_name', 'selected_model', 'model', 'selected_model_id']);
  var storeRuntime = chatReadModelField(storeAgent, ['runtime_model', 'current_model', 'resolved_model']);
  var storeOverride = chatReadModelField(storeAgent, ['model_override', 'active_model_ref']);
  var suggestion = vm.selectedFreshInitModelSuggestion ? vm.selectedFreshInitModelSuggestion() : null;
  var suggestionRef = vm.normalizeFreshInitModelRef ? vm.normalizeFreshInitModelRef(suggestion) : '';
  var providerFallback = chatReadModelField(vm.currentAgent, ['model_provider', 'provider', 'selected_provider']);
  if (!providerFallback) providerFallback = chatReadModelField(storeAgent, ['model_provider', 'provider', 'selected_provider']);
  providerFallback = String(providerFallback || '').trim().toLowerCase();
  if (vm.isPlaceholderModelRef(selected)) selected = '';
  if (vm.isPlaceholderModelRef(runtime)) runtime = '';
  if (vm.isPlaceholderModelRef(modelOverride)) modelOverride = '';
  if (vm.isPlaceholderModelRef(storeSelected)) storeSelected = '';
  if (vm.isPlaceholderModelRef(storeRuntime)) storeRuntime = '';
  if (vm.isPlaceholderModelRef(storeOverride)) storeOverride = '';
  if (vm.isPlaceholderModelRef(suggestionRef)) suggestionRef = '';
  if (selected.toLowerCase() === 'auto') {
    var resolved = vm.truncateModelLabel(runtime);
    var autoLabel = resolved ? ('Auto: ' + resolved) : 'Auto';
    return autoLabel.length > 24 ? autoLabel.substring(0, 22) + '\u2026' : autoLabel;
  }
  var active = vm.resolveActiveSwitcherModel ? vm.resolveActiveSwitcherModel(vm._modelCache || []) : null;
  var activeId = String((active && active.id) || '').trim();
  var candidates = [selected, runtime, modelOverride, storeSelected, storeRuntime, storeOverride, suggestionRef, activeId];
  for (var ci = 0; ci < candidates.length; ci += 1) {
    var compactCandidate = vm.truncateModelLabel(candidates[ci]);
    if (!compactCandidate) continue;
    return compactCandidate.length > 24 ? compactCandidate.substring(0, 22) + '\u2026' : compactCandidate;
  }
  if (providerFallback === 'auto' || !providerFallback) return 'Auto';
  return providerFallback.length > 24 ? providerFallback.substring(0, 22) + '\u2026' : providerFallback;
}

function chatMenuModelLabel(vm) {
  var label = String(chatModelDisplayName(vm) || '').trim();
  if (!label) label = 'Auto';
  if (label.length > 7) return label.substring(0, 7) + '...';
  return label;
}
