const COMPONENT_TAG = 'infring-taskbar-hero-menu-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-taskbar-hero-menu-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-hero';
  export let parentOwnedMechanics = true;

  let open = false;
  let pending = false;
  let version = '0.0.0';
  let refreshTurns = 0;
  let anchorEl = null;
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

  function call(name) {
    var store = appStore();
    if (!store || typeof store[name] !== 'function') return undefined;
    var args = Array.prototype.slice.call(arguments, 1);
    try { return store[name].apply(store, args); } catch (_) { return undefined; }
  }

  function syncFromStore() {
    var store = appStore() || {};
    open = !!store.taskbarHeroMenuOpen;
    pending = !!store.taskbarHeroActionPending;
    version = String(store.version || '0.0.0');
    refreshTurns = Number(store.taskbarRefreshTurns || 0);
    if (!Number.isFinite(refreshTurns)) refreshTurns = 0;
  }

  function toggle(event) {
    if (event) event.stopPropagation();
    call('toggleTaskbarHeroMenu');
    syncFromStore();
  }

  function close() {
    call('closeTaskbarHeroMenu');
    syncFromStore();
  }

  async function runCommand(action) {
    if (pending) return;
    await call('runTaskbarHeroCommand', action);
    syncFromStore();
  }

  function handleDocumentPointerDown(event) {
    if (!open || !anchorEl || (event && anchorEl.contains(event.target))) return;
    close();
  }

  onMount(function() {
    syncFromStore();
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(syncFromStore);
    }
    pollTimer = window.setInterval(syncFromStore, 250);
    document.addEventListener('pointerdown', handleDocumentPointerDown, true);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (pollTimer) window.clearInterval(pollTimer);
    document.removeEventListener('pointerdown', handleDocumentPointerDown, true);
  });
</script>

<div
  id="taskbar-hero-menu-anchor"
  class="taskbar-hero-menu-anchor"
  bind:this={anchorEl}
  data-shell-primitive={shellPrimitive}
  data-wrapper-role={wrapperRole}
  data-parent-owned-mechanics={parentOwnedMechanics ? 'true' : 'false'}
>
  <button
    class="taskbar-brand taskbar-brand-trigger"
    type="button"
    on:click={toggle}
    aria-expanded={open ? 'true' : 'false'}
    aria-haspopup="menu"
    title="System actions"
  >
    <div class="brand-mark infring-logo" aria-hidden="true"><span class="brand-mark-glyph infring-logo-glyph">&infin;</span></div>
    <div><div class="taskbar-brand-title">INFRING</div></div>
  </button>
  {#if open}
    <infring-taskbar-menu-shell
      class="taskbar-hero-menu dashboard-dropdown-surface"
      shellprimitive="taskbar-dock"
      wrapperrole="taskbar-menu"
      parentownedmechanics="true"
      anchorid="taskbar-hero-menu-anchor"
      fallbackside="bottom"
      layoutkey="taskbar-hero-menu"
    >
      <button class="taskbar-hero-menu-item" type="button" on:click={() => runCommand('restart')} disabled={pending}>
        <svg class="taskbar-refresh-icon taskbar-hero-menu-icon" style={'transform: rotate(' + (refreshTurns * 360) + 'deg)'} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"></path><path d="M21 3v5h-5"></path></svg>
        <span>Restart</span>
      </button>
      <button class="taskbar-hero-menu-item" type="button" on:click={() => runCommand('update')} disabled={pending}>
        <svg class="taskbar-hero-menu-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.05" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="8"></circle><path d="M12 8v8"></path><path d="m8.5 12.5 3.5 3.5 3.5-3.5"></path></svg>
        <span>Update</span>
      </button>
      <button class="taskbar-hero-menu-item" type="button" on:click={() => runCommand('shutdown')} disabled={pending}>
        <svg class="taskbar-hero-menu-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.05" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 2v8"></path><path d="M8.2 5.8A8 8 0 1 0 15.8 5.8"></path></svg>
        <span>Shut down</span>
      </button>
      <div class="taskbar-hero-menu-version">v{version}</div>
    </infring-taskbar-menu-shell>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
