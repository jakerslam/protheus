<script lang="ts">
  import { spawnTemplateAgent, readTemplates, type DashboardTemplateRow } from '$lib/agents';
  import { configureChannel, readChannels, startWhatsappQr, type DashboardChannelRow, type DashboardWhatsappQrState } from '$lib/channels';
  import { dashboardClassicHref, dashboardPageHref } from '$lib/dashboard';
  import { createDraftAgent } from '$lib/chat';
  import { readProviders, saveProviderKey, saveProviderUrl, testProvider, type DashboardProviderRow } from '$lib/settings';
  import { onMount } from 'svelte';

  let step = 1;
  let providers: DashboardProviderRow[] = [];
  let templates: DashboardTemplateRow[] = [];
  let channels: DashboardChannelRow[] = [];
  let selectedProviderId = '';
  let selectedTemplateName = '';
  let selectedChannelName = '';
  let providerKey = '';
  let providerUrl = '';
  let channelValues: Record<string, string> = {};
  let createdAgentId = '';
  let qrState: DashboardWhatsappQrState | null = null;
  let loading = true;
  let busyKey = '';
  let error = '';
  let notice = '';

  $: selectedProvider = providers.find((row) => row.id === selectedProviderId) || providers[0] || null;
  $: selectedTemplate = templates.find((row) => row.name === selectedTemplateName) || templates[0] || null;
  $: selectedChannel = channels.find((row) => row.name === selectedChannelName) || channels[0] || null;

  onMount(async () => {
    await refresh();
  });

  async function refresh(): Promise<void> {
    loading = true;
    error = '';
    try {
      [providers, templates, channels] = await Promise.all([readProviders(), readTemplates(), readChannels()]);
      selectedProviderId = selectedProviderId || providers[0]?.id || '';
      selectedTemplateName = selectedTemplateName || templates[0]?.name || '';
      selectedChannelName = selectedChannelName || channels[0]?.name || '';
      syncChannelDefaults();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'wizard_unavailable');
    } finally {
      loading = false;
    }
  }

  function syncChannelDefaults(): void {
    const next: Record<string, string> = {};
    for (const field of selectedChannel?.fields || []) {
      next[field.key] = channelValues[field.key] || field.value || '';
    }
    channelValues = next;
  }

  async function saveProvider(): Promise<void> {
    if (!selectedProvider) return;
    busyKey = 'provider';
    try {
      if (providerKey.trim()) {
        notice = await saveProviderKey(selectedProvider.id, providerKey.trim());
      }
      if (providerUrl.trim() && selectedProvider.is_local) {
        notice = await saveProviderUrl(selectedProvider.id, providerUrl.trim());
      }
      const result = await testProvider(selectedProvider.id);
      notice = result.status === 'ok'
        ? `${selectedProvider.display_name} connected`
        : `${selectedProvider.display_name}: ${result.error || 'connection failed'}`;
      step = Math.max(step, 2);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'wizard_provider_failed');
    } finally {
      busyKey = '';
    }
  }

  async function createAgentFromWizard(): Promise<void> {
    busyKey = 'agent';
    try {
      if (selectedTemplate?.name) {
        createdAgentId = await spawnTemplateAgent(selectedTemplate.name);
      } else {
        createdAgentId = (await createDraftAgent()).id;
      }
      notice = `Agent ready: ${createdAgentId}`;
      step = Math.max(step, 3);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'wizard_agent_failed');
    } finally {
      busyKey = '';
    }
  }

  async function saveChannel(): Promise<void> {
    if (!selectedChannel) return;
    busyKey = 'channel';
    try {
      if (selectedChannel.name === 'whatsapp' && selectedChannel.setup_type === 'qr') {
        qrState = await startWhatsappQr();
        notice = qrState.message || 'WhatsApp QR started';
      } else {
        const fields: Record<string, string> = {};
        for (const field of selectedChannel.fields) {
          const value = String(channelValues[field.key] || '').trim();
          if (!value && !field.required) continue;
          fields[field.key] = value;
        }
        notice = await configureChannel(selectedChannel.name, fields);
      }
      step = Math.max(step, 4);
      channels = await readChannels();
      syncChannelDefaults();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause || 'wizard_channel_failed');
    } finally {
      busyKey = '';
    }
  }

  function chooseChannel(name: string): void {
    selectedChannelName = name;
    qrState = null;
    syncChannelDefaults();
  }
</script>

<section class="page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native setup wizard</p>
      <h2>First-run provider, agent, and optional channel setup in the Svelte shell.</h2>
    </div>
    <div class="hero-actions">
      <a class="ghost" href={dashboardClassicHref('wizard')}>Open classic wizard</a>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <div class="steps">
    <div class:active={step >= 1}>1. Provider</div>
    <div class:active={step >= 2}>2. Agent</div>
    <div class:active={step >= 3}>3. Channel</div>
    <div class:active={step >= 4}>4. Finish</div>
  </div>

  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Provider</h3><span class="meta">{selectedProvider?.display_name || 'Select one'}</span></div>
      <select bind:value={selectedProviderId} class="field">
        {#each providers as provider}
          <option value={provider.id}>{provider.display_name}</option>
        {/each}
      </select>
      <input bind:value={providerKey} class="field" type="password" placeholder={selectedProvider?.api_key_env || 'API key'} />
      {#if selectedProvider?.is_local}
        <input bind:value={providerUrl} class="field" type="url" placeholder="Local provider URL" />
      {/if}
      <button class="primary small" type="button" disabled={busyKey === 'provider'} on:click={() => void saveProvider()}>{busyKey === 'provider' ? 'Saving…' : 'Save and test provider'}</button>
    </article>

    <article class="panel">
      <div class="panel-head"><h3>Agent</h3><span class="meta">{createdAgentId || 'Not created yet'}</span></div>
      <select bind:value={selectedTemplateName} class="field">
        {#each templates as template}
          <option value={template.name}>{template.name} · {template.category}</option>
        {/each}
      </select>
      <p class="summary">{selectedTemplate?.description || 'Use a template or create a draft agent.'}</p>
      <button class="primary small" type="button" disabled={busyKey === 'agent'} on:click={() => void createAgentFromWizard()}>{busyKey === 'agent' ? 'Creating…' : 'Create agent'}</button>
    </article>
  </div>

  <div class="grid">
    <article class="panel">
      <div class="panel-head"><h3>Optional channel</h3><span class="meta">{selectedChannel?.display_name || 'Select one'}</span></div>
      <div class="rows">
        {#each channels as channel}
          <button class:selected={selectedChannel?.name === channel.name} class="row picker" type="button" on:click={() => chooseChannel(channel.name)}>
            <div class="row-copy">
              <strong>{channel.display_name}</strong>
              <span>{channel.description || channel.name}</span>
            </div>
            <span>{channel.connected ? 'Ready' : 'Setup'}</span>
          </button>
        {/each}
      </div>
      {#if selectedChannel}
        <div class="form-grid">
          {#each selectedChannel.fields as field}
            <input
              class="field"
              type={field.type === 'secret' ? 'password' : 'text'}
              value={channelValues[field.key] || ''}
              placeholder={field.label}
              on:input={(event) => channelValues = { ...channelValues, [field.key]: (event.currentTarget as HTMLInputElement).value }}
            />
          {/each}
        </div>
        <button class="ghost small" type="button" disabled={busyKey === 'channel'} on:click={() => void saveChannel()}>{busyKey === 'channel' ? 'Saving…' : (selectedChannel.name === 'whatsapp' ? 'Start QR / Save channel' : 'Save channel')}</button>
        {#if qrState?.qr_data_url}
          <img alt="Channel QR code" class="qr" src={qrState.qr_data_url} />
        {/if}
      {/if}
    </article>

    <article class="panel">
      <div class="panel-head"><h3>Finish</h3><span class="meta">Native shell</span></div>
      <div class="rows">
        <div class="row"><div class="row-copy"><strong>Provider</strong><span>{selectedProvider?.display_name || 'Pending'}</span></div><span>{step >= 2 ? 'Done' : 'Next'}</span></div>
        <div class="row"><div class="row-copy"><strong>Agent</strong><span>{createdAgentId || 'Pending'}</span></div><span>{createdAgentId ? 'Done' : 'Next'}</span></div>
        <div class="row"><div class="row-copy"><strong>Channel</strong><span>{selectedChannel?.display_name || 'Optional'}</span></div><span>{step >= 4 ? 'Done' : 'Optional'}</span></div>
      </div>
      <div class="row-actions">
        <a class="ghost small" href={dashboardPageHref('agents')}>Open agents</a>
        <a class="primary small" href={dashboardPageHref('overview')}>Open overview</a>
      </div>
    </article>
  </div>
</section>

<style>
  .page, .grid, .rows, .steps, .form-grid { display: grid; gap: 18px; }
  .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .hero, .panel, .banner, .row, .field { border: 1px solid rgba(158,188,255,0.16); background: rgba(11,22,39,0.82); color: inherit; border-radius: 24px; }
  .hero, .panel, .banner { padding: 20px; }
  .hero, .hero-actions, .panel-head, .row, .row-actions { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .steps { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  .steps > div { padding: 10px 14px; border-radius: 18px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: #8aa4cf; }
  .steps > div.active { background: rgba(75,120,198,0.16); color: #dce6ff; }
  .row { padding: 12px 14px; background: rgba(255,255,255,0.04); }
  .picker { width: 100%; text-align: left; cursor: pointer; }
  .picker.selected { border-color: rgba(158,188,255,0.4); background: rgba(75,120,198,0.14); }
  .row-copy { display: grid; gap: 4px; }
  .field { padding: 0.75rem 0.85rem; font: inherit; width: 100%; }
  .ghost, .primary { padding: 0.75rem 0.95rem; border-radius: 16px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.04); color: inherit; text-decoration: none; }
  .small { padding: 0.5rem 0.75rem; }
  .eyebrow, .meta, span { color: #8aa4cf; }
  .notice { background: rgba(23,68,45,0.58); }
  .error { background: rgba(91,31,23,0.58); }
  .summary { margin: 0; color: #dce6ff; }
  .qr { width: min(220px, 100%); border-radius: 20px; border: 1px solid rgba(158,188,255,0.16); background: rgba(255,255,255,0.08); padding: 10px; }
  @media (max-width: 980px) { .grid, .steps { grid-template-columns: 1fr; } }
  @media (max-width: 760px) { .hero, .hero-actions, .row, .row-actions { flex-direction: column; align-items: flex-start; } }
</style>
