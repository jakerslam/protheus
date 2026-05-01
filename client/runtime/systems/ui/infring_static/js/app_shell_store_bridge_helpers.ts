function infringAppShellStoreBridge() {
  return infringShellAppStoreBridge();
}

function infringNotifyAppShellStore(page, reason) {
  var bridge = page.shellAppStoreBridge();
  if (bridge && typeof bridge.notify === 'function') bridge.notify(reason || 'shell_root_changed');
}

function infringGetAppStore(page) {
  var bridge = page.shellAppStoreBridge();
  if (bridge && typeof bridge.current === 'function') {
    var bridgedStore = bridge.current();
    if (bridgedStore && typeof bridgedStore === 'object') return bridgedStore;
  }
  return (typeof window !== 'undefined' && window.InfringApp && typeof window.InfringApp === 'object')
    ? window.InfringApp
    : null;
}

function infringReadAppStoreAgents(page) {
  var store = page.getAppStore();
  return store && Array.isArray(store.agents) ? store.agents : [];
}
