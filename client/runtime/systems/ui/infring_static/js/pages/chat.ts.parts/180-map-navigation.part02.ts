            await store.refreshAgents({ force: true });
          }
        }
        var resolved = this.resolveAgent(agentId);
        if (resolved) {
          this.currentAgent = Object.assign({}, resolved, {
            archived: false,
            state: String(resolved.state || 'running')
          });
        } else if (this.currentAgent && String((this.currentAgent && this.currentAgent.id) || '') === agentId) {
          this.currentAgent = Object.assign({}, this.currentAgent, { archived: false, state: 'running' });
        }
        this.showAgentDrawer = false;
        this.showFreshArchetypeTiles = false;
        await this.loadSessions(agentId);
        await this.loadSession(agentId, false);
        this.requestContextTelemetry(true);
        InfringToast.success('Revived ' + (resolved && (resolved.name || resolved.id) ? (resolved.name || resolved.id) : agentId));
      } catch (e) {
        InfringToast.error('Failed to revive archived agent: ' + (e && e.message ? e.message : 'unknown_error'));
      }
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
