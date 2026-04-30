// Infring Setup Wizard — First-run guided setup (Provider + Agent + Channel)
'use strict';

/** Escape a string for use inside TOML triple-quoted strings ("""\n...\n"""). */
function wizardTomlMultilineEscape(s) {
  return s.replace(/\\/g, '\\\\').replace(/"""/g, '""\\"');
}

/** Escape a string for use inside a TOML basic (single-line) string ("..."). */
function wizardTomlBasicEscape(s) {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n').replace(/\r/g, '\\r').replace(/\t/g, '\\t');
}

function wizardPage() {
  return {
    step: 1,
    totalSteps: 6,
    loading: false,
    error: '',

    // Step 2: Provider setup
    providers: [],
    selectedProvider: '',
    apiKeyInput: '',
    testingProvider: false,
    testResult: null,
    savingKey: false,
    keySaved: false,

    // Step 3: Agent creation
    ...infringWizardStaticSetupState(),

    // ── Lifecycle ──

    async loadData() {
      this.loading = true;
      this.error = '';
      try {
        await this.loadProviders();
      } catch(e) {
        this.error = e.message || 'Could not load setup data.';
      }
      this.loading = false;
    },

    // ── Navigation ──

    nextStep() {
      if (this.step === 3 && !this.createdAgent) {
        // Skip "Try It" if no agent was created
        this.step = 5;
      } else if (this.step < this.totalSteps) {
        this.step++;
      }
    },

    prevStep() {
      if (this.step === 5 && !this.createdAgent) {
        // Skip back past "Try It" if no agent was created
        this.step = 3;
      } else if (this.step > 1) {
        this.step--;
      }
    },

    goToStep(n) {
      if (n >= 1 && n <= this.totalSteps) {
        if (n === 4 && !this.createdAgent) return; // Can't go to Try It without agent
        this.step = n;
      }
    },

    stepLabel(n) {
      var labels = ['Welcome', 'Provider', 'Agent', 'Try It', 'Channel', 'Done'];
      return labels[n - 1] || '';
    },

    get canGoNext() {
      if (this.step === 2) return this.keySaved || this.hasConfiguredProvider || this.claudeCodeDetected;
      if (this.step === 3) return this.agentName.trim().length > 0;
      return true;
    },

    claudeCodeDetected: false,

    get hasConfiguredProvider() {
      var self = this;
      return this.providers.some(function(p) {
        return p.auth_status === 'configured';
      });
    },

    // ── Step 2: Providers ──

    async loadProviders() {
      try {
        var data = await InfringAPI.get('/api/providers');
        this.providers = data.providers || [];
        // Pre-select first unconfigured provider, or first one
        var unconfigured = this.providers.filter(function(p) {
          return p.auth_status !== 'configured' && p.api_key_env;
        });
        if (unconfigured.length > 0) {
          this.selectedProvider = unconfigured[0].id;
        } else if (this.providers.length > 0) {
          this.selectedProvider = this.providers[0].id;
        }
      } catch(e) { this.providers = []; }
    },

    get selectedProviderObj() {
      var self = this;
      var match = this.providers.filter(function(p) { return p.id === self.selectedProvider; });
      return match.length > 0 ? match[0] : null;
    },

    get popularProviders() {
      var popular = ['frontier_provider', 'openai', 'google', 'gemini', 'groq', 'deepseek', 'openrouter', 'claude-code'];
      return this.providers.filter(function(p) {
        return popular.indexOf(p.id) >= 0;
      }).sort(function(a, b) {
        return popular.indexOf(a.id) - popular.indexOf(b.id);
      });
    },

    get otherProviders() {
      var popular = ['frontier_provider', 'openai', 'google', 'gemini', 'groq', 'deepseek', 'openrouter', 'claude-code'];
      return this.providers.filter(function(p) {
        return popular.indexOf(p.id) < 0;
      });
    },

    selectProvider(id) {
      this.selectedProvider = id;
      this.apiKeyInput = '';
      this.testResult = null;
      this.keySaved = false;
    },

    providerHelp: function(id) {
      var help = {
        frontier_provider: { url: 'https://console.frontier_provider.com/settings/keys', text: 'Get your key from the Frontier Provider Console' },
        openai: { url: 'https://platform.openai.com/api-keys', text: 'Get your key from the OpenAI Platform' },
        gemini: { url: 'https://aistudio.google.com/apikey', text: 'Get your key from Google AI Studio' },
        google: { url: 'https://aistudio.google.com/apikey', text: 'Get your key from Google AI Studio' },
        groq: { url: 'https://console.groq.com/keys', text: 'Get your key from the Groq Console (free tier available)' },
        deepseek: { url: 'https://platform.deepseek.com/api_keys', text: 'Get your key from the DeepSeek Platform (very affordable)' },
        openrouter: { url: 'https://openrouter.ai/keys', text: 'Get your key from OpenRouter (access 100+ models with one key)' },
        mistral: { url: 'https://console.mistral.ai/api-keys', text: 'Get your key from the Mistral Console' },
        together: { url: 'https://api.together.xyz/settings/api-keys', text: 'Get your key from Together AI' },
        fireworks: { url: 'https://fireworks.ai/account/api-keys', text: 'Get your key from Fireworks AI' },
        perplexity: { url: 'https://www.perplexity.ai/settings/api', text: 'Get your key from Perplexity Settings' },
        cohere: { url: 'https://dashboard.cohere.com/api-keys', text: 'Get your key from the Cohere Dashboard' },
        xai: { url: 'https://console.x.ai/', text: 'Get your key from the xAI Console' },
        'claude-code': { url: 'https://docs.frontier_provider.com/en/docs/claude-code', text: 'Install: npm install -g @frontier_provider-ai/claude-code && claude auth (no API key needed)' }
      };
      return help[id] || null;
    },

    providerIsConfigured(p) {
      return p && p.auth_status === 'configured';
    },

    async saveKey() {
      var provider = this.selectedProviderObj;
      if (!provider) return;
      var key = this.apiKeyInput.trim();
      if (!key) {
        InfringToast.error('Please enter an API key');
        return;
      }
      this.savingKey = true;
      try {
        await InfringAPI.post('/api/providers/' + encodeURIComponent(provider.id) + '/key', { key: key });
        this.apiKeyInput = '';
        this.keySaved = true;
        this.setupSummary.provider = provider.display_name;
        InfringToast.success('API key saved for ' + provider.display_name);
        await this.loadProviders();
        // Auto-test after saving
        await this.testKey();
      } catch(e) {
        InfringToast.error('Failed to save key: ' + e.message);
      }

      this.savingKey = false;
    },

    async testKey() {
      var provider = this.selectedProviderObj;
      if (!provider) return;
      this.testingProvider = true;
      this.testResult = null;
      try {
        var result = await InfringAPI.post('/api/providers/' + encodeURIComponent(provider.id) + '/test', {});
        this.testResult = result;
        if (result.status === 'ok') {
          InfringToast.success(provider.display_name + ' connected (' + (result.latency_ms || '?') + 'ms)');
        } else {
          InfringToast.error(provider.display_name + ': ' + (result.error || 'Connection failed'));
        }
      } catch(e) {
        this.testResult = { status: 'error', error: e.message };
        InfringToast.error('Test failed: ' + e.message);
      }
      this.testingProvider = false;
    },

    async detectClaudeCode() {
      this.testingProvider = true;
      this.testResult = null;
      try {
        var result = await InfringAPI.post('/api/providers/claude-code/test', {});
        this.testResult = result;
        if (result.status === 'ok') {
          this.claudeCodeDetected = true;
          this.keySaved = true;
          this.setupSummary.provider = 'Claude Code';
          InfringToast.success('Claude Code detected (' + (result.latency_ms || '?') + 'ms)');
        } else {
          this.testResult = { status: 'error', error: 'Claude Code CLI not detected' };
          InfringToast.error('Claude Code CLI not detected. Make sure you\'ve run: npm install -g @frontier_provider-ai/claude-code && claude auth');
        }
      } catch(e) {
        this.testResult = { status: 'error', error: e.message };
        InfringToast.error('Claude Code CLI not detected. Make sure you\'ve run: npm install -g @frontier_provider-ai/claude-code && claude auth');
      }
      this.testingProvider = false;
    },

    // ── Step 3: Agent creation ──

    selectTemplate(index) {
      this.selectedTemplate = index;
      var tpl = this.templates[index];
      if (tpl) {
        this.agentName = tpl.name.toLowerCase().replace(/\s+/g, '-');
      }
    },

    async createAgent() {
      var tpl = this.templates[this.selectedTemplate];
      if (!tpl) return;
      var name = this.agentName.trim();
      if (!name) {
        InfringToast.error('Please enter a name for your agent');
        return;
      }

      // Use the provider the user just configured, or the template default
      var provider = tpl.provider;
      var model = tpl.model;
      if (this.selectedProviderObj && this.providerIsConfigured(this.selectedProviderObj)) {
        provider = this.selectedProviderObj.id;
        // Use a sensible default model for the provider
        model = this.defaultModelForProvider(provider) || tpl.model;
      }

      var toml = '[agent]\n';
      toml += 'name = "' + wizardTomlBasicEscape(name) + '"\n';
      toml += 'description = "' + wizardTomlBasicEscape(tpl.description) + '"\n';
      toml += 'profile = "' + tpl.profile + '"\n\n';
      toml += '[model]\nprovider = "' + provider + '"\n';
      toml += 'model = "' + model + '"\n';
      toml += 'system_prompt = """\n' + wizardTomlMultilineEscape(tpl.system_prompt) + '\n"""\n';

      this.creatingAgent = true;
      try {
        var res = await InfringAPI.post('/api/agents', { manifest_toml: toml });
        if (res.agent_id) {
          this.createdAgent = { id: res.agent_id, name: res.name || name };
          this.setupSummary.agent = res.name || name;
          InfringToast.success('Agent "' + (res.name || name) + '" created');
          var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
            ? InfringSharedShellServices.appStore
            : null;
          var refreshAgents = bridge && typeof bridge.method === 'function'
            ? bridge.method('refreshAgents')
            : null;
          if (typeof refreshAgents === 'function') await refreshAgents();
        } else {
          InfringToast.error('Failed: ' + (res.error || 'Unknown error'));
        }
      } catch(e) {
        InfringToast.error('Failed to create agent: ' + e.message);
      }
      this.creatingAgent = false;
    },

    defaultModelForProvider(providerId) {
      var defaults = {
        frontier_provider: 'claude-sonnet-4-20250514',
        openai: 'gpt-4o',
        gemini: 'gemini-2.5-flash',
        groq: 'llama-3.3-70b-versatile',
        deepseek: 'deepseek-chat',
        openrouter: 'openrouter/google/gemini-2.5-flash',
        mistral: 'mistral-large-latest',
        together: 'meta-llama/Llama-3-70b-chat-hf',
        fireworks: 'accounts/fireworks/models/llama-v3p1-70b-instruct',
        perplexity: 'llama-3.1-sonar-large-128k-online',
        cohere: 'command-r-plus',
        xai: 'grok-2',
        'claude-code': 'claude-code/sonnet'
      };
      return defaults[providerId] || '';
    },

    // ── Step 5: Channel setup ──

    selectChannel(name) {
      if (this.channelType === name) {
        this.channelType = '';
        this.channelToken = '';
      } else {
        this.channelType = name;
        this.channelToken = '';
      }
    },

    get selectedChannelObj() {
      var self = this;
      var match = this.channelOptions.filter(function(ch) { return ch.name === self.channelType; });
      return match.length > 0 ? match[0] : null;
    },

    async configureChannel() {
      var ch = this.selectedChannelObj;
      if (!ch) return;
      var token = this.channelToken.trim();
      if (!token) {
        InfringToast.error('Please enter the ' + ch.token_label);
        return;
      }
      this.configuringChannel = true;
      try {
        var fields = {};
        fields[ch.token_env.toLowerCase()] = token;
        fields.token = token;
        await InfringAPI.post('/api/channels/' + ch.name + '/configure', { fields: fields });
        this.channelConfigured = true;
        this.setupSummary.channel = ch.display_name;
        InfringToast.success(ch.display_name + ' configured and activated.');
      } catch(e) {
        InfringToast.error('Failed: ' + (e.message || 'Unknown error'));
      }
      this.configuringChannel = false;
    },

    // ── Step 6: Finish ──

    finish() {
      localStorage.setItem('infring-onboarded', 'true');
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      if (bridge && typeof bridge.set === 'function') bridge.set('showOnboarding', false);
      // Navigate to agents with chat if an agent was created, otherwise overview
      if (this.createdAgent) {
        var agent = this.createdAgent;
        if (bridge && typeof bridge.set === 'function') {
          bridge.set('pendingAgent', { id: agent.id, name: agent.name, model_provider: '?', model_name: '?' });
        }
        window.location.hash = 'agents';
      } else {
        window.location.hash = 'overview';
      }
    },

    finishAndDismiss() {
      localStorage.setItem('infring-onboarded', 'true');
      var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
        ? InfringSharedShellServices.appStore
        : null;
      if (bridge && typeof bridge.set === 'function') bridge.set('showOnboarding', false);
      window.location.hash = 'overview';
    }
  };
}
