const COMPONENT_TAG = 'infring-sidebar-nav-link-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-nav-link-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  export let label = '';
  export let route = '';
  export let activeroutes = '';
  export let icon = '';
  export let popup = 'show';

  let page = '';
  let collapsed = false;
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

  function routeList() {
    return String(activeroutes || route || '')
      .split(',')
      .map(function(item) { return item.trim(); })
      .filter(Boolean);
  }

  function active() {
    return routeList().indexOf(page) >= 0;
  }

  function syncFromStore() {
    var store = appStore() || {};
    page = typeof store.page === 'string' ? store.page : '';
    collapsed = !!store.sidebarCollapsed;
  }

  function navigateTo(event) {
    if (event) event.preventDefault();
    if (route) call('navigate', route);
  }

  function showCollapsedPopup(event) {
    if (!collapsed) return;
    if (popup === 'hide') {
      call('hideDashboardPopupBySource', 'sidebar');
      return;
    }
    call('showCollapsedSidebarNavPopup', label, event);
  }

  function hideCollapsedPopup() {
    if (!collapsed) return;
    call('hideDashboardPopupBySource', 'sidebar');
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

<a
  class="nav-item sidebar-tab-item {icon === 'settings' ? 'sidebar-settings-item' : ''}"
  class:active={active()}
  href={route ? '#' : undefined}
  aria-current={active() ? 'page' : undefined}
  on:click={navigateTo}
  on:mouseenter={showCollapsedPopup}
  on:mousemove={showCollapsedPopup}
  on:mouseleave={hideCollapsedPopup}
>
  <span class="nav-icon">
    {#if icon === 'chat'}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg>
    {:else if icon === 'agents'}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
    {:else if icon === 'automation'}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
    {:else if icon === 'apps'}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <rect x="4" y="4" width="6" height="6" rx="1.5"></rect>
        <rect x="14" y="4" width="6" height="6" rx="1.5"></rect>
        <rect x="4" y="14" width="6" height="6" rx="1.5"></rect>
        <rect x="14" y="14" width="6" height="6" rx="1.5"></rect>
      </svg>
    {:else if icon === 'system'}
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/></svg>
    {:else if icon === 'settings'}
      <svg class="settings-cog-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06A1.65 1.65 0 0 0 15 19.4a1.65 1.65 0 0 0-1 .6 1.65 1.65 0 0 0-.33 1V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-.33-1 1.65 1.65 0 0 0-1-.6 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-.6-1 1.65 1.65 0 0 0-1-.33H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1-.33 1.65 1.65 0 0 0 .6-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06A2 2 0 1 1 7.13 3.6l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-.6 1.65 1.65 0 0 0 .33-1V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 .33 1 1.65 1.65 0 0 0 1 .6 1.65 1.65 0 0 0 1.82-.33l.06-.06A2 2 0 1 1 20.4 7.13l-.06.06A1.65 1.65 0 0 0 19.4 9c0 .38.13.74.36 1.03.23.29.57.5.94.57H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1 .4c-.29.24-.5.57-.57.94z"/></svg>
    {/if}
  </span>
  <span class="nav-label">{label}</span>
</a>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
