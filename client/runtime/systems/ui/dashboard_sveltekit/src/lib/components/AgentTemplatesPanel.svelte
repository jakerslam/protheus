<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DashboardTemplateRow } from '$lib/agents';

  export let templates: DashboardTemplateRow[] = [];
  export let busyKey = '';

  let searchQuery = '';

  const dispatch = createEventDispatcher<{ spawn: { templateName: string } }>();

  $: filteredTemplates = templates.filter((row) => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) return true;
    return `${row.name} ${row.description} ${row.category}`.toLowerCase().includes(query);
  });
</script>

<article class="panel">
  <div class="panel-head">
    <h3>Templates</h3>
    <input bind:value={searchQuery} class="search" type="text" placeholder="Search templates…" />
  </div>
  <div class="template-list">
    {#if filteredTemplates.length === 0}
      <div class="empty-card">No templates matched this search.</div>
    {:else}
      {#each filteredTemplates.slice(0, 8) as template}
        <div class="template-row">
          <div>
            <strong>{template.name}</strong>
            <p>{template.description || 'No description provided.'}</p>
            <span>{template.category}</span>
          </div>
          <button class="ghost small" type="button" disabled={busyKey === `template:${template.name}`} on:click={() => dispatch('spawn', { templateName: template.name })}>
            {busyKey === `template:${template.name}` ? 'Spawning…' : 'Spawn'}
          </button>
        </div>
      {/each}
    {/if}
  </div>
</article>

<style>
  .panel,
  .search,
  .template-row {
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
  .template-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  h3,
  p {
    margin: 0;
  }

  .template-list {
    display: grid;
    gap: 18px;
  }

  .search {
    border-radius: 14px;
    padding: 0.75rem 0.85rem;
    font: inherit;
    min-width: 220px;
  }

  .template-row,
  .empty-card {
    border-radius: 20px;
    padding: 14px;
  }

  .template-row > div {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .template-row span {
    color: #8aa4cf;
  }

  .ghost {
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
    border-radius: 16px;
    padding: 0.8rem 1rem;
  }

  .small {
    padding: 0.55rem 0.8rem;
  }

  .empty-card {
    background: rgba(255, 255, 255, 0.03);
  }

  @media (max-width: 760px) {
    .panel-head,
    .template-row {
      flex-direction: column;
      align-items: flex-start;
    }

    .search {
      min-width: 0;
      width: 100%;
    }
  }
</style>
