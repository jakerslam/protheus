const COMPONENT_TAG = 'infring-channels-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-channels-page-shell', shadow: 'none' }} />
<script>
  import { onMount, onDestroy } from 'svelte';

  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'channels';
  export let panelRole = 'page';
  export let routeContract = 'channels';
  export let parentOwnedData = false;

  const categories = [
    { key: 'all', label: 'All' },
    { key: 'messaging', label: 'Messaging' },
    { key: 'social', label: 'Social' },
    { key: 'enterprise', label: 'Enterprise' },
    { key: 'developer', label: 'Developer' },
    { key: 'notifications', label: 'Notifications' }
  ];
  const emptyQr = { loading: false, available: false, dataUrl: '', sessionId: '', message: '', help: '', connected: false, expired: false, error: '' };

  let allChannels = [];
  let showTemplateChannels = false;
  let categoryFilter = 'all';
  let searchQuery = '';
  let setupModal = null;
  let configuring = false;
  let testing = {};
  let formValues = {};
  let showAdvanced = false;
  let showBusinessApi = false;
  let loading = true;
  let loadError = '';
  let pollTimer = null;
  let setupStep = 1;
  let testPassed = false;
  let qr = Object.assign({}, emptyQr);
  let qrPollTimer = null;

  $: activeChannels = allChannels.filter(function(channel) {
    var tier = String(channel && channel.channel_tier || '').toLowerCase();
    var real = channel && Object.prototype.hasOwnProperty.call(channel, 'real_channel') ? !!channel.real_channel : (tier ? tier === 'native' : true);
    return showTemplateChannels || real;
  });
  $: filteredChannels = activeChannels.filter(function(channel) {
    if (categoryFilter !== 'all' && channel.category !== categoryFilter) return false;
    var needle = String(searchQuery || '').toLowerCase();
    if (!needle) return true;
    return String(channel.name || '').toLowerCase().indexOf(needle) !== -1 ||
      String(channel.display_name || '').toLowerCase().indexOf(needle) !== -1 ||
      String(channel.description || '').toLowerCase().indexOf(needle) !== -1;
  });
  $: basicFields = setupModal && Array.isArray(setupModal.fields) ? setupModal.fields.filter(function(field) { return !field.advanced; }) : [];
  $: advancedFields = setupModal && Array.isArray(setupModal.fields) ? setupModal.fields.filter(function(field) { return field.advanced; }) : [];
  $: isQr = !!(setupModal && setupModal.setup_type === 'qr');
  $: modalFields = isQr && showBusinessApi ? advancedFields : basicFields;

  function api() {
    return typeof window !== 'undefined' ? window.InfringAPI : null;
  }

  function toast() {
    return typeof window !== 'undefined' ? window.InfringToast : null;
  }

  function notifySuccess(message) {
    var t = toast();
    if (t && typeof t.success === 'function') t.success(message);
  }

  function notifyError(message) {
    var t = toast();
    if (t && typeof t.error === 'function') t.error(message);
  }

  function navigate(page) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(page);
    else if (typeof window !== 'undefined') window.location.hash = page;
  }

  function categoryCount(category) {
    var rows = activeChannels.filter(function(channel) { return category === 'all' || channel.category === category; });
    var configured = rows.filter(function(channel) { return channel.configured; });
    return configured.length + '/' + rows.length;
  }

  function statusBadge(channel) {
    if (!channel.configured) return { text: 'Not Configured', cls: 'badge-muted' };
    if (!channel.has_token) return { text: 'Missing Token', cls: 'badge-warn' };
    if (channel.connected) return { text: 'Ready', cls: 'badge-success' };
    return { text: 'Configured', cls: 'badge-info' };
  }

  function tierBadge(channel) {
    var tier = String(channel && channel.channel_tier || '').toLowerCase();
    var native = !!(channel && channel.real_channel) || tier === 'native';
    return native ? { text: 'Native', cls: 'badge-success' } : { text: 'Template', cls: 'badge-muted' };
  }

  function difficultyClass(value) {
    if (value === 'Easy') return 'difficulty-easy';
    if (value === 'Hard') return 'difficulty-hard';
    return 'difficulty-medium';
  }

  function fieldInputType(field) {
    if (field.type === 'secret') return 'password';
    if (field.type === 'number') return 'number';
    return 'text';
  }

  function fieldPlaceholder(field, advanced) {
    if (field.type === 'secret' && field.has_value) return advanced ? '******* (set)' : '******* (set - leave blank to keep)';
    if (field.type === 'list') return String(field.placeholder || '') + ' (comma-separated)';
    return String(field.placeholder || '');
  }

  async function loadChannels() {
    loading = true;
    loadError = '';
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var data = await client.get('/api/channels');
      allChannels = (Array.isArray(data && data.channels) ? data.channels : []).map(function(channel) {
        return Object.assign({}, channel, { connected: !!(channel.configured && channel.has_token) });
      });
    } catch (e) {
      allChannels = [];
      loadError = e && e.message ? e.message : 'Could not load channels.';
    }
    loading = false;
    startPolling();
  }

  function startPolling() {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(refreshStatus, 15000);
  }

  async function refreshStatus() {
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') return;
      var data = await client.get('/api/channels');
      var byName = {};
      (Array.isArray(data && data.channels) ? data.channels : []).forEach(function(channel) { byName[channel.name] = channel; });
      allChannels = allChannels.map(function(channel) {
        var fresh = byName[channel.name];
        if (!fresh) return channel;
        return Object.assign({}, channel, fresh, { connected: !!(fresh.configured && fresh.has_token) });
      });
    } catch (_) {}
  }

  function resetQR() {
    qr = Object.assign({}, emptyQr);
    if (qrPollTimer) {
      clearInterval(qrPollTimer);
      qrPollTimer = null;
    }
  }

  function closeSetup() {
    setupModal = null;
    resetQR();
  }

  function openSetup(channel) {
    setupModal = channel;
    var values = {};
    (Array.isArray(channel.fields) ? channel.fields : []).forEach(function(field) {
      if (field.value !== undefined && field.value !== null && field.type !== 'secret') values[field.key] = String(field.value);
    });
    formValues = values;
    showAdvanced = false;
    showBusinessApi = false;
    setupStep = channel.configured ? 3 : 1;
    testPassed = !!channel.configured;
    resetQR();
    if (channel.setup_type === 'qr') startQR();
  }

  async function startQR() {
    qr = Object.assign({}, qr, { loading: true, error: '', connected: false, expired: false });
    try {
      var client = api();
      if (!client || typeof client.post !== 'function') throw new Error('Shell API client is unavailable.');
      var result = await client.post('/api/channels/whatsapp/qr/start', {});
      qr = {
        loading: false,
        available: !!result.available,
        dataUrl: String(result.qr_data_url || ''),
        sessionId: String(result.session_id || ''),
        message: String(result.message || ''),
        help: String(result.help || ''),
        connected: !!result.connected,
        expired: false,
        error: ''
      };
      if (qr.available && qr.dataUrl && !qr.connected) pollQR();
      if (qr.connected) {
        notifySuccess('WhatsApp connected!');
        await refreshStatus();
      }
    } catch (e) {
      qr = Object.assign({}, qr, { loading: false, error: e && e.message ? e.message : 'Could not start QR login' });
    }
  }

  function pollQR() {
    if (qrPollTimer) clearInterval(qrPollTimer);
    qrPollTimer = setInterval(async function() {
      try {
        var client = api();
        if (!client || typeof client.get !== 'function') return;
        var result = await client.get('/api/channels/whatsapp/qr/status?session_id=' + encodeURIComponent(qr.sessionId));
        if (result.connected) {
          clearInterval(qrPollTimer);
          qrPollTimer = null;
          qr = Object.assign({}, qr, { connected: true, message: String(result.message || 'Connected!') });
          notifySuccess('WhatsApp linked successfully!');
          await refreshStatus();
        } else if (result.expired) {
          clearInterval(qrPollTimer);
          qrPollTimer = null;
          qr = Object.assign({}, qr, { expired: true, message: 'QR code expired. Click to generate a new one.' });
        } else {
          qr = Object.assign({}, qr, { message: String(result.message || 'Waiting for scan...') });
        }
      } catch (_) {}
    }, 3000);
  }

  async function saveChannel() {
    if (!setupModal) return;
    var name = setupModal.name;
    configuring = true;
    try {
      var client = api();
      if (!client || typeof client.post !== 'function') throw new Error('Shell API client is unavailable.');
      await client.post('/api/channels/' + name + '/configure', { fields: formValues });
      setupStep = 2;
      try {
        var testResult = await client.post('/api/channels/' + name + '/test', { force_live: true });
        if (testResult.status === 'ok') {
          testPassed = true;
          setupStep = 3;
          notifySuccess(setupModal.display_name + ' activated!');
        } else {
          notifySuccess(setupModal.display_name + ' saved. ' + String(testResult.message || ''));
        }
      } catch (_) {
        notifySuccess(setupModal.display_name + ' saved. Test to verify connection.');
      }
      await refreshStatus();
    } catch (e) {
      notifyError('Failed: ' + (e && e.message ? e.message : 'Unknown error'));
    }
    configuring = false;
  }

  async function testChannel() {
    if (!setupModal) return;
    var name = setupModal.name;
    testing = Object.assign({}, testing, { [name]: true });
    try {
      var client = api();
      if (!client || typeof client.post !== 'function') throw new Error('Shell API client is unavailable.');
      var result = await client.post('/api/channels/' + name + '/test', { force_live: true });
      if (result.status === 'ok') {
        testPassed = true;
        setupStep = 3;
        notifySuccess(String(result.message || 'Connection verified'));
      } else {
        notifyError(String(result.message || 'Connection test failed'));
      }
    } catch (e) {
      notifyError('Test failed: ' + (e && e.message ? e.message : 'Unknown error'));
    }
    testing = Object.assign({}, testing, { [name]: false });
  }

  async function removeChannel() {
    if (!setupModal) return;
    var name = setupModal.name;
    var displayName = setupModal.display_name;
    var t = toast();
    var run = async function() {
      try {
        var client = api();
        if (!client || typeof client.delete !== 'function') throw new Error('Shell API client is unavailable.');
        await client.delete('/api/channels/' + name + '/configure');
        notifySuccess(displayName + ' removed and deactivated.');
        await refreshStatus();
        closeSetup();
      } catch (e) {
        notifyError('Failed: ' + (e && e.message ? e.message : 'Unknown error'));
      }
    };
    if (t && typeof t.confirm === 'function') t.confirm('Remove Channel', 'Remove ' + displayName + ' configuration? This will deactivate the channel.', run);
    else await run();
  }

  onMount(loadChannels);
  onDestroy(function() {
    if (pollTimer) clearInterval(pollTimer);
    if (qrPollTimer) clearInterval(qrPollTimer);
  });
</script>

<div class="page-header page-header-subtabs-center">
  <div class="tabs mt-3" role="tablist">
    <button class="tab" role="tab" on:click={() => navigate('skills')}>Apps</button>
    <button class="tab active" role="tab" on:click={() => navigate('channels')}>Channels</button>
    <button class="tab" role="tab" on:click={() => navigate('eyes')}>Eyes</button>
    <button class="tab" role="tab" on:click={() => navigate('hands')}>Hands</button>
  </div>
</div>

<div class="page-body">
  {#if loading}
    <div class="loading-state"><div class="spinner"></div><span>Loading channels...</span></div>
  {:else if loadError}
    <div class="error-state"><span class="error-icon">!</span><p>{loadError}</p><button class="btn btn-ghost btn-sm" on:click={loadChannels}>Retry</button></div>
  {:else}
    <div class="flex gap-2 mb-4" style="flex-wrap:wrap">
      {#each categories as cat (cat.key)}
        <button class:btn-primary={categoryFilter === cat.key} class:btn-ghost={categoryFilter !== cat.key} class="btn btn-sm" on:click={() => categoryFilter = cat.key}>{cat.label} ({categoryCount(cat.key)})</button>
      {/each}
      <button class:btn-primary={showTemplateChannels} class:btn-ghost={!showTemplateChannels} class="btn btn-sm" title={showTemplateChannels ? 'Hide generic templates' : 'Show generic template channels'} on:click={() => showTemplateChannels = !showTemplateChannels}>{showTemplateChannels ? 'Hide Templates' : 'Show Templates'}</button>
    </div>
    <div class="mb-4"><input class="form-input" type="text" placeholder="Search channels..." bind:value={searchQuery} style="max-width:400px"></div>
    <div class="card-grid">
      {#each filteredChannels as channel (channel.name)}
        <div class:card-unconfigured={!channel.configured} class="card" style="cursor:pointer" on:click={() => openSetup(channel)}>
          <div class="flex justify-between items-center mb-2">
            <div class="flex items-center gap-2"><span class="channel-icon">{channel.icon}</span><div class="card-header" style="margin:0">{channel.display_name}</div></div>
            <div class="flex items-center gap-1"><span class={'badge ' + tierBadge(channel).cls}>{tierBadge(channel).text}</span><span class={'badge ' + statusBadge(channel).cls}>{statusBadge(channel).text}</span></div>
          </div>
          <div class="card-meta">{channel.description}</div>
          <div class="flex justify-between items-center mt-2">
            <span class={'difficulty-badge ' + difficultyClass(channel.difficulty)}>{channel.difficulty} - {channel.setup_time}</span>
            <button class="btn btn-ghost btn-sm" on:click|stopPropagation={() => openSetup(channel)}>{channel.configured ? 'Edit' : 'Set up'}</button>
          </div>
        </div>
      {/each}
    </div>
    {#if filteredChannels.length === 0}<div class="text-dim mt-4" style="text-align:center"><p>No channels match your search.</p></div>{/if}
  {/if}

  {#if setupModal}
    <div class="modal-overlay" on:click={closeSetup}>
      <div class="modal" style="max-width:480px" on:click|stopPropagation>
        <div class="modal-header">
          <div><h3 style="display:flex;align-items:center;gap:0.5rem"><span class="channel-icon" style="font-size:1rem">{setupModal.icon}</span><span>{setupModal.display_name}</span></h3><div class="text-xs text-dim mt-1">{setupModal.quick_setup || setupModal.description}</div></div>
          <button class="modal-close" on:click={closeSetup}>&times;</button>
        </div>
        {#if !isQr}
          <div class="channel-steps">
            {#each [1, 2, 3] as step, i}
              <div class="channel-step-item"><div class:active={setupStep === step} class:done={setupStep > step || (step === 3 && setupStep >= 3)} class="channel-step-num">{setupStep > step || (step === 3 && setupStep >= 3) ? '\u2713' : step}</div><span class:active={setupStep === step} class:done={setupStep > step || (step === 3 && setupStep >= 3)} class="channel-step-label">{step === 1 ? 'Configure' : step === 2 ? 'Verify' : 'Ready'}</span></div>
              {#if i < 2}<div class:done={setupStep > step} class="channel-step-line"></div>{/if}
            {/each}
          </div>
        {/if}
        {#if !isQr && setupStep === 3 && testPassed}
          <div class="ready-panel"><div class="ready-panel-icon">&#10003;</div><div class="ready-panel-title">{setupModal.display_name} is ready!</div><div class="ready-panel-desc">Your channel is configured and verified. It will activate automatically.</div><div class="flex gap-2 mt-4" style="justify-content:center"><button class="btn btn-ghost btn-sm" on:click={() => setupStep = 1}>Edit Config</button><button class="btn btn-primary btn-sm" on:click={closeSetup}>Done</button></div></div>
        {:else if isQr && !showBusinessApi}
          {#if qr.loading}<div style="text-align:center;padding:2rem 0"><div class="spinner"></div><p class="text-sm text-dim mt-2">Connecting to WhatsApp Web gateway...</p></div>{/if}
          {#if !qr.loading && qr.available && qr.dataUrl && !qr.connected}<div style="text-align:center"><div style="background:#fff;display:inline-block;padding:1rem;border-radius:12px;margin:0.5rem 0"><img src={qr.dataUrl} alt="WhatsApp QR Code" style="width:256px;height:256px;image-rendering:pixelated"></div><ol style="text-align:left;font-size:0.85rem;margin:1rem 0;padding-left:1.5rem;opacity:0.8">{#each setupModal.setup_steps || [] as step}<li style="margin-bottom:0.25rem">{step}</li>{/each}</ol><p class="text-xs text-dim">{qr.message}</p>{#if qr.expired}<button class="btn btn-ghost btn-sm mt-2" on:click={startQR}>Generate New QR</button>{/if}</div>{/if}
          {#if !qr.loading && qr.connected}<div style="text-align:center;padding:2rem 0"><div style="font-size:3rem;margin-bottom:0.5rem">&#10003;</div><p class="text-sm" style="font-weight:600">{qr.message || 'WhatsApp linked successfully!'}</p><p class="text-xs text-dim mt-1">Channel will activate automatically.</p></div>{/if}
          {#if !qr.loading && !qr.available}<div style="padding:1rem 0"><div style="background:var(--bg-secondary,#1a1a2e);border-radius:8px;padding:1.25rem;text-align:center"><div style="font-size:2rem;margin-bottom:0.5rem;opacity:0.5">Phone</div><p class="text-sm">{qr.message || 'WhatsApp Web gateway not available'}</p>{#if qr.help}<p class="text-xs text-dim mt-2">{qr.help}</p>{/if}{#if qr.error}<p class="text-xs text-dim mt-1" style="color:var(--red,#ef4444)">{qr.error}</p>{/if}</div><p class="text-xs text-dim mt-3" style="text-align:center">Or use the <button class="btn-link" on:click={() => showBusinessApi = true} style="font-size:inherit;text-decoration:underline;cursor:pointer;background:none;border:none;color:var(--accent,#818cf8);padding:0">Business API</button> with a Meta developer account.</p></div>{/if}
          {#if !qr.loading && qr.available}<div class="text-xs text-dim mt-2" style="text-align:center">Have a Meta Business account? <button class="btn-link" on:click={() => showBusinessApi = true} style="font-size:inherit;text-decoration:underline;cursor:pointer;background:none;border:none;color:var(--accent,#818cf8);padding:0">Use Business API instead</button></div>{/if}
          {#if !qr.loading}<div class="flex gap-2 mt-4" style="flex-wrap:wrap;justify-content:center">{#if qr.available && !qr.connected && !qr.expired}<button class="btn btn-ghost" on:click={startQR}>Refresh QR</button>{/if}{#if setupModal.configured}<button class="btn btn-ghost" on:click={testChannel} disabled={testing[setupModal.name]}>{testing[setupModal.name] ? 'Testing...' : 'Test Connection'}</button><button class="btn btn-ghost" style="color:var(--red,#ef4444)" on:click={removeChannel}>Remove</button>{/if}</div>{/if}
        {:else}
          {#if isQr && showBusinessApi}<div class="mb-3"><button class="btn btn-ghost btn-sm" on:click={() => showBusinessApi = false} style="font-size:0.8rem">&larr; Back to QR scan</button><p class="text-xs text-dim mt-1">Configure via WhatsApp Cloud API (requires a Meta Business developer account).</p></div>{/if}
          {#if !isQr}<details class="mb-4" style="font-size:0.8rem"><summary class="text-dim" style="cursor:pointer">How to get credentials</summary><ol class="setup-steps" style="margin-top:0.5rem">{#each setupModal.setup_steps || [] as step}<li class="text-sm">{step}</li>{/each}</ol></details>{/if}
          {#each modalFields as field (field.key)}
            <div style="margin-bottom:0.75rem"><label class="text-sm" style="display:block;margin-bottom:0.25rem">{field.label}{field.required ? ' *' : ''}</label>{#if field.type === 'secret'}<input class="form-input" type="password" bind:value={formValues[field.key]} placeholder={fieldPlaceholder(field, false)} required={field.required}>{:else if field.type === 'number'}<input class="form-input" type="number" bind:value={formValues[field.key]} placeholder={fieldPlaceholder(field, false)} required={field.required}>{:else}<input class="form-input" type="text" bind:value={formValues[field.key]} placeholder={fieldPlaceholder(field, false)} required={field.required}>{/if}{#if field.env_var && field.has_value}<div class="text-xs text-dim" style="margin-top:2px">{field.env_var} is set</div>{/if}</div>
          {/each}
          {#if !isQr && advancedFields.length}<button class="btn btn-ghost btn-sm mb-2" on:click={() => showAdvanced = !showAdvanced}>{showAdvanced ? 'Hide advanced' : 'Show advanced (' + advancedFields.length + ')'}</button>{/if}
          {#if showAdvanced}<div>{#each advancedFields as field (field.key)}<div style="margin-bottom:0.75rem"><label class="text-sm text-dim" style="display:block;margin-bottom:0.25rem">{field.label}</label>{#if field.type === 'secret'}<input class="form-input" type="password" bind:value={formValues[field.key]} placeholder={fieldPlaceholder(field, true)}>{:else if field.type === 'number'}<input class="form-input" type="number" bind:value={formValues[field.key]} placeholder={fieldPlaceholder(field, true)}>{:else}<input class="form-input" type="text" bind:value={formValues[field.key]} placeholder={fieldPlaceholder(field, true)}>{/if}</div>{/each}</div>{/if}
          <div class="flex gap-2 mt-4" style="flex-wrap:wrap"><button class="btn btn-primary" on:click={saveChannel} disabled={configuring}>{configuring ? 'Saving...' : (setupModal.configured ? 'Update' : 'Save & Test')}</button>{#if setupModal.configured}<button class="btn btn-ghost" on:click={testChannel} disabled={testing[setupModal.name]}>{testing[setupModal.name] ? 'Testing...' : 'Test'}</button><button class="btn btn-ghost" style="color:var(--red,#ef4444)" on:click={removeChannel}>Remove</button>{/if}</div>
        {/if}
      </div>
    </div>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
