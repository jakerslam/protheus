<script lang="ts">
  import { dashboardClassicHref } from '$lib/dashboard';
  import { postCommsTask, readCommsSnapshot, sendCommsMessage, type DashboardCommsSnapshot } from '$lib/comms';
  import { onDestroy, onMount } from 'svelte';

  let snapshot: DashboardCommsSnapshot | null = null;
  let sendFrom = '';
  let sendTo = '';
  let sendMessage = '';
  let taskTitle = '';
  let taskDesc = '';
  let taskAssign = '';
  let busyKey = '';
  let loading = true;
  let error = '';
  let notice = '';
  let pollHandle: ReturnType<typeof setInterval> | null = null;

  onMount(async () => {
    await refresh();
    pollHandle = setInterval(() => void refresh(), 5000);
  });

  onDestroy(() => {
    if (pollHandle) clearInterval(pollHandle);
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      snapshot = await readCommsSnapshot();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'comms_unavailable');
    } finally {
      loading = false;
    }
  }

  async function send(): Promise<void> {
    if (!sendFrom || !sendTo || !sendMessage.trim()) return;
    busyKey = 'send';
    try {
      notice = await sendCommsMessage({ from_agent_id: sendFrom, to_agent_id: sendTo, message: sendMessage.trim() });
      sendMessage = '';
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'send_failed');
    } finally {
      busyKey = '';
    }
  }

  async function postTask(): Promise<void> {
    if (!taskTitle.trim()) return;
    busyKey = 'task';
    try {
      notice = await postCommsTask({ title: taskTitle.trim(), description: taskDesc.trim(), assigned_to: taskAssign });
      taskTitle = '';
      taskDesc = '';
      taskAssign = '';
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'task_failed');
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
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native comms</p>
      <h2>Agent topology, recent events, and lightweight coordination in the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button>
      <a class="ghost" href={dashboardClassicHref('comms')}>Open classic comms</a>
    </div>
  </div>
  {#if error}<div class="banner error">{error}</div>{:else if notice}<div class="banner notice">{notice}</div>{/if}
  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Agent topology</h3><span class="meta">{snapshot?.nodes.length || 0} nodes</span></div>
      <div class="rows">
        {#each snapshot?.nodes || [] as node}
          <div class="row">
            <div class="row-copy"><strong>{node.name}</strong><span>{node.id}</span></div>
            <span>{node.state}</span>
          </div>
        {/each}
      </div>
    </article>
    <article class="panel">
      <div class="panel-head"><h3>Recent events</h3><span class="meta">{snapshot?.events.length || 0}</span></div>
      <div class="rows">
        {#each (snapshot?.events || []).slice(0, 30) as event}
          <div class="row">
            <div class="row-copy"><strong>{event.kind}</strong><span>{event.title}</span></div>
            <span>{formatRelativeTime(event.ts)}</span>
          </div>
        {/each}
      </div>
    </article>
  </div>
  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Send message</h3></div>
      <div class="form-grid">
        <select bind:value={sendFrom} class="field"><option value="">From agent</option>{#each snapshot?.agents || [] as agent}<option value={agent.id}>{agent.name || agent.id}</option>{/each}</select>
        <select bind:value={sendTo} class="field"><option value="">To agent</option>{#each snapshot?.agents || [] as agent}<option value={agent.id}>{agent.name || agent.id}</option>{/each}</select>
        <textarea bind:value={sendMessage} class="field area" rows="4" placeholder="Message"></textarea>
        <button class="primary small" type="button" disabled={busyKey === 'send'} on:click={() => void send()}>{busyKey === 'send' ? 'Sending…' : 'Send'}</button>
      </div>
    </article>
    <article class="panel">
      <div class="panel-head"><h3>Post task</h3></div>
      <div class="form-grid">
        <input bind:value={taskTitle} class="field" type="text" placeholder="Task title" />
        <select bind:value={taskAssign} class="field"><option value="">Unassigned</option>{#each snapshot?.agents || [] as agent}<option value={agent.id}>{agent.name || agent.id}</option>{/each}</select>
        <textarea bind:value={taskDesc} class="field area" rows="4" placeholder="Task description"></textarea>
        <button class="primary small" type="button" disabled={busyKey === 'task'} on:click={() => void postTask()}>{busyKey === 'task' ? 'Posting…' : 'Post task'}</button>
      </div>
    </article>
  </div>
</section>

<style>
  .page, .grid, .rows, .form-grid { display: grid; gap: 18px; }
  .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row, .field { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; border-radius: 20px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .field { padding: 0.75rem 0.85rem; font: inherit; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row { flex-direction: column; align-items: flex-start; } }
</style>
