const COMPONENT_TAG = 'infring-taskbar-search-popup-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-taskbar-search-popup-shell', shadow: 'none' }} />
<svelte:window on:keydown={handleWindowKeydown} />
<script>
  import { onDestroy, onMount, tick } from 'svelte';

  let probe;
  let uiTick = 0;
  let unsubscribe = null;
  let timer = 0;
  let outsideCleanup = null;

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
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.method === 'function') {
      var method = storeBridge.method(name);
      if (method) {
        var methodArgs = Array.prototype.slice.call(arguments, 1);
        try { return method.apply(null, methodArgs); } catch (_) { return undefined; }
      }
    }
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
    return host && String(host.tagName || '').toLowerCase() === 'infring-taskbar-search-popup-shell' ? host : null;
  }

  function isOpen(_tick) {
    return !!((appStore() || {}).taskbarSearchOpen);
  }

  function queryValue(_tick) {
    return String((appStore() || {}).taskbarSearchQuery || '');
  }

  function syncHost() {
    var host = hostElement();
    if (!host) return;
    var open = isOpen(uiTick);
    host.hidden = !open;
    host.setAttribute('aria-hidden', open ? 'false' : 'true');
  }

  function bump() {
    uiTick += 1;
    syncHost();
  }

  function closeSearch() {
    call('closeTaskbarSearch');
    bump();
  }

  function updateQuery(event) {
    var value = event && event.target ? event.target.value : '';
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.set === 'function') storeBridge.set('taskbarSearchQuery', value);
    else {
      var store = appStore();
      if (store) store.taskbarSearchQuery = value;
    }
    bump();
  }

  function handleWindowKeydown(event) {
    if (!isOpen(uiTick) || !event || event.key !== 'Escape') return;
    closeSearch();
  }

  function bindOutsideClose() {
    if (outsideCleanup || typeof document === 'undefined') return;
    var listener = function(event) {
      var host = hostElement();
      if (!isOpen(uiTick) || !host || host.contains(event.target)) return;
      closeSearch();
    };
    document.addEventListener('pointerdown', listener, true);
    outsideCleanup = function() {
      document.removeEventListener('pointerdown', listener, true);
    };
  }

  onMount(function() {
    tick().then(function() {
      syncHost();
      bindOutsideClose();
    });
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(bump);
    }
    timer = window.setInterval(bump, 300);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (timer) window.clearInterval(timer);
    if (typeof outsideCleanup === 'function') outsideCleanup();
    outsideCleanup = null;
  });
</script>

<span bind:this={probe} class="taskbar-search-popup-shell-probe" hidden aria-hidden="true"></span>
<infring-popup-window-shell class={"taskbar-search-popup dashboard-popup-surface dashboard-popup-surface--interactive" + (isOpen(uiTick) ? " is-active" : "")}>
  <span class="taskbar-search-icon" aria-hidden="true">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.05" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="11" cy="11" r="6"></circle>
      <path d="m20 20-3.7-3.7"></path>
    </svg>
  </span>
  <input
    id="taskbar-search-input"
    type="text"
    class="taskbar-search-input"
    value={queryValue(uiTick)}
    placeholder="Search conversations..."
    autocomplete="off"
    spellcheck="false"
    aria-label="Search"
    on:input={updateQuery}
  />
</infring-popup-window-shell>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
