<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let value = '';
  export let files: File[] = [];
  export let disabled = false;
  export let sending = false;

  const dispatch = createEventDispatcher<{ submit: void }>();

  function handleFiles(event: Event): void {
    const target = event.currentTarget as HTMLInputElement | null;
    const nextFiles = Array.from(target?.files || []);
    files = [...files, ...nextFiles];
    if (target) target.value = '';
  }

  function removeFile(index: number): void {
    files = files.filter((_, current) => current !== index);
  }

  function handleComposerKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      dispatch('submit');
    }
  }
</script>

<div class="composer-card">
  <div class="composer-head">
    <label class="attach-button">
      <input type="file" multiple disabled={disabled} on:change={handleFiles} />
      <span>Add files</span>
    </label>
    {#if files.length}
      <span class="file-count">{files.length} attachment(s) queued</span>
    {/if}
  </div>

  {#if files.length}
    <div class="file-list">
      {#each files as file, index}
        <button class="file-chip" type="button" on:click={() => removeFile(index)} disabled={disabled}>
          <strong>{file.name}</strong>
          <span>Remove</span>
        </button>
      {/each}
    </div>
  {/if}

  <textarea
    bind:value
    class="composer"
    rows="4"
    placeholder={disabled ? 'Create or select a conversation first…' : 'Send a message to this conversation…'}
    {disabled}
    on:keydown={handleComposerKeydown}
  ></textarea>

  <div class="composer-actions">
    <span>{sending ? 'Waiting for authoritative response…' : 'Enter to send · Shift+Enter for newline'}</span>
    <button class="send-button" type="button" on:click={() => dispatch('submit')} disabled={disabled || (!value.trim() && files.length === 0)}>
      {sending ? 'Sending…' : 'Send'}
    </button>
  </div>
</div>

<style>
  .composer-card {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
    padding: 18px 20px;
    display: grid;
    gap: 12px;
  }

  .composer-head,
  .composer-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .attach-button,
  .file-chip,
  .send-button {
    border: 1px solid rgba(158, 188, 255, 0.18);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
  }

  .attach-button,
  .send-button {
    border-radius: 16px;
    padding: 0.8rem 1rem;
    cursor: pointer;
  }

  .attach-button input {
    display: none;
  }

  .file-count,
  .composer-actions span,
  .file-chip span {
    color: #8aa4cf;
  }

  .file-list {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .file-chip {
    border-radius: 16px;
    padding: 0.7rem 0.9rem;
    display: inline-grid;
    gap: 3px;
    text-align: left;
    cursor: pointer;
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

  @media (max-width: 760px) {
    .composer-head,
    .composer-actions {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
