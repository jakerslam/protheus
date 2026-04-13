<script lang="ts">
  import { deleteAgentMemoryKv, deleteSession, readAgentMemoryKv, readSessions, upsertAgentMemoryKv, type DashboardMemoryKvRow, type DashboardSessionRow } from '$lib/sessions';
  import { onMount } from 'svelte';

  let sessions: DashboardSessionRow[] = [];
  let selectedAgentId = '';
  let kvPairs: DashboardMemoryKvRow[] = [];
  let search = '';
  let keyDraft = '';
  let valueDraft = '""';
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
      sessions = await readSessions();
      if (!selectedAgentId && sessions[0]?.agent_id) {
        selectedAgentId = sessions[0].agent_id;
        await refreshMemory();
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'sessions_unavailable');
    } finally {
      loading = false;
    }
  }

  async function refreshMemory(): Promise<void> {
    if (!selectedAgentId) {
      kvPairs = [];
      return;
    }
    kvPairs = await readAgentMemoryKv(selectedAgentId);
  }

  async function removeSession(sessionId: string): Promise<void> {
    busyKey = `session:${sessionId}`;
    try {
      notice = await deleteSession(sessionId);
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'session_delete_failed');
    } finally {
      busyKey = '';
    }
  }

  async function saveMemoryKey(): Promise<void> {
    if (!selectedAgentId || !keyDraft.trim()) return;
    busyKey = `memory:${keyDraft}`;
    try {
      let value: unknown;
      try { value = JSON.parse(valueDraft); } catch { value = valueDraft; }
      notice = await upsertAgentMemoryKv(selectedAgentId, keyDraft.trim(), value);
      keyDraft = '';
      valueDraft = '""';
      await refreshMemory();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'memory_save_failed');
    } finally {
      busyKey = '';
    }
  }

  async function removeMemoryKey(key: string): Promise<void> {
    busyKey = `memory-delete:${key}`;
    try {
      notice = await deleteAgentMemoryKv(selectedAgentId, key);
      await refreshMemory();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'memory_delete_failed');
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

  $: filteredSessions = sessions.filter((row) => {
    const q = search.trim().toLowerCase();
    return !q || `${row.agent_name} ${row.agent_id}`.toLowerCase().includes(q);
  });
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native sessions</p>
      <h2>Session inventory and agent memory keys without the legacy page.</h2>
    </div>
    <div class="hero-actions">
      <input bind:value={search} class="field" type="text" placeholder="Search sessions…" />
    </div>
  </div>
  {#if error}<div class="banner error">{error}</div>{:else if notice}<div class="banner notice">{notice}</div>{/if}
  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Sessions</h3><button class="ghost small" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button></div>
      <div class="rows">
        {#each filteredSessions as session}
          <div class="row">
            <div class="row-copy">
              <strong>{session.agent_name || session.agent_id}</strong>
              <span>{session.session_id} · {formatRelativeTime(session.updated_at)}</span>
            </div>
            <div class="row-actions">
              <button class="ghost small" type="button" on:click={() => { selectedAgentId = session.agent_id; void refreshMemory(); }}>Memory</button>
              <button class="ghost small" type="button" disabled={busyKey === `session:${session.session_id}`} on:click={() => void removeSession(session.session_id)}>Delete</button>
            </div>
          </div>
        {/each}
      </div>
    </article>
    <article class="panel">
      <div class="panel-head"><h3>Agent memory</h3><span class="meta">{selectedAgentId || 'select a session'}</span></div>
      <div class="memory-form">
        <input bind:value={keyDraft} class="field" type="text" placeholder="Key" />
        <textarea bind:value={valueDraft} class="field area" rows="4" placeholder='""'></textarea>
        <button class="primary small" type="button" disabled={!selectedAgentId || busyKey === `memory:${keyDraft}`} on:click={() => void saveMemoryKey()}>Save key</button>
      </div>
      <div class="rows">
        {#each kvPairs as pair}
          <div class="row">
            <div class="row-copy">
              <strong>{pair.key}</strong>
              <span>{JSON.stringify(pair.value)}</span>
            </div>
            <button class="ghost small" type="button" disabled={busyKey === `memory-delete:${pair.key}`} on:click={() => void removeMemoryKey(pair.key)}>Delete</button>
          </div>
        {/each}
      </div>
    </article>
  </div>
</section>

<style>
  .page, .grid, .rows, .memory-form { display: grid; gap: 18px; }
  .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row, .field { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; border-radius: 20px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .field { padding: 0.75rem 0.85rem; font: inherit; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row, .row-actions { flex-direction: column; align-items: flex-start; } }
</style>
