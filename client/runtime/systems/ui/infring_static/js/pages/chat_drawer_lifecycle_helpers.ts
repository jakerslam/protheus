// Chat agent drawer lifecycle and refresh helpers.
'use strict';

function infringChatDrawerLifecycleMethods() {
  return {
    async openAgentDrawer() {
      if (!this.currentAgent || !this.currentAgent.id) return;
      if (this.isSystemThreadAgent && this.isSystemThreadAgent(this.currentAgent)) return;
      if (this.isCurrentAgentArchived && this.isCurrentAgentArchived()) return;
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
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
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
        emoji: this.sanitizeAgentEmojiForDisplay
          ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer, (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '')
          : ((this.agentDrawer.identity && this.agentDrawer.identity.emoji) || ''),
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
          emoji: this.sanitizeAgentEmojiForDisplay
            ? this.sanitizeAgentEmojiForDisplay(this.agentDrawer, (this.agentDrawer.identity && this.agentDrawer.identity.emoji) || '')
            : ((this.agentDrawer.identity && this.agentDrawer.identity.emoji) || ''),
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
      this.drawerAvatarUrlPickerOpen = false;
      this.drawerAvatarUrlDraft = '';
      this.drawerAvatarUploadError = '';
    },

    toggleAgentDrawer() {
      if (this.isCurrentAgentArchived && this.isCurrentAgentArchived()) return;
      if (this.showAgentDrawer) {
        this.closeAgentDrawer();
        return;
      }
      this.openAgentDrawer();
    },

    async reviveCurrentArchivedAgent() {
      var agent = this.currentAgent && typeof this.currentAgent === 'object' ? this.currentAgent : null;
      if (!agent || !agent.id) return;
      if (!(this.isArchivedAgentRecord && this.isArchivedAgentRecord(agent))) return;
      var agentId = String(agent.id || '').trim();
      if (!agentId) return;
      try {
        await InfringAPI.post('/api/agents/' + encodeURIComponent(agentId) + '/revive', {
          role: String(agent.role || 'analyst')
        });
        this.currentAgent = Object.assign({}, agent, {
          archived: false,
          state: 'running'
        });
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
        if (store) {
          var nextAgents = null;
          if (Array.isArray(store.agents)) {
            nextAgents = store.agents.map(function(row) {
              if (!row || String((row && row.id) || '') !== agentId) return row;
              return Object.assign({}, row, { archived: false, state: 'running' });
            });
          }
          var pendingPatch = {};
          if (store.pendingAgent && String((store.pendingAgent && store.pendingAgent.id) || '') === agentId) {
            pendingPatch.pendingAgent = null;
            pendingPatch.pendingFreshAgentId = null;
          }
          var nextArchivedIds = null;
          if (Array.isArray(store.archivedAgentIds)) {
            nextArchivedIds = store.archivedAgentIds.filter(function(id) {
              return String(id || '') !== agentId;
            });
          }
          if (bridge && typeof bridge.assign === 'function') {
            var patch = Object.assign({}, pendingPatch);
            if (nextAgents) patch.agents = nextAgents;
            if (nextArchivedIds) patch.archivedAgentIds = nextArchivedIds;
            bridge.assign(patch);
          } else {
            if (nextAgents) store.agents = nextAgents;
            if (Object.prototype.hasOwnProperty.call(pendingPatch, 'pendingAgent')) store.pendingAgent = pendingPatch.pendingAgent;
            if (Object.prototype.hasOwnProperty.call(pendingPatch, 'pendingFreshAgentId')) store.pendingFreshAgentId = pendingPatch.pendingFreshAgentId;
            if (nextArchivedIds) store.archivedAgentIds = nextArchivedIds;
          }
          if (nextArchivedIds) {
            var persistArchivedAgentIds = bridge && typeof bridge.method === 'function'
              ? bridge.method('persistArchivedAgentIds')
              : null;
            if (typeof persistArchivedAgentIds === 'function') {
              persistArchivedAgentIds();
            } else {
              try {
                localStorage.setItem('infring-archived-agent-ids', JSON.stringify(nextArchivedIds));
              } catch(_) {}
            }
          }
          var setActiveAgentId = bridge && typeof bridge.method === 'function'
            ? bridge.method('setActiveAgentId')
            : null;
          if (typeof setActiveAgentId === 'function') setActiveAgentId(agentId);
          else if (bridge && typeof bridge.set === 'function') bridge.set('activeAgentId', agentId);
          else store.activeAgentId = agentId;
          var refreshAgents = bridge && typeof bridge.method === 'function'
            ? bridge.method('refreshAgents')
            : null;
          if (typeof refreshAgents === 'function') {
            await refreshAgents({ force: true });
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
        var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
          ? InfringSharedShellServices.appStore
          : null;
        var refreshAgents = bridge && typeof bridge.method === 'function'
          ? bridge.method('refreshAgents')
          : null;
        if (typeof refreshAgents === 'function') await refreshAgents();
      } catch {}
      var refreshed = this.resolveAgent(this.agentDrawer.id);
      if (refreshed) {
        this.currentAgent = refreshed;
      }
      await this.openAgentDrawer();
    },

  };
}
