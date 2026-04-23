// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          return this.chatSidebarSortComparator(a, b);
        }.bind(this));
        target = rows.length ? rows[0] : null;
      }
      if (target && target.id) {
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(target.id);
        else store.activeAgentId = target.id;
      }
      this.bootSelectionApplied = true;
      this.navigate('chat');
      this.closeAgentChatsSidebar();
    },
    sidebarAgentSortTs(agent) {
      if (!agent) return 0;
      var serverTs = Number(agent.sidebar_sort_ts);
      if (Number.isFinite(serverTs) && serverTs > 0) return Math.round(serverTs);
      return 0;
    },
    chatSidebarTopologyKey(agent) {
      if (!agent || !agent.id) return 'z|~~~~|';
      var serverKey = String(agent.sidebar_topology_key || '').trim().toLowerCase();
      if (serverKey) return serverKey;
      return 'z|' + String(agent.id || '').trim().toLowerCase();
    },
    chatSidebarSortComparator(a, b) {
      var mode = String(this.chatSidebarSortMode || '').toLowerCase();
      if (mode === 'topology') {
        var topoA = this.chatSidebarTopologyKey(a);
        var topoB = this.chatSidebarTopologyKey(b);
        if (topoA < topoB) return -1;
        if (topoA > topoB) return 1;
      }
      var byTs = this.sidebarAgentSortTs(b) - this.sidebarAgentSortTs(a);
      if (byTs !== 0) return byTs;
      var aName = String((a && (a.name || a.id)) || '').toLowerCase();
      var bName = String((b && (b.name || b.id)) || '').toLowerCase();
      if (aName < bName) return -1;
      if (aName > bName) return 1;
      return 0;
    },
    syncChatSidebarTopologyOrderFromAgents() {
      var self = this;
      var pool = (this.agents || []).filter(function(agent) {
        if (!agent || !agent.id) return false;
        return !(typeof self.isSidebarArchivedAgent === 'function' && self.isSidebarArchivedAgent(agent));
      });
      pool.sort(function(a, b) {
        return self.chatSidebarSortComparator(a, b);
      });
      var liveIds = pool.map(function(agent) { return String(agent.id); });
      var liveSet = new Set(liveIds);
      var seen = {};
      var prior = Array.isArray(this.chatSidebarTopologyOrder) ? this.chatSidebarTopologyOrder : [];
      var next = [];
      prior.forEach(function(id) {
        var key = String(id || '').trim();
        if (!key || seen[key] || !liveSet.has(key)) return;
        seen[key] = true;
        next.push(key);
      });
      liveIds.forEach(function(id) {
        if (seen[id]) return;
        seen[id] = true;
        next.push(id);
      });
      var changed = next.length !== prior.length;
      if (!changed) changed = next.some(function(id, idx) { return id !== String(prior[idx] || ''); });
      if (changed) {
        this.chatSidebarTopologyOrder = next;
        this.persistChatSidebarTopologyOrder();
      }
    },
    setChatSidebarSortMode(mode) {
      var normalized = String(mode || '').trim().toLowerCase() === 'topology' ? 'topology' : 'age';
      this.chatSidebarSortMode = normalized;
      if (normalized === 'topology' && typeof this.syncChatSidebarTopologyOrderFromAgents === 'function') {
        this.syncChatSidebarTopologyOrderFromAgents();
      } else if (typeof this.endChatSidebarTopologyDrag === 'function') {
        this.endChatSidebarTopologyDrag();
      }
      try {
        localStorage.setItem('infring-chat-sidebar-sort-mode', normalized);
      } catch(_) {}
      this.scheduleSidebarScrollIndicators();
    },
    chatSidebarPreview(agent) {
      if (!agent) return { text: 'No messages yet', ts: 0, role: 'agent', has_tools: false, tool_state: '', tool_label: '', unread_response: false };
      if (agent.revive_recommended === true) {
        return {
          text: 'Open chat to revive',
          ts: this.sidebarAgentSortTs(agent),
          role: 'agent',
          has_tools: false,
          tool_state: '',
          tool_label: '',
          unread_response: false
        };
      }
      var isSystemThread = agent.is_system_thread === true || String(agent.id || '').toLowerCase() === 'system';
      var fallbackText = isSystemThread ? '' : 'No messages yet'; if (typeof this._isCollapsedHoverStatePlaceholderText === 'function' && this._isCollapsedHoverStatePlaceholderText(fallbackText)) fallbackText = '';
      var store = this.getAppStore();
      var preview = store && typeof store.getAgentChatPreview === 'function' ? store.getAgentChatPreview(agent.id) : null;
      var serverPreview = agent && agent.sidebar_preview && typeof agent.sidebar_preview === 'object' ? agent.sidebar_preview : null;
      if (serverPreview && typeof serverPreview === 'object') {
        var serverText = String(serverPreview.text || '').trim();
        return {
          text: serverText || fallbackText,
          ts: Number(serverPreview.ts || this.sidebarAgentSortTs(agent)) || this.sidebarAgentSortTs(agent),
          role: String(serverPreview.role || 'assistant'),
          has_tools: !!serverPreview.has_tools,
          tool_state: String(serverPreview.tool_state || ''),
          tool_label: String(serverPreview.tool_label || ''),
          unread_response: !!(preview && preview.unread_response)
        };
      }
      if (isSystemThread) {
        return {
          text: '',
          ts: preview && preview.ts ? preview.ts : this.sidebarAgentSortTs(agent),
          role: 'agent',
          has_tools: !!(preview && preview.has_tools),
          tool_state: preview && preview.tool_state ? preview.tool_state : '',
          tool_label: preview && preview.tool_label ? preview.tool_label : '',
          unread_response: !!(preview && preview.unread_response)
        };
      }
      return { text: fallbackText, ts: this.sidebarAgentSortTs(agent), role: 'agent', has_tools: false, tool_state: '', tool_label: '', unread_response: false };
    },
    sidebarDisplayEmoji(agent) {
      if (!agent) return '';
      var isSystem = this.isSystemSidebarThread && this.isSystemSidebarThread(agent);
      if (isSystem) return '\u2699\ufe0f';
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      if (this.isReservedSystemEmoji && this.isReservedSystemEmoji(emoji)) return '';
      return emoji;
    },
    async archiveAgentFromSidebar(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if (typeof this.isSidebarArchivedAgent === 'function' && this.isSidebarArchivedAgent(agent)) return;
      this.confirmArchiveAgentId = '';
      var missingPurged = false;
      try {
        await InfringAPI.del('/api/agents/' + encodeURIComponent(agentId));
      } catch(e) {
        var msg = String(e && e.message ? e.message : '');
        if (msg.indexOf('agent_not_found') >= 0) {
          missingPurged = true;
        } else {
          InfringToast.error('Failed to archive agent: ' + (e && e.message ? e.message : 'unknown error'));
          return;
        }
      }
      this.syncChatSidebarTopologyOrderFromAgents();
      var store = this.getAppStore();
      if (store.activeAgentId === agent.id) {
        var next = this.chatSidebarAgents.length ? this.chatSidebarAgents[0] : null;
        if (next && next.id) {
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(next.id);
          else store.activeAgentId = next.id;
        } else {
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
          else store.activeAgentId = null;
        }
      }
      await store.refreshAgents();
      if (missingPurged) {
        InfringToast.success('Removed stale agent "' + (agent.name || agent.id) + '"');
      } else {
        InfringToast.success('Archived "' + (agent.name || agent.id) + '"');
      }
      this.scheduleSidebarScrollIndicators();
    },
    async createSidebarAgentChat() {
      if (this.sidebarSpawningAgent) return;
      this.confirmArchiveAgentId = '';
      this.sidebarSpawningAgent = true;
      try {
        var res = await InfringAPI.post('/api/agents', {
          role: 'analyst'
        });
        var createdId = String((res && (res.id || res.agent_id)) || '').trim();
        if (!createdId) throw new Error('spawn_failed');
        var createdStatusState = String((res && res.sidebar_status_state) || '').trim().toLowerCase();
        if (createdStatusState !== 'active' && createdStatusState !== 'idle' && createdStatusState !== 'offline') {
          createdStatusState = '';
        }
        var createdStatusLabel = String((res && res.sidebar_status_label) || '').trim().toLowerCase();
        if (createdStatusLabel !== 'active' && createdStatusLabel !== 'idle' && createdStatusLabel !== 'offline') {
          createdStatusLabel = createdStatusState;
        }
        var createdFreshness = {
          source: String((res && res.sidebar_status_source) || ''),
          source_sequence: String((res && res.sidebar_status_source_sequence) || ''),
          age_seconds: Number((res && res.sidebar_status_age_seconds) || 0),
          stale: !!(res && res.sidebar_status_stale === true)
        };
        var created = {
          id: createdId,
          name: String((res && res.name) || createdId),
          identity: (res && res.identity && typeof res.identity === 'object') ? res.identity : {},
          state: createdStatusLabel || createdStatusState || 'offline',
          sidebar_status_state: createdStatusState || 'offline',
          sidebar_status_label: createdStatusLabel || createdStatusState || 'offline',
          sidebar_status_source: createdFreshness.source,
          sidebar_status_source_sequence: createdFreshness.source_sequence,
          sidebar_status_age_seconds: createdFreshness.age_seconds,
          sidebar_status_stale: createdFreshness.stale,
          sidebar_status_freshness: createdFreshness,
          model_name: String((res && (res.model_name || res.runtime_model || '')) || ''),
          model_provider: String((res && res.model_provider) || ''),
          runtime_model: String((res && res.runtime_model) || ''),
          created_at: String((res && res.created_at) || new Date().toISOString())
        };
        var store = this.getAppStore();
        if (!store || typeof store.refreshAgents !== 'function') throw new Error('app_store_unavailable');
        this.syncChatSidebarTopologyOrderFromAgents();
        store.pendingAgent = created;
        store.pendingFreshAgentId = created.id;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(created.id);
        else store.activeAgentId = created.id;
        this.navigate('chat');
        this.closeAgentChatsSidebar();
        InfringToast.success('Agent draft created. Complete initialization to launch.');
        this.scheduleSidebarScrollIndicators();
        // Keep draft agent hidden from rosters until launch completes.
      } catch(e) {
        InfringToast.error('Failed to create agent: ' + (e && e.message ? e.message : 'unknown error'));
      }
      this.sidebarSpawningAgent = false;
    },
    selectAgentChatFromSidebar(agent) {
      if (!agent || !agent.id) return;
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      this.confirmArchiveAgentId = '';
      var quickAction = agent && agent._sidebar_quick_action && typeof agent._sidebar_quick_action === 'object' ? agent._sidebar_quick_action : null;
      if (quickAction) {
        var actionType = String(quickAction.type || '').trim().toLowerCase();
        if (actionType === 'copy_connect') {
          var checklist = 'Gateway connect checklist: open Settings, verify pairing or API token setup, and use HTTPS or localhost when device identity is required.';
          try { if (navigator && navigator.clipboard && typeof navigator.clipboard.writeText === 'function') navigator.clipboard.writeText(checklist).catch(function() {}); } catch(_) {}
          InfringToast.success('Copied connection checklist');
        }
        this.navigate(quickAction.page || 'chat');
        this.clearChatSidebarSearch();
        this.closeAgentChatsSidebar();
        this.scheduleSidebarScrollIndicators();
        return;
      }
      var store = this.getAppStore();
      var archived = typeof this.isSidebarArchivedAgent === 'function' && this.isSidebarArchivedAgent(agent);
      if (store && archived) {
        var pendingState = '';
        var rawSidebarStatusState = (typeof agent.sidebar_status_state === 'string')
          ? agent.sidebar_status_state
          : '';
        var rawSidebarStatusLabel = (typeof agent.sidebar_status_label === 'string')
          ? agent.sidebar_status_label
          : '';
        if (typeof this.agentStatusLabel === 'function') {
          pendingState = String(this.agentStatusLabel(agent) || '').trim().toLowerCase();
        }
        if (!pendingState) pendingState = 'offline';
        var pending = {
          id: String(agent.id),
          name: String(agent.name || agent.id),
          state: pendingState,
          archived: true,
          avatar_url: String(agent.avatar_url || '').trim(),
          sidebar_status_state: String(rawSidebarStatusState).trim().toLowerCase(),
          sidebar_status_label: String(rawSidebarStatusLabel).trim().toLowerCase(),
          sidebar_status_source: String(agent.sidebar_status_source || ''),
          sidebar_status_source_sequence: String(agent.sidebar_status_source_sequence || ''),
          sidebar_status_age_seconds: Number(agent.sidebar_status_age_seconds || 0),
          sidebar_status_stale: !!(agent.sidebar_status_stale === true),
          sidebar_status_freshness: agent.sidebar_status_freshness && typeof agent.sidebar_status_freshness === 'object'
            ? agent.sidebar_status_freshness
            : {
                source: String(agent.sidebar_status_source || ''),
                source_sequence: String(agent.sidebar_status_source_sequence || ''),
                age_seconds: Number(agent.sidebar_status_age_seconds || 0),
                stale: !!(agent.sidebar_status_stale === true)
              },
          identity: { emoji: String((agent.identity && agent.identity.emoji) || '') },
          role: String(agent.role || 'analyst')
        };
        store.pendingAgent = pending;
        store.pendingFreshAgentId = null;
      }
      if (store && typeof store.setActiveAgentId === 'function') store.setActiveAgentId(agent.id);
      else if (store) store.activeAgentId = agent.id;
      this.navigate('chat');
      this.closeAgentChatsSidebar();
      this.scheduleSidebarScrollIndicators();
      if (agent.revive_recommended === true) {
        var reviveId = String(agent.id || '').trim();
        if (reviveId) {
          InfringAPI.post('/api/agents/' + encodeURIComponent(reviveId) + '/revive', {
            reason: 'sidebar_contract_revival'
          }).then(function() {
            if (store && typeof store.refreshAgents === 'function') {
              store.refreshAgents({ force: true }).catch(function() {});
            }
          }).catch(function() {});
        }
      }
    },
    formatChatSidebarTime(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      var now = new Date();
      var sameDay = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
      if (sameDay) return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
      var y = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      var isYesterday = d.getFullYear() === y.getFullYear() && d.getMonth() === y.getMonth() && d.getDate() === y.getDate();
      if (isYesterday) return 'Yesterday';
      return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
    },
    agentAutoTerminateEnabled(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (typeof agent.auto_terminate_allowed === 'boolean') {
        return agent.auto_terminate_allowed;
      }
      // Server contract should provide explicit policy; default fail-closed.
      return false;
    },
    agentContractRemainingMs(agent) {
      // Force recompute every second for live countdown updates.
      var _tick = Number(this.clockTick || 0);
      void _tick;
      if (!this.agentAutoTerminateEnabled(agent)) return null;
      var store = this.getAppStore();
      var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
      var ageDriftMs =
        Number.isFinite(lastRefreshAt) && lastRefreshAt > 0
          ? Math.max(0, Date.now() - lastRefreshAt)
          : 0;
      if (!agent || typeof agent !== 'object') return null;
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) {
        return Math.max(0, Math.floor(directRemaining - ageDriftMs));
      }
      return null;
    },
    agentContractHasFiniteExpiry(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.revive_recommended === true) return true;
      if (typeof agent.contract_finite_expiry === 'boolean') {
        return agent.contract_finite_expiry;
      }
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) return true;
      var totalMs = Number(agent.contract_total_ms);
      return Number.isFinite(totalMs) && totalMs > 0;
    },
    agentContractTerminationGraceMs() {
      return 10000;
    },
    isAgentPendingTermination(agent) {
      if (!this.agentAutoTerminateEnabled(agent)) return false;
      if (!this.agentContractHasFiniteExpiry(agent)) return false;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null || remainingMs > 0) return false;
      var store = this.getAppStore();
      var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
      if (!Number.isFinite(lastRefreshAt) || lastRefreshAt <= 0) return true;
      var refreshAgeMs = Math.max(0, Date.now() - lastRefreshAt);
      return refreshAgeMs < this.agentContractTerminationGraceMs();
    },
    shouldShowInfinityLifespan(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.revive_recommended === true) return false;
      if (typeof agent.contract_finite_expiry === 'boolean') {
        if (agent.contract_finite_expiry) return false;
        return !this.agentAutoTerminateEnabled(agent);
      }
      if (!this.agentAutoTerminateEnabled(agent)) return true;
      // Unknown contract timing should not be rendered as explicit infinity.
      return false;
    },
    shouldShowExpiryCountdown(agent) {
      if (agent && agent.revive_recommended === true) return true;
      if (!this.agentAutoTerminateEnabled(agent)) return false;
      if (!this.agentContractHasFiniteExpiry(agent)) return false;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      if (remainingMs <= 0) return this.isAgentPendingTermination(agent);
      return true;
    },
    expiryCountdownLabel(agent) {
      if (agent && agent.revive_recommended === true) return 'timed out';
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return '';
