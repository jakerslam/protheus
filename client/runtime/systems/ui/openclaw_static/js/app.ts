// Infring App — Alpine.js init, hash router, global store
'use strict';

// Marked.js configuration
if (typeof marked !== 'undefined') {
  marked.setOptions({
    breaks: true,
    gfm: true,
    highlight: function(code, lang) {
      if (typeof hljs !== 'undefined' && lang && hljs.getLanguage(lang)) {
        try { return hljs.highlight(code, { language: lang }).value; } catch(e) {}
      }
      return code;
    }
  });
}

function escapeHtml(text) {
  var div = document.createElement('div');
  div.textContent = text || '';
  return div.innerHTML;
}

function renderMarkdown(text) {
  if (!text) return '';
  if (typeof marked !== 'undefined') {
    // Protect LaTeX blocks from marked.js mangling (underscores, backslashes, etc.)
    var latexBlocks = [];
    var protected_ = text;
    // Protect display math $$...$$ first (greedy across lines)
    protected_ = protected_.replace(/\$\$([\s\S]+?)\$\$/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect inline math $...$ (single line, not empty, not starting/ending with space)
    protected_ = protected_.replace(/\$([^\s$](?:[^$]*[^\s$])?)\$/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect \[...\] display math
    protected_ = protected_.replace(/\\\[([\s\S]+?)\\\]/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });
    // Protect \(...\) inline math
    protected_ = protected_.replace(/\\\(([\s\S]+?)\\\)/g, function(match) {
      var idx = latexBlocks.length;
      latexBlocks.push(match);
      return '\x00LATEX' + idx + '\x00';
    });

    var html = marked.parse(protected_);
    // Restore LaTeX blocks
    for (var i = 0; i < latexBlocks.length; i++) {
      html = html.replace('\x00LATEX' + i + '\x00', latexBlocks[i]);
    }
    // Add copy buttons to code blocks
    html = html.replace(/<pre><code/g, '<pre><button class="copy-btn" onclick="copyCode(this)">Copy</button><code');
    // Open external links in new tab
    html = html.replace(/<a\s+href="(https?:\/\/[^"]*)"(?![^>]*target=)([^>]*)>/gi, '<a href="$1" target="_blank" rel="noopener"$2>');
    return html;
  }
  return escapeHtml(text);
}

// Render LaTeX math in the chat message container using KaTeX auto-render.
// Call this after new messages are inserted into the DOM.
function renderLatex(el) {
  if (typeof renderMathInElement !== 'function') return;
  var target = el || document.getElementById('messages');
  if (!target) return;
  try {
    renderMathInElement(target, {
      delimiters: [
        { left: '$$', right: '$$', display: true },
        { left: '\\[', right: '\\]', display: true },
        { left: '$', right: '$', display: false },
        { left: '\\(', right: '\\)', display: false }
      ],
      throwOnError: false,
      trust: false
    });
  } catch(e) { /* KaTeX render error — ignore gracefully */ }
}

function copyCode(btn) {
  var code = btn.nextElementSibling;
  if (code) {
    navigator.clipboard.writeText(code.textContent).then(function() {
      btn.textContent = 'Copied!';
      btn.classList.add('copied');
      setTimeout(function() { btn.textContent = 'Copy'; btn.classList.remove('copied'); }, 1500);
    });
  }
}

// Tool category icon SVGs — returns inline SVG for each tool category
function toolIcon(toolName) {
  if (!toolName) return '';
  var n = toolName.toLowerCase();
  var s = 'width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"';
  // File/directory operations
  if (n.indexOf('file_') === 0 || n.indexOf('directory_') === 0)
    return '<svg ' + s + '><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/><path d="M16 13H8"/><path d="M16 17H8"/></svg>';
  // Web/fetch
  if (n.indexOf('web_') === 0 || n.indexOf('link_') === 0)
    return '<svg ' + s + '><circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15 15 0 0 1 4 10 15 15 0 0 1-4 10 15 15 0 0 1-4-10 15 15 0 0 1 4-10z"/></svg>';
  // Shell/exec
  if (n.indexOf('shell') === 0 || n.indexOf('exec_') === 0)
    return '<svg ' + s + '><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>';
  // Agent operations
  if (n.indexOf('agent_') === 0)
    return '<svg ' + s + '><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>';
  // Memory/knowledge
  if (n.indexOf('memory_') === 0 || n.indexOf('knowledge_') === 0)
    return '<svg ' + s + '><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>';
  // Cron/schedule
  if (n.indexOf('cron_') === 0 || n.indexOf('schedule_') === 0)
    return '<svg ' + s + '><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>';
  // Browser/playwright
  if (n.indexOf('browser_') === 0 || n.indexOf('playwright_') === 0)
    return '<svg ' + s + '><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8"/><path d="M12 17v4"/></svg>';
  // Container/docker
  if (n.indexOf('container_') === 0 || n.indexOf('docker_') === 0)
    return '<svg ' + s + '><path d="M22 12H2"/><path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/></svg>';
  // Image/media
  if (n.indexOf('image_') === 0 || n.indexOf('tts_') === 0)
    return '<svg ' + s + '><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>';
  // Hand tools
  if (n.indexOf('hand_') === 0)
    return '<svg ' + s + '><path d="M18 11V6a2 2 0 0 0-2-2 2 2 0 0 0-2 2"/><path d="M14 10V4a2 2 0 0 0-2-2 2 2 0 0 0-2 2v6"/><path d="M10 10.5V6a2 2 0 0 0-2-2 2 2 0 0 0-2 2v8"/><path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.9-5.7-2.4L3.4 16a2 2 0 0 1 3.2-2.4L8 15"/></svg>';
  // Task/collab
  if (n.indexOf('task_') === 0)
    return '<svg ' + s + '><path d="M9 11l3 3L22 4"/><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11"/></svg>';
  // Default — wrench
  return '<svg ' + s + '><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/></svg>';
}

// Alpine.js global store
document.addEventListener('alpine:init', function() {
  // Restore saved API key on load
  var savedKey = localStorage.getItem('infring-api-key');
  if (savedKey) InfringAPI.setAuthToken(savedKey);

  Alpine.store('app', {
    agents: [],
    connected: false,
    booting: true,
    wsConnected: false,
    connectionState: 'connected',
    lastError: '',
    version: '0.1.0',
    gitBranch: '',
    agentCount: 0,
    pendingAgent: null,
    pendingFreshAgentId: null,
    activeAgentId: (() => {
      try {
        var saved = localStorage.getItem('infring-last-active-agent-id');
        return saved ? String(saved) : null;
      } catch(_) {
        return null;
      }
    })(),
    focusMode: localStorage.getItem('infring-focus') === 'true',
    showOnboarding: false,
    showAuthPrompt: false,
    authMode: 'apikey',
    sessionUser: null,
    notifications: [],
    notificationsOpen: false,
    unreadNotifications: 0,
    notificationBubble: null,
    _notificationBubbleTimer: null,
    _notificationSeq: 0,
    agentChatPreviews: {},
    agentLiveActivity: {},

    toggleFocusMode() {
      this.focusMode = !this.focusMode;
      localStorage.setItem('infring-focus', this.focusMode);
    },

    setActiveAgentId(agentId) {
      this.activeAgentId = agentId ? String(agentId) : null;
      try {
        if (this.activeAgentId) localStorage.setItem('infring-last-active-agent-id', this.activeAgentId);
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch(_) {}
    },

    async refreshAgents() {
      try {
        var agents = await InfringAPI.get('/api/agents');
        this.agents = Array.isArray(agents) ? agents : [];
        var keep = {};
        for (var ai = 0; ai < this.agents.length; ai++) {
          var row = this.agents[ai];
          if (row && row.id) keep[String(row.id)] = true;
        }
        var nextActivity = {};
        var now = Date.now();
        var srcActivity = this.agentLiveActivity || {};
        Object.keys(srcActivity).forEach(function(id) {
          var entry = srcActivity[id];
          if (!keep[id] || !entry) return;
          var ts = Number(entry.ts || 0);
          if (!Number.isFinite(ts) || (now - ts) > 20000) return;
          nextActivity[id] = entry;
        });
        this.agentLiveActivity = nextActivity;
        if (this.activeAgentId) {
          var stillActive = this.agents.some(function(agent) {
            return agent && agent.id === this.activeAgentId;
          }.bind(this));
          if (!stillActive) {
            this.setActiveAgentId(null);
          }
        }
        this.agentCount = this.agents.length;
      } catch(e) { /* silent */ }
    },

    async checkStatus() {
      try {
        var s = await InfringAPI.get('/api/status');
        this.connected = true;
        this.booting = false;
        this.lastError = '';
        this.version = s.version || '0.1.0';
        this.gitBranch = s.git_branch ? String(s.git_branch) : (this.gitBranch || '');
        this.agentCount = s.agent_count || 0;
      } catch(e) {
        this.connected = false;
        this.lastError = e.message || 'Unknown error';
        console.warn('[Infring] Status check failed:', e.message);
      }
    },

    async checkOnboarding() {
      if (localStorage.getItem('infring-onboarded')) return;
      try {
        var config = await InfringAPI.get('/api/config');
        var apiKey = config && config.api_key;
        var noKey = !apiKey || apiKey === 'not set' || apiKey === '';
        if (noKey && this.agentCount === 0) {
          this.showOnboarding = true;
        }
      } catch(e) {
        // If config endpoint fails, still show onboarding if no agents
        if (this.agentCount === 0) this.showOnboarding = true;
      }
    },

    dismissOnboarding() {
      this.showOnboarding = false;
      localStorage.setItem('infring-onboarded', 'true');
    },

    async checkAuth() {
      try {
        // First check if session-based auth is configured
        var authInfo = await InfringAPI.get('/api/auth/check');
        if (authInfo.mode === 'none') {
          // No session auth — fall back to API key detection
          this.authMode = 'apikey';
          this.sessionUser = null;
        } else if (authInfo.mode === 'session') {
          this.authMode = 'session';
          if (authInfo.authenticated) {
            this.sessionUser = authInfo.username;
            this.showAuthPrompt = false;
            return;
          }
          // Session auth enabled but not authenticated — show login prompt
          this.showAuthPrompt = true;
          return;
        }
      } catch(e) { /* ignore — fall through to API key check */ }

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

    addNotification(payload) {
      var p = payload || {};
      var note = {
        id: p.id || ('notif-' + (++this._notificationSeq) + '-' + Date.now()),
        message: String(p.message || ''),
        type: String(p.type || 'info'),
        ts: Number(p.ts || Date.now()),
        read: !!this.notificationsOpen
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
      var preview = {
        text: '',
        ts: Date.now(),
        role: 'agent',
        has_tools: false,
        tool_state: '',
        tool_label: ''
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
      this.agentChatPreviews[String(agentId)] = preview;
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
        var ts = Number(entry.ts || 0);
        if (Number.isFinite(ts) && (Date.now() - ts) <= 15000) return true;
      }
      var state = String(agent.state || '').toLowerCase();
      return state.indexOf('typing') >= 0 || state.indexOf('working') >= 0 || state.indexOf('processing') >= 0;
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
    confirmArchiveAgentId: '',
    archivedAgentIds: (() => {
      try {
        var raw = localStorage.getItem('infring-archived-agent-ids');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return [];
        return parsed.map(function(id) { return String(id); });
      } catch(_) {
        return [];
      }
    })(),
    sidebarSpawningAgent: false,
    connected: false,
    wsConnected: false,
    version: '0.1.0',
    agentCount: 0,
    bootSelectionApplied: false,

    get agents() { return Alpine.store('app').agents; },

    get chatSidebarAgents() {
      var list = (this.agents || []).slice();
      var self = this;
      var archivedSet = new Set((this.archivedAgentIds || []).map(function(id) { return String(id); }));
      list = list.filter(function(agent) {
        if (!agent || !agent.id) return false;
        return !archivedSet.has(String(agent.id));
      });
      list.sort(function(a, b) {
        return self.sidebarAgentSortTs(b) - self.sidebarAgentSortTs(a);
      });
      var q = String(this.chatSidebarQuery || '').trim().toLowerCase();
      if (!q) return list;
      return list.filter(function(agent) {
        var name = String((agent && agent.name) || (agent && agent.id) || '').toLowerCase();
        var preview = self.chatSidebarPreview(agent);
        var text = String((preview && preview.text) || '').toLowerCase();
        return name.indexOf(q) >= 0 || text.indexOf(q) >= 0;
      });
    },

    init() {
      var self = this;

      // Listen for OS theme changes (only matters when mode is 'system')
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function(e) {
        if (self.themeMode === 'system') {
          self.theme = e.matches ? 'dark' : 'light';
        }
      });

      // Hash routing
      var validPages = ['overview','chat','agents','sessions','approvals','comms','workflows','scheduler','channels','skills','hands','analytics','logs','runtime','settings','wizard'];
      var pageRedirects = {
        'templates': 'agents',
        'triggers': 'workflows',
        'cron': 'scheduler',
        'schedules': 'scheduler',
        'memory': 'sessions',
        'audit': 'logs',
        'security': 'settings',
        'peers': 'settings',
        'migration': 'settings',
        'usage': 'analytics',
        'approval': 'approvals'
      };
      function handleHash() {
        var hash = window.location.hash.replace('#', '') || 'chat';
        if (pageRedirects[hash]) {
          hash = pageRedirects[hash];
          window.location.hash = hash;
        }
        if (validPages.indexOf(hash) >= 0) self.page = hash;
        if (hash !== 'chat') self.closeAgentChatsSidebar();
      }
      window.addEventListener('hashchange', handleHash);
      handleHash();

      // Keyboard shortcuts
      document.addEventListener('keydown', function(e) {
        // Ctrl+K — focus agent switch / go to agents
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
          e.preventDefault();
          self.navigate('agents');
        }
        // Ctrl+N — new agent
        if ((e.ctrlKey || e.metaKey) && e.key === 'n' && !e.shiftKey) {
          e.preventDefault();
          self.createSidebarAgentChat();
        }
        // Ctrl+Shift+F — toggle focus mode
        if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'F') {
          e.preventDefault();
          Alpine.store('app').toggleFocusMode();
        }
        // Escape — close mobile menu
        if (e.key === 'Escape') {
          self.mobileMenuOpen = false;
          self.closeAgentChatsSidebar();
        }
      });

      document.addEventListener('click', function(e) {
        if (self.chatSidebarMode !== 'agent_list' || self.page !== 'chat') return;
        var target = e && e.target;
        if (!target || !target.closest) {
          self.closeAgentChatsSidebar();
          return;
        }
        if (target.closest('[data-agent-chat-sidebar]')) return;
        self.closeAgentChatsSidebar();
      });

      // Connection state listener
      InfringAPI.onConnectionChange(function(state) {
        Alpine.store('app').connectionState = state;
      });

      if (!window.__infringToastCaptureInstalled) {
        window.addEventListener('infring:toast', function(ev) {
          var detail = (ev && ev.detail) ? ev.detail : {};
          var store = Alpine.store('app');
          if (store && typeof store.addNotification === 'function') {
            store.addNotification(detail);
          }
        });
        window.__infringToastCaptureInstalled = true;
      }

      // Initial data load
      this.pollStatus();
      Alpine.store('app').checkOnboarding();
      Alpine.store('app').checkAuth();
      setInterval(function() { self.pollStatus(); }, 5000);
    },

    navigate(p) {
      this.page = p;
      window.location.hash = p;
      this.mobileMenuOpen = false;
      if (p !== 'chat') this.closeAgentChatsSidebar();
    },

    setTheme(mode) {
      this.themeMode = mode;
      localStorage.setItem('infring-theme-mode', mode);
      if (mode === 'system') {
        this.theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      } else {
        this.theme = mode;
      }
    },

    toggleTheme() {
      var modes = ['light', 'system', 'dark'];
      var next = modes[(modes.indexOf(this.themeMode) + 1) % modes.length];
      this.setTheme(next);
    },

    toggleSidebar() {
      this.sidebarCollapsed = !this.sidebarCollapsed;
      localStorage.setItem('infring-sidebar', this.sidebarCollapsed ? 'collapsed' : 'expanded');
    },

    toggleAgentChatsSidebar() {
      if (this.page !== 'chat') {
        this.navigate('chat');
      }
      this.chatSidebarMode = this.chatSidebarMode === 'agent_list' ? 'default' : 'agent_list';
      if (this.chatSidebarMode === 'agent_list') {
        this.chatSidebarQuery = '';
        if (this.sidebarCollapsed) {
          this.sidebarCollapsed = false;
          localStorage.setItem('infring-sidebar', 'expanded');
        }
      }
    },

    closeAgentChatsSidebar() {
      if (this.chatSidebarMode !== 'default') {
        this.chatSidebarMode = 'default';
        this.chatSidebarQuery = '';
      }
      this.confirmArchiveAgentId = '';
    },

    async applyBootChatSelection() {
      if (this.bootSelectionApplied) return;
      this.bootSelectionApplied = true;
      var store = Alpine.store('app');
      var rows = Array.isArray(store.agents) ? store.agents.slice() : [];
      if (!rows.length) {
        this.navigate('chat');
        await this.createSidebarAgentChat();
        return;
      }
      var target = null;
      if (store.activeAgentId) {
        var saved = String(store.activeAgentId);
        target = rows.find(function(agent) { return agent && String(agent.id) === saved; }) || null;
      }
      if (!target) {
        rows.sort(function(a, b) {
          return this.sidebarAgentSortTs(b) - this.sidebarAgentSortTs(a);
        }.bind(this));
        target = rows.length ? rows[0] : null;
      }
      if (target && target.id) {
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(target.id);
        else store.activeAgentId = target.id;
      }
      this.navigate('chat');
      this.closeAgentChatsSidebar();
    },

    sidebarAgentSortTs(agent) {
      if (!agent) return 0;
      var store = Alpine.store('app');
      var preview = store && typeof store.getAgentChatPreview === 'function'
        ? store.getAgentChatPreview(agent.id)
        : null;
      if (preview && preview.ts) return Number(preview.ts) || 0;
      if (agent.updated_at) return Number(new Date(agent.updated_at).getTime()) || 0;
      if (agent.created_at) return Number(new Date(agent.created_at).getTime()) || 0;
      return 0;
    },

    chatSidebarPreview(agent) {
      if (!agent) return { text: 'No messages yet', ts: 0, role: 'agent', has_tools: false, tool_state: '', tool_label: '' };
      var store = Alpine.store('app');
      var preview = store && typeof store.getAgentChatPreview === 'function'
        ? store.getAgentChatPreview(agent.id)
        : null;
      if (!preview || !preview.text) return { text: 'No messages yet', ts: this.sidebarAgentSortTs(agent), role: 'agent', has_tools: false, tool_state: '', tool_label: '' };
      return preview;
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

    async archiveAgentFromSidebar(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if ((this.archivedAgentIds || []).indexOf(agentId) >= 0) return;
      this.confirmArchiveAgentId = '';
      try {
        await InfringAPI.del('/api/agents/' + encodeURIComponent(agentId));
      } catch(e) {
        InfringToast.error('Failed to archive agent: ' + (e && e.message ? e.message : 'unknown error'));
        return;
      }
      this.archivedAgentIds = (this.archivedAgentIds || []).concat([agentId]);
      this.persistArchivedAgentIds();
      var store = Alpine.store('app');
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
      InfringToast.success('Archived "' + (agent.name || agent.id) + '"');
    },

    async createSidebarAgentChat() {
      if (this.sidebarSpawningAgent) return;
      this.confirmArchiveAgentId = '';
      this.sidebarSpawningAgent = true;
      var stamp = Date.now().toString(36);
      var rand = Math.floor(Math.random() * 46656).toString(36).padStart(3, '0');
      var agentName = 'agent-' + stamp + '-' + rand;
      try {
        var res = await InfringAPI.post('/api/agents', {
          name: agentName,
          role: 'analyst',
          contract: {
            mission: 'Fresh chat initialization',
            termination_condition: 'task_or_timeout',
            expiry_seconds: 3600
          }
        });
        var createdId = String((res && (res.id || res.agent_id)) || '').trim();
        if (!createdId) throw new Error('spawn_failed');
        await Alpine.store('app').refreshAgents();
        var created = (this.agents || []).find(function(a) { return a && a.id === createdId; })
          || { id: createdId, name: (res && res.name) || agentName };
        this.archivedAgentIds = (this.archivedAgentIds || []).filter(function(id) { return String(id) !== createdId; });
        this.persistArchivedAgentIds();
        Alpine.store('app').pendingAgent = created;
        Alpine.store('app').pendingFreshAgentId = created.id;
        if (typeof Alpine.store('app').setActiveAgentId === 'function') Alpine.store('app').setActiveAgentId(created.id);
        else Alpine.store('app').activeAgentId = created.id;
        this.navigate('chat');
        this.closeAgentChatsSidebar();
        InfringToast.success('Agent "' + (created.name || created.id || agentName) + '" created');
      } catch(e) {
        InfringToast.error('Failed to create agent: ' + (e && e.message ? e.message : 'unknown error'));
      }
      this.sidebarSpawningAgent = false;
    },

    selectAgentChatFromSidebar(agent) {
      if (!agent || !agent.id) return;
      this.confirmArchiveAgentId = '';
      if (typeof Alpine.store('app').setActiveAgentId === 'function') Alpine.store('app').setActiveAgentId(agent.id);
      else Alpine.store('app').activeAgentId = agent.id;
      this.navigate('chat');
      this.closeAgentChatsSidebar();
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

    async pollStatus() {
      var store = Alpine.store('app');
      await store.checkStatus();
      await store.refreshAgents();
      this.connected = store.connected;
      this.version = store.version;
      this.agentCount = store.agentCount;
      this.wsConnected = InfringAPI.isWsConnected();
      if (!this.bootSelectionApplied) {
        await this.applyBootChatSelection();
      }
    }
  };
}
