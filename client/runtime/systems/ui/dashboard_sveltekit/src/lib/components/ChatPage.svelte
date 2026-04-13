<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { dashboardClassicHref } from '$lib/dashboard';
  import ChatTranscript from '$lib/components/ChatTranscript.svelte';
  import type { DashboardAgentRow, DashboardChatMessage } from '$lib/chat';
  import { createDraftAgent, readAgentSession, readSidebarAgents, sendAgentMessage } from '$lib/chat';

  let agents: DashboardAgentRow[] = [];
  let activeAgentId = '';
  let messages: DashboardChatMessage[] = [];
  let composer = '';
  let error = '';
  let loadingAgents = true;
  let loadingSession = false;
  let creatingAgent = false;
  let sending = false;

  $: activeAgent = agents.find((row) => row.id === activeAgentId) || null;
  $: activeModel = activeAgent ? String(activeAgent.runtime_model || activeAgent.model_name || 'Server default') : 'Select a conversation';

  onMount(async () => {
    await refreshAgents({ syncQuery: false });
  });

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

  function preferredAgentId(rows: DashboardAgentRow[], requestedId: string): string {
    const cleanRequested = String(requestedId || '').trim();
    if (cleanRequested && rows.some((row) => row.id === cleanRequested)) return cleanRequested;
    return rows.length ? rows[0].id : '';
  }

  async function syncAgentQuery(agentId: string): Promise<void> {
    const url = new URL($page.url);
    if (agentId) url.searchParams.set('agent', agentId);
    else url.searchParams.delete('agent');
    await goto(`${url.pathname}${url.search}`, {
      replaceState: true,
      noScroll: true,
      keepFocus: true,
      invalidateAll: false,
    });
  }

  async function loadSession(agentId: string): Promise<void> {
    if (!agentId) {
      messages = [];
      return;
    }
    loadingSession = true;
    error = '';
    try {
      const session = await readAgentSession(agentId);
      if (agentId !== activeAgentId) return;
      messages = session.messages;
    } catch (cause) {
      if (agentId === activeAgentId) {
        error = cause instanceof Error ? cause.message : String(cause || 'session_unavailable');
      }
    } finally {
      if (agentId === activeAgentId) loadingSession = false;
    }
  }

  async function activateAgent(agentId: string, options: { syncQuery?: boolean; forceReload?: boolean } = {}): Promise<void> {
    if (!agentId) return;
    const changed = agentId !== activeAgentId;
    activeAgentId = agentId;
    if (options.syncQuery !== false) await syncAgentQuery(agentId);
    if (changed || options.forceReload) await loadSession(agentId);
  }

  async function refreshAgents(options: { syncQuery?: boolean; preferredId?: string } = {}): Promise<void> {
    loadingAgents = true;
    error = '';
    try {
      const rows = await readSidebarAgents();
      agents = rows;
      const requestedId = String(options.preferredId || $page.url.searchParams.get('agent') || activeAgentId || '').trim();
      const nextId = preferredAgentId(rows, requestedId);
      if (!nextId) {
        activeAgentId = '';
        messages = [];
        if (options.syncQuery !== false) await syncAgentQuery('');
        return;
      }
      await activateAgent(nextId, {
        syncQuery: options.syncQuery !== false && nextId !== requestedId,
        forceReload: true,
      });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'agent_roster_unavailable');
    } finally {
      loadingAgents = false;
    }
  }

  async function handleCreateAgent(): Promise<void> {
    creatingAgent = true;
    error = '';
    try {
      const created = await createDraftAgent();
      await refreshAgents({ preferredId: created.id });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'spawn_failed');
    } finally {
      creatingAgent = false;
    }
  }

  async function handleSelectAgent(agentId: string): Promise<void> {
    await activateAgent(agentId, { syncQuery: true, forceReload: agentId !== activeAgentId || messages.length === 0 });
  }

  async function handleSend(): Promise<void> {
    const raw = String(composer || '');
    const text = raw.trim();
    if (!text || !activeAgentId || sending) return;
    composer = '';
    const optimistic: DashboardChatMessage = {
      id: `pending-${Date.now()}`,
      role: 'user',
      text: raw,
      meta: 'Sending…',
      ts: Date.now(),
      tools: [],
      pending: true,
    };
    messages = [...messages, optimistic];
    sending = true;
    error = '';
    try {
      await sendAgentMessage(activeAgentId, raw);
      await loadSession(activeAgentId);
    } catch (cause) {
      const reason = cause instanceof Error ? cause.message : String(cause || 'send_failed');
      error = reason;
      messages = [
        ...messages,
        {
          id: `send-error-${Date.now()}`,
          role: 'system',
          text: `Message failed: ${reason}`,
          meta: 'Send failed',
          ts: Date.now(),
          tools: [],
        },
      ];
    } finally {
      sending = false;
    }
  }

  function handleComposerKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      void handleSend();
    }
  }
</script>

<section class="chat-page">
  <aside class="chat-rail">
    <div class="rail-header">
      <div>
        <p class="eyebrow">Native chat</p>
        <h2>Conversations</h2>
      </div>
      <button class="ghost" type="button" on:click={() => void refreshAgents()} disabled={loadingAgents}>
        Refresh
      </button>
    </div>

    <button class="create-button" type="button" on:click={() => void handleCreateAgent()} disabled={creatingAgent}>
      {creatingAgent ? 'Creating draft…' : 'New draft chat'}
    </button>

    <div class="rail-list" aria-label="Conversation roster">
      {#if loadingAgents && agents.length === 0}
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
            on:click={() => void handleSelectAgent(agent.id)}
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

  <div class="chat-workbench">
    <header class="chat-header">
      <div>
        <p class="eyebrow">Authoritative lane</p>
        <h2>{activeAgent ? agentLabel(activeAgent) : 'Select a conversation'}</h2>
        <p class="summary">
          {activeAgent ? `${agentState(activeAgent)} · ${activeModel}` : 'This route now talks directly to the real /api/agents session and message surfaces.'}
        </p>
      </div>
      <a class="ghost" href={dashboardClassicHref('chat')}>Open classic chat</a>
    </header>

    {#if error}
      <div class="banner error">{error}</div>
    {/if}

    <ChatTranscript {activeAgentId} loading={loadingSession} {messages} />

    <div class="composer-card">
      <textarea
        bind:value={composer}
        class="composer"
        rows="4"
        placeholder={activeAgentId ? 'Send a message to this conversation…' : 'Create or select a conversation first…'}
        disabled={!activeAgentId || sending}
        on:keydown={handleComposerKeydown}
      ></textarea>
      <div class="composer-actions">
        <span>{sending ? 'Waiting for authoritative response…' : 'Enter to send · Shift+Enter for newline'}</span>
        <button class="send-button" type="button" on:click={() => void handleSend()} disabled={!activeAgentId || sending || !composer.trim()}>
          {sending ? 'Sending…' : 'Send'}
        </button>
      </div>
    </div>
  </div>
</section>

<style>
  .chat-page {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    gap: 18px;
    min-height: calc(100vh - 170px);
  }

  .chat-rail,
  .chat-header,
  .composer-card,
  .banner {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
  }

  .chat-rail {
    padding: 18px;
    display: grid;
    align-content: start;
    gap: 14px;
  }

  .rail-header,
  .chat-header,
  .composer-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  h2,
  p {
    margin: 0;
  }

  .eyebrow,
  .summary,
  .composer-actions span,
  .agent-copy span {
    color: #8aa4cf;
  }

  .rail-list,
  .chat-workbench {
    display: grid;
    gap: 12px;
  }

  .chat-workbench {
    min-width: 0;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
  }

  .chat-header,
  .composer-card,
  .banner {
    padding: 18px 20px;
  }

  .agent-row,
  .ghost,
  .create-button,
  .send-button {
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
  .create-button:hover,
  .send-button:hover {
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
  .send-button,
  .ghost {
    border-radius: 16px;
    padding: 0.8rem 1rem;
    text-decoration: none;
  }

  .create-button,
  .send-button {
    cursor: pointer;
  }

  .empty-card {
    border-radius: 20px;
    border: 1px solid rgba(158, 188, 255, 0.12);
    background: rgba(255, 255, 255, 0.03);
  }

  .composer-card {
    display: grid;
    gap: 12px;
  }

  .composer {
    width: 100%;
    min-height: 112px;
    resize: vertical;
    border: 1px solid rgba(158, 188, 255, 0.18);
    border-radius: 18px;
    background: rgba(4, 11, 20, 0.4);
    color: inherit;
    padding: 14px 16px;
    font: inherit;
    box-sizing: border-box;
  }

  .error {
    border-color: rgba(229, 112, 93, 0.28);
    background: rgba(91, 31, 23, 0.58);
  }

  @media (max-width: 1080px) {
    .chat-page {
      grid-template-columns: 1fr;
    }
  }
</style>
