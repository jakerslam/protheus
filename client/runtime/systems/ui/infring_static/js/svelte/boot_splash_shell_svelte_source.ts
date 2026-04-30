const COMPONENT_TAG = 'infring-boot-splash-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-boot-splash-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';
  import { fade } from 'svelte/transition';

  let visible = true;
  let progressPercent = 6;
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

  function clampProgress(value) {
    var numeric = Number(value || 0);
    if (!Number.isFinite(numeric)) return 0;
    return Math.max(0, Math.min(100, numeric));
  }

  function syncFromStore() {
    var store = appStore();
    visible = store ? !!store.bootSplashVisible : true;
    progressPercent = clampProgress(store ? store.bootProgressPercent : progressPercent);
  }

  onMount(function() {
    syncFromStore();
    var storeBridge = bridge();
    if (storeBridge && typeof storeBridge.subscribe === 'function') {
      unsubscribe = storeBridge.subscribe(syncFromStore);
    }
    pollTimer = window.setInterval(syncFromStore, 120);
  });

  onDestroy(function() {
    if (typeof unsubscribe === 'function') unsubscribe();
    if (pollTimer) window.clearInterval(pollTimer);
  });
</script>

{#if visible}
  <div
    class="boot-splash"
    transition:fade={{ duration: 220 }}
    aria-hidden="true"
  >
    <div class="boot-splash-inner">
      <div class="brand-mark boot-splash-mark infring-logo"><span class="brand-mark-glyph infring-logo-glyph">&infin;</span></div>
      <div class="boot-splash-wordmark">INFRING</div>
      <div
        class="boot-splash-progress"
        role="progressbar"
        aria-label="Loading progress"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(progressPercent)}
      >
        <span
          class="boot-splash-progress-fill"
          style={'width:' + progressPercent + '%'}
        ></span>
      </div>
    </div>
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
