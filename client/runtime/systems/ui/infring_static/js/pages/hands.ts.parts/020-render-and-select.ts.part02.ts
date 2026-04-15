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

