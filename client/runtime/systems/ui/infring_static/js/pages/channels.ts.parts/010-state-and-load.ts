// Infring Channels Page — Infring-style setup UX with QR code support
'use strict';

function channelsPage() {
  return {
    allChannels: [],
    showTemplateChannels: false,
    categoryFilter: 'all',
    searchQuery: '',
    setupModal: null,
    configuring: false,
    testing: {},
    formValues: {},
    showAdvanced: false,
    showBusinessApi: false,
    loading: true,
    loadError: '',
    pollTimer: null,

    // Setup flow step tracking
    setupStep: 1, // 1=Configure, 2=Verify, 3=Ready
    testPassed: false,

    // WhatsApp QR state
    qr: {
      loading: false,
      available: false,
      dataUrl: '',
      sessionId: '',
      message: '',
      help: '',
      connected: false,
      expired: false,
      error: ''
    },
    qrPollTimer: null,

    categories: [
      { key: 'all', label: 'All' },
      { key: 'messaging', label: 'Messaging' },
      { key: 'social', label: 'Social' },
      { key: 'enterprise', label: 'Enterprise' },
      { key: 'developer', label: 'Developer' },
      { key: 'notifications', label: 'Notifications' }
    ],

    get activeChannels() {
      var includeTemplates = !!this.showTemplateChannels;
      return this.allChannels.filter(function(ch) {
        var tier = String((ch && ch.channel_tier) || '').toLowerCase();
        var real = ch && Object.prototype.hasOwnProperty.call(ch, 'real_channel')
          ? !!ch.real_channel
          : (tier ? tier === 'native' : true);
        return includeTemplates || real;
      });
    },

    get filteredChannels() {
      var self = this;
      return this.activeChannels.filter(function(ch) {
        if (self.categoryFilter !== 'all' && ch.category !== self.categoryFilter) return false;
        if (self.searchQuery) {
          var q = self.searchQuery.toLowerCase();
          return ch.name.toLowerCase().indexOf(q) !== -1 ||
                 ch.display_name.toLowerCase().indexOf(q) !== -1 ||
                 ch.description.toLowerCase().indexOf(q) !== -1;
        }
        return true;
      });
    },

    get configuredCount() {
      return this.activeChannels.filter(function(ch) { return ch.configured; }).length;
    },

    get nativeChannelCount() {
      return this.allChannels.filter(function(ch) {
        var tier = String((ch && ch.channel_tier) || '').toLowerCase();
        return !!(ch && ch.real_channel) || tier === 'native';
      }).length;
    },

    get templateChannelCount() {
      var nativeCount = Number(this.nativeChannelCount || 0);
      return Math.max(0, this.allChannels.length - nativeCount);
    },

    get visibleChannelCount() {
      return this.activeChannels.length;
    },

    categoryCount(cat) {
      var all = this.activeChannels.filter(function(ch) { return cat === 'all' || ch.category === cat; });
      var configured = all.filter(function(ch) { return ch.configured; });
      return configured.length + '/' + all.length;
    },

    basicFields() {
      if (!this.setupModal || !this.setupModal.fields) return [];
      return this.setupModal.fields.filter(function(f) { return !f.advanced; });
    },

    advancedFields() {
      if (!this.setupModal || !this.setupModal.fields) return [];
      return this.setupModal.fields.filter(function(f) { return f.advanced; });
    },

    hasAdvanced() {
      return this.advancedFields().length > 0;
    },

    isQrChannel() {
      return this.setupModal && this.setupModal.setup_type === 'qr';
    },

    async loadChannels() {
      this.loading = true;
      this.loadError = '';
      try {
        var data = await InfringAPI.get('/api/channels');
        this.allChannels = (data.channels || []).map(function(ch) {
          ch.connected = ch.configured && ch.has_token;
          return ch;
        });
      } catch(e) {
        this.loadError = e.message || 'Could not load channels.';
      }
      this.loading = false;
      this.startPolling();
    },

    async loadData() { return this.loadChannels(); },

    startPolling() {
      var self = this;
      if (this.pollTimer) clearInterval(this.pollTimer);
      this.pollTimer = setInterval(function() { self.refreshStatus(); }, 15000);
    },

    async refreshStatus() {
      try {
        var data = await InfringAPI.get('/api/channels');
        var byName = {};
        (data.channels || []).forEach(function(ch) { byName[ch.name] = ch; });
        this.allChannels.forEach(function(c) {
          var fresh = byName[c.name];
          if (fresh) {
            c.configured = fresh.configured;
            c.has_token = fresh.has_token;
            c.connected = fresh.configured && fresh.has_token;
            c.fields = fresh.fields;
          }
        });
      } catch(e) { console.warn('Channel refresh failed:', e.message); }
    },

    statusBadge(ch) {
      if (!ch.configured) return { text: 'Not Configured', cls: 'badge-muted' };
      if (!ch.has_token) return { text: 'Missing Token', cls: 'badge-warn' };
      if (ch.connected) return { text: 'Ready', cls: 'badge-success' };
      return { text: 'Configured', cls: 'badge-info' };
    },

    tierBadge(ch) {
      var tier = String((ch && ch.channel_tier) || '').toLowerCase();
      var isNative = !!(ch && ch.real_channel) || tier === 'native';
      return isNative
        ? { text: 'Native', cls: 'badge-success' }
        : { text: 'Template', cls: 'badge-muted' };
    },

    difficultyClass(d) {
      if (d === 'Easy') return 'difficulty-easy';
      if (d === 'Hard') return 'difficulty-hard';
      return 'difficulty-medium';
    },

    openSetup(ch) {
      this.setupModal = ch;
      // Pre-populate form values from saved config (non-secret fields).
      var vals = {};
      if (ch.fields) {
        ch.fields.forEach(function(f) {
          if (f.value !== undefined && f.value !== null && f.type !== 'secret') {
            vals[f.key] = String(f.value);
          }
        });
      }
      this.formValues = vals;
      this.showAdvanced = false;
      this.showBusinessApi = false;
      this.setupStep = ch.configured ? 3 : 1;
      this.testPassed = !!ch.configured;
      this.resetQR();
      // Auto-start QR flow for QR-type channels
      if (ch.setup_type === 'qr') {
        this.startQR();
      }
    },
