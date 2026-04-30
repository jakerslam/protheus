const COMPONENT_TAG = 'infring-chat-empty-state-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-chat-empty-state-shell', shadow: 'none' }} />
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

  function currentAgent(_tick) {
    var page = chatPage();
    if (page && page.currentAgent) return page.currentAgent;
    var app = appStore();
    return app && app.currentAgent ? app.currentAgent : null;
  }

  function messageCount(_tick) {
    var page = chatPage();
    var rows = page && Array.isArray(page.messages) ? page.messages : [];
    return rows.length;
  }

  function agentsLoading(_tick) {
    var app = appStore() || {};
    return !!app.agentsLoading;
  }

  function agentsHydrated(_tick) {
    var app = appStore() || {};
    return !!app.agentsHydrated;
  }

  function showFreshInit(_tick) {
    var page = chatPage();
    return !!(page && page.showFreshArchetypeTiles);
  }

  function sessionLoading(_tick) {
    var page = chatPage();
    return !!(page && page.sessionLoading);
  }

  function showNoAgent(_tick) {
    return !currentAgent(uiTick) && !(agentsLoading(uiTick) || !agentsHydrated(uiTick));
  }

  function showNoMessages(_tick) {
    return !!currentAgent(uiTick) && !sessionLoading(uiTick) && messageCount(uiTick) === 0 && !showFreshInit(uiTick);
  }

  function openAgents() {
    callPageOrApp('navigate', 'agents');
    bump();
  }

  function initializeAgent() {
    var agent = currentAgent(uiTick);
    if (!agent) return;
    callPageOrApp('ensureFreshInitThread', agent);
    bump();
  }

  function startChat() {
    callPageOrApp('focusChatComposerFromInit', '');
    bump();
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

{#if showNoAgent(uiTick)}
  <infring-chat-stream-shell class="empty-state">
    <h4>No agent selected</h4>
    <p class="hint">Create or select an agent to start chatting.</p>
    <button class="btn btn-primary btn-sm" on:click={openAgents}>Open agents</button>
  </infring-chat-stream-shell>
{:else if showNoMessages(uiTick)}
  <infring-chat-stream-shell class="empty-state">
    <h4>No messages yet</h4>
    <p class="hint">Start chatting or initialize this agent.</p>
    <div style="display:flex;align-items:center;justify-content:center;gap:8px">
      <button class="btn btn-ghost btn-sm" on:click={initializeAgent}>Initialize agent</button>
      <button class="btn btn-primary btn-sm" on:click={startChat}>Start chat</button>
    </div>
  </infring-chat-stream-shell>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
