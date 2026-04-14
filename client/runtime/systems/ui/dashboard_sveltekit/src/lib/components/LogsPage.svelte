<script lang="ts">
  import { readAuditVerification, readLogEntries, type DashboardAuditVerification, type DashboardLogEntry } from '$lib/logs';
  import { onDestroy, onMount } from 'svelte';

  let entries: DashboardLogEntry[] = [];
  let verification: DashboardAuditVerification | null = null;
  let levelFilter = '';
  let loading = true;
  let error = '';
  let pollHandle: ReturnType<typeof setInterval> | null = null;

  onMount(async () => {
    await refresh();
    pollHandle = setInterval(() => void refreshLogsOnly(), 4000);
  });

  onDestroy(() => {
    if (pollHandle) clearInterval(pollHandle);
  });

  async function refreshLogsOnly(): Promise<void> {
    try {
      entries = await readLogEntries();
    } catch {
      // keep current view on transient refresh failures
    }
  }

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      const [nextEntries, nextVerification] = await Promise.all([readLogEntries(), readAuditVerification()]);
      entries = nextEntries;
      verification = nextVerification;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'logs_unavailable');
    } finally {
      loading = false;
    }
  }

  function classifyLevel(action: string): string {
    const lower = String(action || '').toLowerCase();
    if (lower.includes('error') || lower.includes('fail') || lower.includes('crash')) return 'error';
    if (lower.includes('warn') || lower.includes('deny') || lower.includes('block')) return 'warn';
    return 'info';
  }

  function formatRelativeTime(value: string): string {
    const stamp = String(value || '').trim();
    if (!stamp) return 'recent';
    const ts = new Date(stamp).getTime();
    if (!Number.isFinite(ts)) return stamp;
    const delta = Math.max(0, Math.floor((Date.now() - ts) / 1000));
    if (delta < 60) return `${delta}s ago`;
    if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
    if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
    return `${Math.floor(delta / 86400)}d ago`;
  }

  $: filtered = entries.filter((entry) => !levelFilter || classifyLevel(entry.action) === levelFilter);
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native logs</p>
      <h2>Recent audit activity and receipt verification without the legacy page.</h2>
    </div>
    <div class="hero-actions">
      <select bind:value={levelFilter} class="field select">
        <option value="">All levels</option>
        <option value="info">Info</option>
        <option value="warn">Warn</option>
        <option value="error">Error</option>
      </select>
    </div>
  </div>

  {#if verification}
    <article class="panel verify">
      <strong>Audit chain</strong>
      <span>{verification.chain_valid === true ? 'valid' : verification.chain_valid === false ? 'invalid' : 'unknown'} · {verification.tip_hash || 'no tip hash reported'}</span>
    </article>
  {/if}

  {#if error}
    <div class="banner error">{error}</div>
  {/if}

  <article class="panel">
    <div class="panel-head"><h3>Recent audit entries</h3><button class="ghost small" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button></div>
    {#if filtered.length === 0}
      <div class="empty-card">No audit entries matched this filter.</div>
    {:else}
      <div class="rows">
        {#each filtered.slice(0, 80) as entry}
          <div class="row">
            <div class="row-copy">
              <strong>{entry.action}</strong>
              <span>{entry.actor || entry.agent_id || 'system'} · {formatRelativeTime(entry.ts)}</span>
            </div>
            <span class:warn={classifyLevel(entry.action) === 'warn'} class:error={classifyLevel(entry.action) === 'error'}>{classifyLevel(entry.action)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </article>
</section>

<style>
  .page, .rows { display: grid; gap: 18px; }
  .hero, .panel, .banner, .row { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .verify { background: rgba(30,56,95,0.58); }
  .row { padding: 12px 14px; border-radius: 20px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .ghost, .field { padding: 0.8rem 1rem; border-radius: 16px; text-decoration: none; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, span { color: #8aa4cf; }
  .warn { color: #f2b35a; }
  .error { color: #eb7c69; }
  .banner.error { background: rgba(91,31,23,0.58); }
  @media (max-width: 760px) {
    .hero, .hero-actions, .row { flex-direction: column; align-items: flex-start; }
  }
</style>
