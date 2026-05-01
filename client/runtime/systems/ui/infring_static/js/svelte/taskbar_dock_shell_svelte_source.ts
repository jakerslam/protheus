const COMPONENT_TAG = 'infring-taskbar-dock-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-taskbar-dock-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount, tick } from 'svelte';

  let probe;
  let unsubscribe = null;
  let timer = 0;
  let pointerCleanup = null;

  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.current === 'function') {
      var current = storeBridge.current();
      if (current) return current;
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

  function hostElement() {
    if (!probe) return null;
    var root = typeof probe.getRootNode === 'function' ? probe.getRootNode() : null;
    var shadowHost = root && root.host ? root.host : null;
    var host = shadowHost || probe.parentElement;
    return host && String(host.tagName || '').toLowerCase() === 'infring-taskbar-dock-shell' ? host : null;
  }

  function syncHost() {
    var host = hostElement();
    if (!host) return;
    var store = appStore() || {};
    var edge = String(store.taskbarDockEdge || '').trim().toLowerCase();
    host.classList.toggle('is-dock-dragging', !!store.taskbarDockDragActive);
    host.classList.toggle('is-docked-bottom', edge === 'bottom');
    host.classList.toggle('is-docked-top', edge !== 'bottom');
    var styleText = String(call('taskbarContainerStyle') || '');
    if (styleText) host.setAttribute('style', styleText);
    else host.removeAttribute('style');
  }

  function startDockDrag(event) {
    call('startTaskbarDockPointerDrag', event);
    syncHost();
  }

  function bindHostEvents() {
    var host = hostElement();
    if (!host || pointerCleanup) return;
    host.addEventListener('pointerdown', startDockDrag, true);
    pointerCleanup = function() {
      host.removeEventListener('pointerdown', startDockDrag, true);
    };
  }

  onMount(function() {
    tick().then(function() {
      syncHost();
      bindHostEvents();
    });
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(syncHost);
    }
    timer = window.setInterval(syncHost, 200);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (timer) window.clearInterval(timer);
    if (typeof pointerCleanup === 'function') pointerCleanup();
    pointerCleanup = null;
  });
</script>
<span bind:this={probe} class="taskbar-dock-shell-probe" hidden aria-hidden="true"></span>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
