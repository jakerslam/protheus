<script lang="ts">
  import type { RuntimeWebStatus } from '$lib/runtime';

  export let web: RuntimeWebStatus | null = null;

  function formatReceiptTime(value: string): string {
    const stamp = String(value || '').trim();
    if (!stamp) return 'recent';
    const date = new Date(stamp);
    return Number.isNaN(date.getTime()) ? stamp : date.toLocaleString();
  }
</script>

<article class="panel">
  <div class="panel-head">
    <h3>Web tooling</h3>
  </div>

  {#if !web}
    <div class="empty-card">Web tooling status unavailable.</div>
  {:else}
    <div class="stats-grid">
      <div class="stat-card"><span>Status</span><strong>{web.enabled ? 'enabled' : 'disabled'}</strong></div>
      <div class="stat-card"><span>Rate limit</span><strong>{web.rate_limit}</strong></div>
      <div class="stat-card"><span>Total receipts</span><strong>{web.receipts_total}</strong></div>
      <div class="stat-card"><span>Recent denied</span><strong>{web.recent_denied}</strong></div>
      <div class="stat-card wide"><span>Last URL</span><strong>{web.last_url}</strong></div>
    </div>

    {#if web.recent_receipts.length > 0}
      <div class="receipts">
        {#each web.recent_receipts as receipt}
          <div class="receipt-row">
            <div>
              <strong>{receipt.method} · {receipt.status}</strong>
              <p>{receipt.requested_url}</p>
            </div>
            <span class:blocked={receipt.blocked}>{receipt.blocked ? 'blocked' : formatReceiptTime(receipt.created_at)}</span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty-card">No recent web receipts recorded.</div>
    {/if}
  {/if}
</article>

<style>
  .panel,
  .stat-card,
  .receipt-row,
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
  .receipt-row,
  .empty-card {
    border-radius: 20px;
    padding: 14px;
  }
  .stat-card {
    display: grid;
    gap: 6px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  .receipts {
    display: grid;
    gap: 12px;
  }
  .receipt-row {
    display: flex;
    justify-content: space-between;
    gap: 14px;
  }
  .blocked {
    color: #f2b35a;
  }
  span,
  p {
    color: #8aa4cf;
    margin: 0;
  }
  strong {
    word-break: break-word;
  }
  @media (max-width: 760px) {
    .stats-grid {
      grid-template-columns: 1fr;
    }
    .receipt-row {
      flex-direction: column;
    }
  }
</style>
