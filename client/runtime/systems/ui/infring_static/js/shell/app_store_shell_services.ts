'use strict';

var InfringSharedShellServices = (function(existing) {
  var services = existing && typeof existing === 'object' ? existing : {};
  var sourceStore = null;
  var shellRoot = null;
  var listeners = [];
  var version = 0;

  function clean(value) {
    return String(value == null ? '' : value).trim();
  }

  function currentHashPage() {
    if (typeof window === 'undefined') return '';
    try {
      return clean(String(window.location && window.location.hash || '').replace(/^#/, '')).toLowerCase();
    } catch (_) {
      return '';
    }
  }

  function current() {
    if (sourceStore && typeof sourceStore === 'object') return sourceStore;
    if (typeof window !== 'undefined' && window.InfringApp && typeof window.InfringApp === 'object') {
      sourceStore = window.InfringApp;
      return sourceStore;
    }
    return null;
  }

  function root() {
    return shellRoot && typeof shellRoot === 'object' ? shellRoot : null;
  }

  function agentsFrom(source) {
    return source && Array.isArray(source.agents) ? source.agents : [];
  }

  function snapshot() {
    var store = current() || {};
    var rootState = root() || {};
    var agents = agentsFrom(store);
    var page = clean(rootState.page || store.page || currentHashPage() || 'chat').toLowerCase();
    var theme = clean(rootState.theme || store.theme);
    var themeMode = clean(rootState.themeMode || store.themeMode || theme);
    return {
      version: version,
      page: page,
      route: page,
      theme: theme,
      themeMode: themeMode,
      agents: agents,
      agentCount: Number(store.agentCount || agents.length || 0) || 0,
      activeAgentId: clean(store.activeAgentId),
      pendingFreshAgentId: clean(store.pendingFreshAgentId),
      focusMode: !!store.focusMode,
      connected: !!store.connected,
      wsConnected: !!store.wsConnected,
      connectionState: clean(store.connectionState || (store.connected ? 'connected' : 'disconnected')),
      notifications: Array.isArray(store.notifications) ? store.notifications : [],
      unreadNotifications: Number(store.unreadNotifications || 0) || 0,
      raw: store,
      root: rootState
    };
  }

  function emit(reason) {
    version += 1;
    var state = snapshot();
    var active = listeners.slice();
    for (var i = 0; i < active.length; i += 1) {
      try { active[i](state); } catch (_) {}
    }
    if (typeof window !== 'undefined' && typeof window.dispatchEvent === 'function') {
      try {
        window.dispatchEvent(new CustomEvent('infring:shell-app-store-changed', {
          detail: { reason: clean(reason || 'changed'), state: state }
        }));
      } catch (_) {}
    }
    return state;
  }

  function subscribe(listener) {
    if (typeof listener !== 'function') return function() {};
    listeners.push(listener);
    try { listener(snapshot()); } catch (_) {}
    return function unsubscribe() {
      listeners = listeners.filter(function(row) { return row !== listener; });
    };
  }

  function registerSource(store, reason) {
    if (store && typeof store === 'object') {
      sourceStore = store;
      if (typeof window !== 'undefined') window.InfringApp = store;
      emit(reason || 'source_registered');
    }
    return sourceStore;
  }

  function registerShellRoot(rootState) {
    if (rootState && typeof rootState === 'object') {
      shellRoot = rootState;
      emit('root_registered');
    }
    return shellRoot;
  }

  function registerAlpineStore(runtime, name, definition) {
    var registry = runtime && typeof runtime.store === 'function' ? runtime : null;
    if (!registry) return registerSource(definition, 'fallback_source_registered');
    registry.store(name || 'app', definition || {});
    return registerSource(registry.store(name || 'app'), 'alpine_compat_registered');
  }

  function set(key, value) {
    var store = current();
    if (!store || !key) return store;
    store[key] = value;
    emit('set:' + key);
    return store;
  }

  function assign(values) {
    var store = current();
    if (!store || !values || typeof values !== 'object') return store;
    Object.assign(store, values);
    emit('assign');
    return store;
  }

  function method(name) {
    var store = current();
    var fn = store && store[name];
    return typeof fn === 'function' ? fn.bind(store) : null;
  }

  if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') {
    window.addEventListener('hashchange', function() { emit('hashchange'); }, { passive: true });
  }

  services.appStore = Object.assign({}, services.appStore || {}, {
    current: current,
    root: root,
    snapshot: snapshot,
    subscribe: subscribe,
    notify: emit,
    registerSource: registerSource,
    registerShellRoot: registerShellRoot,
    registerAlpineStore: registerAlpineStore,
    set: set,
    assign: assign,
    method: method
  });

  return services;
})(typeof InfringSharedShellServices === 'object' ? InfringSharedShellServices : null);

if (typeof window !== 'undefined') {
  window.InfringSharedShellServices = InfringSharedShellServices;
}
