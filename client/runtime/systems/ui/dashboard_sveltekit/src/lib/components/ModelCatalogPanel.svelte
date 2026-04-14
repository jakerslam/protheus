<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DashboardModelRow } from '$lib/chat';

  export let models: DashboardModelRow[] = [];
  export let busyKey = '';
  export let customModelId = '';
  export let customModelProvider = 'openrouter';
  export let customModelContext = 128000;
  export let customModelMaxOutput = 8192;

  const dispatch = createEventDispatcher<{
    addcustom: void;
    deletecustom: { modelId: string };
  }>();

  let modelSearch = '';
  let providerFilter = '';

  $: providerNames = Array.from(new Set(models.map((row) => String(row.provider || '').trim()).filter(Boolean))).sort((a, b) => a.localeCompare(b));
  $: filteredModels = models.filter((row) => {
    const query = modelSearch.trim().toLowerCase();
    const provider = providerFilter.trim().toLowerCase();
    const haystack = `${row.display_name} ${row.id} ${row.provider}`.toLowerCase();
    if (provider && String(row.provider || '').trim().toLowerCase() !== provider) return false;
    return !query || haystack.includes(query);
  });
</script>

<article class="panel">
  <div class="panel-head">
    <h3>Models</h3>
    <div class="filters">
      <input bind:value={modelSearch} class="field" type="text" placeholder="Search models…" />
      <select bind:value={providerFilter} class="field select">
        <option value="">All providers</option>
        {#each providerNames as provider}
          <option value={provider}>{provider}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="custom-card">
    <div class="custom-grid">
      <input bind:value={customModelId} class="field" type="text" placeholder="Model ID" />
      <input bind:value={customModelProvider} class="field" type="text" placeholder="Provider" />
      <input bind:value={customModelContext} class="field" type="number" placeholder="Context window" />
      <input bind:value={customModelMaxOutput} class="field" type="number" placeholder="Max output tokens" />
    </div>
    <button class="primary small" type="button" disabled={busyKey === 'add-custom' || !String(customModelId || '').trim()} on:click={() => dispatch('addcustom')}>
      {busyKey === 'add-custom' ? 'Adding…' : 'Add custom model'}
    </button>
  </div>

  {#if filteredModels.length === 0}
    <div class="empty-card">No models matched this filter.</div>
  {:else}
    <div class="model-list">
      {#each filteredModels.slice(0, 30) as model}
        <div class="model-row">
          <div>
            <strong>{model.display_name || model.id}</strong>
            <p>{model.provider || 'unknown provider'} · {model.id}</p>
          </div>
          {#if String(model.id || '').trim()}
            <button class="ghost small" type="button" disabled={busyKey === `delete-model:${model.id}`} on:click={() => dispatch('deletecustom', { modelId: model.id })}>
              Delete
            </button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</article>

<style>
  .panel,
  .custom-card,
  .model-row,
  .field,
  .ghost,
  .primary {
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
  .panel-head,
  .filters,
  .model-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .custom-card,
  .model-list {
    display: grid;
    gap: 14px;
  }
  .custom-card,
  .model-row,
  .empty-card {
    border-radius: 20px;
    padding: 14px;
  }
  .custom-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .field {
    border-radius: 14px;
    padding: 0.75rem 0.85rem;
    font: inherit;
    min-width: 0;
  }
  .field.select {
    min-width: 180px;
  }
  .ghost,
  .primary {
    border-radius: 16px;
    padding: 0.8rem 1rem;
  }
  .small {
    padding: 0.55rem 0.8rem;
  }
  p {
    color: #8aa4cf;
    margin: 0;
  }
  @media (max-width: 760px) {
    .panel-head,
    .filters,
    .model-row {
      flex-direction: column;
      align-items: flex-start;
    }
    .custom-grid {
      grid-template-columns: 1fr;
    }
    .field,
    .field.select {
      width: 100%;
    }
  }
</style>
