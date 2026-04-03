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
      var store = this.getAppStore();
      var preview = store && typeof store.getAgentChatPreview === 'function'
        ? store.getAgentChatPreview(agent.id)
        : null;
      if (preview && preview.ts) return Number(preview.ts) || 0;
      if (agent.updated_at) return Number(new Date(agent.updated_at).getTime()) || 0;
      if (agent.created_at) return Number(new Date(agent.created_at).getTime()) || 0;
      return 0;
    },

    chatSidebarTopologyKey(agent) {
      if (!agent || !agent.id) return 'z|~~~~|';
      var treeKind = String(agent.git_tree_kind || '').trim().toLowerCase();
      var branch = String(
        agent.git_branch ||
        agent.branch ||
        agent.git_tree ||
        agent.tree ||
        ''
      ).trim().toLowerCase();
      var root = treeKind === 'main' || treeKind === 'master' || branch === 'main' || branch === 'master';
      var depthRaw = Number(
        agent.topology_depth != null
          ? agent.topology_depth
          : (agent.depth != null ? agent.depth : (root ? 0 : 1))
      );
      var depth = Number.isFinite(depthRaw) ? Math.max(0, Math.floor(depthRaw)) : (root ? 0 : 1);
      var depthKey = String(depth).padStart(4, '0');
      var branchKey = branch || String(agent.parent_agent_id || '').trim().toLowerCase() || String(agent.id || '').trim().toLowerCase();
      return (root ? '0' : '1') + '|' + depthKey + '|' + branchKey;
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
      var archivedSet = new Set((this.archivedAgentIds || []).map(function(id) { return String(id || ''); }));
      var pool = (this.agents || []).filter(function(agent) {
        if (!agent || !agent.id) return false;
        if (self.isSystemSidebarThread && self.isSystemSidebarThread(agent)) return false;
        return !archivedSet.has(String(agent.id));
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
      var isSystemThread = agent.is_system_thread === true || String(agent.id || '').toLowerCase() === 'system';
      var fallbackText = isSystemThread ? 'System events and terminal output' : 'No messages yet';
      if (typeof this._isCollapsedHoverStatePlaceholderText === 'function' && this._isCollapsedHoverStatePlaceholderText(fallbackText)) {
        fallbackText = '';
      }
      if (agent._sidebar_search_result) {
        var snippet = String(agent._sidebar_preview_text || '').trim();
        return {
          text: snippet || 'No matching text',
          ts: this.sidebarAgentSortTs(agent),
          role: 'agent',
          has_tools: false,
          tool_state: '',
          tool_label: '',
          unread_response: false
        };
      }
      var store = this.getAppStore();
      var preview = store && typeof store.getAgentChatPreview === 'function'
        ? store.getAgentChatPreview(agent.id)
        : null;
      if (!preview || !preview.text) return { text: fallbackText, ts: this.sidebarAgentSortTs(agent), role: 'agent', has_tools: false, tool_state: '', tool_label: '', unread_response: false };
      return preview;
    },

    sidebarDisplayEmoji(agent) {
      if (!agent) return '';
      var isSystem = this.isSystemSidebarThread && this.isSystemSidebarThread(agent);
      if (isSystem) return '\u2699\ufe0f';
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      if (this.isReservedSystemEmoji && this.isReservedSystemEmoji(emoji)) return '';
      return emoji;
    },

    persistArchivedAgentIds() {
      var seen = {};
      var out = [];
      (this.archivedAgentIds || []).forEach(function(id) {
        var key = String(id || '').trim();
        if (!key || seen[key]) return;
        seen[key] = true;
        out.push(key);
      });
      this.archivedAgentIds = out;
      try {
        localStorage.setItem('infring-archived-agent-ids', JSON.stringify(out));
      } catch(_) {}
    },

    reconcileArchivedAgentIdsWithLiveAgents() {
      var liveSet = new Set((this.agents || []).map(function(agent) {
        return String((agent && agent.id) || '');
      }).filter(Boolean));
      if (!liveSet.size || !Array.isArray(this.archivedAgentIds) || this.archivedAgentIds.length === 0) return;
      var next = this.archivedAgentIds.filter(function(id) {
        return !liveSet.has(String(id || ''));
      });
      if (next.length !== this.archivedAgentIds.length) {
        this.archivedAgentIds = next;
        this.persistArchivedAgentIds();
      }
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

    async archiveAgentFromSidebar(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if ((this.archivedAgentIds || []).indexOf(agentId) >= 0) return;
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
      this.archivedAgentIds = (this.archivedAgentIds || []).concat([agentId]);
      this.persistArchivedAgentIds();
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
          role: 'analyst',
          contract: {
            mission: 'Fresh chat initialization',
            termination_condition: 'task_or_timeout',
            expiry_seconds: 3600,
            auto_terminate_allowed: false,
            idle_terminate_allowed: false,
            conversation_hold: true
          }
        });
        var createdId = String((res && (res.id || res.agent_id)) || '').trim();
        if (!createdId) throw new Error('spawn_failed');
        var created = {
          id: createdId,
          name: createdId,
          identity: { emoji: '∞' },
          state: String((res && res.state) || 'running'),
          model_name: String((res && (res.model_name || res.runtime_model || '')) || ''),
          model_provider: String((res && res.model_provider) || ''),
          runtime_model: String((res && res.runtime_model) || ''),
          created_at: String((res && res.created_at) || new Date().toISOString())
        };
        var store = this.getAppStore();
        if (!store || typeof store.refreshAgents !== 'function') throw new Error('app_store_unavailable');

        this.archivedAgentIds = (this.archivedAgentIds || []).filter(function(id) { return String(id) !== createdId; });
        this.persistArchivedAgentIds();
        this.syncChatSidebarTopologyOrderFromAgents();
        store.pendingAgent = created;
        store.pendingFreshAgentId = created.id;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(created.id);
        else store.activeAgentId = created.id;
        this.navigate('chat');
        this.closeAgentChatsSidebar();
        InfringToast.success('Agent draft created. Complete initialization to launch.');
        this.scheduleSidebarScrollIndicators();

        var preferredModel = this.mostRecentModelFromUsageCache();
        (async function() {
          if (preferredModel) {
            try {
              await InfringAPI.put('/api/agents/' + encodeURIComponent(createdId) + '/model', {
                model: preferredModel
              });
            } catch(_) {
              // Keep default server model if model handoff fails.
            }
          }
          // Keep draft agent hidden from rosters until launch completes.
        })();
      } catch(e) {
        InfringToast.error('Failed to create agent: ' + (e && e.message ? e.message : 'unknown error'));
      }
      this.sidebarSpawningAgent = false;
    },

    selectAgentChatFromSidebar(agent) {
      if (!agent || !agent.id) return;
      if (typeof this.hideCollapsedAgentHover === 'function') this.hideCollapsedAgentHover();
      this.confirmArchiveAgentId = '';
      var store = this.getAppStore();
      var archived = agent.archived === true;
      if (store && archived) {
        var pending = {
          id: String(agent.id),
          name: String(agent.name || agent.id),
          state: String(agent.state || 'archived'),
          archived: true,
          avatar_url: String(agent.avatar_url || '').trim(),
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
      if (agent.auto_terminate_allowed === false) return false;
      if (agent.is_master_agent === true) return false;
      var treeKind = String(agent.git_tree_kind || '').trim().toLowerCase();
      if (treeKind === 'master' || treeKind === 'main') return false;
      var branch = String(agent.git_branch || agent.branch || '').trim().toLowerCase();
      if (branch === 'main' || branch === 'master') return false;
      var contract = (agent.contract && typeof agent.contract === 'object') ? agent.contract : null;
      if (contract && contract.auto_terminate_allowed === false) return false;
      return true;
    },

    agentContractRemainingMs(agent) {
      // Force recompute every second for live countdown updates.
      var _tick = Number(this.clockTick || 0);
      void _tick;
      if (!this.agentAutoTerminateEnabled(agent)) return null;
      var store = this.getAppStore();
      var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
      var ageDriftMs = Math.max(0, Date.now() - lastRefreshAt);
      if (!agent || typeof agent !== 'object') return null;
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) {
        return Math.max(0, Math.floor(directRemaining - ageDriftMs));
      }
      var contract = (agent.contract && typeof agent.contract === 'object') ? agent.contract : null;
      if (contract && contract.remaining_ms != null) {
        var remainingFromContract = Number(contract.remaining_ms);
        if (Number.isFinite(remainingFromContract) && remainingFromContract >= 0) {
          return Math.max(0, Math.floor(remainingFromContract - ageDriftMs));
        }
      }
      var expiresAt = String(
        agent.contract_expires_at ||
        (contract && contract.expires_at ? contract.expires_at : '') ||
        ''
      ).trim();
      if (!expiresAt) return null;
      var expiryTs = Number(new Date(expiresAt).getTime());
      if (!Number.isFinite(expiryTs) || expiryTs <= 0) return null;
      return Math.max(0, expiryTs - Date.now());
    },

    agentContractExpiryMs(agent) {
      if (!agent || typeof agent !== 'object') return 0;
      var contract = (agent.contract && typeof agent.contract === 'object') ? agent.contract : null;
      var expiresAt = String(
        agent.contract_expires_at ||
        (contract && contract.expires_at ? contract.expires_at : '') ||
        ''
      ).trim();
      if (!expiresAt) return 0;
      var expiryTs = Number(new Date(expiresAt).getTime());
      if (!Number.isFinite(expiryTs) || expiryTs <= 0) return 0;
      return expiryTs;
    },

    agentContractHasFiniteExpiry(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var contract = (agent.contract && typeof agent.contract === 'object') ? agent.contract : null;
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) return true;
      if (contract && contract.remaining_ms != null) {
        var remainingFromContract = Number(contract.remaining_ms);
        if (Number.isFinite(remainingFromContract) && remainingFromContract >= 0) return true;
      }
      return this.agentContractExpiryMs(agent) > 0;
    },

    agentContractTerminationGraceMs() {
      return 10000;
    },

    agentContractOverdueMs(agent) {
      if (!this.agentAutoTerminateEnabled(agent)) return null;
      var expiryTs = this.agentContractExpiryMs(agent);
      if (!expiryTs) return null;
      return Math.max(0, Date.now() - expiryTs);
    },

    isAgentPendingTermination(agent) {
      if (!this.agentAutoTerminateEnabled(agent)) return false;
      if (!this.agentContractHasFiniteExpiry(agent)) return false;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null || remainingMs > 0) return false;
      var overdueMs = this.agentContractOverdueMs(agent);
      if (overdueMs == null) return false;
      return overdueMs < this.agentContractTerminationGraceMs();
    },

    shouldShowInfinityLifespan(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (!this.agentAutoTerminateEnabled(agent)) return true;
      return !this.agentContractHasFiniteExpiry(agent);
    },

    shouldPulseExpiringAgent(agent) {
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      return remainingMs > 0 && remainingMs <= 3000;
    },

    shouldShowExpiryCountdown(agent) {
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      if (remainingMs <= 0) return this.isAgentPendingTermination(agent);
      return remainingMs <= 60000;
    },

    expiryCountdownLabel(agent) {
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return '';
