        icon: 'DC',
        description: 'Connect your agent to a Discord server via bot token.',
        token_label: 'Bot Token',
        token_placeholder: 'MTIz...abc',
        token_env: 'DISCORD_BOT_TOKEN',
        help: 'Create a Discord application at discord.com/developers and add a bot.'
      },
      {
        name: 'slack',
        display_name: 'Slack',
        icon: 'SL',
        description: 'Connect your agent to a Slack workspace.',
        token_label: 'Bot Token',
        token_placeholder: 'xoxb-...',
        token_env: 'SLACK_BOT_TOKEN',
        help: 'Create a Slack app at api.slack.com/apps and install it to your workspace.'
      }
    ],
    channelToken: '',
    configuringChannel: false,
    channelConfigured: false,

    // Step 5: Summary
    setupSummary: {
      provider: '',
      agent: '',
      channel: ''
    },

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

