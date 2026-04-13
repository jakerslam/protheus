<script lang="ts">
  import type { DashboardSystemInfo } from '$lib/settings';

  export let info: DashboardSystemInfo | null = null;

  function formatUptime(seconds: number): string {
    const total = Number(seconds || 0);
    if (!Number.isFinite(total) || total <= 0) return '0m';
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    if (hours <= 0) return `${minutes}m`;
    return `${hours}h ${minutes}m`;
  }
</script>

<article class="panel">
  <div class="panel-head">
    <h3>System info</h3>
  </div>

  {#if !info}
    <div class="empty-card">System info unavailable.</div>
  {:else}
    <div class="stats-grid">
      <div class="stat-card"><span>Version</span><strong>{info.version}</strong></div>
      <div class="stat-card"><span>Platform</span><strong>{info.platform} / {info.arch}</strong></div>
      <div class="stat-card"><span>Uptime</span><strong>{formatUptime(info.uptime_seconds)}</strong></div>
      <div class="stat-card"><span>Agents</span><strong>{info.agent_count}</strong></div>
      <div class="stat-card"><span>Default provider</span><strong>{info.default_provider}</strong></div>
      <div class="stat-card"><span>Default model</span><strong>{info.default_model}</strong></div>
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
  span {
    color: #8aa4cf;
  }
  @media (max-width: 760px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
