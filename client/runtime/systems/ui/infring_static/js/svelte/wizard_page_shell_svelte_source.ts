const COMPONENT_TAG = 'infring-wizard-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-wizard-page-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'wizard';
  export let panelRole = 'page';
  export let routeContract = 'wizard';
  export let parentOwnedData = false;

  let vm = null;
  let loadError = '';

  $: progressSteps = vm ? Array.from({ length: Number(vm.totalSteps || 6) }, function(_, index) { return index + 1; }) : [];
  $: selectedProvider = vm ? vm.selectedProviderObj : null;
  $: selectedChannel = vm ? vm.selectedChannelObj : null;
  $: selectedTemplate = vm && vm.templates ? vm.templates[Number(vm.selectedTemplate || 0)] : null;
  $: providerHelp = vm && selectedProvider && typeof vm.providerHelp === 'function' ? vm.providerHelp(vm.selectedProvider) : null;
  $: popularProviders = vm && Array.isArray(vm.popularProviders) ? vm.popularProviders : [];
  $: otherProviders = vm && Array.isArray(vm.otherProviders) ? vm.otherProviders : [];
  $: filteredTemplates = vm && Array.isArray(vm.filteredTemplates) ? vm.filteredTemplates : [];
  $: templateCategories = vm && Array.isArray(vm.templateCategories) ? vm.templateCategories : [];
  $: currentSuggestions = vm && Array.isArray(vm.currentSuggestions) ? vm.currentSuggestions : [];

  function refresh() {
    vm = vm;
  }

  function createFallbackWizard() {
    return {
      step: 1,
      totalSteps: 6,
      loading: false,
      error: 'Setup wizard controller is unavailable.',
      providers: [],
      templates: [],
      channelOptions: [],
      setupSummary: {},
      stepLabel: function(step) { return ['Welcome', 'Provider', 'Agent', 'Try It', 'Channel', 'Done'][step - 1] || ''; },
      finishAndDismiss: function() { window.location.hash = 'overview'; }
    };
  }

  async function initialize() {
    try {
      var factory = typeof window !== 'undefined' ? window.wizardPage : null;
      vm = typeof factory === 'function' ? factory() : createFallbackWizard();
      refresh();
      if (vm && typeof vm.loadData === 'function') await invoke('loadData');
    } catch (error) {
      loadError = error && error.message ? error.message : 'Could not start setup wizard.';
      vm = createFallbackWizard();
    }
  }

  async function invoke(name) {
    if (!vm || typeof vm[name] !== 'function') return;
    var args = Array.prototype.slice.call(arguments, 1);
    try {
      var result = vm[name].apply(vm, args);
      refresh();
      if (result && typeof result.then === 'function') await result;
    } finally {
      refresh();
    }
  }

  function setField(name, value) {
    if (!vm) return;
    vm[name] = value;
    refresh();
  }

  function providerReady(provider) {
    return !!(vm && typeof vm.providerIsConfigured === 'function' && vm.providerIsConfigured(provider));
  }

  function profileInfo(name) {
    return vm && typeof vm.profileInfo === 'function' ? vm.profileInfo(name) : { label: name || '', desc: '' };
  }

  function templateIndex(template) {
    return vm && Array.isArray(vm.templates) ? vm.templates.indexOf(template) : -1;
  }

  function selectTemplate(template) {
    var index = templateIndex(template);
    if (index >= 0) invoke('selectTemplate', index);
  }

  function sendTryIt(value) {
    invoke('sendTryItMessage', value);
  }

  onMount(initialize);
</script>

{#if !vm}
  <div class="page-body"><div class="loading-state"><div class="spinner"></div><span>Loading setup wizard...</span></div></div>
{:else}
  <div class="page-header">
    <h2>Setup Wizard</h2>
    <button class="btn btn-ghost btn-sm" on:click={() => invoke('finishAndDismiss')}>Skip Setup</button>
  </div>
  <div class="page-body">
    {#if vm.loading}
      <div class="loading-state"><div class="spinner"></div><span>Loading...</span></div>
    {:else if loadError || vm.error}
      <div class="error-state">
        <span class="error-icon">!</span>
        <p>{loadError || vm.error}</p>
        <button class="btn btn-ghost btn-sm" on:click={() => invoke('loadData')}>Retry</button>
      </div>
    {:else}
      <div class="wizard-progress">
        {#each progressSteps as stepNumber}
          <div class:wiz-active={vm.step === stepNumber} class:wiz-done={vm.step > stepNumber} class="wizard-progress-step" on:click={() => invoke('goToStep', stepNumber)}>
            <div class="wizard-progress-circle">{vm.step > stepNumber ? String.fromCharCode(10003) : stepNumber}</div>
            <span class="wizard-progress-label">{vm.stepLabel(stepNumber)}</span>
          </div>
        {/each}
        <div class="wizard-progress-line"><div class="wizard-progress-line-fill" style={'width:' + (((vm.step - 1) / Math.max(1, progressSteps.length - 1)) * 100) + '%'}></div></div>
      </div>

      {#if vm.step === 1}
        <div class="wizard-step">
          <div class="wizard-card" style="text-align:center;max-width:600px;margin:0 auto">
            <div class="logo-hero-wordmark">INFRING</div>
            <h3 style="font-size:22px;font-weight:700;margin-bottom:12px;color:var(--accent)">Welcome to Infring</h3>
            <p style="font-size:13px;color:var(--text-dim);line-height:1.8;max-width:480px;margin:0 auto 24px">Infring is an open-source Agent Operating System. It lets you run AI agents that can chat, use tools, access memory, and connect to messaging channels, all from a single shell.</p>
            <div class="card" style="text-align:left;margin-bottom:20px">
              <div class="card-header">This wizard will help you:</div>
              {#each ['Connect an LLM provider (Frontier Provider, OpenAI, Gemini, etc.)', 'Create your first AI agent from 10 templates', 'Try it out with a quick test message', 'Optionally connect a messaging channel (Telegram, Discord, Slack)'] as item, index}
                <div style="display:flex;align-items:center;gap:10px;padding:8px 0;border-bottom:{index < 3 ? '1px solid var(--border)' : '0'}">
                  <span class="badge badge-info" style="min-width:20px;justify-content:center">{index + 1}</span>
                  <span style="font-size:12px">{item}</span>
                </div>
              {/each}
            </div>
            <p style="font-size:11px;color:var(--text-muted)">Takes about 2 minutes. You can skip any step and configure later.</p>
          </div>
          <div class="wizard-nav"><div></div><button class="btn btn-primary" on:click={() => invoke('nextStep')}>Get Started</button></div>
        </div>
      {:else if vm.step === 2}
        <div class="wizard-step">
          <div class="wizard-card">
            <h3 style="font-size:16px;font-weight:700;margin-bottom:4px">Connect an LLM Provider</h3>
            <p style="font-size:12px;color:var(--text-dim);margin-bottom:16px;line-height:1.6">Infring needs at least one LLM provider to power your agents. Select a provider and enter your API key.</p>
            {#if vm.hasConfiguredProvider}<div class="info-card" style="border-left-color:var(--success)"><h4 style="color:var(--success)">Provider Already Configured</h4><p>You already have at least one provider set up. You can continue or configure additional providers.</p></div>{/if}
            <div style="margin-bottom:16px">
              <div class="text-xs font-bold text-dim mb-2" style="text-transform:uppercase;letter-spacing:0.5px">Popular Providers</div>
              <div class="card-grid" style="grid-template-columns:repeat(auto-fill, minmax(200px, 1fr))">
                {#each popularProviders as provider}
                  <div class:wizard-provider-selected={vm.selectedProvider === provider.id} class:configured={providerReady(provider)} class="card wizard-provider-card provider-card" on:click={() => invoke('selectProvider', provider.id)} style="cursor:pointer;padding:12px">
                    <div class="flex justify-between items-center"><span class="font-bold" style="font-size:13px">{provider.display_name}</span>{#if providerReady(provider)}<span class="badge badge-success" style="font-size:8px">READY</span>{/if}</div>
                    <div class="text-xs text-dim mt-1">{provider.model_count || 0} models</div>
                  </div>
                {/each}
              </div>
            </div>
            {#if otherProviders.length}
              <div style="margin-bottom:16px">
                <div class="text-xs font-bold text-dim mb-2" style="text-transform:uppercase;letter-spacing:0.5px">Other Providers</div>
                <div class="card-grid" style="grid-template-columns:repeat(auto-fill, minmax(200px, 1fr))">
                  {#each otherProviders as provider}
                    <div class:wizard-provider-selected={vm.selectedProvider === provider.id} class:configured={providerReady(provider)} class="card wizard-provider-card provider-card" on:click={() => invoke('selectProvider', provider.id)} style="cursor:pointer;padding:12px">
                      <div class="flex justify-between items-center"><span class="font-bold" style="font-size:13px">{provider.display_name}</span>{#if providerReady(provider)}<span class="badge badge-success" style="font-size:8px">READY</span>{/if}</div>
                      <div class="text-xs text-dim mt-1">{provider.model_count || 0} models</div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedProvider && !providerReady(selectedProvider) && vm.selectedProvider === 'claude-code'}
              <div class="card" style="border-left:3px solid var(--accent);margin-top:16px">
                <div class="card-header">Configure Claude Code</div>
                <div class="text-xs text-dim mb-2" style="line-height:1.8">Claude Code uses its own CLI authentication, no API key needed.</div>
                <div style="background:var(--bg);border-radius:4px;padding:10px 12px;margin-bottom:12px;font-size:12px;line-height:1.8">
                  <div><span style="color:var(--accent)">1.</span> Install: <code>npm install -g @frontier_provider-ai/claude-code</code></div>
                  <div><span style="color:var(--accent)">2.</span> Authenticate: <code>claude auth</code></div>
                  <div><span style="color:var(--accent)">3.</span> Click <strong>Detect</strong> below to verify</div>
                </div>
                <button class="btn btn-primary btn-sm" disabled={vm.testingProvider} on:click={() => invoke('detectClaudeCode')}>{vm.testingProvider ? 'Detecting...' : 'Detect Claude Code'}</button>
              </div>
            {:else if selectedProvider && !providerReady(selectedProvider)}
              <div class="card" style="border-left:3px solid var(--accent);margin-top:16px">
                <div class="card-header">Configure {selectedProvider.display_name}</div>
                {#if selectedProvider.api_key_env}<div class="text-xs text-dim mb-2">Environment variable: <code>{selectedProvider.api_key_env}</code></div>{/if}
                {#if providerHelp}<div class="text-xs mb-3" style="color:var(--accent-light)"><a href={providerHelp.url} target="_blank" rel="noopener" style="color:var(--accent);text-decoration:underline">{providerHelp.text}</a></div>{/if}
                <div class="form-group">
                  <label>API Key</label>
                  <div class="key-input-group">
                    <input type="password" placeholder={'Enter your ' + selectedProvider.display_name + ' API key'} value={vm.apiKeyInput || ''} on:input={(event) => setField('apiKeyInput', event.currentTarget.value)} on:keydown={(event) => { if (event.key === 'Enter') invoke('saveKey'); }}>
                    <button class="btn btn-primary btn-sm" disabled={vm.savingKey || !(vm.apiKeyInput || '').trim()} on:click={() => invoke('saveKey')}>{vm.savingKey ? 'Saving...' : 'Save & Test'}</button>
                  </div>
                </div>
              </div>
            {:else if selectedProvider}
              <div class="card" style="border-left:3px solid var(--success);margin-top:16px">
                <div class="flex items-center gap-2"><span style="color:var(--success);font-size:18px">&#10003;</span><div><div class="font-bold" style="font-size:13px">{selectedProvider.display_name} is configured and ready</div><div class="text-xs text-dim">You can test the connection or continue to the next step.</div></div></div>
                <div class="flex gap-2 mt-2"><button class="btn btn-ghost btn-sm" disabled={vm.testingProvider} on:click={() => invoke('testKey')}>{vm.testingProvider ? 'Testing...' : 'Test Connection'}</button></div>
              </div>
            {/if}
            {#if vm.testResult}<div class="mt-2"><div class={vm.testResult.status === 'ok' ? 'badge badge-success' : 'badge badge-error'} style="padding:6px 12px">{vm.testResult.status === 'ok' ? 'Connected successfully' : (vm.testResult.error || 'Connection failed')}</div></div>{/if}
          </div>
          <div class="wizard-nav"><button class="btn btn-ghost" on:click={() => invoke('prevStep')}>Back</button><button class="btn btn-primary" disabled={!vm.canGoNext} on:click={() => invoke('nextStep')}>{vm.hasConfiguredProvider || vm.keySaved ? 'Next' : 'Skip'}</button></div>
        </div>
      {:else if vm.step === 3}
        <div class="wizard-step">
          <div class="wizard-card">
            <h3 style="font-size:16px;font-weight:700;margin-bottom:4px">Create Your First Agent</h3>
            <p style="font-size:12px;color:var(--text-dim);margin-bottom:16px;line-height:1.6">Pick a template to get started quickly. You can customize the agent later or create more from the Agents page.</p>
            <div class="wizard-category-pills">{#each templateCategories as category}<button class:active={vm.templateCategory === category} class="wizard-category-pill" on:click={() => setField('templateCategory', category)}>{category}</button>{/each}</div>
            <div class="card-grid" style="grid-template-columns:repeat(auto-fill, minmax(220px, 1fr));margin-bottom:20px">
              {#each filteredTemplates as template}
                <div class:wizard-template-selected={Number(vm.selectedTemplate || 0) === templateIndex(template)} class="card wizard-template-card" on:click={() => selectTemplate(template)} style="cursor:pointer">
                  <div class="flex items-center gap-2 mb-2"><span class="channel-icon" style="background:var(--accent);color:var(--bg-primary);font-weight:700">{template.icon}</span><div><span class="font-bold" style="font-size:13px">{template.name}</span><span class="category-badge" style="margin-left:6px">{template.category}</span></div></div>
                  <div class="text-xs text-dim" style="line-height:1.6">{template.description}</div>
                  <div class="flex justify-between items-center mt-2"><span class="text-xs" style="color:var(--text-muted)">{template.provider} / {template.model}</span><span class="badge badge-muted">{template.profile}</span></div>
                  {#if profileInfo(template.profile).desc}<div class="text-xs mt-1 text-dim">{profileInfo(template.profile).desc}</div>{/if}
                </div>
              {/each}
            </div>
            <div class="card" style="border-left:3px solid var(--accent)">
              <div class="form-group" style="margin-bottom:8px"><label>Agent Name</label><input class="form-input" type="text" value={vm.agentName || ''} placeholder="my-assistant" style="max-width:320px" on:input={(event) => setField('agentName', event.currentTarget.value)} on:keydown={(event) => { if (event.key === 'Enter' && !event.isComposing) invoke('createAgent'); }}></div>
              {#if selectedTemplate}<div class="text-xs text-dim">Will use {selectedTemplate.provider} / {selectedTemplate.model} with {profileInfo(selectedTemplate.profile).label} profile</div>{/if}
              <div class="mt-2"><button class="btn btn-primary" disabled={vm.creatingAgent || !(vm.agentName || '').trim()} on:click={() => invoke('createAgent')}>{vm.creatingAgent ? 'Creating...' : 'Create Agent'}</button></div>
              {#if vm.createdAgent}<div class="mt-2"><div class="badge badge-success" style="padding:6px 12px">Agent "{vm.createdAgent.name}" created successfully</div></div>{/if}
            </div>
          </div>
          <div class="wizard-nav"><button class="btn btn-ghost" on:click={() => invoke('prevStep')}>Back</button><button class="btn btn-primary" on:click={() => invoke('nextStep')}>{vm.createdAgent ? 'Next: Try It' : 'Skip'}</button></div>
        </div>
      {:else if vm.step === 4}
        <div class="wizard-step">
          <div class="wizard-card">
            <h3 style="font-size:16px;font-weight:700;margin-bottom:4px">Try Your Agent</h3>
            <p style="font-size:12px;color:var(--text-dim);margin-bottom:16px;line-height:1.6">Send a quick message to test your new agent. Try one of the suggestions below or type your own.</p>
            <div style="display:flex;flex-wrap:wrap;gap:6px;margin-bottom:12px">{#each currentSuggestions as suggestion}<button class="suggest-chip" disabled={vm.tryItSending} on:click={() => sendTryIt(suggestion)}>{suggestion}</button>{/each}</div>
            <div class="tryit-messages" style="min-height:60px">
              {#each vm.tryItMessages || [] as message}<div class={message.role === 'user' ? 'tryit-msg tryit-msg-user' : 'tryit-msg tryit-msg-agent'}>{message.text}</div>{/each}
              {#if vm.tryItSending}<div class="tryit-msg tryit-msg-agent" style="opacity:0.5">Thinking...</div>{/if}
            </div>
            <div style="display:flex;gap:8px;margin-top:12px"><input class="form-input" type="text" value={vm.tryItInput || ''} placeholder="Type a message..." disabled={vm.tryItSending} style="flex:1" on:input={(event) => setField('tryItInput', event.currentTarget.value)} on:keydown={(event) => { if (event.key === 'Enter' && !event.isComposing) sendTryIt(vm.tryItInput); }}><button class="btn btn-primary btn-sm" disabled={vm.tryItSending || !(vm.tryItInput || '').trim()} on:click={() => sendTryIt(vm.tryItInput)}>Send</button></div>
          </div>
          <div class="wizard-nav"><button class="btn btn-ghost" on:click={() => invoke('prevStep')}>Back</button><button class="btn btn-primary" on:click={() => invoke('nextStep')}>Continue</button></div>
        </div>
      {:else if vm.step === 5}
        <div class="wizard-step">
          <div class="wizard-card">
            <h3 style="font-size:16px;font-weight:700;margin-bottom:4px">Connect a Channel <span class="badge badge-muted">Optional</span></h3>
            <p style="font-size:12px;color:var(--text-dim);margin-bottom:16px;line-height:1.6">Channels let your agent communicate via messaging platforms. This is optional; you can always use the built-in web chat.</p>
            <div class="card-grid" style="grid-template-columns:repeat(auto-fill, minmax(220px, 1fr));margin-bottom:16px">
              {#each vm.channelOptions || [] as channel}
                <div class:wizard-template-selected={vm.channelType === channel.name} class="card wizard-template-card" on:click={() => invoke('selectChannel', channel.name)} style="cursor:pointer"><div class="flex items-center gap-2 mb-2"><span class="channel-icon">{channel.icon}</span><span class="font-bold" style="font-size:13px">{channel.display_name}</span></div><div class="text-xs text-dim" style="line-height:1.6">{channel.description}</div></div>
              {/each}
            </div>
            {#if selectedChannel}
              <div class="card" style="border-left:3px solid var(--accent)">
                <div class="card-header">Configure {selectedChannel.display_name}</div>
                <div class="text-xs text-dim mb-2">{selectedChannel.help}</div>
                <div class="form-group"><label>{selectedChannel.token_label}</label><div class="key-input-group"><input type="password" placeholder={selectedChannel.token_placeholder} value={vm.channelToken || ''} on:input={(event) => setField('channelToken', event.currentTarget.value)} on:keydown={(event) => { if (event.key === 'Enter') invoke('configureChannel'); }}><button class="btn btn-primary btn-sm" disabled={vm.configuringChannel || !(vm.channelToken || '').trim()} on:click={() => invoke('configureChannel')}>{vm.configuringChannel ? 'Saving...' : 'Save'}</button></div></div>
                <div class="text-xs text-dim">Or set {selectedChannel.token_env} in your environment</div>
                {#if vm.channelConfigured}<div class="mt-2"><div class="badge badge-success" style="padding:6px 12px">{selectedChannel.display_name} configured and activated.</div></div>{/if}
              </div>
            {:else}
              <div class="info-card"><p>You can skip this step. The built-in web chat is always available from the <strong>Agents</strong> page. Add channels any time from <strong>Settings &gt; Channels</strong>.</p></div>
            {/if}
          </div>
          <div class="wizard-nav"><button class="btn btn-ghost" on:click={() => invoke('prevStep')}>Back</button><button class="btn btn-primary" on:click={() => invoke('nextStep')}>{vm.channelConfigured ? 'Next' : 'Skip'}</button></div>
        </div>
      {:else}
        <div class="wizard-step">
          <div class="wizard-card" style="text-align:center;max-width:560px;margin:0 auto">
            <div style="font-size:56px;margin-bottom:12px;color:var(--success)">&#10003;</div>
            <h3 style="font-size:20px;font-weight:700;margin-bottom:8px;color:var(--accent)">You're All Set!</h3>
            <p style="font-size:13px;color:var(--text-dim);line-height:1.8;margin-bottom:24px">Infring is configured and ready to go. Here is a summary of what was set up:</p>
            <div class="card" style="text-align:left;margin-bottom:20px">
              <div class="detail-grid">
                <div class="detail-row"><span class="detail-label">LLM Provider</span><span class="detail-value">{vm.setupSummary.provider || (vm.hasConfiguredProvider ? 'Pre-configured' : 'Skipped')}</span></div>
                <div class="detail-row"><span class="detail-label">First Agent</span><span class="detail-value">{vm.setupSummary.agent || 'Skipped'}</span></div>
                <div class="detail-row"><span class="detail-label">Channel</span><span class="detail-value">{vm.setupSummary.channel || 'None (web chat available)'}</span></div>
              </div>
            </div>
            <div class="card" style="text-align:left;margin-bottom:20px"><div class="card-header">Next Steps</div><div style="margin-top:8px;font-size:12px;color:var(--text-dim);line-height:1.8"><div style="padding:4px 0">- {vm.createdAgent ? 'Open Agents to start talking to your agent' : 'Go to Agents to create your first agent'}</div><div style="padding:4px 0">- Browse Plugins to add capabilities</div><div style="padding:4px 0">- Check Settings for advanced configuration</div>{#if !vm.setupSummary.channel}<div style="padding:4px 0">- Visit Channels to connect messaging platforms</div>{/if}</div></div>
            <div class="flex gap-2" style="justify-content:center"><button class="btn btn-primary" on:click={() => invoke('finish')}>{vm.createdAgent ? 'Start Chatting' : 'Go to Dashboard'}</button><button class="btn btn-ghost" on:click={() => invoke('prevStep')}>Back</button></div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
{/if}
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
