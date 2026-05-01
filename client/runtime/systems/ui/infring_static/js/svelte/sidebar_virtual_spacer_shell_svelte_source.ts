const COMPONENT_TAG = 'infring-sidebar-virtual-spacer-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-virtual-spacer-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  export let edge = 'top';

  let height = 0;
  let visible = false;
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

  function selectedHeight(store) {
    var key = String(edge || '').toLowerCase() === 'bottom'
      ? 'chatSidebarVirtualPadBottom'
      : 'chatSidebarVirtualPadTop';
    var next = Number(store && store[key]);
    return Number.isFinite(next) ? Math.max(0, next) : 0;
  }

  function syncFromStore() {
    var store = appStore() || {};
    height = selectedHeight(store);
    visible = !!store.chatSidebarVirtualized && height > 0;
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

{#if visible}
  <div
    class="nav-sub-virtual-spacer"
    style={'height:' + height + 'px'}
    aria-hidden="true"
  ></div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
