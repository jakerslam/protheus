<script lang="ts">
  import { dashboardClassicHref } from '$lib/dashboard';
  import { approveApproval, readApprovals, rejectApproval, type DashboardApprovalRow } from '$lib/approvals';
  import { onMount } from 'svelte';

  let approvals: DashboardApprovalRow[] = [];
  let filterStatus = 'all';
  let busyKey = '';
  let loading = true;
  let error = '';
  let notice = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      approvals = await readApprovals();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'approvals_unavailable');
    } finally {
      loading = false;
    }
  }

  async function approve(id: string): Promise<void> {
    if (busyKey) return;
    busyKey = `approve:${id}`;
    try {
      notice = await approveApproval(id);
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'approve_failed');
    } finally {
      busyKey = '';
    }
  }

  async function reject(id: string): Promise<void> {
    if (busyKey) return;
    busyKey = `reject:${id}`;
    try {
      notice = await rejectApproval(id);
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'reject_failed');
    } finally {
      busyKey = '';
    }
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

  $: filtered = approvals.filter((row) => filterStatus === 'all' || row.status === filterStatus);
  $: pendingCount = approvals.filter((row) => row.status === 'pending').length;
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native approvals</p>
      <h2>Review sensitive agent actions without dropping back to classic.</h2>
    </div>
    <div class="hero-actions">
      <select bind:value={filterStatus} class="field select">
        <option value="all">All</option>
        <option value="pending">Pending</option>
        <option value="approved">Approved</option>
        <option value="rejected">Rejected</option>
      </select>
      <a class="ghost" href={dashboardClassicHref('approvals')}>Open classic approvals</a>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <article class="panel">
    <div class="panel-head">
      <h3>Approval queue</h3>
      <span class="meta">{pendingCount} pending</span>
    </div>
    {#if loading}
      <div class="empty-card">Loading approvals…</div>
    {:else if filtered.length === 0}
      <div class="empty-card">No approvals matched this filter.</div>
    {:else}
      <div class="rows">
        {#each filtered as approval}
          <div class="row">
            <div class="row-copy">
              <strong>{approval.title}</strong>
              <p>{approval.summary || 'No summary provided.'}</p>
              <span class="meta">{approval.requested_by} · {formatRelativeTime(approval.created_at)}</span>
            </div>
            <div class="row-actions">
              <span class:pending={approval.status === 'pending'}>{approval.status}</span>
              {#if approval.status === 'pending'}
                <button class="primary small" type="button" disabled={busyKey === `approve:${approval.id}`} on:click={() => void approve(approval.id)}>Approve</button>
                <button class="ghost small" type="button" disabled={busyKey === `reject:${approval.id}`} on:click={() => void reject(approval.id)}>Reject</button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </article>
</section>

<style>
  .page, .rows { display: grid; gap: 18px; }
  .hero, .panel, .banner, .row, .field { border: 1px solid rgba(158, 188, 255, 0.16); background: rgba(11, 22, 39, 0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 14px; border-radius: 20px; background: rgba(255, 255, 255, 0.04); }
  .row-copy { display: grid; gap: 6px; }
  .field { padding: 0.7rem 0.85rem; border-radius: 16px; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; text-decoration: none; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .small { padding: 0.5rem 0.75rem; }
  .pending { color: #f2b35a; }
  .meta, .eyebrow, p { color: #8aa4cf; margin: 0; }
  .notice { background: rgba(23, 68, 45, 0.58); }
  .error { background: rgba(91, 31, 23, 0.58); }
  @media (max-width: 760px) {
    .hero, .hero-actions, .row, .row-actions { flex-direction: column; align-items: flex-start; }
  }
</style>
