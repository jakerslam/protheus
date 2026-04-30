const COMPONENT_TAG = 'infring-workspace-panel-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-workspace-panel-shell', shadow: 'none' }} />
<svelte:window on:keydown={handleWindowKeydown} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let uiTick = 0;
  let unsubscribe = null;
  let timer = 0;
  let hostElement = null;

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

  function call(owner, name) {
    var args = Array.prototype.slice.call(arguments, 2);
    if (owner && typeof owner[name] === 'function') {
      try { return owner[name].apply(owner, args); } catch (_) { return undefined; }
    }
    return undefined;
  }

  function callPageOrApp(name) {
    var args = Array.prototype.slice.call(arguments, 1);
    var page = chatPage();
    var result = call.apply(null, [page, name].concat(args));
    if (typeof result !== 'undefined') return result;
    var app = appStore();
    return call.apply(null, [app, name].concat(args));
  }

  function bump() {
    uiTick += 1;
  }

  function isOpen(_tick) {
    return !!callPageOrApp('isWorkspacePanelOpen');
  }

  function payload(_tick) {
    var value = callPageOrApp('workspacePanelPayload');
    if (!value || typeof value !== 'object') {
      return {
        id: '',
        actor: '',
        timestamp: '',
        preview: '',
        sources: [],
        trace: [],
        artifacts: [],
        rows_count: 0
      };
    }
    return value;
  }

  function subtitle(_tick) {
    var model = payload(uiTick);
    var actor = String(model.actor || '').trim();
    var timestamp = String(model.timestamp || '').trim();
    if (actor && timestamp) return actor + ' · ' + timestamp;
    return actor || timestamp || '';
  }

  function close() {
    callPageOrApp('closeWorkspacePanel');
    bump();
  }

  function handleWindowKeydown(event) {
    if (!event || event.key !== 'Escape') return;
    if (isOpen(uiTick)) close();
  }

  function handleDocumentPointerDown(event) {
    if (!isOpen(uiTick) || !hostElement || !event || !event.target) return;
    if (hostElement.contains(event.target)) return;
    close();
  }

  onMount(function() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(bump);
    }
    timer = window.setInterval(bump, 250);
    document.addEventListener('pointerdown', handleDocumentPointerDown, true);
    document.addEventListener('mousedown', handleDocumentPointerDown, true);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (timer) window.clearInterval(timer);
    document.removeEventListener('pointerdown', handleDocumentPointerDown, true);
    document.removeEventListener('mousedown', handleDocumentPointerDown, true);
  });
</script>

{#if isOpen(uiTick)}
  <infring-popup-window-shell class="chat-workspace-panel overlay-shared-surface" bind:this={hostElement}>
    <div class="chat-workspace-panel-head">
      <div>
        <div class="chat-workspace-panel-title">Workspace</div>
        <div class="chat-workspace-panel-subtitle">{subtitle(uiTick)}</div>
      </div>
      <button type="button" class="message-stat-btn" on:click={close} title="Close workspace">✕</button>
    </div>
    <div class="chat-workspace-panel-body">
      {#if payload(uiTick).preview}
        <div class="chat-workspace-panel-section">
          <div class="chat-workspace-panel-label">Summary</div>
          <div class="chat-workspace-panel-text">{payload(uiTick).preview}</div>
        </div>
      {/if}
    </div>
  </infring-popup-window-shell>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
