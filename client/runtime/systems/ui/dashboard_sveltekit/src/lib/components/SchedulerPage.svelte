<script lang="ts">
  import { readSidebarAgents } from '$lib/chat';
  import { createCronJob, deleteCronJob, deleteTrigger, readCronJobs, readTriggers, runCronJobNow, setCronJobEnabled, setTriggerEnabled, type DashboardCronJobRow, type DashboardTriggerRow } from '$lib/scheduler';
  import { onMount } from 'svelte';

  let jobs: DashboardCronJobRow[] = [];
  let triggers: DashboardTriggerRow[] = [];
  let agents: Array<{ id: string; name?: string }> = [];
  let jobName = '';
  let jobCron = '';
  let jobAgentId = '';
  let jobMessage = '';
  let jobEnabled = true;
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
      [jobs, triggers, agents] = await Promise.all([readCronJobs(), readTriggers(), readSidebarAgents()]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'scheduler_unavailable');
    } finally {
      loading = false;
    }
  }

  async function saveJob(): Promise<void> {
    if (!jobName.trim() || !jobCron.trim()) return;
    busyKey = 'create-job';
    try {
      notice = await createCronJob({ agent_id: jobAgentId, name: jobName.trim(), cron: jobCron.trim(), message: jobMessage.trim(), enabled: jobEnabled });
      jobName = '';
      jobCron = '';
      jobMessage = '';
      jobAgentId = '';
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'job_create_failed');
    } finally {
      busyKey = '';
    }
  }

  function formatTime(value: string): string {
    const stamp = String(value || '').trim();
    if (!stamp) return '-';
    const date = new Date(stamp);
    return Number.isNaN(date.getTime()) ? stamp : date.toLocaleString();
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native scheduler</p>
      <h2>Schedules and triggers in the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button>
    </div>
  </div>
  {#if error}<div class="banner error">{error}</div>{:else if notice}<div class="banner notice">{notice}</div>{/if}
  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Create schedule</h3></div>
      <div class="form-grid">
        <input bind:value={jobName} class="field" type="text" placeholder="Schedule name" />
        <input bind:value={jobCron} class="field" type="text" placeholder="Cron expression" />
        <select bind:value={jobAgentId} class="field"><option value="">Any agent</option>{#each agents as agent}<option value={agent.id}>{agent.name || agent.id}</option>{/each}</select>
        <textarea bind:value={jobMessage} class="field area" rows="3" placeholder="Run message"></textarea>
        <label><input bind:checked={jobEnabled} type="checkbox" /> Enabled</label>
        <button class="primary small" type="button" disabled={busyKey === 'create-job'} on:click={() => void saveJob()}>{busyKey === 'create-job' ? 'Saving…' : 'Create schedule'}</button>
      </div>
    </article>
    <article class="panel">
      <div class="panel-head"><h3>Schedules</h3></div>
      <div class="rows">
        {#each jobs as job}
          <div class="row">
            <div class="row-copy">
              <strong>{job.name}</strong>
              <span>{job.cron} · next {formatTime(job.next_run)} · last {formatTime(job.last_run)}</span>
            </div>
            <div class="row-actions">
              <button class="ghost small" type="button" on:click={() => setCronJobEnabled(job.id, !job.enabled).then(() => refresh())}>{job.enabled ? 'Pause' : 'Enable'}</button>
              <button class="ghost small" type="button" on:click={() => runCronJobNow(job.id).then((msg) => { notice = msg; refresh(); })}>Run now</button>
              <button class="ghost small" type="button" on:click={() => deleteCronJob(job.id).then((msg) => { notice = msg; refresh(); })}>Delete</button>
            </div>
          </div>
        {/each}
      </div>
    </article>
  </div>
  <article class="panel">
    <div class="panel-head"><h3>Triggers</h3></div>
    <div class="rows">
      {#each triggers as trigger}
        <div class="row">
          <div class="row-copy">
            <strong>{JSON.stringify(trigger.pattern)}</strong>
            <span>{trigger.fire_count} fires · {formatTime(trigger.created_at)}</span>
          </div>
          <div class="row-actions">
            <button class="ghost small" type="button" on:click={() => setTriggerEnabled(trigger.id, !trigger.enabled).then(() => refresh())}>{trigger.enabled ? 'Disable' : 'Enable'}</button>
            <button class="ghost small" type="button" on:click={() => deleteTrigger(trigger.id).then((msg) => { notice = msg; refresh(); })}>Delete</button>
          </div>
        </div>
      {/each}
    </div>
  </article>
</section>

<style>
  .page, .grid, .rows, .form-grid { display: grid; gap: 18px; }
  .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row, .field { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; border-radius: 20px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .field { padding: 0.75rem 0.85rem; font: inherit; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row, .row-actions { flex-direction: column; align-items: flex-start; } }
</style>
