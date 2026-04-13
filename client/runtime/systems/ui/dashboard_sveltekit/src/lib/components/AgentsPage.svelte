<script lang="ts">
  import { onMount } from 'svelte';
  import { dashboardPageHref } from '$lib/dashboard';
  import type { DashboardAgentRow, DashboardModelRow } from '$lib/chat';
  import { createDraftAgent, readModels, readSidebarAgents, updateAgentConfig, updateAgentModel } from '$lib/chat';
  import type { DashboardTemplateRow, DashboardTerminatedAgentRow } from '$lib/agents';
  import {
    archiveAgent,
    clearAgentHistory,
    cloneAgent,
    deleteTerminatedAgent,
    readTemplates,
    readTerminatedAgents,
    reviveTerminatedAgent,
    spawnTemplateAgent,
  } from '$lib/agents';
  import AgentDetailPanel from '$lib/components/AgentDetailPanel.svelte';
  import AgentTemplatesPanel from '$lib/components/AgentTemplatesPanel.svelte';

  let agents: DashboardAgentRow[] = [];
  let archived: DashboardTerminatedAgentRow[] = [];
  let templates: DashboardTemplateRow[] = [];
  let models: DashboardModelRow[] = [];
  let selectedAgentId = '';
  let notice = '';
  let error = '';
  let loading = true;
  let loadingModels = false;
  let busyKey = '';
  let nameDraft = '';
  let modelDraft = '';

  $: selectedAgent = agents.find((row) => row.id === selectedAgentId) || null;
  $: if (selectedAgent) {
    nameDraft = nameDraft || String(selectedAgent.name || '').trim();
    modelDraft = modelDraft || String(selectedAgent.runtime_model || selectedAgent.model_name || '').trim();
  }

  onMount(async () => {
    await Promise.all([refreshAgents(), refreshArchived(), refreshTemplates(), loadModels()]);
  });

  function setNotice(text: string): void { notice = String(text || '').trim(); error = ''; }

  function setErrorMessage(text: string): void { error = String(text || '').trim(); notice = ''; }

  function resetDrafts(agent: DashboardAgentRow | null): void {
    nameDraft = String(agent?.name || '').trim();
    modelDraft = String(agent?.runtime_model || agent?.model_name || '').trim();
  }

  async function refreshAgents(preferredId = ''): Promise<void> {
    try {
      const rows = await readSidebarAgents();
      agents = rows;
      const nextId = preferredId && rows.some((row) => row.id === preferredId) ? preferredId : (rows[0]?.id || '');
      selectedAgentId = nextId;
      resetDrafts(rows.find((row) => row.id === nextId) || null);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'agents_unavailable'));
    } finally {
      loading = false;
    }
  }

  async function refreshArchived(): Promise<void> {
    try {
      archived = await readTerminatedAgents();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'archived_unavailable'));
    }
  }

  async function refreshTemplates(): Promise<void> {
    try {
      templates = await readTemplates();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'templates_unavailable'));
    }
  }

  async function loadModels(): Promise<void> {
    loadingModels = true;
    try {
      models = await readModels();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'models_unavailable'));
    } finally {
      loadingModels = false;
    }
  }

  async function handleCreateDraft(): Promise<void> {
    busyKey = 'draft';
    try {
      const created = await createDraftAgent();
      setNotice(`Draft chat created for ${created.name || created.id}.`);
      await refreshAgents(created.id);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'draft_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleSpawnTemplate(templateName: string): Promise<void> {
    busyKey = `template:${templateName}`;
    try {
      const createdId = await spawnTemplateAgent(templateName);
      setNotice(`Spawned ${templateName} as ${createdId}.`);
      await refreshAgents(createdId);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'template_spawn_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleSaveName(): Promise<void> {
    if (!selectedAgent || !nameDraft.trim() || busyKey) return;
    busyKey = 'name';
    try {
      await updateAgentConfig(selectedAgent.id, { name: nameDraft.trim() });
      setNotice('Agent name updated.');
      await refreshAgents(selectedAgent.id);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'name_update_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleSaveModel(): Promise<void> {
    if (!selectedAgent || !modelDraft.trim() || busyKey) return;
    busyKey = 'model';
    try {
      await updateAgentModel(selectedAgent.id, modelDraft.trim());
      setNotice('Model switched. Backend reset session memory as part of the contract.');
      await refreshAgents(selectedAgent.id);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'model_update_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleClone(): Promise<void> {
    if (!selectedAgent || busyKey) return;
    busyKey = 'clone';
    try {
      const cloneName = await cloneAgent(selectedAgent);
      setNotice(`Cloned agent as ${cloneName}.`);
      await refreshAgents();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'clone_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleClearHistory(): Promise<void> {
    if (!selectedAgent || busyKey) return;
    busyKey = 'history';
    try {
      setNotice(await clearAgentHistory(selectedAgent.id));
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'history_clear_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleArchiveSelected(): Promise<void> {
    if (!selectedAgent || busyKey) return;
    busyKey = 'archive';
    try {
      setNotice(await archiveAgent(selectedAgent));
      await Promise.all([refreshAgents(), refreshArchived()]);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'archive_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleRevive(entry: DashboardTerminatedAgentRow): Promise<void> {
    busyKey = `revive:${entry.agent_id}`;
    try {
      setNotice(`Revived ${await reviveTerminatedAgent(entry)}.`);
      await Promise.all([refreshAgents(entry.agent_id), refreshArchived()]);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'revive_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleDeleteArchived(entry: DashboardTerminatedAgentRow): Promise<void> {
    busyKey = `delete:${entry.agent_id}`;
    try {
      setNotice(await deleteTerminatedAgent(entry));
      await refreshArchived();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'delete_failed'));
    } finally {
      busyKey = '';
    }
  }

  function formatTimestamp(value: string): string {
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

<section class="agents-page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native agents</p>
      <h2>Roster, lifecycle, and spawning without the classic fallback.</h2>
      <p class="hero-copy">
        This first native agents slice covers the authoritative roster, archived-agent lifecycle, template spawning, and the highest-value detail controls.
      </p>
    </div>
    <div class="hero-actions">
      <button class="primary" type="button" on:click={() => void handleCreateDraft()} disabled={busyKey === 'draft'}>
        {busyKey === 'draft' ? 'Creating…' : 'New draft chat'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <div class="content-grid">
    <div class="column">
      <article class="panel">
        <div class="panel-head">
          <h3>Active roster</h3>
          <button class="ghost small" type="button" on:click={() => void refreshAgents(selectedAgentId)}>Refresh</button>
        </div>
        {#if loading && agents.length === 0}
          <div class="empty-card">Loading agents…</div>
        {:else if agents.length === 0}
          <div class="empty-card">No active agents yet.</div>
        {:else}
          <div class="agent-list">
            {#each agents as agent}
              <div class:active={agent.id === selectedAgentId} class="agent-row">
                <span class="agent-avatar">{String(agent.identity?.emoji || '∞')}</span>
                <span class="agent-copy">
                  <strong>{String(agent.name || agent.id)}</strong>
                  <span>{String(agent.state || 'running')} · {String(agent.runtime_model || agent.model_name || 'server default')}</span>
                </span>
                <button class="ghost small" type="button" on:click={() => { selectedAgentId = agent.id; resetDrafts(agent); }}>Manage</button>
                <a class="link-button" href={`${dashboardPageHref('chat')}?agent=${encodeURIComponent(agent.id)}`}>Open chat</a>
              </div>
            {/each}
          </div>
        {/if}
      </article>

      <AgentTemplatesPanel {templates} {busyKey} on:spawn={(event) => void handleSpawnTemplate(event.detail.templateName)} />

      <article class="panel">
        <div class="panel-head">
          <h3>Archived agents</h3>
          <button class="ghost small" type="button" on:click={() => void refreshArchived()}>Refresh</button>
        </div>
        {#if archived.length === 0}
          <div class="empty-card">No archived agents.</div>
        {:else}
          <div class="archive-list">
            {#each archived as entry}
              <div class="archive-row">
                <div>
                  <strong>{entry.agent_name || entry.agent_id}</strong>
                  <p>{entry.termination_reason || 'terminated'} · {formatTimestamp(entry.terminated_at)}</p>
                </div>
                <div class="row-actions">
                  <a class="ghost small" href={`${dashboardPageHref('chat')}?agent=${encodeURIComponent(entry.agent_id)}`}>View chat</a>
                  <button class="ghost small" type="button" disabled={busyKey === `revive:${entry.agent_id}`} on:click={() => void handleRevive(entry)}>
                    {busyKey === `revive:${entry.agent_id}` ? 'Reviving…' : 'Revive'}
                  </button>
                  <button class="danger small" type="button" disabled={busyKey === `delete:${entry.agent_id}`} on:click={() => void handleDeleteArchived(entry)}>
                    {busyKey === `delete:${entry.agent_id}` ? 'Deleting…' : 'Delete'}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </article>
    </div>

    <AgentDetailPanel
      agent={selectedAgent}
      {models}
      {loadingModels}
      {busyKey}
      bind:nameDraft
      bind:modelDraft
      on:refreshmodels={() => void loadModels()}
      on:savename={() => void handleSaveName()}
      on:savemodel={() => void handleSaveModel()}
      on:clone={() => void handleClone()}
      on:clearhistory={() => void handleClearHistory()}
      on:archive={() => void handleArchiveSelected()}
    />
  </div>
</section>

<style>
  .agents-page,
  .column,
  .agent-list,
  .archive-list {
    display: grid;
    gap: 18px;
  }

  .hero,
  .panel,
  .banner {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
  }

  .hero,
  .banner,
  .panel {
    padding: 20px;
  }

  .hero,
  .panel-head,
  .hero-actions,
  .row-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .content-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 18px;
  }

  h2,
  h3,
  p {
    margin: 0;
  }

  .eyebrow,
  .hero-copy,
  .agent-copy span,
  .archive-row p {
    color: #8aa4cf;
  }

  .ghost,
  .primary,
  .danger,
  .agent-row,
  .archive-row {
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
  }

  .ghost,
  .primary,
  .danger,
  .link-button {
    border-radius: 16px;
    padding: 0.8rem 1rem;
    text-decoration: none;
  }

  .agent-row,
  .archive-row,
  .empty-card {
    border-radius: 20px;
    padding: 14px;
  }

  .agent-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .agent-row.active {
    background: rgba(74, 116, 182, 0.18);
  }

  .agent-avatar {
    width: 36px;
    height: 36px;
    border-radius: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(122, 168, 255, 0.3);
    background: rgba(40, 79, 138, 0.24);
  }

  .agent-copy,
  .archive-row > div:first-child {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .archive-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .empty-card {
    background: rgba(255, 255, 255, 0.03);
  }

  .primary {
    background: rgba(40, 79, 138, 0.28);
  }

  .danger {
    background: rgba(128, 34, 27, 0.35);
    border-color: rgba(229, 112, 93, 0.24);
  }

  .link-button {
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
  }

  .error {
    border-color: rgba(229, 112, 93, 0.28);
    background: rgba(91, 31, 23, 0.58);
  }

  .notice {
    border-color: rgba(105, 165, 126, 0.24);
    background: rgba(23, 68, 45, 0.58);
  }

  .small {
    padding: 0.55rem 0.8rem;
  }

  @media (max-width: 1120px) {
    .content-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 760px) {
    .hero,
    .panel-head,
    .hero-actions,
    .archive-row,
    .row-actions {
      flex-direction: column;
      align-items: flex-start;
    }

    .agent-row {
      grid-template-columns: auto minmax(0, 1fr);
    }

    .link-button,
    .agent-row .ghost {
      grid-column: 1 / -1;
    }
  }
</style>
