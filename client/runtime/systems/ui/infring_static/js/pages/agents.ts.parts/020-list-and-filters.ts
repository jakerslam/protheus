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
      store.pendingAgent = agent;
      store.activeAgentId = agent.id || null;
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
