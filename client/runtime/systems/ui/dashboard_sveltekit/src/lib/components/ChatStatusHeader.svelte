<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DashboardAgentRow } from '$lib/chat';

  export let activeAgent: DashboardAgentRow | null = null;
  export let activeModel = 'Select a conversation';
  export let streamState = 'disconnected';
  export let drawerOpen = false;
  export let error = '';
  export let notice = '';

  const dispatch = createEventDispatcher<{ toggledrawer: void }>();

  function agentLabel(agent: DashboardAgentRow | null): string {
    if (!agent) return 'Select a conversation';
    return String(agent.name || agent.id || 'Conversation').trim() || 'Conversation';
  }

  function agentState(agent: DashboardAgentRow | null): string {
    if (!agent) return 'No conversation selected';
    if (agent.archived) return 'Archived';
    if (agent.draft) return 'Draft';
    return String(agent.state || 'running').trim() || 'running';
  }
</script>

<header class="chat-header">
  <div>
    <p class="eyebrow">Authoritative lane</p>
    <h2>{agentLabel(activeAgent)}</h2>
    <p class="summary">
      {#if activeAgent}
        {`${agentState(activeAgent)} · ${activeModel} · stream ${streamState}`}
      {:else}
        This route now talks directly to the real /api/agents session, message, upload, and websocket surfaces.
      {/if}
    </p>
  </div>
  <div class="header-actions">
    <button class="ghost" type="button" on:click={() => dispatch('toggledrawer')}>
      {drawerOpen ? 'Hide details' : 'Details'}
    </button>
  </div>
</header>

{#if error}
  <div class="banner error">{error}</div>
{:else if notice}
  <div class="banner notice">{notice}</div>
{/if}

<style>
  .chat-header,
  .banner {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
    padding: 18px 20px;
  }

  .chat-header,
  .header-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .eyebrow,
  .summary {
    color: #8aa4cf;
  }

  h2,
  p {
    margin: 0;
  }

  .ghost {
    border: 1px solid rgba(158, 188, 255, 0.18);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
    border-radius: 16px;
    padding: 0.8rem 1rem;
    text-decoration: none;
  }

  .ghost:hover {
    background: rgba(74, 116, 182, 0.18);
  }

  .error {
    border-color: rgba(229, 112, 93, 0.28);
    background: rgba(91, 31, 23, 0.58);
  }

  .notice {
    border-color: rgba(105, 165, 126, 0.24);
    background: rgba(23, 68, 45, 0.58);
  }

  @media (max-width: 1080px) {
    .chat-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .header-actions {
      width: 100%;
      justify-content: flex-start;
      flex-wrap: wrap;
    }
  }
</style>
