const COMPONENT_TAG = 'infring-sidebar-pulltab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-pulltab-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let page = '';
  let collapsed = false;
  let dragging = false;
  let styleText = '';
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

  function popupTitle() {
    return collapsed ? 'Expand sidebar' : 'Collapse sidebar';
  }

  function popupId() {
    return 'sidebar-utility:' + popupTitle().toLowerCase().replace(/[^a-z0-9_-]+/g, '-');
  }

  function popupBody() {
    return collapsed ? 'Open the chat navigation rail' : 'Hide the chat navigation rail';
  }

  function syncFromStore() {
    var store = appStore() || {};
    page = typeof store.page === 'string' ? store.page : '';
    collapsed = !!store.sidebarCollapsed;
    dragging = !!store.chatSidebarDragActive;
    styleText = typeof store.chatSidebarPulltabStyle === 'function'
      ? String(call('chatSidebarPulltabStyle') || '')
      : '';
  }

  function toggle() {
    call('toggleSidebar');
    syncFromStore();
  }

  function startDrag(event) {
    call('startChatSidebarPointerDrag', event);
    syncFromStore();
  }

  function showPopup(event) {
    call('showDashboardPopup', popupId(), popupTitle(), event, {
      source: 'sidebar',
      side: 'left',
      body: popupBody(),
      meta_origin: 'Sidebar'
    });
  }

  function hidePopup() {
    call('hideDashboardPopup', popupId());
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

{#if page === 'chat'}
  <button
    class="overlay-pulltab-object sidebar-pulltab drag-bar drag-bar-pulltab overlay-shared-surface pulltab-border-top-active pulltab-border-right-active pulltab-border-bottom-active pulltab-border-left-inactive"
    class:is-container-dragging={dragging}
    data-dragbar-pulltab="chat-sidebar"
    style={styleText}
    on:click={toggle}
    on:pointerdown|capture={startDrag}
    on:mousedown|capture={startDrag}
    on:mouseenter={showPopup}
    on:mousemove={showPopup}
    on:mouseleave={hidePopup}
    aria-label="Toggle sidebar"
  >
    <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-top-left" aria-hidden="true"></span>
    <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-top-right" aria-hidden="true"></span>
    <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-bottom-left" aria-hidden="true"></span>
    <span class="overlay-pulltab-object-joint sidebar-pulltab-joint sidebar-pulltab-joint-bottom-right" aria-hidden="true"></span>
    <svg class="overlay-pulltab-object-icon sidebar-pulltab-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      {#if collapsed}
        <path d="m9 6 6 6-6 6"></path>
      {:else}
        <path d="m15 6-6 6 6 6"></path>
      {/if}
    </svg>
  </button>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
