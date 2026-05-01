const COMPONENT_TAG = 'infring-mobile-menu-button-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-mobile-menu-button-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let open = false;
  let unsubscribe = null;

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
    open = !!(store && store.mobileMenuOpen);
  }

  function toggleMobileMenu() {
    var storeBridge = bridge();
    var store = appStore();
    var nextOpen = !(store && store.mobileMenuOpen);
    if (storeBridge && typeof storeBridge.set === 'function') {
      storeBridge.set('mobileMenuOpen', nextOpen);
    } else if (store) {
      store.mobileMenuOpen = nextOpen;
    }
    open = nextOpen;
  }

  onMount(function() {
    syncFromStore();
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(syncFromStore);
    }
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
  });
</script>

<button
  class="mobile-menu-btn btn btn-ghost"
  type="button"
  aria-label="Toggle mobile menu"
  aria-expanded={open ? 'true' : 'false'}
  on:click={toggleMobileMenu}
  style="position:fixed;top:54px;left:8px;z-index:98;padding:6px 10px"
>
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg>
</button>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
