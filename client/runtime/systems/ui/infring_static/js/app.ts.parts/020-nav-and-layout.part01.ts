
      // API key mode detection
      try {
        await InfringAPI.get('/api/tools');
        this.showAuthPrompt = false;
      } catch(e) {
        if (e.message && (e.message.indexOf('Not authorized') >= 0 || e.message.indexOf('401') >= 0 || e.message.indexOf('Missing Authorization') >= 0 || e.message.indexOf('Unauthorized') >= 0)) {
          var saved = localStorage.getItem('infring-api-key');
          if (saved) {
            InfringAPI.setAuthToken('');
            localStorage.removeItem('infring-api-key');
          }
          this.showAuthPrompt = true;
        }
      }
    },

    submitApiKey(key) {
      if (!key || !key.trim()) return;
      InfringAPI.setAuthToken(key.trim());
      localStorage.setItem('infring-api-key', key.trim());
      this.showAuthPrompt = false;
      this.refreshAgents();
    },

    async sessionLogin(username, password) {
      try {
        var result = await InfringAPI.post('/api/auth/login', { username: username, password: password });
        if (result.status === 'ok') {
          this.sessionUser = result.username;
          this.showAuthPrompt = false;
          this.refreshAgents();
        } else {
          InfringToast.error(result.error || 'Login failed');
        }
      } catch(e) {
        InfringToast.error(e.message || 'Login failed');
      }
    },

    async sessionLogout() {
      try {
        await InfringAPI.post('/api/auth/logout');
      } catch(e) { /* ignore */ }
      this.sessionUser = null;
      this.showAuthPrompt = true;
    },

    normalizeNotificationType(rawType, message) {
      var value = String(rawType || '').trim().toLowerCase();
      if (!value) {
        var text = String(message || '').toLowerCase();
        if (/(completed|complete|done|success|succeeded|finished|resolved)/.test(text)) {
          value = 'completed';
        } else if (/(error|failed|failure|aborted|abort|exception|crash|denied|timeout)/.test(text)) {
          value = 'error';
        } else {
          value = 'info';
        }
      }
      if (['completed', 'complete', 'done', 'success', 'ok', 'resolved', 'action_completed', 'task_completed'].indexOf(value) >= 0) {
        return 'completed';
      }
      if (['error', 'failed', 'failure', 'fatal', 'critical', 'danger', 'exception', 'aborted', 'abort', 'timeout'].indexOf(value) >= 0) {
        return 'error';
      }
      return 'info';
    },

    addNotification(payload) {
      var p = payload || {};
      var noteTs = Number(p.ts || Date.now());
      if (!Number.isFinite(noteTs) || noteTs <= 0) noteTs = Date.now();
      var noteMessage = String(p.message || '');
      var noteType = this.normalizeNotificationType(p.type, noteMessage);
      var noteAgentId = String(p.agent_id || p.agentId || '').trim();
      if (this.notifications && this.notifications.length) {
        var prior = this.notifications[0] || null;
        if (
          prior &&
          String(prior.message || '') === noteMessage &&
          String(prior.type || '') === noteType &&
          String(prior.agent_id || '') === noteAgentId &&
          Math.abs(noteTs - Number(prior.ts || 0)) <= 2200
        ) {
          return;
        }
      }
      var note = {
        id: p.id || ('notif-' + (++this._notificationSeq) + '-' + Date.now()),
        message: noteMessage,
        type: noteType,
        ts: noteTs,
        read: !!this.notificationsOpen,
        page: String(p.page || '').trim(),
        agent_id: noteAgentId,
        source: String(p.source || '').trim()
      };
      this.notifications.unshift(note);
      if (this.notifications.length > 150) this.notifications = this.notifications.slice(0, 150);
      this.unreadNotifications = this.notifications.filter(function(n) { return !n.read; }).length;
      this.showNotificationBubble(note);
    },

    showNotificationBubble(note) {
      var n = note || null;
      if (!n) return;
      this.notificationBubble = {
        id: n.id,
        message: n.message,
        type: n.type,
        ts: n.ts,
      };
      if (this._notificationBubbleTimer) clearTimeout(this._notificationBubbleTimer);
      var self = this;
      this._notificationBubbleTimer = setTimeout(function() {
        self.notificationBubble = null;
      }, 5200);
    },

    toggleNotifications() {
      this.notificationsOpen = !this.notificationsOpen;
      if (this.notificationsOpen) this.markAllNotificationsRead();
    },

    markNotificationRead(id) {
      this.notifications = this.notifications.map(function(n) {
        if (n.id === id) n.read = true;
        return n;
      });
      this.unreadNotifications = this.notifications.filter(function(n) { return !n.read; }).length;
    },

    markAllNotificationsRead() {
      this.notifications = this.notifications.map(function(n) {
        n.read = true;
        return n;
      });
      this.unreadNotifications = 0;
    },

    dismissNotification(id) {
      var targetId = String(id || '').trim();
      if (!targetId) return;
      this.notifications = this.notifications.filter(function(n) {
        return String(n && n.id ? n.id : '') !== targetId;
      });
      this.unreadNotifications = this.notifications.filter(function(n) { return !n.read; }).length;
      if (this.notificationBubble && String(this.notificationBubble.id || '') === targetId) {
        this.dismissNotificationBubble();
      }
    },

    clearNotifications() {
      this.notifications = [];
      this.notificationsOpen = false;
      this.unreadNotifications = 0;
      this.notificationBubble = null;
      if (this._notificationBubbleTimer) {
        clearTimeout(this._notificationBubbleTimer);
        this._notificationBubbleTimer = null;
      }
    },

    reopenNotification(note) {
      if (!note) return;
      this.markNotificationRead(note.id);
      this.showNotificationBubble(note);
      this.notificationsOpen = false;
      var targetAgentId = String(note.agent_id || '').trim();
      var targetPage = String(note.page || '').trim();
      if (targetAgentId) {
        if (typeof this.setActiveAgentId === 'function') {
          this.setActiveAgentId(targetAgentId);
        } else {
          this.activeAgentId = targetAgentId;
        }
      }
      if (targetPage) {
        window.location.hash = targetPage;
      } else if (targetAgentId) {
        window.location.hash = 'chat';
      }
    },

    dismissNotificationBubble() {
      this.notificationBubble = null;
      if (this._notificationBubbleTimer) {
        clearTimeout(this._notificationBubbleTimer);
        this._notificationBubbleTimer = null;
      }
    },

    saveAgentChatPreview(agentId, messages) {
      if (!agentId) return;
      var list = Array.isArray(messages) ? messages : [];
      var previewKey = String(agentId);
      var existingPreview = this.agentChatPreviews && this.agentChatPreviews[previewKey]
        ? this.agentChatPreviews[previewKey]
        : null;
      var preview = {
        text: '',
        ts: Date.now(),
        role: 'agent',
        has_tools: false,
        tool_state: '',
        tool_label: '',
        unread_response: !!(existingPreview && existingPreview.unread_response)
      };
      var toolStateRank = { success: 1, warning: 2, error: 3 };
      var classifyTool = function(tool) {
        if (!tool) return '';
        if (tool.running) return 'warning';
        var status = String(tool.status || '').toLowerCase();
        var result = String(tool.result || '').toLowerCase();
        var blocked = tool.blocked === true || status === 'blocked' ||
          result.indexOf('blocked') >= 0 ||
          result.indexOf('policy') >= 0 ||
          result.indexOf('denied') >= 0 ||
          result.indexOf('not allowed') >= 0 ||
          result.indexOf('forbidden') >= 0 ||
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
      var state = String(agent.state || '').toLowerCase();
      var offlineHints = ['offline', 'archived', 'archive', 'terminated', 'stopped', 'crashed', 'error', 'failed', 'dead', 'disabled'];
      for (var i = 0; i < offlineHints.length; i++) {
        if (state.indexOf(offlineHints[i]) >= 0) return 'offline';
      }
      var ts = this.agentLastActivityTs(agent);
      if (ts > 0) {
        var ageMinutes = (Date.now() - ts) / 60000;
        if (ageMinutes <= 10) return 'active';
        if (ageMinutes <= 90) return 'idle';
      }
      var activeHints = ['running', 'active', 'connected', 'online'];
      for (var j = 0; j < activeHints.length; j++) {
        if (state.indexOf(activeHints[j]) >= 0) return 'idle';
      }
      if (state.indexOf('idle') >= 0 || state.indexOf('paused') >= 0 || state.indexOf('suspend') >= 0) return 'idle';
      return 'offline';
    },

    agentStatusLabel(agent) {
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
      var agentState = String(agent.state || '').toLowerCase();
      return agentState.indexOf('typing') >= 0 || agentState.indexOf('working') >= 0 || agentState.indexOf('processing') >= 0;
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
