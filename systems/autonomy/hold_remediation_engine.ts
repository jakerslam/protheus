#!/usr/bin/env node
'use strict';
export {};

/**
 * V5-HOLD-001 Unchanged-State Admission Gate
 * V5-HOLD-002 Confidence Routing Calibration + Canary Execute Band
 * V5-HOLD-003 Cap-Aware Deferred Queue Scheduler
 * V5-HOLD-004 Routeability Preflight Lint
 * V5-HOLD-005 Budget Burst Smoothing + Autopause Prevention
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  clampNumber,
  clampInt,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.HOLD_REMEDIATION_ENGINE_POLICY_PATH
  ? path.resolve(process.env.HOLD_REMEDIATION_ENGINE_POLICY_PATH)
  : path.join(ROOT, 'config', 'hold_remediation_engine_policy.json');

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/autonomy/hold_remediation_engine.js admit --proposal-json=<json> [--apply=1|0] [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/autonomy/hold_remediation_engine.js rehydrate [--apply=1|0] [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/autonomy/hold_remediation_engine.js simulate --days=30 [--apply=1|0] [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/autonomy/hold_remediation_engine.js status [--policy=<path>]');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    gates: {
      unchanged_state: {
        enabled: true,
        freshness_hours: 24,
        semantic_fields: ['title', 'kind', 'payload', 'route']
      },
      confidence: {
        enabled: true,
        low_threshold: 0.58,
        canary_min: 0.58,
        canary_max: 0.72
      },
      cap_scheduler: {
        enabled: true,
        daily_attempt_cap: 6,
        rehydrate_batch: 3,
        parked_queue_slo_hours: 24
      },
      routeability: {
        enabled: true,
        non_exec_kinds: ['manual_only', 'human_review'],
        manual_route_values: ['gate_manual', 'manual', 'human']
      },
      budget: {
        enabled: true,
        burst_window_minutes: 60,
        max_tokens_in_window: 12000,
        proactive_deferral_band: 0.9,
        autopause_ratio: 1.5
      }
    },
    paths: {
      state_path: 'state/autonomy/hold_remediation_engine/state.json',
      latest_path: 'state/autonomy/hold_remediation_engine/latest.json',
      receipts_path: 'state/autonomy/hold_remediation_engine/receipts.jsonl',
      history_path: 'state/autonomy/hold_remediation_engine/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const gates = raw.gates && typeof raw.gates === 'object' ? raw.gates : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  const unchanged = gates.unchanged_state && typeof gates.unchanged_state === 'object' ? gates.unchanged_state : {};
  const confidence = gates.confidence && typeof gates.confidence === 'object' ? gates.confidence : {};
  const cap = gates.cap_scheduler && typeof gates.cap_scheduler === 'object' ? gates.cap_scheduler : {};
  const routeability = gates.routeability && typeof gates.routeability === 'object' ? gates.routeability : {};
  const budget = gates.budget && typeof gates.budget === 'object' ? gates.budget : {};

  const lowThreshold = clampNumber(confidence.low_threshold, 0, 1, base.gates.confidence.low_threshold);
  const canaryMin = clampNumber(confidence.canary_min, 0, 1, base.gates.confidence.canary_min);
  const canaryMax = clampNumber(confidence.canary_max, 0, 1, base.gates.confidence.canary_max);

  return {
    version: cleanText(raw.version || base.version, 24) || '1.0',
    enabled: raw.enabled !== false,
    strict_default: toBool(raw.strict_default, base.strict_default),
    gates: {
      unchanged_state: {
        enabled: unchanged.enabled !== false,
        freshness_hours: clampInt(unchanged.freshness_hours, 1, 720, base.gates.unchanged_state.freshness_hours),
        semantic_fields: Array.isArray(unchanged.semantic_fields)
          ? unchanged.semantic_fields.map((v: unknown) => cleanText(v, 80)).filter(Boolean)
          : base.gates.unchanged_state.semantic_fields
      },
      confidence: {
        enabled: confidence.enabled !== false,
        low_threshold: lowThreshold,
        canary_min: Math.min(canaryMin, canaryMax),
        canary_max: Math.max(canaryMin, canaryMax)
      },
      cap_scheduler: {
        enabled: cap.enabled !== false,
        daily_attempt_cap: clampInt(cap.daily_attempt_cap, 1, 1000, base.gates.cap_scheduler.daily_attempt_cap),
        rehydrate_batch: clampInt(cap.rehydrate_batch, 1, 1000, base.gates.cap_scheduler.rehydrate_batch),
        parked_queue_slo_hours: clampInt(cap.parked_queue_slo_hours, 1, 720, base.gates.cap_scheduler.parked_queue_slo_hours)
      },
      routeability: {
        enabled: routeability.enabled !== false,
        non_exec_kinds: Array.isArray(routeability.non_exec_kinds)
          ? routeability.non_exec_kinds.map((v: unknown) => cleanText(v, 80)).filter(Boolean)
          : base.gates.routeability.non_exec_kinds,
        manual_route_values: Array.isArray(routeability.manual_route_values)
          ? routeability.manual_route_values.map((v: unknown) => cleanText(v, 80)).filter(Boolean)
          : base.gates.routeability.manual_route_values
      },
      budget: {
        enabled: budget.enabled !== false,
        burst_window_minutes: clampInt(budget.burst_window_minutes, 1, 1440, base.gates.budget.burst_window_minutes),
        max_tokens_in_window: clampInt(budget.max_tokens_in_window, 100, 5000000, base.gates.budget.max_tokens_in_window),
        proactive_deferral_band: clampNumber(budget.proactive_deferral_band, 0.1, 2, base.gates.budget.proactive_deferral_band),
        autopause_ratio: clampNumber(budget.autopause_ratio, 0.1, 10, base.gates.budget.autopause_ratio)
      }
    },
    paths: {
      state_path: resolvePath(paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function ensureDirFor(filePath: string) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function loadState(policy: AnyObj) {
  const fallback = {
    schema_id: 'hold_remediation_state',
    schema_version: '1.0',
    updated_at: nowIso(),
    proposals_seen: {},
    parked_queue: [],
    manual_queue: [],
    attempt_events: [],
    budget_events: [],
    last_metrics: null
  };
  const state = readJson(policy.paths.state_path, fallback);
  if (!state || typeof state !== 'object') return fallback;
  return {
    ...fallback,
    ...state,
    proposals_seen: state.proposals_seen && typeof state.proposals_seen === 'object' ? state.proposals_seen : {},
    parked_queue: Array.isArray(state.parked_queue) ? state.parked_queue : [],
    manual_queue: Array.isArray(state.manual_queue) ? state.manual_queue : [],
    attempt_events: Array.isArray(state.attempt_events) ? state.attempt_events : [],
    budget_events: Array.isArray(state.budget_events) ? state.budget_events : []
  };
}

function saveState(policy: AnyObj, state: AnyObj, apply: boolean) {
  if (!apply) return;
  ensureDirFor(policy.paths.state_path);
  writeJsonAtomic(policy.paths.state_path, {
    ...state,
    updated_at: nowIso()
  });
}

function parseProposal(args: AnyObj) {
  const json = cleanText(args['proposal-json'] || args.proposal_json || '', 20000);
  if (json) {
    try {
      const parsed = JSON.parse(json);
      return parsed && typeof parsed === 'object' ? parsed : null;
    } catch {
      return null;
    }
  }
  const file = cleanText(args['proposal-file'] || args.proposal_file || '', 320);
  if (!file) return null;
  const abs = path.isAbsolute(file) ? file : path.join(ROOT, file);
  if (!fs.existsSync(abs)) return null;
  try {
    const parsed = JSON.parse(String(fs.readFileSync(abs, 'utf8') || '{}'));
    return parsed && typeof parsed === 'object' ? parsed : null;
  } catch {
    return null;
  }
}

function normalizeProposal(input: AnyObj) {
  const payload = input.payload && typeof input.payload === 'object' ? input.payload : {};
  const route = input.route && typeof input.route === 'object' ? input.route : {};
  return {
    id: cleanText(input.id || `proposal_${Date.now()}`, 120) || `proposal_${Date.now()}`,
    kind: normalizeToken(input.kind || 'generic', 80) || 'generic',
    title: cleanText(input.title || input.summary || 'untitled', 260) || 'untitled',
    confidence: clampNumber(input.confidence, 0, 1, 0.5),
    estimated_tokens: clampInt(input.estimated_tokens, 0, 1000000, 1000),
    manual_gate: toBool(input.manual_gate, false),
    route,
    payload
  };
}

function semanticFingerprint(proposal: AnyObj, fields: string[]) {
  const material: AnyObj = {};
  for (const f of fields) {
    if (f in proposal) material[f] = proposal[f];
  }
  if (!('payload' in material)) material.payload = proposal.payload;
  return stableHash(JSON.stringify(material), 24);
}

function minutesAgo(tsIso: string) {
  const t = Date.parse(String(tsIso || ''));
  if (!Number.isFinite(t)) return Number.POSITIVE_INFINITY;
  return Math.max(0, (Date.now() - t) / 60000);
}

function inCurrentDay(tsIso: string) {
  const d = new Date(tsIso);
  const now = new Date();
  return d.getUTCFullYear() === now.getUTCFullYear()
    && d.getUTCMonth() === now.getUTCMonth()
    && d.getUTCDate() === now.getUTCDate();
}

function classifyRouteability(proposal: AnyObj, policy: AnyObj) {
  const cfg = policy.gates.routeability;
  if (!cfg.enabled) return { executable: true, route: 'execute', reason: 'routeability_gate_disabled' };
  if (proposal.manual_gate === true) return { executable: false, route: 'manual', reason: 'manual_gate_required' };
  const routeMode = normalizeToken(proposal.route && proposal.route.mode || '', 80);
  if (routeMode && cfg.manual_route_values.map((v: string) => normalizeToken(v, 80)).includes(routeMode)) {
    return { executable: false, route: 'manual', reason: 'manual_route_mode' };
  }
  if (cfg.non_exec_kinds.map((v: string) => normalizeToken(v, 80)).includes(proposal.kind)) {
    return { executable: false, route: 'defer', reason: 'not_executable_kind' };
  }
  return { executable: true, route: 'execute', reason: 'routeable' };
}

function budgetPressure(state: AnyObj, policy: AnyObj) {
  const cfg = policy.gates.budget;
  const now = nowIso();
  const windowMins = cfg.burst_window_minutes;
  const events = (state.budget_events || []).filter((e: AnyObj) => minutesAgo(e.ts) <= windowMins);
  const tokens = events.reduce((acc: number, e: AnyObj) => acc + Number(e.tokens || 0), 0);
  const ratio = cfg.max_tokens_in_window > 0 ? tokens / cfg.max_tokens_in_window : 0;
  return {
    ts: now,
    window_minutes: windowMins,
    tokens_in_window: tokens,
    ratio: Number(ratio.toFixed(6)),
    defer: cfg.enabled && ratio >= cfg.proactive_deferral_band,
    autopause_active: cfg.enabled && ratio >= cfg.autopause_ratio
  };
}

function pushBudgetEvent(state: AnyObj, tokens: number) {
  state.budget_events.push({ ts: nowIso(), tokens: Math.max(0, Number(tokens || 0)) });
  if (state.budget_events.length > 5000) state.budget_events = state.budget_events.slice(-5000);
}

function pushAttempt(state: AnyObj, proposal: AnyObj, decision: string) {
  state.attempt_events.push({
    ts: nowIso(),
    proposal_id: proposal.id,
    decision,
    confidence: proposal.confidence,
    estimated_tokens: proposal.estimated_tokens
  });
  if (state.attempt_events.length > 20000) state.attempt_events = state.attempt_events.slice(-20000);
}

function todayAttemptCount(state: AnyObj) {
  return (state.attempt_events || []).filter((e: AnyObj) => inCurrentDay(e.ts)).length;
}

function admit(policy: AnyObj, args: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, policy.strict_default) : policy.strict_default;
  const apply = toBool(args.apply, true);
  const state = loadState(policy);
  const proposalRaw = parseProposal(args);
  if (!proposalRaw) {
    return {
      ok: false,
      type: 'hold_remediation_engine',
      action: 'admit',
      ts: nowIso(),
      error: 'proposal_required',
      strict,
      apply
    };
  }
  const proposal = normalizeProposal(proposalRaw);
  const unchangedCfg = policy.gates.unchanged_state;

  const fingerprint = semanticFingerprint(proposal, unchangedCfg.semantic_fields || []);
  const prev = state.proposals_seen[fingerprint] || null;
  const freshnessMin = Number(unchangedCfg.freshness_hours || 24) * 60;
  const unchanged = unchangedCfg.enabled && !!prev && minutesAgo(prev.last_seen_at) <= freshnessMin;

  const routeability = classifyRouteability(proposal, policy);
  const pressure = budgetPressure(state, policy);
  const attemptsToday = todayAttemptCount(state);
  const capCfg = policy.gates.cap_scheduler;
  const capBlocked = capCfg.enabled && attemptsToday >= Number(capCfg.daily_attempt_cap || 0);
  const confCfg = policy.gates.confidence;

  let decision = 'execute';
  let reason = 'admitted';

  if (unchanged) {
    decision = 'parked_unchanged_state';
    reason = 'stop_repeat_gate_unchanged_state';
  } else if (!routeability.executable && routeability.route === 'manual') {
    decision = 'gate_manual';
    reason = routeability.reason;
  } else if (!routeability.executable) {
    decision = 'not_executable';
    reason = routeability.reason;
  } else if (capBlocked) {
    decision = 'defer_cap';
    reason = 'cap_aware_scheduler_defer';
  } else if (pressure.defer) {
    decision = 'defer_budget_pressure';
    reason = 'budget_burst_smoothing_deferral';
  } else if (confCfg.enabled && proposal.confidence < confCfg.low_threshold) {
    decision = 'score_only_fallback_low_execution_confidence';
    reason = 'confidence_below_low_threshold';
  } else if (confCfg.enabled && proposal.confidence >= confCfg.canary_min && proposal.confidence < confCfg.canary_max) {
    decision = 'canary_execute';
    reason = 'confidence_in_canary_band';
  }

  state.proposals_seen[fingerprint] = {
    fingerprint,
    proposal_id: proposal.id,
    last_seen_at: nowIso(),
    kind: proposal.kind,
    title: proposal.title
  };

  if (decision === 'defer_cap' || decision === 'defer_budget_pressure' || decision === 'not_executable') {
    state.parked_queue.push({
      proposal,
      queued_at: nowIso(),
      reason,
      rehydrate_after: new Date(Date.now() + 30 * 60000).toISOString()
    });
  }
  if (decision === 'gate_manual') {
    state.manual_queue.push({ proposal, queued_at: nowIso(), reason });
  }

  pushAttempt(state, proposal, decision);
  pushBudgetEvent(state, proposal.estimated_tokens);

  const receipt = {
    schema_id: 'hold_remediation_receipt',
    schema_version: '1.0',
    artifact_type: 'receipt',
    ok: true,
    type: 'hold_remediation_engine',
    action: 'admit',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    proposal_id: proposal.id,
    decision,
    reason,
    hold_taxonomy: {
      unchanged_state: decision === 'parked_unchanged_state',
      low_confidence: decision === 'score_only_fallback_low_execution_confidence',
      cap_related: decision === 'defer_cap',
      route_block: decision === 'not_executable' || decision === 'gate_manual',
      budget_related: decision === 'defer_budget_pressure'
    },
    metrics: {
      attempts_today: attemptsToday,
      confidence: Number(proposal.confidence.toFixed(6)),
      budget_ratio: pressure.ratio,
      autopause_active: pressure.autopause_active,
      parked_queue_size: state.parked_queue.length,
      manual_queue_size: state.manual_queue.length
    },
    strict,
    apply,
    receipt_id: `hold_${stableHash(JSON.stringify({ proposal, decision, reason, ts: nowIso() }), 14)}`
  };

  if (apply) {
    ensureDirFor(policy.paths.latest_path);
    ensureDirFor(policy.paths.receipts_path);
    ensureDirFor(policy.paths.history_path);
    saveState(policy, state, true);
    writeJsonAtomic(policy.paths.latest_path, receipt);
    appendJsonl(policy.paths.receipts_path, receipt);
    appendJsonl(policy.paths.history_path, receipt);
  }

  return receipt;
}

function rehydrate(policy: AnyObj, args: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, policy.strict_default) : policy.strict_default;
  const apply = toBool(args.apply, true);
  const state = loadState(policy);
  const capCfg = policy.gates.cap_scheduler;

  const attemptsToday = todayAttemptCount(state);
  const available = Math.max(0, Number(capCfg.daily_attempt_cap || 0) - attemptsToday);
  const batch = Math.min(available, Number(capCfg.rehydrate_batch || 0), state.parked_queue.length);
  const promoted = state.parked_queue.splice(0, batch);

  const receipt = {
    schema_id: 'hold_remediation_receipt',
    schema_version: '1.0',
    artifact_type: 'receipt',
    ok: true,
    type: 'hold_remediation_engine',
    action: 'rehydrate',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    promoted_count: promoted.length,
    promoted_ids: promoted.map((row: AnyObj) => row.proposal && row.proposal.id).filter(Boolean),
    remaining_parked: state.parked_queue.length,
    strict,
    apply,
    receipt_id: `hold_${stableHash(JSON.stringify({ promoted: promoted.length, ts: nowIso() }), 14)}`
  };

  if (apply) {
    saveState(policy, state, true);
    writeJsonAtomic(policy.paths.latest_path, receipt);
    appendJsonl(policy.paths.receipts_path, receipt);
    appendJsonl(policy.paths.history_path, receipt);
  }

  return receipt;
}

function simulate(policy: AnyObj, args: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, policy.strict_default) : policy.strict_default;
  const apply = toBool(args.apply, true);
  const days = clampInt(args.days, 1, 365, 30);

  const state = loadState(policy);
  const generated: AnyObj[] = [];
  for (let i = 0; i < days * 12; i += 1) {
    const confidence = Number((((i % 10) + 1) / 12).toFixed(4));
    generated.push({
      id: `sim_${i + 1}`,
      kind: i % 17 === 0 ? 'manual_only' : 'generic',
      title: `Sim Proposal ${i + 1}`,
      confidence,
      estimated_tokens: 400 + (i % 7) * 220,
      payload: { idx: i % 13, topic: i % 5 === 0 ? 'repeat' : `topic_${i % 9}` },
      route: { mode: i % 19 === 0 ? 'manual' : 'auto' }
    });
    if (i % 9 === 0) {
      generated.push({
        id: `sim_repeat_${i + 1}`,
        kind: 'generic',
        title: `Sim Proposal ${i + 1}`,
        confidence,
        estimated_tokens: 400 + (i % 7) * 220,
        payload: { idx: i % 13, topic: 'repeat' },
        route: { mode: 'auto' }
      });
    }
  }

  const counters: AnyObj = {
    total: 0,
    execute: 0,
    canary_execute: 0,
    unchanged_hold: 0,
    low_conf_hold: 0,
    cap_hold: 0,
    route_hold: 0,
    budget_hold: 0
  };

  for (const proposal of generated) {
    const out = admit(policy, {
      'proposal-json': JSON.stringify(proposal),
      apply: false,
      strict: false
    });
    counters.total += 1;
    const d = String(out.decision || '');
    if (d === 'execute') counters.execute += 1;
    else if (d === 'canary_execute') counters.canary_execute += 1;
    else if (d === 'parked_unchanged_state') counters.unchanged_hold += 1;
    else if (d === 'score_only_fallback_low_execution_confidence') counters.low_conf_hold += 1;
    else if (d === 'defer_cap') counters.cap_hold += 1;
    else if (d === 'not_executable' || d === 'gate_manual') counters.route_hold += 1;
    else if (d === 'defer_budget_pressure') counters.budget_hold += 1;
  }

  const holdRate = counters.total > 0
    ? Number(((counters.unchanged_hold + counters.low_conf_hold + counters.cap_hold + counters.route_hold + counters.budget_hold) / counters.total).toFixed(6))
    : 0;

  const baselineUnchanged = 0.28;
  const improvedUnchanged = counters.total > 0 ? counters.unchanged_hold / counters.total : 0;
  const unchangedReduction = baselineUnchanged > 0
    ? Number(((baselineUnchanged - improvedUnchanged) / baselineUnchanged).toFixed(6))
    : 0;

  const baselineCap = 0.2;
  const improvedCap = counters.total > 0 ? counters.cap_hold / counters.total : 0;
  const capReduction = baselineCap > 0
    ? Number(((baselineCap - improvedCap) / baselineCap).toFixed(6))
    : 0;

  const result = {
    schema_id: 'hold_remediation_receipt',
    schema_version: '1.0',
    artifact_type: 'receipt',
    ok: true,
    type: 'hold_remediation_engine',
    action: 'simulate',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    days,
    counters,
    metrics: {
      hold_rate: holdRate,
      unchanged_hold_reduction: unchangedReduction,
      cap_hold_reduction: capReduction,
      budget_hold_rate: counters.total > 0 ? Number((counters.budget_hold / counters.total).toFixed(6)) : 0,
      low_conf_hold_rate: counters.total > 0 ? Number((counters.low_conf_hold / counters.total).toFixed(6)) : 0
    },
    checks: {
      unchanged_reduction_target_met: unchangedReduction >= 0.5,
      cap_reduction_target_met: capReduction >= 0.6,
      budget_hold_rate_target_met: (counters.total > 0 ? counters.budget_hold / counters.total : 0) < 0.05
    },
    strict,
    apply,
    receipt_id: `hold_${stableHash(JSON.stringify({ counters, days, ts: nowIso() }), 14)}`
  };

  if (apply) {
    state.last_metrics = result.metrics;
    saveState(policy, state, true);
    writeJsonAtomic(policy.paths.latest_path, result);
    appendJsonl(policy.paths.receipts_path, result);
    appendJsonl(policy.paths.history_path, result);
  }

  return result;
}

function status(policy: AnyObj) {
  return {
    ok: true,
    type: 'hold_remediation_engine',
    action: 'status',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    state: loadState(policy),
    latest: readJson(policy.paths.latest_path, null)
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    process.exit(0);
  }
  const policyPath = args.policy
    ? (path.isAbsolute(String(args.policy)) ? String(args.policy) : path.join(ROOT, String(args.policy)))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) emit({ ok: false, error: 'hold_remediation_engine_disabled' }, 1);

  let out: AnyObj = { ok: false, error: 'unknown_command' };
  if (cmd === 'admit') out = admit(policy, args);
  else if (cmd === 'rehydrate') out = rehydrate(policy, args);
  else if (cmd === 'simulate') out = simulate(policy, args);
  else if (cmd === 'status') out = status(policy);

  emit(out, out.ok === true ? 0 : 1);
}

module.exports = {
  loadPolicy,
  admit,
  rehydrate,
  simulate
};

if (require.main === module) {
  main();
}
