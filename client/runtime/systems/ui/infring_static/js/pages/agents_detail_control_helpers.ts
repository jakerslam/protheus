// Agents page chat/detail/file/config control helpers.
'use strict';

function infringAgentsDetailControlMethods() {
  return {
    chatWithAgent(agent) {
      if (!agent) return;
      var pendingAgent = typeof this.normalizePendingAgent === 'function' ? this.normalizePendingAgent(agent) : agent;
      if (!pendingAgent) return;
      this.assignShellAppStore({ pendingAgent: pendingAgent });
      this.setActiveAgentIdViaShellStore(pendingAgent.id || null);
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
          await self.refreshAgentsViaShellStore();
          await self.loadLifecycle();
        } catch(e) {
          if (self.isAgentMissingError(e)) {
            InfringToast.success('Removed stale agent "' + (agent.name || agent.id) + '"');
            self.showDetailModal = false;
            await self.refreshAgentsViaShellStore();
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
        await self.refreshAgentsViaShellStore();
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
        await this.refreshAgentsViaShellStore();
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
      var store = this.shellAppStore();
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
        await this.refreshAgentsViaShellStore();
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
          await this.refreshAgentsViaShellStore();
          await this.loadLifecycle();
          this.showDetailModal = false;
        }
      } catch(e) {
        InfringToast.error('Clone failed: ' + e.message);
      }
    },

  };
}
