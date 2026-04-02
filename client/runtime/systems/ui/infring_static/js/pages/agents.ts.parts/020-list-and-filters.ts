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

    async loadLifecycle() {
      var firstLoad = !this.terminatedHydrated;
      var now = Date.now();
      var recentlyLoaded = Number(this._lifecycleLoadedAt || 0);
      if (!firstLoad && (now - recentlyLoaded) < 1200) return;
      this.lifecycleLoading = true;
      if (firstLoad) this.terminatedLoading = true;
      try {
        var snapshot = await InfringAPI.getDashboardSnapshot(this._dashboardSnapshotHash || '');
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
        }
        if (!lifecycle || !Array.isArray(lifecycle.terminated_recent) || lifecycle.terminated_recent.length === 0) {
          var terminated = await InfringAPI.get('/api/agents/terminated');
          if (terminated && Array.isArray(terminated.entries)) {
            this.agentLifecycle = {
              ...(this.agentLifecycle || {}),
              terminated_recent: terminated.entries,
            };
          }
        }
      } catch (e) {
        // keep last-known lifecycle state to avoid UI flicker
      } finally {
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
      var name = String(row.agent_name || row.name || agentId).trim();
      store.pendingAgent = {
        id: agentId,
        name: name || agentId,
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
      var ok = window.confirm('Are you sure you want to delete all archived agents? This cannot be undone');
      if (!ok) return;
      try {
        var result = await InfringAPI.del('/api/agents/terminated?all=1');
        var removed = Number(result && result.deleted_archived_agents || 0);
        InfringToast.success('Deleted ' + removed + ' archived agent(s).');
        await Alpine.store('app').refreshAgents();
        await this.loadLifecycle();
      } catch (e) {
        InfringToast.error('Failed to delete archived agents: ' + (e && e.message ? e.message : 'unknown_error'));
      }
    },

    async archiveAllAgents() {
      this.confirmArchiveAllAgents = false;
      var rows = Array.isArray(this.agents) ? this.agents.slice() : [];
      var targetIds = rows
        .map(function(row) { return String((row && row.id) || '').trim(); })
        .filter(function(id) { return !!id && id.toLowerCase() !== 'system'; });
      if (!targetIds.length) return;
      var ok = window.confirm('Are you sure you want to archive all agents?');
      if (!ok) return;
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
      await this.loadLifecycle();

      var self = this;
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
            if (!this.isAgentMissingError(e)) failures.push(survivorId);
          }
        }
        if (store && typeof store.refreshAgents === 'function') {
          await store.refreshAgents({ force: true });
        }
        await this.loadLifecycle();
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
      if (store.pendingAgent) {
        this.chatWithAgent(store.pendingAgent);
      }
      // Watch for future pendingAgent changes
      this.$watch('$store.app.pendingAgent', function(agent) {
        if (agent) {
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
      store.pendingAgent = agent;
      store.activeAgentId = agent.id || null;
      var messageCount = Number(agent && agent.message_count != null ? agent.message_count : 0);
      if (agent.id && Number.isFinite(messageCount) && messageCount <= 0) {
        store.pendingFreshAgentId = String(agent.id);
      }
      this.activeChatAgent = null;
      window.location.hash = 'chat';
    },

    closeChat() {
      this.activeChatAgent = null;
      InfringAPI.wsDisconnect();
    },

    async showDetail(agent) {
      this.detailAgent = agent;
      this.detailAgent._fallbacks = [];
      this.detailTab = 'info';
      this.agentFiles = [];
      this.editingFile = null;
      this.fileContent = '';
      this.editingFallback = false;
      this.newFallbackValue = '';
      this.configForm = {
        name: agent.name || '',
        system_prompt: agent.system_prompt || '',
        emoji: (agent.identity && agent.identity.emoji) || '',
        color: (agent.identity && agent.identity.color) || '#2563EB',
        archetype: (agent.identity && agent.identity.archetype) || '',
        vibe: (agent.identity && agent.identity.vibe) || ''
      };
      this.showDetailModal = true;
      // Fetch full agent detail to get fallback_models
      try {
        var full = await InfringAPI.get('/api/agents/' + agent.id);
        this.detailAgent._fallbacks = full.fallback_models || [];
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

    // ── Multi-step wizard navigation ──
    async openSpawnWizard() {
      this.showSpawnModal = true;
      this.spawnStep = 1;
      this.spawnMode = 'wizard';
      this.spawnIdentity = { emoji: '', color: '#2563EB', archetype: '' };
      this.selectedPreset = '';
      this.soulContent = '';
      this.spawnForm.name = '';
      this.spawnForm.provider = 'groq';
      this.spawnForm.model = 'llama-3.3-70b-versatile';
      this.spawnForm.systemPrompt = 'You are a helpful assistant.';
      this.spawnForm.profile = 'full';
      try {
        var res = await fetch('/api/status');
        if (res.ok) {
          var status = await res.json();
          if (status.default_provider) this.spawnForm.provider = status.default_provider;
          if (status.default_model) this.spawnForm.model = status.default_model;
        }
      } catch(e) { /* keep hardcoded defaults */ }
      var recentModel = this.mostRecentModelFromUsageCache();
      if (recentModel) {
        var parts = String(recentModel).split('/');
        if (parts.length > 1) {
          this.spawnForm.provider = parts[0] || this.spawnForm.provider;
          this.spawnForm.model = parts.slice(1).join('/') || this.spawnForm.model;
        } else {
          this.spawnForm.model = recentModel;
        }
      }
    },

    nextStep() {
      if (this.spawnStep === 1 && !this.spawnForm.name.trim()) {
        InfringToast.warn('Please enter an agent name');
        return;
      }
      if (this.spawnStep < 5) this.spawnStep++;
    },

    prevStep() {
      if (this.spawnStep > 1) this.spawnStep--;
    },

    selectPreset(preset) {
      this.selectedPreset = preset.id;
      this.soulContent = preset.soul;
    },

    generateToml() {
      var f = this.spawnForm;
      var si = this.spawnIdentity;
      var lines = [
        'name = "' + tomlBasicEscape(f.name) + '"',
        'module = "builtin:chat"'
      ];
      if (f.profile && f.profile !== 'custom') {
        lines.push('profile = "' + f.profile + '"');
      }
      lines.push('', '[model]');
      lines.push('provider = "' + f.provider + '"');
      lines.push('model = "' + f.model + '"');
      lines.push('system_prompt = """\n' + tomlMultilineEscape(f.systemPrompt) + '\n"""');
      if (f.profile === 'custom') {
        lines.push('', '[capabilities]');
        if (f.caps.memory_read) lines.push('memory_read = ["*"]');
        if (f.caps.memory_write) lines.push('memory_write = ["self.*"]');
        if (f.caps.network) lines.push('network = ["*"]');
        if (f.caps.shell) lines.push('shell = ["*"]');
        if (f.caps.agent_spawn) lines.push('agent_spawn = true');
      }
      return lines.join('\n');
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

    async spawnAgent() {
      this.spawning = true;
      var toml = this.spawnMode === 'wizard' ? this.generateToml() : this.spawnToml;
      if (!toml.trim()) {
        this.spawning = false;
        InfringToast.warn('Manifest is empty \u2014 enter agent config first');
        return;
      }

      try {
        var res = await InfringAPI.post('/api/agents', { manifest_toml: toml });
        if (res.agent_id) {
          // Post-spawn: update identity + write SOUL.md if personality preset selected
          var patchBody = {};
          if (this.spawnIdentity.emoji) patchBody.emoji = this.spawnIdentity.emoji;
          if (this.spawnIdentity.color) patchBody.color = this.spawnIdentity.color;
          if (this.spawnIdentity.archetype) patchBody.archetype = this.spawnIdentity.archetype;
          if (this.selectedPreset) patchBody.vibe = this.selectedPreset;

          if (Object.keys(patchBody).length) {
            InfringAPI.patch('/api/agents/' + res.agent_id + '/config', patchBody).catch(function(e) { console.warn('Post-spawn config patch failed:', e.message); });
          }
          if (this.soulContent.trim()) {
            InfringAPI.put('/api/agents/' + res.agent_id + '/files/SOUL.md', { content: '# Soul\n' + this.soulContent }).catch(function(e) { console.warn('SOUL.md write failed:', e.message); });
          }

          this.showSpawnModal = false;
          this.spawnForm.name = '';
          this.spawnToml = '';
          this.spawnStep = 1;
          var spawnedName = String((res && (res.name || res.agent_id)) || 'agent').trim() || 'agent';
          var spawnedRole = String((this.spawnForm && this.spawnForm.profile) || (this.spawnIdentity && this.spawnIdentity.archetype) || 'agent').trim() || 'agent';
          InfringToast.success('Launched ' + spawnedName + ' as ' + spawnedRole);
          await Alpine.store('app').refreshAgents();
          await this.loadLifecycle();
          this.chatWithAgent({ id: res.agent_id, name: res.name, model_provider: '?', model_name: '?' });
        } else {
          InfringToast.error('Spawn failed: ' + (res.error || 'Unknown error'));
        }
      } catch(e) {
        InfringToast.error('Failed to spawn agent: ' + e.message);
      }
      this.spawning = false;
    },
