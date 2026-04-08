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

    // ── QR Code Flow (WhatsApp Web style) ──────────────────────────

    resetQR() {
      this.qr = {
        loading: false, available: false, dataUrl: '', sessionId: '',
        message: '', help: '', connected: false, expired: false, error: ''
      };
      if (this.qrPollTimer) { clearInterval(this.qrPollTimer); this.qrPollTimer = null; }
    },

    async startQR() {
      this.qr.loading = true;
      this.qr.error = '';
      this.qr.connected = false;
      this.qr.expired = false;
      try {
        var result = await InfringAPI.post('/api/channels/whatsapp/qr/start', {});
        this.qr.available = result.available || false;
        this.qr.dataUrl = result.qr_data_url || '';
        this.qr.sessionId = result.session_id || '';
        this.qr.message = result.message || '';
        this.qr.help = result.help || '';
        this.qr.connected = result.connected || false;
        if (this.qr.available && this.qr.dataUrl && !this.qr.connected) {
          this.pollQR();
        }
        if (this.qr.connected) {
          InfringToast.success('WhatsApp connected!');
          await this.refreshStatus();
        }
      } catch(e) {
        this.qr.error = e.message || 'Could not start QR login';
      }
      this.qr.loading = false;
    },

    pollQR() {
      var self = this;
      if (this.qrPollTimer) clearInterval(this.qrPollTimer);
      this.qrPollTimer = setInterval(async function() {
        try {
          var result = await InfringAPI.get('/api/channels/whatsapp/qr/status?session_id=' + encodeURIComponent(self.qr.sessionId));
          if (result.connected) {
            clearInterval(self.qrPollTimer);
            self.qrPollTimer = null;
            self.qr.connected = true;
            self.qr.message = result.message || 'Connected!';
            InfringToast.success('WhatsApp linked successfully!');
            await self.refreshStatus();
          } else if (result.expired) {
            clearInterval(self.qrPollTimer);
            self.qrPollTimer = null;
            self.qr.expired = true;
            self.qr.message = 'QR code expired. Click to generate a new one.';
          } else {
            self.qr.message = result.message || 'Waiting for scan...';
          }
        } catch(e) { /* silent retry */ }
      }, 3000);
    },

    // ── Standard Form Flow ─────────────────────────────────────────

    async saveChannel() {
      if (!this.setupModal) return;
      var name = this.setupModal.name;
      this.configuring = true;
      try {
        await InfringAPI.post('/api/channels/' + name + '/configure', {
          fields: this.formValues
        });
        this.setupStep = 2;
        // Auto-test after save
        try {
          var testResult = await InfringAPI.post('/api/channels/' + name + '/test', { force_live: true });
          if (testResult.status === 'ok') {
            this.testPassed = true;
            this.setupStep = 3;
            InfringToast.success(this.setupModal.display_name + ' activated!');
          } else {
            InfringToast.success(this.setupModal.display_name + ' saved. ' + (testResult.message || ''));
          }
        } catch(te) {
          InfringToast.success(this.setupModal.display_name + ' saved. Test to verify connection.');
        }
        await this.refreshStatus();
      } catch(e) {
        InfringToast.error('Failed: ' + (e.message || 'Unknown error'));
      }
      this.configuring = false;
    },

    async removeChannel() {
      if (!this.setupModal) return;
      var name = this.setupModal.name;
      var displayName = this.setupModal.display_name;
      var self = this;
      InfringToast.confirm('Remove Channel', 'Remove ' + displayName + ' configuration? This will deactivate the channel.', async function() {
        try {
          await InfringAPI.delete('/api/channels/' + name + '/configure');
          InfringToast.success(displayName + ' removed and deactivated.');
          await self.refreshStatus();
          self.setupModal = null;
        } catch(e) {
          InfringToast.error('Failed: ' + (e.message || 'Unknown error'));
        }
      });
    },

    async testChannel() {
      if (!this.setupModal) return;
      var name = this.setupModal.name;
      this.testing[name] = true;
      try {
        var result = await InfringAPI.post('/api/channels/' + name + '/test', { force_live: true });
        if (result.status === 'ok') {
          this.testPassed = true;
          this.setupStep = 3;
          InfringToast.success(result.message);
        } else {
          InfringToast.error(result.message);
        }
      } catch(e) {
        InfringToast.error('Test failed: ' + (e.message || 'Unknown error'));
      }
      this.testing[name] = false;
    },

    async copyConfig(ch) {
      var tpl = ch ? ch.config_template : (this.setupModal ? this.setupModal.config_template : '');
      if (!tpl) return;
      try {
        await navigator.clipboard.writeText(tpl);
        InfringToast.success('Copied to clipboard');
      } catch(e) {
        InfringToast.error('Copy failed');
      }
    },

    destroy() {
      if (this.pollTimer) { clearInterval(this.pollTimer); this.pollTimer = null; }
      if (this.qrPollTimer) { clearInterval(this.qrPollTimer); this.qrPollTimer = null; }
    }
  };
}
