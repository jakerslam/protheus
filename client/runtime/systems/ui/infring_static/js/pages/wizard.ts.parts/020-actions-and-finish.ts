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
          await Alpine.store('app').refreshAgents();
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

