// Infring Sessions Page — Session listing + Memory tab
'use strict';

var SESSIONS_KV_MAX_KEY_LENGTH = 256;

function sessionsNormalizeKvKey(raw) {
  var key = String(raw || '').trim();
  if (!key) return '';
  key = key.replace(/[\u0000-\u001F\u007F]/g, '');
  if (key.length > SESSIONS_KV_MAX_KEY_LENGTH) key = key.slice(0, SESSIONS_KV_MAX_KEY_LENGTH);
  return key.trim();
}

function sessionsParseKvValue(rawText) {
  try { return JSON.parse(rawText); } catch(_) { return rawText; }
}

function sessionsStringifyKvValue(rawValue) {
  if (rawValue && typeof rawValue === 'object') {
    try { return JSON.stringify(rawValue, null, 2); } catch(_) {}
  }
  return String(rawValue);
}

function sessionsPage() {
  return {
    tab: 'sessions',
    // -- Sessions state --
    sessions: [],
    searchFilter: '',
    loading: true,
    loadError: '',

    // -- Memory state --
    memAgentId: '',
    kvPairs: [],
    showAdd: false,
    newKey: '',
    newValue: '""',
    editingKey: null,
    editingValue: '',
    memLoading: false,
    memLoadError: '',

    // -- Sessions methods --
    async loadSessions() {
      this.loading = true;
      this.loadError = '';
      try {
        var data = await InfringAPI.get('/api/sessions');
        var sessions = data.sessions || [];
        var agents = Alpine.store('app').agents;
        var agentMap = {};
        agents.forEach(function(a) { agentMap[a.id] = a.name; });
        sessions.forEach(function(s) {
          s.agent_name = agentMap[s.agent_id] || '';
        });
        this.sessions = sessions;
      } catch(e) {
        this.sessions = [];
        this.loadError = e.message || 'Could not load sessions.';
      }
      this.loading = false;
    },

    async loadData() { return this.loadSessions(); },

    get filteredSessions() {
      var f = this.searchFilter.toLowerCase();
      if (!f) return this.sessions;
      return this.sessions.filter(function(s) {
        return (s.agent_name || '').toLowerCase().indexOf(f) !== -1 ||
               (s.agent_id || '').toLowerCase().indexOf(f) !== -1;
      });
    },

    openInChat(session) {
      var agents = Alpine.store('app').agents;
      var agent = agents.find(function(a) { return a.id === session.agent_id; });
      if (agent) {
        Alpine.store('app').pendingAgent = agent;
      }
      location.hash = 'agents';
    },

    deleteSession(sessionId) {
      var self = this;
      InfringToast.confirm('Delete Session', 'This will permanently remove the session and its messages.', async function() {
        try {
          await InfringAPI.del('/api/sessions/' + sessionId);
          self.sessions = self.sessions.filter(function(s) { return s.session_id !== sessionId; });
          InfringToast.success('Session deleted');
        } catch(e) {
          InfringToast.error('Failed to delete session: ' + e.message);
        }
      });
    },

    // -- Memory methods --
    async loadKv() {
      if (!this.memAgentId) { this.kvPairs = []; return; }
      this.memLoading = true;
      this.memLoadError = '';
      try {
        var data = await InfringAPI.get('/api/memory/agents/' + this.memAgentId + '/kv');
        this.kvPairs = data.kv_pairs || [];
      } catch(e) {
        this.kvPairs = [];
        this.memLoadError = e.message || 'Could not load memory data.';
      }
      this.memLoading = false;
    },

    async addKey() {
      if (!this.memAgentId) return;
      var key = sessionsNormalizeKvKey(this.newKey);
      if (!key) {
        InfringToast.error('Memory key is required');
        return;
      }
      var value = sessionsParseKvValue(this.newValue);
      try {
        await InfringAPI.put('/api/memory/agents/' + this.memAgentId + '/kv/' + encodeURIComponent(key), { value: value });
        this.showAdd = false;
        InfringToast.success('Key "' + key + '" saved');
        this.newKey = '';
        this.newValue = '""';
        await this.loadKv();
      } catch(e) {
        InfringToast.error('Failed to save key: ' + e.message);
      }
    },

    deleteKey(key) {
      var self = this;
      InfringToast.confirm('Delete Key', 'Delete key "' + key + '"? This cannot be undone.', async function() {
        try {
          await InfringAPI.del('/api/memory/agents/' + self.memAgentId + '/kv/' + encodeURIComponent(key));
          InfringToast.success('Key "' + key + '" deleted');
          await self.loadKv();
        } catch(e) {
          InfringToast.error('Failed to delete key: ' + e.message);
        }
      });
    },

    startEdit(kv) {
      this.editingKey = kv.key;
      this.editingValue = sessionsStringifyKvValue(kv.value);
    },

    cancelEdit() {
      this.editingKey = null;
      this.editingValue = '';
    },

    async saveEdit() {
      if (!this.editingKey || !this.memAgentId) return;
      var key = sessionsNormalizeKvKey(this.editingKey);
      if (!key) {
        InfringToast.error('Memory key is invalid');
        return;
      }
      var value = sessionsParseKvValue(this.editingValue);
      try {
        await InfringAPI.put('/api/memory/agents/' + this.memAgentId + '/kv/' + encodeURIComponent(key), { value: value });
        InfringToast.success('Key "' + key + '" updated');
        this.editingKey = null;
        this.editingValue = '';
        await this.loadKv();
      } catch(e) {
        InfringToast.error('Failed to save: ' + e.message);
      }
    }
  };
}
