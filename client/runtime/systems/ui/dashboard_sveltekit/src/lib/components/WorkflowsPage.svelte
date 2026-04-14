<script lang="ts">
  import WorkflowEditorPanel from '$lib/components/WorkflowEditorPanel.svelte';
  import WorkflowRunPanel from '$lib/components/WorkflowRunPanel.svelte';
  import { createWorkflow, deleteWorkflow, readWorkflow, readWorkflowRuns, readWorkflows, runWorkflow, updateWorkflow, type DashboardWorkflowRow, type WorkflowStepInput } from '$lib/workflows';
  import { onMount } from 'svelte';

  let workflows: DashboardWorkflowRow[] = [];
  let selected: DashboardWorkflowRow | null = null;
  let formName = '';
  let formDescription = '';
  let formSteps: WorkflowStepInput[] = [{ name: '', agent_name: '', mode: 'sequential', prompt: '{{input}}' }];
  let runInput = '';
  let runResult = '';
  let loading = true;
  let saving = false;
  let error = '';
  let notice = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      workflows = await readWorkflows();
      if (selected) {
        const match = workflows.find((row) => row.id === selected?.id) || null;
        selected = match;
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'workflows_unavailable');
    } finally {
      loading = false;
    }
  }

  function resetForm(): void {
    formName = '';
    formDescription = '';
    formSteps = [{ name: '', agent_name: '', mode: 'sequential', prompt: '{{input}}' }];
  }

  async function editWorkflow(row: DashboardWorkflowRow): Promise<void> {
    selected = await readWorkflow(row.id);
    formName = selected.name;
    formDescription = selected.description;
    formSteps = selected.steps.map((step) => ({ ...step }));
  }

  async function save(): Promise<void> {
    saving = true;
    try {
      const payload = { name: formName.trim(), description: formDescription.trim(), steps: formSteps.map((step) => ({ ...step, name: step.name || 'step', prompt: step.prompt || '{{input}}' })) };
      notice = selected ? await updateWorkflow(selected.id, payload) : await createWorkflow(payload);
      selected = null;
      resetForm();
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'workflow_save_failed');
    } finally {
      saving = false;
    }
  }

  async function remove(row: DashboardWorkflowRow): Promise<void> {
    saving = true;
    try {
      notice = await deleteWorkflow(row.id);
      if (selected?.id === row.id) selected = null;
      resetForm();
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'workflow_delete_failed');
    } finally {
      saving = false;
    }
  }

  async function runSelected(): Promise<void> {
    if (!selected) return;
    saving = true;
    try {
      runResult = await runWorkflow(selected.id, runInput);
      notice = `Ran ${selected.name}`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'workflow_run_failed');
    } finally {
      saving = false;
    }
  }

  async function viewRuns(): Promise<void> {
    if (!selected) return;
    saving = true;
    try {
      runResult = await readWorkflowRuns(selected.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'workflow_runs_failed');
    } finally {
      saving = false;
    }
  }

  function addStep(): void {
    formSteps = [...formSteps, { name: '', agent_name: '', mode: 'sequential', prompt: '{{input}}' }];
  }

  function removeStep(index: number): void {
    formSteps = formSteps.filter((_, idx) => idx !== index);
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native workflows</p>
      <h2>Define, edit, run, and inspect workflows in the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => { selected = null; resetForm(); }}>New workflow</button>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <div class="content-grid">
    <article class="panel">
      <div class="panel-head"><h3>Workflow library</h3><button class="ghost small" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button></div>
      <div class="rows">
        {#each workflows as row}
          <div class="row">
            <div class="row-copy">
              <strong>{row.name}</strong>
              <span>{row.steps.length} steps · {row.description || 'No description'}</span>
            </div>
            <div class="row-actions">
              <button class="ghost small" type="button" on:click={() => void editWorkflow(row)}>Edit</button>
              <button class="ghost small" type="button" on:click={() => { selected = row; runResult = ''; runInput = ''; }}>Run</button>
              <button class="ghost small" type="button" disabled={saving} on:click={() => void remove(row)}>Delete</button>
            </div>
          </div>
        {/each}
      </div>
    </article>

    <WorkflowEditorPanel
      title={selected ? `Edit ${selected.name}` : 'Create workflow'}
      bind:name={formName}
      bind:description={formDescription}
      bind:steps={formSteps}
      busy={saving}
      on:submit={() => void save()}
      on:addstep={() => addStep()}
      on:removestep={(event) => removeStep(event.detail.index)}
    />
  </div>

  <WorkflowRunPanel workflow={selected} bind:input={runInput} bind:result={runResult} busy={saving} on:run={() => void runSelected()} on:runs={() => void viewRuns()} />
</section>

<style>
  .page, .content-grid, .rows { display: grid; gap: 18px; }
  .content-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; border-radius: 20px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .ghost { padding: 0.8rem 1rem; border-radius: 16px; text-decoration: none; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  @media (max-width: 980px) {
    .content-grid { grid-template-columns: 1fr; }
  }
  @media (max-width: 760px) {
    .hero, .hero-actions, .row, .row-actions { flex-direction: column; align-items: flex-start; }
  }
</style>
