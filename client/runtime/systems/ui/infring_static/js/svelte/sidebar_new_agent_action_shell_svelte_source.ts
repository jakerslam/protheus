const COMPONENT_TAG = 'infring-sidebar-new-agent-action-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-new-agent-action-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  export let variant = 'expanded';

  let collapsed = false;
  let spawning = false;
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
    collapsed = !!store.sidebarCollapsed;
    spawning = !!store.sidebarSpawningAgent;
  }

  function createAgent(event) {
    if (event) event.preventDefault();
    if (!spawning) call('createSidebarAgentChat');
  }

  function popupBody() {
    return spawning ? 'Agent creation already in progress' : 'Create a new agent conversation';
  }

  function showCollapsedPopup(event) {
    call('showDashboardPopup', 'sidebar-utility:new-agent', 'New agent', event, {
      source: 'sidebar',
      side: 'right',
      body: popupBody(),
      meta_origin: 'Sidebar'
    });
  }

  function hideCollapsedPopup() {
    call('hideDashboardPopup', 'sidebar-utility:new-agent');
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

{#if variant === 'collapsed'}
  {#if collapsed}
    <a
      class="nav-item nav-sub-item nav-sub-item-action nav-sub-item-action-collapsed nav-sub-item-action-new-agent"
      class:is-loading={spawning}
      aria-disabled={spawning ? 'true' : 'false'}
      on:click={createAgent}
      on:mouseenter={showCollapsedPopup}
      on:mousemove={showCollapsedPopup}
      on:mouseleave={hideCollapsedPopup}
      aria-label="New agent"
    >
      <span class="nav-icon new-agent-plus-icon-wrap">
        <svg viewBox="0 0 24 24" aria-hidden="true" style={spawning ? 'opacity:0.45' : ''}><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.1 2.1 0 0 1 3 3L12 15l-4 1 1-4z"/></svg>
        {#if spawning}<span class="spinner-sm new-agent-plus-spinner" aria-hidden="true"></span>{/if}
      </span>
    </a>
  {/if}
{:else}
  <a
    class="nav-item nav-sub-item nav-sub-item-action nav-sub-item-action-right nav-sub-item-action-new-agent"
    class:is-loading={spawning}
    aria-disabled={spawning ? 'true' : 'false'}
    on:click={createAgent}
  >
    <span class="nav-icon new-agent-plus-icon-wrap">
      <svg viewBox="0 0 24 24" aria-hidden="true" style={spawning ? 'opacity:0.45' : ''}><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.1 2.1 0 0 1 3 3L12 15l-4 1 1-4z"/></svg>
      {#if spawning}<span class="spinner-sm new-agent-plus-spinner" aria-hidden="true"></span>{/if}
    </span>
    <span class="nav-label">{spawning ? 'Creating...' : 'New agent'}</span>
  </a>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
