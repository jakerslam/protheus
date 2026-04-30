function infringRuntimeFacadeHealthSummary(page) {
  var summary = page.healthSummary && typeof page.healthSummary === 'object' ? page.healthSummary : null;
  if (!summary) return null;
  var loadedAt = Number(page._healthSummaryLoadedAt || 0);
  if (loadedAt > 0 && (Date.now() - loadedAt) > 60000) return null;
  return summary;
}

function infringRuntimeFacadeState(page) {
  var store = page.getAppStore();
  var conn = page.normalizeConnectionIndicatorState(
    page.connectionIndicatorState ||
    ((store && store.connectionState) || page.connectionState || '')
  );
  if (conn === 'connecting') return 'connecting';
  if (conn === 'disconnected') return page.runtimeFacadeHealthSummary() ? 'connecting' : 'down';
  if (page.runtimeEtaSeconds() > 0) return 'active';
  return 'connected';
}

function infringRuntimeFacadeClass(page) {
  var state = page.runtimeFacadeState();
  if (state === 'connected' || state === 'active') return 'health-ok';
  if (state === 'connecting') return 'health-connecting';
  return 'health-down';
}

function infringRuntimeFacadeAgentCount(page, store, health) {
  return ((store && store.agents && store.agents.length) ||
    (store && store.agentCount) ||
    page.agentCount ||
    Number(health && health.agent_count || 0) ||
    Number(health && health.agents && health.agents.length || 0));
}

function infringRuntimeFacadeLabel(page) {
  var state = page.runtimeFacadeState();
  if (state === 'active') return 'Active';
  if (state === 'connected') {
    var store = page.getAppStore();
    var health = page.runtimeFacadeHealthSummary();
    return String(infringRuntimeFacadeAgentCount(page, store, health)) + ' agents';
  }
  if (state === 'connecting' && page.runtimeFacadeHealthSummary()) return 'Reconnecting...';
  if (state === 'connecting') return 'Connecting...';
  return 'Disconnected';
}

function infringRuntimeFacadeDisplayLabel(page) {
  var label = String(page.runtimeFacadeLabel() || '').trim();
  if (!label) return '';
  return label.replace(/\s+agents?$/i, '');
}

function infringRuntimeSync(page) {
  var store = page.getAppStore();
  return store && store.runtimeSync && typeof store.runtimeSync === 'object'
    ? store.runtimeSync
    : null;
}

function infringRuntimeResponseP95Ms(page) {
  var runtime = infringRuntimeSync(page);
  if (!runtime) {
    var health = page.runtimeFacadeHealthSummary();
    var durationMs = Number(health && health.durationMs);
    return Number.isFinite(durationMs) && durationMs >= 0 ? Math.round(durationMs) : null;
  }
  var facadeP95 = Number(runtime.facade_response_p95_ms);
  if (Number.isFinite(facadeP95) && facadeP95 > 0) return Math.round(facadeP95);
  var p95 = Number(runtime.receipt_latency_p95_ms);
  if (Number.isFinite(p95) && p95 > 0) return Math.round(p95);
  var p99 = Number(runtime.receipt_latency_p99_ms);
  if (Number.isFinite(p99) && p99 > 0) return Math.round(p99);
  return null;
}

function infringRuntimeConfidencePercent(page) {
  var runtime = infringRuntimeSync(page);
  if (!runtime) return page.runtimeFacadeHealthSummary() ? 92 : 80;
  var facadeConfidence = Number(runtime.facade_confidence_percent);
  if (Number.isFinite(facadeConfidence) && facadeConfidence > 0) {
    return Math.max(10, Math.min(100, Math.round(facadeConfidence)));
  }

  var score = 100;
  var queueDepth = Number(runtime.queue_depth || 0);
  var stale = Number(runtime.cockpit_stale_blocks || 0);
  var gaps = Number(runtime.health_coverage_gap_count || 0);
  var conduitSignals = Number(runtime.conduit_signals || 0);
  var targetSignals = Math.max(1, Number(runtime.target_conduit_signals || 4));
  var benchmark = String(runtime.benchmark_sanity_cockpit_status || runtime.benchmark_sanity_status || 'unknown').toLowerCase();
  var spine = Number(runtime.spine_success_rate);

  if (queueDepth > 20) score -= Math.min(20, Math.floor((queueDepth - 20) / 2));
  if (stale > 0) score -= Math.min(20, stale * 2);
  if (gaps > 0) score -= Math.min(20, gaps * 6);
  if (conduitSignals < Math.max(3, Math.floor(targetSignals * 0.5))) score -= 12;
  if (benchmark === 'warn') score -= 8;
  if (benchmark === 'fail' || benchmark === 'error') score -= 20;
  if (Number.isFinite(spine)) {
    if (spine < 0.9) score -= 15;
    if (spine < 0.6) score -= 10;
  }

  score = Math.max(10, Math.min(100, Math.round(score)));
  return score;
}

function infringRuntimeEtaSeconds(page) {
  var runtime = infringRuntimeSync(page);
  if (!runtime) return 0;
  var facadeEta = Number(runtime.facade_eta_seconds);
  if (Number.isFinite(facadeEta) && facadeEta >= 0) {
    return Math.max(0, Math.min(300, Math.round(facadeEta)));
  }
  var queueDepth = Math.max(0, Number(runtime.queue_depth || 0));
  if (queueDepth <= 0) return 0;
  return Math.max(1, Math.min(300, Math.ceil(queueDepth / 8)));
}

function infringRuntimeFacadeDetail(page) {
  var state = page.runtimeFacadeState();
  var store = page.getAppStore();
  var bootStage = String((store && store.bootStage) || '').trim();
  var stageSuffix = bootStage ? (' · ' + bootStage.replace(/_/g, ' ')) : '';
  if (state === 'connecting' && page.runtimeFacadeHealthSummary()) return 'HTTP health OK · reconnecting live runtime' + stageSuffix;
  if (state === 'connecting') return 'Establishing runtime link' + stageSuffix;
  if (state === 'down') return 'Runtime unavailable' + stageSuffix;
  var response = page.runtimeResponseP95Ms();
  var confidence = page.runtimeConfidencePercent();
  var health = page.runtimeFacadeHealthSummary();
  var agents = infringRuntimeFacadeAgentCount(page, store, health);
  var base = 'Response ' + (response != null ? (response + 'ms') : '—') + ' · Confidence ' + confidence + '%';
  if (store && store.statusDegraded) {
    return base + ' · Status degraded' + stageSuffix;
  }
  if (state === 'active') {
    var eta = page.runtimeEtaSeconds();
    return (eta > 0 ? ('ETA ~' + eta + 's · ') : '') + base;
  }
  return base + ' · ' + agents + ' agent(s)';
}

function infringRuntimeFacadeTitle(page) {
  return page.runtimeFacadeLabel();
}
