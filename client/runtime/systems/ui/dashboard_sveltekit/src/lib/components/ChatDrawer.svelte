<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DashboardAgentRow, DashboardModelRow } from '$lib/chat';

  export let open = false;
  export let agent: DashboardAgentRow | null = null;
  export let models: DashboardModelRow[] = [];
  export let loadingModels = false;
  export let connectionState = 'disconnected';
  export let busyKey = '';
  export let nameDraft = '';
  export let modelDraft = '';

  const dispatch = createEventDispatcher<{
    close: void;
    refreshmodels: void;
    savename: void;
    savemodel: void;
    compact: void;
    reset: void;
    stop: void;
  }>();

  function formatTimestamp(value: string | undefined): string {
    const ts = Date.parse(String(value || ''));
    if (!Number.isFinite(ts)) return 'Unknown';
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    }).format(ts);
  }
</script>

{#if open}
  <aside class="drawer">
    <div class="drawer-head">
      <div>
        <p class="eyebrow">Conversation details</p>
        <h3>{agent ? String(agent.name || agent.id || 'Conversation') : 'No conversation selected'}</h3>
      </div>
      <button class="ghost" type="button" on:click={() => dispatch('close')}>Close</button>
    </div>

    {#if agent}
      <div class="card">
        <label>
          <span>Name</span>
          <input bind:value={nameDraft} type="text" maxlength="80" />
        </label>
        <button class="primary" type="button" disabled={busyKey !== '' || !nameDraft.trim()} on:click={() => dispatch('savename')}>
          {busyKey === 'name' ? 'Saving…' : 'Save name'}
        </button>
      </div>

      <div class="card">
        <div class="card-head">
          <strong>Model</strong>
          <button class="ghost small" type="button" on:click={() => dispatch('refreshmodels')} disabled={loadingModels}>
            {loadingModels ? 'Refreshing…' : 'Refresh models'}
          </button>
        </div>
        <label>
          <span>Current</span>
          <select bind:value={modelDraft}>
            <option value="">Server default</option>
            {#each models as model}
              <option value={model.id}>{model.provider ? `${model.provider}/${model.id}` : model.id}</option>
            {/each}
          </select>
        </label>
        <button class="primary" type="button" disabled={busyKey !== '' || !modelDraft.trim()} on:click={() => dispatch('savemodel')}>
          {busyKey === 'model' ? 'Switching…' : 'Switch model'}
        </button>
      </div>

      <div class="card">
        <strong>Power actions</strong>
        <div class="action-grid">
          <button class="ghost" type="button" disabled={busyKey !== ''} on:click={() => dispatch('compact')}>
            {busyKey === 'compact' ? 'Compacting…' : 'Compact session'}
          </button>
          <button class="ghost" type="button" disabled={busyKey !== ''} on:click={() => dispatch('reset')}>
            {busyKey === 'reset' ? 'Resetting…' : 'Reset session'}
          </button>
          <button class="danger" type="button" disabled={busyKey !== ''} on:click={() => dispatch('stop')}>
            {busyKey === 'stop' ? 'Stopping…' : 'Stop agent'}
          </button>
        </div>
      </div>

      <div class="card facts">
        <div><span>State</span><strong>{String(agent.state || 'running')}</strong></div>
        <div><span>Connection</span><strong>{connectionState}</strong></div>
        <div><span>Created</span><strong>{formatTimestamp(agent.created_at)}</strong></div>
        <div><span>Last activity</span><strong>{formatTimestamp(agent.last_activity_at || agent.updated_at)}</strong></div>
      </div>
    {:else}
      <div class="card">Select a conversation to use native controls.</div>
    {/if}
  </aside>
{/if}

<style>
  .drawer {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
    padding: 18px;
    display: grid;
    align-content: start;
    gap: 14px;
  }

  .drawer-head,
  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .eyebrow,
  label span,
  .facts span {
    color: #8aa4cf;
  }

  h3,
  p {
    margin: 0;
  }

  .card,
  input,
  select,
  .ghost,
  .primary,
  .danger {
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
  }

  .card {
    border-radius: 20px;
    padding: 14px;
    display: grid;
    gap: 12px;
  }

  label {
    display: grid;
    gap: 8px;
  }

  input,
  select {
    border-radius: 14px;
    padding: 0.75rem 0.85rem;
    font: inherit;
  }

  .action-grid {
    display: grid;
    gap: 10px;
  }

  .ghost,
  .primary,
  .danger {
    border-radius: 16px;
    padding: 0.8rem 1rem;
    cursor: pointer;
  }

  .primary {
    background: rgba(40, 79, 138, 0.28);
  }

  .danger {
    background: rgba(128, 34, 27, 0.35);
    border-color: rgba(229, 112, 93, 0.24);
  }

  .small {
    padding: 0.55rem 0.8rem;
  }

  .facts {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .facts div {
    display: grid;
    gap: 4px;
  }
</style>
