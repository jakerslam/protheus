<script lang="ts">
  import type { RuntimeOverview } from '$lib/runtime';

  export let overview: RuntimeOverview | null = null;

  function formatUptime(seconds: number): string {
    const total = Number(seconds || 0);
    if (!Number.isFinite(total) || total <= 0) return '0m';
    if (total < 60) return `${Math.floor(total)}s`;
    if (total < 3600) return `${Math.floor(total / 60)}m ${Math.floor(total % 60)}s`;
    if (total < 86400) return `${Math.floor(total / 3600)}h ${Math.floor((total % 3600) / 60)}m`;
    return `${Math.floor(total / 86400)}d ${Math.floor((total % 86400) / 3600)}h`;
  }
</script>

<article class="panel">
  <div class="panel-head">
    <h3>Runtime overview</h3>
  </div>

  {#if !overview}
    <div class="empty-card">Runtime status unavailable.</div>
  {:else}
    <div class="stats-grid">
      <div class="stat-card"><span>Version</span><strong>{overview.version}</strong></div>
      <div class="stat-card"><span>Platform</span><strong>{overview.platform} / {overview.arch}</strong></div>
      <div class="stat-card"><span>Uptime</span><strong>{formatUptime(overview.uptime_seconds)}</strong></div>
      <div class="stat-card"><span>Agents</span><strong>{overview.agent_count}</strong></div>
      <div class="stat-card"><span>Default provider</span><strong>{overview.default_provider}</strong></div>
      <div class="stat-card"><span>Default model</span><strong>{overview.default_model}</strong></div>
      <div class="stat-card"><span>API listen</span><strong>{overview.api_listen}</strong></div>
      <div class="stat-card"><span>Log level</span><strong>{overview.log_level}</strong></div>
      <div class="stat-card wide"><span>Home dir</span><strong>{overview.home_dir}</strong></div>
      <div class="stat-card"><span>Network</span><strong>{overview.network_enabled ? 'enabled' : 'disabled'}</strong></div>
    </div>
  {/if}
</article>

<style>
  .panel,
  .stat-card,
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
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  .stat-card,
  .empty-card {
    border-radius: 20px;
    padding: 14px;
    display: grid;
    gap: 6px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  span {
    color: #8aa4cf;
  }
  strong {
    word-break: break-word;
  }
  @media (max-width: 760px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
