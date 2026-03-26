// Infring Chat Page — Agent chat with markdown + streaming
'use strict';

function chatPage() {
  var msgId = 0;
  return {
    currentAgent: null,
    messages: [],
    inputText: '',
    sending: false,
    messageQueue: [],    // Queue for messages sent while streaming
    promptQueueDragId: '',
    _promptQueueSeq: 0,
    thinkingMode: 'off', // 'off' | 'on' | 'stream'
    _wsAgent: null,
    showAttachMenu: false,
    showSlashMenu: false,
    slashFilter: '',
    slashIdx: 0,
    attachments: [],
    dragOver: false,
    contextPressure: 'low',
    contextWindow: 8192,
    contextApproxTokens: 0,
    terminalMode: false,
    terminalCwd: '/workspace',
    terminalShortcutHint: 'Ctrl+\\',
    terminalCursorFocused: false,
    terminalSelectionStart: 0,
    _contextTelemetryTimer: null,
    _lastContextRequestAt: 0,
    _contextWindowByModel: {},
    _contextModelsFetchedAt: 0,
    _typingTimeout: null,
    // Multi-session state
    sessions: [],
    sessionsOpen: false,
    searchOpen: false,
    searchQuery: '',
    // Voice recording state
    recording: false,
    _mediaRecorder: null,
    _audioChunks: [],
    recordingTime: 0,
    _recordingTimer: null,
    // Model autocomplete state
    showModelPicker: false,
    modelPickerList: [],
    modelPickerFilter: '',
    modelPickerIdx: 0,
    // Model switcher dropdown
    showModelSwitcher: false,
    modelSwitcherFilter: '',
    modelSwitcherProviderFilter: '',
    modelSwitcherIdx: 0,
    showGitTreeMenu: false,
    gitTreeMenuLoading: false,
    gitTreeMenuError: '',
    gitTreeMenuItems: [],
    gitTreeSwitching: false,
    modelApiKeyInput: '',
    modelApiKeySaving: false,
    modelApiKeyStatus: '',
    modelDownloadBusy: {},
    modelSwitching: false,
    _modelCache: null,
    _modelCacheTime: 0,
    _chatMapWheelLockInstalled: false,
    sessionLoading: false,
    _sessionLoadSeq: 0,
    messageHydration: {},
    _forcedHydrateById: {},
    _renderWindowRaf: 0,
    showFreshArchetypeTiles: false,
    freshInitTemplateDef: null,
    freshInitTemplateName: '',
    freshInitName: '',
    freshInitEmoji: '',
    freshInitLaunching: false,
    freshInitRevealMenu: false,
    freshInitStageToken: 0,
    conversationCache: {},
    conversationCacheKey: 'of-chat-conversation-cache-v1',
    conversationCacheVersionKey: 'of-chat-conversation-cache-version',
    conversationCacheVersion: 'v2-source-runs-20260325',
    _persistTimer: null,
    _responseStartedAt: 0,
    _pointerGridHideTimer: null,
    _pendingAutoModelSwitchBaseline: '',
    _pendingWsRequest: null,
    _pendingWsRecovering: false,
    _inflightPayload: null,
    _inflightFailoverInProgress: false,
    _sendWatchdogTimer: null,
    modelNoticeCache: {},
    modelNoticeCacheKey: 'of-chat-model-notices-v1',
    modelUsageCache: {},
    modelUsageCacheKey: 'of-chat-model-usage-v1',
    showScrollDown: false,
    hoveredMessageDomId: '',
    selectedMessageDomId: '',
    mapStepIndex: -1,
    activeMapPreviewDomId: '',
    activeMapPreviewDayKey: '',
    suppressMapPreview: false,
    _mapPreviewSuppressTimer: null,
    _scrollSyncFrame: 0,
    _lastInactiveNoticeKey: '',
    _agentMissingSince: 0,
    _agentMissingAgentId: '',
    _agentMissingGraceMs: 12000,
    collapsedMessageDays: {},
    showAgentDrawer: false,
    agentDrawerLoading: false,
    agentDrawer: null,
    drawerTab: 'info',
    drawerConfigForm: {},
    drawerConfigSaving: false,
    drawerModelSaving: false,
    drawerIdentitySaving: false,
    drawerSavePending: false,
    drawerEditingModel: false,
    drawerEditingProvider: false,
    drawerEditingFallback: false,
    drawerEditingName: false,
    drawerEditingEmoji: false,
    drawerEmojiPickerOpen: false,
    drawerEmojiSearch: '',
    drawerAvatarUploading: false,
    drawerAvatarUploadError: '',
    drawerNewModelValue: '',
    drawerNewProviderValue: '',
    drawerNewFallbackValue: '',
    drawerEmojiCatalog: [
      { emoji: '🤖', name: 'robot' },
      { emoji: '🧠', name: 'brain' },
      { emoji: '🧑\u200d💻', name: 'developer' },
      { emoji: '🛠️', name: 'tools' },
      { emoji: '🔬', name: 'research' },
      { emoji: '🧪', name: 'experiment' },
      { emoji: '🛰️', name: 'signal' },
      { emoji: '📡', name: 'telemetry' },
      { emoji: '🚀', name: 'launch' },
      { emoji: '🧭', name: 'navigator' },
      { emoji: '🦾', name: 'strong arm' },
      { emoji: '⚙️', name: 'gear' },
      { emoji: '🔐', name: 'security' },
      { emoji: '🛡️', name: 'shield' },
      { emoji: '📈', name: 'growth' },
      { emoji: '📊', name: 'analytics' },
      { emoji: '📝', name: 'writer' },
      { emoji: '🎯', name: 'target' },
      { emoji: '💡', name: 'idea' },
      { emoji: '🌐', name: 'network' },
      { emoji: '🧱', name: 'builder' },
      { emoji: '🧰', name: 'toolbox' },
      { emoji: '🦉', name: 'wise owl' },
      { emoji: '🔥', name: 'fire' }
    ],
    drawerArchetypeOptions: ['Assistant', 'Researcher', 'Coder', 'Writer', 'DevOps', 'Support', 'Analyst', 'Custom'],
    drawerVibeOptions: ['professional', 'friendly', 'technical', 'creative', 'concise', 'mentor'],
    chatArchetypeTemplates: [
      {
        name: 'General Assistant',
        category: 'General',
        description: 'Versatile helper for everyday tasks and recommendations.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        archetype: 'assistant',
        system_prompt: 'You are a helpful, friendly assistant. Provide clear, accurate, and concise responses. Ask clarifying questions when needed.'
      },
      {
        name: 'Code Helper',
        category: 'Development',
        description: 'Programming-focused agent for writing, reviewing, and debugging.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        archetype: 'coder',
        system_prompt: 'You are an expert programmer. Help users write clean, efficient code. Explain your reasoning and follow best practices.'
      },
      {
        name: 'Researcher',
        category: 'Research',
        description: 'Analytical agent for complex topics and synthesis.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'research',
        archetype: 'researcher',
        system_prompt: 'You are a research analyst. Break down complex topics with clear structure and concise findings.'
      },
      {
        name: 'Writer',
        category: 'Writing',
        description: 'Creative drafting, editing, and tone adaptation.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        archetype: 'writer',
        system_prompt: 'You are a skilled writer and editor. Help users create polished content and offer constructive improvements.'
      },
      {
        name: 'Data Analyst',
        category: 'Development',
        description: 'Data analysis, SQL/Python queries, and interpretation.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        archetype: 'analyst',
        system_prompt: 'You are a data analysis expert. Help with dataset analysis, SQL/Python queries, and actionable interpretation.'
      },
      {
        name: 'DevOps Engineer',
        category: 'Development',
        description: 'CI/CD, infra, containers, and deployment reliability.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'automation',
        archetype: 'devops',
        system_prompt: 'You are a DevOps engineer. Help with CI/CD pipelines, Docker, infrastructure, deployment, and reliability.'
      },
      {
        name: 'Customer Support',
        category: 'Business',
        description: 'Empathetic and professional issue resolution.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'messaging',
        archetype: 'support',
        system_prompt: 'You are a professional support agent. Be empathetic, concise, and solution-oriented.'
      },
      {
        name: 'Tutor',
        category: 'General',
        description: 'Step-by-step explanations adapted to learner level.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        archetype: 'assistant',
        system_prompt: 'You are a patient tutor. Explain step-by-step, check understanding, and adapt to learner pace.'
      },
      {
        name: 'API Designer',
        category: 'Development',
        description: 'REST design, schema consistency, and versioning.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        archetype: 'coder',
        system_prompt: 'You are an API design expert. Design clean RESTful APIs with robust schemas, error handling, and versioning.'
      },
      {
        name: 'Meeting Notes',
        category: 'Business',
        description: 'Summaries with decisions and action items.',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'minimal',
        archetype: 'analyst',
        system_prompt: 'You summarize meetings into key decisions, action items, highlights, and follow-up questions.'
      }
    ],
    slashCommands: [
      { cmd: '/help', desc: 'Show available commands' },
      { cmd: '/agents', desc: 'Switch to Agents page' },
      { cmd: '/new', desc: 'Reset session (clear history)' },
      { cmd: '/compact', desc: 'Trigger LLM session compaction' },
      { cmd: '/model', desc: 'Show or switch model (/model [name])' },
      { cmd: '/file', desc: 'Render full file output in chat (/file [path])' },
      { cmd: '/folder', desc: 'Render folder tree + downloadable archive (/folder [path])' },
      { cmd: '/stop', desc: 'Cancel current agent run' },
      { cmd: '/usage', desc: 'Show session token usage & cost' },
      { cmd: '/think', desc: 'Toggle extended thinking (/think [on|off|stream])' },
      { cmd: '/context', desc: 'Show context window usage & pressure' },
      { cmd: '/verbose', desc: 'Cycle tool detail level (/verbose [off|on|full])' },
      { cmd: '/queue', desc: 'Check if agent is processing' },
      { cmd: '/status', desc: 'Show system status' },
      { cmd: '/clear', desc: 'Clear chat display' },
      { cmd: '/exit', desc: 'Disconnect from agent' },
      { cmd: '/budget', desc: 'Show spending limits and current costs' },
      { cmd: '/peers', desc: 'Show OFP peer network status' },
      { cmd: '/a2a', desc: 'List discovered external A2A agents' }
    ],
    tokenCount: 0,
    promptSuggestions: [],
    suggestionsLoading: false,
    _suggestionFetchSeq: 0,
    _lastSuggestionsAt: 0,
    _lastSuggestionsAgentId: '',
    _pointerTrailLastAt: 0,
    _pointerTrailRaf: 0,
    _pointerTrailLastX: 0,
    _pointerTrailLastY: 0,
    _pointerTrailSeeded: false,
    _pointerTrailHeadLastAt: 0,
    _progressCache: {},
    _freshInitThreadShownFor: '',

    // ── Tip Bar ──
    tipIndex: 0,
    tips: ['Type / for commands', '/think on for reasoning', 'Ctrl+Shift+F for focus mode', 'Ctrl+T or Ctrl+\\ for terminal mode', 'Ctrl+F to add files', '/model to switch models', '/context to check usage', '/verbose off to hide tool details'],
    tipTimer: null,
    get currentTip() {
      if (localStorage.getItem('of-tips-off') === 'true') return '';
      return this.tips[this.tipIndex % this.tips.length];
    },
    dismissTips: function() { localStorage.setItem('of-tips-off', 'true'); },
    startTipCycle: function() {
      var self = this;
      if (this.tipTimer) clearInterval(this.tipTimer);
      this.tipTimer = setInterval(function() {
        self.tipIndex = (self.tipIndex + 1) % self.tips.length;
      }, 30000);
    },

    // Backward compat helper
    get thinkingEnabled() { return this.thinkingMode !== 'off'; },

    get terminalPromptPath() {
      return this.terminalCwd || '/workspace';
    },

    get terminalPromptPrefix() {
      return this.terminalPromptPath + ' % ';
    },

    get terminalPromptChars() {
      var len = this.terminalPromptPrefix.length;
      if (!Number.isFinite(len)) return 18;
      if (len < 18) return 18;
      return len;
    },

    get terminalCursorIndex() {
      var text = String(this.inputText || '');
      var max = text.length;
      var raw = Number(this.terminalSelectionStart);
      if (!Number.isFinite(raw)) return max;
      if (raw < 0) return 0;
      if (raw > max) return max;
      return Math.floor(raw);
    },

    get terminalCursorRow() {
      var text = String(this.inputText || '');
      if (!text) return 0;
      var upto = text.slice(0, this.terminalCursorIndex);
      var parts = upto.split('\n');
      return Math.max(0, parts.length - 1);
    },

    get terminalCursorColumn() {
      var text = String(this.inputText || '');
      if (!text) return 0;
      var upto = text.slice(0, this.terminalCursorIndex);
      var parts = upto.split('\n');
      return (parts[parts.length - 1] || '').length;
    },

    get terminalCursorStyle() {
      return '--terminal-cursor-ch:' + (this.terminalPromptChars + this.terminalCursorColumn) +
        '; --terminal-cursor-row:' + this.terminalCursorRow + ';';
    },

    formatTokenK(value) {
      var raw = Number(value || 0);
      if (!Number.isFinite(raw) || raw <= 0) return '0k';
      var k = raw / 1000;
      if (k >= 100) return Math.round(k) + 'k';
      if (k >= 10) return (Math.round(k * 10) / 10).toFixed(1).replace(/\.0$/, '') + 'k';
      return (Math.round(k * 100) / 100).toFixed(2).replace(/0$/, '').replace(/\.$/, '') + 'k';
    },

    get contextUsagePercent() {
      var windowSize = Number(this.contextWindow || 0);
      var used = Number(this.contextApproxTokens || 0);
      if (windowSize > 0 && used >= 0) {
        var ratio = Math.round((used / windowSize) * 100);
        if (ratio < 0) return 0;
        if (ratio > 100) return 100;
        return ratio;
      }
      switch (this.contextPressure) {
        case 'critical': return 95;
        case 'high': return 80;
        case 'medium': return 55;
        default: return 25;
      }
    },

    get contextRingArcLength() {
      // 330deg sweep: starts at 1 o'clock and ends at 12 o'clock at 100%.
      var maxArc = 91.6667;
      var usage = this.contextUsagePercent;
      if (!Number.isFinite(usage) || usage <= 0) return 0;
      if (usage >= 100) return maxArc;
      return Number(((usage / 100) * maxArc).toFixed(3));
    },

    get contextRingProgressStyle() {
      return 'stroke-dasharray: ' + this.contextRingArcLength + ' 100; stroke-dashoffset: 0;';
    },

    get contextRingTooltip() {
      return 'Context window\n' +
        this.contextUsagePercent + '% full\n' +
        ' ' + this.formatTokenK(this.contextApproxTokens) + ' / ' + this.formatTokenK(this.contextWindow) + ' tokens used\n\n' +
        ' Infring dynamically prunes its context';
    },

    get activeGitBranchLabel() {
      var agentBranch = this.currentAgent && this.currentAgent.git_branch
        ? String(this.currentAgent.git_branch).trim()
        : '';
      if (agentBranch) return agentBranch;
      try {
        var store = Alpine.store('app');
        var branch = store && store.gitBranch ? String(store.gitBranch).trim() : '';
        return branch || '';
      } catch(_) {
        return '';
      }
    },

    normalizeBranchName: function(value) {
      var raw = String(value == null ? '' : value).trim();
      if (!raw) return '';
      var normalized = raw
        .replace(/[^A-Za-z0-9._/-]+/g, '-')
        .replace(/\/+/g, '/')
        .replace(/^[-./]+|[-./]+$/g, '');
      return normalized;
    },

    closeGitTreeMenu: function() {
      this.showGitTreeMenu = false;
      this.gitTreeMenuError = '';
    },

    async refreshGitTreeMenu(force) {
      if (!this.currentAgent || !this.currentAgent.id) {
        this.gitTreeMenuItems = [];
        this.gitTreeMenuError = '';
        return;
      }
      if (!force && this.gitTreeMenuLoading) return;
      this.gitTreeMenuLoading = true;
      this.gitTreeMenuError = '';
      try {
        var payload = await InfringAPI.get('/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/git-trees');
        var options = Array.isArray(payload && payload.options) ? payload.options : [];
        this.gitTreeMenuItems = options.map(function(row) {
          return {
            branch: String((row && row.branch) || '').trim(),
            current: !!(row && row.current),
            main: !!(row && row.main),
            kind: String((row && row.kind) || '').trim(),
            in_use_by_agents: Number((row && row.in_use_by_agents) || 0) || 0
          };
        }).filter(function(row) { return !!row.branch; });
        this.applyAgentGitTreeState(this.currentAgent, payload && payload.current ? payload.current : {});
      } catch (e) {
        this.gitTreeMenuItems = [];
        this.gitTreeMenuError = (e && e.message) ? String(e.message) : 'failed_to_load_git_trees';
      } finally {
        this.gitTreeMenuLoading = false;
      }
    },

    async toggleGitTreeMenu() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.showGitTreeMenu) {
        this.closeGitTreeMenu();
        return;
      }
      this.showGitTreeMenu = true;
      await this.refreshGitTreeMenu(true);
    },

    async switchAgentGitTree(branchName, options) {
      if (!this.currentAgent || !this.currentAgent.id || this.gitTreeSwitching) return;
      var branch = this.normalizeBranchName(branchName);
      if (!branch) return;
      var requireNew = !!(options && options.requireNew === true);
      var current = this.normalizeBranchName(this.activeGitBranchLabel);
      if (!requireNew && current && current === branch) {
        this.closeGitTreeMenu();
        return;
      }
      this.gitTreeSwitching = true;
      this.gitTreeMenuError = '';
      try {
        var result = await InfringAPI.post(
          '/api/agents/' + encodeURIComponent(this.currentAgent.id) + '/git-tree/switch',
          {
            branch: branch,
            require_new: requireNew
          }
        );
        this.applyAgentGitTreeState(this.currentAgent, result && result.current ? result.current : {});
        if (result && result.current && result.current.workspace_dir) {
          this.terminalCwd = String(result.current.workspace_dir || this.terminalCwd || '').trim() || this.terminalCwd;
        }
        var store = Alpine.store('app');
        if (store && typeof store.refreshAgents === 'function') {
          await store.refreshAgents({ force: true });
        }
        await this.refreshGitTreeMenu(true);
        this.closeGitTreeMenu();
        InfringToast.success('Switched to branch ' + branch);
      } catch (e) {
        var message = (e && e.message) ? String(e.message) : 'git_tree_switch_failed';
        this.gitTreeMenuError = message;
        InfringToast.error('Git tree switch failed: ' + message);
      } finally {
        this.gitTreeSwitching = false;
      }
    },

    async createAndCheckoutGitBranch() {
      if (!this.currentAgent || !this.currentAgent.id || this.gitTreeSwitching) return;
      var suggested = this.normalizeBranchName('feature/' + String(this.currentAgent.id || '').trim().toLowerCase());
      var input = prompt('Create and checkout new branch:', suggested || 'feature/new-branch');
      if (input == null) return;
      var branch = this.normalizeBranchName(input);
      if (!branch) {
        InfringToast.error('Enter a valid branch name');
        return;
      }
      await this.switchAgentGitTree(branch, { requireNew: true });
    },

    get freshInitCanLaunch() {
      var hasName = String(this.freshInitName || '').trim().length > 0;
      return !!(this.showFreshArchetypeTiles && !this.freshInitLaunching && hasName && this.freshInitTemplateDef);
    },

    get modelDisplayName() {
      if (!this.currentAgent) return '';
      var selected = String(this.currentAgent.model_name || '').trim();
      var runtime = String(this.currentAgent.runtime_model || '').trim();
      if (selected.toLowerCase() === 'auto') {
        var resolved = runtime ? runtime.replace(/-\d{8}$/, '') : '';
        var autoLabel = resolved ? ('Auto: ' + resolved) : 'Auto';
        return autoLabel.length > 24 ? autoLabel.substring(0, 22) + '\u2026' : autoLabel;
      }
      var short = selected.replace(/-\d{8}$/, '');
      return short.length > 24 ? short.substring(0, 22) + '\u2026' : short;
    },

    get switcherProviders() {
      var seen = {};
      (this._modelCache || []).forEach(function(m) { seen[m.provider] = true; });
      return Object.keys(seen).sort();
    },

    get filteredSwitcherModels() {
      var models = this._modelCache || [];
      var provFilter = this.modelSwitcherProviderFilter;
      var textFilter = this.modelSwitcherFilter ? this.modelSwitcherFilter.toLowerCase() : '';
      var filtered = models.filter(function(m) {
        if (provFilter && m.provider !== provFilter) return false;
        if (textFilter) {
          return m.id.toLowerCase().indexOf(textFilter) !== -1 ||
                 (m.display_name || '').toLowerCase().indexOf(textFilter) !== -1 ||
                 m.provider.toLowerCase().indexOf(textFilter) !== -1;
        }
        return true;
      });
      var self = this;
      filtered.sort(function(a, b) {
        var aId = String((a && a.id) || '').trim();
        var bId = String((b && b.id) || '').trim();
        var aUsage = self.modelUsageTs(aId);
        var bUsage = self.modelUsageTs(bId);
        if (bUsage !== aUsage) return bUsage - aUsage;
        var activeIds = self.activeModelCandidateIds();
        var aActive = aId && activeIds.indexOf(aId) >= 0 ? 1 : 0;
        var bActive = bId && activeIds.indexOf(bId) >= 0 ? 1 : 0;
        if (bActive !== aActive) return bActive - aActive;
        var aProvider = String((a && a.provider) || '').toLowerCase();
        var bProvider = String((b && b.provider) || '').toLowerCase();
        if (aProvider !== bProvider) return aProvider.localeCompare(bProvider);
        return aId.toLowerCase().localeCompare(bId.toLowerCase());
      });
      return filtered;
    },

    activeModelCandidateIds: function() {
      var out = [];
      var seen = {};
      var add = function(value) {
        var id = String(value || '').trim();
        if (!id || seen[id]) return;
        seen[id] = true;
        out.push(id);
      };
      var agent = this.currentAgent || {};
      var selected = String(agent.model_name || '').trim();
      var runtime = String(agent.runtime_model || '').trim();
      var provider = String(agent.model_provider || '').trim().toLowerCase();
      if (selected) add(selected);
      if (runtime) add(runtime);
      if (selected && provider && provider !== 'ollama' && selected.indexOf('/') < 0) add(provider + '/' + selected);
      if (runtime && provider && provider !== 'ollama' && runtime.indexOf('/') < 0) add(provider + '/' + runtime);
      return out;
    },

    isSwitcherModelActive: function(model) {
      var id = String(model && model.id ? model.id : '').trim();
      if (!id) return false;
      return this.activeModelCandidateIds().indexOf(id) >= 0;
    },

    resolveActiveSwitcherModel: function(filtered) {
      var rows = Array.isArray(filtered) ? filtered : [];
      var activeIds = this.activeModelCandidateIds();
      for (var i = 0; i < activeIds.length; i++) {
        var id = activeIds[i];
        for (var j = 0; j < rows.length; j++) {
          var row = rows[j];
          if (row && String(row.id || '').trim() === id) return row;
        }
      }
      var agent = this.currentAgent || null;
      if (!agent) return null;
      var selected = String(agent.model_name || '').trim();
      var runtime = String(agent.runtime_model || '').trim();
      var provider = String(agent.model_provider || '').trim();
      var activeId = selected.toLowerCase() === 'auto' && runtime ? runtime : (selected || runtime);
      if (!activeId) return null;
      return {
        id: activeId,
        provider: provider || (activeId.indexOf('/') >= 0 ? activeId.split('/')[0] : 'unknown'),
        display_name: activeId.indexOf('/') >= 0 ? activeId.split('/').slice(-1)[0] : activeId,
        tier: 'Active',
        context_window: Number(agent.context_window || 0) || null,
        is_local: provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp',
        deployment: (provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp') ? 'local' : (provider.toLowerCase() === 'cloud' ? 'cloud' : 'api'),
        power_rating: 3,
        cost_rating: provider.toLowerCase() === 'ollama' || provider.toLowerCase() === 'llama.cpp' ? 1 : 3,
        local_download_path: '',
        download_available: false,
      };
    },

    get groupedSwitcherModels() {
      var filtered = this.filteredSwitcherModels;
      var groups = [];
      var active = this.resolveActiveSwitcherModel(filtered);
      if (active) groups.push({ provider: 'Active', models: [active] });
      var activeId = active ? String(active.id || '').trim() : '';
      var recent = filtered.filter(function(m) {
        var id = String((m && m.id) || '').trim();
        return !activeId || id !== activeId;
      });
      if (recent.length) {
        groups.push({ provider: 'Recent', models: recent });
      } else if (!groups.length && filtered.length) {
        groups.push({ provider: 'Recent', models: filtered });
      }
      return groups;
    },

    modelSwitcherItemName: function(m) {
      var model = m || {};
      var provider = String(model.provider || '').trim();
      var id = String(model.id || '').trim();
      var display = String(model.display_name || id).trim();
      var isAutoRow = provider.toLowerCase() === 'auto' || id.toLowerCase() === 'auto';
      if (!isAutoRow) return provider && provider.toLowerCase() !== 'unknown' ? (provider + ':' + display) : display;
      var activeAuto = this.currentAgent && String(this.currentAgent.model_name || '').trim().toLowerCase() === 'auto';
      var runtime = activeAuto ? String(this.currentAgent.runtime_model || '').trim() : '';
      if (!runtime) return 'Auto';
      var short = runtime.replace(/-\d{8}$/, '');
      return short ? ('Auto: ' + short) : 'Auto';
    },

    modelDeploymentKind: function(model) {
      var row = model || {};
      var deployment = String(row.deployment || '').trim().toLowerCase();
      if (deployment === 'local' || deployment === 'cloud' || deployment === 'api') return deployment;
      if (row.is_local === true) return 'local';
      var provider = String(row.provider || '').trim().toLowerCase();
      if (provider === 'ollama' || provider === 'llama.cpp') return 'local';
      if (provider === 'cloud') return 'cloud';
      return 'api';
    },

    modelDeploymentLabel: function(model) {
      var kind = this.modelDeploymentKind(model);
      if (kind === 'local') return 'Local model';
      if (kind === 'api') return 'API model';
      return 'Cloud model';
    },

    modelPowerIcons: function(model) {
      var level = Number(model && model.power_rating != null ? model.power_rating : 3);
      if (!Number.isFinite(level)) level = 3;
      level = Math.max(1, Math.min(5, Math.round(level)));
      return '🔥'.repeat(level);
    },

    modelCostIcons: function(model) {
      var level = Number(model && model.cost_rating != null ? model.cost_rating : 3);
      if (!Number.isFinite(level)) level = 3;
      level = Math.max(1, Math.min(5, Math.round(level)));
      return '$'.repeat(level);
    },

    modelDownloadKey: function(model) {
      var row = model || {};
      var provider = String(row.provider || '').trim().toLowerCase();
      var id = String(row.id || row.display_name || '').trim().toLowerCase();
      return provider + '::' + id;
    },

    isModelDownloadable: function(model) {
      var row = model || {};
      return !!(row && (row.download_available === true || String(row.local_download_path || '').trim()));
    },

    isModelDownloadBusy: function(model) {
      var key = this.modelDownloadKey(model);
      return !!(key && this.modelDownloadBusy && this.modelDownloadBusy[key] === true);
    },

    downloadModelToLocal: function(model) {
      var self = this;
      var row = model || {};
      if (!self.isModelDownloadable(row)) {
        InfringToast.error('No local download path is available for this model');
        return;
      }
      var key = self.modelDownloadKey(row);
      if (!key) return;
      if (!self.modelDownloadBusy) self.modelDownloadBusy = {};
      if (self.modelDownloadBusy[key]) return;
      self.modelDownloadBusy[key] = true;
      var modelRef = String(row.id || row.display_name || '').trim();
      var provider = String(row.provider || '').trim();
      InfringAPI.post('/api/models/download', {
        model: modelRef,
        provider: provider
      }).then(function(resp) {
        var method = String((resp && resp.method) || '').trim();
        var localPath = String((resp && resp.download_path) || '').trim();
        if (method === 'ollama_pull') {
          InfringToast.success('Model downloaded locally: ' + localPath);
        } else {
          InfringToast.success('Local download path prepared: ' + localPath);
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return InfringAPI.get('/api/models');
      }).then(function(data) {
        var models = (data && data.models) || [];
        self._modelCache = models.filter(function(m) { return m.available; });
        self._modelCacheTime = Date.now();
        self.modelPickerList = self._modelCache;
      }).catch(function(e) {
        InfringToast.error('Model download failed: ' + (e && e.message ? e.message : e));
      }).finally(function() {
        self.modelDownloadBusy[key] = false;
      });
    },

    pickDefaultAgent(agents) {
      if (!Array.isArray(agents) || !agents.length) return null;
      // Prefer the master/default agent when present; otherwise first running agent.
      var i;
      for (i = 0; i < agents.length; i++) {
        var a = agents[i] || {};
        var text = ((a.id || '') + ' ' + (a.name || '') + ' ' + (a.role || '')).toLowerCase();
        if (text.indexOf('master') >= 0 || text.indexOf('default') >= 0 || text.indexOf('primary') >= 0) {
          return a;
        }
      }
      for (i = 0; i < agents.length; i++) {
        var b = agents[i] || {};
        if (String(b.state || '').toLowerCase() === 'running') return b;
      }
      return agents[0];
    },

    resolveAgent(agentOrId) {
      if (!agentOrId) return null;
      var id = typeof agentOrId === 'string' ? agentOrId : agentOrId.id;
      if (!id) return null;
      var list = (Alpine.store('app') && Alpine.store('app').agents) || [];
      for (var i = 0; i < list.length; i++) {
        if (list[i] && String(list[i].id) === String(id)) return list[i];
      }
      if (typeof agentOrId === 'object' && agentOrId.id) return agentOrId;
      return null;
    },

    applyAgentGitTreeState(targetAgent, sourceState) {
      var target = targetAgent && typeof targetAgent === 'object' ? targetAgent : null;
      var source = sourceState && typeof sourceState === 'object' ? sourceState : null;
      if (!target || !source) return target;
      if (Object.prototype.hasOwnProperty.call(source, 'git_branch')) {
        var branch = source.git_branch ? String(source.git_branch).trim() : '';
        if (branch) target.git_branch = branch;
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_kind')) {
        target.git_tree_kind = source.git_tree_kind ? String(source.git_tree_kind).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'workspace_dir')) {
        var workspace = source.workspace_dir ? String(source.workspace_dir).trim() : '';
        if (workspace) {
          target.workspace_dir = workspace;
          this.terminalCwd = workspace;
        }
      }
      if (Object.prototype.hasOwnProperty.call(source, 'workspace_rel')) {
        target.workspace_rel = source.workspace_rel ? String(source.workspace_rel).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_ready')) {
        target.git_tree_ready = !!source.git_tree_ready;
      }
      if (Object.prototype.hasOwnProperty.call(source, 'git_tree_error')) {
        target.git_tree_error = source.git_tree_error ? String(source.git_tree_error).trim() : '';
      }
      if (Object.prototype.hasOwnProperty.call(source, 'is_master_agent')) {
        target.is_master_agent = !!source.is_master_agent;
      }
      return target;
    },

    syncCurrentAgentFromStore: function(sourceAgent) {
      var source = sourceAgent && typeof sourceAgent === 'object' ? sourceAgent : null;
      if (!source || !this.currentAgent || !this.currentAgent.id) return false;
      if (String(this.currentAgent.id) !== String(source.id)) return false;
      this.applyAgentGitTreeState(this.currentAgent, source);
      var keys = Object.keys(source);
      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];
        if (key === 'id') continue;
        this.currentAgent[key] = source[key];
      }
      return true;
    },

    setStoreActiveAgentId: function(agentId) {
      var store = Alpine.store('app');
      if (!store) return;
      if (typeof store.setActiveAgentId === 'function') {
        store.setActiveAgentId(agentId || null);
        return;
      }
      store.activeAgentId = agentId || null;
      try {
        if (store.activeAgentId) localStorage.setItem('infring-last-active-agent-id', String(store.activeAgentId));
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch {}
    },

    cacheAgentConversation(agentId) {
      if (!agentId) return;
      if (!this.conversationCache) this.conversationCache = {};
      try {
        var cachedMessages = this.sanitizeConversationForCache(this.messages || []);
        this.conversationCache[String(agentId)] = {
          saved_at: Date.now(),
          token_count: this.tokenCount || 0,
          messages: cachedMessages,
        };
        var appStore = Alpine.store('app');
        if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
          appStore.saveAgentChatPreview(agentId, this.conversationCache[String(agentId)].messages);
        }
        this.persistConversationCache();
      } catch {}
    },

    cacheCurrentConversation() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      this.cacheAgentConversation(this.currentAgent.id);
    },

    scheduleConversationPersist() {
      var self = this;
      if (this._persistTimer) clearTimeout(this._persistTimer);
      this._persistTimer = setTimeout(function() {
        self.cacheCurrentConversation();
      }, 80);
    },

    sanitizeConversationForCache(messages) {
      var source = Array.isArray(messages) ? messages : [];
      var out = [];
      for (var i = 0; i < source.length; i++) {
        var msg = source[i];
        if (!msg || typeof msg !== 'object') continue;
        if (msg.thinking || msg.streaming || (msg.terminal && msg.thinking)) continue;
        var cloned = null;
        try {
          cloned = JSON.parse(JSON.stringify(msg));
        } catch(_) {
          cloned = null;
        }
        if (!cloned || typeof cloned !== 'object') continue;
        delete cloned.thinking;
        delete cloned.streaming;
        delete cloned.thoughtStreaming;
        delete cloned._streamRawText;
        delete cloned._cleanText;
        delete cloned._thoughtText;
        delete cloned._toolTextDetected;
        delete cloned._reasoning;
        if (Array.isArray(cloned.tools)) {
          for (var ti = 0; ti < cloned.tools.length; ti++) {
            if (cloned.tools[ti] && typeof cloned.tools[ti] === 'object') {
              cloned.tools[ti].running = false;
            }
          }
        }
        out.push(cloned);
      }
      return out;
    },

    restoreAgentConversation(agentId) {
      if (!agentId || !this.conversationCache) return false;
      const cached = this.conversationCache[String(agentId)];
      if (!cached || !Array.isArray(cached.messages)) return false;
      try {
        var sanitized = this.sanitizeConversationForCache(cached.messages || []);
        this.messages = this.mergeModelNoticesForAgent(agentId, sanitized);
        this.tokenCount = Number(cached.token_count || 0);
        this.sending = false;
        this._responseStartedAt = 0;
        this._clearPendingWsRequest();
        if (sanitized.length !== cached.messages.length) {
          this.conversationCache[String(agentId)].messages = sanitized;
          this.persistConversationCache();
        }
        this.recomputeContextEstimate();
        this.$nextTick(() => this.scrollToBottomImmediate());
        return true;
      } catch {
        return false;
      }
    },

    loadConversationCache() {
      try {
        var cacheVersion = localStorage.getItem(this.conversationCacheVersionKey);
        if (cacheVersion !== this.conversationCacheVersion) {
          localStorage.removeItem(this.conversationCacheKey);
          localStorage.setItem(this.conversationCacheVersionKey, this.conversationCacheVersion);
          return {};
        }
        var raw = localStorage.getItem(this.conversationCacheKey);
        if (!raw) return {};
        var parsed = JSON.parse(raw);
        if (!parsed || typeof parsed !== 'object') return {};
        return parsed;
      } catch {
        return {};
      }
    },

    persistConversationCache() {
      try {
        localStorage.setItem(this.conversationCacheVersionKey, this.conversationCacheVersion);
        localStorage.setItem(this.conversationCacheKey, JSON.stringify(this.conversationCache || {}));
      } catch {}
    },

    estimateTokensFromText(text) {
      return Math.max(0, Math.round(String(text || '').length / 4));
    },

    recomputeContextEstimate() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var total = 0;
      for (var i = 0; i < rows.length; i++) {
        total += this.estimateTokensFromText(rows[i] && rows[i].text ? rows[i].text : '');
      }
      this.contextApproxTokens = total;
      this.refreshContextPressure();
    },

    applyContextTelemetry(data) {
      if (!data || typeof data !== 'object') return;
      var approx = Number(data.context_tokens || data.context_used_tokens || data.context_total_tokens || 0);
      if (Number.isFinite(approx) && approx > 0) {
        this.contextApproxTokens = approx;
      } else if (typeof data.message === 'string') {
        var tokenMatch = data.message.match(/~?\s*([0-9,]+)\s+tokens/i);
        if (tokenMatch && tokenMatch[1]) {
          var parsed = Number(String(tokenMatch[1]).replace(/,/g, ''));
          if (Number.isFinite(parsed) && parsed > 0) this.contextApproxTokens = parsed;
        }
      }
      var windowSize = Number(data.context_window || data.context_window_tokens || 0);
      if (Number.isFinite(windowSize) && windowSize > 0) {
        this.contextWindow = windowSize;
      }
      var ratio = Number(data.context_ratio || 0);
      if ((!Number.isFinite(approx) || approx <= 0) && Number.isFinite(ratio) && ratio > 0 && this.contextWindow > 0) {
        this.contextApproxTokens = Math.round(this.contextWindow * ratio);
      }
      if (data.context_pressure) {
        this.contextPressure = data.context_pressure;
      } else {
        this.refreshContextPressure();
      }
    },

    isAutoModelSelected() {
      return !!(
        this.currentAgent &&
        String(this.currentAgent.model_name || '').trim().toLowerCase() === 'auto'
      );
    },

    formatAutoRouteMeta(route) {
      if (!route || typeof route !== 'object') return '';
      var provider = String(route.provider || route.selected_provider || '').trim();
      var model = String(route.model || route.selected_model || route.selected_model_id || '').trim();
      if (!model) return '';
      var shortModel = model;
      if (shortModel.indexOf('/') >= 0) {
        shortModel = shortModel.split('/').slice(-1)[0];
      }
      var reason = String(route.reason || '').trim();
      if (reason.length > 80) reason = reason.slice(0, 77) + '...';
      var prefix = provider ? ('Auto -> ' + provider + '/' + shortModel) : ('Auto -> ' + shortModel);
      return reason ? (prefix + ' (' + reason + ')') : prefix;
    },

    normalizeAutoModelNoticeName(modelId) {
      var value = String(modelId || '').trim();
      if (!value) return '';
      if (value.indexOf('/') >= 0) {
        value = value.split('/').slice(-1)[0];
      }
      return value.replace(/-\d{8}$/, '');
    },

    formatAutoModelSwitchLabel(modelId) {
      var normalized = this.normalizeAutoModelNoticeName(modelId);
      if (!normalized && this.currentAgent) {
        normalized = this.normalizeAutoModelNoticeName(
          this.currentAgent.runtime_model || this.currentAgent.model_name || ''
        );
      }
      return 'Auto:[' + (normalized || 'unknown') + ']';
    },

    captureAutoModelSwitchBaseline() {
      if (!this.currentAgent || !this.isAutoModelSelected()) return '';
      var current = String(this.currentAgent.runtime_model || this.currentAgent.model_name || '').trim();
      return this.formatAutoModelSwitchLabel(current);
    },

    maybeAddAutoModelSwitchNotice(previousLabel, route) {
      if (!this.currentAgent || !this.isAutoModelSelected()) return;
      var previous = String(previousLabel || '').trim();
      if (!previous) {
        previous = this.formatAutoModelSwitchLabel(this.currentAgent.runtime_model || this.currentAgent.model_name || '');
      }
      var nextModel = '';
      if (route && typeof route === 'object') {
        nextModel = String(route.model || route.selected_model || route.selected_model_id || '').trim();
      }
      if (!nextModel) {
        nextModel = String(this.currentAgent.runtime_model || this.currentAgent.model_name || '').trim();
      }
      var next = this.formatAutoModelSwitchLabel(nextModel);
      if (!next || previous === next) return;
      this.addNoticeEvent({
        notice_label: 'Model switched from ' + previous + ' to ' + next,
        notice_type: 'model',
        ts: Date.now()
      });
    },

    applyAutoRouteTelemetry(data) {
      if (!data || typeof data !== 'object') return null;
      var route = null;
      if (data.auto_route && typeof data.auto_route === 'object') {
        route = data.auto_route;
      } else if (data.route && typeof data.route === 'object') {
        route = data.route;
      }
      if (!route) return null;
      if (!this.currentAgent) return route;
      if (!this.isAutoModelSelected()) return route;
      var provider = String(route.provider || route.selected_provider || this.currentAgent.model_provider || '').trim();
      var model = String(route.model || route.selected_model || route.selected_model_id || '').trim();
      if (provider) this.currentAgent.model_provider = provider;
      if (model) {
        this.currentAgent.runtime_model = model.indexOf('/') >= 0 ? model.split('/').slice(-1)[0] : model;
        this.touchModelUsage(this.currentAgent.runtime_model);
      }
      this.setContextWindowFromCurrentAgent();
      return route;
    },

    async fetchAutoRoutePreflight(message, uploadedFiles) {
      if (!this.currentAgent || !this.isAutoModelSelected()) return null;
      var text = String(message || '').trim();
      if (!text) return null;
      var files = Array.isArray(uploadedFiles) ? uploadedFiles : [];
      var hasVision = files.some(function(f) {
        return String(f && f.content_type ? f.content_type : '').toLowerCase().indexOf('image/') === 0;
      });
      try {
        var result = await InfringAPI.post('/api/route/auto', {
          agent_id: this.currentAgent.id,
          message: text,
          token_count: this.estimateTokensFromText(text),
          has_vision: hasVision,
          attachments: files,
        });
        if (result && result.route && typeof result.route === 'object') return result.route;
        if (result && (result.selected_model || result.selected_provider)) return result;
      } catch (_) {}
      return null;
    },

    inferContextWindowFromModelId(modelId) {
      var value = String(modelId || '').toLowerCase();
      if (!value) return 0;
      var explicitK = value.match(/(?:^|[^0-9])([0-9]{2,4})k(?:[^a-z0-9]|$)/);
      if (explicitK && explicitK[1]) {
        var parsedK = Number(explicitK[1]);
        if (Number.isFinite(parsedK) && parsedK > 0) return parsedK * 1000;
      }
      var explicitM = value.match(/(?:^|[^0-9])([0-9]{1,3})m(?:[^a-z0-9]|$)/);
      if (explicitM && explicitM[1]) {
        var parsedM = Number(explicitM[1]);
        if (Number.isFinite(parsedM) && parsedM > 0) return parsedM * 1000000;
      }
      if (value.indexOf('qwen2.5') >= 0 || value.indexOf('qwen3') >= 0) return 131072;
      if (value.indexOf('kimi') >= 0 || value.indexOf('moonshot') >= 0) return 262144;
      if (value.indexOf('llama-3.3') >= 0 || value.indexOf('llama3.3') >= 0) return 131072;
      if (value.indexOf('llama-3.2') >= 0 || value.indexOf('llama3.2') >= 0) return 128000;
      if (value.indexOf('mistral-nemo') >= 0 || value.indexOf('mixtral') >= 0) return 32000;
      return 0;
    },

    refreshContextWindowMap(models) {
      var next = {};
      var rows = Array.isArray(models) ? models : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var id = String(row.id || '').trim();
        if (!id) continue;
        var windowSize = Number(row.context_window || row.context_window_tokens || 0);
        if (!Number.isFinite(windowSize) || windowSize <= 0) {
          windowSize = this.inferContextWindowFromModelId(id);
        }
        if (Number.isFinite(windowSize) && windowSize > 0) {
          next[id] = Math.round(windowSize);
        }
      }
      this._contextWindowByModel = next;
    },

    setContextWindowFromCurrentAgent() {
      var agent = this.currentAgent || {};
      var direct = Number(agent.context_window || agent.context_window_tokens || 0);
      if (Number.isFinite(direct) && direct > 0) {
        this.contextWindow = Math.round(direct);
        this.refreshContextPressure();
        return;
      }
      var modelName = String(agent.model_name || agent.runtime_model || '').trim();
      var fromMap = Number((this._contextWindowByModel || {})[modelName] || 0);
      if (Number.isFinite(fromMap) && fromMap > 0) {
        this.contextWindow = Math.round(fromMap);
        this.refreshContextPressure();
        return;
      }
      var inferred = this.inferContextWindowFromModelId(modelName);
      if (Number.isFinite(inferred) && inferred > 0) {
        this.contextWindow = Math.round(inferred);
        this.refreshContextPressure();
      }
    },

    refreshContextPressure() {
      var windowSize = Number(this.contextWindow || 0);
      var used = Number(this.contextApproxTokens || 0);
      if (!Number.isFinite(windowSize) || windowSize <= 0 || !Number.isFinite(used) || used < 0) return;
      var ratio = used / windowSize;
      if (ratio >= 0.96) this.contextPressure = 'critical';
      else if (ratio >= 0.82) this.contextPressure = 'high';
      else if (ratio >= 0.55) this.contextPressure = 'medium';
      else this.contextPressure = 'low';
    },

    normalizePromptSuggestions(rows) {
      var source = Array.isArray(rows) ? rows : [];
      var seen = {};
      var out = [];
      var wordCount = function(text) {
        return String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean).length;
      };
      var clampWords = function(text, maxWords) {
        var cap = Number(maxWords || 12);
        var words = String(text == null ? '' : text).trim().split(/\s+/g).filter(Boolean);
        if (!words.length) return '';
        if (!Number.isFinite(cap) || cap < 4) cap = 12;
        if (words.length <= cap) return words.join(' ');
        return words.slice(0, cap).join(' ');
      };
      var normalizeVoice = function(value) {
        var row = String(value == null ? '' : value)
          .replace(/\s+/g, ' ')
          .trim();
        if (!row) return '';
        row = row
          .replace(/^\s*[-*0-9.)\]]+\s*/, '')
          .replace(/^\s*\[[^\]\n]{2,96}\]\s*/, '')
          .replace(/^\s*(?:\*\*)?(?:agent|assistant|system|model|ai|jarvis|user|human)(?:\*\*)?\s*:\s*/i, '')
          .replace(/^ask\s+[^.?!]{0,140}?\s+to\s+/i, '')
          .replace(/^ask\s+[^.?!]{0,140}?\s+for\s+/i, 'Can you ')
          .replace(/^ask\s+for\s+/i, 'Can you ')
          .replace(/^request\s+/i, 'Can you ')
          .replace(/^please\s+request\s+/i, 'Can you ')
          .replace(/^give me\s+/i, 'Can you ')
          .replace(/^show me\s+/i, 'Can you show ')
          .replace(/\s+/g, ' ')
          .trim();
        row = clampWords(row, 12);
        row = row.replace(/[.!?]+$/g, '').trim();
        if (!row) return '';
        if (/^(can|could|would|should|what|why|how|when|where|who)\b/i.test(row)) row = row + '?';
        if (row.length) row = row.charAt(0).toUpperCase() + row.slice(1);
        if (row.length > 180) row = row.substring(0, 177) + '...';
        return row;
      };
      var isLowValue = function(text) {
        var lowered = String(text || '').toLowerCase();
        if (!lowered) return true;
        if (lowered.indexOf('the infring runtime is currently') >= 0) return true;
        if (lowered.indexOf('if you need help') >= 0 || lowered.indexOf('feel free to ask') >= 0) return true;
        if (lowered.indexOf('the user wants exactly 3 actionable next user prompts') >= 0) return true;
        if (lowered.indexOf('json array of strings') >= 0) return true;
        if (lowered.indexOf('output only the') >= 0) return true;
        if (lowered.indexOf('do not include numbering') >= 0) return true;
        if (lowered.indexOf('highest-roi') >= 0) return true;
        if (lowered.indexOf('runbook') >= 0) return true;
        if (lowered.indexOf('reliability remediation') >= 0) return true;
        if (lowered.indexOf('rollback criteria') >= 0) return true;
        if (lowered.indexOf('3-step execution plan') >= 0) return true;
        if (lowered.indexOf('this task') >= 0) return true;
        if (lowered === 'thinking...' || lowered === 'thinking..' || lowered === 'thinking.') return true;
        var sentenceCount = (text.match(/[.!?]/g) || []).length;
        if (sentenceCount > 2) return true;
        if (/[\"“”]/.test(text) && text.length > 120) return true;
        if (/^(give me|request|ask for)\b/i.test(text)) return true;
        var words = wordCount(text);
        if (words < 3 || words > 14) return true;
        var actionableStart =
          /^(can|could|would|should|what|why|how|when|where|who|show|fix|check|run|retry|switch|clear|drain|scale|continue|compare|explain|validate|review|open|trace)\b/i.test(text);
        if (!actionableStart && text.indexOf('?') < 0 && /^\s*(the|it|this|that)\b/i.test(text)) return true;
        return false;
      };
      for (var i = 0; i < source.length; i++) {
        var raw = normalizeVoice(source[i]);
        if (!raw || isLowValue(raw)) continue;
        var key = String(raw || '').toLowerCase();
        if (seen[key]) continue;
        seen[key] = true;
        out.push(raw);
        if (out.length >= 4) break;
      }
      return out;
    },

    derivePromptSuggestionFallback(agent, hint) {
      var rows = [];
      var compact = function(value) {
        var text = String(value == null ? '' : value)
          .replace(/^\s*(?:agent|assistant|system|user|jarvis)\s*:\s*/i, '')
          .replace(/\s+/g, ' ')
          .trim();
        if (!text) return '';
        if (text.length > 180) return text.substring(0, 177) + '...';
        return text;
      };
      var sanitizeHint = function(value) {
        var text = compact(value || '');
        if (!text) return '';
        var lowered = text.toLowerCase();
        if (
          lowered === 'post-response' ||
          lowered === 'post-silent' ||
          lowered === 'post-error' ||
          lowered === 'post-terminal' ||
          lowered === 'init' ||
          lowered === 'refresh'
        ) return '';
        if (/^post-[a-z0-9_-]+$/i.test(text)) return '';
        return text;
      };
      var role = compact(agent && agent.role ? agent.role : '') || 'assistant';
      var context = this.collectPromptSuggestionContext();
      var lastUser = compact(context.lastUser || '');
      var lastAgent = compact(context.lastAgent || '');
      var cleanHint = sanitizeHint(hint || '');
      var topic = compact(cleanHint || lastUser || lastAgent || '');
      var topicWords = String(topic || '')
        .toLowerCase()
        .split(/[^a-z0-9_:-]+/g)
        .filter(function(word) {
        return word && word.length >= 4 && ['that', 'with', 'from', 'this', 'your', 'have', 'will', 'into'].indexOf(word) === -1;
      })
        .slice(0, 3);
      var topicLabel = topicWords.length ? topicWords.join(' ') : 'current task';
      var combinedLower = [cleanHint, lastUser, lastAgent].join(' ').toLowerCase();
      var rotateSeed = topicLabel + '|' + String(context.signature || '') + '|' + cleanHint;
      var rotate = 0;
      for (var ridx = 0; ridx < rotateSeed.length; ridx++) {
        rotate = (rotate + rotateSeed.charCodeAt(ridx)) % 97;
      }

      if (/\bcouldn'?t reach|failed to|timeout|lane_timeout|backend unavailable|provider-sync\b/i.test(combinedLower)) {
        rows.push('Can you auto-switch models and retry the same request');
        rows.push('What failed first, provider sync or app-plane lane');
      }
      if (/\bqueue|cockpit|conduit|latency|backpressure|reconnect|stale\b/i.test(combinedLower)) {
        rows.push('Can you reclaim stale blocks and verify queue depth after');
        rows.push('Can you scale conduit and report before and after metrics');
      }
      if (/\bupload|file|attachment\b/i.test(combinedLower)) {
        rows.push('Can you retry upload and show the failing endpoint');
      }
      if (/\bdiff|patch|commit|branch|git\b/i.test(combinedLower)) {
        rows.push('Can you show the exact diff for that change');
      }
      if (cleanHint) rows.push('Can you take the next step on ' + topicLabel);
      if (lastAgent && !/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(lastAgent)) {
        rows.push('Can you turn that into a concrete checklist');
        rows.push('Can you show the first command to run now');
      }
      if (lastUser) {
        rows.push('Can you continue this and keep the same direction');
        rows.push('Can you summarize progress in three concrete bullets');
      }
      rows.push('Can you propose the best next move from here');
      rows.push('Can you verify the latest change and report result');

      if (rows.length > 1) {
        rotate = rotate % rows.length;
        rows = rows.slice(rotate).concat(rows.slice(0, rotate));
      }

      var normalized = this.normalizePromptSuggestions(rows);
      while (normalized.length < 3) {
        var fill = this.normalizePromptSuggestions([
          'Can you show the single next command to run',
          'Can you summarize progress in three concrete bullets',
          'Can you verify the latest change and report result'
        ]);
        if (!fill.length) break;
        for (var j = 0; j < fill.length && normalized.length < 3; j++) {
          if (normalized.indexOf(fill[j]) >= 0) continue;
          normalized.push(fill[j]);
        }
        break;
      }
      return normalized.slice(0, 4);
    },

    collectPromptSuggestionContext() {
      var out = { lastUser: '', lastAgent: '', history: [], signature: '' };
      var history = Array.isArray(this.messages) ? this.messages : [];
      var compact = function(value, maxLen) {
        var cap = Number(maxLen || 240);
        var text = String(value == null ? '' : value).replace(/\s+/g, ' ').trim();
        if (!text) return '';
        if (text.length > cap) return text.substring(0, Math.max(8, cap - 3)) + '...';
        return text;
      };
      for (var i = history.length - 1; i >= 0; i--) {
        var row = history[i];
        if (!row || row.thinking || row.streaming || row.terminal || row.is_notice) continue;
        var text = compact(row.text, 240);
        if (!text) continue;
        if (/^\[runtime-task\]/i.test(text)) continue;
        if (/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(text)) continue;
        if (/the user wants exactly 3 actionable next user prompts/i.test(text)) continue;
        if (String(text || '').toLowerCase() === 'heartbeat_ok') continue;
        if (out.history.length < 8) {
          out.history.unshift({
            role: compact(row.role || '', 16).toLowerCase() || (row.user ? 'user' : row.assistant ? 'agent' : 'agent'),
            text: text
          });
        }
        if (!out.lastUser && row.role === 'user') {
          out.lastUser = text;
          continue;
        }
        if (!out.lastAgent && row.role === 'agent') {
          out.lastAgent = text;
        }
        if (out.lastUser && out.lastAgent) break;
      }
      out.signature = compact(
        out.history
          .map(function(entry) {
            return compact(entry.role || 'agent', 20) + ':' + compact(entry.text || '', 180);
          })
          .join(' || ') ||
          (String(out.lastUser || '') + '|' + String(out.lastAgent || '')),
        1200
      );
      return out;
    },

    nextPromptQueueId() {
      this._promptQueueSeq = Number(this._promptQueueSeq || 0) + 1;
      return 'pq-' + String(Date.now()) + '-' + String(this._promptQueueSeq);
    },

    get promptQueueItems() {
      var queue = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var out = [];
      for (var i = 0; i < queue.length; i++) {
        var row = queue[i];
        if (!row || row.terminal) continue;
        if (!row.queue_id) row.queue_id = this.nextPromptQueueId();
        if (!row.queue_kind) row.queue_kind = 'prompt';
        out.push({
          queue_id: String(row.queue_id),
          queue_index: i,
          text: String(row.text || '').trim(),
          files: Array.isArray(row.files) ? row.files : [],
          images: Array.isArray(row.images) ? row.images : [],
          queued_at: Number(row.queued_at || 0) || Date.now()
        });
      }
      return out;
    },

    get hasPromptQueue() {
      return Array.isArray(this.promptQueueItems) && this.promptQueueItems.length > 0;
    },

    queuePromptPreview(item) {
      var text = String(item && item.text ? item.text : '').replace(/\s+/g, ' ').trim();
      if (!text) return '(queued prompt)';
      return text.length > 140 ? text.substring(0, 137) + '...' : text;
    },

    removePromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      if (!this.hasPromptQueue && !this.sending && this.currentAgent) {
        this.refreshPromptSuggestions(false, 'queue-cleared');
      }
      this.scheduleConversationPersist();
    },

    movePromptQueueItem(sourceId, targetId) {
      var src = String(sourceId || '').trim();
      var dst = String(targetId || '').trim();
      if (!src || !dst || src === dst) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue.slice() : [];
      var srcIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === src); });
      var dstIdx = rows.findIndex(function(row) { return !!(row && String(row.queue_id || '').trim() === dst); });
      if (srcIdx < 0 || dstIdx < 0) return;
      var moving = rows[srcIdx];
      rows.splice(srcIdx, 1);
      if (dstIdx > srcIdx) dstIdx -= 1;
      rows.splice(dstIdx, 0, moving);
      this.messageQueue = rows;
      this.scheduleConversationPersist();
    },

    onPromptQueueDragStart(queueId, event) {
      var id = String(queueId || '').trim();
      if (!id) return;
      this.promptQueueDragId = id;
      if (event && event.dataTransfer) {
        event.dataTransfer.effectAllowed = 'move';
        try { event.dataTransfer.setData('text/plain', id); } catch(_) {}
      }
    },

    onPromptQueueDrop(targetId, event) {
      if (event && typeof event.preventDefault === 'function') event.preventDefault();
      var sourceId = String(this.promptQueueDragId || '').trim();
      if (!sourceId && event && event.dataTransfer) {
        try {
          sourceId = String(event.dataTransfer.getData('text/plain') || '').trim();
        } catch(_) {}
      }
      var destinationId = String(targetId || '').trim();
      if (sourceId && destinationId) {
        this.movePromptQueueItem(sourceId, destinationId);
      }
      this.promptQueueDragId = '';
    },

    onPromptQueueDragEnd() {
      this.promptQueueDragId = '';
    },

    async steerPromptQueueItem(queueId) {
      var id = String(queueId || '').trim();
      if (!id) return;
      var rows = Array.isArray(this.messageQueue) ? this.messageQueue : [];
      var idx = rows.findIndex(function(row) {
        return !!(row && String(row.queue_id || '').trim() === id);
      });
      if (idx < 0) return;
      var item = rows[idx];
      rows.splice(idx, 1);
      this.messageQueue = rows.slice();
      var text = String(item && item.text ? item.text : '').trim();
      var files = Array.isArray(item && item.files) ? item.files : [];
      var images = Array.isArray(item && item.images) ? item.images : [];
      if (!text) return;
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: 'Steer injected into active workflow.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();

      if (!this.sending) {
        this._sendPayload(text, files, images, { steer_injected: true });
        return;
      }

      var wsPayload = { type: 'message', content: text, steer: true, priority: 'steer' };
      if (files.length) wsPayload.attachments = files;
      if (InfringAPI.wsSend(wsPayload)) return;

      if (this.currentAgent && this.currentAgent.id) {
        try {
          await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/message', {
            message: text,
            attachments: files,
            steer: true,
            priority: 'steer',
          });
          return;
        } catch(_) {}
      }

      this.messageQueue.unshift({
        queue_id: id,
        queue_kind: 'prompt',
        text: text,
        files: files,
        images: images,
        queued_at: Number(item && item.queued_at ? item.queued_at : Date.now()),
      });
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: 'Steer injection failed; prompt returned to queue.',
        meta: '',
        tools: [],
        system_origin: 'prompt_queue:steer',
        ts: Date.now(),
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    clearPromptSuggestions() {
      this.promptSuggestions = [];
      this.suggestionsLoading = false;
      this._lastSuggestionsAt = 0;
      this._lastSuggestionsAgentId = '';
    },

    async applyPromptSuggestion(suggestion) {
      var text = String(suggestion == null ? '' : suggestion).trim();
      if (!text) return;
      this.inputText = text;
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showAttachMenu = false;
      await this.sendMessage();
    },

    async refreshPromptSuggestions(force, hint) {
      var agent = this.currentAgent;
      if (!agent || !agent.id) {
        this.promptSuggestions = [];
        return;
      }
      if (this.terminalMode || this.showFreshArchetypeTiles) {
        this.promptSuggestions = [];
        return;
      }
      if (this.hasPromptQueue) {
        this.promptSuggestions = [];
        this.suggestionsLoading = false;
        return;
      }
      var now = Date.now();
      var agentId = String(agent.id);
      var recentlyFetched =
        !force &&
        this._lastSuggestionsAgentId === agentId &&
        (now - Number(this._lastSuggestionsAt || 0)) < 12000 &&
        Array.isArray(this.promptSuggestions) &&
        this.promptSuggestions.length > 0;
      if (recentlyFetched) return;

      var seq = Number(this._suggestionFetchSeq || 0) + 1;
      this._suggestionFetchSeq = seq;
      this.suggestionsLoading = true;
      try {
        var payload = {};
        var cleanHint = String(hint || '').trim();
        if (/^(post-(response|silent|error|terminal)|init|refresh)$/i.test(cleanHint)) cleanHint = '';
        if (cleanHint) payload.hint = cleanHint;
        var context = this.collectPromptSuggestionContext();
        if (context.lastUser) payload.last_user_message = String(context.lastUser).trim();
        if (context.lastAgent) payload.last_agent_message = String(context.lastAgent).trim();
        if (context.signature) payload.recent_context = String(context.signature).trim();
        if (Array.isArray(context.history) && context.history.length) {
          payload.recent_history = context.history
            .map(function(entry) {
              return String(entry && entry.role ? entry.role : 'agent') + ': ' + String(entry && entry.text ? entry.text : '');
            })
            .join(' || ')
            .trim();
        }
        var activeModel = String(agent && (agent.runtime_model || agent.model_name) ? (agent.runtime_model || agent.model_name) : '').trim();
        if (activeModel) payload.current_model = activeModel;
        var result = await InfringAPI.post('/api/agents/' + encodeURIComponent(agentId) + '/suggestions', payload);
        if (this._suggestionFetchSeq !== seq) return;
        var suggestions = this.normalizePromptSuggestions(result && result.suggestions ? result.suggestions : []);
        if (!suggestions.length) {
          suggestions = this.derivePromptSuggestionFallback(agent, cleanHint);
        }
        this.promptSuggestions = suggestions;
        this._lastSuggestionsAt = Date.now();
        this._lastSuggestionsAgentId = agentId;
      } catch (_) {
        if (this._suggestionFetchSeq === seq) {
          this.promptSuggestions = this.derivePromptSuggestionFallback(agent, cleanHint);
          this._lastSuggestionsAt = Date.now();
          this._lastSuggestionsAgentId = agentId;
        }
      } finally {
        if (this._suggestionFetchSeq === seq) this.suggestionsLoading = false;
      }
    },

    startFreshInitSequence(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      var token = Number(this.freshInitStageToken || 0) + 1;
      this.freshInitStageToken = token;
      this._freshInitThreadShownFor = agentId;
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      var agentName = String(agent.name || agent.id || 'agent').trim() || 'agent';
      this.messages = [
        {
          id: ++msgId,
          role: 'agent',
          text: 'Thinking...',
          meta: '',
          tools: [],
          ts: Date.now(),
          thinking: true,
          agent_id: agentId,
          agent_name: agentName
        }
      ];
      this.recomputeContextEstimate();
      this.cacheAgentConversation(agentId);
      var self = this;
      this.$nextTick(function() {
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
      });

      setTimeout(function() {
        if (Number(self.freshInitStageToken || 0) !== token) return;
        if (!self.currentAgent || String(self.currentAgent.id || '') !== agentId) return;
        self.messages = [
          {
            id: ++msgId,
            role: 'agent',
            text: 'Who am I?',
            meta: '',
            tools: [],
            ts: Date.now(),
            agent_id: agentId,
            agent_name: agentName
          }
        ];
        self.recomputeContextEstimate();
        self.cacheAgentConversation(agentId);
        self.$nextTick(function() {
          self.scrollToBottomImmediate();
          self.stabilizeBottomScroll();
        });

        setTimeout(function() {
          if (Number(self.freshInitStageToken || 0) !== token) return;
          if (!self.currentAgent || String(self.currentAgent.id || '') !== agentId) return;
          self.freshInitRevealMenu = true;
          self.showFreshArchetypeTiles = true;
          self.$nextTick(function() {
            self.scrollToBottomImmediate();
            self.stabilizeBottomScroll();
          });
        }, 500);
      }, 500);
    },

    ensureFreshInitThread(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if (this._freshInitThreadShownFor === agentId && Array.isArray(this.messages) && this.messages.length > 0) {
        return;
      }
      this.startFreshInitSequence(agent);
    },

    pointerFxThemeMode() {
      try {
        var bodyTheme = '';
        var rootTheme = '';
        if (document && document.body && document.body.dataset) {
          bodyTheme = String(document.body.dataset.theme || '').toLowerCase().trim();
        }
        if (document && document.documentElement) {
          rootTheme = String(
            (document.documentElement.dataset && document.documentElement.dataset.theme) ||
            document.documentElement.getAttribute('data-theme') ||
            ''
          ).toLowerCase().trim();
        }
        var resolved = bodyTheme || rootTheme;
        if (!resolved) {
          try {
            resolved = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
              ? 'dark'
              : 'light';
          } catch(_) {
            resolved = 'light';
          }
        }
        if (document && document.body && document.body.dataset) {
          if (!bodyTheme || bodyTheme !== resolved) {
            document.body.dataset.theme = resolved;
          }
        }
        return resolved === 'dark' ? 'dark' : 'light';
      } catch(_) {
        return 'light';
      }
    },

    spawnPointerTrail(container, x, y, opts) {
      if (!container) return;
      var options = opts || {};
      var marker = document.createElement('span');
      marker.className = 'chat-pointer-trail-dot';
      marker.style.left = x + 'px';
      marker.style.top = y + 'px';
      if (Number.isFinite(Number(options.size))) marker.style.setProperty('--trail-size', String(Number(options.size)));
      if (Number.isFinite(Number(options.opacity))) marker.style.setProperty('--trail-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.scale))) marker.style.setProperty('--trail-scale', String(Number(options.scale)));
      if (Number.isFinite(Number(options.hueShift))) marker.style.setProperty('--trail-hue-shift', String(Number(options.hueShift)) + 'deg');
      container.appendChild(marker);
      setTimeout(function() {
        try { marker.remove(); } catch(_) {}
      }, 860);
    },

    spawnPointerTrailSegment(container, x0, y0, x1, y1, opts) {
      if (!container) return;
      var dx = Number(x1 || 0) - Number(x0 || 0);
      var dy = Number(y1 || 0) - Number(y0 || 0);
      var dist = Math.sqrt(dx * dx + dy * dy);
      if (!Number.isFinite(dist) || dist < 0.75) return;
      var options = opts || {};
      var seg = document.createElement('span');
      seg.className = 'chat-pointer-trail-segment';
      var mx = Number(x0 || 0) + (dx * 0.5);
      var my = Number(y0 || 0) + (dy * 0.5);
      var angle = Math.atan2(dy, dx) * (180 / Math.PI);
      seg.style.left = mx + 'px';
      seg.style.top = my + 'px';
      seg.style.width = Math.max(2, dist + 1) + 'px';
      seg.style.transform = 'translate(-50%, -50%) rotate(' + angle + 'deg)';
      if (Number.isFinite(Number(options.thickness))) seg.style.setProperty('--trail-seg-thickness', String(Number(options.thickness)));
      if (Number.isFinite(Number(options.opacity))) seg.style.setProperty('--trail-seg-opacity', String(Number(options.opacity)));
      if (Number.isFinite(Number(options.hueShift))) seg.style.setProperty('--trail-seg-hue-shift', String(Number(options.hueShift)) + 'deg');
      container.appendChild(seg);
      setTimeout(function() {
        try { seg.remove(); } catch(_) {}
      }, 760);
    },

    spawnPointerRipple(container, x, y) {
      if (!container) return;
      var ripple = document.createElement('span');
      ripple.className = 'chat-pointer-ripple';
      ripple.style.left = x + 'px';
      ripple.style.top = y + 'px';
      container.appendChild(ripple);
      setTimeout(function() {
        try { ripple.remove(); } catch(_) {}
      }, 820);
    },

    handleMessagesPointerMove(event) {
      if (!event || !event.currentTarget) return;
      if (this.pointerFxThemeMode() !== 'dark') return;
      var now = Date.now();
      if ((now - Number(this._pointerTrailLastAt || 0)) < 8) return;
      this._pointerTrailLastAt = now;
      var host = event.currentTarget;
      var rect = host.getBoundingClientRect();
      // Place markers in scroll-content coordinates so they stay locked to cursor in a scrolling container.
      var x = event.clientX - rect.left + Number(host.scrollLeft || 0);
      var y = event.clientY - rect.top + Number(host.scrollTop || 0);
      host.style.setProperty('--chat-grid-x', Math.round(x) + 'px');
      host.style.setProperty('--chat-grid-y', Math.round(y) + 'px');
      host.style.setProperty('--chat-grid-active', '1');
      if (this._pointerGridHideTimer) {
        clearTimeout(this._pointerGridHideTimer);
      }
      var self = this;
      this._pointerGridHideTimer = setTimeout(function() {
        try { host.style.setProperty('--chat-grid-active', '0'); } catch(_) {}
        self._pointerGridHideTimer = null;
      }, 180);
      if (!this._pointerTrailSeeded) {
        this._pointerTrailLastX = x;
        this._pointerTrailLastY = y;
        this._pointerTrailSeeded = true;
      }
      var dx = x - Number(this._pointerTrailLastX || x);
      var dy = y - Number(this._pointerTrailLastY || y);
      var dist = Math.sqrt(dx * dx + dy * dy);
      var spacing = 0.72;
      var steps = Math.max(1, Math.min(52, Math.ceil(dist / spacing)));
      for (var i = 1; i <= steps; i++) {
        var t0 = (i - 1) / steps;
        var t1 = i / steps;
        var sx0 = this._pointerTrailLastX + (dx * t0);
        var sy0 = this._pointerTrailLastY + (dy * t0);
        var sx1 = this._pointerTrailLastX + (dx * t1);
        var sy1 = this._pointerTrailLastY + (dy * t1);
        var progress = t1;
        var thickness = 2.05 + (progress * 1.85);
        var alpha = 0.32 + (progress * 0.45);
        var hueShift = -4 + (progress * 8);
        this.spawnPointerTrailSegment(host, sx0, sy0, sx1, sy1, {
          thickness: thickness,
          opacity: alpha,
          hueShift: hueShift,
        });
      }
      var canSpawnHead = (now - Number(this._pointerTrailHeadLastAt || 0)) >= 36;
      if (canSpawnHead || dist < 1.5) {
        // Render several smaller head particles instead of one large dot.
        var invDist = dist > 0.0001 ? (1 / dist) : 0;
        var nx = dist > 0.0001 ? (dx * invDist) : 1;
        var ny = dist > 0.0001 ? (dy * invDist) : 0;
        var pxTrail = [
          { back: 0.0, lateral: 0.0, size: 3.3, opacity: 0.56, hue: 0 },
          { back: 1.8, lateral: 0.72, size: 2.9, opacity: 0.48, hue: 2 },
          { back: 2.8, lateral: -0.66, size: 2.6, opacity: 0.42, hue: -2 },
          { back: 3.6, lateral: 0.0, size: 2.3, opacity: 0.36, hue: 1 },
        ];
        for (var j = 0; j < pxTrail.length; j++) {
          var p = pxTrail[j];
          var px = x - (nx * p.back) + (-ny * p.lateral);
          var py = y - (ny * p.back) + (nx * p.lateral);
          this.spawnPointerTrail(host, px, py, {
            size: p.size,
            opacity: p.opacity,
            scale: 1.03,
            hueShift: p.hue,
          });
        }
        this._pointerTrailHeadLastAt = now;
      }
      this._pointerTrailLastX = x;
      this._pointerTrailLastY = y;
    },

    handleMessagesPointerDown(event) {
      if (!event || !event.currentTarget) return;
      if (this.pointerFxThemeMode() !== 'light') return;
      var host = event.currentTarget;
      var rect = host.getBoundingClientRect();
      var x = event.clientX - rect.left;
      var y = event.clientY - rect.top;
      this.spawnPointerRipple(host, x, y);
    },

    clearPointerFx(event) {
      if (!event || !event.currentTarget) return;
      var host = event.currentTarget;
      if (this._pointerGridHideTimer) {
        clearTimeout(this._pointerGridHideTimer);
        this._pointerGridHideTimer = null;
      }
      try { host.style.setProperty('--chat-grid-active', '0'); } catch(_) {}
      var dots = host.querySelectorAll('.chat-pointer-trail-dot,.chat-pointer-trail-segment,.chat-pointer-ripple');
      for (var i = 0; i < dots.length; i++) {
        try { dots[i].remove(); } catch(_) {}
      }
      this._pointerTrailSeeded = false;
      this._pointerTrailHeadLastAt = 0;
    },

    markAgentMessageComplete(msg) {
      if (!msg || msg.role !== 'agent') return;
      msg._finish_bounce = true;
      setTimeout(function() {
        try { msg._finish_bounce = false; } catch(_) {}
      }, 300);
    },

    fetchModelContextWindows(force) {
      var now = Date.now();
      if (!force && this._contextModelsFetchedAt && (now - this._contextModelsFetchedAt) < 300000) {
        this.setContextWindowFromCurrentAgent();
        return Promise.resolve();
      }
      var self = this;
      return InfringAPI.get('/api/models').then(function(data) {
        self.refreshContextWindowMap(data && data.models ? data.models : []);
        self._contextModelsFetchedAt = Date.now();
        self.setContextWindowFromCurrentAgent();
      }).catch(function() {});
    },

    requestContextTelemetry(force) {
      if (!this.currentAgent || !InfringAPI.isWsConnected()) return false;
      var now = Date.now();
      if (!force && (now - Number(this._lastContextRequestAt || 0)) < 2500) return false;
      this._lastContextRequestAt = now;
      return !!InfringAPI.wsSend({ type: 'command', command: 'context', silent: true });
    },

    normalizeModelUsageKey: function(modelId) {
      return String(modelId || '').trim().toLowerCase();
    },

    loadModelUsageCache: function() {
      try {
        var raw = localStorage.getItem(this.modelUsageCacheKey);
        if (!raw) {
          this.modelUsageCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelUsageCache = parsed && typeof parsed === 'object' ? parsed : {};
      } catch {
        this.modelUsageCache = {};
      }
    },

    persistModelUsageCache: function() {
      try {
        localStorage.setItem(this.modelUsageCacheKey, JSON.stringify(this.modelUsageCache || {}));
      } catch {}
    },

    modelUsageTs: function(modelId) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key || !this.modelUsageCache || typeof this.modelUsageCache !== 'object') return 0;
      var ts = Number(this.modelUsageCache[key] || 0);
      return Number.isFinite(ts) && ts > 0 ? ts : 0;
    },

    touchModelUsage: function(modelId, ts) {
      var key = this.normalizeModelUsageKey(modelId);
      if (!key) return;
      if (!this.modelUsageCache || typeof this.modelUsageCache !== 'object') {
        this.modelUsageCache = {};
      }
      var stamp = Number(ts || Date.now());
      this.modelUsageCache[key] = Number.isFinite(stamp) && stamp > 0 ? stamp : Date.now();
      this.persistModelUsageCache();
    },

    loadModelNoticeCache: function() {
      try {
        var raw = localStorage.getItem(this.modelNoticeCacheKey);
        if (!raw) {
          this.modelNoticeCache = {};
          return;
        }
        var parsed = JSON.parse(raw);
        this.modelNoticeCache = (parsed && typeof parsed === 'object') ? parsed : {};
      } catch {
        this.modelNoticeCache = {};
      }
    },

    persistModelNoticeCache: function() {
      try {
        localStorage.setItem(this.modelNoticeCacheKey, JSON.stringify(this.modelNoticeCache || {}));
      } catch {}
    },

    normalizeNoticeType: function(value, fallbackType) {
      var fallback = String(fallbackType || 'info').toLowerCase();
      if (fallback !== 'model' && fallback !== 'info') fallback = 'info';
      var raw = String(value || '').toLowerCase().trim();
      if (raw === 'model' || raw === 'info') return raw;
      return fallback;
    },

    isModelSwitchNoticeLabel: function(label) {
      var text = String(label || '').trim();
      if (!text) return false;
      return /^Model switched (?:to\b|from\b)/i.test(text);
    },

    rememberModelNotice: function(agentId, label, ts, noticeType, noticeIcon) {
      if (!agentId || !label) return;
      if (!this.modelNoticeCache || typeof this.modelNoticeCache !== 'object') {
        this.modelNoticeCache = {};
      }
      var key = String(agentId);
      if (!Array.isArray(this.modelNoticeCache[key])) this.modelNoticeCache[key] = [];
      var list = this.modelNoticeCache[key];
      var tsNum = Number(ts || Date.now());
      var normalizedType = this.normalizeNoticeType(
        noticeType,
        this.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
      );
      var normalizedIcon = String(noticeIcon || '').trim();
      var exists = list.some(function(entry) {
        return (
          entry &&
          entry.label === label &&
          Number(entry.ts || 0) === tsNum &&
          String(entry.type || '') === normalizedType
        );
      });
      if (!exists) list.push({ label: label, ts: tsNum, type: normalizedType, icon: normalizedIcon });
      if (list.length > 120) this.modelNoticeCache[key] = list.slice(list.length - 120);
      this.persistModelNoticeCache();
    },

    mergeModelNoticesForAgent: function(agentId, rows) {
      var list = Array.isArray(rows) ? rows.slice() : [];
      if (!agentId || !this.modelNoticeCache) return list;
      var notices = this.modelNoticeCache[String(agentId)];
      if (!Array.isArray(notices) || !notices.length) return list;
      var existing = {};
      var self = this;
      list.forEach(function(msg) {
        if (!msg) return;
        var label = msg.notice_label || '';
        if (!label && msg.role === 'system' && typeof msg.text === 'string' && self.isModelSwitchNoticeLabel(msg.text.trim())) {
          label = msg.text.trim();
        }
        if (!label) return;
        var type = self.normalizeNoticeType(
          msg.notice_type,
          self.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
        );
        existing[type + '|' + label + '|' + Number(msg.ts || 0)] = true;
      });
      for (var i = 0; i < notices.length; i++) {
        var n = notices[i] || {};
        var nLabel = String(n.label || '').trim();
        if (!nLabel) continue;
        var nTs = Number(n.ts || 0) || Date.now();
        var nType = this.normalizeNoticeType(
          n.type || n.notice_type,
          this.isModelSwitchNoticeLabel(nLabel) ? 'model' : 'info'
        );
        var nIcon = String(n.icon || n.notice_icon || '').trim();
        var nKey = nType + '|' + nLabel + '|' + nTs;
        if (existing[nKey]) continue;
        list.push({
          id: ++msgId,
          role: 'system',
          text: '',
          meta: '',
          tools: [],
          system_origin: 'notice:' + nType,
          is_notice: true,
          notice_label: nLabel,
          notice_type: nType,
          notice_icon: nIcon,
          ts: nTs
        });
      }
      list.sort(function(a, b) {
        return Number((a && a.ts) || 0) - Number((b && b.ts) || 0);
      });
      return list;
    },

    normalizeSessionMessages(data) {
      var source = [];
      if (data && Array.isArray(data.messages)) {
        source = data.messages;
      } else if (data && Array.isArray(data.turns)) {
        var turns = data.turns;
        var turnRows = [];
        turns.forEach(function(turn) {
          var ts = turn && turn.ts ? turn.ts : Date.now();
          if (turn && typeof turn.user === 'string' && turn.user.trim()) {
            turnRows.push({ role: 'User', content: turn.user, ts: ts });
          }
          if (turn && typeof turn.assistant === 'string' && turn.assistant.trim()) {
            turnRows.push({ role: 'Agent', content: turn.assistant, ts: ts });
          }
        });
        source = turnRows;
      } else {
        source = [];
      }
      var self = this;
      return source.map(function(m) {
        var roleRaw = String((m && (m.role || m.type)) || '').toLowerCase();
        var isTerminal = roleRaw.indexOf('terminal') >= 0 || !!(m && m.terminal);
        var role = isTerminal
          ? 'terminal'
          : (roleRaw.indexOf('user') >= 0 ? 'user' : (roleRaw.indexOf('system') >= 0 ? 'system' : 'agent'));
        var textSource = m && (m.content != null ? m.content : (m.text != null ? m.text : m.message));
        if (role === 'user' && m && m.user != null) textSource = m.user;
        if (role !== 'user' && !isTerminal && m && m.assistant != null) textSource = m.assistant;
        var text = typeof textSource === 'string' ? textSource : JSON.stringify(textSource || '');
        text = self.sanitizeToolText(text);
        if (role === 'agent') text = self.stripModelPrefix(text);
        var derivedSystemOrigin = '';
        if (role === 'user' && /^\s*protheus(?:-ops)?\s+/i.test(String(text || ''))) {
          role = 'system';
          derivedSystemOrigin = 'runtime:ops_command';
        }
        if (role === 'user' && /^\s*\[runtime-task\]/i.test(String(text || ''))) {
          role = 'system';
          if (!derivedSystemOrigin) derivedSystemOrigin = 'runtime:task';
        }

        var tools = (m && Array.isArray(m.tools) ? m.tools : []).map(function(t, idx) {
          return {
            id: (t.name || 'tool') + '-hist-' + idx,
            name: t.name || 'unknown',
            running: false,
            expanded: false,
            input: t.input || '',
            result: t.result || '',
            is_error: !!t.is_error
          };
        });
        var images = (m && Array.isArray(m.images) ? m.images : []).map(function(img) {
          return { file_id: img.file_id, filename: img.filename || 'image' };
        });
        var tsRaw = m && (m.ts || m.timestamp || m.created_at || m.createdAt) ? (m.ts || m.timestamp || m.created_at || m.createdAt) : null;
        var ts = null;
        if (typeof tsRaw === 'number') {
          ts = tsRaw;
        } else if (typeof tsRaw === 'string') {
          var parsedTs = Date.parse(tsRaw);
          ts = Number.isNaN(parsedTs) ? null : parsedTs;
        }
        var meta = typeof (m && m.meta) === 'string' ? m.meta : '';
        if (!meta && m && (m.input_tokens || m.output_tokens)) {
          meta = (m.input_tokens || 0) + ' in / ' + (m.output_tokens || 0) + ' out';
        }
        var isNotice = false;
        var noticeLabel = '';
        var noticeType = '';
        var noticeIcon = '';
        if (m && (m.is_notice || m.notice_label || m.notice_type)) {
          var explicitLabel = String(m.notice_label || '').trim();
          var inferredLabel = typeof text === 'string' ? text.trim() : '';
          noticeLabel = explicitLabel || inferredLabel;
          if (noticeLabel) {
            isNotice = true;
            text = '';
            noticeType = self.normalizeNoticeType(
              m.notice_type,
              self.isModelSwitchNoticeLabel(noticeLabel) ? 'model' : 'info'
            );
            noticeIcon = String(m.notice_icon || '').trim();
          }
        }
        if (!isNotice && role === 'system' && typeof text === 'string') {
          var compact = text.trim();
          if (self.isModelSwitchNoticeLabel(compact)) {
            isNotice = true;
            noticeLabel = compact;
            text = '';
            noticeType = 'model';
          }
        }
        var systemOrigin = m && m.system_origin ? String(m.system_origin) : derivedSystemOrigin;
        var compactText = typeof text === 'string' ? text.trim() : '';
        if (
          role === 'system' &&
          !isNotice &&
          !systemOrigin &&
          (
            /^\[runtime-task\]/i.test(compactText) ||
            /^task accepted\.\s*report findings in this thread with receipt-backed evidence\.?$/i.test(compactText)
          )
        ) {
          // Legacy synthetic runtime-task chatter (no origin tag) is noise; skip rendering.
          return null;
        }
        return {
          id: ++msgId,
          role: role,
          text: text,
          meta: meta,
          tools: tools,
          images: images,
          ts: ts,
          is_notice: isNotice,
          notice_label: noticeLabel,
          notice_type: noticeType,
          notice_icon: noticeIcon,
          terminal: isTerminal,
          cwd: m && m.cwd ? String(m.cwd) : '',
          agent_id: m && m.agent_id ? String(m.agent_id) : '',
          agent_name: m && m.agent_name ? String(m.agent_name) : '',
          source_agent_id: m && m.source_agent_id ? String(m.source_agent_id) : '',
          agent_origin: m && m.agent_origin ? String(m.agent_origin) : '',
          system_origin: systemOrigin,
          actor_id: m && m.actor_id ? String(m.actor_id) : '',
          actor: m && m.actor ? String(m.actor) : ''
        };
      }).filter(function(row) { return !!row; });
    },

    init() {
      var self = this;

      if (typeof window !== 'undefined') {
        window.__infringChatCache = window.__infringChatCache || {};
        var persistedCache = this.loadConversationCache();
        var runtimeCache = window.__infringChatCache || {};
        this.conversationCache = Object.assign({}, persistedCache, runtimeCache);
        window.__infringChatCache = this.conversationCache;
      }
      this.loadModelNoticeCache();
      this.loadModelUsageCache();

      // Start tip cycle
      this.startTipCycle();

      // Fetch dynamic commands from server
      this.fetchCommands();
      this.fetchModelContextWindows();

      // Ctrl+/ keyboard shortcut
      document.addEventListener('keydown', function(e) {
        var key = String(e && e.key ? e.key : '').toLowerCase();
        // Ctrl+T or Ctrl+\ toggles terminal compose mode.
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && (key === 't' || key === '\\') && self.currentAgent) {
          e.preventDefault();
          self.toggleTerminalMode();
          return;
        }
        if ((e.ctrlKey || e.metaKey) && e.key === '/') {
          e.preventDefault();
          var input = document.getElementById('msg-input');
          if (input) { input.focus(); self.inputText = '/'; }
        }
        // Ctrl+M for model switcher
        if ((e.ctrlKey || e.metaKey) && e.key === 'm' && self.currentAgent) {
          e.preventDefault();
          self.toggleModelSwitcher();
        }
        // Ctrl+F opens file picker from chat compose.
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && key === 'f' && self.currentAgent) {
          e.preventDefault();
          if (self.terminalMode) {
            self.toggleTerminalMode();
          }
          self.showAttachMenu = true;
          self.$nextTick(function() {
            var input = self.$refs && self.$refs.fileInput ? self.$refs.fileInput : null;
            if (input && typeof input.click === 'function') input.click();
          });
          return;
        }
        // Ctrl+G for chat search
        if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && key === 'g' && self.currentAgent) {
          e.preventDefault();
          self.toggleSearch();
        }
      });

      if (this._sendWatchdogTimer) clearInterval(this._sendWatchdogTimer);
      this._sendWatchdogTimer = setInterval(function() {
        if (self.sending) self._reconcileSendingState();
      }, 3000);
      window.addEventListener('beforeunload', function() {
        if (self._sendWatchdogTimer) {
          clearInterval(self._sendWatchdogTimer);
          self._sendWatchdogTimer = null;
        }
      });

      // Load session + session list when agent changes
      this.$watch('currentAgent', function(agent) {
        if (agent) {
          self.loadSessions(agent.id);
          self.setContextWindowFromCurrentAgent();
          self.requestContextTelemetry(true);
          self.refreshPromptSuggestions(false);
        } else {
          self.clearPromptSuggestions();
        }
      });

      // Check for pending agent from Agents page (set before chat mounted)
      var store = Alpine.store('app');
      if (store.pendingAgent) {
        self.selectAgent(store.pendingAgent);
        store.pendingAgent = null;
      } else if (store.activeAgentId) {
        self.selectAgent(store.activeAgentId);
      } else {
        var preferred = self.pickDefaultAgent(store.agents || []);
        if (preferred) self.selectAgent(preferred);
      }

      // Watch for future pending agent selections (e.g., user clicks agent while on chat)
      this.$watch('$store.app.pendingAgent', function(agent) {
        if (agent) {
          self.selectAgent(agent);
          Alpine.store('app').pendingAgent = null;
        }
      });

      // Keep chat selection synced when an explicit active agent is set globally.
      this.$watch('$store.app.activeAgentId', function(agentId) {
        if (!agentId) return;
        if (!self.currentAgent || self.currentAgent.id !== agentId) {
          self.selectAgent(agentId);
        }
      });

      // Auto-select the first available agent in chat mode.
      this.$watch('$store.app.agents', function(agents) {
        var store = Alpine.store('app');
        var rows = Array.isArray(agents) ? agents : [];
        self.fetchModelContextWindows();
        if (self.currentAgent && self.currentAgent.id) {
          var currentLive = null;
          for (var ai = 0; ai < rows.length; ai++) {
            if (rows[ai] && String(rows[ai].id) === String(self.currentAgent.id)) {
              currentLive = rows[ai];
              break;
            }
          }
          if (!currentLive) {
            var connectionState = String((store && store.connectionState) || '').toLowerCase();
            if (connectionState && connectionState !== 'connected') return;
            var currentId = String(self.currentAgent.id);
            var now = Date.now();
            if (self._agentMissingAgentId !== currentId) {
              self._agentMissingAgentId = currentId;
              self._agentMissingSince = now;
              return;
            }
            var missingForMs = self._agentMissingSince > 0 ? now - self._agentMissingSince : 0;
            var graceMs = Number(self._agentMissingGraceMs || 0);
            if (graceMs > 0 && missingForMs < graceMs) return;
            self._agentMissingAgentId = '';
            self._agentMissingSince = 0;
            self.handleAgentInactive(self.currentAgent.id, 'inactive', { silentNotice: true });
          } else {
            self._agentMissingAgentId = '';
            self._agentMissingSince = 0;
            if (!self.syncCurrentAgentFromStore(currentLive)) {
              self.currentAgent = currentLive;
            }
          }
        }
        if (store.activeAgentId) {
          var resolved = self.resolveAgent(store.activeAgentId);
          if (resolved) {
            if (!self.currentAgent || self.currentAgent.id !== resolved.id) {
              self.selectAgent(resolved);
            } else {
              // Refresh visible metadata without resetting the thread.
              self.syncCurrentAgentFromStore(resolved);
            }
            return;
          }
        }
        if (!self.currentAgent) {
          var preferred = self.pickDefaultAgent(agents || []);
          if (preferred) self.selectAgent(preferred);
        }
      });

      // Watch for slash commands + model autocomplete
      this.$watch('inputText', function(val) {
        if (self.terminalMode) {
          self.updateTerminalCursor();
          self.showSlashMenu = false;
          self.showModelPicker = false;
          return;
        }
        var modelMatch = val.match(/^\/model\s+(.*)$/i);
        if (modelMatch) {
          self.showSlashMenu = false;
          self.modelPickerFilter = modelMatch[1].toLowerCase();
          if (!self.modelPickerList.length) {
            InfringAPI.get('/api/models').then(function(data) {
              self.modelPickerList = (data.models || []).filter(function(m) { return m.available; });
              self.showModelPicker = true;
              self.modelPickerIdx = 0;
            }).catch(function() {});
          } else {
            self.showModelPicker = true;
          }
        } else if (val.startsWith('/')) {
          self.showModelPicker = false;
          self.slashFilter = val.slice(1).toLowerCase();
          self.showSlashMenu = true;
          self.slashIdx = 0;
        } else {
          self.showSlashMenu = false;
          self.showModelPicker = false;
        }
      });

      this.$nextTick(function() {
        self.handleMessagesScroll();
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
      });

      InfringAPI.get('/api/status').then(function(status) {
        var suggested = status && (status.workspace_dir || status.root_dir || status.home_dir)
          ? String(status.workspace_dir || status.root_dir || status.home_dir)
          : '';
        if (suggested) self.terminalCwd = suggested;
      }).catch(function() {});

      if (this._contextTelemetryTimer) clearInterval(this._contextTelemetryTimer);
      this._contextTelemetryTimer = setInterval(function() {
        self.requestContextTelemetry(false);
      }, 8000);
    },

    toggleTerminalMode() {
      this.terminalMode = !this.terminalMode;
      this.showSlashMenu = false;
      this.showModelPicker = false;
      this.showModelSwitcher = false;
      this.terminalCursorFocused = false;
      if (!this.terminalMode) this.terminalSelectionStart = 0;
      if (this.terminalMode && !this.terminalCwd) {
        this.terminalCwd = '/workspace';
      }
      if (this.terminalMode && this.currentAgent) {
        this.connectWs(this.currentAgent.id);
      }
      if (this.terminalMode && Array.isArray(this.attachments) && this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          if (this.attachments[i] && this.attachments[i].preview) {
            try { URL.revokeObjectURL(this.attachments[i].preview); } catch(_) {}
          }
        }
        this.attachments = [];
      }
      var self = this;
      this.$nextTick(function() {
        var input = document.getElementById('msg-input');
        if (input) {
          input.focus();
          if (self.terminalMode) {
            self.setTerminalCursorFocus(true, { target: input });
            self.updateTerminalCursor({ target: input });
          }
        }
        self.scheduleConversationPersist();
      });
    },

    setTerminalCursorFocus(active, event) {
      if (!this.terminalMode) {
        this.terminalCursorFocused = false;
        return;
      }
      this.terminalCursorFocused = !!active;
      if (this.terminalCursorFocused) this.updateTerminalCursor(event);
    },

    updateTerminalCursor(event) {
      if (!this.terminalMode) {
        this.terminalSelectionStart = 0;
        return;
      }
      var text = String(this.inputText || '');
      var active = (typeof document !== 'undefined' && document.activeElement && document.activeElement.id === 'msg-input')
        ? document.activeElement
        : null;
      var el = event && event.target ? event.target : (active || document.getElementById('msg-input'));
      var pos = text.length;
      if (el && Number.isFinite(Number(el.selectionStart))) pos = Number(el.selectionStart);
      if (!Number.isFinite(pos) || pos < 0) pos = text.length;
      if (pos > text.length) pos = text.length;
      this.terminalSelectionStart = Math.floor(pos);
    },

    installChatMapWheelLock() {
      var maps = document.querySelectorAll('.chat-map-scroll');
      if (!maps || !maps.length) return;
      for (var i = 0; i < maps.length; i++) {
        var map = maps[i];
        if (!map || map.__ofWheelLock) continue;
        map.__ofWheelLock = true;
        map.addEventListener('wheel', function(ev) {
          var target = ev.currentTarget;
          if (!target) return;
          if (!target.matches(':hover')) return;
          // Keep wheel behavior scoped to chat map so the page does not scroll beneath it.
          var delta = Number(ev.deltaY || 0);
          if (delta !== 0) {
            target.scrollTop += delta;
          }
          ev.preventDefault();
        }, { passive: false });
      }
    },

    get filteredModelPicker() {
      if (!this.modelPickerFilter) return this.modelPickerList.slice(0, 15);
      var f = this.modelPickerFilter;
      return this.modelPickerList.filter(function(m) {
        return m.id.toLowerCase().indexOf(f) !== -1 || (m.display_name || '').toLowerCase().indexOf(f) !== -1 || m.provider.toLowerCase().indexOf(f) !== -1;
      }).slice(0, 15);
    },

    pickModel(modelId) {
      this.showModelPicker = false;
      this.inputText = '/model ' + modelId;
      this.sendMessage();
    },

    toggleModelSwitcher() {
      if (this.showModelSwitcher) { this.showModelSwitcher = false; return; }
      var self = this;
      var now = Date.now();
      this.modelApiKeyStatus = '';
      if (this._modelCache && (now - this._modelCacheTime) < 300000) {
        this.modelSwitcherFilter = '';
        this.modelSwitcherProviderFilter = '';
        this.modelSwitcherIdx = 0;
        this.showModelSwitcher = true;
        this.$nextTick(function() {
          var el = document.getElementById('model-switcher-search');
          if (el) el.focus();
        });
        return;
      }
      InfringAPI.get('/api/models').then(function(data) {
        var models = (data.models || []).filter(function(m) { return m.available; });
        self._modelCache = models;
        self._modelCacheTime = Date.now();
        self.modelPickerList = models;
        self.modelSwitcherFilter = '';
        self.modelSwitcherProviderFilter = '';
        self.modelSwitcherIdx = 0;
        self.showModelSwitcher = true;
        self.$nextTick(function() {
          var el = document.getElementById('model-switcher-search');
          if (el) el.focus();
        });
      }).catch(function(e) {
        InfringToast.error('Failed to load models: ' + e.message);
      });
    },

    discoverModelsFromApiKey: function() {
      var self = this;
      var entry = String(this.modelApiKeyInput || '').trim();
      if (!entry) {
        InfringToast.error('Enter an API key or local model path first');
        return;
      }
      this.modelApiKeySaving = true;
      this.modelApiKeyStatus = 'Detecting...';
      InfringAPI.post('/api/models/discover', {
        input: entry,
        api_key: entry
      }).then(function(resp) {
        var provider = String((resp && resp.provider) || '').trim();
        var inputKind = String((resp && resp.input_kind) || '').trim().toLowerCase();
        var count = Number((resp && resp.model_count) || ((resp && resp.models && resp.models.length) || 0));
        self.modelApiKeyInput = '';
        if (inputKind === 'local_path') {
          self.modelApiKeyStatus = provider
            ? ('Indexed local path to ' + provider + ' (' + count + ' models)')
            : ('Indexed local path (' + count + ' models)');
        } else {
          self.modelApiKeyStatus = provider ? ('Added ' + provider + ' (' + count + ' models)') : 'API key saved';
        }
        self._modelCache = null;
        self._modelCacheTime = 0;
        return InfringAPI.get('/api/models');
      }).then(function(data) {
        var models = (data && data.models) || [];
        self._modelCache = models.filter(function(m) { return m.available; });
        self._modelCacheTime = Date.now();
        self.modelPickerList = self._modelCache;
      }).catch(function(e) {
        self.modelApiKeyStatus = '';
        InfringToast.error('Model discovery failed: ' + (e && e.message ? e.message : e));
      }).finally(function() {
        self.modelApiKeySaving = false;
      });
    },

    resolveModelContextWindowForSwitch: function(targetModelRef) {
      var modelId = '';
      var explicitWindow = 0;
      if (targetModelRef && typeof targetModelRef === 'object') {
        modelId = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
        explicitWindow = Number(
          targetModelRef.context_window || targetModelRef.context_window_tokens || 0
        );
      } else {
        modelId = String(targetModelRef || '').trim();
      }
      if (Number.isFinite(explicitWindow) && explicitWindow > 0) {
        return Math.round(explicitWindow);
      }
      var map = this._contextWindowByModel || {};
      var candidates = [];
      if (modelId) {
        candidates.push(modelId);
        if (modelId.indexOf('/') >= 0) {
          candidates.push(modelId.split('/').slice(-1)[0]);
        }
      }
      for (var i = 0; i < candidates.length; i++) {
        var fromMap = Number(map[candidates[i]] || 0);
        if (Number.isFinite(fromMap) && fromMap > 0) {
          return Math.round(fromMap);
        }
      }
      var inferred = this.inferContextWindowFromModelId(
        modelId.indexOf('/') >= 0 ? modelId.split('/').slice(-1)[0] : modelId
      );
      if (Number.isFinite(inferred) && inferred > 0) {
        return Math.round(inferred);
      }
      return 0;
    },

    ensureContextBudgetForModelSwitch: function(agentId, targetModelRef, options) {
      var self = this;
      var opts = options && typeof options === 'object' ? options : {};
      var id = String(agentId || '').trim();
      if (!id) return Promise.resolve({ compacted: false });
      var targetWindow = self.resolveModelContextWindowForSwitch(targetModelRef);
      var usedTokens = Number(self.contextApproxTokens || 0);
      if (
        !Number.isFinite(targetWindow) ||
        targetWindow <= 0 ||
        !Number.isFinite(usedTokens) ||
        usedTokens <= targetWindow
      ) {
        return Promise.resolve({
          compacted: false,
          target_context_window: targetWindow,
          before_tokens: Math.max(0, Math.round(usedTokens || 0)),
          after_tokens: Math.max(0, Math.round(usedTokens || 0))
        });
      }
      var targetRatio = Number(opts.target_ratio);
      if (!Number.isFinite(targetRatio) || targetRatio <= 0 || targetRatio >= 1) {
        targetRatio = 0.8;
      }
      var targetTokens = Math.max(1, Math.floor(targetWindow * targetRatio));
      InfringToast.info('Switching to a model with smaller context may degrade performance.');
      return InfringAPI.post('/api/agents/' + encodeURIComponent(id) + '/session/compact', {
        target_context_window: targetWindow,
        target_ratio: targetRatio,
        min_recent_messages: 12,
        max_messages: 200
      }).then(function(resp) {
        var beforeTokens = Number(
          resp && resp.before_tokens != null ? resp.before_tokens : usedTokens
        );
        var afterTokens = Number(
          resp && resp.after_tokens != null ? resp.after_tokens : Math.min(usedTokens, targetTokens)
        );
        if (Number.isFinite(afterTokens) && afterTokens >= 0) {
          self.contextApproxTokens = Math.max(0, Math.round(afterTokens));
        }
        if (Number.isFinite(targetWindow) && targetWindow > 0) {
          self.contextWindow = Math.round(targetWindow);
        }
        self.refreshContextPressure();
        self.addNoticeEvent({
          notice_label:
            'Context compacted from ' +
            self.formatTokenK(beforeTokens) +
            ' to ' +
            self.formatTokenK(afterTokens) +
            ' tokens (target ' +
            self.formatTokenK(targetTokens) +
            ')',
          notice_type: 'info',
          ts: Date.now()
        });
        return resp || {};
      });
    },

    switchAgentModelWithGuards: function(targetModelRef, options) {
      var self = this;
      var opts = options && typeof options === 'object' ? options : {};
      var agentId = String(
        opts.agent_id || (self.currentAgent && self.currentAgent.id) || ''
      ).trim();
      if (!agentId) return Promise.reject(new Error('No agent selected'));
      var requestedModel = '';
      if (targetModelRef && typeof targetModelRef === 'object') {
        requestedModel = String(
          targetModelRef.id || targetModelRef.model || targetModelRef.model_name || ''
        ).trim();
      } else {
        requestedModel = String(targetModelRef || '').trim();
      }
      if (!requestedModel) return Promise.reject(new Error('Model is required'));
      var previousModel = String(
        opts.previous_model != null
          ? opts.previous_model
          : ((self.currentAgent && (self.currentAgent.runtime_model || self.currentAgent.model_name)) || '')
      ).trim();
      var previousProvider = String(
        opts.previous_provider != null
          ? opts.previous_provider
          : ((self.currentAgent && self.currentAgent.model_provider) || '')
      ).trim();
      return self
        .ensureContextBudgetForModelSwitch(agentId, targetModelRef, opts)
        .catch(function(error) {
          InfringToast.error(
            'Context compaction failed before model switch: ' +
              (error && error.message ? error.message : error)
          );
          return null;
        })
        .then(function() {
          return InfringAPI.put('/api/agents/' + encodeURIComponent(agentId) + '/model', {
            model: requestedModel
          });
        })
        .then(function(resp) {
          if (self.currentAgent && String(self.currentAgent.id || '') === agentId) {
            self.currentAgent.model_name = (resp && resp.model) || requestedModel;
            self.currentAgent.model_provider =
              (resp && resp.provider) || self.currentAgent.model_provider || '';
            self.currentAgent.runtime_model =
              (resp && resp.runtime_model) ||
              self.currentAgent.runtime_model ||
              self.currentAgent.model_name;
            var resolvedContextWindow = Number(
              resp && resp.context_window != null ? resp.context_window : 0
            );
            if (Number.isFinite(resolvedContextWindow) && resolvedContextWindow > 0) {
              self.currentAgent.context_window = Math.round(resolvedContextWindow);
              self.contextWindow = Math.round(resolvedContextWindow);
              self.refreshContextPressure();
            }
            self.touchModelUsage(requestedModel || '');
            self.touchModelUsage(self.currentAgent.model_name || '');
            self.touchModelUsage(self.currentAgent.runtime_model || '');
            if (self.currentAgent.model_provider && self.currentAgent.model_name) {
              self.touchModelUsage(
                self.currentAgent.model_provider + '/' + self.currentAgent.model_name
              );
            }
            if (self.currentAgent.model_provider && self.currentAgent.runtime_model) {
              self.touchModelUsage(
                self.currentAgent.model_provider + '/' + self.currentAgent.runtime_model
              );
            }
            self.addModelSwitchNotice(
              previousModel,
              previousProvider,
              self.currentAgent.model_name,
              self.currentAgent.model_provider
            );
          }
          return resp || {};
        });
    },

    switchModel(model) {
      if (!this.currentAgent) return;
      if (model.id === this.currentAgent.model_name) {
        this.touchModelUsage(model.id || '');
        this.showModelSwitcher = false;
        return;
      }
      var self = this;
      this.modelSwitching = true;
      self.switchAgentModelWithGuards(model, {
        agent_id: self.currentAgent.id
      }).then(function() {
        InfringToast.success('Switched to ' + (model.display_name || model.id));
        self.showModelSwitcher = false;
      }).catch(function(e) {
        InfringToast.error('Switch failed: ' + e.message);
      }).finally(function() {
        self.modelSwitching = false;
      });
    },

    ensureFailoverModelCache: function() {
      var now = Date.now();
      if (this._modelCache && (now - Number(this._modelCacheTime || 0)) < 180000) {
        return Promise.resolve(this._modelCache);
      }
      var self = this;
      return InfringAPI.get('/api/models')
        .then(function(data) {
          var models = Array.isArray(data && data.models) ? data.models : [];
          var available = models.filter(function(m) { return !!(m && m.available); });
          self._modelCache = available;
          self._modelCacheTime = Date.now();
          self.modelPickerList = available;
          return available;
        })
        .catch(function() {
          return Array.isArray(self._modelCache) ? self._modelCache : [];
        });
    },

    normalizeFailoverCandidateId: function(entry) {
      if (!entry) return '';
      if (typeof entry === 'string') return String(entry || '').trim();
      if (typeof entry !== 'object') return '';
      var model = String(entry.id || entry.model || entry.model_name || '').trim();
      var provider = String(entry.provider || entry.model_provider || '').trim();
      if (!model) return '';
      if (provider && model.indexOf('/') < 0) return provider + '/' + model;
      return model;
    },

    modelIdVariantSet: function(values) {
      var set = {};
      var add = function(value) {
        var raw = String(value || '').trim();
        if (!raw) return;
        var lower = raw.toLowerCase();
        set[lower] = true;
        if (raw.indexOf('/') >= 0) {
          var tail = String(raw.split('/').slice(-1)[0] || '').toLowerCase();
          if (tail) set[tail] = true;
        }
      };
      if (Array.isArray(values)) {
        for (var i = 0; i < values.length; i++) add(values[i]);
      } else {
        add(values);
      }
      return set;
    },

    extractRecoverableBackendFailure: function(text) {
      var raw = String(text || '').trim();
      if (!raw) return null;
      var lower = raw.toLowerCase();
      var markers = [
        "couldn't reach a chat model backend",
        'could not reach a chat model backend',
        'hosted_model_provider_sync_failed',
        'provider-sync',
        'switch-provider',
        'lane_timeout_1500ms',
        'start ollama',
        'configure app-plane',
        'model backend unavailable',
        'no chat model backend',
        'app_plane_chat_ui'
      ];
      var matched = false;
      for (var i = 0; i < markers.length; i++) {
        if (lower.indexOf(markers[i]) >= 0) {
          matched = true;
          break;
        }
      }
      if (!matched) return null;
      var summary = raw.replace(/\s+/g, ' ').trim();
      if (summary.length > 220) summary = summary.slice(0, 217) + '...';
      return { raw: raw, summary: summary };
    },

    collectFailoverModelCandidates: async function() {
      var self = this;
      var activeSet = this.modelIdVariantSet(this.activeModelCandidateIds());
      var out = [];
      var seen = {};
      var push = function(id) {
        var modelId = String(id || '').trim();
        if (!modelId || modelId.toLowerCase() === 'auto') return;
        var variants = self.modelIdVariantSet(modelId);
        var keys = Object.keys(variants);
        for (var i = 0; i < keys.length; i++) {
          if (activeSet[keys[i]]) return;
        }
        var normalized = modelId.toLowerCase();
        if (seen[normalized]) return;
        seen[normalized] = true;
        out.push(modelId);
      };

      var agent = this.currentAgent || {};
      var fallbacks = Array.isArray(agent.fallback_models)
        ? agent.fallback_models
        : (this.agentDrawer && Array.isArray(this.agentDrawer._fallbacks) ? this.agentDrawer._fallbacks : []);
      for (var f = 0; f < fallbacks.length; f++) {
        push(this.normalizeFailoverCandidateId(fallbacks[f]));
      }

      var models = await this.ensureFailoverModelCache();
      var sorted = (Array.isArray(models) ? models.slice() : []).filter(function(row) {
        return !!(row && row.id);
      });
      sorted.sort(function(a, b) {
        var aId = String((a && a.id) || '').trim();
        var bId = String((b && b.id) || '').trim();
        var aUsage = self.modelUsageTs(aId);
        var bUsage = self.modelUsageTs(bId);
        if (bUsage !== aUsage) return bUsage - aUsage;
        var aProvider = String((a && a.provider) || '').toLowerCase();
        var bProvider = String((b && b.provider) || '').toLowerCase();
        if (aProvider !== bProvider) return aProvider.localeCompare(bProvider);
        return aId.toLowerCase().localeCompare(bId.toLowerCase());
      });
      for (var m = 0; m < sorted.length; m++) {
        push(this.normalizeFailoverCandidateId(sorted[m]));
      }
      return out;
    },

    attemptAutomaticFailoverRecovery: async function(source, rawFailure, options) {
      var failure = this.extractRecoverableBackendFailure(rawFailure);
      if (!failure) return false;
      if (this._inflightFailoverInProgress) return false;
      if (!this.currentAgent || !this.currentAgent.id) return false;
      var agentId = String(this.currentAgent.id || '').trim();
      if (!agentId) return false;
      var payload = this._inflightPayload;
      if (!payload || String(payload.agent_id || '') !== agentId) return false;
      if (payload.failover_attempted) return false;

      var opts = options && typeof options === 'object' ? options : {};
      this._inflightFailoverInProgress = true;
      payload.failover_attempted = true;
      payload.failover_reason = failure.summary;
      payload.failover_source = String(source || 'runtime');

      try {
        var candidates = await this.collectFailoverModelCandidates();
        if (!candidates.length) return false;
        var targetModel = String(candidates[0] || '').trim();
        if (!targetModel) return false;

        if (opts.remove_last_agent_failure) {
          var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (last && last.role === 'agent') {
            var lastText = String(last.text || '').trim();
            if (this.extractRecoverableBackendFailure(lastText)) {
              this.messages.pop();
            }
          }
        }

        this.messages.push({
          id: ++msgId,
          role: 'system',
          text:
            'Model backend failed (' +
            failure.summary +
            '). Switching to ' +
            targetModel +
            ' and retrying the last request automatically.',
          meta: '',
          tools: [],
          system_origin: 'model:auto-recover',
          ts: Date.now()
        });
        this.scrollToBottom();
        this.scheduleConversationPersist();

        var previousModel = String(
          (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || 'unknown'
        ).trim() || 'unknown';
        var previousProvider = String(
          (this.currentAgent && this.currentAgent.model_provider) || ''
        ).trim();
        await this.switchAgentModelWithGuards({ id: targetModel }, {
          agent_id: agentId,
          previous_model: previousModel,
          previous_provider: previousProvider
        });

        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._clearTypingTimeout();
        this._clearPendingWsRequest(agentId);
        this.setAgentLiveActivity(agentId, 'idle');
        await this._sendPayload(
          payload.final_text,
          Array.isArray(payload.uploaded_files) ? payload.uploaded_files : [],
          Array.isArray(payload.msg_images) ? payload.msg_images : [],
          { retry_from_failover: true }
        );
        return true;
      } catch (error) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text:
            'Automatic model recovery failed: ' +
            String(error && error.message ? error.message : error),
          meta: '',
          tools: [],
          system_origin: 'model:auto-recover:error',
          ts: Date.now()
        });
        this.scheduleConversationPersist();
        return false;
      } finally {
        this._inflightFailoverInProgress = false;
      }
    },

    // Fetch dynamic slash commands from server
    fetchCommands: function() {
      var self = this;
      InfringAPI.get('/api/commands').then(function(data) {
        if (data.commands && data.commands.length) {
          // Build a set of known cmds to avoid duplicates
          var existing = {};
          self.slashCommands.forEach(function(c) { existing[c.cmd] = true; });
          data.commands.forEach(function(c) {
            if (!existing[c.cmd]) {
              self.slashCommands.push({ cmd: c.cmd, desc: c.desc || '', source: c.source || 'server' });
              existing[c.cmd] = true;
            }
          });
        }
      }).catch(function() { /* silent — use hardcoded list */ });
    },

    get filteredSlashCommands() {
      if (!this.slashFilter) return this.slashCommands;
      var f = this.slashFilter;
      return this.slashCommands.filter(function(c) {
        return c.cmd.toLowerCase().indexOf(f) !== -1 || c.desc.toLowerCase().indexOf(f) !== -1;
      });
    },

    // Clear any stuck typing indicator after 120s
    _resetTypingTimeout: function() {
      var self = this;
      if (self._typingTimeout) clearTimeout(self._typingTimeout);
      self._typingTimeout = setTimeout(function() {
        // Auto-clear stuck typing indicators
        var timeoutEnvelope = self.collectStreamedAssistantEnvelope();
        var timeoutThought = String(timeoutEnvelope.thought || '').trim();
        var timeoutTools = timeoutEnvelope.tools || [];
        var timeoutText = self.sanitizeToolText(String(timeoutEnvelope.text || '').trim());
        self._clearStreamingTypewriters();
        if (timeoutThought) {
          timeoutTools.unshift(self.makeThoughtToolCard(timeoutThought, Math.max(0, Date.now() - Number(self._responseStartedAt || Date.now()))));
        }
        self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; });
        if (!timeoutText) {
          timeoutText = self.defaultAssistantFallback(timeoutThought, timeoutTools);
        }
        if (timeoutText) {
          self.messages.push({
            id: ++msgId,
            role: 'agent',
            text: timeoutText,
            meta: 'transport timeout',
            tools: timeoutTools,
            ts: Date.now(),
            _auto_fallback: true
          });
        }
        self.sending = false;
        self._responseStartedAt = 0;
        self.tokenCount = 0;
        self._inflightPayload = null;
        self._clearPendingWsRequest();
        self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', 'idle');
        self.scheduleConversationPersist();
      }, 120000);
    },

    _clearTypingTimeout: function() {
      if (this._typingTimeout) {
        clearTimeout(this._typingTimeout);
        this._typingTimeout = null;
      }
    },

    _clearMessageTypewriter: function(message) {
      if (!message || typeof message !== 'object') return;
      if (message._typewriterTimer) {
        clearTimeout(message._typewriterTimer);
        message._typewriterTimer = null;
      }
      message._typewriterRunning = false;
    },

    _clearStreamingTypewriters: function() {
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        this._clearMessageTypewriter(rows[i]);
      }
    },

    _queueStreamTypingRender: function(message, nextText) {
      if (!message || typeof message !== 'object') return;
      var targetText = String(nextText || '');
      message._streamTargetText = targetText;
      if (message._typewriterRunning) return;
      var self = this;
      message._typewriterRunning = true;

      var step = function() {
        if (!message || !message.streaming) {
          self._clearMessageTypewriter(message);
          return;
        }
        var target = String(message._streamTargetText || '');
        var current = String(message.text || '');
        if (target === current) {
          self._clearMessageTypewriter(message);
          return;
        }
        // If sanitization trims or rewrites content, snap to the newest safe value.
        if (target.length < current.length || target.indexOf(current) !== 0) {
          message.text = target;
          self._clearMessageTypewriter(message);
          self.scrollToBottom();
          return;
        }
        var remaining = target.length - current.length;
        var take = Math.max(1, Math.min(8, Math.ceil(remaining / 4)));
        message.text = target.slice(0, current.length + take);
        self.scrollToBottom();
        if (message.text.length < target.length) {
          message._typewriterTimer = setTimeout(step, 14);
          return;
        }
        self._clearMessageTypewriter(message);
      };

      step();
    },

    _reconcileSendingState: function() {
      if (!this.sending) return false;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      var hasVisiblePending = false;
      var touchedPendingRows = false;
      var now = Date.now();
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row) continue;
        if (row.thinking || row.streaming || (row.terminal && row.thinking)) {
          var activityAt = Number(row._stream_updated_at || row.ts || 0);
          var ageMs = activityAt > 0 ? Math.max(0, now - activityAt) : 0;
          // Keep pending rows pending while transport recovers; do not emit
          // premature fallback assistant messages for long-running thoughts.
          hasVisiblePending = true;
          if (row.thinking && !String(row.text || '').trim() && ageMs >= 12000) {
            row.text = 'Thinking...';
            touchedPendingRows = true;
          }
        }
      }
      if (touchedPendingRows) {
        this.scheduleConversationPersist();
      }
      var pending = this._pendingWsRequest && this._pendingWsRequest.agent_id ? this._pendingWsRequest : null;
      var hasPendingWs = !!pending;
      if (pending) {
        var pendingAgentId = String(pending.agent_id || '');
        var currentAgentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
        var pendingAgeMs = Math.max(0, now - Number(pending.started_at || now));
        if (currentAgentId && pendingAgentId && pendingAgentId !== currentAgentId) {
          hasPendingWs = false;
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('cross_agent_pending');
          }
        } else if (pendingAgeMs >= 12000) {
          if (!this._pendingWsRecovering) {
            this._recoverPendingWsRequest('stale_pending');
          }
          if (pendingAgeMs >= 30000) {
            this._clearPendingWsRequest();
            hasPendingWs = false;
          }
        }
      }
      if (hasVisiblePending || hasPendingWs) return false;
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._clearTypingTimeout();
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '', 'idle');
      return true;
    },

    _setPendingWsRequest: function(agentId, messageText) {
      var id = String(agentId || '').trim();
      if (!id) return;
      this._pendingWsRequest = {
        agent_id: id,
        message_text: String(messageText || '').trim(),
        started_at: Date.now(),
      };
      this._pendingWsRecovering = false;
    },

    _clearPendingWsRequest: function(agentId) {
      if (!this._pendingWsRequest) return;
      if (!agentId) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
        return;
      }
      var current = String(this._pendingWsRequest.agent_id || '').trim();
      if (current && current === String(agentId)) {
        this._pendingWsRequest = null;
        this._pendingWsRecovering = false;
      }
    },

    _markAgentPreviewUnread: function(agentId, unread) {
      var id = String(agentId || '').trim();
      if (!id) return;
      try {
        var store = Alpine.store('app');
        if (!store) return;
        if (typeof store.markAgentPreviewUnread === 'function') {
          store.markAgentPreviewUnread(id, unread !== false);
        } else if (store.agentChatPreviews && store.agentChatPreviews[id]) {
          store.agentChatPreviews[id].unread_response = unread !== false;
        }
      } catch(_) {}
    },

    _recoverPendingWsRequest: async function(reason) {
      if (this._pendingWsRecovering) return;
      var pending = this._pendingWsRequest;
      if (!pending || !pending.agent_id) return;
      this._pendingWsRecovering = true;
      var agentId = String(pending.agent_id);
      var startedAt = Number(pending.started_at || Date.now());
      var recoverStartedAt = Date.now();
      var maxRecoverMs = 60000;
      var resolved = false;
      for (var attempt = 0; attempt < 120; attempt++) {
        if (!this._pendingWsRequest || String(this._pendingWsRequest.agent_id || '') !== agentId) {
          break;
        }
        if ((Date.now() - recoverStartedAt) > maxRecoverMs) {
          break;
        }
        try {
          var sessionData = await InfringAPI.get('/api/agents/' + encodeURIComponent(agentId) + '/session');
          var normalized = this.normalizeSessionMessages(sessionData);
          var hasFreshAgentReply = normalized.some(function(msg) {
            var role = String(msg && msg.role ? msg.role : '').toLowerCase();
            var ts = Number(msg && msg.ts ? msg.ts : 0);
            var text = String(msg && msg.text ? msg.text : '').trim();
            return role === 'agent' && text && ts >= startedAt;
          });
          if (!hasFreshAgentReply) {
            await new Promise(function(resolve) { setTimeout(resolve, 650); });
            continue;
          }
          if (!this.conversationCache) this.conversationCache = {};
          this.conversationCache[String(agentId)] = {
            saved_at: Date.now(),
            token_count: Number(this.contextApproxTokens || 0),
            messages: JSON.parse(JSON.stringify(normalized || [])),
          };
          try {
            var appStore = Alpine.store('app');
            if (appStore && typeof appStore.saveAgentChatPreview === 'function') {
              appStore.saveAgentChatPreview(agentId, this.conversationCache[String(agentId)].messages);
            }
          } catch(_) {}
          var isActive = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
          if (isActive) {
            this.messages = this.mergeModelNoticesForAgent(agentId, JSON.parse(JSON.stringify(normalized || [])));
            this.scrollToBottom();
          } else {
            this._markAgentPreviewUnread(agentId, true);
          }
          this.persistConversationCache();
          resolved = true;
          break;
        } catch(_) {
          await new Promise(function(resolve) { setTimeout(resolve, 500); });
        }
      }

      var stillActiveAgent = !!(this.currentAgent && String(this.currentAgent.id || '') === agentId);
      if (!resolved && stillActiveAgent) {
        this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: 'Connection dropped before the agent reply was delivered. Please retry.',
          meta: '',
          tools: [],
          system_origin: 'transport:recovery',
          ts: Date.now()
        });
        this.scrollToBottom();
      }
      if (!resolved && !stillActiveAgent) {
        this._pendingWsRecovering = false;
        return;
      }
      this.setAgentLiveActivity(agentId, 'idle');
      if (stillActiveAgent) {
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('msg-input'); if (el) el.focus();
          self._processQueue();
        });
      }
      this._clearPendingWsRequest(agentId);
    },

    async executeSlashCommand(cmd, cmdArgs) {
      this.showSlashMenu = false;
      this.inputText = '';
      var self = this;
      cmdArgs = cmdArgs || '';
      switch (cmd) {
        case '/help':
          self.messages.push({ id: ++msgId, role: 'system', text: self.slashCommands.map(function(c) { return '`' + c.cmd + '` — ' + c.desc; }).join('\n'), meta: '', tools: [], system_origin: 'slash:help' });
          self.scrollToBottom();
          break;
        case '/agents':
          location.hash = 'agents';
          break;
        case '/new':
          if (self.currentAgent) {
            InfringAPI.post('/api/agents/' + self.currentAgent.id + '/session/reset', {}).then(function() {
              self.messages = [];
              InfringToast.success('Session reset');
            }).catch(function(e) { InfringToast.error('Reset failed: ' + e.message); });
          }
          break;
        case '/compact':
          if (self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Compacting session...', meta: '', tools: [], system_origin: 'slash:compact' });
            InfringAPI.post('/api/agents/' + self.currentAgent.id + '/session/compact', {}).then(function(res) {
              self.messages.push({ id: ++msgId, role: 'system', text: res.message || 'Compaction complete', meta: '', tools: [], system_origin: 'slash:compact' });
              self.scrollToBottom();
            }).catch(function(e) { InfringToast.error('Compaction failed: ' + e.message); });
          }
          break;
        case '/stop':
          self.stopAgent();
          break;
        case '/usage':
          if (self.currentAgent) {
            var approxTokens = self.messages.reduce(function(sum, m) { return sum + Math.round((m.text || '').length / 4); }, 0);
            self.messages.push({ id: ++msgId, role: 'system', text: '**Session Usage**\n- Messages: ' + self.messages.length + '\n- Approx tokens: ~' + approxTokens, meta: '', tools: [], system_origin: 'slash:usage' });
            self.scrollToBottom();
          }
          break;
        case '/think':
          if (cmdArgs === 'on') {
            self.thinkingMode = 'on';
          } else if (cmdArgs === 'off') {
            self.thinkingMode = 'off';
          } else if (cmdArgs === 'stream') {
            self.thinkingMode = 'stream';
          } else {
            // Cycle: off -> on -> stream -> off
            if (self.thinkingMode === 'off') self.thinkingMode = 'on';
            else if (self.thinkingMode === 'on') self.thinkingMode = 'stream';
            else self.thinkingMode = 'off';
          }
          var modeLabel = self.thinkingMode === 'stream' ? 'enabled (streaming reasoning)' : (self.thinkingMode === 'on' ? 'enabled' : 'disabled');
          self.messages.push({ id: ++msgId, role: 'system', text: 'Extended thinking **' + modeLabel + '**. ' +
            (self.thinkingMode === 'stream' ? 'Reasoning tokens will appear in a collapsible panel.' :
             self.thinkingMode === 'on' ? 'The agent will show its reasoning when supported by the model.' :
             'Normal response mode.'), meta: '', tools: [], system_origin: 'slash:think' });
          self.scrollToBottom();
          break;
        case '/context':
          // Visual-only update for context ring; no chat message noise.
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'context', args: '', silent: true });
          } else {
            self.recomputeContextEstimate();
            self.setContextWindowFromCurrentAgent();
          }
          break;
        case '/verbose':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'verbose', args: cmdArgs });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected. Connect to an agent first.', meta: '', tools: [], system_origin: 'slash:verbose' });
            self.scrollToBottom();
          }
          break;
        case '/queue':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'queue', args: '' });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected.', meta: '', tools: [], system_origin: 'slash:queue' });
            self.scrollToBottom();
          }
          break;
        case '/status':
          InfringAPI.get('/api/status').then(function(s) {
            self.messages.push({ id: ++msgId, role: 'system', text: '**System Status**\n- Agents: ' + (s.agent_count || 0) + '\n- Uptime: ' + (s.uptime_seconds || 0) + 's\n- Version: ' + (s.version || '?'), meta: '', tools: [], system_origin: 'slash:status' });
            self.scrollToBottom();
          }).catch(function() {});
          break;
        case '/model':
          if (self.currentAgent) {
            if (cmdArgs) {
              self.switchAgentModelWithGuards({ id: cmdArgs }, {
                agent_id: self.currentAgent.id
              }).catch(function(e) {
                InfringToast.error('Model switch failed: ' + e.message);
              });
            } else {
              self.messages.push({ id: ++msgId, role: 'system', text: '**Current Model**\n- Provider: `' + (self.currentAgent.model_provider || '?') + '`\n- Model: `' + (self.currentAgent.model_name || '?') + '`', meta: '', tools: [], system_origin: 'slash:model' });
              self.scrollToBottom();
            }
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:model' });
            self.scrollToBottom();
          }
          break;
        case '/file':
          if (!self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:file' });
            self.scrollToBottom();
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Usage: `/file <path>`', meta: '', tools: [], system_origin: 'slash:file' });
            self.scrollToBottom();
            break;
          }
          try {
            var fileRes = await InfringAPI.post('/api/agents/' + self.currentAgent.id + '/file/read', {
              path: String(cmdArgs || '').trim()
            });
            var fileMeta = fileRes && fileRes.file ? fileRes.file : null;
            if (!fileMeta || !fileMeta.ok) {
              self.messages.push({
                id: ++msgId,
                role: 'system',
                text: 'Error: failed to read file output.',
                meta: '',
                tools: [],
                system_origin: 'slash:file',
                ts: Date.now()
              });
            } else {
              var bytes = Number(fileMeta.bytes || 0);
              var fileMetaText = (bytes > 0 ? (bytes + ' bytes') : '');
              if (fileMeta.truncated) {
                var maxBytes = Number(fileMeta.max_bytes || 0);
                fileMetaText += (fileMetaText ? ' | ' : '') + 'truncated to ' + (maxBytes > 0 ? maxBytes : 'limit') + ' bytes';
              }
              self.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: fileMetaText,
                tools: [],
                ts: Date.now(),
                file_output: {
                  path: String(fileMeta.path || cmdArgs || ''),
                  content: String(fileMeta.content || ''),
                  truncated: !!fileMeta.truncated,
                  bytes: bytes
                }
              });
            }
            self.scrollToBottom();
          } catch (e) {
            self.messages.push({
              id: ++msgId,
              role: 'system',
              text: 'Error: ' + (e && e.message ? e.message : 'file read failed'),
              meta: '',
              tools: [],
              system_origin: 'slash:file',
              ts: Date.now()
            });
            self.scrollToBottom();
          }
          break;
        case '/folder':
          if (!self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:folder' });
            self.scrollToBottom();
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Usage: `/folder <path>`', meta: '', tools: [], system_origin: 'slash:folder' });
            self.scrollToBottom();
            break;
          }
          try {
            var folderRes = await InfringAPI.post('/api/agents/' + self.currentAgent.id + '/folder/export', {
              path: String(cmdArgs || '').trim()
            });
            var folderMeta = folderRes && folderRes.folder ? folderRes.folder : null;
            var archiveMeta = folderRes && folderRes.archive ? folderRes.archive : null;
            if (!folderMeta || !folderMeta.ok) {
              self.messages.push({
                id: ++msgId,
                role: 'system',
                text: 'Error: failed to export folder output.',
                meta: '',
                tools: [],
                system_origin: 'slash:folder',
                ts: Date.now()
              });
            } else {
              var entryCount = Number(folderMeta.entries || 0);
              var folderMetaText = (entryCount > 0 ? (entryCount + ' entries') : '');
              if (folderMeta.truncated) folderMetaText += (folderMetaText ? ' | ' : '') + 'tree truncated';
              if (archiveMeta && archiveMeta.file_name) {
                folderMetaText += (folderMetaText ? ' | ' : '') + archiveMeta.file_name;
              }
              self.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: folderMetaText,
                tools: [],
                ts: Date.now(),
                folder_output: {
                  path: String(folderMeta.path || cmdArgs || ''),
                  tree: String(folderMeta.tree || ''),
                  entries: entryCount,
                  truncated: !!folderMeta.truncated,
                  download_url: archiveMeta && archiveMeta.download_url ? String(archiveMeta.download_url) : '',
                  archive_name: archiveMeta && archiveMeta.file_name ? String(archiveMeta.file_name) : '',
                  archive_bytes: Number(archiveMeta && archiveMeta.bytes ? archiveMeta.bytes : 0)
                }
              });
            }
            self.scrollToBottom();
          } catch (e2) {
            self.messages.push({
              id: ++msgId,
              role: 'system',
              text: 'Error: ' + (e2 && e2.message ? e2.message : 'folder export failed'),
              meta: '',
              tools: [],
              system_origin: 'slash:folder',
              ts: Date.now()
            });
            self.scrollToBottom();
          }
          break;
        case '/clear':
          self.messages = [];
          break;
        case '/exit':
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          window.dispatchEvent(new Event('close-chat'));
          break;
        case '/budget':
          InfringAPI.get('/api/budget').then(function(b) {
            var fmt = function(v) { return v > 0 ? '$' + v.toFixed(2) : 'unlimited'; };
            self.messages.push({ id: ++msgId, role: 'system', text: '**Budget Status**\n' +
              '- Hourly: $' + (b.hourly_spend||0).toFixed(4) + ' / ' + fmt(b.hourly_limit) + '\n' +
              '- Daily: $' + (b.daily_spend||0).toFixed(4) + ' / ' + fmt(b.daily_limit) + '\n' +
              '- Monthly: $' + (b.monthly_spend||0).toFixed(4) + ' / ' + fmt(b.monthly_limit), meta: '', tools: [], system_origin: 'slash:budget' });
            self.scrollToBottom();
          }).catch(function() {});
          break;
        case '/peers':
          InfringAPI.get('/api/network/status').then(function(ns) {
            self.messages.push({ id: ++msgId, role: 'system', text: '**OFP Network**\n' +
              '- Status: ' + (ns.enabled ? 'Enabled' : 'Disabled') + '\n' +
              '- Connected peers: ' + (ns.connected_peers||0) + ' / ' + (ns.total_peers||0), meta: '', tools: [], system_origin: 'slash:peers' });
            self.scrollToBottom();
          }).catch(function() {});
          break;
        case '/a2a':
          InfringAPI.get('/api/a2a/agents').then(function(res) {
            var agents = res.agents || [];
            if (!agents.length) {
              self.messages.push({ id: ++msgId, role: 'system', text: 'No external A2A agents discovered.', meta: '', tools: [], system_origin: 'slash:a2a' });
            } else {
              var lines = agents.map(function(a) { return '- **' + a.name + '** — ' + a.url; });
              self.messages.push({ id: ++msgId, role: 'system', text: '**A2A Agents (' + agents.length + ')**\n' + lines.join('\n'), meta: '', tools: [], system_origin: 'slash:a2a' });
            }
            self.scrollToBottom();
          }).catch(function() {});
          break;
      }
      this.scheduleConversationPersist();
    },

    selectAgent(agent) {
      var resolved = this.resolveAgent(agent);
      if (!resolved) return;
      this.closeGitTreeMenu();
      this._markAgentPreviewUnread(resolved.id, false);
      var store = Alpine.store('app');
      var pendingFreshId = store && store.pendingFreshAgentId ? String(store.pendingFreshAgentId) : '';
      var forceFreshSession = pendingFreshId && String(resolved.id) === pendingFreshId;
      this.clearHoveredMessageHard();
      this.activeMapPreviewDomId = '';
      this.activeMapPreviewDayKey = '';
      if (this.currentAgent && this.currentAgent.id && this.currentAgent.id !== resolved.id) {
        var switchingFrom = String(this.currentAgent.id || '');
        if (
          this.sending &&
          this._pendingWsRequest &&
          String(this._pendingWsRequest.agent_id || '') === switchingFrom
        ) {
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.sending = false;
          this._responseStartedAt = 0;
          this.setAgentLiveActivity(switchingFrom, 'working');
          this._recoverPendingWsRequest('agent_switch');
        }
        this.cacheAgentConversation(this.currentAgent.id);
      }
      if (this.currentAgent && this.currentAgent.id === resolved.id) {
        this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
        this.touchModelUsage(resolved.model_name || resolved.runtime_model || '');
        if (forceFreshSession) {
          this.messages = [];
          this.showFreshArchetypeTiles = false;
          this.freshInitRevealMenu = false;
          this.freshInitTemplateDef = null;
          this.freshInitTemplateName = '';
          this.freshInitLaunching = false;
          this.freshInitName = String(resolved.name || resolved.id || '').trim() || String(resolved.id || '');
          this.freshInitEmoji = String(
            (resolved.identity && resolved.identity.emoji) ||
            (this.agentDrawer && this.agentDrawer.identity && this.agentDrawer.identity.emoji) ||
            this.defaultFreshEmojiForAgent(resolved)
          ).trim() || this.defaultFreshEmojiForAgent(resolved);
          if (this.conversationCache) {
            delete this.conversationCache[String(resolved.id)];
            this.persistConversationCache();
          }
          InfringAPI.post('/api/agents/' + resolved.id + '/session/reset', {}).catch(function() {});
          this.connectWs(resolved.id);
          this.loadSessions(resolved.id);
          this.requestContextTelemetry(true);
          this.clearPromptSuggestions();
          this.startFreshInitSequence(resolved);
          var selfFreshCurrent = this;
          this.$nextTick(function() {
            selfFreshCurrent.scrollToBottomImmediate();
            selfFreshCurrent.stabilizeBottomScroll();
            selfFreshCurrent.installChatMapWheelLock();
            selfFreshCurrent.scheduleMessageRenderWindowUpdate();
          });
        } else {
          this.loadSession(resolved.id, true);
        }
        return;
      }
      this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
      if (store) this.setStoreActiveAgentId(resolved.id || null);
      this.touchModelUsage(resolved.model_name || resolved.runtime_model || '');
      this.setContextWindowFromCurrentAgent();
      if (forceFreshSession && this.conversationCache) {
        delete this.conversationCache[String(resolved.id)];
        this.persistConversationCache();
        InfringAPI.post('/api/agents/' + resolved.id + '/session/reset', {}).catch(function() {});
      }
      var restored = forceFreshSession ? false : this.restoreAgentConversation(resolved.id);
      if (!restored) this.messages = [];
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      if (forceFreshSession) {
        this.freshInitTemplateDef = null;
        this.freshInitTemplateName = '';
        this.freshInitLaunching = false;
        this.freshInitName = String(resolved.name || resolved.id || '').trim() || String(resolved.id || '');
        this.freshInitEmoji = String(
          (resolved.identity && resolved.identity.emoji) ||
          (this.agentDrawer && this.agentDrawer.identity && this.agentDrawer.identity.emoji) ||
          this.defaultFreshEmojiForAgent(resolved)
        ).trim() || this.defaultFreshEmojiForAgent(resolved);
        this.clearPromptSuggestions();
        this.startFreshInitSequence(resolved);
      } else {
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this._freshInitThreadShownFor = '';
      }
      this._reconcileSendingState();
      this.connectWs(resolved.id);
      // Show welcome tips on first use
      if (!restored && !this.showFreshArchetypeTiles && !localStorage.getItem('of-chat-tips-seen')) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: '**Welcome to Infring Chat!**\n\n' +
            '- Type `/` to see available commands\n' +
            '- `/help` shows all commands\n' +
            '- `/think on` enables extended reasoning\n' +
            '- `/context` shows context window usage\n' +
            '- `/verbose off` hides tool details\n' +
            '- `Ctrl+Shift+F` toggles focus mode\n' +
            '- `Ctrl+F` opens file picker\n' +
            '- Drag & drop files to attach them\n' +
            '- `Ctrl+/` opens the command palette',
          meta: '',
          tools: [],
          system_origin: 'chat:welcome'
        });
        localStorage.setItem('of-chat-tips-seen', 'true');
      }
      if (!forceFreshSession) {
        this.loadSession(resolved.id, restored);
      }
      this.loadSessions(resolved.id);
      this.requestContextTelemetry(true);
      if (!forceFreshSession) {
        this.refreshPromptSuggestions(false);
      }
      if (this.showAgentDrawer) {
        this.openAgentDrawer();
      }
      // Focus input after agent selection
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) el.focus();
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
      });
    },

    shouldRenderMessage(msg, idx) {
      if (!msg || msg.is_notice) return true;
      if (!this.currentAgent) return true;
      var id = this.messageDomId(msg, idx);
      if (this.messageHydration && this.messageHydration[id]) return true;
      // Always hydrate newest messages for streaming responsiveness.
      if (idx >= (this.messages.length - 24)) return true;
      return false;
    },

    forceMessageRender(msg, idx, ttlMs) {
      if (!msg) return;
      var id = this.messageDomId(msg, idx);
      if (!id) return;
      var ttl = Number(ttlMs || 0);
      var until = Date.now() + (ttl > 0 ? ttl : 6000);
      if (!this._forcedHydrateById || typeof this._forcedHydrateById !== 'object') {
        this._forcedHydrateById = {};
      }
      this._forcedHydrateById[id] = until;
      this.scheduleMessageRenderWindowUpdate();
    },

    scheduleMessageRenderWindowUpdate(container) {
      var self = this;
      if (this._renderWindowRaf && typeof cancelAnimationFrame === 'function') {
        cancelAnimationFrame(this._renderWindowRaf);
        this._renderWindowRaf = 0;
      }
      var run = function() {
        self._renderWindowRaf = 0;
        self.updateMessageRenderWindow(container);
      };
      if (typeof requestAnimationFrame === 'function') {
        this._renderWindowRaf = requestAnimationFrame(run);
      } else {
        setTimeout(run, 0);
      }
    },

    updateMessageRenderWindow(container) {
      var el = this.resolveMessagesScroller(container || null);
      if (!el || !this.currentAgent) return;
      var viewportHeight = Number(el.clientHeight || 0);
      if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) return;
      var minY = Math.max(0, el.scrollTop - viewportHeight);
      var maxY = el.scrollTop + (viewportHeight * 2);
      var next = {};
      var blocks = el.querySelectorAll('.chat-message-block[data-msg-idx]');
      for (var i = 0; i < blocks.length; i++) {
        var block = blocks[i];
        if (!block || !block.id) continue;
        var top = Number(block.offsetTop || 0);
        var height = Number(block.offsetHeight || 0);
        if (!Number.isFinite(height) || height <= 0) height = 48;
        var bottom = top + height;
        if (bottom >= minY && top <= maxY) next[block.id] = true;
      }
      var now = Date.now();
      var forced = this._forcedHydrateById || {};
      Object.keys(forced).forEach(function(id) {
        var until = Number(forced[id] || 0);
        if (until > now) {
          next[id] = true;
        } else {
          delete forced[id];
        }
      });
      if (this.selectedMessageDomId) next[this.selectedMessageDomId] = true;
      if (this.hoveredMessageDomId) next[this.hoveredMessageDomId] = true;
      this.messageHydration = next;
    },

    isFreshInitTemplateSelected(templateDef) {
      if (!templateDef) return false;
      var key = String(templateDef.name || '').trim();
      return !!key && key === String(this.freshInitTemplateName || '').trim();
    },

    async applyChatArchetypeTemplate(templateDef) {
      if (!templateDef) return;
      this.freshInitTemplateDef = templateDef;
      this.freshInitTemplateName = String(templateDef.name || '').trim();
    },

    async launchFreshAgentInitialization() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.freshInitLaunching) return;
      if (!this.freshInitTemplateDef) {
        InfringToast.info('Select an archetype first.');
        return;
      }
      var agentId = this.currentAgent.id;
      var templateDef = this.freshInitTemplateDef;
      var provider = String(templateDef.provider || '').trim();
      var model = String(templateDef.model || '').trim();
      var agentName = String(this.freshInitName || '').trim() || String(this.currentAgent.name || this.currentAgent.id || '').trim() || String(agentId);
      var agentEmoji = String(this.freshInitEmoji || '').trim() || this.defaultFreshEmojiForAgent(agentId);
      this.freshInitLaunching = true;
      try {
        if (provider && model) {
          await InfringAPI.put('/api/agents/' + agentId + '/model', {
            model: provider + '/' + model
          });
        }
        await InfringAPI.patch('/api/agents/' + agentId + '/config', {
          name: agentName,
          identity: { emoji: agentEmoji },
          system_prompt: String(templateDef.system_prompt || '').trim(),
          archetype: String(templateDef.archetype || '').trim(),
          profile: String(templateDef.profile || '').trim()
        });
        this.addNoticeEvent({
          notice_label: 'Initialized ' + agentName + ' as ' + String(templateDef.name || 'template'),
          notice_type: 'info',
          ts: Date.now()
        });
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this.freshInitRevealMenu = false;
        this.showFreshArchetypeTiles = false;
        this.freshInitTemplateDef = null;
        this.freshInitTemplateName = '';
        this.freshInitLaunching = false;
        try {
          var store = Alpine.store('app');
          if (store) {
            store.pendingFreshAgentId = null;
            if (typeof store.refreshAgents === 'function') {
              await store.refreshAgents();
            }
          }
        } catch(_) {}
        await this.syncDrawerAgentAfterChange();
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        InfringToast.success('Launched ' + String(templateDef.name || 'agent setup'));
      } catch (e) {
        this.freshInitLaunching = false;
        InfringToast.error('Failed to initialize agent: ' + e.message);
      }
    },

    async loadSession(agentId, keepCurrent) {
      var self = this;
      var loadSeq = ++this._sessionLoadSeq;
      this.sessionLoading = true;
      try {
        var data = await InfringAPI.get('/api/agents/' + agentId + '/session');
        if (self.currentAgent && String(self.currentAgent.id || '') === String(agentId || '')) {
          self.applyAgentGitTreeState(self.currentAgent, data || {});
        }
        var normalized = self.mergeModelNoticesForAgent(agentId, self.normalizeSessionMessages(data));
        if (normalized.length) {
          self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
          self.freshInitRevealMenu = false;
          self.showFreshArchetypeTiles = false;
          if (!keepCurrent || !self.messages || !self.messages.length || normalized.length >= self.messages.length) {
            self.messages = normalized;
            self.clearHoveredMessageHard();
            self.activeMapPreviewDomId = '';
            self.activeMapPreviewDayKey = '';
            self.recomputeContextEstimate();
          }
          self.cacheAgentConversation(agentId);
          self.$nextTick(function() {
            self.scrollToBottomImmediate();
            self.stabilizeBottomScroll();
          });
        } else if (!keepCurrent) {
          self.freshInitStageToken = Number(self.freshInitStageToken || 0) + 1;
          self.freshInitRevealMenu = false;
          self.messages = [];
          self.clearHoveredMessageHard();
          self.activeMapPreviewDomId = '';
          self.activeMapPreviewDayKey = '';
          self.recomputeContextEstimate();
          self.cacheAgentConversation(agentId);
          self.$nextTick(function() {
            self.scrollToBottomImmediate();
            self.stabilizeBottomScroll();
          });
        }
      } catch(e) { /* silent */ }
      finally {
        if (self._sessionLoadSeq === loadSeq) {
          await new Promise(function(resolve) {
            self.$nextTick(function() {
              self.scrollToBottomImmediate();
              self.stabilizeBottomScroll();
              self.scheduleMessageRenderWindowUpdate();
              resolve();
            });
          });
          await self.waitForSessionRender(agentId, loadSeq);
          if (self._sessionLoadSeq === loadSeq) {
            self.sessionLoading = false;
          }
          self._reconcileSendingState();
          if (!self.showFreshArchetypeTiles) {
            self.refreshPromptSuggestions(false);
          }
        }
      }
    },

    waitForAnimationFrame() {
      return new Promise(function(resolve) {
        if (typeof requestAnimationFrame === 'function') {
          requestAnimationFrame(function() { resolve(); });
        } else {
          setTimeout(resolve, 16);
        }
      });
    },

    async waitForSessionRender(agentId, loadSeq) {
      var self = this;
      var expectedAgent = String(agentId || '');
      var hasSessionMessages = Array.isArray(this.messages) && this.messages.length > 0;
      var minFrames = hasSessionMessages ? 2 : 1;
      var maxFrames = hasSessionMessages ? 42 : 6;
      var messagesEl = null;

      // Let Alpine commit template updates before probing for rendered blocks.
      await this.waitForAnimationFrame();
      await this.waitForAnimationFrame();

      for (var frame = 0; frame < maxFrames; frame++) {
        if (self._sessionLoadSeq !== loadSeq) return;
        if (!self.currentAgent || String(self.currentAgent.id || '') !== expectedAgent) return;
        if (!messagesEl) messagesEl = self.resolveMessagesScroller();
        if (!messagesEl) {
          await self.waitForAnimationFrame();
          continue;
        }

        self.scheduleMessageRenderWindowUpdate(messagesEl);

        if (!hasSessionMessages) {
          if (frame >= minFrames) return;
          await self.waitForAnimationFrame();
          continue;
        }

        var blockCount = messagesEl.querySelectorAll('.chat-message-block').length;
        var renderedCount = messagesEl.querySelectorAll('.chat-message-block .message, .chat-message-block .message-placeholder-shell, .chat-day-anchor, .chat-day-divider').length;
        if (blockCount > 0 && renderedCount > 0 && frame >= minFrames) {
          return;
        }

        await self.waitForAnimationFrame();
      }
    },

    // Multi-session: load session list for current agent
    async loadSessions(agentId) {
      try {
        var data = await InfringAPI.get('/api/agents/' + agentId + '/sessions');
        this.sessions = data.sessions || [];
      } catch(e) { this.sessions = []; }
    },

    // Multi-session: create a new session
    async createSession() {
      if (!this.currentAgent) return;
      this.cacheCurrentConversation();
      var label = prompt('Session name (optional):');
      if (label === null) return; // cancelled
      try {
        await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/sessions', {
          label: label.trim() || undefined
        });
        await this.loadSessions(this.currentAgent.id);
        await this.loadSession(this.currentAgent.id);
        if (typeof InfringToast !== 'undefined') InfringToast.success('New session created');
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to create session');
      }
    },

    // Multi-session: switch to an existing session
    async switchSession(sessionId) {
      if (!this.currentAgent) return;
      this.cacheCurrentConversation();
      try {
        await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/sessions/' + sessionId + '/switch', {});
        await this.loadSession(this.currentAgent.id);
        await this.loadSessions(this.currentAgent.id);
        // Reconnect WebSocket for new session
        this._wsAgent = null;
        this.connectWs(this.currentAgent.id);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to switch session');
      }
    },

    connectWs(agentId) {
      if (this._wsAgent === agentId && InfringAPI.isWsConnected()) return;
      this._wsAgent = agentId;
      var self = this;

      InfringAPI.wsConnect(agentId, {
        onOpen: function() {
          Alpine.store('app').wsConnected = true;
          self.requestContextTelemetry(true);
        },
        onMessage: function(data) { self.handleWsMessage(data); },
        onClose: function() {
          Alpine.store('app').wsConnected = false;
          self._wsAgent = null;
          var pending = self._pendingWsRequest;
          if (self.sending && pending && pending.agent_id) {
            self._clearTypingTimeout();
            self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            self.sending = false;
            self._responseStartedAt = 0;
            self._recoverPendingWsRequest('ws_close');
          }
          if (self.currentAgent && self.currentAgent.id) {
            Alpine.store('app').refreshAgents().then(function() {
              var stillLive = self.resolveAgent(self.currentAgent.id);
              if (!stillLive) {
                self.handleAgentInactive(self.currentAgent.id, 'inactive');
              }
            }).catch(function() {});
          }
        },
        onError: function() {
          Alpine.store('app').wsConnected = false;
          self._wsAgent = null;
          var pending = self._pendingWsRequest;
          if (self.sending && pending && pending.agent_id) {
            self._clearTypingTimeout();
            self.messages = self.messages.filter(function(m) { return !m.thinking && !m.streaming; });
            self.sending = false;
            self._responseStartedAt = 0;
            self._recoverPendingWsRequest('ws_error');
          }
        }
      });
    },

    formatInactiveReason: function(reason) {
      var raw = String(reason || '').trim();
      if (!raw) return 'inactive';
      raw = raw.replace(/^agent_contract_/, '');
      raw = raw.replace(/^rogue_/, '');
      raw = raw.replace(/_/g, ' ').trim();
      return raw || 'inactive';
    },

    handleAgentInactive: function(agentId, reason, options) {
      var opts = options || {};
      var targetId = String(agentId || (this.currentAgent && this.currentAgent.id) || '').trim();
      var reasonLabel = this.formatInactiveReason(reason || 'inactive');
      var noticeKey = targetId + '|' + reasonLabel;
      var self = this;

      this._clearTypingTimeout();
      this._clearPendingWsRequest(targetId);
      this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this._inflightPayload = null;
      this.setAgentLiveActivity(targetId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle');

      if (!opts.silentNotice && noticeKey !== this._lastInactiveNoticeKey) {
        var noticeText = opts.noticeText || '';
        if (!noticeText) {
          noticeText = targetId
            ? ('Agent ' + targetId + ' is now inactive (' + reasonLabel + ').')
            : ('Agent is now inactive (' + reasonLabel + ').');
        }
        this.messages.push({ id: ++msgId, role: 'system', text: noticeText, meta: '', tools: [], system_origin: 'agent:inactive', ts: Date.now() });
        this._lastInactiveNoticeKey = noticeKey;
      }

      if (targetId && this._wsAgent && String(this._wsAgent) === targetId) {
        InfringAPI.wsDisconnect();
        this._wsAgent = null;
      }

      if (this.currentAgent && this.currentAgent.id && (!targetId || String(this.currentAgent.id) === targetId)) {
        this.currentAgent = null;
        this.setStoreActiveAgentId(null);
        this.showAgentDrawer = false;
      }

      this.scrollToBottom();
      this.$nextTick(function() { self._processQueue(); });

      try { Alpine.store('app').refreshAgents(); } catch(_) {}
    },

    setAgentLiveActivity(agentId, state) {
      var id = String(agentId || '').trim();
      if (!id) return;
      try {
        var store = Alpine.store('app');
        if (store && typeof store.setAgentLiveActivity === 'function') {
          store.setAgentLiveActivity(id, state);
        }
      } catch(_) {}
    },

    handleStopResponse: function(agentId, payload) {
      var result = payload && typeof payload === 'object' ? payload : {};
      var reasonRaw = String(result.reason || result.error || '').trim();
      var reason = reasonRaw || (result.contract_terminated ? 'contract_terminated' : '');
      var state = String(result.state || '').trim().toLowerCase();
      var reasonLower = reason.toLowerCase();
      var isInactive =
        !!result.archived ||
        !!result.contract_terminated ||
        state === 'inactive' ||
        state === 'archived' ||
        state === 'terminated' ||
        String(result.type || '').toLowerCase() === 'agent_archived' ||
        reasonLower.indexOf('inactive') >= 0 ||
        reasonLower.indexOf('terminated') >= 0;

      if (isInactive) {
        this.handleAgentInactive(
          agentId,
          reason || (result.contract_terminated ? 'contract_terminated' : 'inactive'),
          result.message ? { noticeText: String(result.message) } : {}
        );
        return;
      }

      this.setAgentLiveActivity(agentId || (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''), 'idle');
      this._clearTypingTimeout();
      this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
      this.messages.push({ id: ++msgId, role: 'system', text: result.message || 'Run cancelled', meta: '', tools: [], system_origin: 'agent:stop', ts: Date.now() });
      this.sending = false;
      this._responseStartedAt = 0;
      this.tokenCount = 0;
      this.scrollToBottom();
      var self = this;
      this.$nextTick(function() { self._processQueue(); });
      try { Alpine.store('app').refreshAgents(); } catch(_) {}
    },

    handleWsMessage(data) {
      switch (data.type) {
        case 'connected': break;

        case 'context_state':
          this.applyContextTelemetry(data);
          break;

        // Legacy thinking event (backward compat)
        case 'thinking':
          if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
            var thinkLabel = data.level ? 'Thinking (' + data.level + ')...' : 'Processing...';
            this.messages.push({
              id: ++msgId,
              role: 'agent',
              text: '*' + thinkLabel + '*',
              meta: '',
              thinking: true,
              streaming: true,
              tools: [],
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            });
            this.scrollToBottom();
            this._resetTypingTimeout();
          } else if (data.level) {
            var lastThink = this.messages[this.messages.length - 1];
            if (lastThink && lastThink.thinking) lastThink.text = '*Thinking (' + data.level + ')...*';
          }
          break;

        // New typing lifecycle
        case 'typing':
          if (data.state === 'start') {
            this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
            if (!this.messages.length || !this.messages[this.messages.length - 1].thinking) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '*Processing...*',
                meta: '',
                thinking: true,
                streaming: true,
                tools: [],
                agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
                agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
              });
              this.scrollToBottom();
            }
            this._resetTypingTimeout();
          } else if (data.state === 'tool') {
            this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working');
            var typingMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
            if (typingMsg && (typingMsg.thinking || typingMsg.streaming)) {
              typingMsg.text = '*Using ' + (data.tool || 'tool') + '...*';
            }
            this._resetTypingTimeout();
          } else if (data.state === 'stop') {
            this._clearTypingTimeout();
          }
          break;

        case 'phase':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working');
          // Show tool/phase progress so the user sees the agent is working
          var phaseMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (phaseMsg && (phaseMsg.thinking || phaseMsg.streaming)) {
            var phasePercent = Number(
              data && data.progress_percent != null
                ? data.progress_percent
                : (data && data.percent != null ? data.percent : NaN)
            );
            if (Number.isFinite(phasePercent)) {
              phaseMsg.progress = {
                percent: Math.max(0, Math.min(100, Math.round(phasePercent))),
                label: data && data.phase ? ('Progress · ' + String(data.phase)) : 'Progress'
              };
            }
            // Skip phases that have no user-meaningful display text — "streaming"
            // and "done" are lifecycle signals, not status to show in the chat bubble.
            if (data.phase === 'streaming' || data.phase === 'done') {
              break;
            }
            // Context warning: show prominently as a separate system message
            if (data.phase === 'context_warning') {
              var cwDetail = data.detail || 'Context limit reached.';
              this.messages.push({ id: ++msgId, role: 'system', text: cwDetail, meta: '', tools: [], system_origin: 'context:warning' });
            } else if (data.phase === 'thinking') {
              var thoughtChunk = String(data.detail || '').trim();
              if (thoughtChunk) {
                phaseMsg._thoughtText = this.appendThoughtChunk(phaseMsg._thoughtText, thoughtChunk);
                phaseMsg._reasoning = phaseMsg._thoughtText;
                phaseMsg.isHtml = true;
                phaseMsg.thoughtStreaming = true;
                phaseMsg.text = this.renderLiveThoughtHtml(phaseMsg._thoughtText);
              } else if (phaseMsg.thinking) {
                phaseMsg.text = 'Thinking...';
              }
            } else if (phaseMsg.thinking) {
              // Only update text on messages still in thinking state (not yet
              // receiving streamed content) to avoid overwriting accumulated text.
              var phaseDetail;
              if (data.phase === 'tool_use') {
                phaseDetail = 'Using ' + (data.detail || 'tool') + '...';
              } else if (data.phase === 'thinking') {
                phaseDetail = 'Thinking...';
              } else {
                phaseDetail = data.detail || 'Working...';
              }
              phaseMsg.text = phaseDetail;
            }
          }
          this.scrollToBottom();
          break;

        case 'text_delta':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
          var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (last && last.streaming) {
            if (last.thinking) { last.text = ''; last.thinking = false; }
            // If we already detected a text-based tool call, skip further text
            if (last._toolTextDetected) break;
            var deltaText = String(data.content || '');
            last._streamRawText = String(last._streamRawText || '') + deltaText;
            last._stream_updated_at = Date.now();
            var streamingSplit = this.extractThinkingLeak(last._streamRawText);
            var visibleText = this.stripModelPrefix(streamingSplit.content || '');
            last._cleanText = visibleText;
            last._thoughtText = streamingSplit.thought || '';
            if (streamingSplit.thought && !visibleText.trim()) {
              this._clearMessageTypewriter(last);
              last.isHtml = true;
              last.thoughtStreaming = true;
              last.text = this.renderLiveThoughtHtml(streamingSplit.thought);
            } else {
              if (last.isHtml) last.isHtml = false;
              last.thoughtStreaming = false;
              this._queueStreamTypingRender(last, visibleText);
            }
            // Detect function-call patterns streamed as text and convert to tool cards
            var toolScanText = String(last._cleanText || '');
            var fcIdx = toolScanText.search(/\w+<\/function[=,>]/);
            if (fcIdx === -1) fcIdx = toolScanText.search(/<function=\w+>/);
            if (fcIdx !== -1) {
              var fcPart = toolScanText.substring(fcIdx);
              var toolMatch = fcPart.match(/^(\w+)<\/function/) || fcPart.match(/^<function=(\w+)>/);
              var trimmedVisible = toolScanText.substring(0, fcIdx).trim();
              if (streamingSplit.thought && !trimmedVisible) {
                this._clearMessageTypewriter(last);
                last.isHtml = true;
                last.thoughtStreaming = true;
                last.text = this.renderLiveThoughtHtml(streamingSplit.thought);
              } else {
                if (last.isHtml) last.isHtml = false;
                last.thoughtStreaming = false;
                this._clearMessageTypewriter(last);
                last.text = trimmedVisible;
              }
              last._cleanText = trimmedVisible;
              last._toolTextDetected = true;
              if (toolMatch) {
                if (!last.tools) last.tools = [];
                var inputMatch = fcPart.match(/[=,>]\s*(\{[\s\S]*)/);
                last.tools.push({
                  id: toolMatch[1] + '-txt-' + Date.now(),
                  name: toolMatch[1],
                  running: true,
                  expanded: false,
                  input: inputMatch ? inputMatch[1].replace(/<\/function>?\s*$/, '').trim() : '',
                  result: '',
                  is_error: false
                });
              }
            }
            this.tokenCount = Math.round(String(last._cleanText || '').length / 4);
          } else {
            var firstChunk = this.stripModelPrefix(data.content || '');
            var firstSplit = this.extractThinkingLeak(firstChunk);
            var firstVisible = firstSplit.content || '';
            var firstMessage = {
              id: ++msgId,
              role: 'agent',
              text: '',
              meta: '',
              streaming: true,
              tools: [],
              _streamRawText: firstChunk,
              _cleanText: firstVisible,
              _thoughtText: firstSplit.thought || '',
              _stream_updated_at: Date.now(),
              thoughtStreaming: false,
              ts: Date.now(),
              agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
              agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
            };
            if (firstSplit.thought && !firstVisible.trim()) {
              firstMessage.isHtml = true;
              firstMessage.thoughtStreaming = true;
              firstMessage.text = this.renderLiveThoughtHtml(firstSplit.thought);
            }
            this.messages.push(firstMessage);
            if (!firstMessage.isHtml) {
              this._queueStreamTypingRender(firstMessage, firstVisible);
            }
          }
          this.scrollToBottom();
          break;

        case 'tool_start':
          var lastMsg = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg && lastMsg.streaming) {
            if (!lastMsg.tools) lastMsg.tools = [];
            lastMsg.tools.push({ id: data.tool + '-' + Date.now(), name: data.tool, running: true, expanded: false, input: '', result: '', is_error: false });
          }
          this.scrollToBottom();
          break;

        case 'tool_end':
          // Tool call parsed by LLM — update tool card with input params
          var lastMsg2 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg2 && lastMsg2.tools) {
            for (var ti = lastMsg2.tools.length - 1; ti >= 0; ti--) {
              if (lastMsg2.tools[ti].name === data.tool && lastMsg2.tools[ti].running) {
                lastMsg2.tools[ti].input = data.input || '';
                break;
              }
            }
          }
          break;

        case 'tool_result':
          // Tool execution completed — update tool card with result
          var lastMsg3 = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (lastMsg3 && lastMsg3.tools) {
            for (var ri = lastMsg3.tools.length - 1; ri >= 0; ri--) {
              if (lastMsg3.tools[ri].name === data.tool && lastMsg3.tools[ri].running) {
                lastMsg3.tools[ri].running = false;
                lastMsg3.tools[ri].result = data.result || '';
                lastMsg3.tools[ri].is_error = !!data.is_error;
                // Extract image URLs from image_generate or browser_screenshot results
                if ((data.tool === 'image_generate' || data.tool === 'browser_screenshot') && !data.is_error) {
                  try {
                    var parsed = JSON.parse(data.result);
                    if (parsed.image_urls && parsed.image_urls.length) {
                      lastMsg3.tools[ri]._imageUrls = parsed.image_urls;
                    }
                  } catch(e) { /* not JSON */ }
                }
                // Extract audio file path from text_to_speech results
                if (data.tool === 'text_to_speech' && !data.is_error) {
                  try {
                    var ttsResult = JSON.parse(data.result);
                    if (ttsResult.saved_to) {
                      lastMsg3.tools[ri]._audioFile = ttsResult.saved_to;
                      lastMsg3.tools[ri]._audioDuration = ttsResult.duration_estimate_ms;
                    }
                  } catch(e) { /* not JSON */ }
                }
                break;
              }
            }
          }
          this.scrollToBottom();
          break;

        case 'response':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this.applyContextTelemetry(data);
          var wsAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();
          if (!wsAutoSwitchPrevious) wsAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
          var wsRoute = this.applyAutoRouteTelemetry(data);
          var envelope = this.collectStreamedAssistantEnvelope();
          var streamedText = envelope.text;
          var streamedTools = envelope.tools;
          var streamedThought = envelope.thought;
          streamedTools.forEach(function(t) {
            t.running = false;
            // Text-detected tool calls (model leaked as text) — mark as not executed
            if (t.id && t.id.indexOf('-txt-') !== -1 && !t.result) {
              t.result = 'Model attempted this call as text (not executed via tool system)';
              t.is_error = true;
            }
          });
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          var meta = (data.input_tokens || 0) + ' in / ' + (data.output_tokens || 0) + ' out';
          if (data.cost_usd != null) meta += ' | $' + data.cost_usd.toFixed(4);
          if (data.iterations) meta += ' | ' + data.iterations + ' iter';
          if (data.fallback_model) meta += ' | fallback: ' + data.fallback_model;
          var wsDurationMs = Number(data.duration_ms || data.elapsed_ms || data.response_ms || 0);
          if (!wsDurationMs && this._responseStartedAt) {
            wsDurationMs = Math.max(0, Date.now() - this._responseStartedAt);
          }
          var wsDuration = this.formatResponseDuration(wsDurationMs);
          if (wsDuration) meta += ' | ' + wsDuration;
          var wsRouteMeta = this.formatAutoRouteMeta(wsRoute);
          if (wsRouteMeta) meta += ' | ' + wsRouteMeta;
          // Use server response if non-empty, otherwise preserve accumulated streamed text
          var finalText = (data.content && data.content.trim()) ? data.content : streamedText;
          finalText = this.stripModelPrefix(finalText);
          var artifactDirectives = this.extractArtifactDirectives(finalText);
          var finalSplit = this.extractThinkingLeak(finalText);
          if (finalSplit.thought) {
            if (!streamedThought) {
              streamedThought = finalSplit.thought;
            } else if (streamedThought.indexOf(finalSplit.thought) === -1) {
              streamedThought += '\n' + finalSplit.thought;
            }
            finalText = finalSplit.content || '';
          }
          // Strip raw function-call JSON that some models leak as text
          finalText = this.sanitizeToolText(finalText);
          finalText = this.stripArtifactDirectivesFromText(finalText);
          var collapsedThought = String(streamedThought || '').trim();
          var maybePlaceholder = /^(thinking|processing|working)\.\.\.$/i.test(String(finalText || '').trim());
          if (maybePlaceholder && collapsedThought) {
            finalText = '';
          }
          if (collapsedThought) {
            streamedTools.unshift(this.makeThoughtToolCard(collapsedThought, wsDurationMs));
          }
          var usedFallback = false;
          if (!finalText.trim()) {
            finalText = this.defaultAssistantFallback(collapsedThought, streamedTools);
            usedFallback = true;
          }
          var finalMessage = {
            id: ++msgId,
            role: 'agent',
            text: finalText,
            meta: meta,
            tools: streamedTools,
            ts: Date.now(),
            _auto_fallback: usedFallback,
            agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
            agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
          };
          var lastStable = this.messages.length ? this.messages[this.messages.length - 1] : null;
          if (!usedFallback && lastStable && lastStable.role === 'agent' && lastStable._auto_fallback) {
            this.messages[this.messages.length - 1] = finalMessage;
          } else {
            this.messages.push(finalMessage);
          }
          this.markAgentMessageComplete(finalMessage);
          var wsFailure = this.extractRecoverableBackendFailure(finalText);
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this.scrollToBottom();
          this.requestContextTelemetry(false);
          this.maybeAddAutoModelSwitchNotice(wsAutoSwitchPrevious, wsRoute);
          this._pendingAutoModelSwitchBaseline = '';
          if (artifactDirectives && artifactDirectives.length) {
            this.resolveArtifactDirectives(artifactDirectives);
          }
          var self3 = this;
          if (wsFailure) {
            this.attemptAutomaticFailoverRecovery('ws_response', finalText, {
              remove_last_agent_failure: true
            }).then(function(recovered) {
              if (recovered) return;
              self3._inflightPayload = null;
              self3.refreshPromptSuggestions(true, 'post-response-failed-recover');
              self3.$nextTick(function() {
                var el = document.getElementById('msg-input'); if (el) el.focus();
                self3._processQueue();
              });
            });
          } else {
            this._inflightPayload = null;
            this.refreshPromptSuggestions(true, 'post-response');
            this.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              self3._processQueue();
            });
          }
          break;

        case 'silent_complete':
          // Agent intentionally chose not to reply (NO_REPLY)
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this._inflightPayload = null;
          this._pendingAutoModelSwitchBaseline = '';
          var silentEnvelope = this.collectStreamedAssistantEnvelope();
          var silentThought = String(silentEnvelope.thought || '').trim();
          var silentTools = silentEnvelope.tools || [];
          if (silentThought) {
            silentTools.unshift(this.makeThoughtToolCard(silentThought, Number(data && data.duration_ms ? data.duration_ms : 0)));
          }
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.messages.push({
            id: ++msgId,
            role: 'agent',
            text: this.defaultAssistantFallback(silentThought, silentTools),
            meta: '',
            tools: silentTools,
            ts: Date.now(),
            _auto_fallback: true,
            agent_id: data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
            agent_name: data && data.agent_name ? String(data.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
          });
          this.markAgentMessageComplete(this.messages[this.messages.length - 1]);
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          var selfSilent = this;
          this.$nextTick(function() { selfSilent._processQueue(); });
          this.refreshPromptSuggestions(true, 'post-silent');
          break;

        case 'error':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearPendingWsRequest(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '');
          this._clearTypingTimeout();
          this._clearStreamingTypewriters();
          this._pendingAutoModelSwitchBaseline = '';
          var rawError = String(data && data.content ? data.content : 'unknown_error');
          var errorText = 'Error: ' + rawError;
          var lowerError = rawError.toLowerCase();
          if (lowerError.indexOf('agent contract terminated') !== -1 || lowerError.indexOf('agent_contract_terminated') !== -1) {
            this.handleAgentInactive(
              this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
              'contract_terminated',
              { noticeText: errorText }
            );
            break;
          }
          if (lowerError.indexOf('agent is inactive') !== -1 || lowerError.indexOf('agent_inactive') !== -1) {
            this.handleAgentInactive(
              this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
              'inactive',
              { noticeText: errorText }
            );
            break;
          }
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          var self2 = this;
          this.attemptAutomaticFailoverRecovery('ws_error', rawError, {
            remove_last_agent_failure: false
          }).then(function(recovered) {
            if (recovered) return;
            self2.messages.push({
              id: ++msgId,
              role: 'system',
              text: errorText,
              meta: '',
              tools: [],
              system_origin: 'runtime:error',
              ts: Date.now()
            });
            self2._inflightPayload = null;
            self2.scrollToBottom();
            self2.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              self2._processQueue();
            });
            self2.refreshPromptSuggestions(true, 'post-error');
          });
          break;

        case 'agent_archived':
          this.setAgentLiveActivity(
            data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''),
            'idle'
          );
          this._clearPendingWsRequest(data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''));
          this.handleAgentInactive(
            data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''),
            data && data.reason ? String(data.reason) : 'archived'
          );
          break;

        case 'agents_updated':
          if (data.agents) {
            Alpine.store('app').agents = data.agents;
            Alpine.store('app').agentCount = data.agents.length;
          }
          break;

        case 'command_result':
          this.applyContextTelemetry(data);
          var isContextTelemetryResult = Object.prototype.hasOwnProperty.call(data || {}, 'context_tokens') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_window') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_ratio') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_pressure');
          if (!data.silent && !isContextTelemetryResult) {
            this.messages.push({ id: ++msgId, role: 'system', text: data.message || 'Command executed.', meta: '', tools: [], system_origin: 'command:result' });
            this.scrollToBottom();
          }
          break;

        case 'terminal_output':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !(m && m.terminal && m.thinking); });
          var stdout = typeof data.stdout === 'string' ? data.stdout : '';
          var stderr = typeof data.stderr === 'string' ? data.stderr : '';
          var termText = '';
          if (stdout.trim()) termText += stdout;
          if (stderr.trim()) termText += (termText ? '\n' : '') + stderr;
          if (!termText.trim()) termText = '(no output)';
          var termMeta = 'exit ' + (Number.isFinite(Number(data.exit_code)) ? String(Number(data.exit_code)) : '1');
          var termDuration = this.formatResponseDuration(Number(data.duration_ms || 0));
          if (termDuration) termMeta += ' | ' + termDuration;
          var termCwd = this.terminalPromptPath;
          if (data.cwd) {
            termCwd = String(data.cwd);
            this.terminalCwd = termCwd;
            termMeta += ' | ' + termCwd;
          }
          this._appendTerminalMessage({
            role: 'terminal',
            text: termText,
            meta: termMeta,
            tools: [],
            ts: Date.now(),
            cwd: termCwd
          });
          this.sending = false;
          this._responseStartedAt = 0;
          this.scrollToBottom();
          this.$nextTick(() => this._processQueue());
          this.refreshPromptSuggestions(true, 'post-terminal');
          break;

        case 'terminal_error':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !(m && m.terminal && m.thinking); });
          this._appendTerminalMessage({
            role: 'terminal',
            text: 'Terminal error: ' + (data && data.message ? data.message : 'command failed'),
            meta: '',
            tools: [],
            ts: Date.now(),
            cwd: this.terminalPromptPath
          });
          this.sending = false;
          this._responseStartedAt = 0;
          this.scrollToBottom();
          this.$nextTick(() => this._processQueue());
          break;

        case 'canvas':
          // Agent presented an interactive canvas — render it in an iframe sandbox
          var canvasHtml = '<div class="canvas-panel" style="border:1px solid var(--border);border-radius:8px;margin:8px 0;overflow:hidden;">';
          canvasHtml += '<div style="padding:6px 12px;background:var(--surface);border-bottom:1px solid var(--border);font-size:0.85em;display:flex;justify-content:space-between;align-items:center;">';
          canvasHtml += '<span>' + (data.title || 'Canvas') + '</span>';
          canvasHtml += '<span style="opacity:0.5;font-size:0.8em;">' + (data.canvas_id || '').substring(0, 8) + '</span></div>';
          canvasHtml += '<iframe sandbox="allow-scripts" srcdoc="' + (data.html || '').replace(/"/g, '&quot;') + '" ';
          canvasHtml += 'style="width:100%;min-height:300px;border:none;background:#fff;" loading="lazy"></iframe></div>';
          this.messages.push({ id: ++msgId, role: 'agent', text: canvasHtml, meta: 'canvas', isHtml: true, tools: [] });
          this.scrollToBottom();
          break;

        case 'pong': break;
      }
      this.scheduleConversationPersist();
    },

    // Format timestamp for display
    formatTime: function(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      var h = d.getHours();
      var m = d.getMinutes();
      var ampm = h >= 12 ? 'PM' : 'AM';
      h = h % 12 || 12;
      return h + ':' + (m < 10 ? '0' : '') + m + ' ' + ampm;
    },

    isSameDay: function(a, b) {
      if (!a || !b) return false;
      return (
        a.getFullYear() === b.getFullYear() &&
        a.getMonth() === b.getMonth() &&
        a.getDate() === b.getDate()
      );
    },

    // UI-safe timestamp formatter for templates
    messageTs: function(msg) {
      if (!msg || !msg.ts) return '';
      var ts = new Date(msg.ts);
      if (Number.isNaN(ts.getTime())) return '';
      var now = new Date();
      if (this.isSameDay(ts, now)) return this.formatTime(ts);
      var yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      if (this.isSameDay(ts, yesterday)) {
        return 'Yesterday at ' + this.formatTime(ts);
      }
      var dateText = ts.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
      return dateText + ' at ' + this.formatTime(ts);
    },

    parseProgressFromText: function(text) {
      var value = String(text || '');
      if (!value) return null;
      var explicit = value.match(/\[\[\s*progress\s*:\s*([0-9]{1,3})(?:\s*\/\s*([0-9]{1,3}))?\s*\]\]/i);
      if (explicit) {
        var part = Number(explicit[1] || 0);
        var total = Number(explicit[2] || 100);
        if (Number.isFinite(part) && Number.isFinite(total) && total > 0) {
          var pct = Math.max(0, Math.min(100, Math.round((part / total) * 100)));
          return { percent: pct, label: 'Progress ' + pct + '%' };
        }
      }
      var percent = value.match(/\bprogress(?:\s+is)?\s*[:=-]?\s*([0-9]{1,3})\s*%/i);
      if (percent) {
        var p = Number(percent[1] || 0);
        if (Number.isFinite(p)) {
          var clamped = Math.max(0, Math.min(100, Math.round(p)));
          return { percent: clamped, label: 'Progress ' + clamped + '%' };
        }
      }
      return null;
    },

    messageProgress: function(msg) {
      if (!msg || msg.terminal || msg.is_notice) return null;
      var key = String(msg.id || '') + '|' + String(msg.text || '').length + '|' + String(msg.meta || '').length;
      if (!this._progressCache || typeof this._progressCache !== 'object') this._progressCache = {};
      var keys = Object.keys(this._progressCache);
      if (keys.length > 4096) {
        this._progressCache = {};
      }
      if (Object.prototype.hasOwnProperty.call(this._progressCache, key)) return this._progressCache[key];

      var progress = null;
      if (msg.progress && typeof msg.progress === 'object') {
        var pct = Number(msg.progress.percent);
        if (Number.isFinite(pct)) {
          progress = {
            percent: Math.max(0, Math.min(100, Math.round(pct))),
            label: String(msg.progress.label || ('Progress ' + Math.round(pct) + '%')).trim()
          };
        }
      }
      if (!progress) progress = this.parseProgressFromText(msg.text || '');
      this._progressCache[key] = progress;
      return progress;
    },

    progressFillStyle: function(msg) {
      var progress = this.messageProgress(msg);
      if (!progress) return 'width:0%';
      return 'width:' + progress.percent + '%';
    },

    messageDomId: function(msg, idx) {
      var suffix = (msg && msg.id != null) ? String(msg.id) : String(idx || 0);
      return 'chat-msg-' + suffix;
    },

    messageRoleClass: function(msg) {
      if (msg && msg.terminal) return 'terminal';
      if (!msg || !msg.role) return 'agent';
      return String(msg.role);
    },

    thinkingDisplayText: function(msg) {
      var value = String(msg && msg.text ? msg.text : '').trim();
      if (!value) return 'Thinking...';
      value = value.replace(/^\*+|\*+$/g, '').trim();
      return value || 'Thinking...';
    },

    messageGroupRole: function(msg) {
      if (!msg) return '';
      if (msg.terminal) return 'terminal';
      return String(msg.role || '');
    },

    messageSourceKey: function(msg) {
      if (!msg || msg.is_notice) return '';
      if (msg.terminal) {
        var terminalAgentId = String((msg && msg.agent_id) || (this.currentAgent && this.currentAgent.id) || '').trim();
        return terminalAgentId ? ('terminal:' + terminalAgentId) : 'terminal';
      }
      var role = String(msg.role || '').trim().toLowerCase();
      if (!role) return '';
      if (role === 'user') return 'user';
      if (role === 'system') {
        var systemOrigin = String(
          (msg && msg.system_origin) ||
          (msg && msg.agent_origin) ||
          (msg && msg.agent_id) ||
          (msg && msg.actor_id) ||
          (msg && msg.actor) ||
          ''
        ).trim();
        if (systemOrigin) return 'system:' + systemOrigin.toLowerCase();
        // Legacy/cached rows may not carry system_origin; avoid collapsing all
        // such rows into one visual run.
        var legacySystemId = String(
          (msg && msg.id != null) ? msg.id : ((msg && msg.ts != null) ? msg.ts : '')
        ).trim();
        if (legacySystemId) return 'system:legacy:' + legacySystemId.toLowerCase();
        return 'system';
      }
      if (role === 'agent') {
        var agentOrigin = String(
          (msg && msg.agent_origin) ||
          (msg && msg.source_agent_id) ||
          (msg && msg.agent_id) ||
          (msg && msg.actor_id) ||
          (msg && msg.actor) ||
          (msg && msg.agent_name) ||
          ''
        ).trim();
        if (!agentOrigin && this.currentAgent && this.currentAgent.id) {
          agentOrigin = String(this.currentAgent.id || '').trim();
        }
        return agentOrigin ? ('agent:' + agentOrigin.toLowerCase()) : 'agent';
      }
      var genericOrigin = String(
        (msg && msg.agent_id) ||
        (msg && msg.actor_id) ||
        (msg && msg.actor) ||
        ''
      ).trim();
      return genericOrigin ? (role + ':' + genericOrigin.toLowerCase()) : role;
    },

    isFirstInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx === 0) return true;
      var prev = list[idx - 1];
      if (!prev || prev.is_notice) return true;
      var prevKey = this.messageSourceKey(prev);
      return prevKey !== currKey;
    },

    isLastInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx >= list.length - 1) return true;
      var next = list[idx + 1];
      if (!next || next.is_notice) return true;
      var nextKey = this.messageSourceKey(next);
      return nextKey !== currKey;
    },

    messagePreview: function(msg) {
      if (!msg) return '';
      if (msg.is_notice && msg.notice_label) {
        return String(msg.notice_label);
      }
      var raw = '';
      if (typeof msg.text === 'string' && msg.text.trim()) {
        raw = msg.text;
      } else if (Array.isArray(msg.tools) && msg.tools.length) {
        raw = 'Tool calls: ' + msg.tools.map(function(tool) {
          return tool && tool.name ? tool.name : 'tool';
        }).join(', ');
      } else {
        raw = '[' + (msg.role || 'message') + ']';
      }
      var compact = raw.replace(/\s+/g, ' ').trim();
      if (compact.length > 140) return compact.slice(0, 137) + '...';
      return compact;
    },

    messageMapPreview: function(msg) {
      if (this.messageMapMarkerType(msg) === 'tool') {
        return this.messageToolPreview(msg);
      }
      return this.messagePreview(msg);
    },

    messageToolPreview: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) {
        return this.messagePreview(msg);
      }
      var self = this;
      var compactToolText = function(value, maxLen) {
        if (value == null) return '';
        var raw = '';
        if (typeof value === 'string') {
          raw = value;
        } else {
          try {
            raw = JSON.stringify(value);
          } catch (e) {
            raw = String(value);
          }
        }
        var compact = raw.replace(/\s+/g, ' ').trim();
        if (!compact) return '';
        if (compact.length > maxLen) return compact.slice(0, maxLen - 3) + '...';
        return compact;
      };

      var parts = msg.tools.map(function(tool) {
        if (!tool) return '';
        var name = self.toolDisplayName(tool);
        var status = self.toolStatusText(tool);
        var summary = status ? (name + ' [' + status + ']') : name;
        var inputPreview = compactToolText(tool.input, 96);
        var resultPreview = compactToolText(tool.result, 120);
        var detail = '';
        if (inputPreview && resultPreview) {
          detail = inputPreview + ' -> ' + resultPreview;
        } else {
          detail = inputPreview || resultPreview;
        }
        if (detail) summary += ': ' + detail;
        return summary;
      }).filter(function(part) { return !!part; });

      if (!parts.length) return 'Tool call';
      var preview = parts.join(' | ');
      if (preview.length > 220) return preview.slice(0, 217) + '...';
      return preview;
    },

    isLongMessagePreview: function(msg) {
      if (!msg) return false;
      var raw = '';
      if (typeof msg.text === 'string' && msg.text.trim()) {
        raw = msg.text;
      } else if (Array.isArray(msg.tools) && msg.tools.length) {
        raw = msg.tools.map(function(tool) {
          return tool && tool.name ? tool.name : 'tool';
        }).join(', ');
      }
      if (!raw) return false;
      var compact = raw.replace(/\s+/g, ' ').trim();
      if (compact.length >= 220) return true;
      if (raw.indexOf('\n\n') >= 0) return true;
      return false;
    },

    isSelectedMessage: function(msg, idx) {
      if (!this.selectedMessageDomId) return false;
      return this.selectedMessageDomId === this.messageDomId(msg, idx);
    },

    truncateActorLabel: function(label, maxChars) {
      var text = String(label || '').replace(/\s+/g, ' ').trim();
      if (!text) return '';
      var limitRaw = Number(maxChars || 0);
      var limit = Number.isFinite(limitRaw) && limitRaw > 0 ? Math.max(8, Math.floor(limitRaw)) : 24;
      if (text.length <= limit) return text;
      return text.slice(0, limit - 1) + '\u2026';
    },

    messageAgentLabel: function(msg) {
      var name = '';
      if (msg && msg.agent_name) name = String(msg.agent_name || '');
      if (!name && msg && msg.agent_id) {
        var resolved = this.resolveAgent(msg.agent_id);
        if (resolved && resolved.name) name = String(resolved.name || '');
      }
      if (!name && this.currentAgent && this.currentAgent.name) {
        name = String(this.currentAgent.name || '');
      }
      var shortName = this.truncateActorLabel(name, 28);
      return shortName || 'Agent';
    },

    messageActorLabel: function(msg) {
      if (!msg) return 'Message';
      if (msg.is_notice) {
        if (this.normalizeNoticeType(msg.notice_type, 'model') === 'info') return '\u24d8 Info';
        return 'Model';
      }
      if (msg.terminal) return 'Terminal';
      if (Array.isArray(msg.tools) && msg.tools.length && (!msg.text || !String(msg.text).trim())) {
        return 'Tool';
      }
      if (msg.role === 'user') return 'You';
      if (msg.role === 'system') return 'System';
      if (msg.role === 'agent') {
        var name = '';
        if (msg && msg.agent_name) name = String(msg.agent_name || '');
        if (!name && msg && msg.agent_id) {
          var resolved = this.resolveAgent(msg.agent_id);
          if (resolved && resolved.name) name = String(resolved.name || '');
        }
        if (!name && this.currentAgent && this.currentAgent.name) {
          name = String(this.currentAgent.name || '');
        }
        var shortName = this.truncateActorLabel(name, 24);
        if (shortName) return shortName;
      }
      return 'Agent';
    },

    isRenameNotice: function(msg) {
      if (!msg || !msg.is_notice) return false;
      return /^changed name from /i.test(String(msg.notice_label || '').trim());
    },

    messageMapToolOutcome: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
      var hasError = false;
      var hasWarning = false;
      for (var i = 0; i < msg.tools.length; i++) {
        var tool = msg.tools[i] || {};
        if (tool.running || this.isBlockedTool(tool)) {
          hasWarning = true;
          continue;
        }
        if (tool.is_error) {
          hasError = true;
        }
      }
      if (hasError) return 'error';
      if (hasWarning) return 'warning';
      return 'success';
    },

    messageMapMarkerType: function(msg) {
      if (!msg) return '';
      if (msg.is_notice) {
        return this.normalizeNoticeType(msg.notice_type, 'model') === 'info' ? 'info' : 'model';
      }
      if (msg.terminal) return 'terminal';
      if (Array.isArray(msg.tools) && msg.tools.length) return 'tool';
      return '';
    },

    messageMapShowMarker: function(msg) {
      return this.messageMapMarkerType(msg) !== '';
    },

    messageMapMarkerTitle: function(msg) {
      var type = this.messageMapMarkerType(msg);
      if (type === 'model') {
        return msg && msg.notice_label ? String(msg.notice_label) : 'Model switched';
      }
      if (type === 'info') {
        return msg && msg.notice_label ? String(msg.notice_label) : 'Info';
      }
      if (type === 'tool') {
        var outcome = this.messageMapToolOutcome(msg) || 'success';
        if (outcome === 'error') return 'Tool call error';
        if (outcome === 'warning') return 'Tool call warning';
        return 'Tool call success';
      }
      if (type === 'terminal') {
        return 'Terminal activity';
      }
      return '';
    },

    messageDayKey: function(msg) {
      if (!msg || !msg.ts) return '';
      var d = new Date(msg.ts);
      if (Number.isNaN(d.getTime())) return '';
      var y = d.getFullYear();
      var m = String(d.getMonth() + 1).padStart(2, '0');
      var day = String(d.getDate()).padStart(2, '0');
      return y + '-' + m + '-' + day;
    },

    messageDayLabel: function(msg) {
      if (!msg || !msg.ts) return 'Unknown day';
      var d = new Date(msg.ts);
      if (Number.isNaN(d.getTime())) return 'Unknown day';
      return d.toLocaleDateString(undefined, { weekday: 'long', month: 'short', day: 'numeric', year: 'numeric' });
    },

    messageDayDomId: function(msg) {
      var key = this.messageDayKey(msg);
      return key ? ('chat-day-' + key) : '';
    },

    isMessageDayCollapsed: function(msg) {
      var key = this.messageDayKey(msg);
      if (!key) return false;
      return !!(this.collapsedMessageDays && this.collapsedMessageDays[key]);
    },

    toggleMessageDayCollapse: function(msg) {
      var key = this.messageDayKey(msg);
      if (!key) return;
      if (!this.collapsedMessageDays) this.collapsedMessageDays = {};
      this.collapsedMessageDays[key] = !this.collapsedMessageDays[key];
    },

    isNewMessageDay: function(list, idx) {
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      if (idx === 0) return true;
      var curr = this.messageDayKey(list[idx]);
      var prev = this.messageDayKey(list[idx - 1]);
      if (!curr) return false;
      return curr !== prev;
    },

    jumpToMessage: function(msg, idx) {
      var id = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      var target = document.getElementById(id);
      if (!target) return;
      this.selectedMessageDomId = id;
      this.hoveredMessageDomId = id;
      target.scrollIntoView({ behavior: 'smooth', block: 'center' });
      this.mapStepIndex = idx;
      this.centerChatMapOnMessage(id);
    },

    jumpToMessageDay: function(msg) {
      var key = this.messageDayKey(msg);
      if (!key) return;
      var target = document.querySelector('.chat-day-anchor[data-day="' + key + '"]');
      if (!target) return;
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    },

    addNoticeEvent: function(notice) {
      if (!notice || typeof notice !== 'object') return;
      var label = String(notice.notice_label || notice.label || '').trim();
      if (!label) return;
      var type = this.normalizeNoticeType(
        notice.notice_type || notice.type,
        this.isModelSwitchNoticeLabel(label) ? 'model' : 'info'
      );
      var icon = String(notice.notice_icon || notice.icon || '').trim();
      if (type === 'info' && /^changed name from /i.test(label)) {
        icon = '';
      }
      var tsRaw = Number(notice.ts || 0);
      var ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: '',
        meta: '',
        tools: [],
        system_origin: 'notice:' + type,
        is_notice: true,
        notice_label: label,
        notice_type: type,
        notice_icon: icon,
        ts: ts
      });
      if (this.currentAgent && this.currentAgent.id) {
        this.rememberModelNotice(this.currentAgent.id, label, ts, type, icon);
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    addModelSwitchNotice: function(previousModelName, previousProviderName, modelName, providerName) {
      var legacyCall = arguments.length < 3;
      var previousModel = '';
      var model = '';
      if (legacyCall) {
        model = String(previousModelName || '').trim();
      } else {
        previousModel = String(previousModelName || '').trim();
        model = String(modelName || '').trim();
      }
      if (!model) return;
      if (!previousModel && this.currentAgent) {
        previousModel = String(this.currentAgent.runtime_model || this.currentAgent.model_name || '').trim();
      }
      if (!previousModel) previousModel = 'unknown';
      var label = 'Model switched from ' + previousModel + ' to ' + model;
      this.touchModelUsage(model);
      this.addNoticeEvent({ notice_label: label, notice_type: 'model', ts: Date.now() });
    },

    addAgentRenameNotice: function(previousName, nextName) {
      var fromName = String(previousName || '').trim();
      var toName = String(nextName || '').trim();
      if (!toName || fromName === toName) return;
      if (!fromName) fromName = 'Unnamed agent';
      this.addNoticeEvent({
        notice_label: 'changed name from ' + fromName + ' to ' + toName,
        notice_type: 'info',
        ts: Date.now()
      });
    },

    formatResponseDuration: function(ms) {
      var num = Number(ms || 0);
      if (!Number.isFinite(num) || num <= 0) return '';
      if (num < 1000) return Math.round(num) + 'ms';
      if (num < 60000) {
        return (num < 10000 ? (num / 1000).toFixed(1) : Math.round(num / 1000)) + 's';
      }
      var min = Math.floor(num / 60000);
      var sec = Math.round((num % 60000) / 1000);
      return min + 'm ' + sec + 's';
    },

    stepMessageMap: function(list, dir) {
      if (!Array.isArray(list) || !list.length) return;
      this.suppressMapPreview = true;
      this.activeMapPreviewDomId = '';
      this.activeMapPreviewDayKey = '';
      if (this._mapPreviewSuppressTimer) clearTimeout(this._mapPreviewSuppressTimer);
      var visibleIndexes = [];
      for (var i = 0; i < list.length; i++) {
        if (!this.isMessageDayCollapsed(list[i])) visibleIndexes.push(i);
      }
      if (!visibleIndexes.length) return;

      var activePos = -1;
      var anchorDomId = String(this.selectedMessageDomId || '');
      if (anchorDomId) {
        for (var p = 0; p < visibleIndexes.length; p++) {
          var vi = visibleIndexes[p];
          if (this.messageDomId(list[vi], vi) === anchorDomId) {
            activePos = p;
            break;
          }
        }
      }
      if (activePos < 0) {
        for (var p2 = 0; p2 < visibleIndexes.length; p2++) {
          if (visibleIndexes[p2] === this.mapStepIndex) {
            activePos = p2;
            break;
          }
        }
      }

      if (activePos < 0) {
        activePos = dir > 0 ? 0 : (visibleIndexes.length - 1);
      } else {
        activePos = activePos + (dir > 0 ? 1 : -1);
        if (activePos < 0) activePos = 0;
        if (activePos > visibleIndexes.length - 1) activePos = visibleIndexes.length - 1;
      }

      var next = visibleIndexes[activePos];
      var msg = list[next];
      if (!msg) return;
      this.setHoveredMessage(msg, next);
      this.jumpToMessage(msg, next);
      this.centerChatMapOnMessage(this.messageDomId(msg, next));
      var self = this;
      this._mapPreviewSuppressTimer = setTimeout(function() {
        self.suppressMapPreview = false;
      }, 220);
    },

    setMapItemHover: function(msg, idx) {
      if (!msg) return;
      var domId = this.messageDomId(msg, idx);
      this.forceMessageRender(msg, idx, 9000);
      this.suppressMapPreview = false;
      this.activeMapPreviewDomId = domId;
      this.activeMapPreviewDayKey = '';
      this.selectedMessageDomId = domId;
      this.mapStepIndex = idx;
      this.setHoveredMessage(msg, idx);
    },

    clearMapItemHover: function() {
      this.activeMapPreviewDomId = '';
      this.clearHoveredMessage();
    },

    setMapDayHover: function(msg) {
      if (!msg) return;
      this.suppressMapPreview = false;
      this.activeMapPreviewDayKey = this.messageDayKey(msg);
      this.activeMapPreviewDomId = '';
    },

    clearMapDayHover: function() {
      this.activeMapPreviewDayKey = '';
    },

    isMapPreviewVisible: function(msg, idx) {
      if (this.suppressMapPreview) return false;
      if (!msg) return false;
      return this.activeMapPreviewDomId === this.messageDomId(msg, idx);
    },

    isMapDayPreviewVisible: function(msg) {
      if (this.suppressMapPreview) return false;
      if (!msg) return false;
      return this.activeMapPreviewDayKey === this.messageDayKey(msg);
    },

    setHoveredMessage: function(msg, idx) {
      if (!msg && msg !== 0) {
        this.hoveredMessageDomId = this.selectedMessageDomId || '';
        return;
      }
      this.hoveredMessageDomId = this.messageDomId(msg, idx);
    },

    clearHoveredMessage: function() {
      this.hoveredMessageDomId = this.selectedMessageDomId || '';
    },

    clearHoveredMessageHard: function() {
      this.hoveredMessageDomId = '';
      this.selectedMessageDomId = '';
    },

    isHoveredMessage: function(msg, idx) {
      if (!this.hoveredMessageDomId) return false;
      return this.hoveredMessageDomId === this.messageDomId(msg, idx);
    },

    centerChatMapOnMessage: function(domId, options) {
      if (!domId) return;
      var immediate = !!(options && options.immediate);
      var map = null;
      var maps = document.querySelectorAll('.chat-map-scroll');
      for (var i = 0; i < maps.length; i++) {
        var candidate = maps[i];
        if (candidate && candidate.offsetParent !== null) {
          map = candidate;
          break;
        }
      }
      if (!map) return;
      var host = map.closest('.chat-map') || map;
      var item = host.querySelector('.chat-map-item[data-msg-dom-id="' + domId + '"]');
      if (!item) return;
      var topGuard = 28;
      var bottomGuard = 28;
      var viewport = Math.max(20, map.clientHeight - topGuard - bottomGuard);
      var desired = item.offsetTop + (item.offsetHeight / 2) - (viewport / 2) - topGuard;
      var max = Math.max(0, map.scrollHeight - map.clientHeight);
      var nextTop = Math.max(0, Math.min(max, desired));
      var diff = Math.abs(map.scrollTop - nextTop);
      if (diff < 3) return;
      map.scrollTo({ top: nextTop, behavior: (immediate || this.suppressMapPreview) ? 'auto' : 'smooth' });
    },

    filteredDrawerEmojiCatalog: function() {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      var query = String(this.drawerEmojiSearch || '').trim().toLowerCase();
      if (!query) return source.slice(0, 24);
      return source.filter(function(row) {
        var emoji = String((row && row.emoji) || '');
        var name = String((row && row.name) || '').toLowerCase();
        return emoji.indexOf(query) >= 0 || name.indexOf(query) >= 0;
      }).slice(0, 24);
    },

    defaultFreshEmojiForAgent: function(agentRef) {
      var source = Array.isArray(this.drawerEmojiCatalog) ? this.drawerEmojiCatalog : [];
      if (!source.length) return '🤖';
      var key = '';
      if (agentRef && typeof agentRef === 'object') {
        key = String(agentRef.id || agentRef.name || '').trim();
      } else {
        key = String(agentRef || '').trim();
      }
      if (!key) return String((source[0] && source[0].emoji) || '🤖');
      var hash = 0;
      for (var idx = 0; idx < key.length; idx += 1) {
        hash = ((hash * 33) ^ key.charCodeAt(idx)) >>> 0;
      }
      var bucket = hash % source.length;
      return String((source[bucket] && source[bucket].emoji) || '🤖');
    },

    toggleDrawerEmojiPicker: function() {
      this.drawerEmojiPickerOpen = !this.drawerEmojiPickerOpen;
      if (!this.drawerEmojiPickerOpen) {
        this.drawerEmojiSearch = '';
      } else {
        this.drawerEditingEmoji = true;
      }
    },

    selectDrawerEmoji: function(choice) {
      var emoji = String(choice && choice.emoji ? choice.emoji : choice || '').trim();
      if (!emoji) return;
      if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
        this.drawerConfigForm = {};
      }
      this.drawerConfigForm.emoji = emoji;
      // Choosing emoji explicitly switches away from image avatar mode.
      this.drawerConfigForm.avatar_url = '';
      if (this.agentDrawer && typeof this.agentDrawer === 'object') {
        this.agentDrawer.avatar_url = '';
      }
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerEditingEmoji = false;
    },

    openDrawerAvatarPicker: function() {
      if (this.$refs && this.$refs.drawerAvatarInput) {
        this.$refs.drawerAvatarInput.click();
      }
    },

    uploadDrawerAvatar: async function(fileList) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var files = Array.isArray(fileList) ? fileList : Array.from(fileList || []);
      if (!files.length) return;
      var file = files[0];
      if (!file) return;
      var mime = String(file.type || '').toLowerCase();
      if (mime && mime.indexOf('image/') !== 0) {
        InfringToast.error('Avatar must be an image file.');
        return;
      }
      this.drawerAvatarUploading = true;
      this.drawerAvatarUploadError = '';
      try {
        var headers = {
          'Content-Type': file.type || 'application/octet-stream',
          'X-Filename': file.name || 'avatar'
        };
        var token = (typeof InfringAPI !== 'undefined' && typeof InfringAPI.getToken === 'function')
          ? String(InfringAPI.getToken() || '')
          : '';
        if (token) headers.Authorization = 'Bearer ' + token;
        var response = await fetch('/api/agents/' + encodeURIComponent(this.agentDrawer.id) + '/avatar', {
          method: 'POST',
          headers: headers,
          body: file
        });
        var payload = null;
        try {
          payload = await response.json();
        } catch (_) {
          payload = null;
        }
        if (!response.ok || !payload || !payload.ok || !payload.avatar_url) {
          var reason = payload && payload.error ? payload.error : 'avatar_upload_failed';
          throw new Error(String(reason));
        }
        if (!this.drawerConfigForm || typeof this.drawerConfigForm !== 'object') {
          this.drawerConfigForm = {};
        }
        this.drawerConfigForm.avatar_url = String(payload.avatar_url || '').trim();
        this.agentDrawer.avatar_url = String(payload.avatar_url || '').trim();
        this.drawerEditingEmoji = false;
        this.drawerEmojiPickerOpen = false;
        InfringToast.success('Avatar uploaded');
        await this.saveDrawerIdentity('avatar');
      } catch (error) {
        var message = (error && error.message) ? String(error.message) : 'avatar_upload_failed';
        this.drawerAvatarUploadError = message;
        InfringToast.error('Failed to upload avatar: ' + message);
      } finally {
        this.drawerAvatarUploading = false;
      }
    },

    async openAgentDrawer() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      this.showAgentDrawer = true;
      this.agentDrawerLoading = true;
      this.drawerTab = 'info';
      this.drawerEditingModel = false;
      this.drawerEditingProvider = false;
      this.drawerEditingFallback = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUploading = false;
      this.drawerAvatarUploadError = '';
      this.drawerIdentitySaving = false;
      this.drawerSavePending = false;
      this.drawerNewModelValue = '';
      this.drawerNewProviderValue = '';
      this.drawerNewFallbackValue = '';
      var base = this.resolveAgent(this.currentAgent) || this.currentAgent;
      this.agentDrawer = Object.assign({}, base, {
        _fallbacks: Array.isArray(base && base._fallbacks) ? base._fallbacks : []
      });
      this.drawerConfigForm = {
        name: this.agentDrawer.name || '',
        system_prompt: this.agentDrawer.system_prompt || '',
        emoji: (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '',
        avatar_url: this.agentDrawer.avatar_url || '',
        color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
        archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
        vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
      };
      try {
        var full = await InfringAPI.get('/api/agents/' + this.currentAgent.id);
        this.agentDrawer = Object.assign({}, base, full || {}, {
          _fallbacks: Array.isArray(full && full.fallback_models) ? full.fallback_models : []
        });
        this.drawerConfigForm = {
          name: this.agentDrawer.name || '',
          system_prompt: this.agentDrawer.system_prompt || '',
          emoji: (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '',
          avatar_url: this.agentDrawer.avatar_url || '',
          color: (this.agentDrawer.identity && this.agentDrawer.identity.color) || '#2563EB',
          archetype: (this.agentDrawer.identity && this.agentDrawer.identity.archetype) || '',
          vibe: (this.agentDrawer.identity && this.agentDrawer.identity.vibe) || '',
        };
      } catch(e) {
        // Keep best-effort drawer data from current agent/store.
      } finally {
        this.agentDrawerLoading = false;
      }
    },

    closeAgentDrawer() {
      this.showAgentDrawer = false;
      this.drawerEditingName = false;
      this.drawerEditingEmoji = false;
      this.drawerEmojiPickerOpen = false;
      this.drawerEmojiSearch = '';
      this.drawerAvatarUploadError = '';
    },

    toggleAgentDrawer() {
      if (this.showAgentDrawer) {
        this.closeAgentDrawer();
        return;
      }
      this.openAgentDrawer();
    },

    async syncDrawerAgentAfterChange() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      try {
        await Alpine.store('app').refreshAgents();
      } catch {}
      var refreshed = this.resolveAgent(this.agentDrawer.id);
      if (refreshed) {
        this.currentAgent = refreshed;
      }
      await this.openAgentDrawer();
    },

    async setDrawerMode(mode) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      try {
        await InfringAPI.put('/api/agents/' + this.agentDrawer.id + '/mode', { mode: mode });
        InfringToast.success('Mode set to ' + mode);
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to set mode: ' + e.message);
      }
    },

    async saveDrawerAll() {
      if (!this.agentDrawer || !this.agentDrawer.id || this.drawerSavePending) return;
      var agentId = this.agentDrawer.id;
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      var requestedName = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      var previousFallbacks = Array.isArray(this.agentDrawer._fallbacks) ? this.agentDrawer._fallbacks.slice() : [];
      var appendedFallback = false;
      this.drawerSavePending = true;
      this.drawerConfigSaving = true;
      this.drawerModelSaving = true;
      this.drawerIdentitySaving = true;
      try {
        var configPayload = Object.assign({}, this.drawerConfigForm || {});
        if (this.drawerEditingFallback && String(this.drawerNewFallbackValue || '').trim()) {
          var fallbackParts = String(this.drawerNewFallbackValue || '').trim().split('/');
          var fallbackProvider = fallbackParts.length > 1 ? fallbackParts[0] : this.agentDrawer.model_provider;
          var fallbackModel = fallbackParts.length > 1 ? fallbackParts.slice(1).join('/') : fallbackParts[0];
          if (!Array.isArray(this.agentDrawer._fallbacks)) this.agentDrawer._fallbacks = [];
          this.agentDrawer._fallbacks.push({ provider: fallbackProvider, model: fallbackModel });
          appendedFallback = true;
          configPayload.fallback_models = this.agentDrawer._fallbacks;
        } else if (Array.isArray(this.agentDrawer._fallbacks)) {
          configPayload.fallback_models = this.agentDrawer._fallbacks;
        }

        var configResponse = await InfringAPI.patch('/api/agents/' + agentId + '/config', configPayload);
        if (configResponse && configResponse.rename_notice) {
          this.addNoticeEvent(configResponse.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }

        if (this.drawerEditingProvider && String(this.drawerNewProviderValue || '').trim()) {
          var previousProviderName = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          var previousModelName = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var combined = String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '');
          await this.switchAgentModelWithGuards({ id: combined }, {
            agent_id: agentId,
            previous_model: previousModelName,
            previous_provider: previousProviderName
          });
        } else if (this.drawerEditingModel && String(this.drawerNewModelValue || '').trim()) {
          var previousModelNameForModelEdit = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var previousProviderForModelEdit = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          await this.switchAgentModelWithGuards(
            { id: String(this.drawerNewModelValue || '').trim() },
            {
              agent_id: agentId,
              previous_model: previousModelNameForModelEdit,
              previous_provider: previousProviderForModelEdit
            }
          );
        }

        this.drawerEditingName = false;
        this.drawerEditingEmoji = false;
        this.drawerEditingModel = false;
        this.drawerEditingProvider = false;
        this.drawerEditingFallback = false;
        this.drawerNewModelValue = '';
        this.drawerNewProviderValue = '';
        this.drawerNewFallbackValue = '';
        InfringToast.success('Agent settings saved');
        await this.syncDrawerAgentAfterChange();
      } catch (e) {
        if (appendedFallback) {
          this.agentDrawer._fallbacks = previousFallbacks;
        }
        InfringToast.error('Failed to save agent settings: ' + e.message);
      } finally {
        this.drawerSavePending = false;
        this.drawerConfigSaving = false;
        this.drawerModelSaving = false;
        this.drawerIdentitySaving = false;
      }
    },

    async saveDrawerConfig() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      var requestedName = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      this.drawerConfigSaving = true;
      try {
        var response = await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', this.drawerConfigForm || {});
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }
        InfringToast.success('Config updated');
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to save config: ' + e.message);
      }
      this.drawerConfigSaving = false;
    },

    async saveDrawerIdentity(part) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var payload = {};
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      if (part === 'name') {
        payload.name = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      } else if (part === 'emoji') {
        payload.emoji = String((this.drawerConfigForm && this.drawerConfigForm.emoji) || '').trim();
        payload.avatar_url = '';
        if (this.drawerConfigForm && typeof this.drawerConfigForm === 'object') {
          this.drawerConfigForm.avatar_url = '';
        }
        if (this.agentDrawer && typeof this.agentDrawer === 'object') {
          this.agentDrawer.avatar_url = '';
        }
      } else if (part === 'avatar') {
        payload.avatar_url = String((this.drawerConfigForm && this.drawerConfigForm.avatar_url) || '').trim();
      } else {
        return;
      }
      this.drawerIdentitySaving = true;
      try {
        var response = await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', payload);
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (part === 'name' && payload.name && payload.name !== previousName) {
          this.addAgentRenameNotice(previousName, payload.name);
        }
        if (part === 'name') this.drawerEditingName = false;
        if (part === 'emoji') this.drawerEditingEmoji = false;
        if (part === 'avatar') this.drawerAvatarUploadError = '';
        InfringToast.success(
          part === 'name'
            ? 'Name updated'
            : (part === 'emoji' ? 'Emoji updated' : 'Avatar updated')
        );
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to save ' + part + ': ' + e.message);
      }
      this.drawerIdentitySaving = false;
    },

    async changeDrawerModel() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewModelValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var resp = await this.switchAgentModelWithGuards(
          { id: String(this.drawerNewModelValue || '').trim() },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        var providerInfo = (resp && resp.provider) ? ' (provider: ' + resp.provider + ')' : '';
        InfringToast.success('Model changed' + providerInfo + ' (memory reset)');
        this.drawerEditingModel = false;
        this.drawerNewModelValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change model: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async changeDrawerProvider() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewProviderValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var combined = String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '');
        var resp = await this.switchAgentModelWithGuards(
          { id: combined },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        InfringToast.success('Provider changed to ' + (resp && resp.provider ? resp.provider : String(this.drawerNewProviderValue || '').trim()));
        this.drawerEditingProvider = false;
        this.drawerNewProviderValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change provider: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async addDrawerFallback() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewFallbackValue || '').trim()) return;
      var parts = String(this.drawerNewFallbackValue || '').trim().split('/');
      var provider = parts.length > 1 ? parts[0] : this.agentDrawer.model_provider;
      var model = parts.length > 1 ? parts.slice(1).join('/') : parts[0];
      if (!this.agentDrawer._fallbacks) this.agentDrawer._fallbacks = [];
      this.agentDrawer._fallbacks.push({ provider: provider, model: model });
      try {
        await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        InfringToast.success('Fallback added: ' + provider + '/' + model);
        this.drawerEditingFallback = false;
        this.drawerNewFallbackValue = '';
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        this.agentDrawer._fallbacks.pop();
      }
    },

    async removeDrawerFallback(idx) {
      if (!this.agentDrawer || !this.agentDrawer.id || !Array.isArray(this.agentDrawer._fallbacks)) return;
      var removed = this.agentDrawer._fallbacks.splice(idx, 1);
      try {
        await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        InfringToast.success('Fallback removed');
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        if (removed && removed.length) this.agentDrawer._fallbacks.splice(idx, 0, removed[0]);
      }
    },

    isBlockedTool: function(tool) {
      if (!tool) return false;
      if (tool.blocked === true) return true;
      var txt = String(tool.result || '').toLowerCase();
      if (String(tool.status || '').toLowerCase() === 'blocked') return true;
      if (!tool.is_error) return false;
      return (
        txt.indexOf('blocked') >= 0 ||
        txt.indexOf('policy') >= 0 ||
        txt.indexOf('denied') >= 0 ||
        txt.indexOf('not allowed') >= 0 ||
        txt.indexOf('forbidden') >= 0 ||
        txt.indexOf('approval') >= 0 ||
        txt.indexOf('permission') >= 0 ||
        txt.indexOf('fail-closed') >= 0
      );
    },

    isToolSuccessful: function(tool) {
      if (!tool) return false;
      if (tool.running) return false;
      if (this.isBlockedTool(tool)) return false;
      if (tool.is_error) return false;
      return true;
    },

    isThoughtTool: function(tool) {
      return !!(tool && String(tool.name || '').toLowerCase() === 'thought_process');
    },

    toolDisplayName: function(tool) {
      if (!tool) return 'tool';
      if (this.isThoughtTool(tool)) return 'thought';
      return String(tool.name || 'tool');
    },

    thoughtToolDurationSeconds: function(tool) {
      if (!tool || typeof tool !== 'object') return 0;
      var ms = Number(tool.duration_ms || tool.durationMs || tool.elapsed_ms || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      var seconds = Math.round(ms / 1000);
      if (ms > 0 && seconds < 1) seconds = 1;
      return Math.max(0, seconds);
    },

    thoughtToolLabel: function(tool) {
      return 'Thought for ' + this.thoughtToolDurationSeconds(tool) + ' seconds';
    },

    toolStatusText: function(tool) {
      if (!tool) return '';
      if (tool.running) return 'running...';
      if (this.isThoughtTool(tool)) return 'thought';
      if (this.isBlockedTool(tool)) return 'blocked';
      if (tool.is_error) return 'error';
      if (tool.result) {
        return tool.result.length > 500 ? Math.round(tool.result.length / 1024) + 'KB' : 'done';
      }
      return 'done';
    },

    // Mark chat-rendered error messages for styling
    isErrorMessage: function(msg) {
      if (!msg || !msg.text) return false;
      if (String(msg.role || '').toLowerCase() !== 'system') return false;
      var t = String(msg.text).trim().toLowerCase();
      return t.startsWith('error:');
    },

    messageHasTools: function(msg) {
      return !!(msg && Array.isArray(msg.tools) && msg.tools.length);
    },

    allToolsCollapsed: function(msg) {
      if (!this.messageHasTools(msg)) return true;
      return !msg.tools.some(function(tool) {
        return !!(tool && tool.expanded);
      });
    },

    toggleMessageTools: function(msg) {
      if (!this.messageHasTools(msg)) return;
      var expand = this.allToolsCollapsed(msg);
      msg.tools.forEach(function(tool) {
        if (tool) tool.expanded = expand;
      });
      this.scheduleConversationPersist();
    },

    // Copy message text to clipboard
    copyMessage: function(msg) {
      var text = msg.text || '';
      navigator.clipboard.writeText(text).then(function() {
        msg._copied = true;
        setTimeout(function() { msg._copied = false; }, 2000);
      }).catch(function() {});
    },

    // Process queued messages after current response completes
    _processQueue: function() {
      if (!this.messageQueue.length || this.sending || this._inflightFailoverInProgress) return;
      var next = this.messageQueue.shift();
      if (next && next.terminal) {
        this._sendTerminalPayload(next.command);
        return;
      }
      this._sendPayload(next.text, next.files, next.images);
    },

    _terminalPromptLine: function(cwd, command) {
      var path = String(cwd || this.terminalPromptPath || '/workspace');
      var cmd = String(command || '').trim();
      if (!cmd) return path + ' %';
      return path + ' % ' + cmd;
    },

    _appendTerminalMessage: function(entry) {
      var payload = entry || {};
      var text = String(payload.text || '');
      var now = Date.now();
      var ts = Number.isFinite(Number(payload.ts)) ? Number(payload.ts) : now;
      var role = payload.role ? String(payload.role) : 'terminal';
      var cwd = payload.cwd ? String(payload.cwd) : this.terminalPromptPath;
      var meta = payload.meta == null ? '' : String(payload.meta);
      var tools = Array.isArray(payload.tools) ? payload.tools : [];

      var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (last && !last.thinking && last.terminal) {
        if (text) {
          if (last.text && !/\n$/.test(last.text)) last.text += '\n';
          last.text += text;
        }
        if (meta) last.meta = meta;
        if (cwd) {
          last.cwd = cwd;
          this.terminalCwd = cwd;
        }
        last.ts = ts;
        if (!Array.isArray(last.tools)) last.tools = [];
        if (tools.length) last.tools = last.tools.concat(tools);
        return last;
      }

      var msg = {
        id: ++msgId,
        role: role,
        text: text,
        meta: meta,
        tools: tools,
        ts: ts,
        terminal: true,
        cwd: cwd
      };
      this.messages.push(msg);
      if (cwd) this.terminalCwd = cwd;
      return msg;
    },

    async sendTerminalMessage() {
      if (this.showFreshArchetypeTiles) {
        InfringToast.info('Launch agent initialization before running terminal commands.');
        return;
      }
      if (!this.currentAgent || !this.inputText.trim()) return;
      this.showFreshArchetypeTiles = false;
      var command = this.inputText.trim();
      this.inputText = '';
      this.terminalSelectionStart = 0;

      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';

      if (this.sending) {
        this._reconcileSendingState();
      }
      if (this.sending) {
        this.messageQueue.push({
          queue_id: this.nextPromptQueueId(),
          queue_kind: 'terminal',
          queued_at: Date.now(),
          terminal: true,
          command: command
        });
        return;
      }

      this._sendTerminalPayload(command);
    },

    async sendMessage() {
      if (this.terminalMode) {
        await this.sendTerminalMessage();
        return;
      }
      if (this.showFreshArchetypeTiles) {
        InfringToast.info('Launch agent initialization before chatting.');
        return;
      }
      if (!this.currentAgent || (!this.inputText.trim() && !this.attachments.length)) return;
      this.showFreshArchetypeTiles = false;
      var text = this.inputText.trim();

      // Handle slash commands
      if (text.startsWith('/') && !this.attachments.length) {
        var cmd = text.split(' ')[0].toLowerCase();
        var cmdArgs = text.substring(cmd.length).trim();
        var matched = this.slashCommands.find(function(c) { return c.cmd === cmd; });
        if (matched) {
          this.executeSlashCommand(matched.cmd, cmdArgs);
          return;
        }
      }

      this.inputText = '';

      // Reset textarea height to single line
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';

      // Upload attachments first if any
      var fileRefs = [];
      var uploadedFiles = [];
      if (this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          var att = this.attachments[i];
          att.uploading = true;
          try {
            var uploadRes = await InfringAPI.upload(this.currentAgent.id, att.file);
            fileRefs.push('[File: ' + att.file.name + ']');
            uploadedFiles.push({ file_id: uploadRes.file_id, filename: uploadRes.filename, content_type: uploadRes.content_type });
          } catch(e) {
            var reason = (e && e.message) ? String(e.message) : 'upload_failed';
            InfringToast.error('Failed to upload ' + att.file.name + ': ' + reason);
            fileRefs.push('[File: ' + att.file.name + ' (upload failed)]');
          }
          att.uploading = false;
        }
        // Clean up previews
        for (var j = 0; j < this.attachments.length; j++) {
          if (this.attachments[j].preview) URL.revokeObjectURL(this.attachments[j].preview);
        }
        this.attachments = [];
      }

      // Build final message text
      var finalText = text;
      if (fileRefs.length) {
        finalText = (text ? text + '\n' : '') + fileRefs.join('\n');
      }

      // Collect image references for inline rendering
      var msgImages = uploadedFiles.filter(function(f) { return f.content_type && f.content_type.startsWith('image/'); });

      // Always show user message immediately
      this.messages.push({ id: ++msgId, role: 'user', text: finalText, meta: '', tools: [], images: msgImages, ts: Date.now() });
      this.scrollToBottom();
      localStorage.setItem('of-first-msg', 'true');
      this.promptSuggestions = [];
      this.scheduleConversationPersist();

      // If already streaming, queue this message
      if (this.sending) {
        this._reconcileSendingState();
      }
      if (this.sending) {
        this.messageQueue.push({
          queue_id: this.nextPromptQueueId(),
          queue_kind: 'prompt',
          queued_at: Date.now(),
          text: finalText,
          files: uploadedFiles,
          images: msgImages
        });
        return;
      }

      this._sendPayload(finalText, uploadedFiles, msgImages);
    },

    async _sendTerminalPayload(command) {
      this.sending = true;
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'working');
      this._responseStartedAt = Date.now();
      this._appendTerminalMessage({
        role: 'terminal',
        text: this._terminalPromptLine(this.terminalPromptPath, command),
        meta: this.terminalPromptPath,
        tools: [],
        ts: Date.now(),
        cwd: this.terminalPromptPath
      });
      this.recomputeContextEstimate();
      this.scrollToBottom();
      this.scheduleConversationPersist();

      if (!InfringAPI.isWsConnected() && this.currentAgent) {
        this.connectWs(this.currentAgent.id);
        var wsWaitStarted = Date.now();
        while (!InfringAPI.isWsConnected() && (Date.now() - wsWaitStarted) < 1500) {
          await new Promise(function(resolve) { setTimeout(resolve, 75); });
        }
      }

      if (InfringAPI.wsSend({ type: 'terminal', command: command, cwd: this.terminalPromptPath })) {
        return;
      }

      try {
        var res = await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/terminal', {
          command: command,
          cwd: this.terminalPromptPath,
        });
        this.handleWsMessage({
          type: 'terminal_output',
          stdout: res && res.stdout ? String(res.stdout) : '',
          stderr: res && res.stderr ? String(res.stderr) : '',
          exit_code: Number(res && res.exit_code != null ? res.exit_code : 1),
          duration_ms: Number(res && res.duration_ms ? res.duration_ms : 0),
          cwd: res && res.cwd ? String(res.cwd) : this.terminalPromptPath,
        });
      } catch (e) {
        this.handleWsMessage({
          type: 'terminal_error',
          message: e && e.message ? e.message : 'command failed',
        });
      }
    },

    async _sendPayload(finalText, uploadedFiles, msgImages, options) {
      var opts = options && typeof options === 'object' ? options : {};
      this.sending = true;
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'typing');
      var targetAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var safeFiles = Array.isArray(uploadedFiles) ? uploadedFiles.slice() : [];
      var safeImages = Array.isArray(msgImages) ? msgImages.slice() : [];
      if (
        !opts.retry_from_failover ||
        !this._inflightPayload ||
        String(this._inflightPayload.agent_id || '') !== targetAgentId
      ) {
        this._inflightPayload = {
          agent_id: targetAgentId,
          final_text: String(finalText || ''),
          uploaded_files: safeFiles,
          msg_images: safeImages,
          failover_attempted: !!opts.retry_from_failover,
          created_at: Date.now()
        };
      } else {
        this._inflightPayload.final_text = String(finalText || '');
        this._inflightPayload.uploaded_files = safeFiles;
        this._inflightPayload.msg_images = safeImages;
        this._inflightPayload.retry_started_at = Date.now();
      }
      this._pendingAutoModelSwitchBaseline = this.captureAutoModelSwitchBaseline();
      var preflightRoute = await this.fetchAutoRoutePreflight(finalText, uploadedFiles);
      var preflightMeta = this.formatAutoRouteMeta(preflightRoute);
      if (preflightRoute) this.applyAutoRouteTelemetry({ auto_route: preflightRoute });

      // Try WebSocket first
      var wsPayload = { type: 'message', content: finalText };
      if (uploadedFiles && uploadedFiles.length) wsPayload.attachments = uploadedFiles;
      if (InfringAPI.wsSend(wsPayload)) {
        this._setPendingWsRequest(targetAgentId, finalText);
        this._responseStartedAt = Date.now();
        this.messages.push({
          id: ++msgId,
          role: 'agent',
          text: '',
          meta: preflightMeta || '',
          thinking: true,
          streaming: true,
          tools: [],
          ts: Date.now()
        });
        this.scrollToBottom();
        this.scheduleConversationPersist();
        return;
      }
      this._clearPendingWsRequest(targetAgentId);

      // HTTP fallback
      if (!InfringAPI.isWsConnected()) {
        InfringToast.info('Using HTTP mode (no streaming)');
      }
      this.messages.push({
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: preflightMeta || '',
        thinking: true,
        tools: [],
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
      var httpStartedAt = Date.now();
      var handedOffToRecovery = false;

      try {
        var httpBody = { message: finalText };
        if (uploadedFiles && uploadedFiles.length) httpBody.attachments = uploadedFiles;
        var httpAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();
        if (!httpAutoSwitchPrevious) httpAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
        var res = await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/message', httpBody);
        this.applyContextTelemetry(res);
        var httpRoute = this.applyAutoRouteTelemetry(res);
        this.messages = this.messages.filter(function(m) { return !m.thinking; });
        var httpMeta = (res.input_tokens || 0) + ' in / ' + (res.output_tokens || 0) + ' out';
        if (res.cost_usd != null) httpMeta += ' | $' + res.cost_usd.toFixed(4);
        if (res.iterations) httpMeta += ' | ' + res.iterations + ' iter';
        var httpDurationMs = Math.max(0, Date.now() - httpStartedAt);
        var httpDuration = this.formatResponseDuration(httpDurationMs);
        if (httpDuration) httpMeta += ' | ' + httpDuration;
        var httpRouteMeta = this.formatAutoRouteMeta(httpRoute || preflightRoute);
        if (httpRouteMeta) httpMeta += ' | ' + httpRouteMeta;
        var httpTools = Array.isArray(res.tools)
          ? res.tools.map(function(t, idx) {
              return {
                id: (t && t.id) || ('http-tool-' + Date.now() + '-' + idx),
                name: (t && t.name) || 'tool',
                running: false,
                expanded: false,
                input: (t && t.input) || '',
                result: (t && t.result) || '',
                is_error: !!(t && t.is_error),
              };
            })
          : [];
        var httpText = this.stripModelPrefix(this.sanitizeToolText(res.response || ''));
        var httpArtifactDirectives = this.extractArtifactDirectives(httpText);
        var httpSplit = this.extractThinkingLeak(httpText);
        if (httpSplit.thought) {
          httpTools.unshift(this.makeThoughtToolCard(httpSplit.thought, httpDurationMs));
          httpText = httpSplit.content || '';
        }
        httpText = this.stripArtifactDirectivesFromText(httpText);
        if (!String(httpText || '').trim()) {
          httpText = this.defaultAssistantFallback(httpSplit.thought || '', httpTools);
        }
        var httpFailure = this.extractRecoverableBackendFailure(httpText);
        if (httpFailure) {
          this._clearPendingWsRequest(targetAgentId);
          this._pendingAutoModelSwitchBaseline = '';
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          handedOffToRecovery = await this.attemptAutomaticFailoverRecovery('http_response', httpText, {
            remove_last_agent_failure: false
          });
          if (handedOffToRecovery) {
            this.scheduleConversationPersist();
            return;
          }
        }
        this.messages.push({
          id: ++msgId,
          role: 'agent',
          text: httpText,
          meta: httpMeta,
          tools: httpTools,
          ts: Date.now()
        });
        this.markAgentMessageComplete(this.messages[this.messages.length - 1]);
        this.maybeAddAutoModelSwitchNotice(httpAutoSwitchPrevious, httpRoute || preflightRoute);
        this._pendingAutoModelSwitchBaseline = '';
        this._clearPendingWsRequest(targetAgentId);
        this._inflightPayload = null;
        if (httpArtifactDirectives && httpArtifactDirectives.length) {
          this.resolveArtifactDirectives(httpArtifactDirectives);
        }
        this.scheduleConversationPersist();
      } catch(e) {
        this.messages = this.messages.filter(function(m) { return !m.thinking; });
        this._clearPendingWsRequest(targetAgentId);
        this._pendingAutoModelSwitchBaseline = '';
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._clearTypingTimeout();
        this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
        handedOffToRecovery = await this.attemptAutomaticFailoverRecovery(
          'http_error',
          e && e.message ? e.message : e,
          { remove_last_agent_failure: false }
        );
        if (!handedOffToRecovery) {
          this.messages.push({
            id: ++msgId,
            role: 'system',
            text: 'Error: ' + e.message,
            meta: '',
            tools: [],
            system_origin: 'http:error',
            ts: Date.now()
          });
          this._inflightPayload = null;
          this.scheduleConversationPersist();
        } else {
          return;
        }
      }
      if (handedOffToRecovery) return;
      this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
      this._responseStartedAt = 0;
      this.sending = false;
      this.scrollToBottom();
      // Process next queued message
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input'); if (el) el.focus();
        self._processQueue();
      });
    },

    // Stop the current agent run
    stopAgent: function() {
      if (!this.currentAgent) return;
      var self = this;
      InfringAPI.post('/api/agents/' + this.currentAgent.id + '/stop', {}).then(function(res) {
        self.handleStopResponse(self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '', res || {});
      }).catch(function(e) {
        var raw = String(e && e.message ? e.message : 'stop_failed');
        var lower = raw.toLowerCase();
        if (lower.indexOf('agent_inactive') >= 0 || lower.indexOf('inactive') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'inactive',
            { noticeText: 'Agent is now inactive.' }
          );
          return;
        }
        if (lower.indexOf('agent_contract_terminated') >= 0 || lower.indexOf('contract terminated') >= 0) {
          self.handleAgentInactive(
            self.currentAgent && self.currentAgent.id ? self.currentAgent.id : '',
            'contract_terminated',
            { noticeText: 'Agent contract terminated.' }
          );
          return;
        }
        InfringToast.error('Stop failed: ' + raw);
      });
    },

    killAgent() {
      if (!this.currentAgent) return;
      var self = this;
      var name = this.currentAgent.name;
      InfringToast.confirm('Stop Agent', 'Stop agent "' + name + '"? The agent will be shut down.', async function() {
        try {
          self.setAgentLiveActivity(self.currentAgent && self.currentAgent.id, 'idle');
          await InfringAPI.del('/api/agents/' + self.currentAgent.id);
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          InfringToast.success('Agent "' + name + '" stopped');
          Alpine.store('app').refreshAgents();
        } catch(e) {
          InfringToast.error('Failed to stop agent: ' + e.message);
        }
      });
    },

    _latexTimer: null,

    resolveMessagesScroller: function(preferred) {
      var candidate = preferred || null;
      if (candidate && candidate.id === 'messages' && candidate.offsetParent !== null) return candidate;
      var nodes = document.querySelectorAll('#messages');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (node && node.offsetParent !== null) return node;
      }
      return candidate && candidate.id === 'messages' ? candidate : null;
    },

    syncMapSelectionToScroll: function(container) {
      var el = this.resolveMessagesScroller(container);
      if (!el || !this.currentAgent || !Array.isArray(this.messages) || !this.messages.length) return;
      var nodes = el.querySelectorAll('.chat-message-block[id^="chat-msg-"]');
      if (!nodes || !nodes.length) return;
      var viewport = el.getBoundingClientRect();
      var viewportCenterY = viewport.top + (viewport.height / 2);
      var bestNode = null;
      var bestDiff = Number.POSITIVE_INFINITY;
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node || node.offsetParent === null) continue;
        var rect = node.getBoundingClientRect();
        if (rect.height <= 0) continue;
        if (rect.bottom < viewport.top || rect.top > viewport.bottom) continue;
        var nodeCenter = rect.top + (rect.height / 2);
        var diff = Math.abs(nodeCenter - viewportCenterY);
        if (diff < bestDiff) {
          bestDiff = diff;
          bestNode = node;
        }
      }
      if (!bestNode || !bestNode.id) return;
      var domId = String(bestNode.id);
      if (this.selectedMessageDomId !== domId) {
        this.selectedMessageDomId = domId;
      }
      if (!this.activeMapPreviewDomId) {
        this.hoveredMessageDomId = domId;
      }
      for (var idx = 0; idx < this.messages.length; idx++) {
        if (this.messageDomId(this.messages[idx], idx) === domId) {
          this.mapStepIndex = idx;
          break;
        }
      }
      this.centerChatMapOnMessage(domId, { immediate: true });
    },

    scrollToBottom() {
      var self = this;
      self.$nextTick(function() {
        self.scrollToBottomImmediate();
      });
    },

    scrollToBottomImmediate() {
      var el = this.resolveMessagesScroller();
      if (!el) return;
      el.scrollTop = el.scrollHeight;
      this.showScrollDown = false;
      this.syncMapSelectionToScroll(el);
      this.scheduleMessageRenderWindowUpdate(el);
      // Debounce LaTeX rendering to avoid running on every streaming token
      if (this._latexTimer) clearTimeout(this._latexTimer);
      this._latexTimer = setTimeout(function() { renderLatex(el); }, 150);
    },

    stabilizeBottomScroll: function() {
      var self = this;
      var tries = 3;
      var tick = function() {
        var el = self.resolveMessagesScroller();
        if (!el) return;
        el.scrollTop = el.scrollHeight;
        if (--tries > 0) {
          if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
          else setTimeout(tick, 16);
        }
      };
      if (typeof requestAnimationFrame === 'function') requestAnimationFrame(tick);
      else setTimeout(tick, 0);
    },

    handleMessagesScroll(e) {
      var el = this.resolveMessagesScroller(e && e.target ? e.target : null);
      if (!el) return;
      var hiddenBottom = el.scrollHeight - (el.scrollTop + el.clientHeight);
      this.showScrollDown = hiddenBottom > 120;
      var self = this;
      if (typeof requestAnimationFrame === 'function') {
        if (this._scrollSyncFrame) cancelAnimationFrame(this._scrollSyncFrame);
        this._scrollSyncFrame = requestAnimationFrame(function() {
          self._scrollSyncFrame = 0;
          self.syncMapSelectionToScroll(el);
        });
      } else {
        self.syncMapSelectionToScroll(el);
      }
      this.scheduleMessageRenderWindowUpdate(el);
    },

    addFiles(files) {
      var self = this;
      var allowed = ['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'text/plain', 'application/pdf',
                      'text/markdown', 'application/json', 'text/csv'];
      var allowedExts = ['.txt', '.pdf', '.md', '.json', '.csv'];
      for (var i = 0; i < files.length; i++) {
        var file = files[i];
        if (file.size > 10 * 1024 * 1024) {
          InfringToast.warn('File "' + file.name + '" exceeds 10MB limit');
          continue;
        }
        var typeOk = allowed.indexOf(file.type) !== -1;
        if (!typeOk) {
          var ext = file.name.lastIndexOf('.') !== -1 ? file.name.substring(file.name.lastIndexOf('.')).toLowerCase() : '';
          typeOk = allowedExts.indexOf(ext) !== -1 || file.type.startsWith('image/');
        }
        if (!typeOk) {
          InfringToast.warn('File type not supported: ' + file.name);
          continue;
        }
        var preview = null;
        if (file.type.startsWith('image/')) {
          preview = URL.createObjectURL(file);
        }
        self.attachments.push({ file: file, preview: preview, uploading: false });
      }
    },

    removeAttachment(idx) {
      var att = this.attachments[idx];
      if (att && att.preview) URL.revokeObjectURL(att.preview);
      this.attachments.splice(idx, 1);
    },

    handleDrop(e) {
      e.preventDefault();
      if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length) {
        this.addFiles(e.dataTransfer.files);
      }
    },

    showMessageTitle(msg, idx, rows) {
      if (!msg || msg.is_notice) return false;
      var role = String(msg.role || '').toLowerCase();
      if (role !== 'agent' && role !== 'system') return false;
      return this.isFirstInSourceRun(idx, rows);
    },

    isGrouped(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx <= 0 || idx >= list.length) return false;
      var prev = list[idx - 1];
      var curr = list[idx];
      if (!prev || !curr || prev.is_notice || curr.is_notice) return false;
      if (curr.thinking || prev.thinking) return false;
      return !this.isFirstInSourceRun(idx, list);
    },

    messageHasTailBlockingBox(msg) {
      if (!msg || typeof msg !== 'object') return false;
      if (this.messageHasTools(msg)) return true;
      if (msg.file_output && msg.file_output.path) return true;
      if (msg.folder_output && msg.folder_output.path) return true;
      if (this.messageProgress(msg)) return true;
      return false;
    },

    showMessageTail(msg, idx, rows) {
      if (!msg || msg.is_notice || msg.thinking) return false;
      var role = this.messageGroupRole(msg);
      if (role !== 'user' && role !== 'agent' && role !== 'system') return false;
      // Tail only shows when this bubble is the terminal visible item in its source run.
      if (this.messageHasTailBlockingBox(msg)) return false;
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return true;
      return this.isLastInSourceRun(idx, list);
    },

    // Strip raw function-call text that some models (Llama, Groq, etc.) leak into output.
    // These models don't use proper tool_use blocks — they output function calls as plain text.
    sanitizeToolText: function(text) {
      if (!text) return text;
      // Pattern: tool_name</function={"key":"value"} or tool_name</function,{...}
      text = text.replace(/\s*\w+<\/function[=,]?\s*\{[\s\S]*$/gm, '');
      // Pattern: <function=tool_name>{...}</function>
      text = text.replace(/<function=\w+>[\s\S]*?<\/function>/g, '');
      // Pattern: tool_name{"type":"function",...}
      text = text.replace(/\s*\w+\{"type"\s*:\s*"function"[\s\S]*$/gm, '');
      // Pattern: lone </function...> tags
      text = text.replace(/<\/function[^>]*>/g, '');
      // Pattern: <|python_tag|> or similar special tokens
      text = text.replace(/<\|[\w_]+\|>/g, '');
      return text.trim();
    },

    collectStreamedAssistantEnvelope: function() {
      var streamedText = '';
      var streamedTools = [];
      var streamedThought = '';
      var appendThought = function(value) {
        var clean = String(value || '').trim();
        if (!clean) return;
        if (streamedThought) streamedThought += '\n';
        streamedThought += clean;
      };
      var rows = Array.isArray(this.messages) ? this.messages : [];
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i];
        if (!row || row.role !== 'agent' || (!row.streaming && !row.thinking)) continue;
        if (!row.thinking) {
          streamedText += (typeof row._cleanText === 'string') ? row._cleanText : (row.text || '');
        }
        if (row._thoughtText) appendThought(row._thoughtText);
        if (row._reasoning) appendThought(row._reasoning);
        if (row.thinking && row.text) {
          var pendingThought = String(row.text || '').replace(/^\*+|\*+$/g, '').trim();
          if (pendingThought && pendingThought.toLowerCase() !== 'thinking...') appendThought(pendingThought);
        }
        streamedTools = streamedTools.concat(Array.isArray(row.tools) ? row.tools : []);
      }
      return {
        text: streamedText,
        tools: streamedTools,
        thought: String(streamedThought || '').trim()
      };
    },

    extractThinkingLeak: function(text) {
      if (!text) return { thought: '', content: '' };
      var raw = String(text).replace(/\r\n?/g, '\n');
      var trimmed = raw.replace(/^\s+/, '');
      if (!trimmed) return { thought: '', content: '' };
      var thinkingPrefix = /^(thinking(?:\s+out\s+loud)?(?:\.\.\.|:)?|analysis(?:\.\.\.|:)?|reasoning(?:\.\.\.|:)?)/i;
      if (!thinkingPrefix.test(trimmed)) return { thought: '', content: raw };
      var splitAt = this.findThinkingBoundary(trimmed);
      if (splitAt < 0) return { thought: trimmed.trim(), content: '' };
      return {
        thought: trimmed.slice(0, splitAt).trim(),
        content: trimmed.slice(splitAt).trim()
      };
    },

    findThinkingBoundary: function(text) {
      if (!text) return -1;
      var boundaries = [];
      var markers = [
        /\n\s*final answer\s*:/i,
        /\n\s*answer\s*:/i,
        /\n\s*response\s*:/i,
        /\n\s*output\s*:/i,
        /\n\s*```/i,
        /\n\s*\n(?=\s*[\{\[])/,
      ];
      markers.forEach(function(rx) {
        var match = text.match(rx);
        if (match && Number.isFinite(match.index)) {
          boundaries.push(match.index + 1);
        }
      });
      if (!boundaries.length) return -1;
      boundaries.sort(function(a, b) { return a - b; });
      return boundaries[0];
    },

    makeThoughtToolCard: function(thoughtText, durationMs) {
      var ms = Number(durationMs || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      return {
        id: 'thought-' + Date.now() + '-' + Math.floor(Math.random() * 10000),
        name: 'thought_process',
        running: false,
        expanded: false,
        input: String(thoughtText || '').trim(),
        result: '',
        is_error: false,
        duration_ms: ms
      };
    },

    appendThoughtChunk: function(base, chunk) {
      var prior = String(base || '').trim();
      var next = String(chunk || '').trim();
      if (!next) return prior;
      if (!prior) return next;
      if (prior.slice(-next.length) === next) return prior;
      var merged = prior + '\n' + next;
      if (merged.length > 8000) {
        merged = merged.slice(merged.length - 8000);
      }
      return merged;
    },

    renderLiveThoughtHtml: function(thoughtText) {
      var text = String(thoughtText || '').trim();
      return '<span class="thinking-live-inline"><em>' + escapeHtml(text) + '</em></span>';
    },

    defaultAssistantFallback: function(thoughtText, tools) {
      var thought = String(thoughtText || '').trim();
      var hasToolError = Array.isArray(tools) && tools.some(function(tool) {
        return !!(tool && tool.is_error);
      });
      if (hasToolError) {
        return 'I could not finish the request because a required step failed. Please clarify the goal or try again.';
      }
      if (thought) {
        return 'I need one more clarification before I can finalize a reliable answer. Tell me the exact expected outcome.';
      }
      return 'I could not produce a final answer this turn. Please retry or clarify what you want next.';
    },

    extractArtifactDirectives: function(text) {
      var value = String(text || '');
      if (!value) return [];
      var rx = /\[\[\s*(file|folder)\s*:\s*([^\]]+?)\s*\]\]/gi;
      var out = [];
      var match;
      while ((match = rx.exec(value)) && out.length < 4) {
        var kind = String(match[1] || '').toLowerCase();
        var targetPath = String(match[2] || '').trim();
        if (!targetPath) continue;
        out.push({ kind: kind, path: targetPath });
      }
      return out;
    },

    stripArtifactDirectivesFromText: function(text) {
      var value = String(text || '');
      if (!value) return '';
      return value.replace(/\[\[\s*(file|folder)\s*:\s*[^\]]+?\s*\]\]/gi, '').replace(/\n{3,}/g, '\n\n').trim();
    },

    resolveArtifactDirectives: async function(directives) {
      if (!this.currentAgent || !this.currentAgent.id) return;
      var rows = Array.isArray(directives) ? directives : [];
      if (!rows.length) return;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var targetPath = String(row.path || '').trim();
        if (!targetPath) continue;
        try {
          if (row.kind === 'file') {
            var fileRes = await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/file/read', {
              path: targetPath
            });
            var fileMeta = fileRes && fileRes.file ? fileRes.file : null;
            if (fileMeta && fileMeta.ok) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: (Number(fileMeta.bytes || 0) > 0 ? (Number(fileMeta.bytes || 0) + ' bytes') : ''),
                tools: [],
                ts: Date.now(),
                file_output: {
                  path: String(fileMeta.path || targetPath),
                  content: String(fileMeta.content || ''),
                  truncated: !!fileMeta.truncated,
                  bytes: Number(fileMeta.bytes || 0)
                }
              });
            }
          } else if (row.kind === 'folder') {
            var folderRes = await InfringAPI.post('/api/agents/' + this.currentAgent.id + '/folder/export', {
              path: targetPath
            });
            var folderMeta = folderRes && folderRes.folder ? folderRes.folder : null;
            var archiveMeta = folderRes && folderRes.archive ? folderRes.archive : null;
            if (folderMeta && folderMeta.ok) {
              this.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: Number(folderMeta.entries || 0) + ' entries',
                tools: [],
                ts: Date.now(),
                folder_output: {
                  path: String(folderMeta.path || targetPath),
                  tree: String(folderMeta.tree || ''),
                  entries: Number(folderMeta.entries || 0),
                  truncated: !!folderMeta.truncated,
                  download_url: archiveMeta && archiveMeta.download_url ? String(archiveMeta.download_url) : '',
                  archive_name: archiveMeta && archiveMeta.file_name ? String(archiveMeta.file_name) : '',
                  archive_bytes: Number(archiveMeta && archiveMeta.bytes ? archiveMeta.bytes : 0)
                }
              });
            }
          }
        } catch (_) {}
      }
      this.scrollToBottom();
      this.scheduleConversationPersist();
    },

    // Remove disclosure/speaker prefixes injected by model/backend responses.
    // Examples:
    //   "[openai/gpt-5] hello" -> "hello"
    //   "Agent: hello" -> "hello"
    //   "**Assistant:** hello" -> "hello"
    stripModelPrefix: function(text) {
      if (!text) return text;
      var out = String(text);
      for (var i = 0; i < 6; i++) {
        var prior = out;
        out = out.replace(/^\s*\[[^\]\n]{2,96}\]\s*/, '');
        // Strip leaked transcript wrappers like "User: ... Agent: <answer>".
        var transcriptLead = out.match(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:user|human|you)(?:\*\*)?\s*:\s*[\s\S]{0,1200}?(?:\*\*)?(?:agent|assistant|model|ai|jarvis)(?:\*\*)?\s*:\s*/i
        );
        if (transcriptLead && transcriptLead[0]) {
          out = out.slice(transcriptLead[0].length);
          continue;
        }
        out = out.replace(
          /^\s*(?:[-*]\s*)?(?:\*\*)?(?:agent|assistant|system|model|ai|jarvis|user|human|you)(?:\*\*)?\s*:\s*/i,
          ''
        );
        if (out === prior) break;
      }
      return out;
    },

    formatToolJson: function(text) {
      if (!text) return '';
      try { return JSON.stringify(JSON.parse(text), null, 2); }
      catch(e) { return text; }
    },

    // Voice: start recording
    startRecording: async function() {
      if (this.recording) return;
      try {
        var stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        var mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' :
                       MediaRecorder.isTypeSupported('audio/webm') ? 'audio/webm' : 'audio/ogg';
        this._audioChunks = [];
        this._mediaRecorder = new MediaRecorder(stream, { mimeType: mimeType });
        var self = this;
        this._mediaRecorder.ondataavailable = function(e) {
          if (e.data.size > 0) self._audioChunks.push(e.data);
        };
        this._mediaRecorder.onstop = function() {
          stream.getTracks().forEach(function(t) { t.stop(); });
          self._handleRecordingComplete();
        };
        this._mediaRecorder.start(250);
        this.recording = true;
        this.recordingTime = 0;
        this._recordingTimer = setInterval(function() { self.recordingTime++; }, 1000);
      } catch(e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Microphone access denied');
      }
    },

    // Voice: stop recording
    stopRecording: function() {
      if (!this.recording || !this._mediaRecorder) return;
      this._mediaRecorder.stop();
      this.recording = false;
      if (this._recordingTimer) { clearInterval(this._recordingTimer); this._recordingTimer = null; }
    },

    // Voice: handle completed recording — upload and transcribe
    _handleRecordingComplete: async function() {
      if (!this._audioChunks.length || !this.currentAgent) return;
      var blob = new Blob(this._audioChunks, { type: this._audioChunks[0].type || 'audio/webm' });
      this._audioChunks = [];
      if (blob.size < 100) return; // too small

      // Show a temporary "Transcribing..." message
      this.messages.push({ id: ++msgId, role: 'system', text: 'Transcribing audio...', thinking: true, ts: Date.now(), tools: [], system_origin: 'voice:transcribe' });
      this.scrollToBottom();

      try {
        // Upload audio file
        var ext = blob.type.includes('webm') ? 'webm' : blob.type.includes('ogg') ? 'ogg' : 'mp3';
        var file = new File([blob], 'voice_' + Date.now() + '.' + ext, { type: blob.type });
        var upload = await InfringAPI.upload(this.currentAgent.id, file);

        // Remove the "Transcribing..." message
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });

        // Use server-side transcription if available, otherwise fall back to placeholder
        var text = (upload.transcription && upload.transcription.trim())
          ? upload.transcription.trim()
          : '[Voice message - audio: ' + upload.filename + ']';
        this._sendPayload(text, [upload], []);
      } catch(e) {
        this.messages = this.messages.filter(function(m) { return !m.thinking || m.role !== 'system'; });
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to upload audio: ' + (e.message || 'unknown error'));
      }
    },

    // Voice: format recording time as MM:SS
    formatRecordingTime: function() {
      var m = Math.floor(this.recordingTime / 60);
      var s = this.recordingTime % 60;
      return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
    },

    // Search: toggle open/close
    toggleSearch: function() {
      this.searchOpen = !this.searchOpen;
      if (this.searchOpen) {
        var self = this;
        this.$nextTick(function() {
          var el = document.getElementById('chat-search-input');
          if (el) el.focus();
        });
      } else {
        this.searchQuery = '';
      }
    },

    // Search: filter messages by query
    get filteredMessages() {
      if (!this.searchQuery.trim()) return this.messages;
      var q = this.searchQuery.toLowerCase();
      return this.messages.filter(function(m) {
        return (m.text && m.text.toLowerCase().indexOf(q) !== -1) ||
               (m.tools && m.tools.some(function(t) { return t.name.toLowerCase().indexOf(q) !== -1; }));
      });
    },

    // Search: highlight matched text in a string
    highlightSearch: function(html) {
      if (!this.searchQuery.trim() || !html) return html;
      var q = this.searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      var regex = new RegExp('(' + q + ')', 'gi');
      return html.replace(regex, '<mark style="background:var(--warning);color:var(--bg);border-radius:2px;padding:0 2px">$1</mark>');
    },

    renderMarkdown: renderMarkdown,
    escapeHtml: escapeHtml
  };
}
