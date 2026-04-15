
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
      this.ringNotificationBell();
      this.showNotificationBubble(note);
    },
    ringNotificationBell() {
      var self = this, seq = Number(this._notificationBellPulseSeq || 0) + 1;
      this._notificationBellPulseSeq = seq;
      this.notificationBellPulse = false;
      if (this._notificationBellPulseTimer) {
        clearTimeout(this._notificationBellPulseTimer);
        this._notificationBellPulseTimer = null;
      }
      var arm = function() {
        if (self._notificationBellPulseSeq !== seq) return;
        self.notificationBellPulse = true;
        self._notificationBellPulseTimer = setTimeout(function() {
          if (self._notificationBellPulseSeq !== seq) return;
          self.notificationBellPulse = false;
          self._notificationBellPulseTimer = null;
        }, 760);
      };
      if (typeof requestAnimationFrame === 'function') {
        requestAnimationFrame(arm);
      } else {
        setTimeout(arm, 0);
      }
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
      this.notificationBellPulse = false;
      this._notificationBellPulseSeq = 0;
      if (this._notificationBellPulseTimer) {
        clearTimeout(this._notificationBellPulseTimer);
        this._notificationBellPulseTimer = null;
      }
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
