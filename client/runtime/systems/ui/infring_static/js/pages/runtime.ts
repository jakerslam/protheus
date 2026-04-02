// Runtime page — system overview and provider status
document.addEventListener('alpine:init', function() {
  Alpine.data('runtimePage', function() {
    return {
      loading: true,
      uptime: '-',
      agentCount: 0,
      version: '-',
      defaultModel: '-',
      platform: '-',
      arch: '-',
      apiListen: '-',
      homeDir: '-',
      logLevel: '-',
      networkEnabled: false,
      providers: [],
      webConduitEnabled: false,
      webConduitRateLimit: '-',
      webConduitReceiptsTotal: 0,
      webConduitRecentDenied: 0,
      webConduitLastUrl: '-',
      webConduitRecentReceipts: [],

      async loadData() {
        this.loading = true;
        try {
          var results = await Promise.all([
            InfringAPI.get('/api/status'),
            InfringAPI.get('/api/version'),
            InfringAPI.get('/api/providers'),
            InfringAPI.get('/api/agents'),
            InfringAPI.get('/api/web/status').catch(function() { return {}; }),
            InfringAPI.get('/api/web/receipts?limit=5').catch(function() { return { receipts: [] }; })
          ]);
          var status = results[0];
          var ver = results[1];
          var prov = results[2];
          var agents = results[3];
          var webStatus = results[4] || {};
          var webReceipts = results[5] || { receipts: [] };

          this.version = ver.version || '-';
          this.platform = ver.platform || '-';
          this.arch = ver.arch || '-';
          this.agentCount = Array.isArray(agents) ? agents.length : 0;
          this.defaultModel = status.default_model || '-';
          this.apiListen = status.api_listen || status.listen || '-';
          this.homeDir = status.home_dir || '-';
          this.logLevel = status.log_level || '-';
          this.networkEnabled = !!status.network_enabled;
          this.webConduitEnabled = !!webStatus.enabled;
          this.webConduitRateLimit = (webStatus.policy && webStatus.policy.web_conduit && webStatus.policy.web_conduit.rate_limit_per_minute)
            ? String(webStatus.policy.web_conduit.rate_limit_per_minute) + '/min'
            : '-';
          this.webConduitReceiptsTotal = Number(webStatus.receipts_total || 0);
          this.webConduitRecentDenied = Number(webStatus.recent_denied || 0);
          this.webConduitLastUrl = (webStatus.last_receipt && webStatus.last_receipt.requested_url) || '-';
          this.webConduitRecentReceipts = Array.isArray(webReceipts.receipts) ? webReceipts.receipts : [];

          // Compute uptime from uptime_seconds
          var diff = status.uptime_seconds || 0;
          if (diff < 60) this.uptime = diff + 's';
          else if (diff < 3600) this.uptime = Math.floor(diff / 60) + 'm ' + (diff % 60) + 's';
          else if (diff < 86400) this.uptime = Math.floor(diff / 3600) + 'h ' + Math.floor((diff % 3600) / 60) + 'm';
          else this.uptime = Math.floor(diff / 86400) + 'd ' + Math.floor((diff % 86400) / 3600) + 'h';

          this.providers = (prov.providers || []).filter(function(p) {
            return p.auth_status === 'Configured' || p.reachable || p.is_local;
          });
        } catch(e) {
          console.error('Runtime load error:', e);
        }
        this.loading = false;
      }
    };
  });
});
