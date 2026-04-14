<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DashboardProviderRow } from '$lib/settings';

  export let providers: DashboardProviderRow[] = [];
  export let keyInputs: Record<string, string> = {};
  export let urlInputs: Record<string, string> = {};
  export let busyKey = '';
  export let testResults: Record<string, { status: string; latency_ms: number; error: string }> = {};

  const dispatch = createEventDispatcher<{
    savekey: { providerId: string };
    removekey: { providerId: string };
    testprovider: { providerId: string };
    saveurl: { providerId: string };
  }>();

  function providerBadge(provider: DashboardProviderRow): string {
    const status = String(provider.auth_status || '').toLowerCase();
    if (status.includes('configured') || status.includes('ready')) return 'badge success';
    if (status.includes('missing') || status.includes('needs')) return 'badge warn';
    return 'badge';
  }
</script>

<article class="panel">
  <div class="panel-head">
    <h3>Providers</h3>
    <span class="meta">{providers.length} detected</span>
  </div>

  {#if providers.length === 0}
    <div class="empty-card">No providers reported yet.</div>
  {:else}
    <div class="provider-list">
      {#each providers as provider}
        <div class="provider-card">
          <div class="provider-head">
            <div>
              <strong>{provider.display_name}</strong>
              <p>{provider.id}</p>
            </div>
            <span class={providerBadge(provider)}>{provider.auth_status || 'unknown'}</span>
          </div>

          <div class="field-row">
            <input bind:value={keyInputs[provider.id]} class="field" type="password" placeholder={`Enter ${provider.api_key_env || 'API key'}`} />
            <button class="primary small" type="button" disabled={busyKey === `key:${provider.id}` || !String(keyInputs[provider.id] || '').trim()} on:click={() => dispatch('savekey', { providerId: provider.id })}>
              {busyKey === `key:${provider.id}` ? 'Saving…' : 'Save key'}
            </button>
            <button class="ghost small" type="button" disabled={busyKey === `remove-key:${provider.id}`} on:click={() => dispatch('removekey', { providerId: provider.id })}>
              {busyKey === `remove-key:${provider.id}` ? 'Removing…' : 'Remove'}
            </button>
          </div>

          {#if provider.is_local}
            <div class="field-row">
              <input bind:value={urlInputs[provider.id]} class="field" type="text" placeholder="http://localhost:..." />
              <button class="ghost small" type="button" disabled={busyKey === `url:${provider.id}` || !String(urlInputs[provider.id] || '').trim()} on:click={() => dispatch('saveurl', { providerId: provider.id })}>
                {busyKey === `url:${provider.id}` ? 'Saving…' : 'Save URL'}
              </button>
            </div>
          {/if}

          <div class="provider-actions">
            <button class="ghost small" type="button" disabled={busyKey === `test:${provider.id}`} on:click={() => dispatch('testprovider', { providerId: provider.id })}>
              {busyKey === `test:${provider.id}` ? 'Testing…' : 'Test provider'}
            </button>
            {#if testResults[provider.id]}
              <span class="meta">
                {#if testResults[provider.id].status === 'ok'}
                  ok {testResults[provider.id].latency_ms || 0}ms
                {:else}
                  {testResults[provider.id].error || testResults[provider.id].status}
                {/if}
              </span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</article>

<style>
  .panel,
  .provider-card,
  .field,
  .ghost,
  .primary {
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
  }
  .panel {
    border-radius: 24px;
    padding: 20px;
    display: grid;
    gap: 18px;
  }
  .panel-head,
  .provider-head,
  .provider-actions,
  .field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .provider-list {
    display: grid;
    gap: 14px;
  }
  .provider-card {
    border-radius: 20px;
    padding: 14px;
    display: grid;
    gap: 12px;
  }
  .field {
    border-radius: 14px;
    padding: 0.75rem 0.85rem;
    font: inherit;
    min-width: 0;
    flex: 1;
  }
  .ghost,
  .primary {
    border-radius: 16px;
    padding: 0.8rem 1rem;
  }
  .small {
    padding: 0.55rem 0.8rem;
  }
  .badge {
    border-radius: 999px;
    padding: 0.35rem 0.7rem;
    border: 1px solid rgba(158, 188, 255, 0.18);
  }
  .badge.success {
    background: rgba(23, 68, 45, 0.58);
    border-color: rgba(105, 165, 126, 0.24);
  }
  .badge.warn {
    background: rgba(117, 77, 12, 0.45);
    border-color: rgba(232, 184, 79, 0.22);
  }
  .meta,
  p {
    color: #8aa4cf;
    margin: 0;
  }
  @media (max-width: 760px) {
    .provider-head,
    .provider-actions,
    .field-row {
      flex-direction: column;
      align-items: flex-start;
    }
    .field {
      width: 100%;
    }
  }
</style>
