<script lang="ts">
  import type { DashboardModelRow } from '$lib/chat';
  import type { DashboardProviderRow, DashboardSystemInfo } from '$lib/settings';
  import {
    addCustomModel,
    deleteCustomModel,
    readProviders,
    readSettingsModels,
    readSystemInfo,
    removeProviderKey,
    saveProviderKey,
    saveProviderUrl,
    testProvider,
  } from '$lib/settings';
  import ModelCatalogPanel from '$lib/components/ModelCatalogPanel.svelte';
  import ProviderSettingsPanel from '$lib/components/ProviderSettingsPanel.svelte';
  import SystemInfoPanel from '$lib/components/SystemInfoPanel.svelte';
  import { onMount } from 'svelte';

  let providers: DashboardProviderRow[] = [];
  let models: DashboardModelRow[] = [];
  let systemInfo: DashboardSystemInfo | null = null;
  let providerKeyInputs: Record<string, string> = {};
  let providerUrlInputs: Record<string, string> = {};
  let providerTestResults: Record<string, { status: string; latency_ms: number; error: string }> = {};
  let customModelId = '';
  let customModelProvider = 'openrouter';
  let customModelContext = 128000;
  let customModelMaxOutput = 8192;
  let busyKey = '';
  let loading = true;
  let error = '';
  let notice = '';

  onMount(async () => {
    await refreshAll();
  });

  function setNotice(text: string): void { notice = String(text || '').trim(); error = ''; }
  function setErrorMessage(text: string): void { error = String(text || '').trim(); notice = ''; }

  async function refreshAll(): Promise<void> {
    loading = true;
    try {
      const [nextProviders, nextModels, nextInfo] = await Promise.all([
        readProviders(),
        readSettingsModels(),
        readSystemInfo(),
      ]);
      providers = nextProviders;
      models = nextModels;
      systemInfo = nextInfo;
      providerUrlInputs = Object.fromEntries(nextProviders.filter((row) => row.is_local).map((row) => [row.id, row.base_url || '']));
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'settings_unavailable'));
    } finally {
      loading = false;
    }
  }

  async function handleSaveKey(providerId: string): Promise<void> {
    const value = String(providerKeyInputs[providerId] || '').trim();
    if (!value || busyKey) return;
    busyKey = `key:${providerId}`;
    try {
      setNotice(await saveProviderKey(providerId, value));
      providerKeyInputs[providerId] = '';
      await refreshAll();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'provider_key_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleRemoveKey(providerId: string): Promise<void> {
    if (busyKey) return;
    busyKey = `remove-key:${providerId}`;
    try {
      setNotice(await removeProviderKey(providerId));
      await refreshAll();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'provider_key_remove_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleTestProvider(providerId: string): Promise<void> {
    if (busyKey) return;
    busyKey = `test:${providerId}`;
    try {
      providerTestResults = { ...providerTestResults, [providerId]: await testProvider(providerId) };
      const result = providerTestResults[providerId];
      if (result?.status === 'ok') setNotice(`${providerId} connected in ${result.latency_ms || 0}ms.`);
      else setErrorMessage(result?.error || `${providerId} test failed`);
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'provider_test_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleSaveUrl(providerId: string): Promise<void> {
    const value = String(providerUrlInputs[providerId] || '').trim();
    if (!value || busyKey) return;
    busyKey = `url:${providerId}`;
    try {
      setNotice(await saveProviderUrl(providerId, value));
      await refreshAll();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'provider_url_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleAddCustomModel(): Promise<void> {
    if (!String(customModelId || '').trim() || busyKey) return;
    busyKey = 'add-custom';
    try {
      setNotice(await addCustomModel({
        id: String(customModelId || '').trim(),
        provider: String(customModelProvider || 'openrouter').trim() || 'openrouter',
        context_window: Number(customModelContext || 128000) || 128000,
        max_output_tokens: Number(customModelMaxOutput || 8192) || 8192,
      }));
      customModelId = '';
      await refreshAll();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'custom_model_add_failed'));
    } finally {
      busyKey = '';
    }
  }

  async function handleDeleteCustomModel(modelId: string): Promise<void> {
    if (!modelId || busyKey) return;
    busyKey = `delete-model:${modelId}`;
    try {
      setNotice(await deleteCustomModel(modelId));
      await refreshAll();
    } catch (cause) {
      setErrorMessage(cause instanceof Error ? cause.message : String(cause || 'custom_model_delete_failed'));
    } finally {
      busyKey = '';
    }
  }
</script>

<section class="settings-page">
  <div class="hero">
    <div>
      <p class="eyebrow">Native settings</p>
      <h2>Provider setup, model catalog, and runtime defaults without the legacy host detour.</h2>
      <p class="hero-copy">
        This first native settings slice covers the everyday setup path. Advanced config, security, network, and migration tabs still live behind the classic escape hatch for now.
      </p>
    </div>
    <div class="hero-actions">
      <button class="ghost" type="button" on:click={() => void refreshAll()} disabled={loading}>
        {loading ? 'Refreshing…' : 'Refresh'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="banner error">{error}</div>
  {:else if notice}
    <div class="banner notice">{notice}</div>
  {/if}

  <div class="content-grid">
    <div class="column">
      <ProviderSettingsPanel
        {providers}
        keyInputs={providerKeyInputs}
        urlInputs={providerUrlInputs}
        testResults={providerTestResults}
        {busyKey}
        on:savekey={(event) => void handleSaveKey(event.detail.providerId)}
        on:removekey={(event) => void handleRemoveKey(event.detail.providerId)}
        on:testprovider={(event) => void handleTestProvider(event.detail.providerId)}
        on:saveurl={(event) => void handleSaveUrl(event.detail.providerId)}
      />
      <ModelCatalogPanel
        {models}
        {busyKey}
        bind:customModelId
        bind:customModelProvider
        bind:customModelContext
        bind:customModelMaxOutput
        on:addcustom={() => void handleAddCustomModel()}
        on:deletecustom={(event) => void handleDeleteCustomModel(event.detail.modelId)}
      />
    </div>
    <SystemInfoPanel info={systemInfo} />
  </div>
</section>

<style>
  .settings-page,
  .column {
    display: grid;
    gap: 18px;
  }
  .hero,
  .banner {
    border-radius: 24px;
    border: 1px solid rgba(158, 188, 255, 0.14);
    background: rgba(11, 22, 39, 0.82);
    backdrop-filter: blur(14px);
    padding: 20px;
  }
  .hero,
  .hero-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .content-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 18px;
  }
  h2,
  p {
    margin: 0;
  }
  .eyebrow,
  .hero-copy {
    color: #8aa4cf;
  }
  .ghost {
    border: 1px solid rgba(158, 188, 255, 0.16);
    background: rgba(255, 255, 255, 0.04);
    color: inherit;
    border-radius: 16px;
    padding: 0.8rem 1rem;
    text-decoration: none;
  }
  .error {
    border-color: rgba(229, 112, 93, 0.28);
    background: rgba(91, 31, 23, 0.58);
  }
  .notice {
    border-color: rgba(105, 165, 126, 0.24);
    background: rgba(23, 68, 45, 0.58);
  }
  @media (max-width: 1120px) {
    .content-grid {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 760px) {
    .hero,
    .hero-actions {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
