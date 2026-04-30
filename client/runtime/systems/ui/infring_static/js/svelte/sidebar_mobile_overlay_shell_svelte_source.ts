const COMPONENT_TAG = 'infring-sidebar-mobile-overlay-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-mobile-overlay-shell', shadow: 'none' }} />
<script>
  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.current === 'function') return storeBridge.current();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function closeMobileMenu() {
    var storeBridge = bridge();
    var store = appStore();
    if (storeBridge && typeof storeBridge.set === 'function') {
      storeBridge.set('mobileMenuOpen', false);
    } else if (store) {
      store.mobileMenuOpen = false;
    }
  }
</script>

<button
  class="sidebar-overlay-hitbox"
  type="button"
  aria-label="Close mobile sidebar"
  on:click={closeMobileMenu}
  style="position:absolute;inset:0;width:100%;height:100%;border:0;background:transparent;padding:0;margin:0;cursor:pointer"
></button>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
