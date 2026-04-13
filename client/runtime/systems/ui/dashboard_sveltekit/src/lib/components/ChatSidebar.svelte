<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DashboardAgentRow } from '$lib/chat';

  export let agents: DashboardAgentRow[] = [];
  export let activeAgentId = '';
  export let loading = false;
  export let creating = false;

  const dispatch = createEventDispatcher<{
    refresh: void;
    create: void;
    select: { id: string };
  }>();

  function agentEmoji(agent: DashboardAgentRow | null): string {
    return String(agent?.identity?.emoji || '∞').trim() || '∞';
  }

  function agentLabel(agent: DashboardAgentRow): string {
    return String(agent.name || agent.id || 'Conversation').trim() || 'Conversation';
  }

  function agentState(agent: DashboardAgentRow | null): string {
    if (!agent) return 'No conversation selected';
    if (agent.archived) return 'Archived';
    if (agent.draft) return 'Draft';
    return String(agent.state || 'running').trim() || 'running';
  }
</script>

<aside class="chat-rail">
  <div class="rail-header">
    <div>
      <p class="eyebrow">Native chat</p>
      <h2>Conversations</h2>
    </div>
    <button class="ghost" type="button" on:click={() => dispatch('refresh')} disabled={loading}>
      Refresh
    </button>
  </div>

  <button class="create-button" type="button" on:click={() => dispatch('create')} disabled={creating}>
    {creating ? 'Creating draft…' : 'New draft chat'}
  </button>

  <div class="rail-list" aria-label="Conversation roster">
    {#if loading && agents.length === 0}
      <div class="empty-card">Loading conversations…</div>
    {:else if agents.length === 0}
      <div class="empty-card">
        <strong>No conversations yet</strong>
        <span>Create a draft agent to start a native chat thread.</span>
      </div>
    {:else}
      {#each agents as agent}
        <button
          class:active={agent.id === activeAgentId}
          class="agent-row"
          type="button"
          on:click={() => dispatch('select', { id: agent.id })}
        >
          <span class="agent-mark" aria-hidden="true">{agentEmoji(agent)}</span>
          <span class="agent-copy">
            <strong>{agentLabel(agent)}</strong>
            <span>{agentState(agent)} · {String(agent.runtime_model || agent.model_name || 'server default')}</span>
          </span>
        </button>
      {/each}
    {/if}
  </div>
</aside>

<style>
  .chat-rail {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
    padding: 18px;
    display: grid;
    align-content: start;
    gap: 14px;
  }

  .rail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .eyebrow,
  .agent-copy span {
    color: #8aa4cf;
  }

  h2,
  p {
    margin: 0;
  }

  .rail-list {
    display: grid;
    gap: 12px;
  }

  .agent-row,
  .ghost,
  .create-button {
    border: 1px solid rgba(158, 188, 255, 0.18);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
  }

  .agent-row,
  .empty-card {
    width: 100%;
    border-radius: 18px;
    padding: 14px;
  }

  .agent-row {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    cursor: pointer;
  }

  .agent-row.active,
  .agent-row:hover,
  .ghost:hover,
  .create-button:hover {
    background: rgba(74, 116, 182, 0.18);
  }

  .agent-mark {
    width: 36px;
    height: 36px;
    border-radius: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(122, 168, 255, 0.3);
    background: rgba(40, 79, 138, 0.24);
    flex: 0 0 auto;
  }

  .agent-copy {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .agent-copy strong {
    word-break: break-word;
  }

  .create-button,
  .ghost {
    border-radius: 16px;
    padding: 0.8rem 1rem;
    text-decoration: none;
  }

  .create-button {
    cursor: pointer;
  }

  .empty-card {
    border-radius: 20px;
    border: 1px solid rgba(158, 188, 255, 0.12);
    background: rgba(255, 255, 255, 0.03);
  }
</style>
