const COMPONENT_TAG = 'infring-sidebar-sort-toggle-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-sort-toggle-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let mode = 'age';
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

  function call(name) {
    var store = appStore();
    if (!store || typeof store[name] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return store[name].apply(store, args); } catch (_) { return undefined; }
  }

  function syncFromStore() {
    var store = appStore() || {};
    mode = store.chatSidebarSortMode === 'topology' ? 'topology' : 'age';
  }

  function setMode(nextMode, event) {
    if (event) event.stopPropagation();
    call('setChatSidebarSortMode', nextMode);
    mode = nextMode === 'topology' ? 'topology' : 'age';
  }

  function popupBody(nextMode) {
    return nextMode === 'topology' ? 'Sort by topology' : 'Sort by recent activity';
  }

  function showSortPopup(nextMode, event) {
    call('showDashboardPopup', 'sidebar-utility:sort-conversations', 'Sort conversations', event, {
      source: 'sidebar',
      side: 'right',
      body: popupBody(nextMode),
      meta_origin: 'Sidebar'
    });
  }

  function hideSortPopup() {
    call('hideDashboardPopup', 'sidebar-utility:sort-conversations');
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

<div
  class="nav-sub-sort-group nav-sub-sort-pill toggle-pill"
  role="group"
  aria-label="Sort conversations"
  data-mode={mode === 'topology' ? 'topology' : 'age'}
>
  <button
    type="button"
    class="nav-sub-sort-btn"
    class:active={mode === 'age'}
    on:click={(event) => setMode('age', event)}
    on:mouseenter={(event) => showSortPopup('age', event)}
    on:mousemove={(event) => showSortPopup('age', event)}
    on:mouseleave={hideSortPopup}
    on:focus={(event) => showSortPopup('age', event)}
    on:blur={hideSortPopup}
    aria-label="Sort by recent activity"
  >
    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"></circle><path d="M12 7v6l4 2"></path></svg>
  </button>
  <button
    type="button"
    class="nav-sub-sort-btn"
    class:active={mode === 'topology'}
    on:click={(event) => setMode('topology', event)}
    on:mouseenter={(event) => showSortPopup('topology', event)}
    on:mousemove={(event) => showSortPopup('topology', event)}
    on:mouseleave={hideSortPopup}
    on:focus={(event) => showSortPopup('topology', event)}
    on:blur={hideSortPopup}
    aria-label="Sort by topology"
  >
    {#if mode === 'topology'}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 7h10"></path><path d="M5 12h14"></path><path d="M5 17h8"></path><path d="m16 5 3 2-3 2"></path><path d="m20 10 3 2-3 2"></path><path d="m14 15 3 2-3 2"></path></svg>
    {:else}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M21 15a2 2 0 0 1-2 2H8l-4 4V5a2 2 0 0 1 2-2h13a2 2 0 0 1 2 2z"></path><circle cx="10" cy="11" r="1"></circle><circle cx="14" cy="11" r="1"></circle><circle cx="18" cy="11" r="1"></circle></svg>
    {/if}
  </button>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
