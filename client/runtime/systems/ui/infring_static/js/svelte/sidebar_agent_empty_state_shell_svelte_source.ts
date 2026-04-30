const COMPONENT_TAG = 'infring-sidebar-agent-empty-state-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-agent-empty-state-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let loading = true;
  let hasRows = false;
  let searchActive = false;
  let searchLoading = false;
  let unsubscribe = null;
  let pollTimer = 0;

  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.root === 'function') {
      var root = storeBridge.root();
      if (root) return root;
    }
    if (storeBridge && typeof storeBridge.current === 'function') return storeBridge.current();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function call(store, name) {
    if (!store || typeof store[name] !== 'function') return undefined;
    try { return store[name](); } catch (_) { return undefined; }
  }

  function rowCount(store) {
    var rows = store && Array.isArray(store.chatSidebarRows) ? store.chatSidebarRows : [];
    return rows.length;
  }

  function syncFromStore() {
    var store = appStore() || {};
    loading = !!(store.agentsLoading || !store.agentsHydrated);
    hasRows = rowCount(store) > 0;
    searchActive = !!call(store, 'isChatSidebarSearchActive');
    searchLoading = !!store.chatSidebarSearchLoading;
  }

  onMount(function() {
    syncFromStore();
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(syncFromStore);
    }
    pollTimer = window.setInterval(syncFromStore, 250);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (pollTimer) window.clearInterval(pollTimer);
  });
</script>

{#if loading}
  <div class="nav-item nav-sub-item nav-sub-item-empty">
    <span class="nav-icon nav-loading-icon" aria-hidden="true"></span>
    <span class="nav-label">Loading agents...</span>
  </div>
{:else if !hasRows && !searchActive}
  <div class="nav-item nav-sub-item nav-sub-item-empty">
    <span class="nav-icon" aria-hidden="true">&#8722;</span>
    <span class="nav-label">No agents</span>
  </div>
{:else if searchActive && !searchLoading && !hasRows}
  <div class="nav-item nav-sub-item nav-sub-item-empty">
    <span class="nav-icon" aria-hidden="true">&#8722;</span>
    <span class="nav-label">No matches</span>
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
