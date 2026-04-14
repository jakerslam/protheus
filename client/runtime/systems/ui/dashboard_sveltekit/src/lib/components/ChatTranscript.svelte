<script lang="ts">
  import { afterUpdate } from 'svelte';
  import type { DashboardChatMessage } from '$lib/chat';

  export let activeAgentId = '';
  export let loading = false;
  export let messages: DashboardChatMessage[] = [];

  let transcriptHost: HTMLDivElement | null = null;

  afterUpdate(() => {
    if (!transcriptHost) return;
    transcriptHost.scrollTo({
      top: transcriptHost.scrollHeight,
      behavior: 'smooth',
    });
  });

  function formatTime(timestamp: number | string | undefined): string {
    const value = typeof timestamp === 'number' ? timestamp : Date.parse(String(timestamp || ''));
    if (!Number.isFinite(value)) return 'Unknown time';
    return new Intl.DateTimeFormat(undefined, {
      hour: 'numeric',
      minute: '2-digit',
      month: 'short',
      day: 'numeric',
    }).format(value);
  }
</script>

<div class="transcript" bind:this={transcriptHost}>
  {#if !activeAgentId}
    <div class="empty-state">
      <strong>Pick a conversation</strong>
      <span>The native chat page is live. Select an agent on the left or create a new draft chat.</span>
    </div>
  {:else if loading && messages.length === 0}
    <div class="empty-state">
      <strong>Loading session…</strong>
      <span>Pulling the authoritative transcript from `/api/agents/{activeAgentId}/session`.</span>
    </div>
  {:else if messages.length === 0}
    <div class="empty-state">
      <strong>No messages yet</strong>
      <span>This conversation is ready. Send the first message below.</span>
    </div>
  {:else}
    {#each messages as message}
      <article class:agent={message.role === 'agent'} class:system={message.role === 'system'} class:user={message.role === 'user'} class="message">
        <div class="message-head">
          <strong>{message.role === 'user' ? 'You' : (message.role === 'agent' ? 'Agent' : (message.role === 'system' ? 'System' : 'Terminal'))}</strong>
          <span>{formatTime(message.ts)}</span>
        </div>
        <div class="message-body">{message.text || ' '}</div>
        {#if message.meta}
          <div class="message-meta">{message.meta}</div>
        {/if}
        {#if message.tools.length}
          <div class="tool-list">
            {#each message.tools as tool}
              <div class:tool-error={tool.isError || tool.blocked} class="tool-card">
                <div class="tool-head">
                  <strong>{tool.name}</strong>
                  <span>{tool.status || (tool.isError ? 'error' : 'done')}</span>
                </div>
                {#if tool.result}
                  <p>{tool.result}</p>
                {:else if tool.input}
                  <p>{tool.input}</p>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </article>
    {/each}
  {/if}
</div>

<style>
  .transcript {
    min-height: 420px;
    padding: 18px;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 14px;
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
  }

  .message,
  .empty-state,
  .tool-card {
    border-radius: 20px;
    border: 1px solid rgba(158, 188, 255, 0.12);
    background: rgba(255, 255, 255, 0.03);
  }

  .message {
    padding: 16px 18px;
    display: grid;
    gap: 10px;
  }

  .message.user {
    background: rgba(40, 79, 138, 0.18);
  }

  .message.system {
    background: rgba(102, 57, 25, 0.18);
  }

  .empty-state,
  .tool-card {
    padding: 16px;
  }

  .message-head,
  .tool-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .message-head span,
  .message-meta,
  .tool-head span {
    color: #8aa4cf;
  }

  .message-body,
  .tool-card p {
    word-break: break-word;
    margin: 0;
  }

  .tool-list {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .tool-error {
    border-color: rgba(229, 112, 93, 0.28);
    background: rgba(122, 38, 24, 0.18);
  }
</style>
