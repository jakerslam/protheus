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
