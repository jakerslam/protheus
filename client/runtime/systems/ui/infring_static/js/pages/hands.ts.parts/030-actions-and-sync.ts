                x: {
                  grid: { color: gridColor },
                  ticks: { color: textColor, maxTicksLimit: 8, font: { size: 10 } }
                },
                y: {
                  grid: { color: gridColor },
                  ticks: {
                    color: textColor,
                    font: { size: 10 },
                    callback: function(v) {
                      var amount = Number(v);
                      if (!Number.isFinite(amount)) amount = 0;
                      return '$' + amount.toLocaleString();
                    }
                  }
                }
              }
            }
          });
        }
      }

      // ── Daily P&L Bar Chart ──
      if (d.daily_pnl && d.daily_pnl.length > 0) {
        var pnlCanvas = document.getElementById('traderPnlChart');
        if (pnlCanvas && typeof Chart === 'function' && typeof pnlCanvas.getContext === 'function') {
          var pnlLabels = [];
          var pnlValues = [];
          var pnlColors = [];
          for (var j = 0; j < d.daily_pnl.length; j++) {
            pnlLabels.push(d.daily_pnl[j].date || '');
            var pnlRaw = Number(d.daily_pnl[j].pnl);
            var pnlVal = Number.isFinite(pnlRaw) ? pnlRaw : 0;
            pnlValues.push(pnlVal);
            pnlColors.push(pnlVal >= 0 ? successColor : errorColor);
          }

          this._chartPnl = new Chart(pnlCanvas.getContext('2d'), {
            type: 'bar',
            data: {
              labels: pnlLabels,
              datasets: [{
                data: pnlValues,
                backgroundColor: pnlColors,
                borderRadius: 3,
                borderSkipped: false
              }]
            },
            options: {
              responsive: true,
              maintainAspectRatio: false,
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
                      var v = Number(ctx.parsed && ctx.parsed.y);
                      if (!Number.isFinite(v)) v = 0;
                      return (v >= 0 ? '+$' : '-$') + Math.abs(v).toLocaleString(undefined, {minimumFractionDigits: 2, maximumFractionDigits: 2});
                    }
                  }
                }
              },
              scales: {
                x: {
                  grid: { display: false },
                  ticks: { color: textColor, maxTicksLimit: 7, font: { size: 10 } }
                },
                y: {
                  grid: { color: gridColor },
                  ticks: {
                    color: textColor,
                    font: { size: 10 },
                    callback: function(v) {
                      var amount = Number(v);
                      if (!Number.isFinite(amount)) amount = 0;
                      return (amount >= 0 ? '+$' : '-$') + Math.abs(amount).toLocaleString();
                    }
                  }
                }
              }
            }
          });
        }
      }

      // ── Signal Radar Chart ──
      if (d.signal_radar) {
        var radarCanvas = document.getElementById('traderRadarChart');
        if (radarCanvas && typeof Chart === 'function' && typeof radarCanvas.getContext === 'function') {
          var radarLabels = [];
          var radarValues = [];
          var keys = ['technical', 'fundamental', 'sentiment', 'macro'];
          var displayLabels = ['Technical', 'Fundamental', 'Sentiment', 'Macro'];
          for (var k = 0; k < keys.length; k++) {
            radarLabels.push(displayLabels[k]);
            var radarValue = Number(d.signal_radar[keys[k]]);
            radarValues.push(Number.isFinite(radarValue) ? radarValue : 0);
          }

          this._chartRadar = new Chart(radarCanvas.getContext('2d'), {
            type: 'radar',
            data: {
              labels: radarLabels,
              datasets: [{
                data: radarValues,
                borderColor: accentColor,
                backgroundColor: isDark ? 'rgba(37, 99, 235, 0.2)' : 'rgba(37, 99, 235, 0.12)',
                borderWidth: 2,
                pointBackgroundColor: accentColor,
                pointRadius: 4,
                pointHoverRadius: 6
              }]
            },
            options: {
              responsive: true,
              maintainAspectRatio: true,
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
                      var amount = Number(ctx.parsed && ctx.parsed.r);
                      if (!Number.isFinite(amount)) amount = 0;
                      return amount + '/100';
                    }
                  }
                }
              },
              scales: {
                r: {
                  min: 0,
                  max: 100,
                  beginAtZero: true,
                  grid: { color: gridColor },
                  angleLines: { color: gridColor },
                  pointLabels: {
                    color: textColor,
                    font: { size: 11, weight: '600' }
                  },
                  ticks: {
                    color: textColor,
                    backdropColor: 'transparent',
                    stepSize: 25,
                    font: { size: 9 }
                  }
                }
              }
            }
          });
        }
      }
    }
  };
}
