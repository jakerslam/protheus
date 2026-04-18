// Infring Logs Page — Real-time log viewer (SSE streaming + polling fallback) + Audit Trail tab
'use strict';

var LOGS_MAX_ENTRIES = 500;
var LOGS_POLL_INTERVAL_MS = 2000;

function logsSafeTimestamp(value) {
  var raw = String(value || '').trim();
  if (!raw) return '';
  var parsed = Date.parse(raw);
  if (!Number.isFinite(parsed)) return '';
  return new Date(parsed).toISOString();
}

function logsNormalizeEntry(entry) {
  var row = entry && typeof entry === 'object' ? entry : {};
  return {
    seq: Number.isFinite(Number(row.seq)) ? Number(row.seq) : null,
    timestamp: logsSafeTimestamp(row.timestamp) || new Date().toISOString(),
    action: String(row.action || ''),
    detail: String(row.detail || ''),
    agent_id: String(row.agent_id || ''),
    payload: row.payload
  };
}

function logsEntryKey(entry) {
  if (entry && entry.seq !== null) return 'seq:' + String(entry.seq);
  var action = String(entry && entry.action || '');
  var detail = String(entry && entry.detail || '');
  var timestamp = String(entry && entry.timestamp || '');
  return 'fallback:' + timestamp + '|' + action + '|' + detail;
}

function logsPage() {
  return {
    tab: 'live',
    // -- Live logs state --
    entries: [],
    levelFilter: '',
    textFilter: '',
    autoRefresh: true,
    hovering: false,
    loading: true,
    loadError: '',
    _pollTimer: null,

    // -- SSE streaming state --
    _eventSource: null,
    streamConnected: false,
    streamConnecting: true,
    streamPaused: false,
    _entryKeyIndex: Object.create(null),

    // -- Audit state --
    auditEntries: [],
    tipHash: '',
    chainValid: null,
    filterAction: '',
    auditLoading: false,
    auditLoadError: '',

    resetEntryIndex: function() {
      this._entryKeyIndex = Object.create(null);
    },

    ingestEntries: function(rows) {
      this.entries = [];
      this.resetEntryIndex();
      var source = Array.isArray(rows) ? rows : [];
      for (var i = 0; i < source.length; i++) this.ingestEntry(source[i], true);
      this.entries.sort(function(a, b) {
        if (a.seq !== null && b.seq !== null) return a.seq - b.seq;
        return String(a.timestamp || '').localeCompare(String(b.timestamp || ''));
      });
      if (this.entries.length > LOGS_MAX_ENTRIES) {
        this.entries = this.entries.slice(this.entries.length - LOGS_MAX_ENTRIES);
      }
    },

    ingestEntry: function(raw, skipScroll) {
      var entry = logsNormalizeEntry(raw);
      var key = logsEntryKey(entry);
      if (this._entryKeyIndex[key]) return false;
      this._entryKeyIndex[key] = true;
      this.entries.push(entry);
      if (this.entries.length > LOGS_MAX_ENTRIES) {
        var removed = this.entries.splice(0, this.entries.length - LOGS_MAX_ENTRIES);
        for (var i = 0; i < removed.length; i++) {
          delete this._entryKeyIndex[logsEntryKey(removed[i])];
        }
      }
      if (!skipScroll && this.autoRefresh && !this.hovering) this.scrollToBottom();
      return true;
    },

    scrollToBottom: function() {
      this.$nextTick(function() {
        var el = document.getElementById('log-container');
        if (el) el.scrollTop = el.scrollHeight;
      });
    },

    startStreaming: function() {
      var self = this;
      if (this._eventSource) { this._eventSource.close(); this._eventSource = null; }
      this.streamConnecting = true;

      var url = '/api/logs/stream';
      var sep = '?';
      var token = InfringAPI.getToken();
      if (token) { url += sep + 'token=' + encodeURIComponent(token); sep = '&'; }

      try {
        this._eventSource = new EventSource(url);
      } catch(e) {
        // EventSource not supported or blocked; fall back to polling
        this.streamConnected = false;
        this.streamConnecting = false;
        this.startPolling();
        return;
      }

      this._eventSource.onopen = function() {
        self.streamConnected = true;
        self.streamConnecting = false;
        self.loading = false;
        self.loadError = '';
      };

      this._eventSource.onmessage = function(event) {
        if (self.streamPaused) return;
        try {
          self.ingestEntry(JSON.parse(event.data), false);
        } catch(e) {
          // Ignore parse errors (heartbeat comments are not delivered to onmessage)
        }
      };

      this._eventSource.onerror = function() {
        self.streamConnected = false;
        self.streamConnecting = false;
        if (self._eventSource) {
          self._eventSource.close();
          self._eventSource = null;
        }
        // Fall back to polling
        self.startPolling();
      };
    },

    startPolling: function() {
      var self = this;
      this.streamConnected = false;
      this.streamConnecting = false;
      this.fetchLogs();
      if (this._pollTimer) clearInterval(this._pollTimer);
      this._pollTimer = setInterval(function() {
        if (self.autoRefresh && !self.hovering && self.tab === 'live' && !self.streamPaused) {
          self.fetchLogs();
        }
      }, LOGS_POLL_INTERVAL_MS);
    },

    async fetchLogs() {
      if (this.loading) this.loadError = '';
      try {
        var data = await InfringAPI.get('/api/audit/recent?n=200');
        this.ingestEntries(data.entries || []);
        if (this.autoRefresh && !this.hovering) this.scrollToBottom();
        if (this.loading) this.loading = false;
      } catch(e) {
        if (this.loading) {
          this.loadError = e.message || 'Could not load logs.';
          this.loading = false;
        }
      }
    },

    async loadData() {
      this.loading = true;
      return this.fetchLogs();
    },

    togglePause: function() {
      this.streamPaused = !this.streamPaused;
      if (!this.streamPaused && this.streamConnected) {
        // Resume: scroll to bottom
        this.scrollToBottom();
      }
    },

    clearLogs: function() {
      this.entries = [];
      this.resetEntryIndex();
    },

    classifyLevel: function(action) {
      if (!action) return 'info';
      var a = action.toLowerCase();
      if (a.indexOf('error') !== -1 || a.indexOf('fail') !== -1 || a.indexOf('crash') !== -1) return 'error';
      if (a.indexOf('warn') !== -1 || a.indexOf('deny') !== -1 || a.indexOf('block') !== -1) return 'warn';
      return 'info';
    },

    get filteredEntries() {
      var self = this;
      var levelF = this.levelFilter;
      var textF = this.textFilter.toLowerCase();
      return this.entries.filter(function(e) {
        if (levelF && self.classifyLevel(e.action) !== levelF) return false;
        if (textF) {
          var haystack = ((e.action || '') + ' ' + (e.detail || '') + ' ' + (e.agent_id || '')).toLowerCase();
          if (haystack.indexOf(textF) === -1) return false;
        }
        return true;
      });
    },

    get connectionLabel() {
      if (this.streamPaused) return 'Paused';
      if (this.streamConnecting) return 'Connecting...';
      if (this.streamConnected) return 'Live';
      if (this._pollTimer) return 'Polling';
      return 'Disconnected';
    },

    get connectionClass() {
      if (this.streamPaused) return 'paused';
      if (this.streamConnecting) return 'connecting';
      if (this.streamConnected) return 'live';
      if (this._pollTimer) return 'polling';
      return 'disconnected';
    },

    exportLogs: function() {
      var lines = this.filteredEntries.map(function(e) {
        var stamp = logsSafeTimestamp(e.timestamp) || String(e.timestamp || '');
        return stamp + ' [' + e.action + '] ' + (e.detail || '');
      });
      var blob = new Blob([lines.join('\n')], { type: 'text/plain' });
      var url = URL.createObjectURL(blob);
      var a = document.createElement('a');
      a.href = url;
      a.download = 'infring-logs-' + new Date().toISOString().slice(0, 10) + '.txt';
      a.click();
      URL.revokeObjectURL(url);
    },

    // -- Audit methods --
    get filteredAuditEntries() {
      var self = this;
      if (!self.filterAction) return self.auditEntries;
      return self.auditEntries.filter(function(e) { return e.action === self.filterAction; });
    },

    async loadAudit() {
      this.auditLoading = true;
      this.auditLoadError = '';
      try {
        var data = await InfringAPI.get('/api/audit/recent?n=200');
        this.auditEntries = data.entries || [];
        this.tipHash = data.tip_hash || '';
      } catch(e) {
        this.auditEntries = [];
        this.auditLoadError = e.message || 'Could not load audit log.';
      }
      this.auditLoading = false;
    },

    auditAgentName: function(agentId) {
      if (!agentId) return '-';
      var agents = Alpine.store('app').agents || [];
      var agent = agents.find(function(a) { return a.id === agentId; });
      return agent ? agent.name : agentId.substring(0, 8) + '...';
    },

    friendlyAction: function(action) {
      if (!action) return 'Unknown';
      var map = {
        'AgentSpawn': 'Agent Created', 'AgentKill': 'Agent Stopped', 'AgentTerminated': 'Agent Stopped',
        'ToolInvoke': 'Tool Used', 'ToolResult': 'Tool Completed', 'AgentMessage': 'Message',
        'NetworkAccess': 'Network Access', 'ShellExec': 'Shell Command', 'FileAccess': 'File Access',
        'MemoryAccess': 'Memory Access', 'AuthAttempt': 'Login Attempt', 'AuthSuccess': 'Login Success',
        'AuthFailure': 'Login Failed', 'CapabilityDenied': 'Permission Denied', 'RateLimited': 'Rate Limited'
      };
      return map[action] || action.replace(/([A-Z])/g, ' $1').trim();
    },

    async verifyChain() {
      try {
        var data = await InfringAPI.get('/api/audit/verify');
        this.chainValid = data.valid === true;
        if (this.chainValid) {
          InfringToast.success('Audit chain verified — ' + (data.entries || 0) + ' entries valid');
        } else {
          InfringToast.error('Audit chain broken!');
        }
      } catch(e) {
        this.chainValid = false;
        InfringToast.error('Chain verification failed: ' + e.message);
      }
    },

    destroy: function() {
      if (this._eventSource) { this._eventSource.close(); this._eventSource = null; }
      if (this._pollTimer) { clearInterval(this._pollTimer); this._pollTimer = null; }
      this.resetEntryIndex();
    }
  };
}
