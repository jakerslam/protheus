// Infring Hands Page — curated autonomous capability packages
'use strict';

function handsPage() {
  return {
    tab: 'available',
    hands: [],
    instances: [],
    loading: true,
    activeLoading: false,
    loadError: '',
    activatingId: null,
    activateResult: null,
    detailHand: null,
    settingsValues: {},
    _toastTimer: null,
    browserViewer: null,
    browserViewerOpen: false,
    _browserPollTimer: null,

    // ── Trader Dashboard State ────────────────────────────────────────────
    dashboardOpen: false,
    dashboardLoading: false,
    dashboardData: null,
    _dashboardInst: null,
    _chartEquity: null,
    _chartPnl: null,
    _chartRadar: null,

    // ── Setup Wizard State ──────────────────────────────────────────────
    setupWizard: null,
    setupStep: 1,
    setupLoading: false,
    setupChecking: false,
    clipboardMsg: null,
    _clipboardTimer: null,
    detectedPlatform: 'linux',
    installPlatforms: {},
    apiKeyInputs: {},

    async loadData() {
      this.loading = true;
      this.loadError = '';
      try {
        var data = await InfringAPI.get('/api/hands');
        this.hands = data.hands || [];
      } catch(e) {
        this.hands = [];
        this.loadError = e.message || 'Could not load hands.';
      }
      this.loading = false;
    },

    async loadActive() {
      this.activeLoading = true;
      try {
        var data = await InfringAPI.get('/api/hands/active');
        this.instances = (data.instances || []).map(function(i) {
          i._stats = null;
          return i;
        });
      } catch(e) {
        this.instances = [];
      }
      this.activeLoading = false;
    },

    getHandIcon(handId) {
      for (var i = 0; i < this.hands.length; i++) {
        if (this.hands[i].id === handId) return this.hands[i].icon;
      }
      return '\u{1F91A}';
    },

    async showDetail(handId) {
      try {
        var data = await InfringAPI.get('/api/hands/' + handId);
        this.detailHand = data;
      } catch(e) {
        for (var i = 0; i < this.hands.length; i++) {
          if (this.hands[i].id === handId) {
            this.detailHand = this.hands[i];
            break;
          }
        }
      }
    },

    // ── Setup Wizard ────────────────────────────────────────────────────

    async activate(handId) {
      this.openSetupWizard(handId);
    },

    async openSetupWizard(handId) {
      this.setupLoading = true;
      this.setupWizard = null;
      try {
        var data = await InfringAPI.get('/api/hands/' + handId);
        // Pre-populate settings defaults
        this.settingsValues = {};
        if (data.settings && data.settings.length > 0) {
          for (var i = 0; i < data.settings.length; i++) {
            var s = data.settings[i];
            this.settingsValues[s.key] = s.default || '';
          }
        }
        // Detect platform from server response, fallback to client-side
        if (data.server_platform) {
          this.detectedPlatform = data.server_platform;
        } else {
          this._detectClientPlatform();
        }
        // Initialize per-requirement platform selections and API key inputs
        this.installPlatforms = {};
        this.apiKeyInputs = {};
        if (data.requirements) {
          for (var j = 0; j < data.requirements.length; j++) {
            this.installPlatforms[data.requirements[j].key] = this.detectedPlatform;
            if (data.requirements[j].type === 'ApiKey') {
              this.apiKeyInputs[data.requirements[j].key] = '';
            }
          }
        }
        this.setupWizard = data;
        // Skip deps step if no requirements
        var hasReqs = data.requirements && data.requirements.length > 0;
        this.setupStep = hasReqs ? 1 : 2;
      } catch(e) {
        this.showToast('Could not load hand details: ' + (e.message || 'unknown error'));
      }
      this.setupLoading = false;
    },

    ...infringHandsSetupWizardMethods(),

    // ── Existing methods ────────────────────────────────────────────────

    async pauseHand(inst) {
      try {
        await InfringAPI.post('/api/hands/instances/' + inst.instance_id + '/pause', {});
        inst.status = 'Paused';
      } catch(e) {
        this.showToast('Pause failed: ' + (e.message || 'unknown error'));
      }
    },

    async resumeHand(inst) {
      try {
        await InfringAPI.post('/api/hands/instances/' + inst.instance_id + '/resume', {});
        inst.status = 'Active';
      } catch(e) {
        this.showToast('Resume failed: ' + (e.message || 'unknown error'));
      }
    },

    async deactivate(inst) {
      var self = this;
      var handName = inst.agent_name || inst.hand_id;
      InfringToast.confirm('Deactivate Hand', 'Deactivate hand "' + handName + '"? This will kill its agent.', async function() {
        try {
          await InfringAPI.delete('/api/hands/instances/' + inst.instance_id);
          self.instances = self.instances.filter(function(i) { return i.instance_id !== inst.instance_id; });
          InfringToast.success('Hand deactivated.');
        } catch(e) {
          InfringToast.error('Deactivation failed: ' + (e.message || 'unknown error'));
        }
      });
    },

    async loadStats(inst) {
      try {
        var data = await InfringAPI.get('/api/hands/instances/' + inst.instance_id + '/stats');
        inst._stats = data.metrics || {};
      } catch(e) {
        inst._stats = { 'Error': { value: e.message || 'Could not load stats', format: 'text' } };
      }
    },

    formatMetric(m) {
      if (!m || m.value === null || m.value === undefined) return '-';
      if (m.format === 'duration') {
        var secs = parseInt(m.value, 10);
        if (isNaN(secs)) return String(m.value);
        var h = Math.floor(secs / 3600);
        var min = Math.floor((secs % 3600) / 60);
        var s = secs % 60;
        if (h > 0) return h + 'h ' + min + 'm';
        if (min > 0) return min + 'm ' + s + 's';
        return s + 's';
      }
      if (m.format === 'number') {
        var n = parseFloat(m.value);
        if (isNaN(n)) return String(m.value);
        return n.toLocaleString();
      }
      return String(m.value);
    },

    showToast(msg) {
      var self = this;
      this.activateResult = msg;
      if (this._toastTimer) clearTimeout(this._toastTimer);
      this._toastTimer = setTimeout(function() { self.activateResult = null; }, 4000);
    },

    ...infringHandsDashboardViewerMethods(),

  };
}
