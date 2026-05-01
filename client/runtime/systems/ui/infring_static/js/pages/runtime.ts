// Runtime page — system overview and provider status
function runtimeNumber(value, fallback) {
  var parsed = Number(value);
  if (!Number.isFinite(parsed)) return Number(fallback || 0);
  return parsed;
}

function runtimeFormatUptime(seconds) {
  var diff = Math.max(0, Math.floor(runtimeNumber(seconds, 0)));
  if (diff < 60) return diff + 's';
  if (diff < 3600) return Math.floor(diff / 60) + 'm ' + (diff % 60) + 's';
  if (diff < 86400) return Math.floor(diff / 3600) + 'h ' + Math.floor((diff % 3600) / 60) + 'm';
  return Math.floor(diff / 86400) + 'd ' + Math.floor((diff % 86400) / 3600) + 'h';
}

function runtimeNormalizeProviderRows(payload) {
  var source = payload && typeof payload === 'object' ? payload : {};
  var rows = Array.isArray(source.providers) ? source.providers : [];
  return rows
    .map(function(row) { return row && typeof row === 'object' ? row : {}; })
    .filter(function(row) {
      return row.auth_status === 'Configured' || !!row.reachable || !!row.is_local;
    });
}

function runtimeNormalizeWebReceipts(payload) {
  var source = payload && typeof payload === 'object' ? payload : {};
  var rows = Array.isArray(source.receipts) ? source.receipts : [];
  return rows.slice(0, 20).map(function(row) {
    var item = row && typeof row === 'object' ? row : {};
    return {
      requested_url: String(item.requested_url || ''),
      status_code: runtimeNumber(item.status_code, 0),
      denied: !!item.denied,
      error: String(item.error || ''),
      ts: String(item.ts || item.timestamp || '')
    };
  });
}

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
      loadError: '',

      async loadData() {
        this.loading = true;
        this.loadError = '';
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

          this.version = String(ver.version || '-');
          this.platform = String(ver.platform || '-');
          this.arch = String(ver.arch || '-');
          this.agentCount = Array.isArray(agents) ? agents.length : runtimeNumber(status.agent_count, 0);
          this.defaultModel = String(status.default_model || '-');
          this.apiListen = String(status.api_listen || status.listen || '-');
          this.homeDir = String(status.home_dir || '-');
          this.logLevel = String(status.log_level || '-');
          this.networkEnabled = !!status.network_enabled;
          this.webConduitEnabled = !!webStatus.enabled;
          this.webConduitRateLimit = (webStatus.policy && webStatus.policy.web_conduit && runtimeNumber(webStatus.policy.web_conduit.rate_limit_per_minute, 0) > 0)
            ? String(runtimeNumber(webStatus.policy.web_conduit.rate_limit_per_minute, 0)) + '/min'
            : '-';
          this.webConduitReceiptsTotal = runtimeNumber(webStatus.receipts_total, 0);
          this.webConduitRecentDenied = runtimeNumber(webStatus.recent_denied, 0);
          this.webConduitLastUrl = String((webStatus.last_receipt && webStatus.last_receipt.requested_url) || '-');
          this.webConduitRecentReceipts = runtimeNormalizeWebReceipts(webReceipts);
          this.uptime = runtimeFormatUptime(status.uptime_seconds);
          this.providers = runtimeNormalizeProviderRows(prov);
        } catch(e) {
          this.loadError = e && e.message ? e.message : 'Could not load runtime status.';
          console.error('Runtime load error:', e);
        }
        this.loading = false;
      }
    };
  });
});
