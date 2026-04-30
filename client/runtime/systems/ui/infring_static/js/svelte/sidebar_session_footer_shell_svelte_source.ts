const COMPONENT_TAG = 'infring-sidebar-session-footer-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-session-footer-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let sessionUser = '';
  let unsubscribe = null;
  let pollTimer = 0;

  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.current === 'function') {
      var source = storeBridge.current();
      if (source) return source;
    }
    if (storeBridge && typeof storeBridge.root === 'function') return storeBridge.root();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function syncFromStore() {
    var store = appStore() || {};
    sessionUser = String(store.sessionUser || '').trim();
  }

  function logout() {
    var store = appStore();
    if (!store || typeof store.sessionLogout !== 'function') return;
    try { store.sessionLogout(); } catch (_) {}
    sessionUser = '';
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

{#if sessionUser}
  <div class="sidebar-footer">
    <div style="padding:4px 16px;display:flex;align-items:center;justify-content:space-between">
      <span class="text-xs text-dim" style="letter-spacing:0.5px">{sessionUser}</span>
      <button
        type="button"
        on:click={logout}
        class="btn btn-ghost btn-sm"
        style="font-size:11px;padding:2px 8px;opacity:0.7"
        title="Sign out"
      >Logout</button>
    </div>
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
