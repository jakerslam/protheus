<script lang="ts">
  import { dashboardClassicHref } from '$lib/dashboard';
  import { activateHand, checkHandDependencies, deleteHandInstance, pauseHandInstance, readActiveHands, readHandDetail, readHandsCatalog, resumeHandInstance, type DashboardHandInstanceRow, type DashboardHandRow } from '$lib/hands';
  import { onMount } from 'svelte';

  let hands: DashboardHandRow[] = [];
  let instances: DashboardHandInstanceRow[] = [];
  let selectedId = '';
  let detail: DashboardHandRow | null = null;
  let configText = '{}';
  let loading = true;
  let busyKey = '';
  let error = '';
  let notice = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      [hands, instances] = await Promise.all([readHandsCatalog(), readActiveHands()]);
      if (!hands.some((row) => row.id === selectedId)) {
        selectedId = hands[0]?.id || '';
      }
      await refreshDetail();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'hands_unavailable');
    } finally {
      loading = false;
    }
  }

  async function refreshDetail(): Promise<void> {
    if (!selectedId) {
      detail = null;
      return;
    }
    detail = await readHandDetail(selectedId);
  }

  async function selectHand(id: string): Promise<void> {
    selectedId = id;
    configText = '{}';
    await refreshDetail();
  }

  async function verifyDeps(): Promise<void> {
    if (!selectedId) return;
    busyKey = 'deps';
    try {
      const deps = await checkHandDependencies(selectedId);
      detail = detail ? { ...detail, requirements: deps.requirements, requirements_met: deps.requirements_met } : deps;
      notice = deps.requirements_met ? 'All requirements satisfied' : 'Dependency check complete';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'hands_dependency_check_failed');
    } finally {
      busyKey = '';
    }
  }

  async function launchHand(): Promise<void> {
    if (!selectedId) return;
    busyKey = 'activate';
    try {
      let parsedConfig: Record<string, unknown> = {};
      try {
        parsedConfig = JSON.parse(configText || '{}') as Record<string, unknown>;
      } catch {
        throw new Error('Config must be valid JSON');
      }
      notice = await activateHand(selectedId, parsedConfig);
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'hand_activate_failed');
    } finally {
      busyKey = '';
    }
  }

  async function updateInstance(op: 'pause' | 'resume' | 'delete', instanceId: string): Promise<void> {
    busyKey = `${op}:${instanceId}`;
    try {
      notice = op === 'pause'
        ? await pauseHandInstance(instanceId)
        : op === 'resume'
          ? await resumeHandInstance(instanceId)
          : await deleteHandInstance(instanceId);
      instances = await readActiveHands();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || `hand_${op}_failed`);
    } finally {
      busyKey = '';
    }
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native hands</p>
      <h2>Hand catalog, dependency checks, activation, and active runtime instances in the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button>
      <a class="ghost" href={dashboardClassicHref('hands')}>Open classic hands</a>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Catalog</h3><span class="meta">{hands.length} hands</span></div>
      <div class="rows">
        {#each hands as hand}
          <button class:selected={selectedId === hand.id} class="row hand-row" type="button" on:click={() => void selectHand(hand.id)}>
            <div class="row-copy">
              <strong>{hand.name}</strong>
              <span>{hand.description || hand.id}</span>
            </div>
            <span>{hand.requirements_met ? 'Ready' : 'Needs setup'}</span>
          </button>
        {/each}
      </div>
    </article>

    <article class="panel">
      {#if detail}
        <div class="panel-head"><h3>{detail.name}</h3><span class="meta">{detail.requirements.filter((row) => row.satisfied).length}/{detail.requirements.length} ready</span></div>
        <p class="summary">{detail.description}</p>
        <div class="rows">
          {#each detail.requirements as req}
            <div class="row">
              <div class="row-copy">
                <strong>{req.label}</strong>
                <span>{req.message || req.type}</span>
              </div>
              <span>{req.satisfied ? 'OK' : 'Missing'}</span>
            </div>
          {/each}
        </div>
        {#if detail.settings.length}
          <div class="hint-block">
            <strong>Settings hints</strong>
            <ul>
              {#each detail.settings as setting}
                <li>{setting.key}{setting.default_value ? ` = ${setting.default_value}` : ''}</li>
              {/each}
            </ul>
          </div>
        {/if}
        <textarea bind:value={configText} class="field area" rows="7" placeholder={'{"agent_name":"browser-agent"}'}></textarea>
        <div class="row-actions">
          <button class="ghost small" type="button" disabled={busyKey === 'deps'} on:click={() => void verifyDeps()}>{busyKey === 'deps' ? 'Checking…' : 'Check deps'}</button>
          <button class="primary small" type="button" disabled={busyKey === 'activate'} on:click={() => void launchHand()}>{busyKey === 'activate' ? 'Activating…' : 'Activate hand'}</button>
        </div>
      {:else}
        <div class="empty-state">No hands available.</div>
      {/if}
    </article>
  </div>

  <article class="panel">
    <div class="panel-head"><h3>Active instances</h3><span class="meta">{instances.length} running</span></div>
    <div class="rows">
      {#each instances as instance}
        <div class="row">
          <div class="row-copy">
            <strong>{instance.hand_id}</strong>
            <span>{instance.agent_name || instance.agent_id}</span>
          </div>
          <div class="row-actions">
            <span>{instance.status}</span>
            <button class="ghost small" type="button" disabled={busyKey === `pause:${instance.instance_id}`} on:click={() => void updateInstance('pause', instance.instance_id)}>Pause</button>
            <button class="ghost small" type="button" disabled={busyKey === `resume:${instance.instance_id}`} on:click={() => void updateInstance('resume', instance.instance_id)}>Resume</button>
            <button class="ghost small" type="button" disabled={busyKey === `delete:${instance.instance_id}`} on:click={() => void updateInstance('delete', instance.instance_id)}>Remove</button>
          </div>
        </div>
      {/each}
    </div>
  </article>
</section>

<style>
  .page, .grid, .rows { display: grid; gap: 18px; }
  .grid { grid-template-columns: minmax(280px, 0.95fr) minmax(0, 1.25fr); }
  .hero, .panel, .banner, .row, .field, .hint-block { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner, .hint-block { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .hand-row { width: 100%; text-align: left; cursor: pointer; }
  .hand-row.selected { border-color: rgba(158,188,255,0.4); background: rgba(75,120,198,0.14); }
  .field { padding: 0.75rem 0.85rem; font: inherit; width: 100%; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  .summary { margin: 0; color: #dce6ff; }
  .hint-block ul { margin: 10px 0 0; padding-left: 1.1rem; }
  .empty-state { color: #8aa4cf; }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row, .row-actions { flex-direction: column; align-items: flex-start; } }
</style>
