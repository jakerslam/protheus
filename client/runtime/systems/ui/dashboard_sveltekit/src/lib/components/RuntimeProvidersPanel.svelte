<script lang="ts">
  import type { DashboardProviderRow } from '$lib/settings';

  export let providers: DashboardProviderRow[] = [];

  function badgeClass(provider: DashboardProviderRow): string {
    const status = String(provider.auth_status || '').toLowerCase();
    if (status.includes('configured') || status.includes('ready')) return 'badge success';
    if (provider.is_local) return 'badge info';
    return 'badge';
  }
</script>

<article class="panel">
  <div class="panel-head">
    <h3>Provider health</h3>
    <span class="meta">{providers.length} active</span>
  </div>

  {#if providers.length === 0}
    <div class="empty-card">No configured or reachable providers reported.</div>
  {:else}
    <div class="provider-list">
      {#each providers as provider}
        <div class="provider-card">
          <div class="provider-head">
            <div>
              <strong>{provider.display_name}</strong>
              <p>{provider.id}</p>
            </div>
            <span class={badgeClass(provider)}>{provider.auth_status || 'unknown'}</span>
          </div>
          <div class="provider-meta">
            <span>{provider.api_key_env || 'No env binding reported'}</span>
            {#if provider.is_local}
              <span>{provider.base_url || 'Local endpoint not set'}</span>
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
  .empty-card {
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
  .provider-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .provider-list {
    display: grid;
    gap: 14px;
  }
  .provider-card,
  .empty-card {
    border-radius: 20px;
    padding: 14px;
    display: grid;
    gap: 10px;
  }
  .provider-meta {
    display: grid;
    gap: 4px;
    color: #8aa4cf;
    font-size: 0.92rem;
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
  .badge.info {
    background: rgba(30, 56, 95, 0.58);
    border-color: rgba(96, 165, 250, 0.24);
  }
  .meta,
  p {
    color: #8aa4cf;
    margin: 0;
  }
  @media (max-width: 760px) {
    .provider-head {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
