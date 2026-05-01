const COMPONENT_TAG = 'infring-approvals-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-approvals-page-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  let approvals = [];
  let filterStatus = 'all';
  let loading = true;
  let loadError = '';
  let decisionLoading = {};

  $: filteredApprovals = filterStatus === 'all'
    ? approvals
    : approvals.filter(function(approval) { return approval.status === filterStatus; });

  function navigate(page) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(page);
    else if (typeof window !== 'undefined') window.location.hash = page;
  }

  function normalizeRows(rows) {
    var source = Array.isArray(rows) ? rows : [];
    return source.map(function(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      return Object.assign({}, row, {
        id: String(row.id || row.approval_id || ''),
        status: String(row.status || 'pending'),
        action: String(row.action || 'Action request'),
        description: String(row.description || ''),
        agent_name: String(row.agent_name || row.agent || 'Unknown agent')
      });
    });
  }

  function timeAgo(value) {
    if (!value) return '';
    var parsed = Date.parse(String(value));
    if (!Number.isFinite(parsed)) return '';
    var secs = Math.floor((Date.now() - parsed) / 1000);
    if (!Number.isFinite(secs) || secs < 0) secs = 0;
    if (secs < 60) return secs + 's ago';
    if (secs < 3600) return Math.floor(secs / 60) + 'm ago';
    if (secs < 86400) return Math.floor(secs / 3600) + 'h ago';
    return Math.floor(secs / 86400) + 'd ago';
  }

  function statusBadgeClass(status) {
    if (status === 'pending') return 'badge-warn';
    if (status === 'approved') return 'badge-success';
    if (status === 'rejected') return 'badge-error';
    return 'badge-muted';
  }

  function cardClass(status) {
    return [
      'card',
      'approval-card',
      status === 'approved' ? 'approved' : '',
      status === 'rejected' ? 'rejected' : '',
      status === 'expired' ? 'expired' : ''
    ].filter(Boolean).join(' ');
  }

  function isDecisionBusy(id) {
    return decisionLoading[String(id)] === true;
  }

  function setDecisionBusy(id, busy) {
    var key = String(id);
    if (!key) return;
    if (busy) decisionLoading = Object.assign({}, decisionLoading, { [key]: true });
    else {
      var next = Object.assign({}, decisionLoading);
      delete next[key];
      decisionLoading = next;
    }
  }

  async function loadData() {
    loading = true;
    loadError = '';
    try {
      var api = typeof window !== 'undefined' ? window.InfringAPI : null;
      if (!api || typeof api.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await api.get('/api/approvals');
      approvals = normalizeRows(data && data.approvals);
    } catch(e) {
      loadError = e && e.message ? e.message : 'Could not load approvals.';
    }
    loading = false;
  }

  async function submitDecision(id, action) {
    if (!id || isDecisionBusy(id)) return;
    setDecisionBusy(id, true);
    try {
      var api = typeof window !== 'undefined' ? window.InfringAPI : null;
      if (!api || typeof api.post !== 'function') throw new Error('Shell API client is unavailable.');
      await api.post('/api/approvals/' + id + '/' + action, {});
      if (window.InfringToast && typeof window.InfringToast.success === 'function') {
        window.InfringToast.success(action === 'approve' ? 'Approved' : 'Rejected');
      }
      await loadData();
    } catch(e) {
      var message = e && e.message ? e.message : ('Failed to ' + action);
      if (window.InfringToast && typeof window.InfringToast.error === 'function') window.InfringToast.error(message);
      else loadError = message;
    }
    setDecisionBusy(id, false);
  }

  function approve(id) {
    submitDecision(id, 'approve');
  }

  function reject(id) {
    var toast = typeof window !== 'undefined' ? window.InfringToast : null;
    if (toast && typeof toast.confirm === 'function') {
      toast.confirm('Reject Action', 'Are you sure you want to reject this action?', function() {
        submitDecision(id, 'reject');
      });
      return;
    }
    submitDecision(id, 'reject');
  }

  onMount(loadData);
</script>

<div>
  <div class="page-header">
    <div class="tabs mt-3" role="tablist">
      <div class="tab" role="tab" on:click={() => navigate('agents')}>Agents</div>
      <div class="tab" role="tab" on:click={() => navigate('sessions')}>Sessions</div>
      <div class="tab active" role="tab" on:click={() => navigate('approvals')}>Approvals</div>
    </div>
  </div>
  <div class="page-body">
    {#if loading}
      <div class="loading-state"><div class="spinner"></div><span>Loading...</span></div>
    {:else if loadError}
      <div class="error-state">
        <span class="error-icon">!</span>
        <p>{loadError}</p>
        <button class="btn btn-ghost btn-sm" type="button" on:click={loadData}>Retry</button>
      </div>
    {:else}
      <div>
        <div class="filter-pills mb-4">
          <button class:active={filterStatus === 'all'} class="filter-pill" type="button" on:click={() => filterStatus = 'all'}>All</button>
          <button class:active={filterStatus === 'pending'} class="filter-pill" type="button" on:click={() => filterStatus = 'pending'}>Pending</button>
          <button class:active={filterStatus === 'approved'} class="filter-pill" type="button" on:click={() => filterStatus = 'approved'}>Approved</button>
          <button class:active={filterStatus === 'rejected'} class="filter-pill" type="button" on:click={() => filterStatus = 'rejected'}>Rejected</button>
        </div>
        {#if filteredApprovals.length === 0}
          <infring-chat-stream-shell class="empty-state">
            <h4>No approvals</h4>
            <p class="hint">When agents request permission for sensitive actions, they'll appear here.</p>
          </infring-chat-stream-shell>
        {:else}
          <div class="card-grid">
            {#each filteredApprovals as approval (approval.id)}
              <div class={cardClass(approval.status)}>
                <div class="flex justify-between items-center mb-2">
                  <span class="card-header" style="margin:0">{approval.action}</span>
                  <span class={'badge ' + statusBadgeClass(approval.status)}>{approval.status}</span>
                </div>
                <div class="text-sm text-dim mb-2">{approval.description}</div>
                <div class="text-xs text-dim">Agent: <span>{approval.agent_name}</span> &middot; <span>{timeAgo(approval.created_at)}</span></div>
                {#if approval.status === 'pending'}
                  <div class="approval-actions" style="display:flex;gap:8px;margin-top:12px">
                    <button class="btn btn-success btn-sm" type="button" disabled={isDecisionBusy(approval.id)} on:click={() => approve(approval.id)}>Approve</button>
                    <button class="btn btn-danger btn-sm" type="button" disabled={isDecisionBusy(approval.id)} on:click={() => reject(approval.id)}>Reject</button>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
