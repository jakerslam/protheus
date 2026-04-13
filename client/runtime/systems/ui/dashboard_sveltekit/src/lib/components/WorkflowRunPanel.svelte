<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DashboardWorkflowRow } from '$lib/workflows';

  export let workflow: DashboardWorkflowRow | null = null;
  export let input = '';
  export let result = '';
  export let busy = false;

  const dispatch = createEventDispatcher<{ run: void; runs: void }>();
</script>

<article class="panel">
  <div class="panel-head"><h3>Run workflow</h3></div>
  {#if !workflow}
    <div class="empty-card">Select a workflow to run it or inspect its history.</div>
  {:else}
    <div class="grid">
      <strong>{workflow.name}</strong>
      <textarea bind:value={input} class="field area" rows="4" placeholder="Workflow input"></textarea>
      <div class="actions">
        <button class="ghost small" type="button" disabled={busy} on:click={() => dispatch('runs')}>View runs</button>
        <button class="primary small" type="button" disabled={busy} on:click={() => dispatch('run')}>{busy ? 'Running…' : 'Run now'}</button>
      </div>
      {#if result}
        <pre>{result}</pre>
      {/if}
    </div>
  {/if}
</article>

<style>
  .panel, .field, .empty-card, pre { border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .panel, .empty-card { border-radius: 24px; padding: 20px; display: grid; gap: 16px; }
  .grid { display: grid; gap: 12px; }
  .field { border-radius: 16px; padding: 0.75rem 0.85rem; font: inherit; }
  .area { min-height: 96px; }
  .actions, .panel-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .small { padding: 0.5rem 0.75rem; }
  pre { border-radius: 20px; padding: 14px; white-space: pre-wrap; overflow: auto; }
</style>
