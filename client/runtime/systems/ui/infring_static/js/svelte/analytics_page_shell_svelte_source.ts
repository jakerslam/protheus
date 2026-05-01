const COMPONENT_TAG = 'infring-analytics-page-shell';

const COMPONENT_SOURCE = String.raw`<svelte:options customElement={{ tag: 'infring-analytics-page-shell', shadow: 'none' }} />
<script>
  import { onMount } from 'svelte';

  export let shellPrimitive = 'simple-page-panel';
  export let pageId = 'analytics';
  export let panelRole = 'page';
  export let routeContract = 'analytics';
  export let parentOwnedData = false;

  const providerRules = [
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
  const chartColors = ['#2563EB', '#3B82F6', '#10B981', '#60A5FA', '#8B5CF6', '#EC4899', '#06B6D4', '#EF4444', '#84CC16', '#F97316', '#6366F1', '#14B8A6', '#E11D48', '#A855F7', '#22D3EE'];
  const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const emptySummary = { total_input_tokens: 0, total_output_tokens: 0, total_cost_usd: 0, call_count: 0, total_tool_calls: 0 };

  let tab = 'summary';
  let summary = Object.assign({}, emptySummary);
  let byModel = [];
  let byAgent = [];
  let dailyCosts = [];
  let todayCost = 0;
  let firstEventDate = null;
  let loading = true;
  let loadError = '';

  $: totalTokens = (summary.total_input_tokens || 0) + (summary.total_output_tokens || 0);
  $: runtimeRows = runtimeMetaRows();
  $: providerCosts = costByProvider();
  $: donutRows = donutSegments(providerCosts);
  $: dailyBars = barChartData();
  $: dailyChartWidth = dailyBars.length * 50 + 20;
  $: dailyChartViewBox = '0 0 ' + dailyChartWidth + ' 180';
  $: costModels = costByModelSorted();

  function api() {
    return typeof window !== 'undefined' ? window.InfringAPI : null;
  }

  function navigate(page) {
    var app = typeof window !== 'undefined' ? window.InfringApp : null;
    if (app && typeof app.navigate === 'function') app.navigate(page);
    else if (typeof window !== 'undefined') window.location.hash = page;
  }

  function usageNumber(value) {
    var parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function normalizeSummary(payload) {
    var source = payload && typeof payload === 'object' ? payload : {};
    return {
      total_input_tokens: usageNumber(source.total_input_tokens),
      total_output_tokens: usageNumber(source.total_output_tokens),
      total_cost_usd: usageNumber(source.total_cost_usd),
      call_count: usageNumber(source.call_count),
      total_tool_calls: usageNumber(source.total_tool_calls)
    };
  }

  function normalizeModels(rows) {
    return (Array.isArray(rows) ? rows : []).map(function(row) {
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

  function normalizeAgents(rows) {
    return (Array.isArray(rows) ? rows : []).map(function(row) {
      var item = row && typeof row === 'object' ? row : {};
      return {
        agent_id: String(item.agent_id || ''),
        name: String(item.agent_name || item.agent_id || 'unknown'),
        total_tokens: usageNumber(item.total_input_tokens) + usageNumber(item.total_output_tokens),
        tool_calls: usageNumber(item.total_tool_calls)
      };
    });
  }

  function normalizeDailyCosts(rows) {
    return (Array.isArray(rows) ? rows : []).map(function(row) {
      var item = row && typeof row === 'object' ? row : {};
      return { date: String(item.date || ''), cost_usd: usageNumber(item.cost_usd), tokens: usageNumber(item.tokens), calls: usageNumber(item.calls) };
    });
  }

  async function loadUsage() {
    loading = true;
    loadError = '';
    try {
      var client = api();
      if (!client || typeof client.get !== 'function') throw new Error('Shell API client is unavailable.');
      var results = await Promise.allSettled([
        client.get('/api/usage/summary'),
        client.get('/api/usage/by-model'),
        client.get('/api/usage'),
        client.get('/api/usage/daily')
      ]);
      if (results[0].status === 'fulfilled') summary = normalizeSummary(results[0].value);
      else summary = Object.assign({}, emptySummary);
      byModel = results[1].status === 'fulfilled' ? normalizeModels(results[1].value && results[1].value.models) : [];
      byAgent = results[2].status === 'fulfilled' ? normalizeAgents(results[2].value && results[2].value.agents) : [];
      dailyCosts = results[3].status === 'fulfilled' ? normalizeDailyCosts(results[3].value && results[3].value.days) : [];
      todayCost = results[3].status === 'fulfilled' ? usageNumber(results[3].value && results[3].value.today_cost_usd) : 0;
      firstEventDate = results[3].status === 'fulfilled' && results[3].value && results[3].value.first_event_date ? String(results[3].value.first_event_date) : null;
      if (results[0].status === 'rejected') throw results[0].reason;
    } catch (e) {
      loadError = e && e.message ? e.message : 'Could not load usage data.';
    }
    loading = false;
  }

  function appSnapshot() {
    try {
      var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
      var bridge = services && services.appStore ? services.appStore : null;
      return bridge && typeof bridge.current === 'function' ? bridge.current() : null;
    } catch (_) {}
    return null;
  }

  function runtimeStatusLabel() {
    var app = appSnapshot();
    var state = String((app && app.connectionState) || '').toLowerCase();
    if (state === 'connecting' || state === 'reconnecting') return 'Connecting...';
    if (state === 'disconnected') return 'Disconnected';
    return 'Connected';
  }

  function runtimeMetaRows() {
    var app = appSnapshot();
    var runtime = app && app.runtimeSync && typeof app.runtimeSync === 'object' ? app.runtimeSync : null;
    if (!runtime) return [];
    var spineRate = Number(runtime.spine_success_rate);
    return [
      { label: 'Status', value: runtimeStatusLabel() },
      { label: 'Queue Depth', value: Number(runtime.queue_depth || 0) },
      { label: 'Conduit Signals', value: Number(runtime.conduit_signals || 0) + '/' + Number(runtime.target_conduit_signals || 0) },
      { label: 'Stale Cockpit Blocks', value: Number(runtime.cockpit_stale_blocks || 0) },
      { label: 'Critical Attention', value: Number(runtime.critical_attention_total || runtime.critical_attention || 0) },
      { label: 'Backpressure', value: String(runtime.backpressure_level || runtime.sync_mode || 'normal') },
      { label: 'Receipt P95', value: Number(runtime.receipt_latency_p95_ms || 0) > 0 ? Math.round(Number(runtime.receipt_latency_p95_ms)) + 'ms' : 'n/a' },
      { label: 'Spine Success', value: Number.isFinite(spineRate) ? (Math.round(spineRate * 1000) / 10) + '%' : 'n/a' }
    ];
  }

  function formatTokens(value) {
    var n = usageNumber(value);
    if (n >= 1000000) return (n / 1000000).toFixed(2) + 'M';
    if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
    return String(n);
  }

  function formatCost(value) {
    var cost = usageNumber(value);
    if (cost < 0.01) return '$' + cost.toFixed(4);
    return '$' + cost.toFixed(2);
  }

  function providerFromModel(modelName) {
    var lower = String(modelName || '').toLowerCase();
    if (!lower) return 'Unknown';
    for (var i = 0; i < providerRules.length; i++) {
      for (var j = 0; j < providerRules[i].tokens.length; j++) {
        if (lower.indexOf(providerRules[i].tokens[j]) !== -1) return providerRules[i].provider;
      }
    }
    return 'Other';
  }

  function modelTier(modelName) {
    var lower = String(modelName || '').toLowerCase();
    if (!lower) return 'unknown';
    if (lower.indexOf('opus') !== -1 || lower.indexOf('o1') !== -1 || lower.indexOf('o3') !== -1 || lower.indexOf('deepseek-r1') !== -1) return 'frontier';
    if (lower.indexOf('sonnet') !== -1 || lower.indexOf('gpt-4') !== -1 || lower.indexOf('gemini-2.5') !== -1 || lower.indexOf('gemini-1.5-pro') !== -1) return 'smart';
    if (lower.indexOf('haiku') !== -1 || lower.indexOf('gpt-3.5') !== -1 || lower.indexOf('flash') !== -1 || lower.indexOf('mixtral') !== -1) return 'balanced';
    if (lower.indexOf('llama') !== -1 || lower.indexOf('groq') !== -1 || lower.indexOf('gemma') !== -1) return 'fast';
    return 'balanced';
  }

  function modelTierClass(modelName) {
    return 'tier-badge tier-' + modelTier(modelName);
  }

  function maxTokens() {
    return byModel.reduce(function(max, row) { return Math.max(max, (row.total_input_tokens || 0) + (row.total_output_tokens || 0)); }, 1);
  }

  function barWidth(row) {
    var tokens = (row.total_input_tokens || 0) + (row.total_output_tokens || 0);
    return Math.max(2, Math.round((tokens / maxTokens()) * 100)) + '%';
  }

  function avgCostPerMessage() {
    return summary.call_count ? (summary.total_cost_usd || 0) / summary.call_count : 0;
  }

  function projectedMonthlyCost() {
    if (!firstEventDate || !summary.total_cost_usd) return 0;
    var first = new Date(firstEventDate);
    var diffDays = Math.max(1, (Date.now() - first.getTime()) / (1000 * 60 * 60 * 24));
    return (summary.total_cost_usd / diffDays) * 30;
  }

  function costByProvider() {
    var grouped = {};
    byModel.forEach(function(row) {
      var provider = providerFromModel(row.model);
      if (!grouped[provider]) grouped[provider] = { provider: provider, cost: 0, tokens: 0, calls: 0 };
      grouped[provider].cost += row.total_cost_usd || 0;
      grouped[provider].tokens += (row.total_input_tokens || 0) + (row.total_output_tokens || 0);
      grouped[provider].calls += row.call_count || 0;
    });
    return Object.keys(grouped).map(function(key) { return grouped[key]; }).sort(function(a, b) { return b.cost - a.cost; });
  }

  function donutSegments(rows) {
    var total = rows.reduce(function(sum, row) { return sum + row.cost; }, 0);
    if (!total) return [];
    var circumference = 2 * Math.PI * 60;
    var offset = 0;
    return rows.map(function(row, index) {
      var pct = row.cost / total;
      var dashLen = pct * circumference;
      var segment = { provider: row.provider, cost: row.cost, percent: Math.round(pct * 100), color: chartColors[index % chartColors.length], dasharray: dashLen + ' ' + (circumference - dashLen), dashoffset: -offset };
      offset += dashLen;
      return segment;
    });
  }

  function barChartData() {
    var maxCost = dailyCosts.reduce(function(max, row) { return Math.max(max, row.cost_usd || 0); }, 1);
    return dailyCosts.map(function(row) {
      var date = new Date(row.date + 'T12:00:00');
      return { date: row.date, dayName: dayNames[date.getDay()] || '?', cost: row.cost_usd, calls: row.calls, barHeight: Math.max(2, Math.round(((row.cost_usd || 0) / maxCost) * 120)) };
    });
  }

  function costByModelSorted() {
    return byModel.slice().sort(function(a, b) { return (b.total_cost_usd || 0) - (a.total_cost_usd || 0); });
  }

  function maxModelCost() {
    return byModel.reduce(function(max, row) { return Math.max(max, row.total_cost_usd || 0); }, 1);
  }

  function costBarWidth(row) {
    return Math.max(2, Math.round(((row.total_cost_usd || 0) / maxModelCost()) * 100)) + '%';
  }

  onMount(loadUsage);
</script>

<div class="page-header page-header-subtabs-center">
  <div>
    <div class="tabs mt-3" role="tablist">
      <button type="button" class="tab" class:active={false} role="tab" on:click={() => navigate('runtime')}>Runtime</button>
      <button type="button" class="tab active" role="tab">Analytics</button>
      <button type="button" class="tab" class:active={false} role="tab" on:click={() => navigate('logs')}>Logs</button>
    </div>
  </div>
</div>

<div class="page-body">
  {#if loading}
    <div class="loading-state"><div class="spinner"></div><span>Loading usage data...</span></div>
  {:else if loadError}
    <div class="error-state"><span class="error-icon">!</span><p>{loadError}</p><button class="btn btn-ghost btn-sm" type="button" on:click={loadUsage}>Retry</button></div>
  {:else}
    <div class="stats-row">
      <div class="stat-card"><div class="stat-value">{formatTokens(totalTokens)}</div><div class="stat-label">Total Tokens</div></div>
      <div class="stat-card"><div class="stat-value">{formatCost(summary.total_cost_usd)}</div><div class="stat-label">Estimated Cost</div></div>
      <div class="stat-card"><div class="stat-value">{summary.call_count || 0}</div><div class="stat-label">API Calls</div></div>
      <div class="stat-card"><div class="stat-value">{summary.total_tool_calls || 0}</div><div class="stat-label">Tool Calls</div></div>
    </div>

    <div class="tabs mt-4" role="tablist">
      {#each ['summary', 'by-model', 'by-agent', 'costs'] as item}
        <button type="button" role="tab" class="tab" class:active={tab === item} on:click={() => tab = item}>{item === 'by-model' ? 'By Model' : item === 'by-agent' ? 'By Agent' : item === 'costs' ? 'Costs' : 'Summary'}</button>
      {/each}
    </div>

    {#if tab === 'summary'}
      <section class="card mt-4">
        <div class="card-header">Runtime Telemetry</div>
        {#if runtimeRows.length}
          <div class="detail-grid" style="margin-top:8px">
            {#each runtimeRows as row (row.label)}
              <div class="detail-row"><span class="detail-label">{row.label}</span><span class="detail-value">{row.value}</span></div>
            {/each}
          </div>
        {:else}
          <infring-chat-stream-shell class="empty-state"><p>No runtime telemetry available yet.</p></infring-chat-stream-shell>
        {/if}
      </section>
      <section class="card mt-4">
        <div class="card-header">Token Breakdown</div>
        <div class="detail-grid" style="margin-top:8px">
          <div class="detail-row"><span class="detail-label">Input Tokens</span><span class="detail-value">{formatTokens(summary.total_input_tokens)}</span></div>
          <div class="detail-row"><span class="detail-label">Output Tokens</span><span class="detail-value">{formatTokens(summary.total_output_tokens)}</span></div>
          <div class="detail-row"><span class="detail-label">Total Cost</span><span class="detail-value">{formatCost(summary.total_cost_usd)}</span></div>
          <div class="detail-row"><span class="detail-label">API Calls</span><span class="detail-value">{summary.call_count || 0}</span></div>
          <div class="detail-row"><span class="detail-label">Tool Calls</span><span class="detail-value">{summary.total_tool_calls || 0}</span></div>
        </div>
      </section>
    {:else if tab === 'by-model'}
      {#if byModel.length}
        <div class="table-wrap mt-4">
          <table><thead><tr><th>Model</th><th>Calls</th><th>Input Tokens</th><th>Output Tokens</th><th>Cost</th><th style="width:30%">Usage</th></tr></thead><tbody>
            {#each byModel as model (model.model)}
              <tr><td class="font-bold" style="font-size:11px">{model.model}</td><td>{model.call_count}</td><td>{formatTokens(model.total_input_tokens)}</td><td>{formatTokens(model.total_output_tokens)}</td><td>{formatCost(model.total_cost_usd)}</td><td><div style="background:var(--surface2);border-radius:4px;height:16px;overflow:hidden"><div style="height:100%;border-radius:4px;background:var(--accent);transition:width 0.3s;width:{barWidth(model)}"></div></div></td></tr>
            {/each}
          </tbody></table>
        </div>
      {:else}<infring-chat-stream-shell class="empty-state"><p>No model usage data yet.</p></infring-chat-stream-shell>{/if}
    {:else if tab === 'by-agent'}
      {#if byAgent.length}
        <div class="table-wrap mt-4">
          <table><thead><tr><th>Agent</th><th>Total Tokens</th><th>Tool Calls</th></tr></thead><tbody>
            {#each byAgent as agent (agent.agent_id)}
              <tr><td class="font-bold">{agent.name}</td><td>{agent.total_tokens ? agent.total_tokens.toLocaleString() : '0'}</td><td>{agent.tool_calls || 0}</td></tr>
            {/each}
          </tbody></table>
        </div>
      {:else}<infring-chat-stream-shell class="empty-state"><p>No agent usage data yet.</p></infring-chat-stream-shell>{/if}
    {:else}
      <div class="stats-row mt-4">
        <div class="stat-card"><div class="stat-value">{formatCost(summary.total_cost_usd)}</div><div class="stat-label">Total Spend</div></div>
        <div class="stat-card"><div class="stat-value">{formatCost(todayCost)}</div><div class="stat-label">Today's Spend</div></div>
        <div class="stat-card"><div class="stat-value">{formatCost(projectedMonthlyCost())}</div><div class="stat-label">Projected Monthly</div></div>
        <div class="stat-card"><div class="stat-value">{formatCost(avgCostPerMessage())}</div><div class="stat-label">Avg Cost / Message</div></div>
      </div>
      <div class="cost-charts-row mt-4">
        <section class="card cost-chart-panel">
          <div class="card-header">Cost by Provider</div>
          {#if donutRows.length}
            <div class="donut-chart-wrap"><div class="donut-chart"><svg viewBox="0 0 160 160" width="160" height="160">
              {#each donutRows as segment (segment.provider)}
                <circle cx="80" cy="80" r="60" fill="none" stroke={segment.color} stroke-width="24" stroke-dasharray={segment.dasharray} stroke-dashoffset={segment.dashoffset} transform="rotate(-90 80 80)" class="donut-segment"><title>{segment.provider}: {segment.percent}% ({formatCost(segment.cost)})</title></circle>
              {/each}
              <text x="80" y="76" text-anchor="middle" fill="var(--text)" style="font-size:14px;font-weight:700;font-family:var(--font-mono)">{formatCost(summary.total_cost_usd)}</text>
              <text x="80" y="92" text-anchor="middle" fill="var(--text-muted)" style="font-size:9px;font-family:var(--font-mono)">TOTAL</text>
            </svg></div><div class="donut-legend">
              {#each donutRows as segment (segment.provider + '-legend')}
                <div class="donut-legend-item"><span class="donut-legend-swatch" style="background:{segment.color}"></span><span class="donut-legend-label">{segment.provider}</span><span class="donut-legend-pct">{segment.percent}%</span><span class="donut-legend-cost text-dim">{formatCost(segment.cost)}</span></div>
              {/each}
            </div></div>
          {:else}<div class="text-sm text-dim" style="padding:20px;text-align:center">No cost data yet.</div>{/if}
        </section>
        <section class="card cost-chart-panel">
          <div class="card-header">Daily Cost (Last 7 Days)</div>
          {#if dailyBars.length}
            <div class="bar-chart"><svg viewBox={dailyChartViewBox} width={dailyChartWidth} height="180">
              <line x1="10" x2={dailyChartWidth - 10} y1="150" y2="150" stroke="var(--border)" stroke-width="1"/>
              {#each dailyBars as bar, index (bar.date)}
                <g><rect x={index * 50 + 18} y={150 - bar.barHeight} width="24" height={bar.barHeight} rx="3" fill="var(--accent)" class="cost-bar" style="opacity:0.85"><title>{bar.date}: {formatCost(bar.cost)} ({bar.calls} calls)</title></rect><text x={index * 50 + 30} y="166" text-anchor="middle" fill="var(--text-muted)" style="font-size:9px;font-family:var(--font-mono)">{bar.dayName}</text><text x={index * 50 + 30} y={150 - bar.barHeight - 4} text-anchor="middle" fill="var(--text-dim)" style="font-size:8px;font-family:var(--font-mono)">{formatCost(bar.cost)}</text></g>
              {/each}
            </svg></div>
          {:else}<div class="text-sm text-dim" style="padding:20px;text-align:center">No daily data yet.</div>{/if}
        </section>
      </div>
      <section class="card mt-4">
        <div class="card-header">Cost by Model</div>
        {#if costModels.length}
          <div class="table-wrap" style="border:none;margin-top:8px"><table><thead><tr><th>Model</th><th>Provider</th><th>Tier</th><th>Input Tokens</th><th>Output Tokens</th><th>Calls</th><th>Cost</th><th style="width:20%">Cost Share</th></tr></thead><tbody>
            {#each costModels as model (model.model + '-cost')}
              <tr><td class="font-bold" style="font-size:11px">{model.model}</td><td><span class="badge badge-muted" style="font-size:9px">{providerFromModel(model.model)}</span></td><td><span class={modelTierClass(model.model)}>{modelTier(model.model)}</span></td><td>{formatTokens(model.total_input_tokens)}</td><td>{formatTokens(model.total_output_tokens)}</td><td>{model.call_count}</td><td class="font-bold">{formatCost(model.total_cost_usd)}</td><td><div style="background:var(--surface2);border-radius:4px;height:16px;overflow:hidden"><div style="height:100%;border-radius:4px;background:var(--accent);transition:width 0.3s;width:{costBarWidth(model)}"></div></div></td></tr>
            {/each}
          </tbody></table></div>
        {:else}<div class="text-sm text-dim" style="padding:20px;text-align:center">No model cost data yet.</div>{/if}
      </section>
    {/if}
  {/if}
</div>
`;

module.exports = {
  COMPONENT_TAG,
  COMPONENT_SOURCE,
};
