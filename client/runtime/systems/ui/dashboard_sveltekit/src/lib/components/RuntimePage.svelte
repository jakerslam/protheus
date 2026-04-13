<script lang="ts">
  import { dashboardClassicHref } from '$lib/dashboard';
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
      <a class="ghost" href={dashboardClassicHref('runtime')}>Open classic runtime</a>
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
    .content-grid {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 760px) {
    .hero,
    .hero-actions {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
