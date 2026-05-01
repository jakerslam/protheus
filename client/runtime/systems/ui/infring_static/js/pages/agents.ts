// Infring Agents Page — detail view with tabs, file editor, lifecycle controls
'use strict';

function agentsPage() {
  return {
    tab: 'agents',
    activeChatAgent: null,
    // -- Agents state --
    showDetailModal: false,
    detailAgent: null,
    filterState: 'all',
    loading: true,
    loadError: '',
    lifecycleLoading: false,
    terminatedLoading: true,
    terminatedHydrated: false,
    confirmDeleteTerminatedKey: '',
    confirmDeleteAllArchived: false,
    confirmArchiveAllAgents: false,
    agentLifecycle: {
      active_agents: [],
      terminated_recent: [],
      idle_agents: 0,
      idle_threshold: 0,
      idle_alert: false,
      defaults: {
        default_expiry_seconds: 3600,
        auto_expire_on_complete: true,
        max_idle_agents: 5,
      },
    },
    _lifecycleTimer: null,
    _dashboardSnapshotHash: '',
    _lifecycleLoadedAt: 0,
    _lifecycleLoadSeq: 0,
    // -- Detail modal tabs --
    detailTab: 'info',
    agentFiles: [],
    editingFile: null,
    fileContent: '',
    fileSaving: false,
    filesLoading: false,
    configForm: {},
    configFormOriginal: {},
    configSaving: false,
    agentIdentityLoading: false,
    agentIdentityError: '',
    agentIdentityById: {},
    _agentFileBaseContents: {},
    _agentFileDrafts: {},
    _agentFilesLoadSeq: 0,
    _agentFileContentSeq: 0,
    // -- Tool filters --
    toolFilters: { tool_allowlist: [], tool_blocklist: [] },
    toolCatalog: [],
    toolFiltersLoading: false,
    _toolFiltersLoadSeq: 0,
    newAllowTool: '',
    newBlockTool: '',
    emojiOptions: [
      '\u{1F916}', '\u{1F4BB}', '\u{1F50D}', '\u{270D}\uFE0F', '\u{1F4CA}', '\u{1F6E0}\uFE0F',
      '\u{1F4AC}', '\u{1F393}', '\u{1F310}', '\u{1F512}', '\u{26A1}', '\u{1F680}',
      '\u{1F9EA}', '\u{1F3AF}', '\u{1F4D6}', '\u{1F9D1}\u200D\u{1F4BB}', '\u{1F4E7}', '\u{1F3E2}',
      '\u{2764}\uFE0F', '\u{1F31F}', '\u{1F527}', '\u{1F4DD}', '\u{1F4A1}', '\u{1F3A8}'
    ],
    archetypeOptions: ['Assistant', 'Researcher', 'Coder', 'Writer', 'DevOps', 'Support', 'Analyst', 'Custom'],
    // -- Model switch --
    editingModel: false,
    newModelValue: '',
    editingProvider: false,
    newProviderValue: '',
    modelSaving: false,
    // -- Fallback chain --
    editingFallback: false,
    newFallbackValue: '',

    // -- Templates state --
    tplTemplates: [],
    tplProviders: [],
    tplLoading: false,
    tplLoadError: '',
    selectedCategory: 'All',
    searchQuery: '',

    builtinTemplates: [
      {
        name: 'General Assistant',
        description: 'A versatile conversational agent that can help with everyday tasks, answer questions, and provide recommendations.',
        category: 'General',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        system_prompt: 'You are a helpful, friendly assistant. Provide clear, accurate, and concise responses. Ask clarifying questions when needed.'
      },
      {
        name: 'Code Helper',
        description: 'A programming-focused agent that writes, reviews, and debugs code across multiple languages.',
        category: 'Development',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        system_prompt: 'You are an expert programmer. Help users write clean, efficient code. Explain your reasoning. Follow best practices and conventions for the language being used.'
      },
      {
        name: 'Researcher',
        description: 'An analytical agent that breaks down complex topics, synthesizes information, and provides cited summaries.',
        category: 'Research',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'research',
        system_prompt: 'You are a research analyst. Break down complex topics into clear explanations. Provide structured analysis with key findings. Cite sources when available.'
      },
      {
        name: 'Writer',
        description: 'A creative writing agent that helps with drafting, editing, and improving written content of all kinds.',
        category: 'Writing',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        system_prompt: 'You are a skilled writer and editor. Help users create polished content. Adapt your tone and style to match the intended audience. Offer constructive suggestions for improvement.'
      },
      {
        name: 'Data Analyst',
        description: 'A data-focused agent that helps analyze datasets, create queries, and interpret statistical results.',
        category: 'Development',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        system_prompt: 'You are a data analysis expert. Help users understand their data, write SQL/Python queries, and interpret results. Present findings clearly with actionable insights.'
      },
      {
        name: 'DevOps Engineer',
        description: 'A systems-focused agent for CI/CD, infrastructure, Docker, and deployment troubleshooting.',
        category: 'Development',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'automation',
        system_prompt: 'You are a DevOps engineer. Help with CI/CD pipelines, Docker, Kubernetes, infrastructure as code, and deployment. Prioritize reliability and security.'
      },
      {
        name: 'Customer Support',
        description: 'A professional, empathetic agent for handling customer inquiries and resolving issues.',
        category: 'Business',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'messaging',
        system_prompt: 'You are a professional customer support representative. Be empathetic, patient, and solution-oriented. Acknowledge concerns before offering solutions. Escalate complex issues appropriately.'
      },
      {
        name: 'Tutor',
        description: 'A patient educational agent that explains concepts step-by-step and adapts to the learner\'s level.',
        category: 'General',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'full',
        system_prompt: 'You are a patient and encouraging tutor. Explain concepts step by step, starting from fundamentals. Use analogies and examples. Check understanding before moving on. Adapt to the learner\'s pace.'
      },
      {
        name: 'API Designer',
        description: 'An agent specialized in RESTful API design, OpenAPI specs, and integration architecture.',
        category: 'Development',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'coding',
        system_prompt: 'You are an API design expert. Help users design clean, consistent RESTful APIs following best practices. Cover endpoint naming, request/response schemas, error handling, and versioning.'
      },
      {
        name: 'Meeting Notes',
        description: 'Summarizes meeting transcripts into structured notes with action items and key decisions.',
        category: 'Business',
        provider: 'groq',
        model: 'llama-3.3-70b-versatile',
        profile: 'minimal',
        system_prompt: 'You are a meeting summarizer. When given a meeting transcript or notes, produce a structured summary with: key decisions, action items (with owners), discussion highlights, and follow-up questions.'
      }
    ],

    // ── Profile Descriptions ──
    profileDescriptions: {
      minimal: { label: 'Minimal', desc: 'Read-only file access' },
      coding: { label: 'Coding', desc: 'Files + shell + web fetch' },
      research: { label: 'Research', desc: 'Web search + file read/write' },
      messaging: { label: 'Messaging', desc: 'Agents + memory access' },
      automation: { label: 'Automation', desc: 'All tools except custom' },
      balanced: { label: 'Balanced', desc: 'General-purpose tool set' },
      precise: { label: 'Precise', desc: 'Focused tool set for accuracy' },
      creative: { label: 'Creative', desc: 'Full tools with creative emphasis' },
      full: { label: 'Full', desc: 'All 35+ tools' }
    },
    profileInfo: function(name) {
      return this.profileDescriptions[name] || { label: name, desc: '' };
    },

    mostRecentModelFromUsageCache() {
      try {
        var raw = localStorage.getItem('of-chat-model-usage-v1');
        if (!raw) return '';
        var parsed = JSON.parse(raw);
        if (!parsed || typeof parsed !== 'object') return '';
        var bestModel = '';
        var bestTs = 0;
        Object.keys(parsed).forEach(function(key) {
          var modelId = String(key || '').trim();
          if (!modelId) return;
          var ts = Number(parsed[key] || 0);
          if (!Number.isFinite(ts) || ts <= 0) return;
          if (ts > bestTs) {
            bestTs = ts;
            bestModel = modelId;
          }
        });
        return bestModel;
      } catch(_) {
        return '';
      }
    },

    isAgentMissingError(err) {
      var msg = String(err && err.message ? err.message : '').toLowerCase();
      return msg.indexOf('agent_not_found') >= 0 || msg.indexOf('agent_not_archived') >= 0;
    },
    rememberAgentIdentity(agent, extra) {
      var sourceAgent = agent && typeof agent === 'object' ? agent : {};
      var extraPayload = extra && typeof extra === 'object' ? extra : {};
      var agentId = String(extraPayload.id || extraPayload.agent_id || sourceAgent.id || sourceAgent.agent_id || '').trim();
      if (!agentId) return null;
      if (!this.agentIdentityById || typeof this.agentIdentityById !== 'object') this.agentIdentityById = {};
      var prior = this.agentIdentityById[agentId] && typeof this.agentIdentityById[agentId] === 'object'
        ? this.agentIdentityById[agentId]
        : {};
      var identitySource = Object.assign(
        {},
        sourceAgent.identity && typeof sourceAgent.identity === 'object' ? sourceAgent.identity : {},
        extraPayload.identity && typeof extraPayload.identity === 'object' ? extraPayload.identity : {},
        extraPayload
      );
      if (!identitySource.name) identitySource.name = extraPayload.agent_name || sourceAgent.agent_name || sourceAgent.name || '';
      var mergedSource = Object.assign({}, sourceAgent, extraPayload, {
        id: agentId,
        name: identitySource.name || sourceAgent.name || extraPayload.name || '',
        identity: identitySource
      });
      var next = Object.assign({}, prior, identitySource, { id: agentId });
      var label = normalizeDashboardOptionalString(mergedSource.agent_name) || normalizeDashboardAgentLabel(mergedSource, next);
      var avatarUrl = resolveDashboardAgentAvatar(mergedSource, next);
      var emoji = resolveDashboardAgentEmoji(mergedSource, next);
      if (label) next.name = label;
      if (avatarUrl) {
        next.avatar = avatarUrl;
        next.avatar_url = avatarUrl;
      }
      if (emoji) next.emoji = emoji;
      this.agentIdentityById[agentId] = next;
      return next;
    },

    captureDetailConfigForm(agent, full) {
      var baseAgent = agent && typeof agent === 'object' ? agent : {};
      var source = full && typeof full === 'object' ? full : baseAgent;
      var config = source.config && typeof source.config === 'object'
        ? cloneDashboardConfigObject(source.config)
        : {};
      var configIdentity = config.identity && typeof config.identity === 'object' ? config.identity : {};
      var identity = Object.assign(
        {},
        baseAgent.identity && typeof baseAgent.identity === 'object' ? baseAgent.identity : {},
        source.identity && typeof source.identity === 'object' ? source.identity : {},
        configIdentity
      );
      var nextForm = {
        name: normalizeDashboardOptionalString(source.name || baseAgent.name),
        system_prompt: normalizeDashboardOptionalString(source.system_prompt || config.system_prompt),
        emoji: normalizeDashboardOptionalString(identity.emoji || config.emoji),
        color: normalizeDashboardOptionalString(identity.color || config.color || '#2563EB') || '#2563EB',
        archetype: normalizeDashboardOptionalString(identity.archetype || config.archetype),
        vibe: normalizeDashboardOptionalString(identity.vibe || config.vibe)
      };
      this.configFormOriginal = cloneDashboardConfigObject(nextForm);
      this.configForm = cloneDashboardConfigObject(nextForm);
      return this.configForm;
    },
    resetConfigForm() {
      var original = this.configFormOriginal && typeof this.configFormOriginal === 'object'
        ? this.configFormOriginal
        : {};
      this.configForm = cloneDashboardConfigObject(original);
      return this.configForm;
    },

    normalizePendingAgent(agent) {
      var source = agent && typeof agent === 'object' ? agent : {};
      var agentId = String(source.id || source.agent_id || '').trim();
      if (!agentId) return null;
      var identity = this.rememberAgentIdentity(source, source) || {};
      var label = normalizeDashboardOptionalString(source.agent_name) || normalizeDashboardAgentLabel(source, identity);
      var avatarUrl = resolveDashboardAgentAvatar(source, identity);
      var emoji = resolveDashboardAgentEmoji(source, identity);
      var normalizedIdentity = Object.assign(
        {},
        source.identity && typeof source.identity === 'object' ? source.identity : {},
        identity
      );
      if (avatarUrl) normalizedIdentity.avatar_url = avatarUrl;
      if (emoji) normalizedIdentity.emoji = emoji;
      return Object.assign({}, source, {
        id: agentId,
        name: label || agentId,
        state: normalizeDashboardOptionalString(source.state) || (source.archived ? 'archived' : 'Running'),
        role: normalizeDashboardOptionalString(source.role) || 'analyst',
        avatar_url: avatarUrl || normalizeDashboardOptionalString(source.avatar_url),
        avatar: emoji || normalizeDashboardOptionalString(source.avatar),
        identity: normalizedIdentity
      });
    },

    get agents() {
      var store = Alpine.store('app');
      var rows = Array.isArray(store && store.agents) ? store.agents : [];
      var pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
      rows = rows.filter(function(agent) {
        if (!agent || !agent.id) return false;
        if (store && typeof store.isArchivedLikeAgent === 'function') {
          if (store.isArchivedLikeAgent(agent)) return false;
        } else {
          if (agent.archived === true) return false;
          var state = String(agent.state || '').trim().toLowerCase();
          if (state.indexOf('archived') >= 0 || state.indexOf('inactive') >= 0 || state.indexOf('terminated') >= 0) return false;
        }
        return true;
      });
      if (!pendingFreshId) return rows;
      return rows.filter(function(agent) {
        return String((agent && agent.id) || '').trim() !== pendingFreshId;
      });
    },

    get filteredAgents() {
      var f = this.filterState;
      if (f === 'all') return this.agents;
      return this.agents.filter(function(a) { return a.state.toLowerCase() === f; });
    },

    get runningCount() {
      return this.agents.filter(function(a) { return a.state === 'Running'; }).length;
    },

    get stoppedCount() {
      return this.agents.filter(function(a) { return a.state !== 'Running'; }).length;
    },

    get activeLifecycleAgents() {
      var rows = this.agentLifecycle && Array.isArray(this.agentLifecycle.active_agents)
        ? this.agentLifecycle.active_agents
        : [];
      return rows;
    },

    get terminatedAgents() {
      var rows = this.agentLifecycle && Array.isArray(this.agentLifecycle.terminated_recent)
        ? this.agentLifecycle.terminated_recent
        : [];
      return rows.slice(0, 20);
    },

    terminatedEntryKey(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      var agentId = String(row.agent_id || '').trim();
      var contractId = String(row.contract_id || '').trim();
      return agentId + '::' + contractId;
    },

    formatTerminatedReason(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      var raw = String(
        row.termination_reason
          || row.reason
          || row.archive_reason
          || row.inactive_reason
          || ''
      ).trim();
      if (!raw) return 'terminated';
      var token = raw.toLowerCase().replace(/[\s-]+/g, '_');
      if (token === 'parent_archived' || token === 'archived_by_parent_agent') {
        return 'Archived by parent agent';
      }
      if (token === 'user_archived' || token === 'user_archive' || token === 'user_archive_all') {
        return 'Archived by user';
      }
      if (token === 'archived') {
        return 'Archived';
      }
      if (token === 'contract_expired') {
        return 'Expired (contract)';
      }
      if (token === 'idle_timeout') {
        return 'Expired (idle timeout)';
      }
      if (token === 'stopped') {
        return 'Stopped by user';
      }
      if (token === 'contract_violation') {
        return 'Contract violation';
      }
      return String(raw)
        .replace(/[_-]+/g, ' ')
        .replace(/\b\w/g, function(ch) { return ch.toUpperCase(); });
    },

    setDeleteTerminatedConfirm(entry) {
      this.confirmDeleteTerminatedKey = this.terminatedEntryKey(entry);
    },

    clearDeleteTerminatedConfirm(entry) {
      var key = this.terminatedEntryKey(entry);
      if (this.confirmDeleteTerminatedKey === key) {
        this.confirmDeleteTerminatedKey = '';
      }
    },

    get idleAgentAlertText() {
      var idle = Number(this.agentLifecycle && this.agentLifecycle.idle_agents || 0);
      var threshold = Number(this.agentLifecycle && this.agentLifecycle.idle_threshold || 0);
      if (!threshold) return '';
      if (idle <= threshold) return '';
      return idle + ' idle agents above threshold ' + threshold;
    },

    // -- Templates computed --
    get categories() {
      var cats = { 'All': true };
      this.builtinTemplates.forEach(function(t) { cats[t.category] = true; });
      this.tplTemplates.forEach(function(t) { if (t.category) cats[t.category] = true; });
      return Object.keys(cats);
    },

    get filteredBuiltins() {
      var self = this;
      return this.builtinTemplates.filter(function(t) {
        if (self.selectedCategory !== 'All' && t.category !== self.selectedCategory) return false;
        if (self.searchQuery) {
          var q = self.searchQuery.toLowerCase();
          if (t.name.toLowerCase().indexOf(q) === -1 &&
              t.description.toLowerCase().indexOf(q) === -1) return false;
        }
        return true;
      });
    },

    get filteredCustom() {
      var self = this;
      return this.tplTemplates.filter(function(t) {
        if (self.searchQuery) {
          var q = self.searchQuery.toLowerCase();
          if ((t.name || '').toLowerCase().indexOf(q) === -1 &&
              (t.description || '').toLowerCase().indexOf(q) === -1) return false;
        }
        return true;
      });
    },

    isProviderConfigured(providerName) {
      if (!providerName) return false;
      var p = this.tplProviders.find(function(pr) { return pr.id === providerName; });
      return p ? p.auth_status === 'configured' : false;
    },

    contractForAgent(agent) {
      if (!agent || typeof agent !== 'object') return null;
      if (agent.contract && typeof agent.contract === 'object') return agent.contract;
      var id = String(agent.id || '');
      if (!id) return null;
      var rows = this.activeLifecycleAgents;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        if (String(row.id || '') !== id) continue;
        if (row.contract && typeof row.contract === 'object') return row.contract;
      }
      return null;
    },
    formatDurationMs(ms) {
      var raw = Number(ms || 0);
      if (!Number.isFinite(raw) || raw <= 0) return '0m';
      var totalMin = Math.max(1, Math.ceil(raw / 60000));
      var day = Math.floor(totalMin / 1440);
      var hour = Math.floor((totalMin % 1440) / 60);
      var min = totalMin % 60;
      var parts = [];
      if (day > 0) parts.push(day + 'd');
      if (hour > 0) parts.push(hour + 'h');
      parts.push(min + 'm');
      return parts.join(' ');
    },

    formatIsoTimestamp(value) {
      var ts = String(value || '').trim();
      if (!ts) return '';
      var ms = Date.parse(ts);
      if (!Number.isFinite(ms)) return '';
      return new Date(ms).toLocaleString();
    },
    formatAgentContractLine(agent) {
      var contract = this.contractForAgent(agent);
      if (!contract) return '';
      var status = String(contract.status || 'active').replace(/_/g, ' ');
      var remaining = contract.remaining_ms;
      if (remaining == null || remaining === '') {
        return 'contract ' + status;
      }
      return 'contract ' + status + ' · ' + this.formatDurationMs(remaining) + ' left';
    },

    setConfigFormPath(path, value) {
      var draft = cloneDashboardConfigObject(this.configForm && typeof this.configForm === 'object' ? this.configForm : {});
      setDashboardConfigPathValue(draft, path, value);
      this.configForm = draft;
      return this.configForm;
    },

    removeConfigFormPath(path) {
      var draft = cloneDashboardConfigObject(this.configForm && typeof this.configForm === 'object' ? this.configForm : {});
      removeDashboardConfigPathValue(draft, path);
      this.configForm = draft;
      return this.configForm;
    },

    async loadLifecycle() {
      var firstLoad = !this.terminatedHydrated;
      var now = Date.now();
      var recentlyLoaded = Number(this._lifecycleLoadedAt || 0);
      if (!firstLoad && (now - recentlyLoaded) < 1200) return;
      var seq = Number(this._lifecycleLoadSeq || 0) + 1;
      this._lifecycleLoadSeq = seq;
      this.lifecycleLoading = true;
      if (firstLoad) this.terminatedLoading = true;
      try {
        var snapshot = await InfringAPI.getDashboardSnapshot(this._dashboardSnapshotHash || '');
        if (seq !== Number(this._lifecycleLoadSeq || 0)) return;
        var snapshotHash = String(
          (snapshot && snapshot.sync && snapshot.sync.composite_checksum)
            || (snapshot && snapshot.sync && snapshot.sync.previous_composite_checksum)
            || ''
        ).trim();
        if (snapshotHash) this._dashboardSnapshotHash = snapshotHash;
        var lifecycle = snapshot && snapshot.agent_lifecycle && typeof snapshot.agent_lifecycle === 'object'
          ? snapshot.agent_lifecycle
          : null;
        if (lifecycle) {
          this.agentLifecycle = lifecycle;
          if (typeof this.rememberAgentIdentity === 'function') {
            var lifecycleRows = [];
            if (Array.isArray(lifecycle.active_agents)) lifecycleRows = lifecycleRows.concat(lifecycle.active_agents);
            if (Array.isArray(lifecycle.terminated_recent)) lifecycleRows = lifecycleRows.concat(lifecycle.terminated_recent);
            for (var i = 0; i < lifecycleRows.length; i += 1) this.rememberAgentIdentity(lifecycleRows[i], lifecycleRows[i]);
          }
        }
        if (!lifecycle || !Array.isArray(lifecycle.terminated_recent) || lifecycle.terminated_recent.length === 0) {
          var terminated = await InfringAPI.get('/api/agents/terminated');
          if (seq !== Number(this._lifecycleLoadSeq || 0)) return;
          if (terminated && Array.isArray(terminated.entries)) {
            this.agentLifecycle = {
              ...(this.agentLifecycle || {}),
              terminated_recent: terminated.entries,
            };
            if (typeof this.rememberAgentIdentity === 'function') {
              for (var ti = 0; ti < terminated.entries.length; ti += 1) {
                this.rememberAgentIdentity(terminated.entries[ti], terminated.entries[ti]);
              }
            }
          }
        }
      } catch (e) {
        // keep last-known lifecycle state to avoid UI flicker
      } finally {
        if (seq !== Number(this._lifecycleLoadSeq || 0)) return;
        this._lifecycleLoadedAt = Date.now();
        this.terminatedHydrated = true;
        this.terminatedLoading = false;
        this.lifecycleLoading = false;
      }
    },

    async reviveTerminated(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      var agentId = String(row.agent_id || '').trim();
      if (!agentId) return;
      try {
        await InfringAPI.post('/api/agents/' + encodeURIComponent(agentId) + '/revive', {
          role: row.role || 'analyst'
        });
        InfringToast.success('Revived ' + agentId);
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
      } catch (e) {
        InfringToast.error('Failed to revive ' + agentId + ': ' + (e && e.message ? e.message : 'unknown_error'));
      }
    },

    viewTerminated(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      var agentId = String(row.agent_id || '').trim();
      if (!agentId) return;
      var store = Alpine.store('app');
      if (!store) return;
      var pendingAgent = typeof this.normalizePendingAgent === 'function'
        ? this.normalizePendingAgent({
          id: agentId,
          agent_id: agentId,
          name: row.agent_name || row.name || agentId,
          agent_name: row.agent_name || row.name || agentId,
          state: 'archived',
          archived: true,
          role: String(row.role || 'analyst')
        })
        : {
          id: agentId,
          name: String(row.agent_name || row.name || agentId).trim() || agentId,
          state: 'archived',
          archived: true,
          role: String(row.role || 'analyst')
        };
      store.pendingAgent = pendingAgent || {
        id: agentId,
        name: agentId,
        state: 'archived',
        archived: true,
        role: String(row.role || 'analyst')
      };
      store.pendingFreshAgentId = null;
      if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(agentId);
      else store.activeAgentId = agentId;
      window.location.hash = 'chat';
    },

    async deleteTerminated(entry) {
      var row = entry && typeof entry === 'object' ? entry : {};
      var agentId = String(row.agent_id || '').trim();
      var contractId = String(row.contract_id || '').trim();
      if (!agentId) return;
      this.confirmDeleteTerminatedKey = '';
      try {
        var suffix = '/api/agents/terminated/' + encodeURIComponent(agentId);
        if (contractId) suffix += '?contract_id=' + encodeURIComponent(contractId);
        var result = await InfringAPI.del(suffix);
        var removed = Number(result && result.removed_history_entries || 0);
        var label = removed > 0 ? (' and ' + removed + ' archived record(s)') : '';
        InfringToast.success('Permanently deleted ' + agentId + label);
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
      } catch (e) {
        if (this.isAgentMissingError(e)) {
          InfringToast.success('Removed stale archived agent ' + agentId);
          await Alpine.store('app').refreshAgents();
          await this.loadLifecycle();
          return;
        }
        InfringToast.error('Failed to delete archived agent: ' + (e && e.message ? e.message : 'unknown_error'));
      }
    },

    async deleteAllArchived() {
      this.confirmDeleteAllArchived = false;
      var self = this;
      InfringToast.confirm(
        'Delete Archived Agents',
        'Permanently delete all archived agents? This cannot be undone.',
        async function() {
          try {
            var result = await InfringAPI.del('/api/agents/terminated?all=1');
            var removed = Number(result && result.deleted_archived_agents || 0);
            InfringToast.success('Deleted ' + removed + ' archived agent(s).');
            await Alpine.store('app').refreshAgents();
            await self.loadLifecycle();
          } catch (e) {
            InfringToast.error('Failed to delete archived agents: ' + (e && e.message ? e.message : 'unknown_error'));
          }
        }
      );
    },

    async archiveAllAgents() {
      this.confirmArchiveAllAgents = false;
      var rows = Array.isArray(this.agents) ? this.agents.slice() : [];
      var targetIds = rows
        .map(function(row) { return String((row && row.id) || '').trim(); })
        .filter(function(id) { return !!id && id.toLowerCase() !== 'system'; });
      if (!targetIds.length) return;
      var self = this;
      InfringToast.confirm(
        'Archive All Agents',
        'Archive ' + targetIds.length + ' active agent(s)?',
        async function() {
          var store = Alpine.store('app');
          var failures = [];
          try {
            await InfringAPI.post('/api/agents/archive-all', { reason: 'user_archive_all' });
          } catch (_) {
            // Fallback path below will sweep survivors one-by-one.
          }
          if (store && typeof store.refreshAgents === 'function') {
            await store.refreshAgents({ force: true });
          }
          await self.loadLifecycle();

          var survivors = targetIds.filter(function(id) {
            return Array.isArray(self.agents) && self.agents.some(function(row) {
              return String((row && row.id) || '').trim() === id;
            });
          });
          if (survivors.length) {
            for (var idx = 0; idx < survivors.length; idx += 1) {
              var survivorId = survivors[idx];
              try {
                await InfringAPI.del('/api/agents/' + encodeURIComponent(survivorId));
              } catch (e) {
                if (!self.isAgentMissingError(e)) failures.push(survivorId);
              }
            }
            if (store && typeof store.refreshAgents === 'function') {
              await store.refreshAgents({ force: true });
            }
            await self.loadLifecycle();
          }

          var unresolved = targetIds.filter(function(id) {
            return Array.isArray(self.agents) && self.agents.some(function(row) {
              return String((row && row.id) || '').trim() === id;
            });
          });
          for (var fi = 0; fi < failures.length; fi += 1) {
            if (unresolved.indexOf(failures[fi]) === -1) unresolved.push(failures[fi]);
          }
          if (unresolved.length) {
            InfringToast.error('Failed to archive: ' + unresolved.join(', '));
            return;
          }
          InfringToast.success('Archived ' + targetIds.length + ' agent(s).');
        }
      );
    },

    async init() {
      var self = this;
      this.loading = true;
      this.loadError = '';
      this.confirmDeleteTerminatedKey = '';
      this.confirmDeleteAllArchived = false;
      this.confirmArchiveAllAgents = false;
      try {
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
      } catch(e) {
        this.loadError = e.message || 'Could not load agents. Is the daemon running?';
      }
      this.loading = false;

      if (this._lifecycleTimer) clearInterval(this._lifecycleTimer);
      this._lifecycleTimer = setInterval(function() {
        self.loadLifecycle();
      }, 4000);

      // If a pending agent was set (e.g. from wizard or redirect), route to
      // the primary chat page so we keep one authoritative chat render path.
      var store = Alpine.store('app');
      var pendingFreshId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
      if (pendingFreshId) {
        store.pendingFreshAgentId = null;
        store.pendingAgent = null;
        if (String(store.activeAgentId || '').trim() === pendingFreshId) {
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
          else store.activeAgentId = null;
        }
        InfringAPI.del('/api/agents/' + encodeURIComponent(pendingFreshId)).catch(function() {});
        if (typeof store.refreshAgents === 'function') {
          setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
        }
      } else if (store.pendingAgent) {
        this.chatWithAgent(store.pendingAgent);
      }
      // Watch for future pendingAgent changes
      this.$watch('$store.app.pendingAgent', function(agent) {
        var pendingId = String(store && store.pendingFreshAgentId ? store.pendingFreshAgentId : '').trim();
        if (!pendingId && agent) {
          self.chatWithAgent(agent);
        }
      });
    },

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
      } catch(e) {
        this.loadError = e.message || 'Could not load agents.';
      }
      this.loading = false;
    },

    async loadTemplates() {
      this.tplLoading = true;
      this.tplLoadError = '';
      try {
        var results = await Promise.all([
          InfringAPI.get('/api/templates'),
          InfringAPI.get('/api/providers').catch(function() { return { providers: [] }; })
        ]);
        this.tplTemplates = results[0].templates || [];
        this.tplProviders = results[1].providers || [];
      } catch(e) {
        this.tplTemplates = [];
        this.tplLoadError = e.message || 'Could not load templates.';
      }
      this.tplLoading = false;
    },

    chatWithAgent(agent) {
      if (!agent) return;
      var store = Alpine.store('app');
      var pendingAgent = typeof this.normalizePendingAgent === 'function' ? this.normalizePendingAgent(agent) : agent;
      if (!pendingAgent) return;
      store.pendingAgent = pendingAgent;
      if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(pendingAgent.id || null);
      else store.activeAgentId = pendingAgent.id || null;
      this.activeChatAgent = null;
      window.location.hash = 'chat';
    },

    closeChat() {
      this.activeChatAgent = null;
      InfringAPI.wsDisconnect();
    },

    async showDetail(agent) {
      var normalizedAgent = typeof this.normalizePendingAgent === 'function' ? this.normalizePendingAgent(agent) : agent;
      if (!normalizedAgent) return;
      this.detailAgent = normalizedAgent;
      this.detailAgent._fallbacks = [];
      this.detailTab = 'info';
      this.agentFiles = [];
      this.editingFile = null;
      this.fileContent = '';
      this.editingFallback = false;
      this.newFallbackValue = '';
      if (typeof this.captureDetailConfigForm === 'function') this.captureDetailConfigForm(normalizedAgent, null);
      this.showDetailModal = true;
      // Fetch full agent detail to get fallback_models
      try {
        var agentId = String(normalizedAgent.id || '').trim();
        var full = await InfringAPI.get('/api/agents/' + agentId);
        if (agentId !== this.activeDetailAgentId()) return;
        var refreshed = typeof this.normalizePendingAgent === 'function'
          ? this.normalizePendingAgent(Object.assign({}, normalizedAgent, full || {}))
          : Object.assign({}, normalizedAgent, full || {});
        this.detailAgent = Object.assign({}, refreshed || normalizedAgent, {
          _fallbacks: Array.isArray(full && full.fallback_models) ? full.fallback_models : []
        });
        if (typeof this.captureDetailConfigForm === 'function') this.captureDetailConfigForm(this.detailAgent, full);
      } catch(e) { /* ignore */ }
    },

    killAgent(agent) {
      var self = this;
      InfringToast.confirm('Stop Agent', 'Stop agent "' + agent.name + '"? The agent will be shut down.', async function() {
        try {
          await InfringAPI.del('/api/agents/' + agent.id);
          InfringToast.success('Agent "' + agent.name + '" stopped');
          self.showDetailModal = false;
          await Alpine.store('app').refreshAgents();
          await self.loadLifecycle();
        } catch(e) {
          if (self.isAgentMissingError(e)) {
            InfringToast.success('Removed stale agent "' + (agent.name || agent.id) + '"');
            self.showDetailModal = false;
            await Alpine.store('app').refreshAgents();
            await self.loadLifecycle();
            return;
          }
          InfringToast.error('Failed to stop agent: ' + e.message);
        }
      });
    },

    killAllAgents() {
      var self = this;
      var list = this.filteredAgents;
      if (!list.length) return;
      InfringToast.confirm('Stop All Agents', 'Stop ' + list.length + ' agent(s)? All agents will be shut down.', async function() {
        var errors = [];
        for (var i = 0; i < list.length; i++) {
          try {
            await InfringAPI.del('/api/agents/' + list[i].id);
          } catch(e) {
            if (!self.isAgentMissingError(e)) errors.push(list[i].name + ': ' + e.message);
          }
        }
        await Alpine.store('app').refreshAgents();
        await self.loadLifecycle();
        if (errors.length) {
          InfringToast.error('Some agents failed to stop: ' + errors.join(', '));
        } else {
          InfringToast.success(list.length + ' agent(s) stopped');
        }
      });
    },

    async setMode(agent, mode) {
      try {
        await InfringAPI.put('/api/agents/' + agent.id + '/mode', { mode: mode });
        agent.mode = mode;
        InfringToast.success('Mode set to ' + mode);
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
      } catch(e) {
        InfringToast.error('Failed to set mode: ' + e.message);
      }
    },

    // ── Detail modal: Files tab ──
    activeDetailAgentId() {
      return String((this.detailAgent && this.detailAgent.id) || '').trim();
    },

    ensureAgentFileState() {
      if (!this._agentFileBaseContents || typeof this._agentFileBaseContents !== 'object') this._agentFileBaseContents = {};
      if (!this._agentFileDrafts || typeof this._agentFileDrafts !== 'object') this._agentFileDrafts = {};
      return { base: this._agentFileBaseContents, drafts: this._agentFileDrafts };
    },

    mergeAgentFileEntry(entry) {
      if (!entry || !entry.name) return;
      var name = String(entry.name);
      var found = false;
      this.agentFiles = (Array.isArray(this.agentFiles) ? this.agentFiles : []).map(function(file) {
        if (String((file && file.name) || '') !== name) return file;
        found = true;
        return Object.assign({}, file || {}, entry);
      });
      if (!found) this.agentFiles = this.agentFiles.concat([Object.assign({ exists: true }, entry)]);
    },

    syncDetailAgentFromStore() {
      var detailId = this.activeDetailAgentId();
      var store = Alpine.store('app');
      var agents = Array.isArray(store && store.agents) ? store.agents : [];
      for (var i = 0; i < agents.length; i += 1) {
        if (String((agents[i] && agents[i].id) || '').trim() !== detailId) continue;
        this.detailAgent = agents[i];
        return agents[i];
      }
      return null;
    },

    async loadAgentFiles() {
      var agentId = this.activeDetailAgentId();
      if (!agentId) return;
      var seq = Number(this._agentFilesLoadSeq || 0) + 1;
      this._agentFilesLoadSeq = seq;
      this.filesLoading = true;
      try {
        var data = await InfringAPI.get('/api/agents/' + agentId + '/files');
        if (seq !== Number(this._agentFilesLoadSeq || 0) || agentId !== this.activeDetailAgentId()) return;
        this.agentFiles = Array.isArray(data && data.files) ? data.files : [];
        if (this.editingFile && !this.agentFiles.some(function(file) { return String((file && file.name) || '') === String(this.editingFile || ''); }, this)) {
          this.closeFileEditor();
        }
      } catch(e) {
        if (seq !== Number(this._agentFilesLoadSeq || 0)) return;
        this.agentFiles = [];
        InfringToast.error('Failed to load files: ' + e.message);
      } finally {
        if (seq === Number(this._agentFilesLoadSeq || 0)) this.filesLoading = false;
      }
    },

    async openFile(file) {
      var agentId = this.activeDetailAgentId();
      if (!agentId || !file) return;
      var name = String(file.name || '').trim();
      if (!name) return;
      var fileState = this.ensureAgentFileState();
      if (!file.exists) {
        // Create with empty content
        this.editingFile = name;
        this.fileContent = Object.prototype.hasOwnProperty.call(fileState.drafts, name)
          ? String(fileState.drafts[name] || '')
          : '';
        return;
      }
      if (Object.prototype.hasOwnProperty.call(fileState.drafts, name)) {
        this.editingFile = name;
        this.fileContent = String(fileState.drafts[name] || '');
      }
      var seq = Number(this._agentFileContentSeq || 0) + 1;
      this._agentFileContentSeq = seq;
      try {
        var data = await InfringAPI.get('/api/agents/' + agentId + '/files/' + encodeURIComponent(name));
        if (seq !== Number(this._agentFileContentSeq || 0) || agentId !== this.activeDetailAgentId()) return;
        var nextContent = String((data && data.content) || '');
        var previousBase = Object.prototype.hasOwnProperty.call(fileState.base, name) ? String(fileState.base[name] || '') : '';
        var currentDraft = Object.prototype.hasOwnProperty.call(fileState.drafts, name) ? String(fileState.drafts[name] || '') : null;
        fileState.base[name] = nextContent;
        if (currentDraft == null || currentDraft === previousBase) fileState.drafts[name] = nextContent;
        this.editingFile = name;
        this.fileContent = Object.prototype.hasOwnProperty.call(fileState.drafts, name)
          ? String(fileState.drafts[name] || '')
          : nextContent;
      } catch(e) {
        InfringToast.error('Failed to read file: ' + e.message);
      }
    },

    async saveFile() {
      var agentId = this.activeDetailAgentId();
      if (!this.editingFile || !agentId) return;
      this.fileSaving = true;
      try {
        var fileState = this.ensureAgentFileState();
        var name = String(this.editingFile || '');
        var content = String(this.fileContent || '');
        await InfringAPI.put('/api/agents/' + agentId + '/files/' + encodeURIComponent(name), { content: content });
        fileState.base[name] = content;
        fileState.drafts[name] = content;
        this.mergeAgentFileEntry({ name: name, exists: true, updated_at: new Date().toISOString() });
        InfringToast.success(this.editingFile + ' saved');
        await this.loadAgentFiles();
      } catch(e) {
        InfringToast.error('Failed to save file: ' + e.message);
      }
      this.fileSaving = false;
    },

    closeFileEditor() {
      this.editingFile = null;
      this.fileContent = '';
    },

    // ── Detail modal: Config tab ──
    async saveConfig() {
      var agentId = this.activeDetailAgentId();
      if (!agentId) return;
      this.configSaving = true;
      try {
        await InfringAPI.patch('/api/agents/' + agentId + '/config', this.configForm);
        InfringToast.success('Config updated');
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
        this.syncDetailAgentFromStore();
      } catch(e) {
        InfringToast.error('Failed to save config: ' + e.message);
      }
      this.configSaving = false;
    },

    // ── Clone agent ──
    async cloneAgent(agent) {
      var newName = (agent.name || 'agent') + '-copy';
      try {
        var res = await InfringAPI.post('/api/agents/' + agent.id + '/clone', { new_name: newName });
        if (res.agent_id) {
          InfringToast.success('Cloned as "' + res.name + '"');
          await Alpine.store('app').refreshAgents();
          await this.loadLifecycle();
          this.showDetailModal = false;
        }
      } catch(e) {
        InfringToast.error('Clone failed: ' + e.message);
      }
    },

    // -- Template methods --
    async spawnFromTemplate(name) {
      try {
        var data = await InfringAPI.get('/api/templates/' + encodeURIComponent(name));
        if (data.manifest_toml) {
          var tpl = data && data.template && typeof data.template === 'object' ? data.template : {};
          var createPayload = { manifest_toml: data.manifest_toml };
          if (tpl.system_prompt) createPayload.system_prompt = String(tpl.system_prompt || '');
          var res = await InfringAPI.post('/api/agents', createPayload);
          if (res.agent_id) {
            var launchedName = String((res && (res.name || res.agent_id)) || name || 'agent').trim() || 'agent';
            var launchedRole = String(name || 'agent').trim() || 'agent';
            InfringToast.success('Launched ' + launchedName + ' as ' + launchedRole);
            await Alpine.store('app').refreshAgents();
            await this.loadLifecycle();
            this.chatWithAgent({ id: res.agent_id, name: res.name || name, model_provider: '?', model_name: '?' });
          }
        }
      } catch(e) {
        InfringToast.error('Failed to spawn from template: ' + e.message);
      }
    },

    // ── Clear agent history ──
    async clearHistory(agent) {
      var self = this;
      InfringToast.confirm('Clear History', 'Clear all conversation history for "' + agent.name + '"? This cannot be undone.', async function() {
        try {
          await InfringAPI.del('/api/agents/' + agent.id + '/history');
          InfringToast.success('History cleared for "' + agent.name + '"');
        } catch(e) {
          InfringToast.error('Failed to clear history: ' + e.message);
        }
      });
    },

    // ── Model switch ──
    async changeModel() {
      var agentId = this.activeDetailAgentId();
      if (!agentId || !this.newModelValue.trim()) return;
      this.modelSaving = true;
      try {
        var resp = await InfringAPI.put('/api/agents/' + agentId + '/model', { model: this.newModelValue.trim() });
        var providerInfo = (resp && resp.provider) ? ' (provider: ' + resp.provider + ')' : '';
        InfringToast.success('Model changed' + providerInfo + ' (memory reset)');
        this.editingModel = false;
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
        this.syncDetailAgentFromStore();
      } catch(e) {
        InfringToast.error('Failed to change model: ' + e.message);
      }
      this.modelSaving = false;
    },

    // ── Provider switch ──
    async changeProvider() {
      var agentId = this.activeDetailAgentId();
      if (!agentId || !this.newProviderValue.trim()) return;
      this.modelSaving = true;
      try {
        var combined = this.newProviderValue.trim() + '/' + this.detailAgent.model_name;
        var resp = await InfringAPI.put('/api/agents/' + agentId + '/model', { model: combined });
        InfringToast.success('Provider changed to ' + (resp && resp.provider ? resp.provider : this.newProviderValue.trim()));
        this.editingProvider = false;
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
        this.syncDetailAgentFromStore();
      } catch(e) {
        InfringToast.error('Failed to change provider: ' + e.message);
      }
      this.modelSaving = false;
    },

    // ── Fallback model chain ──
    async addFallback() {
      if (!this.detailAgent || !this.newFallbackValue.trim()) return;
      var parts = this.newFallbackValue.trim().split('/');
      var provider = parts.length > 1 ? parts[0] : this.detailAgent.model_provider;
      var model = parts.length > 1 ? parts.slice(1).join('/') : parts[0];
      if (!this.detailAgent._fallbacks) this.detailAgent._fallbacks = [];
      this.detailAgent._fallbacks.push({ provider: provider, model: model });
      try {
        await InfringAPI.patch('/api/agents/' + this.detailAgent.id + '/config', {
          fallback_models: this.detailAgent._fallbacks
        });
        InfringToast.success('Fallback added: ' + provider + '/' + model);
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        this.detailAgent._fallbacks.pop();
      }
      this.editingFallback = false;
      this.newFallbackValue = '';
    },

    async removeFallback(idx) {
      if (!this.detailAgent || !this.detailAgent._fallbacks) return;
      var removed = this.detailAgent._fallbacks.splice(idx, 1);
      try {
        await InfringAPI.patch('/api/agents/' + this.detailAgent.id + '/config', {
          fallback_models: this.detailAgent._fallbacks
        });
        InfringToast.success('Fallback removed');
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        this.detailAgent._fallbacks.splice(idx, 0, removed[0]);
      }
    },

    // ── Tool filters ──
    normalizeToolFilterName(value) {
      var raw = String(value || '').trim().toLowerCase();
      if (!raw) return '';
      return raw.replace(/[\s-]+/g, '_');
    },

    normalizeToolFilterList(list) {
      var rows = Array.isArray(list) ? list : [];
      var out = [];
      var seen = {};
      for (var i = 0; i < rows.length; i += 1) {
        var key = this.normalizeToolFilterName(rows[i]);
        if (!key || seen[key]) continue;
        seen[key] = true;
        out.push(key);
      }
      return out;
    },

    catalogToolIdsFromFilters(filters) {
      var payload = filters && typeof filters === 'object' ? filters : {};
      var variants = [];
      if (Array.isArray(payload.available_tools)) variants = variants.concat(payload.available_tools);
      if (Array.isArray(payload.tools)) variants = variants.concat(payload.tools);
      if (Array.isArray(payload.catalog)) variants = variants.concat(payload.catalog);
      var out = [];
      for (var i = 0; i < variants.length; i += 1) {
        var row = variants[i];
        if (typeof row === 'string') {
          out.push(row);
          continue;
        }
        if (!row || typeof row !== 'object') continue;
        out.push(
          row.id,
          row.name,
          row.tool,
          row.tool_name
        );
      }
      if (!out.length && this.detailAgent && Array.isArray(this.detailAgent.tools)) {
        out = out.concat(this.detailAgent.tools);
      }
      return this.normalizeToolFilterList(out);
    },

    async loadToolFilters() {
      var agentId = this.activeDetailAgentId();
      if (!agentId) return;
      var seq = Number(this._toolFiltersLoadSeq || 0) + 1;
      this._toolFiltersLoadSeq = seq;
      this.toolFiltersLoading = true;
      try {
        var filters = await InfringAPI.get('/api/agents/' + agentId + '/tools');
        if (seq !== Number(this._toolFiltersLoadSeq || 0) || agentId !== this.activeDetailAgentId()) return;
        var allowList = this.normalizeToolFilterList(filters && filters.tool_allowlist);
        var blockList = this.normalizeToolFilterList(filters && filters.tool_blocklist);
        this.toolFilters = {
          tool_allowlist: allowList,
          tool_blocklist: blockList
        };
        this.toolCatalog = this.catalogToolIdsFromFilters(filters);
      } catch(e) {
        if (seq !== Number(this._toolFiltersLoadSeq || 0)) return;
        this.toolFilters = { tool_allowlist: [], tool_blocklist: [] };
        this.toolCatalog = [];
      } finally {
        if (seq === Number(this._toolFiltersLoadSeq || 0)) this.toolFiltersLoading = false;
      }
    },

    addAllowTool() {
      var t = this.normalizeToolFilterName(this.newAllowTool);
      if (t && this.toolFilters.tool_allowlist.indexOf(t) === -1) {
        this.toolFilters.tool_allowlist.push(t);
        this.toolFilters.tool_blocklist = this.toolFilters.tool_blocklist.filter(function(item) { return item !== t; });
        this.newAllowTool = '';
        this.saveToolFilters();
      }
    },

    removeAllowTool(tool) {
      var normalized = this.normalizeToolFilterName(tool);
      this.toolFilters.tool_allowlist = this.toolFilters.tool_allowlist.filter(function(t) { return t !== normalized; });
      this.saveToolFilters();
    },

    addBlockTool() {
      var t = this.normalizeToolFilterName(this.newBlockTool);
      if (t && this.toolFilters.tool_blocklist.indexOf(t) === -1) {
        this.toolFilters.tool_blocklist.push(t);
        this.toolFilters.tool_allowlist = this.toolFilters.tool_allowlist.filter(function(item) { return item !== t; });
        this.newBlockTool = '';
        this.saveToolFilters();
      }
    },

    removeBlockTool(tool) {
      var normalized = this.normalizeToolFilterName(tool);
      this.toolFilters.tool_blocklist = this.toolFilters.tool_blocklist.filter(function(t) { return t !== normalized; });
      this.saveToolFilters();
    },

    enableAllCatalogTools() {
      var catalog = this.normalizeToolFilterList(this.toolCatalog);
      if (!catalog.length) {
        InfringToast.warn('No runtime tool catalog is available yet.');
        return;
      }
      this.toolFilters.tool_allowlist = catalog.slice();
      this.toolFilters.tool_blocklist = [];
      this.saveToolFilters();
    },

    disableAllCatalogTools() {
      var catalog = this.normalizeToolFilterList(this.toolCatalog);
      if (!catalog.length) {
        InfringToast.warn('No runtime tool catalog is available yet.');
        return;
      }
      this.toolFilters.tool_allowlist = [];
      this.toolFilters.tool_blocklist = catalog.slice();
      this.saveToolFilters();
    },

    resetToolFilters() {
      this.toolFilters.tool_allowlist = [];
      this.toolFilters.tool_blocklist = [];
      this.saveToolFilters();
    },

    async saveToolFilters() {
      if (!this.detailAgent) return;
      try {
        await InfringAPI.put('/api/agents/' + this.detailAgent.id + '/tools', this.toolFilters);
      } catch(e) {
        InfringToast.error('Failed to update tool filters: ' + e.message);
      }
    },

    async spawnBuiltin(t) {
      var toml = 'name = "' + tomlBasicEscape(t.name) + '"\n';
      toml += 'description = "' + tomlBasicEscape(t.description) + '"\n';
      toml += 'module = "builtin:chat"\n';
      toml += 'profile = "' + t.profile + '"\n\n';
      toml += '[model]\nprovider = "' + t.provider + '"\nmodel = "' + t.model + '"\n';
      toml += 'system_prompt = """\n' + tomlMultilineEscape(t.system_prompt) + '\n"""\n';

      try {
        var res = await InfringAPI.post('/api/agents', { manifest_toml: toml });
        if (res.agent_id) {
          var builtinName = String((res && (res.name || res.agent_id)) || t.name || 'agent').trim() || 'agent';
          var builtinRole = String((t && (t.profile || t.name)) || 'agent').trim() || 'agent';
          InfringToast.success('Launched ' + builtinName + ' as ' + builtinRole);
          await Alpine.store('app').refreshAgents();
          await this.loadLifecycle();
          this.chatWithAgent({ id: res.agent_id, name: t.name, model_provider: t.provider, model_name: t.model });
        }
      } catch(e) {
        InfringToast.error('Failed to spawn agent: ' + e.message);
      }
    }
  };
}
