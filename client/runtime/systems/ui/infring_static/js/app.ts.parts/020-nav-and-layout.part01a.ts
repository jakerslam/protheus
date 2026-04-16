          result.indexOf('approval') >= 0 ||
          result.indexOf('permission') >= 0 ||
          result.indexOf('fail-closed') >= 0;
        if (blocked) return 'warning';
        if (tool.is_error) return 'error';
        return 'success';
      };
      var summarizeTools = function(tools) {
        if (!Array.isArray(tools) || !tools.length) return { has_tools: false, tool_state: '', tool_label: '' };
        var state = 'success';
        for (var ti = 0; ti < tools.length; ti++) {
          var s = classifyTool(tools[ti]) || 'success';
          if ((toolStateRank[s] || 0) > (toolStateRank[state] || 0)) state = s;
        }
        var label = state === 'error'
          ? 'Tool error'
          : (state === 'warning' ? 'Tool warning' : 'Tool success');
        return { has_tools: true, tool_state: state, tool_label: label };
      };
      for (var i = list.length - 1; i >= 0; i--) {
        var msg = list[i] || {};
        var text = '';
        var toolInfo = summarizeTools(msg.tools);
        if (typeof msg.text === 'string' && msg.text.trim()) {
          text = msg.text.replace(/\s+/g, ' ').trim();
        } else if (Array.isArray(msg.tools) && msg.tools.length) {
          text = '[Processes] ' + msg.tools.map(function(tool) {
            return tool && tool.name ? tool.name : 'tool';
          }).join(', ');
        }
        if (text) {
          preview.text = text;
          preview.ts = Number(msg.ts || Date.now());
          preview.role = String(msg.role || 'agent');
          preview.has_tools = !!toolInfo.has_tools;
          preview.tool_state = toolInfo.tool_state || '';
          preview.tool_label = toolInfo.tool_label || '';
          break;
        }
      }
      if (preview.role === 'agent') {
        preview.unread_response = String(this.activeAgentId || '') !== previewKey;
      } else if (String(this.activeAgentId || '') === previewKey) {
        preview.unread_response = false;
      }
      var previewChanged = !!existingPreview && (
        Number(preview.ts || 0) > Number(existingPreview.ts || 0) ||
        String(preview.text || '') !== String(existingPreview.text || '') ||
        String(preview.role || '') !== String(existingPreview.role || '') ||
        String(preview.tool_state || '') !== String(existingPreview.tool_state || '')
      );
      var inactiveAgent = String(this.activeAgentId || '') !== previewKey;
      if (previewChanged && inactiveAgent && preview.role === 'agent' && String(preview.text || '').trim()) {
        var label = 'Agent';
        if (Array.isArray(this.agents)) {
          var found = this.agents.find(function(row) {
            return row && String(row.id || '') === previewKey;
          });
          if (found) {
            var foundName = String(found.name || '').trim();
            if (foundName) label = foundName;
          }
        }
        var compact = String(preview.text || '').replace(/\s+/g, ' ').trim();
        if (compact.length > 120) compact = compact.slice(0, 117) + '...';
        this.addNotification({
          type: 'info',
          message: label + ': ' + compact,
          agent_id: previewKey,
          page: 'chat',
          source: 'agent_preview',
          ts: Number(preview.ts || Date.now())
        });
      }
      this.agentChatPreviews[previewKey] = preview;
    },

    getAgentChatPreview(agentId) {
      if (!agentId) return null;
      return this.agentChatPreviews[String(agentId)] || null;
    },

    coerceAgentTimestamp(value) {
      if (value === null || typeof value === 'undefined' || value === '') return 0;
      if (typeof value === 'number') {
        if (!Number.isFinite(value)) return 0;
        return value < 1e12 ? Math.round(value * 1000) : Math.round(value);
      }
      var asNum = Number(value);
      if (Number.isFinite(asNum) && String(value).trim() !== '') {
        return asNum < 1e12 ? Math.round(asNum * 1000) : Math.round(asNum);
      }
      var asDate = Number(new Date(value).getTime());
      return Number.isFinite(asDate) ? asDate : 0;
    },

    agentLastActivityTs(agent) {
      if (!agent) return 0;
      var latest = 0;
      var keys = ['last_active_at', 'last_activity_at', 'last_message_at', 'last_seen_at', 'updated_at'];
      for (var i = 0; i < keys.length; i++) {
        var ts = this.coerceAgentTimestamp(agent[keys[i]]);
        if (ts > latest) latest = ts;
      }
      if (agent.id) {
        var preview = this.getAgentChatPreview(agent.id);
        var previewTs = this.coerceAgentTimestamp(preview && preview.ts);
        if (previewTs > latest) latest = previewTs;
      }
      return latest;
    },

    agentStatusState(agent) {
      if (!agent) return 'offline';
      var serverState = String(agent.sidebar_status_state || '').trim().toLowerCase();
      if (serverState === 'active' || serverState === 'idle' || serverState === 'offline') return serverState;
      return 'offline';
    },

    agentStatusLabel(agent) {
      var serverLabel = String(agent && agent.sidebar_status_label ? agent.sidebar_status_label : '').trim().toLowerCase();
      if (serverLabel === 'active' || serverLabel === 'idle' || serverLabel === 'offline') return serverLabel;
      var status = this.agentStatusState(agent);
      if (status === 'active') return 'active';
      if (status === 'idle') return 'idle';
      return 'offline';
    },

    setAgentLiveActivity(agentId, state) {
      var id = String(agentId || '').trim();
      if (!id) return;
      var normalized = String(state || '').trim().toLowerCase();
      if (!normalized || normalized === 'idle' || normalized === 'done' || normalized === 'stop' || normalized === 'stopped') {
        if (this.agentLiveActivity && Object.prototype.hasOwnProperty.call(this.agentLiveActivity, id)) {
          delete this.agentLiveActivity[id];
          this.agentLiveActivity = Object.assign({}, this.agentLiveActivity);
        }
        return;
      }
      this.agentLiveActivity = Object.assign({}, this.agentLiveActivity || {}, {
        [id]: { state: normalized, ts: Date.now() }
      });
    },

    clearAgentLiveActivity(agentId) {
      this.setAgentLiveActivity(agentId, 'idle');
    },

    isAgentLiveBusy(agent) {
      if (!agent || !agent.id) return false;
      var id = String(agent.id);
      var entry = this.agentLiveActivity ? this.agentLiveActivity[id] : null;
      if (entry) {
        var state = String(entry.state || '').toLowerCase();
        var ts = Number(entry.ts || 0);
        var busyState = state.indexOf('typing') >= 0 || state.indexOf('working') >= 0 || state.indexOf('processing') >= 0;
        // Allow longer-lived busy windows so long tool/reasoning phases keep
        // the avatar pulse visible until completion events clear the state.
        if (busyState && Number.isFinite(ts) && (Date.now() - ts) <= 180000) return true;
      }
      return false;
    },

    formatNotificationTime(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
    },

    clearApiKey() {
      InfringAPI.setAuthToken('');
      localStorage.removeItem('infring-api-key');
    }
  });
});

// Main app component
function app() {
  return {
    page: 'agents',
    themeMode: localStorage.getItem('infring-theme-mode') || 'system',
    overlayGlassTemplate: 'fogged-glass',
    theme: (() => {
      var mode = localStorage.getItem('infring-theme-mode') || 'system';
      if (mode === 'system') return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      return mode;
    })(),
    sidebarCollapsed: localStorage.getItem('infring-sidebar') === 'collapsed',
    mobileMenuOpen: false,
    chatSidebarMode: 'default',
    chatSidebarQuery: '',
    chatSidebarSearchResults: [],
    chatSidebarSearchLoading: false,
    chatSidebarSearchError: '',
    chatSidebarSearchSeq: 0,
    _chatSidebarSearchTimer: 0,
    agentChatsSectionCollapsed: false,
    chatSidebarSortMode: (() => {
      try {
        var saved = String(localStorage.getItem('infring-chat-sidebar-sort-mode') || '').trim().toLowerCase();
        return saved === 'topology' ? 'topology' : 'age';
      } catch(_) {
        return 'age';
      }
    })(),
    chatSidebarTopologyOrder: (() => {
      try {
        var raw = localStorage.getItem('infring-chat-sidebar-topology-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return [];
        return parsed.map(function(id) { return String(id || '').trim(); }).filter(Boolean);
      } catch(_) {
        return [];
      }
    })(),
    chatSidebarDragAgentId: '',
