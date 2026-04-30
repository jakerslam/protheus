const COMPONENT_TAG = 'infring-taskbar-dropdown-cluster-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-taskbar-dropdown-cluster-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-dropdowns';
  export let parentOwnedMechanics = true;

  let helpOpen = false;
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
    helpOpen = String(store.taskbarTextMenuOpen || '').trim().toLowerCase() === 'help';
  }

  function toggleHelp(event) {
    if (event) event.stopPropagation();
    call('toggleTaskbarTextMenu', 'help');
    syncFromStore();
  }

  function closeHelp() {
    call('closeTaskbarTextMenu');
    syncFromStore();
  }

  function openManual() {
    call('handleTaskbarHelpManual');
    syncFromStore();
  }

  function reportIssue() {
    call('handleTaskbarHelpReportIssue');
    syncFromStore();
  }

  function handleDocumentPointerDown(event) {
    if (!helpOpen || !anchorEl || (event && anchorEl.contains(event.target))) return;
    closeHelp();
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
  data-shell-primitive={shellPrimitive}
  data-wrapper-role={wrapperRole}
  data-parent-owned-mechanics={parentOwnedMechanics ? 'true' : 'false'}
>
  <infring-taskbar-menu-shell
    class="taskbar-text-menus"
    shellprimitive="taskbar-dock"
    wrapperrole="taskbar-menu"
    parentownedmechanics="true"
    menu="help"
    open={helpOpen ? 'true' : 'false'}
  >
    <div id="taskbar-help-menu-anchor" class="taskbar-text-menu-anchor" bind:this={anchorEl}>
      <button
        class="taskbar-text-menu-btn"
        type="button"
        on:click={toggleHelp}
        aria-expanded={helpOpen ? 'true' : 'false'}
        aria-haspopup="menu"
        aria-label="Help menu"
      >Help</button>
      {#if helpOpen}
        <infring-taskbar-menu-shell
          class="taskbar-text-menu-dropdown dashboard-dropdown-surface"
          shellprimitive="taskbar-dock"
          wrapperrole="taskbar-menu"
          parentownedmechanics="true"
          anchorid="taskbar-help-menu-anchor"
          fallbackside="bottom"
          layoutkey="taskbar-help-menu"
        >
          <button class="taskbar-text-menu-item" type="button" on:click={openManual}>Manual</button>
          <button class="taskbar-text-menu-item" type="button" on:click={reportIssue}>Report an issue</button>
        </infring-taskbar-menu-shell>
      {/if}
    </div>
  </infring-taskbar-menu-shell>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
