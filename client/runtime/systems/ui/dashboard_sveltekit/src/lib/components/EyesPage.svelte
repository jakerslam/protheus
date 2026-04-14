<script lang="ts">
  import { readEyes, saveEye, type DashboardEyeRow } from '$lib/eyes';
  import { onMount } from 'svelte';

  let eyes: DashboardEyeRow[] = [];
  let name = '';
  let status = 'active';
  let url = '';
  let apiKey = '';
  let cadenceHours = 4;
  let topics = '';
  let saving = false;
  let loading = true;
  let error = '';
  let notice = '';

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      eyes = await readEyes();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'eyes_unavailable');
    } finally {
      loading = false;
    }
  }

  async function save(): Promise<void> {
    saving = true;
    try {
      notice = await saveEye({ name, status, url, api_key: apiKey, cadence_hours: cadenceHours, topics });
      url = '';
      apiKey = '';
      topics = '';
      if (name) name = '';
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'eye_save_failed');
    } finally {
      saving = false;
    }
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native eyes</p>
      <h2>System eye catalog and onboarding in the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button>
    </div>
  </div>
  {#if error}<div class="banner error">{error}</div>{:else if notice}<div class="banner notice">{notice}</div>{/if}
  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Add or update eye</h3></div>
      <div class="form-grid">
        <input bind:value={name} class="field" type="text" placeholder="Name" />
        <select bind:value={status} class="field"><option value="active">active</option><option value="paused">paused</option><option value="dormant">dormant</option></select>
        <input bind:value={url} class="field" type="text" placeholder="Source URL" />
        <input bind:value={apiKey} class="field" type="password" placeholder="API key" />
        <input bind:value={cadenceHours} class="field" type="number" min="1" />
        <textarea bind:value={topics} class="field area" rows="3" placeholder="Topics"></textarea>
        <button class="primary small" type="button" disabled={saving} on:click={() => void save()}>{saving ? 'Saving…' : 'Save eye'}</button>
      </div>
    </article>
    <article class="panel">
      <div class="panel-head"><h3>Registered eyes</h3><span class="meta">{eyes.length} total</span></div>
      <div class="rows">
        {#each eyes as eye}
          <div class="row">
            <div class="row-copy">
              <strong>{eye.name}</strong>
              <span>{eye.endpoint_host || eye.endpoint_url || 'system'} · {eye.status}</span>
            </div>
            <span>{eye.api_key_present ? 'api-key' : 'no key'} · {eye.updated_at || 'recent'}</span>
          </div>
        {/each}
      </div>
    </article>
  </div>
</section>

<style>
  .page, .grid, .rows, .form-grid { display: grid; gap: 18px; }
  .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row, .field { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; border-radius: 20px; background: rgba(255,255,255,0.04); }
  .row-copy { display: grid; gap: 4px; }
  .field { padding: 0.75rem 0.85rem; font: inherit; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row { flex-direction: column; align-items: flex-start; } }
</style>
