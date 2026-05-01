const COMPONENT_TAG = 'infring-sidebar-rail-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-sidebar-rail-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy, tick } from 'svelte';

  export let dragbarSurface = 'chat-sidebar';
  export let wall = '';
  export let dragging = false;
  export let parentOwnedMechanics = true;

  let probe;
  let unsub;
  let timer = 0;
  let navScrollCleanup = null;
  let pointerCleanup = [];

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
    return host && String(host.tagName || '').toLowerCase() === 'infring-sidebar-rail-shell' ? host : null;
  }

  function hostState() {
    var store = appStore() || {};
    return {
      collapsed: !!store.sidebarCollapsed,
      mobileOpen: !!store.mobileMenuOpen,
      chatPage: String(store.page || '') === 'chat',
      dragging: !!store.chatSidebarDragActive
    };
  }

  function syncHost() {
    var host = hostElement();
    if (!host) return;
    var state = hostState();
    host.classList.toggle('collapsed', state.collapsed);
    host.classList.toggle('mobile-open', state.mobileOpen);
    host.classList.toggle('chat-only-hidden', !state.chatPage);
    host.classList.toggle('chat-sidebar-dynamic', state.chatPage);
    host.classList.toggle('is-container-dragging', state.dragging);
    var styleText = String(call('chatSidebarContainerStyle') || '');
    if (styleText) host.setAttribute('style', styleText);
    else host.removeAttribute('style');

    var navShell = host.querySelector('.sidebar-nav-shell');
    if (navShell) {
      var navShellStyle = String(call('chatSidebarNavShellStyle') || '');
      if (navShellStyle) navShell.setAttribute('style', navShellStyle);
      else navShell.removeAttribute('style');
    }

    var nav = host.querySelector('.sidebar-nav');
    if (nav) {
      var navStyle = String(call('chatSidebarNavStyle') || '');
      if (navStyle) nav.setAttribute('style', navStyle);
      else nav.removeAttribute('style');
      var store = appStore();
      if (store && typeof store === 'object') {
        try {
          store.$refs = store.$refs || {};
          store.$refs.sidebarNav = nav;
        } catch (_) {}
      }
    }
  }

  function handleStartDrag(event) {
    call('startChatSidebarPointerDrag', event);
    syncHost();
  }

  function handleSidebarScroll() {
    call('scheduleSidebarScrollIndicators');
    call('hideDashboardPopupBySource', 'sidebar');
  }

  function bindHostEvents() {
    var host = hostElement();
    if (!host) return;
    if (!pointerCleanup.length) {
      host.addEventListener('pointerdown', handleStartDrag, true);
      host.addEventListener('mousedown', handleStartDrag, true);
      pointerCleanup = [
        function() { host.removeEventListener('pointerdown', handleStartDrag, true); },
        function() { host.removeEventListener('mousedown', handleStartDrag, true); }
      ];
    }
    if (!navScrollCleanup) {
      var nav = host.querySelector('.sidebar-nav');
      if (nav) {
        nav.addEventListener('scroll', handleSidebarScroll, { passive: true });
        navScrollCleanup = function() { nav.removeEventListener('scroll', handleSidebarScroll); };
      }
    }
  }

  function cleanupHostEvents() {
    for (var i = 0; i < pointerCleanup.length; i += 1) {
      if (typeof pointerCleanup[i] === 'function') pointerCleanup[i]();
    }
    pointerCleanup = [];
    if (typeof navScrollCleanup === 'function') navScrollCleanup();
    navScrollCleanup = null;
  }

  onMount(function() {
    tick().then(function() {
      syncHost();
      bindHostEvents();
    });
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsub = storeBridge.subscribe(function() {
        syncHost();
        bindHostEvents();
      });
    }
    timer = window.setInterval(function() {
      syncHost();
      bindHostEvents();
    }, 200);
  });

  onDestroy(function() {
    if (typeof unsub === 'function') unsub();
    if (timer) window.clearInterval(timer);
    cleanupHostEvents();
    var store = appStore();
    if (store && store.$refs && store.$refs.sidebarNav) {
      try { delete store.$refs.sidebarNav; } catch (_) { store.$refs.sidebarNav = null; }
    }
  });

  $: if (dragbarSurface || wall || dragging || parentOwnedMechanics) tick().then(syncHost);
</script>
<span bind:this={probe} class="sidebar-rail-shell-probe" hidden aria-hidden="true"></span>
<slot />
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
