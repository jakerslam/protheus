// Infring Analytics Page — Full usage analytics with per-model and per-agent breakdowns
// Includes Cost Dashboard with donut chart, bar chart, projections, and provider breakdown.
'use strict';

var USAGE_PROVIDER_RULES = [
  { provider: 'Frontier Provider', tokens: ['claude', 'haiku', 'sonnet', 'opus'] },
  { provider: 'Google', tokens: ['gemini', 'gemma'] },
  { provider: 'OpenAI', tokens: ['gpt', 'o1', 'o3', 'o4'] },
  { provider: 'Groq', tokens: ['llama', 'mixtral', 'groq'] },
  { provider: 'DeepSeek', tokens: ['deepseek'] },
  { provider: 'Mistral', tokens: ['mistral'] },
  { provider: 'Cohere', tokens: ['command', 'cohere'] },
  { provider: 'xAI', tokens: ['grok'] },
  { provider: 'AI21', tokens: ['jamba'] },
  { provider: 'Together', tokens: ['qwen'] }
];

function usageNumber(value) {
  var parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return parsed;
}

function usageNormalizeSummary(payload) {
  var source = payload && typeof payload === 'object' ? payload : {};
  return {
    total_input_tokens: usageNumber(source.total_input_tokens),
    total_output_tokens: usageNumber(source.total_output_tokens),
    total_cost_usd: usageNumber(source.total_cost_usd),
    call_count: usageNumber(source.call_count),
    total_tool_calls: usageNumber(source.total_tool_calls)
  };
}

function usageNormalizeModelRows(rows) {
  var source = Array.isArray(rows) ? rows : [];
  return source.map(function(row) {
    var item = row && typeof row === 'object' ? row : {};
    return {
      model: String(item.model || ''),
      call_count: usageNumber(item.call_count),
      total_input_tokens: usageNumber(item.total_input_tokens),
      total_output_tokens: usageNumber(item.total_output_tokens),
      total_cost_usd: usageNumber(item.total_cost_usd)
    };
  });
}

function usageNormalizeAgentRows(rows) {
  var source = Array.isArray(rows) ? rows : [];
  return source.map(function(row) {
    var item = row && typeof row === 'object' ? row : {};
    return {
      agent_id: String(item.agent_id || ''),
      agent_name: String(item.agent_name || ''),
      call_count: usageNumber(item.call_count),
      total_input_tokens: usageNumber(item.total_input_tokens),
      total_output_tokens: usageNumber(item.total_output_tokens),
      total_cost_usd: usageNumber(item.total_cost_usd),
      total_tool_calls: usageNumber(item.total_tool_calls)
    };
  });
}

function usageNormalizeDailyCosts(rows) {
  var source = Array.isArray(rows) ? rows : [];
  return source.map(function(row) {
    var item = row && typeof row === 'object' ? row : {};
    return {
      date: String(item.date || ''),
      cost_usd: usageNumber(item.cost_usd),
      tokens: usageNumber(item.tokens),
      calls: usageNumber(item.calls)
    };
  });
}

function usageProviderFromModel(modelName) {
  if (!modelName) return 'Unknown';
  var lower = String(modelName).toLowerCase();
  for (var i = 0; i < USAGE_PROVIDER_RULES.length; i++) {
    var rule = USAGE_PROVIDER_RULES[i];
    for (var j = 0; j < rule.tokens.length; j++) {
      if (lower.indexOf(rule.tokens[j]) !== -1) return rule.provider;
    }
  }
  return 'Other';
}

function analyticsPage() {
  return {
    tab: 'summary',
    summary: {},
    byModel: [],
    byAgent: [],
    loading: true,
    loadError: '',

    // Cost tab state
    dailyCosts: [],
    todayCost: 0,
    firstEventDate: null,

    // Chart colors for providers (stable palette)
    _chartColors: [
      '#2563EB', '#3B82F6', '#10B981', '#60A5FA', '#8B5CF6',
      '#EC4899', '#06B6D4', '#EF4444', '#84CC16', '#F97316',
      '#6366F1', '#14B8A6', '#E11D48', '#A855F7', '#22D3EE'
    ],

    async loadUsage() {
      this.loading = true;
      this.loadError = '';
      try {
        await Promise.all([
          this.loadSummary(),
          this.loadByModel(),
          this.loadByAgent(),
          this.loadDailyCosts()
        ]);
      } catch(e) {
        this.loadError = e.message || 'Could not load usage data.';
      }
      this.loading = false;
    },

    async loadData() { return this.loadUsage(); },

    async loadSummary() {
      try {
        this.summary = usageNormalizeSummary(await InfringAPI.get('/api/usage/summary'));
      } catch(e) {
        this.summary = usageNormalizeSummary({});
        throw e;
      }
    },

    async loadByModel() {
      try {
        var data = await InfringAPI.get('/api/usage/by-model');
        this.byModel = usageNormalizeModelRows(data.models);
      } catch(e) { this.byModel = usageNormalizeModelRows([]); }
    },

    async loadByAgent() {
      try {
        var data = await InfringAPI.get('/api/usage');
        this.byAgent = usageNormalizeAgentRows(data.agents);
      } catch(e) { this.byAgent = usageNormalizeAgentRows([]); }
    },

    async loadDailyCosts() {
      try {
        var data = await InfringAPI.get('/api/usage/daily');
        this.dailyCosts = usageNormalizeDailyCosts(data.days);
        this.todayCost = usageNumber(data.today_cost_usd);
        this.firstEventDate = data.first_event_date ? String(data.first_event_date) : null;
      } catch(e) {
        this.dailyCosts = usageNormalizeDailyCosts([]);
        this.todayCost = 0;
        this.firstEventDate = null;
      }
    },

    runtimeSyncSnapshot() {
      try {
        var app = Alpine.store('app');
        if (app && app.runtimeSync && typeof app.runtimeSync === 'object') {
          return app.runtimeSync;
        }
      } catch(_) {}
      return null;
    },

    runtimeStatusLabel() {
      try {
        var app = Alpine.store('app');
        var state = String((app && app.connectionState) || '').toLowerCase();
        if (state === 'connecting' || state === 'reconnecting') return 'Connecting...';
        if (state === 'disconnected') return 'Disconnected';
      } catch(_) {}
      return 'Connected';
    },

    runtimeMetaRows() {
      var runtime = this.runtimeSyncSnapshot();
      if (!runtime) return [];
      var conduit = String(Number(runtime.conduit_signals || 0)) + '/' + String(Number(runtime.target_conduit_signals || 0));
      var spineRate = Number(runtime.spine_success_rate);
      var spineLabel = Number.isFinite(spineRate) ? (Math.round(spineRate * 1000) / 10) + '%' : 'n/a';
      return [
        { label: 'Status', value: this.runtimeStatusLabel() },
        { label: 'Queue Depth', value: Number(runtime.queue_depth || 0) },
        { label: 'Conduit Signals', value: conduit },
        { label: 'Stale Cockpit Blocks', value: Number(runtime.cockpit_stale_blocks || 0) },
        { label: 'Critical Attention', value: Number(runtime.critical_attention_total || runtime.critical_attention || 0) },
        { label: 'Backpressure', value: String(runtime.backpressure_level || runtime.sync_mode || 'normal') },
        { label: 'Receipt P95', value: Number(runtime.receipt_latency_p95_ms || 0) > 0 ? Math.round(Number(runtime.receipt_latency_p95_ms)) + 'ms' : 'n/a' },
        { label: 'Spine Success', value: spineLabel }
      ];
    },

    formatTokens(n) {
      if (!n) return '0';
      if (n >= 1000000) return (n / 1000000).toFixed(2) + 'M';
      if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
      return String(n);
    },

    formatCost(c) {
      if (!c) return '$0.00';
      if (c < 0.01) return '$' + c.toFixed(4);
      return '$' + c.toFixed(2);
    },

    maxTokens() {
      var max = 0;
      this.byModel.forEach(function(m) {
        var t = (m.total_input_tokens || 0) + (m.total_output_tokens || 0);
        if (t > max) max = t;
      });
      return max || 1;
    },

    barWidth(m) {
      var t = (m.total_input_tokens || 0) + (m.total_output_tokens || 0);
      return Math.max(2, Math.round((t / this.maxTokens()) * 100)) + '%';
    },

    // ── Cost tab helpers ──

    avgCostPerMessage() {
      var count = this.summary.call_count || 0;
      if (count === 0) return 0;
      return (this.summary.total_cost_usd || 0) / count;
    },

    projectedMonthlyCost() {
      if (!this.firstEventDate || !this.summary.total_cost_usd) return 0;
      var first = new Date(this.firstEventDate);
      var now = new Date();
      var diffMs = now.getTime() - first.getTime();
      var diffDays = diffMs / (1000 * 60 * 60 * 24);
      if (diffDays < 1) diffDays = 1;
      return (this.summary.total_cost_usd / diffDays) * 30;
    },

    // ── Provider aggregation from byModel data ──

    costByProvider() {
      var providerMap = {};
      var self = this;
      this.byModel.forEach(function(m) {
        var provider = self._extractProvider(m.model);
        if (!providerMap[provider]) {
          providerMap[provider] = { provider: provider, cost: 0, tokens: 0, calls: 0 };
        }
        providerMap[provider].cost += (m.total_cost_usd || 0);
        providerMap[provider].tokens += (m.total_input_tokens || 0) + (m.total_output_tokens || 0);
        providerMap[provider].calls += (m.call_count || 0);
      });
      var result = [];
      for (var key in providerMap) {
        if (providerMap.hasOwnProperty(key)) {
          result.push(providerMap[key]);
        }
      }
      result.sort(function(a, b) { return b.cost - a.cost; });
      return result;
    },

    _extractProvider(modelName) {
      return usageProviderFromModel(modelName);
    },

    // ── Donut chart (stroke-dasharray on circles) ──

    donutSegments() {
      var providers = this.costByProvider();
      var total = 0;
      var colors = this._chartColors;
      providers.forEach(function(p) { total += p.cost; });
      if (total === 0) return [];

      var segments = [];
      var offset = 0;
      var circumference = 2 * Math.PI * 60; // r=60
      for (var i = 0; i < providers.length; i++) {
        var pct = providers[i].cost / total;
        var dashLen = pct * circumference;
        segments.push({
          provider: providers[i].provider,
          cost: providers[i].cost,
          percent: Math.round(pct * 100),
          color: colors[i % colors.length],
          dasharray: dashLen + ' ' + (circumference - dashLen),
          dashoffset: -offset,
          circumference: circumference
        });
        offset += dashLen;
      }
      return segments;
    },

    // ── Bar chart (last 7 days) ──

    barChartData() {
      var days = this.dailyCosts;
      if (!days || days.length === 0) return [];
      var maxCost = 0;
      days.forEach(function(d) { if (d.cost_usd > maxCost) maxCost = d.cost_usd; });
      if (maxCost === 0) maxCost = 1;

      var dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
      var result = [];
      for (var i = 0; i < days.length; i++) {
        var d = new Date(days[i].date + 'T12:00:00');
        var dayName = dayNames[d.getDay()] || '?';
        var heightPct = Math.max(2, Math.round((days[i].cost_usd / maxCost) * 120));
        result.push({
          date: days[i].date,
          dayName: dayName,
          cost: days[i].cost_usd,
          tokens: days[i].tokens,
          calls: days[i].calls,
          barHeight: heightPct
        });
      }
      return result;
    },

    // ── Cost by model table (sorted by cost descending) ──

    costByModelSorted() {
      var models = this.byModel.slice();
      models.sort(function(a, b) { return (b.total_cost_usd || 0) - (a.total_cost_usd || 0); });
      return models;
    },

    maxModelCost() {
      var max = 0;
      this.byModel.forEach(function(m) {
        if ((m.total_cost_usd || 0) > max) max = m.total_cost_usd;
      });
      return max || 1;
    },

    costBarWidth(m) {
      return Math.max(2, Math.round(((m.total_cost_usd || 0) / this.maxModelCost()) * 100)) + '%';
    },

    modelTier(modelName) {
      if (!modelName) return 'unknown';
      var lower = modelName.toLowerCase();
      if (lower.indexOf('opus') !== -1 || lower.indexOf('o1') !== -1 || lower.indexOf('o3') !== -1 || lower.indexOf('deepseek-r1') !== -1) return 'frontier';
      if (lower.indexOf('sonnet') !== -1 || lower.indexOf('gpt-4') !== -1 || lower.indexOf('gemini-2.5') !== -1 || lower.indexOf('gemini-1.5-pro') !== -1) return 'smart';
      if (lower.indexOf('haiku') !== -1 || lower.indexOf('gpt-3.5') !== -1 || lower.indexOf('flash') !== -1 || lower.indexOf('mixtral') !== -1) return 'balanced';
      if (lower.indexOf('llama') !== -1 || lower.indexOf('groq') !== -1 || lower.indexOf('gemma') !== -1) return 'fast';
      return 'balanced';
    }
  };
}
