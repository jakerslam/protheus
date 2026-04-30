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
    ...infringAgentsTemplateState(),

    ...infringAgentsIdentityStateMethods(),

    ...infringAgentsViewStateMethods(),

    ...infringAgentsLifecycleArchiveMethods(),

    ...infringAgentsDetailControlMethods(),

    renderAgentPickerChips: function(host) {
      if (!host) return;
      var rows = Array.isArray(this.agents) ? this.agents : [];
      var self = this;
      while (host.firstChild) host.removeChild(host.firstChild);
      rows.forEach(function(agent) {
        if (!agent) return;
        var chip = document.createElement('div');
        chip.className = 'card agent-chip agent-chip-square agent-chip-square-active';
        chip.style.cursor = 'pointer';
        chip.addEventListener('click', function() { self.chatWithAgent(agent); });

        var avatar = document.createElement('div');
        avatar.style.width = '36px';
        avatar.style.height = '36px';
        avatar.style.borderRadius = '50%';
        avatar.style.background = 'var(--accent-subtle)';
        avatar.style.display = 'flex';
        avatar.style.alignItems = 'center';
        avatar.style.justifyContent = 'center';
        avatar.style.flexShrink = '0';
        if (agent.avatar_url) {
          var img = document.createElement('img');
          img.src = agent.avatar_url || '';
          img.alt = String(agent.name || agent.id || 'agent') + ' avatar';
          img.style.width = '100%';
          img.style.height = '100%';
          img.style.borderRadius = '50%';
          img.style.objectFit = 'cover';
          img.loading = 'lazy';
          avatar.appendChild(img);
        } else if (agent.identity && agent.identity.emoji) {
          var emoji = document.createElement('span');
          emoji.textContent = agent.identity.emoji;
          emoji.style.fontSize = '18px';
          avatar.appendChild(emoji);
        } else {
          var logo = document.createElement('span');
          logo.className = 'infring-logo infring-logo--agent-default';
          logo.setAttribute('aria-hidden', 'true');
          var glyph = document.createElement('span');
          glyph.className = 'infring-logo-glyph';
          glyph.setAttribute('aria-hidden', 'true');
          glyph.textContent = '\u221e';
          logo.appendChild(glyph);
          avatar.appendChild(logo);
        }
        chip.appendChild(avatar);

        var body = document.createElement('div');
        body.style.minWidth = '0';
        body.style.flex = '1';
        body.style.textAlign = 'center';
        body.style.width = '100%';
        var name = document.createElement('div');
        name.className = 'font-bold';
        name.style.fontSize = '13px';
        name.textContent = agent.name || '';
        body.appendChild(name);
        var model = document.createElement('div');
        model.className = 'text-xs text-dim font-mono';
        model.style.fontSize = '11px';
        model.textContent = agent.model_name || '';
        body.appendChild(model);
        var contractLine = this.formatAgentContractLine(agent);
        if (contractLine) {
          var contract = document.createElement('div');
          contract.className = 'text-xs text-dim font-mono';
          contract.style.fontSize = '10px';
          contract.textContent = contractLine;
          body.appendChild(contract);
        }
        chip.appendChild(body);

        var badge = document.createElement('span');
        var state = String(agent.state || '').trim();
        badge.className = 'badge badge-' + state.toLowerCase();
        badge.style.fontSize = '10px';
        badge.textContent = state;
        chip.appendChild(badge);
        host.appendChild(chip);
      }, this);
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
            await this.refreshAgentsViaShellStore();
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
        await this.refreshAgentsViaShellStore();
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
        await this.refreshAgentsViaShellStore();
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
          await this.refreshAgentsViaShellStore();
          await this.loadLifecycle();
          this.chatWithAgent({ id: res.agent_id, name: t.name, model_provider: t.provider, model_name: t.model });
        }
      } catch(e) {
        InfringToast.error('Failed to spawn agent: ' + e.message);
      }
    }
  };
}
