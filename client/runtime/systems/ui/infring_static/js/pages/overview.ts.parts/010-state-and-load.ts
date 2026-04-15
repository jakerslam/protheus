// Infring Overview Dashboard — Landing page with system stats + provider status
'use strict';

function overviewPage() {
  return {
    health: {},
    status: {},
    usageSummary: {},
    recentAudit: [],
    channels: [],
    providers: [],
    mcpServers: [],
    skillCount: 0,
    loading: true,
    loadError: '',
    refreshTimer: null,
    lastRefresh: null,

    async loadOverview() {
      this.loading = true;
      this.loadError = '';
      try {
        await Promise.all([
          this.loadHealth(),
          this.loadStatus(),
          this.loadUsage(),
          this.loadAudit(),
          this.loadChannels(),
          this.loadProviders(),
          this.loadMcpServers(),
          this.loadSkills()
        ]);
        this.lastRefresh = Date.now();
      } catch(e) {
        this.loadError = e.message || 'Could not load overview data.';
      }
      this.loading = false;
    },

    async loadData() { return this.loadOverview(); },

    // Silent background refresh (no loading spinner)
    async silentRefresh() {
      try {
        await Promise.all([
          this.loadHealth(),
          this.loadStatus(),
          this.loadUsage(),
          this.loadAudit(),
          this.loadChannels(),
          this.loadProviders(),
          this.loadMcpServers(),
          this.loadSkills()
        ]);
        this.lastRefresh = Date.now();
      } catch(e) { /* silent */ }
    },

    startAutoRefresh() {
      this.stopAutoRefresh();
      this.refreshTimer = setInterval(() => this.silentRefresh(), 30000);
    },

    stopAutoRefresh() {
      if (this.refreshTimer) {
        clearInterval(this.refreshTimer);
        this.refreshTimer = null;
      }
    },

    async loadHealth() {
      try {
        this.health = await InfringAPI.get('/api/health');
      } catch(e) { this.health = { status: 'unreachable' }; }
    },

    async loadStatus() {
      try {
        this.status = await InfringAPI.get('/api/status');
      } catch(e) { this.status = {}; throw e; }
    },

    async loadUsage() {
      try {
        var data = await InfringAPI.get('/api/usage');
        var agents = data.agents || [];
        var totalTokens = 0;
        var totalTools = 0;
        var totalCost = 0;
        agents.forEach(function(a) {
          totalTokens += (a.total_tokens || 0);
          totalTools += (a.tool_calls || 0);
          totalCost += (a.cost_usd || 0);
        });
        this.usageSummary = {
          total_tokens: totalTokens,
          total_tools: totalTools,
          total_cost: totalCost,
          agent_count: agents.length
        };
      } catch(e) {
        this.usageSummary = { total_tokens: 0, total_tools: 0, total_cost: 0, agent_count: 0 };
      }
    },

    async loadAudit() {
      try {
        var data = await InfringAPI.get('/api/audit/recent?n=8');
        this.recentAudit = data.entries || [];
      } catch(e) { this.recentAudit = []; }
    },

    async loadChannels() {
      try {
        var data = await InfringAPI.get('/api/channels');
        this.channels = (data.channels || []).filter(function(ch) { return ch.has_token; });
      } catch(e) { this.channels = []; }
    },

    async loadProviders() {
      try {
        var data = await InfringAPI.get('/api/providers');
        this.providers = data.providers || [];
      } catch(e) { this.providers = []; }
    },

    async loadMcpServers() {
      try {
        var data = await InfringAPI.get('/api/mcp/servers');
        this.mcpServers = data.servers || [];
      } catch(e) { this.mcpServers = []; }
    },

    async loadSkills() {
      try {
        var data = await InfringAPI.get('/api/skills');
        this.skillCount = (data.skills || []).length;
      } catch(e) { this.skillCount = 0; }
    },

    get configuredProviders() {
      return this.providers.filter(function(p) { return p.auth_status === 'configured'; });
    },

    get unconfiguredProviders() {
      return this.providers.filter(function(p) { return p.auth_status === 'not_set' || p.auth_status === 'missing'; });
    },

    get connectedMcp() {
      return this.mcpServers.filter(function(s) { return s.status === 'connected'; });
    },

    // Provider health badge color
    providerBadgeClass(p) {
      if (p.auth_status === 'configured') {
        if (p.health === 'cooldown' || p.health === 'open') return 'badge-warn';
        return 'badge-success';
      }
      if (p.auth_status === 'not_set' || p.auth_status === 'missing') return 'badge-muted';
      return 'badge-dim';
    },

    // Provider health tooltip
    providerTooltip(p) {
      if (p.health === 'cooldown') return p.display_name + ' \u2014 cooling down (rate limited)';
      if (p.health === 'open') return p.display_name + ' \u2014 circuit breaker open';
      if (p.auth_status === 'configured') return p.display_name + ' \u2014 ready';
      return p.display_name + ' \u2014 not configured';
    },

    // Audit action badge color
    actionBadgeClass(action) {
      if (!action) return 'badge-dim';
      if (action === 'AgentSpawn' || action === 'AuthSuccess') return 'badge-success';
      if (action === 'AgentKill' || action === 'AgentTerminated' || action === 'AuthFailure' || action === 'CapabilityDenied') return 'badge-error';
      if (action === 'RateLimited' || action === 'ToolInvoke') return 'badge-warn';
      return 'badge-created';
    },
