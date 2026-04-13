<script lang="ts">
  import { configureChannel, readChannels, readWhatsappQrStatus, removeChannelConfig, startWhatsappQr, testChannel, type DashboardChannelField, type DashboardChannelRow, type DashboardWhatsappQrState } from '$lib/channels';
  import { onDestroy, onMount } from 'svelte';

  let channels: DashboardChannelRow[] = [];
  let selectedName = '';
  let formValues: Record<string, string> = {};
  let qrState: DashboardWhatsappQrState | null = null;
  let qrHandle: ReturnType<typeof setInterval> | null = null;
  let loading = true;
  let busyKey = '';
  let error = '';
  let notice = '';

  $: selectedChannel = channels.find((row) => row.name === selectedName) || channels[0] || null;

  onMount(async () => {
    await refresh();
  });

  onDestroy(() => {
    stopQrPolling();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      channels = await readChannels();
      if (!channels.some((row) => row.name === selectedName)) {
        selectedName = channels[0]?.name || '';
      }
      syncFormFromSelected();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'channels_unavailable');
    } finally {
      loading = false;
    }
  }

  function syncFormFromSelected(): void {
    const next: Record<string, string> = {};
    for (const field of selectedChannel?.fields || []) {
      if (field.type === 'secret') {
        next[field.key] = '';
        continue;
      }
      next[field.key] = String(formValues[field.key] || field.value || '').trim();
    }
    formValues = next;
  }

  function selectChannel(name: string): void {
    selectedName = name;
    qrState = null;
    stopQrPolling();
    syncFormFromSelected();
  }

  function fieldType(field: DashboardChannelField): string {
    const raw = String(field.type || '').trim().toLowerCase();
    if (raw === 'secret') return 'password';
    if (raw === 'url') return 'url';
    return 'text';
  }

  function setFieldValue(key: string, value: string): void {
    formValues = { ...formValues, [key]: value };
  }

  function fieldsPayload(): Record<string, string> {
    const payload: Record<string, string> = {};
    for (const field of selectedChannel?.fields || []) {
      const value = String(formValues[field.key] || '').trim();
      if (!value && !field.required) continue;
      payload[field.key] = value;
    }
    return payload;
  }

  async function saveSelectedChannel(): Promise<void> {
    if (!selectedChannel) return;
    busyKey = 'save';
    try {
      notice = await configureChannel(selectedChannel.name, fieldsPayload());
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'channel_save_failed');
    } finally {
      busyKey = '';
    }
  }

  async function testSelectedChannel(): Promise<void> {
    if (!selectedChannel) return;
    busyKey = 'test';
    try {
      notice = await testChannel(selectedChannel.name);
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'channel_test_failed');
    } finally {
      busyKey = '';
    }
  }

  async function removeSelectedChannel(): Promise<void> {
    if (!selectedChannel) return;
    busyKey = 'remove';
    try {
      notice = await removeChannelConfig(selectedChannel.name);
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'channel_remove_failed');
    } finally {
      busyKey = '';
    }
  }

  function stopQrPolling(): void {
    if (qrHandle) clearInterval(qrHandle);
    qrHandle = null;
  }

  function beginQrPolling(sessionId: string): void {
    stopQrPolling();
    if (!sessionId) return;
    qrHandle = setInterval(async () => {
      try {
        qrState = await readWhatsappQrStatus(sessionId);
        if (qrState.connected || qrState.expired) {
          stopQrPolling();
          if (qrState.connected) {
            notice = qrState.message || 'WhatsApp connected';
            await refresh();
          }
        }
      } catch {
        stopQrPolling();
      }
    }, 3000);
  }

  async function startQrFlow(): Promise<void> {
    busyKey = 'qr';
    try {
      qrState = await startWhatsappQr();
      if (qrState.session_id && !qrState.connected) {
        beginQrPolling(qrState.session_id);
      }
      notice = qrState.message || 'QR session started';
      await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'whatsapp_qr_failed');
    } finally {
      busyKey = '';
    }
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native channels</p>
      <h2>Channel setup, verification, and WhatsApp QR state without dropping back to classic.</h2>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refresh()} disabled={loading}>{loading ? 'Refreshing…' : 'Refresh'}</button>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Catalog</h3><span class="meta">{channels.length} total</span></div>
      <div class="rows">
        {#each channels as channel}
          <button class:selected={selectedChannel?.name === channel.name} class="row channel-row" type="button" on:click={() => selectChannel(channel.name)}>
            <div class="row-copy">
              <strong>{channel.display_name}</strong>
              <span>{channel.description || channel.name}</span>
            </div>
            <span>{channel.connected ? 'Ready' : (channel.configured ? 'Configured' : 'Setup')}</span>
          </button>
        {/each}
      </div>
    </article>

    <article class="panel">
      {#if selectedChannel}
        <div class="panel-head"><h3>{selectedChannel.display_name}</h3><span class="meta">{selectedChannel.difficulty} {selectedChannel.setup_time ? `· ${selectedChannel.setup_time}` : ''}</span></div>
        <p class="summary">{selectedChannel.quick_setup || selectedChannel.description}</p>
        {#if selectedChannel.setup_steps.length}
          <ol class="steps">
            {#each selectedChannel.setup_steps as step}
              <li>{step}</li>
            {/each}
          </ol>
        {/if}
        <div class="form-grid">
          {#each selectedChannel.fields as field}
            <label class="field-label">
              <span>{field.label}{field.required ? ' *' : ''}</span>
              <input
                class="field"
                type={fieldType(field)}
                value={formValues[field.key] || ''}
                placeholder={field.placeholder || field.label}
                on:input={(event) => setFieldValue(field.key, (event.currentTarget as HTMLInputElement).value)}
              />
            </label>
          {/each}
        </div>
        <div class="row-actions">
          <button class="primary small" type="button" disabled={busyKey === 'save'} on:click={() => void saveSelectedChannel()}>{busyKey === 'save' ? 'Saving…' : 'Save'}</button>
          <button class="ghost small" type="button" disabled={busyKey === 'test'} on:click={() => void testSelectedChannel()}>{busyKey === 'test' ? 'Testing…' : 'Test'}</button>
          <button class="ghost small" type="button" disabled={busyKey === 'remove'} on:click={() => void removeSelectedChannel()}>{busyKey === 'remove' ? 'Removing…' : 'Remove'}</button>
          {#if selectedChannel.name === 'whatsapp' && selectedChannel.setup_type === 'qr'}
            <button class="ghost small" type="button" disabled={busyKey === 'qr'} on:click={() => void startQrFlow()}>{busyKey === 'qr' ? 'Starting…' : 'Start QR'}</button>
          {/if}
        </div>
        {#if qrState}
          <div class="qr-panel">
            <div class="row-copy">
              <strong>WhatsApp QR</strong>
              <span>{qrState.message || qrState.help || 'Waiting for scan…'}</span>
            </div>
            {#if qrState.qr_data_url}
              <img alt="WhatsApp QR code" class="qr" src={qrState.qr_data_url} />
            {/if}
          </div>
        {/if}
      {:else}
        <div class="empty-state">No channels available.</div>
      {/if}
    </article>
  </div>
</section>

<style>
  .page, .grid, .rows, .form-grid, .steps { display: grid; gap: 18px; }
  .grid { grid-template-columns: minmax(280px, 0.95fr) minmax(0, 1.25fr); }
  .hero, .panel, .banner, .row, .field, .qr-panel { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner, .qr-panel { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .row { padding: 12px 14px; }
  .channel-row { width: 100%; text-align: left; cursor: pointer; display: flex; align-items: center; justify-content: space-between; gap: 12px; background: rgba(255,255,255,0.04); }
  .channel-row.selected { border-color: rgba(158,188,255,0.4); background: rgba(75,120,198,0.14); }
  .row-copy, .field-label { display: grid; gap: 4px; }
  .field { padding: 0.75rem 0.85rem; font: inherit; width: 100%; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  .summary { margin: 0; color: #dce6ff; }
  .steps { margin: 0; padding-left: 1.25rem; }
  .qr { width: min(220px, 100%); border-radius: 20px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.08); padding: 10px; }
  .empty-state { color: #8aa4cf; }
  @media (max-width: 980px) { .grid { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row-actions, .channel-row { flex-direction: column; align-items: flex-start; } }
</style>
