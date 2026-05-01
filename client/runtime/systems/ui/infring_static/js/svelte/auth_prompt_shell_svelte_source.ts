const COMPONENT_TAG = 'infring-auth-prompt-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-auth-prompt-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let visible = false;
  let authMode = 'apikey';
  let apiKeyInput = '';
  let loginUser = '';
  let loginPass = '';
  let unsubscribe = null;
  let pollTimer = 0;

  function bridge() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    return services && services.appStore ? services.appStore : null;
  }

  function appStore() {
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.current === 'function') return storeBridge.current();
    return typeof window !== 'undefined' && window.InfringApp ? window.InfringApp : null;
  }

  function syncFromStore() {
    var store = appStore();
    visible = !!(store && store.showAuthPrompt);
    authMode = store && store.authMode === 'session' ? 'session' : 'apikey';
  }

  async function invokeStoreMethod(name, ...args) {
    var storeBridge = bridge();
    var fn = storeBridge && typeof storeBridge.method === 'function' ? storeBridge.method(name) : null;
    var store = appStore();
    if (!fn && store && typeof store[name] === 'function') fn = store[name].bind(store);
    if (fn) await fn(...args);
    syncFromStore();
    if (storeBridge && typeof storeBridge.notify === 'function') storeBridge.notify('auth_prompt:' + name);
  }

  async function submitApiKey() {
    if (!apiKeyInput || !apiKeyInput.trim()) return;
    await invokeStoreMethod('submitApiKey', apiKeyInput);
    apiKeyInput = '';
  }

  async function submitSessionLogin() {
    await invokeStoreMethod('sessionLogin', loginUser, loginPass);
    loginPass = '';
  }

  onMount(function() {
    syncFromStore();
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(syncFromStore);
    }
    pollTimer = window.setInterval(syncFromStore, 750);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (pollTimer) window.clearInterval(pollTimer);
  });
</script>

{#if visible}
  <div style="position:fixed;inset:0;z-index:9999;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,0.6);backdrop-filter:blur(4px)">
    <div style="background:var(--bg-card,#1e1e2e);border:1px solid var(--border,#333);border-radius:12px;padding:2rem;max-width:400px;width:90%">
      {#if authMode === 'session'}
        <div>
          <h3 style="margin:0 0 0.5rem;font-size:1.1rem">Sign In</h3>
          <p style="color:var(--text-dim,#888);font-size:0.85rem;margin:0 0 1rem">Enter your Shell credentials.</p>
          <input type="text" bind:value={loginUser} placeholder="Username" autocomplete="username" style="width:100%;padding:0.6rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--bg-input,#151520);color:var(--text,#e0e0e0);font-size:0.9rem;box-sizing:border-box;margin-bottom:0.5rem">
          <input type="password" bind:value={loginPass} placeholder="Password" autocomplete="current-password" on:keydown={(event) => { if (event.key === 'Enter') submitSessionLogin(); }} style="width:100%;padding:0.6rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--bg-input,#151520);color:var(--text,#e0e0e0);font-size:0.9rem;box-sizing:border-box;margin-bottom:0.75rem">
          <button on:click={submitSessionLogin} style="width:100%;padding:0.6rem;border-radius:6px;border:none;background:var(--accent,#7c3aed);color:#fff;font-weight:600;cursor:pointer;font-size:0.9rem">Sign In</button>
        </div>
      {:else}
        <div>
          <h3 style="margin:0 0 0.5rem;font-size:1.1rem">API Key Required</h3>
          <p style="color:var(--text-dim,#888);font-size:0.85rem;margin:0 0 0.5rem">This instance requires an API key. Enter the key from your <code>config.toml</code>.</p>
          <p style="color:var(--text-dim,#666);font-size:0.75rem;margin:0 0 1rem">Add <code style="color:var(--accent-light,#a78bfa);background:var(--bg,#111);padding:1px 4px;border-radius:2px">api_key = &quot;your-key&quot;</code> at the <strong>top</strong> of <code>~/.infring/config.toml</code> (not under any [section]).</p>
          <input type="password" bind:value={apiKeyInput} placeholder="Enter API key..." on:keydown={(event) => { if (event.key === 'Enter') submitApiKey(); }} style="width:100%;padding:0.6rem;border-radius:6px;border:1px solid var(--border,#333);background:var(--bg-input,#151520);color:var(--text,#e0e0e0);font-size:0.9rem;box-sizing:border-box;margin-bottom:0.75rem">
          <button on:click={submitApiKey} style="width:100%;padding:0.6rem;border-radius:6px;border:none;background:var(--accent,#7c3aed);color:#fff;font-weight:600;cursor:pointer;font-size:0.9rem">Unlock Shell</button>
        </div>
      {/if}
    </div>
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
