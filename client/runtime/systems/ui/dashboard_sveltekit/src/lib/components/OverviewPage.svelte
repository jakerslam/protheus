<script lang="ts">
  import { onMount } from 'svelte';
  import { dashboardPageHref, highChurnMigrationTargets, nativeDashboardPages } from '$lib/dashboard';
  import { formatRelativeTime, readOverviewSnapshot, type DashboardOverviewSnapshot } from '$lib/runtime';

  let snapshot: DashboardOverviewSnapshot | null = null;
  let error = '';

  onMount(async () => {
    try {
      snapshot = await readOverviewSnapshot();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'overview_unavailable');
    }
  });

  function compactNumber(value: number): string {
    if (!Number.isFinite(value) || value <= 0) return '0';
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
    if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
    return String(Math.round(value));
  }

  function compactCost(value: number): string {
    if (!Number.isFinite(value) || value <= 0) return '$0.00';
    if (value < 0.01) return '<$0.01';
    return `$${value.toFixed(2)}`;
  }

  $: configuredProviders = (snapshot?.providers || []).filter((row) => /configured/i.test(String(row.auth_status || '')));
  $: connectedChannels = (snapshot?.channels || []).filter((row) => row.has_token);
</script>

<section class="overview">
  <div class="hero">
    <div>
      <p class="eyebrow">Migration status</p>
      <h2>SvelteKit is now the primary dashboard shell.</h2>
      <p class="hero-copy">
        The native shell now owns the primary operator routes. Remaining migration work is focused on retiring the top-level classic compatibility host and asset corpus, not reviving per-page iframe fallback.
      </p>
    </div>
    <div class="hero-card">
      <div class="hero-stat">
        <span class="label">Next targets</span>
        <strong>{highChurnMigrationTargets.length}</strong>
      </div>
      <div class="hero-stat">
        <span class="label">Native pages</span>
        <strong>{nativeDashboardPages.length}</strong>
      </div>
    </div>
  </div>

  {#if error}
    <div class="panel error">Overview unavailable: {error}</div>
  {:else if !snapshot}
    <div class="panel loading">Loading dashboard overview…</div>
  {:else}
    <div class="stats-grid">
      <article class="panel stat">
        <span class="label">Connected</span>
        <strong>{snapshot.status.connected === false ? 'No' : 'Yes'}</strong>
        <small>Daemon {snapshot.status.daemon || 'unknown'}</small>
      </article>
      <article class="panel stat">
        <span class="label">Agents</span>
        <strong>{snapshot.agentCount}</strong>
        <small>Active roster from `/api/agents`</small>
      </article>
      <article class="panel stat">
        <span class="label">Providers</span>
        <strong>{configuredProviders.length}</strong>
        <small>Configured model providers</small>
      </article>
      <article class="panel stat">
        <span class="label">Channels</span>
        <strong>{connectedChannels.length}</strong>
        <small>Connected external channels</small>
      </article>
      <article class="panel stat">
        <span class="label">Tokens</span>
        <strong>{compactNumber(snapshot.usageSummary.total_tokens)}</strong>
        <small>Rolled up from `/api/usage`</small>
      </article>
      <article class="panel stat">
        <span class="label">Spend</span>
        <strong>{compactCost(snapshot.usageSummary.total_cost)}</strong>
        <small>Estimated usage spend</small>
      </article>
    </div>

    <div class="detail-grid">
      <article class="panel">
        <h3>Runtime snapshot</h3>
        <dl class="runtime-grid">
          <div>
            <dt>Version</dt>
            <dd>{snapshot.version.version || 'unknown'}</dd>
          </div>
          <div>
            <dt>Platform</dt>
            <dd>{snapshot.version.platform || 'unknown'} / {snapshot.version.arch || 'unknown'}</dd>
          </div>
          <div>
            <dt>Default model</dt>
            <dd>{snapshot.status.default_model || 'not reported'}</dd>
          </div>
          <div>
            <dt>Listen</dt>
            <dd>{snapshot.status.api_listen || snapshot.status.listen || 'not reported'}</dd>
          </div>
        </dl>
      </article>

      <article class="panel">
        <h3>Migration queue</h3>
        <ul class="queue">
          {#each highChurnMigrationTargets as target}
            <li>
              <a href={dashboardPageHref(target.key)}>{target.title}</a>
              <span>{target.summary}</span>
            </li>
          {/each}
        </ul>
      </article>
    </div>

    <div class="detail-grid">
      <article class="panel">
        <h3>Recent audit activity</h3>
        {#if snapshot.recentAudit.length === 0}
          <p class="muted">No recent audit entries were returned.</p>
        {:else}
          <ul class="audit-list">
            {#each snapshot.recentAudit as entry}
              <li>
                <div>
                  <strong>{entry.action || 'Unknown action'}</strong>
                  <span>{entry.actor || entry.agent_id || 'system'}</span>
                </div>
                <time>{formatRelativeTime(entry.ts)}</time>
              </li>
            {/each}
          </ul>
        {/if}
      </article>

      <article class="panel">
        <h3>Providers</h3>
        {#if snapshot.providers.length === 0}
          <p class="muted">No providers reported yet.</p>
        {:else}
          <ul class="provider-list">
            {#each snapshot.providers as provider}
              <li>
                <strong>{provider.display_name || provider.id || 'Provider'}</strong>
                <span>{provider.auth_status || provider.health || 'unknown'}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </article>
    </div>
  {/if}
</section>

<style>
  .overview {
    display: grid;
    gap: 18px;
  }

  .hero,
  .panel {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
  }

  .hero {
    display: flex;
    align-items: stretch;
    justify-content: space-between;
    gap: 18px;
    padding: 22px;
  }

  h2,
  h3 {
    margin: 0;
  }

  .hero-copy,
  .muted {
    color: #bdd0f0;
  }

  .hero-card {
    min-width: 220px;
    display: grid;
    gap: 12px;
  }

  .hero-stat {
    border-radius: 20px;
    padding: 16px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(255, 255, 255, 0.03);
  }

  .hero-stat strong,
  .stat strong {
    font-size: 1.7rem;
  }

  .label,
  dt,
  .audit-list span {
    color: #8aa4cf;
  }

  .stats-grid,
  .detail-grid {
    display: grid;
    gap: 18px;
  }

  .stats-grid {
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  }

  .detail-grid {
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  }

  .panel {
    padding: 20px;
  }

  .stat {
    display: grid;
    gap: 8px;
  }

  .runtime-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 14px;
    margin: 16px 0 0;
  }

  .runtime-grid dd,
  .runtime-grid dt {
    margin: 0;
  }

  .queue,
  .audit-list,
  .provider-list {
    list-style: none;
    padding: 0;
    margin: 16px 0 0;
    display: grid;
    gap: 12px;
  }

  .queue li,
  .audit-list li,
  .provider-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-radius: 18px;
    padding: 14px 16px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(158, 188, 255, 0.1);
  }

  .queue li {
    align-items: flex-start;
    flex-direction: column;
  }

  .queue a {
    color: #eef4ff;
    text-decoration: none;
    font-weight: 600;
  }

  .queue span,
  .provider-list span,
  time,
  small {
    color: #bdd0f0;
  }

  .error {
    color: #ff9b9b;
  }

  @media (max-width: 860px) {
    .hero {
      flex-direction: column;
    }

    .hero-card {
      min-width: 0;
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
