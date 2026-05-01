const COMPONENT_TAG = 'infring-overview-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-overview-page-shell', shadow: 'none' }} />
<script>
  import { onDestroy, onMount } from 'svelte';

  let health = {};
  let status = {};
  let usageSummary = {};
  let recentAudit = [];
  let channels = [];
  let providers = [];
  let mcpServers = [];
  let skillCount = 0;
  let loading = true;
  let loadError = '';
  let refreshTimer = null;
  let checklistDismissed = false;

  $: configuredProviders = providers.filter(function(provider) {
    return normalizeAuthStatus(provider && provider.auth_status) === 'configured';
  });
  $: connectedMcp = mcpServers.filter(function(server) {
    return String(server && server.status || '').toLowerCase() === 'connected';
  });
  $: setupChecklist = buildSetupChecklist();
  $: setupDoneCount = setupChecklist.filter(function(item) { return item.done; }).length;
  $: setupProgress = setupChecklist.length ? (setupDoneCount / setupChecklist.length) * 100 : 0;
  $: showOnboarding = readShowOnboarding();

  function apiClient() {
    return typeof window !== 'undefined' ? window.InfringAPI : null;
  }

  function appStore() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    var bridge = services && services.appStore ? services.appStore : null;
    return bridge && typeof bridge.current === 'function' ? bridge.current() : null;
  }

  function navigate(page) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(page);
    else if (typeof window !== 'undefined') window.location.hash = page;
  }

  function readShowOnboarding() {
    var store = appStore();
    return !!(store && store.showOnboarding);
  }

  function dismissOnboarding() {
    var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
    var bridge = services && services.appStore ? services.appStore : null;
    var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
    if (store && typeof store.dismissOnboarding === 'function') store.dismissOnboarding();
    showOnboarding = false;
  }

  function dismissChecklist() {
    checklistDismissed = true;
    try { window.localStorage.setItem('of-checklist-dismissed', 'true'); } catch (_) {}
  }

  function normalizeAuthStatus(value) {
    return String(value || '').trim().toLowerCase();
  }

  function buildSetupChecklist() {
    var store = appStore();
    var agents = (store && Array.isArray(store.agents)) ? store.agents : [];
    var firstMessageSent = false;
    var skillBrowsed = false;
    try {
      firstMessageSent = window.localStorage.getItem('of-first-msg') === 'true';
      skillBrowsed = window.localStorage.getItem('of-skill-browsed') === 'true';
    } catch (_) {}
    return [
      { key: 'provider', label: 'Configure an LLM provider', done: configuredProviders.length > 0, action: 'settings' },
      { key: 'agent', label: 'Create your first agent', done: agents.length > 0, action: 'agents' },
      { key: 'chat', label: 'Send your first message', done: firstMessageSent, action: 'chat' },
      { key: 'channel', label: 'Connect a messaging channel', done: channels.length > 0, action: 'channels' },
      { key: 'skill', label: 'Browse or install a skill', done: skillBrowsed, action: 'skills' }
    ];
  }

  function formatUptime(secs) {
    if (!secs) return '-';
    var total = Math.max(0, Math.floor(Number(secs) || 0));
    var days = Math.floor(total / 86400);
    var hours = Math.floor((total % 86400) / 3600);
    var minutes = Math.floor((total % 3600) / 60);
    if (days > 0) return days + 'd ' + hours + 'h';
    if (hours > 0) return hours + 'h ' + minutes + 'm';
    return minutes + 'm';
  }

  function formatNumber(value) {
    var number = Number(value || 0);
    if (!Number.isFinite(number) || number <= 0) return '0';
    if (number >= 1000000) return (number / 1000000).toFixed(1) + 'M';
    if (number >= 1000) return (number / 1000).toFixed(1) + 'K';
    return String(number);
  }

  function formatCost(value) {
    var number = Number(value || 0);
    if (!Number.isFinite(number) || number <= 0) return '$0.00';
    if (number < 0.01) return '<$0.01';
    return '$' + number.toFixed(2);
  }

  function timeAgo(timestamp) {
    if (!timestamp) return '';
    var parsed = new Date(timestamp).getTime();
    if (!Number.isFinite(parsed)) return '';
    var diff = Math.floor((Date.now() - parsed) / 1000);
    if (diff < 10) return 'just now';
    if (diff < 60) return diff + 's ago';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return Math.floor(diff / 86400) + 'd ago';
  }

  function providerBadgeClass(provider) {
    var auth = normalizeAuthStatus(provider && provider.auth_status);
    var healthState = String(provider && provider.health || '').toLowerCase();
    if (auth === 'configured') {
      if (healthState === 'cooldown' || healthState === 'open') return 'badge-warn';
      return 'badge-success';
    }
    if (auth === 'not_set' || auth === 'missing') return 'badge-muted';
    return 'badge-dim';
  }

  function providerTooltip(provider) {
    var name = String(provider && provider.display_name || provider && provider.id || 'Provider');
    var auth = normalizeAuthStatus(provider && provider.auth_status);
    var healthState = String(provider && provider.health || '').toLowerCase();
    if (healthState === 'cooldown') return name + ' - cooling down (rate limited)';
    if (healthState === 'open') return name + ' - circuit breaker open';
    if (auth === 'configured') return name + ' - ready';
    return name + ' - not configured';
  }

  function friendlyAction(action) {
    var map = {
      AgentSpawn: 'Agent Created',
      AgentKill: 'Agent Stopped',
      AgentTerminated: 'Agent Stopped',
      ToolInvoke: 'Tool Used',
      ToolResult: 'Tool Completed',
      MessageReceived: 'Message In',
      MessageSent: 'Response Sent',
      SessionReset: 'Session Reset',
      SessionCompact: 'Compacted',
      ModelSwitch: 'Model Changed',
      AuthAttempt: 'Login Attempt',
      AuthSuccess: 'Login OK',
      AuthFailure: 'Login Failed',
      CapabilityDenied: 'Denied',
      RateLimited: 'Rate Limited',
      WorkflowRun: 'Workflow Run',
      TriggerFired: 'Trigger Fired',
      SkillInstalled: 'Skill Installed',
      McpConnected: 'MCP Connected'
    };
    if (!action) return 'Unknown';
    return map[action] || String(action).replace(/([A-Z])/g, ' $1').trim();
  }

  function actionBadgeClass(action) {
    if (!action) return 'badge-dim';
    if (action === 'AgentSpawn' || action === 'AuthSuccess') return 'badge-success';
    if (action === 'AgentKill' || action === 'AgentTerminated' || action === 'AuthFailure' || action === 'CapabilityDenied') return 'badge-error';
    if (action === 'RateLimited' || action === 'ToolInvoke') return 'badge-warn';
    return 'badge-created';
  }

  function actionGlyph(action) {
    if (action === 'AgentSpawn') return '+';
    if (action === 'AgentKill' || action === 'AgentTerminated') return 'x';
    if (action === 'ToolInvoke') return '*';
    if (action === 'MessageReceived') return '<';
    if (action === 'MessageSent') return '>';
    return 'o';
  }

  function agentName(agentId) {
    if (!agentId) return '-';
    var store = appStore();
    var agents = (store && Array.isArray(store.agents)) ? store.agents : [];
    var agent = agents.find(function(row) { return row && row.id === agentId; });
    return agent ? agent.name : String(agentId).substring(0, 8) + '...';
  }

  async function safeGet(path, fallback) {
    var api = apiClient();
    if (!api || typeof api.get !== 'function') throw new Error('Shell API client is unavailable.');
    try { return await api.get(path); } catch (_) { return fallback; }
  }

  async function loadHealth() {
    health = await safeGet('/api/health', { status: 'unreachable' });
  }

  async function loadStatus() {
    var api = apiClient();
    if (!api || typeof api.get !== 'function') throw new Error('Shell API client is unavailable.');
    status = await api.get('/api/status');
  }

  async function loadUsage() {
    var data = await safeGet('/api/usage', { agents: [] });
    var agents = Array.isArray(data && data.agents) ? data.agents : [];
    usageSummary = agents.reduce(function(acc, agent) {
      acc.total_tokens += Number(agent.total_tokens || 0);
      acc.total_tools += Number(agent.tool_calls || 0);
      acc.total_cost += Number(agent.cost_usd || 0);
      acc.agent_count += 1;
      return acc;
    }, { total_tokens: 0, total_tools: 0, total_cost: 0, agent_count: 0 });
  }

  async function loadAudit() {
    var data = await safeGet('/api/audit/recent?n=8', { entries: [] });
    recentAudit = Array.isArray(data && data.entries) ? data.entries : [];
  }

  async function loadChannels() {
    var data = await safeGet('/api/channels', { channels: [] });
    channels = (Array.isArray(data && data.channels) ? data.channels : []).filter(function(channel) { return !!channel.has_token; });
  }

  async function loadProviders() {
    var data = await safeGet('/api/providers', { providers: [] });
    providers = Array.isArray(data && data.providers) ? data.providers : [];
  }

  async function loadMcpServers() {
    var data = await safeGet('/api/mcp/servers', { servers: [] });
    mcpServers = Array.isArray(data && data.servers) ? data.servers : [];
  }

  async function loadSkills() {
    var data = await safeGet('/api/skills', { skills: [] });
    skillCount = Array.isArray(data && data.skills) ? data.skills.length : 0;
  }

  async function loadOverview(showSpinner) {
    if (showSpinner) {
      loading = true;
      loadError = '';
    }
    try {
      await Promise.all([
        loadHealth(),
        loadStatus(),
        loadUsage(),
        loadAudit(),
        loadChannels(),
        loadProviders(),
        loadMcpServers(),
        loadSkills()
      ]);
    } catch (e) {
      if (showSpinner) loadError = e && e.message ? e.message : 'Could not load overview data.';
    }
    if (showSpinner) loading = false;
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    refreshTimer = setInterval(function() { loadOverview(false); }, 30000);
  }

  function stopAutoRefresh() {
    if (refreshTimer) {
      clearInterval(refreshTimer);
      refreshTimer = null;
    }
  }

  onMount(function() {
    try { checklistDismissed = window.localStorage.getItem('of-checklist-dismissed') === 'true'; } catch (_) {}
    loadOverview(true).then(startAutoRefresh);
  });

  onDestroy(stopAutoRefresh);
</script>

<div class="page-body">
  {#if loading}
    <div style="animation:fadeIn 0.2s">
      <div class="stats-row stats-row-lg" style="margin-bottom:20px">
        <div class="stat-card stat-card-lg"><div class="skeleton skeleton-heading" style="width:60px;height:28px"></div><div class="skeleton skeleton-text" style="width:100px;height:12px;margin-top:8px"></div></div>
        <div class="stat-card stat-card-lg"><div class="skeleton skeleton-heading" style="width:40px;height:28px"></div><div class="skeleton skeleton-text" style="width:120px;height:12px;margin-top:8px"></div></div>
        <div class="stat-card stat-card-lg"><div class="skeleton skeleton-heading" style="width:50px;height:28px"></div><div class="skeleton skeleton-text" style="width:80px;height:12px;margin-top:8px"></div></div>
        <div class="stat-card stat-card-lg"><div class="skeleton skeleton-heading" style="width:40px;height:28px"></div><div class="skeleton skeleton-text" style="width:60px;height:12px;margin-top:8px"></div></div>
      </div>
      <div class="overview-grid"><div class="skeleton skeleton-card"></div><div class="skeleton skeleton-card"></div></div>
    </div>
  {:else if loadError}
    <div class="error-state" style="animation:fadeIn 0.3s">
      <div class="empty-state-icon">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--error)" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/></svg>
      </div>
      <h3 style="color:var(--error)">Connection Error</h3>
      <p class="text-xs text-dim">{loadError}</p>
      <button class="btn btn-primary btn-sm" type="button" on:click={() => loadOverview(true)} style="margin-top:8px">Retry</button>
    </div>
  {:else}
    {#if setupProgress < 100 && !checklistDismissed}
      <div class="setup-checklist" style="animation:slideUp 0.3s var(--ease-spring)">
        <div class="card" style="border-left:4px solid var(--accent)">
          <div class="flex justify-between items-center mb-2">
            <div>
              <div class="card-header" style="margin:0">Getting Started</div>
              <div class="text-xs text-dim">{setupDoneCount} of 5 steps completed</div>
            </div>
            <div class="flex gap-2">
              <button class="btn btn-primary btn-sm" type="button" on:click={() => navigate('wizard')}>Setup Wizard</button>
              <button class="btn btn-ghost btn-sm" type="button" on:click={dismissChecklist}>Dismiss</button>
            </div>
          </div>
          <div class="progress-bar mb-2" style="margin-top:8px">
            <div class="progress-bar-fill" style={'width:' + setupProgress + '%'}></div>
          </div>
          {#each setupChecklist as item (item.key)}
            <div class="setup-checklist-item">
              <div class:done={item.done} class="setup-checklist-icon">
                <span>{item.done ? '✓' : '○'}</span>
              </div>
              <span style={item.done ? 'flex:1;text-decoration:line-through;opacity:0.6' : 'flex:1'}>{item.label}</span>
              {#if !item.done}
                <button class="btn btn-ghost btn-sm" type="button" style="font-size:10px;padding:3px 8px" on:click={() => navigate(item.action)}>Go</button>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

    {#if showOnboarding && checklistDismissed}
      <div class="onboarding-banner">
        <h3>Welcome to Infring</h3>
        <p style="font-size:12px;color:var(--text-dim)">Get started quickly with the guided Setup Wizard, or configure manually:</p>
        <div class="flex gap-2">
          <button class="btn btn-primary" type="button" on:click={() => navigate('wizard')}>Launch Setup Wizard</button>
          <button class="btn btn-ghost" type="button" on:click={() => navigate('settings')}>Configure Manually</button>
          <button class="btn btn-ghost" type="button" on:click={dismissOnboarding}>Dismiss</button>
        </div>
      </div>
    {/if}

    <div style="animation:fadeIn 0.3s">
      <div class="stats-row stats-row-lg">
        <div class="stat-card stat-card-lg animate-entry stagger-1" on:click={() => navigate('agents')} style="cursor:pointer">
          <div style="display:flex;align-items:center;gap:8px">
            <div style="width:36px;height:36px;border-radius:var(--radius-md);background:var(--accent-subtle);display:flex;align-items:center;justify-content:center;flex-shrink:0">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>
            </div>
            <div><div class:stat-value-success={(status.agent_count || 0) > 0} class="stat-value">{status.agent_count || 0}</div><div class="stat-label">Agents Running</div></div>
          </div>
        </div>
        <div class="stat-card stat-card-lg animate-entry stagger-2"><div class="stat-value">{formatNumber(usageSummary.total_tokens)}</div><div class="stat-label">Tokens Used</div></div>
        <div class="stat-card stat-card-lg animate-entry stagger-3"><div class="stat-value stat-value-accent">{formatCost(usageSummary.total_cost)}</div><div class="stat-label">Total Cost</div></div>
        <div class="stat-card stat-card-lg animate-entry stagger-4"><div class="stat-value">{formatUptime(status.uptime_seconds)}</div><div class="stat-label">Uptime</div></div>
      </div>

      <div class="stats-row animate-entry stagger-5" style="margin-bottom:16px">
        <div class="stat-card" on:click={() => navigate('channels')} style="cursor:pointer"><div class="stat-value" style="font-size:18px">{channels.length}</div><div class="stat-label">Channels</div></div>
        <div class="stat-card" on:click={() => navigate('skills')} style="cursor:pointer"><div class="stat-value" style="font-size:18px">{skillCount}</div><div class="stat-label">Plugins</div></div>
        <div class="stat-card"><div class="stat-value" style="font-size:18px">{connectedMcp.length}</div><div class="stat-label">MCP Servers</div></div>
        <div class="stat-card"><div class="stat-value" style="font-size:18px">{formatNumber(usageSummary.total_tools)}</div><div class="stat-label">Tool Calls</div></div>
        <div class="stat-card"><div class:stat-value-success={configuredProviders.length > 0} class="stat-value" style="font-size:18px">{configuredProviders.length}</div><div class="stat-label">Providers</div></div>
      </div>

      {#if providers.length}
        <div class="card mb-4" style="overflow:hidden">
          <div class="flex justify-between items-center mb-2">
            <div class="card-header" style="margin:0">LLM Providers</div>
            <span class="text-xs text-dim">{configuredProviders.length}/{providers.length} configured</span>
          </div>
          <div style="display:flex;flex-wrap:wrap;gap:6px;margin-top:8px">
            {#each providers as provider (provider.id || provider.display_name)}
              <button class={'badge ' + providerBadgeClass(provider)} title={providerTooltip(provider)} type="button" style="cursor:pointer;transition:all 0.15s var(--ease-spring);padding:4px 10px" on:click={() => navigate('settings')}>
                {#if normalizeAuthStatus(provider.auth_status) === 'configured'}
                  <span style={'display:inline-block;width:6px;height:6px;border-radius:50%;margin-right:4px;' + ((provider.health === 'cooldown' || provider.health === 'open') ? 'background:var(--warning);animation:pulse-ring 1.5s infinite' : 'background:var(--success)')}></span>
                {/if}
                <span>{provider.display_name}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <div class="overview-grid">
        <div class="card">
          <div class="card-header">System Health</div>
          <div class="detail-grid" style="margin-top:8px">
            <div class="detail-row"><span class="detail-label">Status</span><span class={(health.status === 'ok' ? 'badge badge-running' : 'badge badge-crashed')}>{health.status || 'unknown'}</span></div>
            <div class="detail-row"><span class="detail-label">Version</span><span class="detail-value font-mono">{status.version || '-'}</span></div>
            <div class="detail-row"><span class="detail-label">Provider</span><span class="detail-value">{status.default_provider || '-'}</span></div>
            <div class="detail-row"><span class="detail-label">Model</span><span class="detail-value font-mono" style="font-size:11px">{status.default_model || '-'}</span></div>
          </div>
        </div>
        <div class="card">
          <div class="card-header">Security Systems</div>
          <div style="display:flex;flex-wrap:wrap;gap:5px;margin-top:8px">
            {#each ['Merkle Audit', 'Taint Tracking', 'WASM Sandbox', 'GCRA Rate Limit', 'Ed25519 Signing', 'SSRF Protection', 'Secret Zeroize', 'Loop Guard', 'Session Repair'] as label}
              <span class="badge badge-success" style="font-size:9px;padding:2px 8px">{label}</span>
            {/each}
          </div>
          <div class="text-xs text-dim" style="margin-top:8px">9 defense-in-depth systems active</div>
        </div>
        {#if channels.length}
          <div class="card">
            <div class="card-header">Connected Channels</div>
            <div style="display:flex;flex-wrap:wrap;gap:5px;margin-top:8px">
              {#each channels as channel (channel.name)}
                <span class="badge badge-info" style="font-size:9px;text-transform:capitalize;padding:2px 8px">{channel.name}</span>
              {/each}
            </div>
            <div class="text-xs text-dim" style="margin-top:8px">{channels.length} channel(s) connected</div>
          </div>
        {/if}
        {#if mcpServers.length}
          <div class="card">
            <div class="card-header">MCP Servers</div>
            <div style="display:flex;flex-wrap:wrap;gap:5px;margin-top:8px">
              {#each mcpServers as server (server.name)}
                <div class={(server.status === 'connected' ? 'badge badge-success' : 'badge badge-dim')} style="font-size:9px;padding:2px 8px">
                  <span>{server.name}</span>{#if server.tool_count}<span class="text-xs text-dim"> ({server.tool_count} tools)</span>{/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <div class="card mt-4" style="background:var(--bg-elevated)">
        <div class="card-header" style="margin-bottom:8px">Quick Actions</div>
        <div style="display:flex;flex-wrap:wrap;gap:8px">
          <button class="btn btn-ghost btn-sm" type="button" on:click={() => navigate('agents')}>New Agent</button>
          <button class="btn btn-ghost btn-sm" type="button" on:click={() => navigate('skills')}>Browse Plugins</button>
          <button class="btn btn-ghost btn-sm" type="button" on:click={() => navigate('channels')}>Add Channel</button>
          <button class="btn btn-ghost btn-sm" type="button" on:click={() => navigate('workflows')}>Create Workflow</button>
          <button class="btn btn-ghost btn-sm" type="button" on:click={() => navigate('settings')}>Settings</button>
        </div>
      </div>

      {#if recentAudit.length}
        <div class="card mt-4">
          <div class="flex justify-between items-center">
            <div class="card-header" style="margin:0">Recent Activity</div>
            <button class="btn btn-ghost btn-sm" type="button" on:click={() => navigate('logs')} style="font-size:10px;padding:2px 8px">View All</button>
          </div>
          <div style="margin-top:12px;display:flex;flex-direction:column;gap:1px">
            {#each recentAudit as entry, idx (entry.seq || idx)}
              <div class="detail-row" style={'padding:8px 0;border-bottom:1px solid var(--border-subtle);animation:slideUp 0.2s var(--ease-spring);animation-delay:' + (idx * 30) + 'ms'}>
                <div style="display:flex;align-items:center;gap:8px;flex:1;min-width:0">
                  <div style="width:24px;height:24px;border-radius:var(--radius-sm);background:var(--surface-2);display:flex;align-items:center;justify-content:center;flex-shrink:0;color:var(--text-dim)">{actionGlyph(entry.action)}</div>
                  <div style="min-width:0;flex:1">
                    <div style="display:flex;align-items:center;gap:6px">
                      <span class={'badge ' + actionBadgeClass(entry.action)} style="font-size:9px;padding:1px 6px">{friendlyAction(entry.action)}</span>
                      <span class="text-xs text-dim truncate" style="max-width:100px" title={entry.agent_id}>{agentName(entry.agent_id)}</span>
                    </div>
                    {#if entry.detail}
                      <div class="text-xs text-dim truncate" style="margin-top:2px;max-width:300px" title={entry.detail}>{entry.detail}</div>
                    {/if}
                  </div>
                </div>
                <span class="text-xs text-dim font-mono" style="white-space:nowrap;flex-shrink:0" title={entry.timestamp ? new Date(entry.timestamp).toLocaleString() : ''}>{timeAgo(entry.timestamp)}</span>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="card mt-4">
          <div style="text-align:center;padding:24px 16px">
            <div class="empty-state-icon" style="margin:0 auto 8px">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
            </div>
            <h3 style="color:var(--text-secondary);margin-bottom:4px">No Recent Activity</h3>
            <p class="text-xs text-dim">Activity will appear here once agents start processing.</p>
            <button class="btn btn-primary btn-sm" type="button" on:click={() => navigate('agents')} style="margin-top:12px">Chat with an Agent</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
