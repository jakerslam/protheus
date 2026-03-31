            }
          }
        }
      }
      var config = {};
      for (var key in this.settingsValues) {
        config[key] = this.settingsValues[key];
      }
      this.activatingId = handId;
      try {
        var data = await InfringAPI.post('/api/hands/' + handId + '/activate', { config: config });
        this.showToast('Hand "' + handId + '" activated as ' + (data.agent_name || data.instance_id));
        this.closeSetupWizard();
        await this.loadActive();
        this.tab = 'active';
      } catch(e) {
        this.showToast('Activation failed: ' + (e.message || 'unknown error'));
      }
      this.activatingId = null;
    },

    selectOption(settingKey, value) {
      this.settingsValues[settingKey] = value;
    },

    getSettingDisplayValue(setting) {
      var val = this.settingsValues[setting.key] || setting.default || '';
      if (setting.setting_type === 'toggle') {
        return val === 'true' ? 'Enabled' : 'Disabled';
      }
      if (setting.setting_type === 'select' && setting.options) {
        for (var i = 0; i < setting.options.length; i++) {
          if (setting.options[i].value === val) return setting.options[i].label;
        }
      }
      return val || '-';
    },

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

    // ── Browser Viewer ───────────────────────────────────────────────────

    isBrowserHand(inst) {
      return inst.hand_id === 'browser';
    },

    async openBrowserViewer(inst) {
      this.browserViewer = {
        instance_id: inst.instance_id,
        hand_id: inst.hand_id,
        agent_name: inst.agent_name,
        url: '',
        title: '',
        screenshot: '',
        content: '',
        loading: true,
        error: ''
      };
      this.browserViewerOpen = true;
      await this.refreshBrowserView();
      this.startBrowserPolling();
    },

    async refreshBrowserView() {
      if (!this.browserViewer) return;
      var id = this.browserViewer.instance_id;
      try {
        var data = await InfringAPI.get('/api/hands/instances/' + id + '/browser');
        if (data.active) {
          this.browserViewer.url = data.url || '';
          this.browserViewer.title = data.title || '';
          this.browserViewer.screenshot = data.screenshot_base64 || '';
          this.browserViewer.content = data.content || '';
          this.browserViewer.error = '';
        } else {
          this.browserViewer.error = 'No active browser session';
          this.browserViewer.screenshot = '';
        }
      } catch(e) {
        this.browserViewer.error = e.message || 'Could not load browser state';
      }
      this.browserViewer.loading = false;
    },

    startBrowserPolling() {
      var self = this;
      this.stopBrowserPolling();
      this._browserPollTimer = setInterval(function() {
        if (self.browserViewerOpen) {
          self.refreshBrowserView();
        } else {
          self.stopBrowserPolling();
        }
      }, 3000);
    },

    stopBrowserPolling() {
      if (this._browserPollTimer) {
        clearInterval(this._browserPollTimer);
        this._browserPollTimer = null;
      }
    },

    closeBrowserViewer() {
      this.stopBrowserPolling();
      this.browserViewerOpen = false;
      this.browserViewer = null;
    },

    // ── Trader Dashboard ──────────────────────────────────────────────────

    isTraderHand(inst) {
      return inst.hand_id === 'trader';
    },

    async openDashboard(inst) {
      this._dashboardInst = inst;
      this.dashboardOpen = true;
      this.dashboardLoading = true;
      this.dashboardData = null;
      await this._fetchDashboardData(inst);
      this.dashboardLoading = false;
      // Render charts after DOM update
      var self = this;
      setTimeout(function() { self._renderCharts(); }, 60);
    },

    async refreshDashboard() {
      if (!this._dashboardInst) return;
      this.dashboardLoading = true;
      await this._fetchDashboardData(this._dashboardInst);
      this.dashboardLoading = false;
      var self = this;
      setTimeout(function() { self._renderCharts(); }, 60);
    },

    closeDashboard() {
      this.dashboardOpen = false;
      this._destroyCharts();
      this.dashboardData = null;
      this._dashboardInst = null;
    },

    async _fetchDashboardData(inst) {
      var data = {
        agent_name: inst.agent_name || inst.hand_id,
        portfolio_value: null,
        total_pnl: null,
        win_rate: null,
        sharpe_ratio: null,
        max_drawdown: null,
        trades_count: null,
        equity_curve: [],
        daily_pnl: [],
        watchlist_heatmap: [],
        signal_radar: null,
        recent_trades: []
      };

      // Fetch basic stats from the hand stats endpoint
      try {
        var stats = await InfringAPI.get('/api/hands/instances/' + inst.instance_id + '/stats');
        var m = stats.metrics || {};
        if (m['Portfolio Value']) data.portfolio_value = this._metricVal(m['Portfolio Value']);
        if (m['Total P&L']) data.total_pnl = this._metricVal(m['Total P&L']);
        if (m['Win Rate']) data.win_rate = this._metricVal(m['Win Rate']);
        if (m['Sharpe Ratio']) data.sharpe_ratio = this._metricVal(m['Sharpe Ratio']);
        if (m['Max Drawdown']) data.max_drawdown = this._metricVal(m['Max Drawdown']);
        if (m['Trades Executed']) data.trades_count = this._metricVal(m['Trades Executed']);
      } catch(e) {
        // Stats endpoint might fail — continue with KV data
      }

      // Fetch rich chart data from agent memory KV
      var agentId = inst.agent_id || 'shared';
      var kvKeys = [
        'trader_hand_equity_curve',
        'trader_hand_daily_pnl',
        'trader_hand_watchlist_heatmap',
        'trader_hand_signal_radar',
        'trader_hand_recent_trades',
        'trader_hand_portfolio_value',
        'trader_hand_total_pnl',
        'trader_hand_win_rate',
        'trader_hand_sharpe_ratio',
        'trader_hand_max_drawdown',
        'trader_hand_trades_count'
      ];

      for (var i = 0; i < kvKeys.length; i++) {
        try {
          var resp = await InfringAPI.get('/api/memory/agents/' + agentId + '/kv/' + kvKeys[i]);
          if (resp && resp.value !== null && resp.value !== undefined) {
            var val = resp.value;
            this._applyKvToData(data, kvKeys[i], val);
          }
        } catch(e) {
          // Key might not exist yet — that's fine
        }
      }

      this.dashboardData = data;
    },

    _metricVal(metric) {
      if (!metric) return null;
      var v = metric.value;
      if (v === null || v === undefined) return null;
      // Values come as JSON values — could be string, number, etc.
      if (typeof v === 'string') return v;
      return String(v);
    },

    _applyKvToData(data, key, val) {
      // Values from KV can be strings (JSON-encoded) or already parsed
      var parsed = val;
      if (typeof val === 'string') {
        try { parsed = JSON.parse(val); } catch(e) { parsed = val; }
      }

      switch(key) {
        case 'trader_hand_portfolio_value':
          if (!data.portfolio_value) data.portfolio_value = String(parsed);
          break;
        case 'trader_hand_total_pnl':
          if (!data.total_pnl) data.total_pnl = String(parsed);
          break;
        case 'trader_hand_win_rate':
          if (!data.win_rate) data.win_rate = String(parsed);
          break;
        case 'trader_hand_sharpe_ratio':
          if (!data.sharpe_ratio) data.sharpe_ratio = String(parsed);
          break;
        case 'trader_hand_max_drawdown':
          if (!data.max_drawdown) data.max_drawdown = String(parsed);
          break;
        case 'trader_hand_trades_count':
          if (!data.trades_count) data.trades_count = String(parsed);
          break;
        case 'trader_hand_equity_curve':
          if (Array.isArray(parsed)) data.equity_curve = parsed;
          break;
        case 'trader_hand_daily_pnl':
          if (Array.isArray(parsed)) data.daily_pnl = parsed;
          break;
        case 'trader_hand_watchlist_heatmap':
          if (Array.isArray(parsed)) data.watchlist_heatmap = parsed;
          break;
        case 'trader_hand_signal_radar':
          if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) data.signal_radar = parsed;
          break;
        case 'trader_hand_recent_trades':
          if (Array.isArray(parsed)) data.recent_trades = parsed;
          break;
      }
    },

    _destroyCharts() {
      if (this._chartEquity) { this._chartEquity.destroy(); this._chartEquity = null; }
      if (this._chartPnl) { this._chartPnl.destroy(); this._chartPnl = null; }
      if (this._chartRadar) { this._chartRadar.destroy(); this._chartRadar = null; }
    },

    _renderCharts() {
      if (typeof Chart === 'undefined') return;
      this._destroyCharts();
      if (!this.dashboardData) return;

      var d = this.dashboardData;

      // Detect theme
      var isDark = document.documentElement.getAttribute('data-theme') === 'dark' ||
        (!document.documentElement.getAttribute('data-theme') && window.matchMedia('(prefers-color-scheme: dark)').matches);
      var gridColor = isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)';
      var textColor = isDark ? '#8A8380' : '#6B6560';
      var accentColor = '#2563EB';
      var successColor = isDark ? '#4ADE80' : '#22C55E';
      var errorColor = '#EF4444';

      // ── Equity Curve ──
      if (d.equity_curve && d.equity_curve.length > 0) {
        var eqCanvas = document.getElementById('traderEquityChart');
        if (eqCanvas) {
          var labels = [];
          var values = [];
          for (var i = 0; i < d.equity_curve.length; i++) {
            labels.push(d.equity_curve[i].date || '');
            values.push(parseFloat(d.equity_curve[i].value) || 0);
          }
          // Determine gradient
          var eqCtx = eqCanvas.getContext('2d');
          var gradient = eqCtx.createLinearGradient(0, 0, 0, eqCanvas.parentElement.clientHeight || 180);
          gradient.addColorStop(0, isDark ? 'rgba(37, 99, 235, 0.25)' : 'rgba(37, 99, 235, 0.15)');
          gradient.addColorStop(1, 'rgba(37, 99, 235, 0)');

          this._chartEquity = new Chart(eqCtx, {
            type: 'line',
            data: {
              labels: labels,
              datasets: [{
                data: values,
                borderColor: accentColor,
                backgroundColor: gradient,
                borderWidth: 2,
                fill: true,
                tension: 0.3,
                pointRadius: d.equity_curve.length > 20 ? 0 : 3,
                pointHoverRadius: 5,
                pointBackgroundColor: accentColor
              }]
            },
            options: {
              responsive: true,
              maintainAspectRatio: false,
              interaction: { mode: 'index', intersect: false },
              plugins: {
                legend: { display: false },
                tooltip: {
                  backgroundColor: isDark ? '#1a1a1a' : '#fff',
                  titleColor: textColor,
                  bodyColor: isDark ? '#e0e0e0' : '#333',
                  borderColor: gridColor,
                  borderWidth: 1,
                  padding: 10,
                  callbacks: {
                    label: function(ctx) {
                      return '$' + ctx.parsed.y.toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2});
                    }
                  }
                }
              },
              scales: {
