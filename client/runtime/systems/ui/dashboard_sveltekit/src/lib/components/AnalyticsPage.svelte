<script lang="ts">
  import { readAnalyticsSnapshot, type AnalyticsSnapshot } from '$lib/analytics';
  import { onMount } from 'svelte';

  let snapshot: AnalyticsSnapshot | null = null;
  let loading = true;
  let error = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      snapshot = await readAnalyticsSnapshot();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'analytics_unavailable');
    } finally {
      loading = false;
    }
  }

  function formatTokens(value: number): string {
    if (!value) return '0';
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`;
    if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
    return String(value);
  }

  function formatCost(value: number): string {
    if (!value) return '$0.00';
    if (value < 0.01) return `$${value.toFixed(4)}`;
    return `$${value.toFixed(2)}`;
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native analytics</p>
      <h2>Usage, spend, and model/agent breakdowns inside the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {/if}

  {#if !snapshot}
    <article class="panel"><div class="empty-card">Loading analytics…</div></article>
  {:else}
    <div class="stats-grid">
      <article class="panel stat"><span>Calls</span><strong>{snapshot.summary.call_count}</strong></article>
      <article class="panel stat"><span>Tool calls</span><strong>{snapshot.summary.total_tool_calls}</strong></article>
      <article class="panel stat"><span>Input tokens</span><strong>{formatTokens(snapshot.summary.total_input_tokens)}</strong></article>
      <article class="panel stat"><span>Output tokens</span><strong>{formatTokens(snapshot.summary.total_output_tokens)}</strong></article>
      <article class="panel stat"><span>Total spend</span><strong>{formatCost(snapshot.summary.total_cost_usd)}</strong></article>
      <article class="panel stat"><span>Today</span><strong>{formatCost(snapshot.todayCost)}</strong></article>
    </div>

    <div class="content-grid">
      <article class="panel">
        <div class="panel-head"><h3>By model</h3></div>
        <div class="rows">
          {#each snapshot.byModel.slice(0, 12) as row}
            <div class="row">
              <strong>{row.model}</strong>
              <span>{row.call_count} calls · {formatTokens(row.total_input_tokens + row.total_output_tokens)} tokens · {formatCost(row.total_cost_usd)}</span>
            </div>
          {/each}
        </div>
      </article>

      <article class="panel">
        <div class="panel-head"><h3>By agent</h3></div>
        <div class="rows">
          {#each snapshot.byAgent.slice(0, 12) as row}
            <div class="row">
              <strong>{row.agent_name}</strong>
              <span>{formatTokens(row.total_tokens)} tokens · {row.tool_calls} tools · {formatCost(row.cost_usd)}</span>
            </div>
          {/each}
        </div>
      </article>
    </div>

    <article class="panel">
      <div class="panel-head"><h3>Daily cost trend</h3><span class="meta">{snapshot.firstEventDate || 'recent window'}</span></div>
      <div class="rows">
        {#each snapshot.dailyCosts.slice(-14) as day}
          <div class="row">
            <strong>{day.date}</strong>
            <span>{formatCost(day.cost_usd)}</span>
          </div>
        {/each}
      </div>
    </article>
  {/if}
</section>

<style>
  .page, .stats-grid, .content-grid, .rows { display: grid; gap: 18px; }
  .stats-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .content-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; border-radius: 20px; background: rgba(255,255,255,0.04); }
  .ghost { padding: 0.8rem 1rem; border-radius: 16px; text-decoration: none; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .error { background: rgba(91,31,23,0.58); }
  @media (max-width: 980px) {
    .stats-grid, .content-grid { grid-template-columns: 1fr; }
  }
  @media (max-width: 760px) {
    .hero, .hero-actions, .row { flex-direction: column; align-items: flex-start; }
  }
</style>
