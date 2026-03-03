#!/usr/bin/env node
'use strict';
export {};

const crypto = require('crypto');

type AnyObj = Record<string, any>;

type TraceEvent = {
  trace_id: string;
  ts_millis: number;
  source: string;
  operation: string;
  severity: string;
  tags: string[];
  payload_digest: string;
  signed: boolean;
};

type ChaosScenarioRequest = {
  scenario_id: string;
  events: TraceEvent[];
  cycles: number;
  inject_fault_every: number;
  enforce_fail_closed: boolean;
};

type EmbeddedObservabilityProfile = {
  profile_id: string;
  version: number;
  red_legion_trace_channels: string[];
  allowed_emitters: string[];
  stream_policy: {
    trace_window_ms: number;
    max_events_per_window: number;
    min_sampling_rate_pct: number;
    redact_fields: string[];
    require_signature: boolean;
  };
  sovereignty_scorer: {
    integrity_weight_pct: number;
    continuity_weight_pct: number;
    reliability_weight_pct: number;
    chaos_penalty_pct: number;
    fail_closed_threshold_pct: number;
  };
  chaos_hooks: Array<{ id: string; condition: string; action: string; severity: string; enabled: boolean }>;
};

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).trim().replace(/\s+/g, ' ').slice(0, maxLen);
}

function round3(v: number) {
  return Math.round(v * 1000) / 1000;
}

function severityWeight(severity: string) {
  switch (String(severity || '').toLowerCase()) {
    case 'critical': return 1.0;
    case 'high': return 0.7;
    case 'medium': return 0.35;
    case 'low': return 0.15;
    default: return 0.2;
  }
}

function normalizeEvent(raw: AnyObj): TraceEvent {
  return {
    trace_id: cleanText(raw && raw.trace_id, 160),
    ts_millis: Number.isFinite(Number(raw && raw.ts_millis)) ? Math.max(0, Math.floor(Number(raw.ts_millis))) : 0,
    source: cleanText(raw && raw.source, 160),
    operation: cleanText(raw && raw.operation, 160),
    severity: cleanText(raw && raw.severity, 32).toLowerCase(),
    tags: Array.isArray(raw && raw.tags) ? raw.tags.map((v: unknown) => cleanText(v, 80)).filter(Boolean) : [],
    payload_digest: cleanText(raw && raw.payload_digest, 256),
    signed: Boolean(raw && raw.signed)
  };
}

function normalizeProfile(input: AnyObj): EmbeddedObservabilityProfile {
  const defaultProfile: EmbeddedObservabilityProfile = {
    profile_id: 'observability_profile_primary',
    version: 1,
    red_legion_trace_channels: ['runtime.guardrails', 'lane.integrity', 'chaos.replay', 'sovereignty.index'],
    allowed_emitters: ['systems/observability', 'systems/red_legion', 'systems/security', 'crates/observability'],
    stream_policy: {
      trace_window_ms: 1000,
      max_events_per_window: 1024,
      min_sampling_rate_pct: 25,
      redact_fields: ['secret', 'token', 'private_key', 'api_key'],
      require_signature: true
    },
    sovereignty_scorer: {
      integrity_weight_pct: 45,
      continuity_weight_pct: 25,
      reliability_weight_pct: 20,
      chaos_penalty_pct: 10,
      fail_closed_threshold_pct: 60
    },
    chaos_hooks: [
      {
        id: 'hook.fail_closed.on_tamper',
        condition: 'event.severity == critical && event.tag == tamper',
        action: 'trip_fail_closed',
        severity: 'critical',
        enabled: true
      },
      {
        id: 'hook.rate_limit.on_storm',
        condition: 'window.events > max_events_per_window',
        action: 'drop_low_priority',
        severity: 'high',
        enabled: true
      },
      {
        id: 'hook.score_penalty.on_drift',
        condition: 'replay.drift > 0',
        action: 'apply_chaos_penalty',
        severity: 'medium',
        enabled: true
      }
    ]
  };

  const source = input && typeof input === 'object' ? input : defaultProfile;
  return {
    profile_id: cleanText(source.profile_id || defaultProfile.profile_id, 160),
    version: Number.isFinite(Number(source.version)) ? Math.max(1, Math.floor(Number(source.version))) : defaultProfile.version,
    red_legion_trace_channels: Array.isArray(source.red_legion_trace_channels)
      ? source.red_legion_trace_channels.map((v: unknown) => cleanText(v, 120)).filter(Boolean)
      : defaultProfile.red_legion_trace_channels,
    allowed_emitters: Array.isArray(source.allowed_emitters)
      ? source.allowed_emitters.map((v: unknown) => cleanText(v, 120)).filter(Boolean)
      : defaultProfile.allowed_emitters,
    stream_policy: {
      trace_window_ms: Number.isFinite(Number(source.stream_policy && source.stream_policy.trace_window_ms))
        ? Math.max(1, Math.floor(Number(source.stream_policy.trace_window_ms)))
        : defaultProfile.stream_policy.trace_window_ms,
      max_events_per_window: Number.isFinite(Number(source.stream_policy && source.stream_policy.max_events_per_window))
        ? Math.max(1, Math.floor(Number(source.stream_policy.max_events_per_window)))
        : defaultProfile.stream_policy.max_events_per_window,
      min_sampling_rate_pct: Number.isFinite(Number(source.stream_policy && source.stream_policy.min_sampling_rate_pct))
        ? Math.max(0, Math.min(100, Math.floor(Number(source.stream_policy.min_sampling_rate_pct))))
        : defaultProfile.stream_policy.min_sampling_rate_pct,
      redact_fields: Array.isArray(source.stream_policy && source.stream_policy.redact_fields)
        ? source.stream_policy.redact_fields.map((v: unknown) => cleanText(v, 120)).filter(Boolean)
        : defaultProfile.stream_policy.redact_fields,
      require_signature: Boolean(source.stream_policy && source.stream_policy.require_signature != null
        ? source.stream_policy.require_signature
        : defaultProfile.stream_policy.require_signature)
    },
    sovereignty_scorer: {
      integrity_weight_pct: Number.isFinite(Number(source.sovereignty_scorer && source.sovereignty_scorer.integrity_weight_pct))
        ? Math.max(0, Math.min(100, Math.floor(Number(source.sovereignty_scorer.integrity_weight_pct))))
        : defaultProfile.sovereignty_scorer.integrity_weight_pct,
      continuity_weight_pct: Number.isFinite(Number(source.sovereignty_scorer && source.sovereignty_scorer.continuity_weight_pct))
        ? Math.max(0, Math.min(100, Math.floor(Number(source.sovereignty_scorer.continuity_weight_pct))))
        : defaultProfile.sovereignty_scorer.continuity_weight_pct,
      reliability_weight_pct: Number.isFinite(Number(source.sovereignty_scorer && source.sovereignty_scorer.reliability_weight_pct))
        ? Math.max(0, Math.min(100, Math.floor(Number(source.sovereignty_scorer.reliability_weight_pct))))
        : defaultProfile.sovereignty_scorer.reliability_weight_pct,
      chaos_penalty_pct: Number.isFinite(Number(source.sovereignty_scorer && source.sovereignty_scorer.chaos_penalty_pct))
        ? Math.max(0, Math.min(100, Math.floor(Number(source.sovereignty_scorer.chaos_penalty_pct))))
        : defaultProfile.sovereignty_scorer.chaos_penalty_pct,
      fail_closed_threshold_pct: Number.isFinite(Number(source.sovereignty_scorer && source.sovereignty_scorer.fail_closed_threshold_pct))
        ? Math.max(0, Math.min(100, Math.floor(Number(source.sovereignty_scorer.fail_closed_threshold_pct))))
        : defaultProfile.sovereignty_scorer.fail_closed_threshold_pct
    },
    chaos_hooks: Array.isArray(source.chaos_hooks)
      ? source.chaos_hooks.map((row: AnyObj) => ({
        id: cleanText(row && row.id, 120),
        condition: cleanText(row && row.condition, 180),
        action: cleanText(row && row.action, 120),
        severity: cleanText(row && row.severity, 40).toLowerCase(),
        enabled: Boolean(row && row.enabled)
      }))
      : defaultProfile.chaos_hooks
  };
}

function digestLines(lines: string[]) {
  const h = crypto.createHash('sha256');
  for (let i = 0; i < lines.length; i += 1) {
    h.update(`${i}:${lines[i]}|`, 'utf8');
  }
  return h.digest('hex');
}

function eventFingerprint(event: TraceEvent) {
  const tags = Array.isArray(event.tags) ? event.tags.slice().sort() : [];
  return [
    cleanText(event.trace_id, 160),
    String(event.ts_millis),
    cleanText(event.source, 160),
    cleanText(event.operation, 160),
    cleanText(event.severity, 32),
    cleanText(event.payload_digest, 256),
    String(Boolean(event.signed)),
    ...tags.map((tag) => cleanText(tag, 80))
  ].join('|');
}

function channelTriggered(channel: string, events: TraceEvent[]) {
  const c = cleanText(channel, 120).toLowerCase();
  return events.some((event) => (event.tags || []).some((tag) => {
    const t = cleanText(tag, 120).toLowerCase();
    return t === c || t.startsWith(c);
  }));
}

function hookTriggered(hook: AnyObj, traceReport: AnyObj, events: TraceEvent[]) {
  if (!hook || hook.enabled !== true) return false;
  const cond = cleanText(hook.condition, 220).toLowerCase();
  if (cond.includes('tamper')) {
    return events.some((event) => event.severity === 'critical' && (event.tags || []).some((tag) => cleanText(tag, 80).toLowerCase().includes('tamper')));
  }
  if (cond.includes('window.events')) {
    return Number(traceReport && traceReport.dropped_events || 0) > 0;
  }
  if (cond.includes('replay.drift')) {
    return Number(traceReport && traceReport.drift_score_pct || 0) > 0;
  }
  return false;
}

function continuityComponent(events: TraceEvent[], windowMs: number) {
  if (events.length <= 1) return 100;
  let monotonicOk = 0;
  let totalPairs = 0;
  let gapPenalties = 0;
  for (let i = 1; i < events.length; i += 1) {
    totalPairs += 1;
    const left = events[i - 1];
    const right = events[i];
    if (right.ts_millis >= left.ts_millis) monotonicOk += 1;
    if ((right.ts_millis - left.ts_millis) > (windowMs * 2)) gapPenalties += 1;
  }
  if (totalPairs === 0) return 100;
  const monotonicRatio = monotonicOk / totalPairs;
  const gapPenalty = gapPenalties / totalPairs;
  const score = (monotonicRatio * 100) - (gapPenalty * 35);
  return Math.max(0, Math.min(100, score));
}

function reliabilityComponent(events: TraceEvent[], acceptedEvents: number) {
  if (events.length === 0) return 100;
  const acceptedRatio = acceptedEvents / events.length;
  const severeRatio = events.filter((event) => event.severity === 'critical' || event.severity === 'high').length / events.length;
  const score = (acceptedRatio * 100) - (severeRatio * 25);
  return Math.max(0, Math.min(100, score));
}

function evaluateTraceWindow(profile: EmbeddedObservabilityProfile, eventsIn: AnyObj[]) {
  const events = (Array.isArray(eventsIn) ? eventsIn : []).map(normalizeEvent);
  const accepted = events.slice(0, profile.stream_policy.max_events_per_window);
  const droppedEvents = Math.max(0, events.length - accepted.length);
  const highSeverityEvents = accepted.filter((event) => event.severity === 'critical' || event.severity === 'high').length;
  const redLegionChannelsTriggered = profile.red_legion_trace_channels.filter((channel) => channelTriggered(channel, accepted));
  const eventDigest = digestLines(accepted.map(eventFingerprint));

  const driftWeightSum = accepted
    .filter((event) => event.tags.some((tag) => cleanText(tag, 80).toLowerCase().includes('drift')))
    .reduce((sum, event) => sum + severityWeight(event.severity), 0);
  const driftScorePct = accepted.length === 0 ? 0 : Math.max(0, Math.min(100, (driftWeightSum / accepted.length) * 100));

  return {
    accepted_events: accepted.length,
    dropped_events: droppedEvents,
    high_severity_events: highSeverityEvents,
    red_legion_channels_triggered: redLegionChannelsTriggered,
    event_digest: eventDigest,
    drift_score_pct: round3(driftScorePct)
  };
}

function computeSovereigntyIndex(profile: EmbeddedObservabilityProfile, eventsIn: AnyObj[], traceReport: AnyObj, injectFaultEvery: number, enforceFailClosed: boolean) {
  const events = (Array.isArray(eventsIn) ? eventsIn : []).map(normalizeEvent);
  const acceptedEvents = events.slice(0, profile.stream_policy.max_events_per_window);

  const integrityComponentPct = acceptedEvents.length === 0
    ? 100
    : (acceptedEvents.filter((event) => event.signed).length / acceptedEvents.length) * 100;
  const continuityComponentPct = continuityComponent(acceptedEvents, profile.stream_policy.trace_window_ms);
  const reliabilityComponentPct = reliabilityComponent(events, Number(traceReport.accepted_events || 0));

  const faultPenalty = injectFaultEvery <= 0 ? 0 : Math.max(0, Math.min(40, 100 / injectFaultEvery));
  const driftPenalty = Math.max(0, Math.min(25, Number(traceReport.drift_score_pct || 0) * 0.25));
  const chaosPenaltyPct = Math.max(0, Math.min(100, faultPenalty + driftPenalty));

  const weights = profile.sovereignty_scorer;
  const weightedScore = ((integrityComponentPct * weights.integrity_weight_pct)
    + (continuityComponentPct * weights.continuity_weight_pct)
    + (reliabilityComponentPct * weights.reliability_weight_pct)) / 100
    - ((chaosPenaltyPct * weights.chaos_penalty_pct) / 100);

  const scorePct = round3(Math.max(0, Math.min(100, weightedScore)));
  const reasons: string[] = [];
  if (integrityComponentPct < 70) reasons.push('integrity_component_below_70');
  if (continuityComponentPct < 70) reasons.push('continuity_component_below_70');
  if (reliabilityComponentPct < 70) reasons.push('reliability_component_below_70');
  if (chaosPenaltyPct > 15) reasons.push('chaos_penalty_above_15');

  const tamperCritical = acceptedEvents.some((event) => event.severity === 'critical' && event.tags.some((tag) => cleanText(tag, 80).toLowerCase().includes('tamper')));
  if (tamperCritical) reasons.push('critical_tamper_detected');

  const threshold = Number(weights.fail_closed_threshold_pct || 0);
  const failClosed = (scorePct < threshold && enforceFailClosed) || (tamperCritical && enforceFailClosed);
  const status = failClosed ? 'fail_closed' : (scorePct < threshold ? 'degraded' : 'stable');

  return {
    score_pct: scorePct,
    fail_closed: failClosed,
    status,
    reasons,
    integrity_component_pct: round3(integrityComponentPct),
    continuity_component_pct: round3(continuityComponentPct),
    reliability_component_pct: round3(reliabilityComponentPct),
    chaos_penalty_pct: round3(chaosPenaltyPct)
  };
}

function runChaosObservabilityLegacy(requestRaw: AnyObj, profileRaw: AnyObj) {
  const profile = normalizeProfile(profileRaw);
  const request: ChaosScenarioRequest = {
    scenario_id: cleanText(requestRaw && requestRaw.scenario_id, 160),
    events: Array.isArray(requestRaw && requestRaw.events) ? requestRaw.events.map(normalizeEvent) : [],
    cycles: Number.isFinite(Number(requestRaw && requestRaw.cycles)) ? Math.max(0, Math.floor(Number(requestRaw.cycles))) : 0,
    inject_fault_every: Number.isFinite(Number(requestRaw && requestRaw.inject_fault_every)) ? Math.max(0, Math.floor(Number(requestRaw.inject_fault_every))) : 0,
    enforce_fail_closed: Boolean(requestRaw && requestRaw.enforce_fail_closed)
  };

  const traceReport = evaluateTraceWindow(profile, request.events);
  const sovereignty = computeSovereigntyIndex(profile, request.events, traceReport, request.inject_fault_every, request.enforce_fail_closed);

  const acceptedEvents = request.events.slice(0, profile.stream_policy.max_events_per_window);
  const hooksFired = profile.chaos_hooks
    .filter((hook) => hookTriggered(hook, traceReport, acceptedEvents))
    .map((hook) => hook.id);

  const telemetryOverheadMs = round3((Number(traceReport.accepted_events || 0) * 0.00045)
    + ((Array.isArray(traceReport.red_legion_channels_triggered) ? traceReport.red_legion_channels_triggered.length : 0) * 0.08)
    + 0.12);
  const injectFactor = request.inject_fault_every <= 0
    ? 0
    : Math.max(0.05, Math.min(2.5, 250 / request.inject_fault_every));
  const chaosBatteryPct24h = round3((request.cycles / 200000) * 1.2
    + (Number(traceReport.high_severity_events || 0) * 0.01)
    + injectFactor
    + 0.25);
  const resilient = sovereignty.fail_closed !== true && telemetryOverheadMs <= 1.0 && chaosBatteryPct24h <= 3.0;

  return {
    profile_id: profile.profile_id,
    scenario_id: request.scenario_id,
    hooks_fired: hooksFired,
    trace_report: traceReport,
    sovereignty,
    telemetry_overhead_ms: telemetryOverheadMs,
    chaos_battery_pct_24h: chaosBatteryPct24h,
    resilient
  };
}

module.exports = {
  normalizeProfile,
  evaluateTraceWindow,
  computeSovereigntyIndex,
  runChaosObservabilityLegacy
};
