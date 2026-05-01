const COMPONENT_TAG = 'infring-scheduler-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-scheduler-page-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'scheduler';
  export let panelRole = 'page';
  export let routeContract = 'scheduler';
  export let parentOwnedData = false;

  let view = {
    tab: 'jobs',
    jobs: [],
    loading: true,
    loadError: '',
    triggers: [],
    trigLoading: false,
    trigLoadError: '',
    history: [],
    historyLoading: false,
    showCreateForm: false,
    newJob: { name: '', cron: '', agent_id: '', message: '', enabled: true },
    creating: false,
    runningJobId: '',
    cronPresets: []
  };

  function hydrateLegacyViewModel() {
    if (typeof window === 'undefined' || typeof window.schedulerPage !== 'function') return;
    view = window.schedulerPage();
  }

  function repaint() {
    view = view;
  }

  function repaintSoon() {
    setTimeout(repaint, 80);
    setTimeout(repaint, 400);
    setTimeout(repaint, 1200);
  }

  async function call(methodName, ...args) {
    if (!view || typeof view[methodName] !== 'function') return undefined;
    var result = view[methodName].apply(view, args);
    repaint();
    if (result && typeof result.then === 'function') await result;
    repaint();
    repaintSoon();
    return result;
  }

  function navigate(target) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(target);
    else if (typeof window !== 'undefined') window.location.hash = target;
  }

  function setTab(tab) {
    view.tab = tab;
    repaint();
    if (tab === 'triggers' && !view.triggers.length && !view.trigLoading) call('loadTriggers');
    if (tab === 'history') call('loadHistory');
  }

  function showCreateForm() {
    view.showCreateForm = true;
    repaint();
  }

  function closeCreateForm() {
    view.showCreateForm = false;
    repaint();
  }

  function handleWindowKeydown(event) {
    if (view.showCreateForm && event.key === 'Escape') closeCreateForm();
  }

  function toggleNewJobEnabled() {
    view.newJob.enabled = !view.newJob.enabled;
    repaint();
  }

  function jobName(job) {
    return (job && (job.name || job.description)) || '(unnamed)';
  }

  function shortMessage(job) {
    var text = String((job && job.message) || '');
    return text.length > 60 ? text.substring(0, 60) + '...' : text;
  }

  function agentLabel(agentId) {
    return view.agentName ? view.agentName(agentId) : (agentId || '(any)');
  }

  function describeCron(expr) {
    return view.describeCron ? view.describeCron(expr) : String(expr || '');
  }

  function formatTime(value) {
    return view.formatTime ? view.formatTime(value) : '-';
  }

  function relativeTime(value) {
    return view.relativeTime ? view.relativeTime(value) : 'never';
  }

  function jobCount() {
    return view.jobCount ? view.jobCount() : 0;
  }

  function triggerType(pattern) {
    return view.triggerType ? view.triggerType(pattern) : 'unknown';
  }

  function applyPreset(preset) {
    if (view.applyCronPreset) view.applyCronPreset(preset);
    repaint();
  }

  onMount(async function() {
    hydrateLegacyViewModel();
    await call('loadData');
  });
</script>

<svelte:window on:keydown={handleWindowKeydown} />

<div>
  <div class="page-header page-header-subtabs-center">
    <div class="tabs mt-3" role="tablist">
      <div class="tab active" role="tab" on:click={() => navigate('scheduler')}>Scheduler</div>
      <div class="tab" role="tab" on:click={() => navigate('workflows')}>Workflows</div>
    </div>
    <div class="tabs tabs-subnav" role="tablist">
      <div class:active={view.tab === 'jobs'} class="tab" role="tab" on:click={() => setTab('jobs')}>
        Scheduled Jobs {#if view.jobs.length}<span class="badge badge-dim" style="margin-left:4px">{jobCount()}/{view.jobs.length} active</span>{/if}
      </div>
      <div class:active={view.tab === 'triggers'} class="tab" role="tab" on:click={() => setTab('triggers')}>
        Event Triggers {#if view.triggers.length}<span class="badge badge-dim" style="margin-left:4px">{view.triggers.length}</span>{/if}
      </div>
      <div class:active={view.tab === 'history'} class="tab" role="tab" on:click={() => setTab('history')}>Run History</div>
    </div>
  </div>

  <div class="page-body">
    {#if view.tab === 'jobs'}
      <div class="flex gap-2 mb-4"><button class="btn btn-primary btn-sm" type="button" on:click={showCreateForm}>+ New Job</button></div>
      {#if view.loading}
        <div class="loading-state"><div class="spinner"></div><span>Loading scheduled jobs...</span></div>
      {:else if view.loadError}
        <div class="error-state">
          <span class="error-icon">!</span><p>{view.loadError}</p>
          <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('loadData')}>Retry</button>
        </div>
      {:else}
        <div class="card mb-4" style="border-left:3px solid var(--accent)">
          <div class="font-bold" style="font-size:13px;margin-bottom:4px">Scheduled Jobs</div>
          <div class="text-sm text-dim" style="line-height:1.6">
            Create cron-based scheduled jobs that send messages to agents on a recurring schedule.
            Use cron expressions like <code style="color:var(--accent)">*/5 * * * *</code> or
            <code style="color:var(--accent)">0 9 * * 1-5</code>. You can also run any job manually.
          </div>
        </div>
        {#if view.jobs.length}
          <div class="table-wrap">
            <table>
              <thead><tr><th>Name</th><th>Schedule</th><th>Agent</th><th>Status</th><th>Last Run</th><th>Next Run</th><th>Actions</th></tr></thead>
              <tbody>
                {#each view.jobs as job (job.id)}
                  <tr>
                    <td>
                      <span class="font-bold">{jobName(job)}</span>
                      {#if job.message}<div class="text-xs text-dim" title={job.message}>{shortMessage(job)}</div>{/if}
                    </td>
                    <td><code style="font-size:11px;color:var(--accent)">{job.cron}</code><div class="text-xs text-dim">{describeCron(job.cron)}</div></td>
                    <td class="truncate" style="max-width:120px" title={job.agent_id || job.agent}>{agentLabel(job.agent_id || job.agent)}</td>
                    <td><span class={job.enabled ? 'badge badge-success' : 'badge badge-dim'}>{job.enabled ? 'Active' : 'Paused'}</span></td>
                    <td class="text-xs" title={formatTime(job.last_run)}>{relativeTime(job.last_run)}</td>
                    <td class="text-xs" title={formatTime(job.next_run)}>{relativeTime(job.next_run)}</td>
                    <td>
                      <div class="flex gap-1">
                        <button class="btn btn-primary btn-sm" type="button" disabled={view.runningJobId === job.id} on:click={() => call('runNow', job)}>{view.runningJobId === job.id ? '...' : 'Run'}</button>
                        <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('toggleJob', job)}>{job.enabled ? 'Pause' : 'Enable'}</button>
                        <button class="btn btn-danger btn-sm" type="button" on:click={() => call('deleteJob', job)}>Del</button>
                      </div>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <infring-chat-stream-shell class="empty-state">
            <h4>No scheduled jobs</h4>
            <p class="hint">Create a cron job to run agents on a recurring schedule. Jobs are stored persistently and survive restarts.</p>
            <button class="btn btn-primary mt-4" type="button" on:click={showCreateForm}>+ Create Scheduled Job</button>
          </infring-chat-stream-shell>
        {/if}
      {/if}
    {:else if view.tab === 'triggers'}
      {#if view.trigLoading}
        <div class="loading-state"><div class="spinner"></div><span>Loading triggers...</span></div>
      {:else if view.trigLoadError}
        <div class="error-state">
          <span class="error-icon">!</span><p>{view.trigLoadError}</p>
          <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('loadTriggers')}>Retry</button>
        </div>
      {:else}
        <div class="card mb-4" style="border-left:3px solid var(--accent)">
          <div class="font-bold" style="font-size:13px;margin-bottom:4px">Event Triggers</div>
          <div class="text-sm text-dim" style="line-height:1.6">
            Event triggers fire agents in response to system events. Create and manage triggers on the
            <a href="#workflows" style="color:var(--accent)">Workflows</a> page.
          </div>
        </div>
        {#if view.triggers.length}
          <div class="table-wrap">
            <table>
              <thead><tr><th>Agent</th><th>Pattern</th><th>Prompt</th><th>Fires</th><th>Enabled</th><th>Created</th><th>Actions</th></tr></thead>
              <tbody>
                {#each view.triggers as trigger (trigger.id)}
                  <tr>
                    <td class="font-bold truncate" style="max-width:120px" title={trigger.agent_id}>{agentLabel(trigger.agent_id)}</td>
                    <td><span class="badge badge-created trigger-type">{triggerType(trigger.pattern)}</span></td>
                    <td class="truncate text-xs text-dim" style="max-width:180px" title={trigger.prompt_template}>{trigger.prompt_template}</td>
                    <td>{trigger.fire_count}{trigger.max_fires > 0 ? '/' + trigger.max_fires : ''}</td>
                    <td><div class:active={trigger.enabled} class="toggle" on:click={() => call('toggleTrigger', trigger)}></div></td>
                    <td class="text-xs">{new Date(trigger.created_at).toLocaleDateString()}</td>
                    <td><button class="btn btn-danger btn-sm" type="button" on:click={() => call('deleteTrigger', trigger)}>Delete</button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <infring-chat-stream-shell class="empty-state">
            <h4>No event triggers</h4>
            <p class="hint">Create event triggers on the <a href="#workflows" style="color:var(--accent)">Workflows page</a> to fire agents in response to system events.</p>
          </infring-chat-stream-shell>
        {/if}
      {/if}
    {:else}
      {#if view.historyLoading}
        <div class="loading-state"><div class="spinner"></div><span>Loading run history...</span></div>
      {:else}
        <div class="card mb-4" style="border-left:3px solid var(--accent)">
          <div class="font-bold" style="font-size:13px;margin-bottom:4px">Run History</div>
          <div class="text-sm text-dim" style="line-height:1.6">Recent executions of scheduled jobs and event trigger fires.</div>
        </div>
        {#if view.history.length}
          <div class="table-wrap">
            <table>
              <thead><tr><th>Time</th><th>Name</th><th>Type</th><th>Status</th><th>Total Runs</th></tr></thead>
              <tbody>
                {#each view.history as item, idx (idx)}
                  <tr>
                    <td class="text-xs" style="white-space:nowrap">{formatTime(item.timestamp)}</td>
                    <td class="font-bold">{item.name}</td>
                    <td><span class={item.type === 'schedule' ? 'badge badge-created' : 'badge badge-dim'}>{item.type === 'schedule' ? 'Cron Job' : 'Trigger'}</span></td>
                    <td><span class="badge badge-success">{item.status}</span></td>
                    <td>{item.run_count}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <infring-chat-stream-shell class="empty-state">
            <h4>No run history yet</h4>
            <p class="hint">Run history will appear here after scheduled jobs or triggers execute.</p>
          </infring-chat-stream-shell>
        {/if}
      {/if}
    {/if}
  </div>

  {#if view.showCreateForm}
    <div class="modal-overlay" on:click={(event) => { if (event.currentTarget === event.target) closeCreateForm(); }}>
      <div class="modal">
        <div class="modal-header"><h3>Create Scheduled Job</h3><button class="modal-close" type="button" on:click={closeCreateForm}>&times;</button></div>
        <div class="form-group"><label>Job Name</label><input class="form-input" bind:value={view.newJob.name} placeholder="daily-report"></div>
        <div class="form-group">
          <label>Cron Expression</label>
          <input class="form-input" bind:value={view.newJob.cron} placeholder="0 9 * * 1-5" style="font-family:monospace">
          {#if view.newJob.cron}<div class="text-xs text-dim mt-1">{describeCron(view.newJob.cron)}</div>{/if}
          <div class="text-xs text-dim mt-1">Format: <code>minute hour day-of-month month day-of-week</code></div>
        </div>
        <div class="form-group">
          <label>Quick Presets</label>
          <div class="flex gap-1 flex-wrap">
            {#each view.cronPresets as preset (preset.cron)}
              <button class={view.newJob.cron === preset.cron ? 'btn btn-sm btn-primary' : 'btn btn-sm btn-ghost'} type="button" on:click={() => applyPreset(preset)}>{preset.label}</button>
            {/each}
          </div>
        </div>
        <div class="form-group">
          <label>Target Agent</label>
          <select class="form-select" bind:value={view.newJob.agent_id}>
            <option value="">Any available agent</option>
            {#each view.availableAgents || [] as agent (agent.id)}
              <option value={agent.id}>{agent.name} ({agent.model_provider || 'unknown'}:{agent.model_name || 'unknown'})</option>
            {/each}
          </select>
          {#if !(view.availableAgents || []).length}<div class="text-xs text-dim mt-1">No agents running. <a href="#agents" style="color:var(--accent)">Spawn one first.</a></div>{/if}
        </div>
        <div class="form-group">
          <label>Message to Send</label>
          <textarea class="form-textarea" bind:value={view.newJob.message} placeholder="Generate and email the daily status report..." rows="3"></textarea>
          <div class="text-xs text-dim mt-1">The message sent to the agent each time this job runs.</div>
        </div>
        <div class="form-group">
          <label class="flex items-center gap-2">
            <div class:active={view.newJob.enabled} class="toggle" on:click={toggleNewJobEnabled}></div>
            <span>{view.newJob.enabled ? 'Enabled (will start running immediately)' : 'Disabled (create paused)'}</span>
          </label>
        </div>
        <button class="btn btn-primary btn-block mt-4" type="button" disabled={view.creating} on:click={() => call('createJob')}>
          {view.creating ? 'Creating...' : 'Create Schedule'}
        </button>
      </div>
    </div>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
