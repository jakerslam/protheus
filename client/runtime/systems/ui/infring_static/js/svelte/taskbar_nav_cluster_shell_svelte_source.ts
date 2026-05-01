const COMPONENT_TAG = 'infring-taskbar-nav-cluster-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-taskbar-nav-cluster-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  export let shellPrimitive = 'taskbar-dock';
  export let wrapperRole = 'taskbar-nav';
  export let parentOwnedMechanics = true;

  let canBack = false;
  let canForward = false;
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

  function syncFromStore() {
    canBack = !!call('canNavigateBack');
    canForward = !!call('canNavigateForward');
    styleText = String(call('taskbarReorderItemStyle', 'left', 'nav_cluster') || '');
  }

  function navigateBack() {
    call('navigateBackPage');
    syncFromStore();
  }

  function navigateForward() {
    call('navigateForwardPage');
    syncFromStore();
  }

  function showPopup(label, event) {
    call('showTaskbarNavPopup', label, event);
  }

  function hidePopup(label) {
    call('hideDashboardPopup', 'taskbar-nav:' + String(label || '').toLowerCase());
  }

  function reorderEvent(name, event) {
    call(name, 'left', event);
    syncFromStore();
  }

  function simpleEvent(name, event) {
    call(name, event);
    syncFromStore();
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

<div
  class="taskbar-reorder-box taskbar-reorder-box-left"
  on:pointerdown={(event) => reorderEvent('handleTaskbarReorderPointerDown', event)}
  on:pointerup={() => simpleEvent('cancelTaskbarDragHold')}
  on:pointercancel={() => simpleEvent('cancelTaskbarDragHold')}
  on:pointerleave={() => simpleEvent('cancelTaskbarDragHold')}
  on:dragstart={(event) => reorderEvent('handleTaskbarReorderDragStart', event)}
  on:drag={(event) => simpleEvent('handleTaskbarReorderDragMove', event)}
  on:dragenter={(event) => reorderEvent('handleTaskbarReorderDragEnter', event)}
  on:dragover={(event) => reorderEvent('handleTaskbarReorderDragOver', event)}
  on:drop={(event) => reorderEvent('handleTaskbarReorderDrop', event)}
  on:dragend={() => simpleEvent('handleTaskbarDragEnd')}
  data-shell-primitive={shellPrimitive}
  data-wrapper-role={wrapperRole}
  data-parent-owned-mechanics={parentOwnedMechanics ? 'true' : 'false'}
>
  <div
    class="taskbar-reorder-item taskbar-reorder-nav-cluster taskbar-nav-pill"
    data-taskbar-item="nav_cluster"
    style={styleText}
    draggable="true"
  >
    <button
      class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn taskbar-back-btn"
      class:is-disabled={!canBack}
      on:click={navigateBack}
      on:mouseenter={(event) => showPopup('Back', event)}
      on:mousemove={(event) => showPopup('Back', event)}
      on:mouseleave={() => hidePopup('Back')}
      on:focus={(event) => showPopup('Back', event)}
      on:blur={() => hidePopup('Back')}
      aria-disabled={canBack ? 'false' : 'true'}
      aria-label="Back"
    >
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m15 18-6-6 6-6"></path>
      </svg>
    </button>
    <button
      class="btn btn-ghost btn-sm taskbar-icon-btn taskbar-nav-btn taskbar-forward-btn"
      class:is-disabled={!canForward}
      on:click={navigateForward}
      on:mouseenter={(event) => showPopup('Forward', event)}
      on:mousemove={(event) => showPopup('Forward', event)}
      on:mouseleave={() => hidePopup('Forward')}
      on:focus={(event) => showPopup('Forward', event)}
      on:blur={() => hidePopup('Forward')}
      aria-disabled={canForward ? 'false' : 'true'}
      aria-label="Forward"
    >
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="m9 18 6-6-6-6"></path>
      </svg>
    </button>
  </div>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
