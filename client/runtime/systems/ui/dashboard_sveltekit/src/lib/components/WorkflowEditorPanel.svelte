<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { WorkflowStepInput } from '$lib/workflows';

  export let title = 'Create workflow';
  export let name = '';
  export let description = '';
  export let steps: WorkflowStepInput[] = [];
  export let busy = false;

  const dispatch = createEventDispatcher<{
    submit: void;
    addstep: void;
    removestep: { index: number };
  }>();
</script>

<article class="panel">
  <div class="panel-head"><h3>{title}</h3></div>
  <div class="grid">
    <input bind:value={name} class="field" type="text" placeholder="Workflow name" />
    <textarea bind:value={description} class="field area" rows="3" placeholder="What does this workflow do?"></textarea>
    {#each steps as step, index}
      <div class="step-card">
        <input bind:value={step.name} class="field" type="text" placeholder="Step name" />
        <input bind:value={step.agent_name} class="field" type="text" placeholder="Agent name" />
        <select bind:value={step.mode} class="field">
          <option value="sequential">sequential</option>
          <option value="parallel">parallel</option>
        </select>
        <textarea bind:value={step.prompt} class="field area" rows="3" placeholder="{{input}}"></textarea>
        {#if steps.length > 1}
          <button class="ghost small" type="button" on:click={() => dispatch('removestep', { index })}>Remove step</button>
        {/if}
      </div>
    {/each}
    <div class="actions">
      <button class="ghost small" type="button" on:click={() => dispatch('addstep')}>Add step</button>
      <button class="primary small" type="button" disabled={busy || !String(name || '').trim()} on:click={() => dispatch('submit')}>{busy ? 'Saving…' : 'Save workflow'}</button>
    </div>
  </div>
</article>

<style>
  .panel, .field, .step-card { border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .panel, .step-card { border-radius: 24px; padding: 20px; display: grid; gap: 16px; }
  .grid { display: grid; gap: 12px; }
  .field { border-radius: 16px; padding: 0.75rem 0.85rem; font: inherit; }
  .area { min-height: 84px; }
  .actions, .panel-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .small { padding: 0.5rem 0.75rem; }
</style>
