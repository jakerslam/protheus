
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
    async loadToolFilters() {
      var agentId = this.activeDetailAgentId();
      if (!agentId) return;
      var seq = Number(this._toolFiltersLoadSeq || 0) + 1;
      this._toolFiltersLoadSeq = seq;
      this.toolFiltersLoading = true;
      try {
        var filters = await InfringAPI.get('/api/agents/' + agentId + '/tools');
        if (seq !== Number(this._toolFiltersLoadSeq || 0) || agentId !== this.activeDetailAgentId()) return;
        this.toolFilters = {
          tool_allowlist: Array.isArray(filters && filters.tool_allowlist) ? filters.tool_allowlist.slice() : [],
          tool_blocklist: Array.isArray(filters && filters.tool_blocklist) ? filters.tool_blocklist.slice() : []
        };
      } catch(e) {
        if (seq !== Number(this._toolFiltersLoadSeq || 0)) return;
        this.toolFilters = { tool_allowlist: [], tool_blocklist: [] };
      } finally {
        if (seq === Number(this._toolFiltersLoadSeq || 0)) this.toolFiltersLoading = false;
      }
    },

    addAllowTool() {
      var t = this.newAllowTool.trim();
      if (t && this.toolFilters.tool_allowlist.indexOf(t) === -1) {
        this.toolFilters.tool_allowlist.push(t);
        this.newAllowTool = '';
        this.saveToolFilters();
      }
    },

    removeAllowTool(tool) {
      this.toolFilters.tool_allowlist = this.toolFilters.tool_allowlist.filter(function(t) { return t !== tool; });
      this.saveToolFilters();
    },

    addBlockTool() {
      var t = this.newBlockTool.trim();
      if (t && this.toolFilters.tool_blocklist.indexOf(t) === -1) {
        this.toolFilters.tool_blocklist.push(t);
        this.newBlockTool = '';
        this.saveToolFilters();
      }
    },

    removeBlockTool(tool) {
      this.toolFilters.tool_blocklist = this.toolFilters.tool_blocklist.filter(function(t) { return t !== tool; });
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
