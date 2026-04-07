<script lang="ts">
  import { onMount } from 'svelte';
  import { readRuntimeStatus, type RuntimeStatus } from '../lib/runtime';

  let status: RuntimeStatus | null = null;
  let error = '';

  onMount(async () => {
    try {
      status = await readRuntimeStatus();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'status_unavailable');
    }
  });
</script>

<main class="dashboard">
  <section class="panel">
    <h1>Infring Dashboard</h1>
    {#if error}
      <p class="status error">Status unavailable: {error}</p>
    {:else if status}
      <pre>{JSON.stringify(status, null, 2)}</pre>
    {:else}
      <p class="status">Loading runtime status…</p>
    {/if}
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: "IBM Plex Sans", "Inter", system-ui, sans-serif;
    background: radial-gradient(circle at 20% 10%, #132a52 0%, #061123 45%, #020913 100%);
    color: #e7edf7;
  }

  .dashboard {
    min-height: 100vh;
    padding: 24px;
    display: grid;
    gap: 18px;
  }

  .panel {
    border: 1px solid rgba(127, 173, 255, 0.35);
    border-radius: 14px;
    background: rgba(8, 20, 36, 0.7);
    padding: 18px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(8px);
  }

  .panel h1 {
    margin: 0 0 10px;
  }

  .status {
    opacity: 0.9;
  }

  .status.error {
    color: #ff7f7f;
  }

  pre {
    overflow: auto;
    background: rgba(0, 0, 0, 0.3);
    border-radius: 10px;
    padding: 12px;
    border: 1px solid rgba(127, 173, 255, 0.28);
  }
</style>
