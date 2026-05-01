const COMPONENT_TAG = 'infring-sidebar-scroll-controls-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-scroll-controls-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let collapsed = false;
  let hasOverflowAbove = false;
  let hasOverflowBelow = false;
  let unsubscribe = null;
  let pollTimer = 0;

  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.current === 'function') return storeBridge.current();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function syncFromStore() {
    var store = appStore();
    collapsed = !!(store && store.sidebarCollapsed);
    hasOverflowAbove = !!(store && store.sidebarHasOverflowAbove);
    hasOverflowBelow = !!(store && store.sidebarHasOverflowBelow);
  }

  function scheduleIndicators() {
    var store = appStore();
    if (store && typeof store.scheduleSidebarScrollIndicators === 'function') {
      try { store.scheduleSidebarScrollIndicators(); } catch (_) {}
    }
  }

  function sidebarNav() {
    if (typeof document === 'undefined') return null;
    return document.querySelector('.sidebar-nav');
  }

  function scrollToEdge(edge) {
    var nav = sidebarNav();
    if (!nav || typeof nav.scrollTo !== 'function') return;
    var top = edge === 'top' ? 0 : Math.max(0, Number(nav.scrollHeight || 0));
    nav.scrollTo({ top: top, behavior: 'smooth' });
    scheduleIndicators();
    if (typeof window !== 'undefined') {
      window.setTimeout(scheduleIndicators, 220);
    }
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

{#if collapsed && hasOverflowAbove}
  <button
    type="button"
    class="sidebar-scroll-indicator sidebar-scroll-indicator-top"
    on:click={() => scrollToEdge('top')}
    aria-label="Scroll sidebar to top"
  >
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m18 15-6-6-6 6"></path></svg>
  </button>
{/if}

{#if collapsed && hasOverflowBelow}
  <button
    type="button"
    class="sidebar-scroll-indicator sidebar-scroll-indicator-bottom"
    on:click={() => scrollToEdge('bottom')}
    aria-label="Scroll sidebar to bottom"
  >
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"></path></svg>
  </button>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
