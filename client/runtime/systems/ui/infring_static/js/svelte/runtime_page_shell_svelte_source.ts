const COMPONENT_TAG = 'infring-runtime-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-runtime-page-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  let loading = true;
  let loadError = '';
  let uptime = '-';
  let agentCount = 0;
  let version = '-';
  let defaultModel = '-';
  let platform = '-';
  let arch = '-';
  let apiListen = '-';
  let homeDir = '-';
  let logLevel = '-';
  let networkEnabled = false;
  let providers = [];
  let webConduitEnabled = false;
  let webConduitRateLimit = '-';
  let webConduitReceiptsTotal = 0;
  let webConduitRecentDenied = 0;
  let webConduitLastUrl = '-';
  let webConduitRecentReceipts = [];

  function numberValue(value, fallback) {
    var parsed = Number(value);
    if (!Number.isFinite(parsed)) return Number(fallback || 0);
    return parsed;
  }

  function formatUptime(seconds) {
    var diff = Math.max(0, Math.floor(numberValue(seconds, 0)));
    if (diff < 60) return diff + 's';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ' + (diff % 60) + 's';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ' + Math.floor((diff % 3600) / 60) + 'm';
    return Math.floor(diff / 86400) + 'd ' + Math.floor((diff % 86400) / 3600) + 'h';
  }

  function normalizeProviderRows(payload) {
    var source = payload && typeof payload === 'object' ? payload : {};
    var rows = Array.isArray(source.providers) ? source.providers : [];
    return rows
      .map(function(row) { return row && typeof row === 'object' ? row : {}; })
      .filter(function(row) {
        return row.auth_status === 'Configured' || !!row.reachable || !!row.is_local;
      });
  }

  function normalizeWebReceipts(payload) {
    var source = payload && typeof payload === 'object' ? payload : {};
    var rows = Array.isArray(source.receipts) ? source.receipts : [];
    return rows.slice(0, 20).map(function(row) {
      var item = row && typeof row === 'object' ? row : {};
      var requestedUrl = String(item.requested_url || item.url || '');
      var denied = !!item.denied || item.policy_decision === 'deny';
      var domain = String(item.domain || '');
      if (!domain && requestedUrl) {
        try { domain = new URL(requestedUrl).hostname; } catch(_) {}
      }
      return {
        policy_decision: String(item.policy_decision || (denied ? 'deny' : 'allow')),
        domain: domain,
        status_code: numberValue(item.status_code, 0)
      };
    });
  }

  function navigate(page) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(page);
    else if (typeof window !== 'undefined') window.location.hash = page;
  }

  async function loadData() {
    loading = true;
    loadError = '';
    try {
      var api = typeof window !== 'undefined' ? window.InfringAPI : null;
      if (!api || typeof api.get !== 'function') throw new Error('Shell API client is unavailable.');
      var results = await Promise.all([
        api.get('/api/status'),
        api.get('/api/version'),
        api.get('/api/providers'),
        api.get('/api/agents'),
        api.get('/api/web/status').catch(function() { return {}; }),
        api.get('/api/web/receipts?limit=5').catch(function() { return { receipts: [] }; })
      ]);
      var status = results[0] || {};
      var ver = results[1] || {};
      var prov = results[2] || {};
      var agents = results[3] || [];
      var webStatus = results[4] || {};
      var webReceipts = results[5] || { receipts: [] };

      version = String(ver.version || '-');
      platform = String(ver.platform || '-');
      arch = String(ver.arch || '-');
      agentCount = Array.isArray(agents) ? agents.length : numberValue(status.agent_count, 0);
      defaultModel = String(status.default_model || '-');
      apiListen = String(status.api_listen || status.listen || '-');
      homeDir = String(status.home_dir || '-');
      logLevel = String(status.log_level || '-');
      networkEnabled = !!status.network_enabled;
      webConduitEnabled = !!webStatus.enabled;
      webConduitRateLimit = webStatus.policy && webStatus.policy.web_conduit && numberValue(webStatus.policy.web_conduit.rate_limit_per_minute, 0) > 0
        ? String(numberValue(webStatus.policy.web_conduit.rate_limit_per_minute, 0)) + '/min'
        : '-';
      webConduitReceiptsTotal = numberValue(webStatus.receipts_total, 0);
      webConduitRecentDenied = numberValue(webStatus.recent_denied, 0);
      webConduitLastUrl = String((webStatus.last_receipt && webStatus.last_receipt.requested_url) || '-');
      webConduitRecentReceipts = normalizeWebReceipts(webReceipts);
      uptime = formatUptime(status.uptime_seconds);
      providers = normalizeProviderRows(prov);
    } catch(e) {
      loadError = e && e.message ? e.message : 'Could not load runtime status.';
      console.error('Runtime load error:', e);
    }
    loading = false;
  }

  function providerStatusClass(provider) {
    if (provider && provider.reachable) return 'badge-success';
    if (provider && provider.auth_status === 'Configured') return 'badge-success';
    return 'badge-dim';
  }

  function providerStatusLabel(provider) {
    if (provider && provider.reachable) return 'Online';
    if (provider && provider.auth_status === 'Configured') return 'Ready';
    return 'Not configured';
  }

  onMount(loadData);
</script>

<div>
  <div class="page-header page-header-subtabs-center">
    <div>
      <div class="tabs mt-3" role="tablist">
        <div class="tab active" role="tab" on:click={() => navigate('runtime')}>Runtime</div>
        <div class="tab" role="tab" on:click={() => navigate('analytics')}>Analytics</div>
        <div class="tab" role="tab" on:click={() => navigate('logs')}>Logs</div>
      </div>
    </div>
  </div>
  <div class="page-body">
    {#if loading}
      <div class="loading-state"><div class="spinner"></div><span>Loading runtime info...</span></div>
    {:else}
      {#if loadError}
        <div class="error-state"><span class="error-icon">!</span><p>{loadError}</p></div>
      {/if}
      <div class="grid grid-cols-4" style="gap:16px;margin-bottom:24px">
        <div class="card stat-card"><div class="stat-label">Uptime</div><div class="stat-value">{uptime}</div></div>
        <div class="card stat-card"><div class="stat-label">Agents</div><div class="stat-value">{agentCount}</div></div>
        <div class="card stat-card"><div class="stat-label">Version</div><div class="stat-value">{version}</div></div>
        <div class="card stat-card"><div class="stat-label">Default Model</div><div class="stat-value" style="font-size:13px">{defaultModel}</div></div>
      </div>
      <div class="card" style="margin-bottom:16px">
        <div class="card-header">System</div>
        <table class="table" style="margin-top:8px"><tbody>
          <tr><td style="width:180px;font-weight:500">Platform</td><td>{platform}</td></tr>
          <tr><td style="font-weight:500">Architecture</td><td>{arch}</td></tr>
          <tr><td style="font-weight:500">API Listen</td><td>{apiListen}</td></tr>
          <tr><td style="font-weight:500">Home Directory</td><td>{homeDir}</td></tr>
          <tr><td style="font-weight:500">Log Level</td><td>{logLevel}</td></tr>
          <tr><td style="font-weight:500">Network</td><td>{networkEnabled ? 'Enabled' : 'Disabled'}</td></tr>
        </tbody></table>
      </div>
      <div class="card" style="margin-bottom:16px">
        <div class="card-header">Providers</div>
        <table class="table" style="margin-top:8px">
          <thead><tr><th>Provider</th><th>Status</th><th>Models</th><th>Latency</th></tr></thead>
          <tbody>{#each providers as provider (provider.id || provider.display_name)}
            <tr>
              <td>{provider.display_name || provider.id}</td>
              <td><span class={'badge ' + providerStatusClass(provider)}>{providerStatusLabel(provider)}</span></td>
              <td>{provider.model_count}</td>
              <td>{provider.latency_ms ? provider.latency_ms + 'ms' : '-'}</td>
            </tr>
          {/each}</tbody>
        </table>
      </div>
      <div class="card" style="margin-bottom:16px">
        <div class="card-header">Web Conduit</div>
        <table class="table" style="margin-top:8px"><tbody>
          <tr><td style="width:180px;font-weight:500">Enabled</td><td>{webConduitEnabled ? 'Yes' : 'No'}</td></tr>
          <tr><td style="font-weight:500">Rate Limit</td><td>{webConduitRateLimit}</td></tr>
          <tr><td style="font-weight:500">Receipts</td><td>{webConduitReceiptsTotal}</td></tr>
          <tr><td style="font-weight:500">Recent Denied</td><td>{webConduitRecentDenied}</td></tr>
          <tr><td style="font-weight:500">Last URL</td><td>{webConduitLastUrl}</td></tr>
        </tbody></table>
        <table class="table" style="margin-top:12px">
          <thead><tr><th>Decision</th><th>Domain</th><th>Status</th></tr></thead>
          <tbody>{#each webConduitRecentReceipts as receipt, idx ('web-receipt-' + idx)}
            <tr>
              <td><span class={'badge ' + (receipt.policy_decision === 'allow' ? 'badge-success' : 'badge-warn')}>{receipt.policy_decision || '-'}</span></td>
              <td>{receipt.domain || '-'}</td>
              <td>{receipt.status_code || 0}</td>
            </tr>
          {/each}</tbody>
        </table>
      </div>
      <div class="flex gap-2"><button class="btn btn-ghost btn-sm" type="button" on:click={loadData}>Refresh</button></div>
    {/if}
  </div>
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
