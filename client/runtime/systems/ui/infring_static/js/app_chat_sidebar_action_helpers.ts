async function infringArchiveAgentFromSidebar(page, agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if (typeof page.isSidebarArchivedAgent === 'function' && page.isSidebarArchivedAgent(agent)) return;
      page.confirmArchiveAgentId = '';
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
      page.syncChatSidebarTopologyOrderFromAgents();
      var store = page.getAppStore();
      if (store.activeAgentId === agent.id) {
        var next = page.chatSidebarAgents.length ? page.chatSidebarAgents[0] : null;
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
      page.scheduleSidebarScrollIndicators();
}

async function infringCreateSidebarAgentChat(page) {
      if (page.sidebarSpawningAgent) return;
      page.confirmArchiveAgentId = '';
      page.sidebarSpawningAgent = true;
      try {
        var res = await InfringAPI.post('/api/agents', {
          role: 'analyst'
        });
        var createdId = String((res && (res.id || res.agent_id)) || '').trim();
        if (!createdId) throw new Error('spawn_failed');
        var store = page.getAppStore();
        if (!store || typeof store.refreshAgents !== 'function') throw new Error('app_store_unavailable');
        await store.refreshAgents({ force: true });
        var authoritative = null;
        if (Array.isArray(store.agents)) {
          for (var ai = 0; ai < store.agents.length; ai++) {
            var row = store.agents[ai];
            if (row && String((row && row.id) || '') === createdId) {
              authoritative = row;
              break;
            }
          }
        }
        if (!authoritative) {
          try {
            authoritative = await InfringAPI.get('/api/agents/' + encodeURIComponent(createdId));
          } catch(_) {}
        }
        var createdSource = authoritative && typeof authoritative === 'object'
          ? Object.assign({}, res || {}, authoritative)
          : (res && typeof res === 'object' ? Object.assign({}, res) : {});
        var createdStatusState = String((createdSource && createdSource.sidebar_status_state) || '').trim().toLowerCase();
        if (createdStatusState !== 'active' && createdStatusState !== 'idle' && createdStatusState !== 'offline') {
          createdStatusState = '';
        }
        var createdStatusLabel = String((createdSource && createdSource.sidebar_status_label) || '').trim().toLowerCase();
        if (createdStatusLabel !== 'active' && createdStatusLabel !== 'idle' && createdStatusLabel !== 'offline') {
          createdStatusLabel = createdStatusState;
        }
        var createdFreshness = {
          source: String((createdSource && createdSource.sidebar_status_source) || ''),
          source_sequence: String((createdSource && createdSource.sidebar_status_source_sequence) || ''),
          age_seconds: Number((createdSource && createdSource.sidebar_status_age_seconds) || 0),
          stale: !!(createdSource && createdSource.sidebar_status_stale === true)
        };
        var created = Object.assign({}, createdSource, {
          id: createdId,
          agent_id: createdId,
          name: String((createdSource && createdSource.name) || createdId),
          role: String((createdSource && createdSource.role) || 'analyst'),
          identity: (createdSource && createdSource.identity && typeof createdSource.identity === 'object') ? createdSource.identity : {},
          avatar_url: String((createdSource && createdSource.avatar_url) || ''),
          state: String((createdSource && createdSource.state) || createdStatusLabel || createdStatusState || 'Running'),
          sidebar_status_state: createdStatusState || 'active',
          sidebar_status_label: createdStatusLabel || createdStatusState || 'active',
          sidebar_status_source: createdFreshness.source,
          sidebar_status_source_sequence: createdFreshness.source_sequence,
          sidebar_status_age_seconds: createdFreshness.age_seconds,
          sidebar_status_stale: createdFreshness.stale,
          sidebar_status_freshness: createdFreshness,
          model_name: String((createdSource && (createdSource.model_name || createdSource.runtime_model || '')) || ''),
          model_provider: String((createdSource && createdSource.model_provider) || ''),
          runtime_model: String((createdSource && createdSource.runtime_model) || ''),
          created_at: String((createdSource && createdSource.created_at) || new Date().toISOString())
        });
        page.syncChatSidebarTopologyOrderFromAgents();
        store.pendingAgent = created;
        store.pendingFreshAgentId = created.id;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(created.id);
        else store.activeAgentId = created.id;
        page.navigate('chat');
        page.closeAgentChatsSidebar();
        InfringToast.success('Agent draft created. Complete initialization to launch.');
        page.scheduleSidebarScrollIndicators();
        // Keep draft agent hidden from rosters until launch completes.
      } catch(e) {
        InfringToast.error('Failed to create agent: ' + (e && e.message ? e.message : 'unknown error'));
      }
      page.sidebarSpawningAgent = false;
}

function infringSelectAgentChatFromSidebar(page, agent) {
      if (!agent || !agent.id) return;
      if (typeof page.hideDashboardPopupBySource === 'function') page.hideDashboardPopupBySource('sidebar');
      page.confirmArchiveAgentId = '';
      var quickAction = agent && agent._sidebar_quick_action && typeof agent._sidebar_quick_action === 'object' ? agent._sidebar_quick_action : null;
      if (quickAction) {
        var actionType = String(quickAction.type || '').trim().toLowerCase();
        if (actionType === 'copy_connect') {
          var checklist = 'Gateway connect checklist: open Settings, verify pairing or API token setup, and use HTTPS or localhost when device identity is required.';
          try { if (navigator && navigator.clipboard && typeof navigator.clipboard.writeText === 'function') navigator.clipboard.writeText(checklist).catch(function() {}); } catch(_) {}
          InfringToast.success('Copied connection checklist');
        }
        page.navigate(quickAction.page || 'chat');
        page.clearChatSidebarSearch();
        page.closeAgentChatsSidebar();
        page.scheduleSidebarScrollIndicators();
        return;
      }
      var store = page.getAppStore();
      var archived = typeof page.isSidebarArchivedAgent === 'function' && page.isSidebarArchivedAgent(agent);
      if (store && archived) {
        var pendingState = '';
        var rawSidebarStatusState = (typeof agent.sidebar_status_state === 'string')
          ? agent.sidebar_status_state
          : '';
        var rawSidebarStatusLabel = (typeof agent.sidebar_status_label === 'string')
          ? agent.sidebar_status_label
          : '';
        if (typeof page.agentStatusLabel === 'function') {
          pendingState = String(page.agentStatusLabel(agent) || '').trim().toLowerCase();
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
      page.navigate('chat');
      page.closeAgentChatsSidebar();
      page.scheduleSidebarScrollIndicators();
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
}

function infringFormatChatSidebarTime(page, ts) {
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
    }

function infringToggleAgentChatsSidebar(page) {
  if (page.sidebarCollapsed) {
    page.sidebarCollapsed = false;
    localStorage.setItem('infring-sidebar', 'expanded');
  }
  page.hideDashboardPopupBySource('sidebar');
  page.scheduleSidebarScrollIndicators();
}

function infringCloseAgentChatsSidebar(page) {
  if (page.chatSidebarMode !== 'default') {
    page.chatSidebarMode = 'default';
    page.chatSidebarQuery = '';
    page.clearChatSidebarSearch();
  }
  page.confirmArchiveAgentId = '';
  page.scheduleSidebarScrollIndicators();
}

async function infringApplyBootChatSelection(page) {
  if (page.bootSelectionApplied) return;
  var store = page.getAppStore();
  if (!store || store.agentsLoading || !store.agentsHydrated) {
    return;
  }
  var rows = Array.isArray(store.agents) ? store.agents.slice() : [];
  if (!rows.length) {
    page.bootSelectionApplied = true;
    if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
    else store.activeAgentId = null;
    page.navigate('chat');
    page.chatSidebarQuery = '';
    page.clearChatSidebarSearch();
    return;
  }
  var target = null;
  if (store.activeAgentId) {
    var saved = String(store.activeAgentId);
    target = rows.find(function(agent) { return agent && String(agent.id) === saved; }) || null;
  }
  if (!target) {
    rows.sort(function(a, b) {
      return page.chatSidebarSortComparator(a, b);
    });
    target = rows.length ? rows[0] : null;
  }
  if (target && target.id) {
    if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(target.id);
    else store.activeAgentId = target.id;
  }
  page.bootSelectionApplied = true;
  page.navigate('chat');
  page.closeAgentChatsSidebar();
}

function infringUpdateSidebarScrollIndicators(page) {
  var refs = page.$refs || {};
  var navState = page._computeScrollHintState(refs.sidebarNav);
  page.sidebarHasOverflowAbove = !!navState.above;
  page.sidebarHasOverflowBelow = !!navState.below;
  var chatState = page._computeScrollHintState(refs.chatSidebarList);
  page.chatSidebarHasOverflowAbove = !!chatState.above;
  page.chatSidebarHasOverflowBelow = !!chatState.below;
}

function infringScheduleSidebarScrollIndicators(page) {
  if (page._sidebarScrollIndicatorRaf) return;
  page._sidebarScrollIndicatorRaf = requestAnimationFrame(function() {
    page._sidebarScrollIndicatorRaf = 0;
    page.updateSidebarScrollIndicators();
    if (typeof page.maybeAnimateChatSidebarRows === 'function') {
      page.maybeAnimateChatSidebarRows();
    }
  });
}
