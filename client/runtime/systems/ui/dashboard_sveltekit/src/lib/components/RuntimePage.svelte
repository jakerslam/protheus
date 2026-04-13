<script lang="ts">
  import RuntimeOverviewPanel from '$lib/components/RuntimeOverviewPanel.svelte';
  import RuntimeProvidersPanel from '$lib/components/RuntimeProvidersPanel.svelte';
  import RuntimeWebToolingPanel from '$lib/components/RuntimeWebToolingPanel.svelte';
  import { readRuntimePageData, type RuntimePageData } from '$lib/runtime';
  import { onMount } from 'svelte';

  let data: RuntimePageData | null = null;
  let loading = true;
  let error = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      data = await readRuntimePageData();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'runtime_unavailable');
    } finally {
      loading = false;
    }
  }

  function pillState(value: boolean | null | undefined): string {
    if (value === true) return 'enforced';
    if (value === false) return 'missing';
    return 'unknown';
  }
</script>

<section class="runtime-page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native runtime</p>
      <h2>Backend health, provider status, and recent web tooling receipts in the Svelte shell.</h2>
      <p class="hero-copy">
        This native runtime slice covers the day-to-day operational view. Deeper legacy-only runtime tabs can still be reached through the classic escape hatch while we keep migrating.
      </p>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>
        {loading ? 'Refreshing…' : 'Refresh'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {/if}

  <div class="content-grid">
    <div class="column">
      <RuntimeOverviewPanel overview={data?.overview || null} />
      <RuntimeProvidersPanel providers={data?.providers || []} />
    </div>
    <RuntimeWebToolingPanel web={data?.web || null} />
  </div>

  <div class="insights-grid">
    <article class="panel">
      <div class="panel-head">
        <h3>Operator debt</h3>
        <span class:warn={data?.debt.policy_green_but_debt_remaining}>policy green, debt live</span>
      </div>
      <div class="stats-grid">
        <div class="stat-card"><span>Open debt</span><strong>{data?.debt.open_items || 0}</strong></div>
        <div class="stat-card"><span>Blocked debt</span><strong>{data?.debt.blocked_items || 0}</strong></div>
        <div class="stat-card"><span>Classic assets</span><strong>{data?.debt.classic_asset_files || 0}</strong></div>
        <div class="stat-card"><span>Classic hrefs</span><strong>{data?.debt.classic_href_references || 0}</strong></div>
        <div class="stat-card"><span>Embed fallbacks</span><strong>{data?.debt.embedded_fallback_references || 0}</strong></div>
        <div class="stat-card"><span>Size exceptions</span><strong>{data?.debt.size_exception_count || 0}</strong></div>
      </div>

      {#if data?.debt.top_classic_files?.length}
        <div class="list-card">
          <p class="list-label">Largest classic files still driving deletion pressure</p>
          {#each data.debt.top_classic_files as row}
            <div class="list-row">
              <code>{row.path}</code>
              <strong>{row.lines} lines</strong>
            </div>
          {/each}
        </div>
      {/if}
    </article>

    <article class="panel">
      <div class="panel-head">
        <h3>Orchestration surface</h3>
        <span>operator audit</span>
      </div>

      <div class="pill-grid">
        <span class:warn={pillState(data?.orchestration.capability_probes) !== 'enforced'}>{pillState(data?.orchestration.capability_probes)} probes</span>
        <span class:warn={pillState(data?.orchestration.alternative_plans) !== 'enforced'}>{pillState(data?.orchestration.alternative_plans)} alternatives</span>
        <span class:warn={pillState(data?.orchestration.verifier_request) !== 'enforced'}>{pillState(data?.orchestration.verifier_request)} verifier</span>
        <span class:warn={pillState(data?.orchestration.receipt_correlation) !== 'enforced'}>{pillState(data?.orchestration.receipt_correlation)} receipts</span>
        <span class:warn={pillState(data?.orchestration.nested_core_projection) !== 'enforced'}>{pillState(data?.orchestration.nested_core_projection)} nested projection</span>
        <span class:warn={pillState(data?.orchestration.hidden_state_pass) !== 'enforced'}>
          {pillState(data?.orchestration.hidden_state_pass)} hidden-state guard
        </span>
      </div>

      <div class="stats-grid">
        <div class="stat-card">
          <span>Fallback threshold</span>
          <strong>{data?.orchestration.adapter_fallback_threshold == null ? '-' : data.orchestration.adapter_fallback_threshold}</strong>
        </div>
        <div class="stat-card">
          <span>Hidden-state violations</span>
          <strong>{data?.orchestration.hidden_state_violations == null ? '-' : data.orchestration.hidden_state_violations}</strong>
        </div>
      </div>

      <div class="list-card">
        <p class="list-label">Plan variants</p>
        <div class="tag-list">
          {#each data?.orchestration.plan_variants || [] as variant}
            <span>{variant}</span>
          {/each}
        </div>
      </div>

      <div class="list-card">
        <p class="list-label">Correlation fields</p>
        <div class="tag-list">
          {#each data?.orchestration.correlation_fields || [] as field}
            <span>{field}</span>
          {/each}
        </div>
      </div>
    </article>
  </div>
</section>

<style>
  .runtime-page,
  .column {
    display: grid;
    gap: 18px;
  }
  .hero,
  .banner {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
    padding: 20px;
  }
  .hero,
  .hero-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .content-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 360px;
    gap: 18px;
  }
  .insights-grid,
  .stats-grid,
  .pill-grid,
  .tag-list {
    display: grid;
    gap: 12px;
  }
  .insights-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .panel,
  .stat-card,
  .list-card {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
  }
  .panel,
  .list-card,
  .stat-card {
    padding: 18px;
  }
  .panel {
    display: grid;
    gap: 16px;
  }
  .panel-head,
  .list-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .stats-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .stat-card {
    display: grid;
    gap: 6px;
  }
  .pill-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .pill-grid span,
  .tag-list span {
    border-radius: 999px;
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    padding: 0.5rem 0.8rem;
  }
  .tag-list {
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  }
  .list-card {
    display: grid;
    gap: 10px;
  }
  .list-label {
    color: #8aa4cf;
  }
  .warn {
    color: #f2b35a;
  }
  h2,
  p {
    margin: 0;
  }
  .eyebrow,
  .hero-copy {
    color: #8aa4cf;
  }
  .ghost {
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
    border-radius: 16px;
    padding: 0.8rem 1rem;
    text-decoration: none;
  }
  .error {
    border-color: rgba(229, 112, 93, 0.28);
    background: rgba(91, 31, 23, 0.58);
  }
  @media (max-width: 1120px) {
    .content-grid,
    .insights-grid {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 760px) {
    .hero,
    .hero-actions {
      flex-direction: column;
      align-items: flex-start;
    }
    .stats-grid,
    .pill-grid {
      grid-template-columns: 1fr;
    }
    .panel-head,
    .list-row {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
