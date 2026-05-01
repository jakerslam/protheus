const COMPONENT_TAG = 'infring-workflows-list-tab-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-workflows-list-tab-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'workflows';
  export let tabId = 'list';
  export let panelRole = 'workflow-tab';
  export let routeContract = 'workflows:list';
  export let parentOwnedData = false;

  const DEFAULT_STEP = { name: '', agent_name: '', mode: 'sequential', prompt: '{{input}}' };
  const modes = ['sequential', 'fan_out', 'conditional', 'loop'];

  let view = {
    workflows: [],
    showCreateModal: false,
    runModal: null,
    runInput: '',
    runResult: '',
    running: false,
    loading: true,
    loadError: '',
    newWf: { name: '', description: '', steps: [{ ...DEFAULT_STEP }] },
    editModal: null,
    editWf: { name: '', description: '', steps: [] }
  };

  function hydrateLegacyViewModel() {
    if (typeof window === 'undefined' || typeof window.workflowsPage !== 'function') return;
    view = window.workflowsPage();
  }

  function repaint() {
    view = view;
  }

  function repaintSoon() {
    setTimeout(repaint, 80);
    setTimeout(repaint, 400);
    setTimeout(repaint, 1200);
  }

  async function call(methodName, ...args) {
    if (!view || typeof view[methodName] !== 'function') return undefined;
    var result = view[methodName].apply(view, args);
    repaint();
    if (result && typeof result.then === 'function') await result;
    repaint();
    repaintSoon();
    return result;
  }

  function switchToBuilder() {
    if (typeof window === 'undefined') return;
    window.dispatchEvent(new CustomEvent('wf-switch-tab', { detail: 'builder' }));
  }

  function showCreateModal() {
    view.showCreateModal = true;
    repaint();
  }

  function closeCreateModal() {
    view.showCreateModal = false;
    repaint();
  }

  function closeRunModal() {
    view.runModal = null;
    repaint();
  }

  function closeEditModal() {
    view.editModal = null;
    repaint();
  }

  function closeTopModal() {
    if (view.showCreateModal) closeCreateModal();
    else if (view.runModal) closeRunModal();
    else if (view.editModal) closeEditModal();
  }

  function handleWindowKeydown(event) {
    if (event.key === 'Escape') closeTopModal();
  }

  function workflowStepsLabel(workflow) {
    var steps = workflow ? workflow.steps : null;
    if (!Array.isArray(steps)) return String(steps || '');
    return steps.length + ' step' + (steps.length === 1 ? '' : 's');
  }

  function createdDate(value) {
    try { return new Date(value).toLocaleDateString(); } catch (_) { return '-'; }
  }

  function addStep(draftKey) {
    var draft = view[draftKey];
    if (!draft) return;
    if (!Array.isArray(draft.steps)) draft.steps = [];
    draft.steps.push({ ...DEFAULT_STEP });
    repaint();
  }

  function removeStep(draftKey, index) {
    var draft = view[draftKey];
    if (!draft || !Array.isArray(draft.steps)) return;
    draft.steps.splice(index, 1);
    repaint();
  }

  function stepKey(prefix, index) {
    return prefix + '-' + String(index);
  }

  onMount(async function() {
    hydrateLegacyViewModel();
    await call('loadWorkflows');
  });
</script>

<svelte:window on:keydown={handleWindowKeydown} />

<div class="page-body">
  {#if view.loading}
    <div class="loading-state"><div class="spinner"></div><span>Loading workflows...</span></div>
  {:else if view.loadError}
    <div class="error-state">
      <span class="error-icon">!</span>
      <p>{view.loadError}</p>
      <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('loadData')}>Retry</button>
    </div>
  {:else}
    <div class="card mb-4" style="border-left:3px solid var(--accent)">
      <div class="font-bold" style="font-size:13px;margin-bottom:4px">What are Workflows?</div>
      <div class="text-sm text-dim" style="line-height:1.6">
        Workflows chain multiple agents into automated pipelines. Each step runs an agent with a prompt template,
        passing output from one step as input to the next. Steps can run sequentially, fan out in parallel, loop, or branch conditionally.
        <br><span style="margin-top:4px;display:inline-block">Try the <strong style="color:var(--accent);cursor:pointer" on:click={switchToBuilder}>Visual Builder</strong> to drag and drop workflow steps.</span>
      </div>
    </div>
    <div class="flex gap-2 mb-4"><button class="btn btn-primary btn-sm" type="button" on:click={showCreateModal}>+ New Workflow</button></div>

    {#if view.workflows.length}
      <div class="table-wrap">
        <table>
          <thead><tr><th>Name</th><th>Steps</th><th>Created</th><th>Actions</th></tr></thead>
          <tbody>
            {#each view.workflows as workflow (workflow.id)}
              <tr>
                <td><span class="font-bold">{workflow.name}</span><br><span class="text-xs text-dim">{workflow.description}</span></td>
                <td>{workflowStepsLabel(workflow)}</td>
                <td class="text-xs">{createdDate(workflow.created_at)}</td>
                <td>
                  <button class="btn btn-primary btn-sm" type="button" on:click={() => call('showRunModal', workflow)}>Run</button>
                  <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('showEditModal', workflow)}>Edit</button>
                  <button class="btn btn-ghost btn-sm" type="button" on:click={() => call('viewRuns', workflow)}>History</button>
                  <button class="btn btn-danger btn-sm" type="button" on:click={() => call('deleteWorkflow', workflow)}>Delete</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else}
      <infring-chat-stream-shell class="empty-state">
        <div class="empty-state-icon"><svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 3v12M18 9a9 9 0 0 1-9 9"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/></svg></div>
        <h3>No workflows yet</h3>
        <p>Chain multiple agents into automated pipelines with branching, fan-out, and loops.</p>
        <button class="btn btn-primary" type="button" on:click={showCreateModal}>Create Workflow</button>
      </infring-chat-stream-shell>
    {/if}
  {/if}

  {#if view.showCreateModal}
    <div class="modal-overlay" on:click={(event) => { if (event.currentTarget === event.target) closeCreateModal(); }}>
      <div class="modal">
        <div class="modal-header"><h3>Create Workflow</h3><button class="modal-close" type="button" on:click={closeCreateModal}>&times;</button></div>
        <div class="form-group"><label>Name</label><input class="form-input" bind:value={view.newWf.name} placeholder="my-workflow"></div>
        <div class="form-group"><label>Description</label><input class="form-input" bind:value={view.newWf.description} placeholder="What does this workflow do?"></div>
        <div class="mb-4">
          <div class="form-group" style="margin:0"><label>Steps</label></div>
          <div class="text-xs text-dim mb-2">Each step runs an agent. Use <code style="color:var(--accent)">&#123;&#123;input&#125;&#125;</code> in prompts to pass the previous step's output.</div>
          {#each view.newWf.steps as step, index (stepKey('new', index))}
            <div class="card mt-2" style="padding:10px">
              <div class="flex gap-2 items-center">
                <span class="text-xs text-dim font-bold" style="width:24px">#{index + 1}</span>
                <input class="form-input" style="flex:1" bind:value={step.name} placeholder="Step name">
                <input class="form-input" style="flex:1" bind:value={step.agent_name} placeholder="Agent name">
                <select class="form-select" style="width:120px" bind:value={step.mode}>
                  {#each modes as mode}<option value={mode}>{mode === 'fan_out' ? 'Fan Out' : mode.charAt(0).toUpperCase() + mode.slice(1)}</option>{/each}
                </select>
                <button class="btn btn-danger btn-sm" type="button" on:click={() => removeStep('newWf', index)}>&times;</button>
              </div>
              <input class="form-input mt-2" bind:value={step.prompt} placeholder="Prompt template (use &#123;&#123;input&#125;&#125;)">
            </div>
          {/each}
          <button class="btn btn-ghost btn-sm mt-2" type="button" on:click={() => addStep('newWf')}>+ Add Step</button>
        </div>
        <button class="btn btn-primary btn-block" type="button" on:click={() => call('createWorkflow')}>Create</button>
      </div>
    </div>
  {/if}

  {#if view.runModal}
    <div class="modal-overlay" on:click={(event) => { if (event.currentTarget === event.target) closeRunModal(); }}>
      <div class="modal">
        <div class="modal-header"><h3>Run: {view.runModal.name}</h3><button class="modal-close" type="button" on:click={closeRunModal}>&times;</button></div>
        <div class="form-group"><label>Input</label><textarea class="form-textarea" bind:value={view.runInput} placeholder="Enter workflow input..."></textarea></div>
        <button class="btn btn-primary btn-block" type="button" disabled={view.running} on:click={() => call('executeWorkflow')}>{view.running ? 'Running...' : 'Execute'}</button>
        {#if view.runResult}
          <div class="card mt-4"><div class="card-header">Result</div><pre style="font-size:11px;white-space:pre-wrap;margin-top:8px;color:var(--text-dim)">{view.runResult}</pre></div>
        {/if}
      </div>
    </div>
  {/if}

  {#if view.editModal}
    <div class="modal-overlay" on:click={(event) => { if (event.currentTarget === event.target) closeEditModal(); }}>
      <div class="modal">
        <div class="modal-header"><h3>Edit: {view.editModal.name}</h3><button class="modal-close" type="button" on:click={closeEditModal}>&times;</button></div>
        <div class="form-group"><label>Name</label><input class="form-input" bind:value={view.editWf.name} placeholder="Workflow name"></div>
        <div class="form-group"><label>Description</label><input class="form-input" bind:value={view.editWf.description} placeholder="What does this workflow do?"></div>
        <div class="mb-4">
          <div class="form-group" style="margin:0"><label>Steps</label></div>
          <div class="text-xs text-dim mb-2">Each step runs an agent. Use <code style="color:var(--accent)">&#123;&#123;input&#125;&#125;</code> in prompts to pass the previous step's output.</div>
          {#each view.editWf.steps as step, index (stepKey('edit', index))}
            <div class="card mt-2" style="padding:10px">
              <div class="flex gap-2 items-center">
                <span class="text-xs text-dim font-bold" style="width:24px">#{index + 1}</span>
                <input class="form-input" style="flex:1" bind:value={step.name} placeholder="Step name">
                <input class="form-input" style="flex:1" bind:value={step.agent_name} placeholder="Agent name">
                <select class="form-select" style="width:120px" bind:value={step.mode}>
                  {#each modes as mode}<option value={mode}>{mode === 'fan_out' ? 'Fan Out' : mode.charAt(0).toUpperCase() + mode.slice(1)}</option>{/each}
                </select>
                <button class="btn btn-danger btn-sm" type="button" on:click={() => removeStep('editWf', index)}>&times;</button>
              </div>
              <input class="form-input mt-2" bind:value={step.prompt} placeholder="Prompt template (use &#123;&#123;input&#125;&#125;)">
            </div>
          {/each}
          <button class="btn btn-ghost btn-sm mt-2" type="button" on:click={() => addStep('editWf')}>+ Add Step</button>
        </div>
        <button class="btn btn-primary btn-block" type="button" on:click={() => call('saveWorkflow')}>Save Changes</button>
      </div>
    </div>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
