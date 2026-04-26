const COMPONENT_TAG = 'infring-message-meta-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-message-meta-shell', shadow: 'none' }} />
<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let state = '';

  const dispatch = createEventDispatcher();

  function asBoolean(value) {
    if (value === true || value === false) return value;
    const text = String(value == null ? '' : value).trim().toLowerCase();
    return text === '1' || text === 'true' || text === 'yes' || text === 'on';
  }

  function asText(value) {
    return String(value == null ? '' : value).trim();
  }

  function parseState(value) {
    if (value && typeof value === 'object') return value;
    const text = String(value == null ? '' : value).trim();
    if (!text) return {};
    try {
      const parsed = JSON.parse(text);
      return parsed && typeof parsed === 'object' ? parsed : {};
    } catch (_) {
      return {};
    }
  }

  function normalizeState(value) {
    const source = parseState(value);
    return {
      shouldRender: asBoolean(source.shouldRender),
      visible: asBoolean(source.visible),
      copied: asBoolean(source.copied),
      hasTools: asBoolean(source.hasTools),
      toolsCollapsed: asBoolean(source.toolsCollapsed),
      canReportIssue: asBoolean(source.canReportIssue),
      canRetry: asBoolean(source.canRetry),
      canReply: asBoolean(source.canReply),
      canFork: asBoolean(source.canFork),
      timestamp: asText(source.timestamp),
      responseTime: asText(source.responseTime),
      burnLabel: asText(source.burnLabel),
      burnIconSrc: asText(source.burnIconSrc) || '/icons/vecteezy_fire-icon-simple-vector-perfect-illustration_13821331.svg'
    };
  }

  function emit(action) {
    dispatch('message-meta-action', { action });
  }

  $: model = normalizeState(state);
</script>

{#if model.shouldRender}
  <div class="message-stats-row">
    <button type="button" class="message-stat-btn" class:copied={model.copied} on:click={() => emit('copy')} title={model.copied ? 'Copied' : 'Copy message'} aria-label={model.copied ? 'Copied' : 'Copy message'}>
      {#if model.copied}
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"></path></svg>
      {:else}
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
      {/if}
    </button>

    {#if model.canReportIssue}
      <button type="button" class="message-stat-btn message-action-report-issue" on:click|stopPropagation={() => emit('report')} title="Send this chat context to eval review" aria-label="Send this chat context to eval review"><svg class="message-stat-icon message-stat-icon-hazard" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10.3 3.9 1.8 18.4A2 2 0 0 0 3.5 21h17a2 2 0 0 0 1.7-2.6L13.7 3.9a2 2 0 0 0-3.4 0Z"></path><path d="M12 9v5"></path><path d="M12 17h.01"></path></svg></button>
    {/if}

    {#if model.hasTools}
      <button type="button" class="message-stat-btn" on:click={() => emit('toggle-tools')} title={model.toolsCollapsed ? 'Expand processes' : 'Collapse processes'} aria-label={model.toolsCollapsed ? 'Expand processes' : 'Collapse processes'}>
        {#if model.toolsCollapsed}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="m9 6 6 6-6 6"></path></svg>
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"></path></svg>
        {/if}
      </button>
    {/if}

    {#if model.canRetry}
      <button type="button" class="message-stat-btn" on:click={() => emit('retry')} title="Retry from this turn" aria-label="Retry from this turn"><svg class="message-stat-icon message-stat-icon-refresh" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"></path><path d="M21 3v5h-5"></path></svg></button>
    {/if}

    {#if model.canReply}
      <button type="button" class="message-stat-btn message-action-reply" on:click={() => emit('reply')} title="Reply to this message" aria-label="Reply to this message"><svg class="message-stat-icon message-stat-icon-reply" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m9 14-5-5 5-5"></path><path d="M20 20v-5a6 6 0 0 0-6-6H4"></path></svg><span class="message-reply-label">Reply</span></button>
    {/if}

    {#if model.canFork}
      <button type="button" class="message-stat-btn" on:click={() => emit('fork')} title="Fork to a new agent" aria-label="Fork to a new agent"><svg class="message-stat-icon message-stat-icon-fork" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M5 5.372v.878c0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75v-.878a2.25 2.25 0 1 1 1.5 0v.878a2.25 2.25 0 0 1-2.25 2.25h-1.5v2.128a2.251 2.251 0 1 1-1.5 0V8.5h-1.5A2.25 2.25 0 0 1 3.5 6.25v-.878a2.25 2.25 0 1 1 1.5 0ZM5 3.25a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Zm6.75.75a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Zm-3 8.75a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Z"></path></svg></button>
    {/if}

    {#if model.visible && model.timestamp}
      <span class="message-stat-time">{model.timestamp}</span>
    {/if}
    {#if model.visible && model.responseTime}
      <span class="message-stat-meta">{model.responseTime}</span>
    {/if}
    {#if model.visible && model.burnLabel}
      <span class="message-stat-burn"><img class="message-meta-icon message-stat-burn-icon" src={model.burnIconSrc} alt="" aria-hidden="true"><span>{model.burnLabel}</span></span>
    {/if}
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
