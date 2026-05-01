const COMPONENT_TAG = 'infring-chat-loading-overlay-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-loading-overlay-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let uiTick = 0;
  let unsubscribe = null;
  let timer = 0;

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

  function chatPage() {
    return typeof window !== 'undefined' && window.InfringChatPage ? window.InfringChatPage : null;
  }

  function bump() {
    uiTick += 1;
  }

  function showOverlay(_tick) {
    var page = chatPage();
    return !!(page && page.showFreshArchetypeTiles && page.freshInitLaunching);
  }

  onMount(function() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(bump);
    }
    timer = window.setInterval(bump, 250);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (timer) window.clearInterval(timer);
  });
</script>

{#if showOverlay(uiTick)}
  <div class="chat-loading-overlay">
    <infring-chat-loading-content-shell>
      <div class="chat-loading-overlay-content">
        <div class="chat-loading-fairy" aria-hidden="true">
          <span class="chat-loading-fairy-avatar agent-working-pulse"><span class="agent-mark infring-logo infring-logo--agent-default"><span class="infring-logo-glyph" aria-hidden="true">&infin;</span></span></span>
        </div>
        <span>Launching agent...</span>
      </div>
    </infring-chat-loading-content-shell>
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
