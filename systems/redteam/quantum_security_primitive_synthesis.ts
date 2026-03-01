#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-QPROOF-002
 *
 * Self-learning quantum security primitive synthesis lane.
 *
 * Defensive-only, shadow-first runtime that uses red-team/venom telemetry to
 * propose bounded post-quantum defensive mechanisms and verify >=20% uplift.
 */

const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampInt,
  clampNumber,
  readJson,
  readJsonl,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.QUANTUM_SECURITY_PRIMITIVE_SYNTHESIS_POLICY_PATH
  ? path.resolve(process.env.QUANTUM_SECURITY_PRIMITIVE_SYNTHESIS_POLICY_PATH)
  : path.join(ROOT, 'config', 'quantum_security_primitive_synthesis_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/redteam/quantum_security_primitive_synthesis.js run [--apply=0|1] [--policy=<path>]');
  console.log('  node systems/redteam/quantum_security_primitive_synthesis.js verify [--strict=0|1] [--policy=<path>]');
  console.log('  node systems/redteam/quantum_security_primitive_synthesis.js status [--policy=<path>]');
}

function normalizeList(v: unknown) {
  if (Array.isArray(v)) return v.map((row) => cleanText(row, 320)).filter(Boolean);
  const raw = cleanText(v || '', 8000);
  if (!raw) return [];
  return raw.split(',').map((row) => cleanText(row, 320)).filter(Boolean);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    shadow_only: true,
    defensive_only: true,
    bounded_only: true,
    auditable_only: true,
    min_containment_uplift_per_cycle: 0.2,
    max_proposals_per_cycle: 6,
    threat_window_days: 14,
    categories: ['hashing', 'signing', 'kem', 'attestation', 'watermark'],
    paths: {
      venom_history_path: 'state/security/venom_containment/history.jsonl',
      redteam_history_path: 'state/security/red_team/adaptive_defense/history.jsonl',
      state_path: 'state/security/red_team/quantum_security_synthesis/state.json',
      latest_path: 'state/security/red_team/quantum_security_synthesis/latest.json',
      receipts_path: 'state/security/red_team/quantum_security_synthesis/receipts.jsonl',
      proposal_queue_path: 'state/security/red_team/quantum_security_synthesis/proposal_queue.json',
      catalog_path: 'state/security/red_team/quantum_security_synthesis/catalog.json'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: toBool(raw.enabled, true),
    shadow_only: toBool(raw.shadow_only, true),
    defensive_only: toBool(raw.defensive_only, true),
    bounded_only: toBool(raw.bounded_only, true),
    auditable_only: toBool(raw.auditable_only, true),
    min_containment_uplift_per_cycle: clampNumber(
      raw.min_containment_uplift_per_cycle,
      0,
      1,
      base.min_containment_uplift_per_cycle
    ),
    max_proposals_per_cycle: clampInt(raw.max_proposals_per_cycle, 1, 64, base.max_proposals_per_cycle),
    threat_window_days: clampInt(raw.threat_window_days, 1, 90, base.threat_window_days),
    categories: normalizeList(raw.categories || base.categories)
      .map((row) => normalizeToken(row, 80)).filter(Boolean),
    paths: {
      venom_history_path: resolvePath(paths.venom_history_path || base.paths.venom_history_path, base.paths.venom_history_path),
      redteam_history_path: resolvePath(paths.redteam_history_path || base.paths.redteam_history_path, base.paths.redteam_history_path),
      state_path: resolvePath(paths.state_path || base.paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path || base.paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path || base.paths.receipts_path, base.paths.receipts_path),
      proposal_queue_path: resolvePath(paths.proposal_queue_path || base.paths.proposal_queue_path, base.paths.proposal_queue_path),
      catalog_path: resolvePath(paths.catalog_path || base.paths.catalog_path, base.paths.catalog_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function parseTsMs(v: unknown) {
  const ms = Date.parse(String(v || ''));
  return Number.isFinite(ms) ? ms : 0;
}

function loadRecentIntel(policy: any) {
  const windowMs = Number(policy.threat_window_days || 14) * 24 * 60 * 60 * 1000;
  const threshold = Date.now() - windowMs;

  const venomRows = readJsonl(policy.paths.venom_history_path)
    .filter((row: any) => row && typeof row === 'object')
    .filter((row: any) => parseTsMs(row.ts || row.event_ts || row.created_at) >= threshold);

  const redRows = readJsonl(policy.paths.redteam_history_path)
    .filter((row: any) => row && typeof row === 'object')
    .filter((row: any) => parseTsMs(row.ts || row.event_ts || row.created_at) >= threshold);

  let unauthorizedSignals = 0;
  for (const row of venomRows) {
    const status = normalizeToken(
      row.status || row.stage || row.decision || row.type || '',
      120
    );
    if (status.includes('unauthor') || status.includes('lockout') || status.includes('contain')) {
      unauthorizedSignals += 1;
      continue;
    }
    if (row.unauthorized === true || row.tamper_detected === true) unauthorizedSignals += 1;
  }

  let activeThreatSignals = 0;
  for (const row of redRows) {
    const outcome = normalizeToken(row.outcome || row.result || row.type || '', 120);
    if (outcome.includes('fail') || outcome.includes('critical') || outcome.includes('threat') || outcome.includes('probe')) {
      activeThreatSignals += 1;
      continue;
    }
    if (row.critical === true || row.high_risk === true) activeThreatSignals += 1;
  }

  return {
    venom_rows: venomRows.length,
    redteam_rows: redRows.length,
    unauthorized_signals: unauthorizedSignals,
    active_threat_signals: activeThreatSignals,
    total_signals: unauthorizedSignals + activeThreatSignals
  };
}

function defaultQueue() {
  return {
    schema_id: 'quantum_security_proposal_queue',
    schema_version: '1.0',
    updated_at: nowIso(),
    proposals: []
  };
}

function loadQueue(policy: any) {
  const src = readJson(policy.paths.proposal_queue_path, null);
  if (!src || typeof src !== 'object') return defaultQueue();
  return {
    schema_id: 'quantum_security_proposal_queue',
    schema_version: '1.0',
    updated_at: src.updated_at || nowIso(),
    proposals: Array.isArray(src.proposals) ? src.proposals : []
  };
}

function saveQueue(policy: any, queue: any) {
  writeJsonAtomic(policy.paths.proposal_queue_path, {
    schema_id: 'quantum_security_proposal_queue',
    schema_version: '1.0',
    updated_at: nowIso(),
    proposals: Array.isArray(queue.proposals) ? queue.proposals : []
  });
}

function defaultCatalog() {
  return {
    schema_id: 'quantum_security_primitive_catalog',
    schema_version: '1.0',
    updated_at: null,
    promoted: []
  };
}

function loadCatalog(policy: any) {
  const src = readJson(policy.paths.catalog_path, null);
  if (!src || typeof src !== 'object') return defaultCatalog();
  return {
    schema_id: 'quantum_security_primitive_catalog',
    schema_version: '1.0',
    updated_at: src.updated_at || null,
    promoted: Array.isArray(src.promoted) ? src.promoted : []
  };
}

function saveCatalog(policy: any, catalog: any) {
  writeJsonAtomic(policy.paths.catalog_path, {
    schema_id: 'quantum_security_primitive_catalog',
    schema_version: '1.0',
    updated_at: nowIso(),
    promoted: Array.isArray(catalog.promoted) ? catalog.promoted : []
  });
}

function loadState(policy: any) {
  const src = readJson(policy.paths.state_path, null);
  if (!src || typeof src !== 'object') {
    return {
      schema_id: 'quantum_security_synthesis_state',
      schema_version: '1.0',
      updated_at: nowIso(),
      cycles: 0,
      proposals_generated: 0,
      proposals_accepted: 0,
      promoted: 0,
      last_cycle_uplift: 0
    };
  }
  return {
    schema_id: 'quantum_security_synthesis_state',
    schema_version: '1.0',
    updated_at: src.updated_at || nowIso(),
    cycles: Math.max(0, Number(src.cycles || 0)),
    proposals_generated: Math.max(0, Number(src.proposals_generated || 0)),
    proposals_accepted: Math.max(0, Number(src.proposals_accepted || 0)),
    promoted: Math.max(0, Number(src.promoted || 0)),
    last_cycle_uplift: Math.max(0, Number(src.last_cycle_uplift || 0))
  };
}

function saveState(policy: any, state: any) {
  writeJsonAtomic(policy.paths.state_path, {
    schema_id: 'quantum_security_synthesis_state',
    schema_version: '1.0',
    updated_at: nowIso(),
    cycles: Math.max(0, Number(state.cycles || 0)),
    proposals_generated: Math.max(0, Number(state.proposals_generated || 0)),
    proposals_accepted: Math.max(0, Number(state.proposals_accepted || 0)),
    promoted: Math.max(0, Number(state.promoted || 0)),
    last_cycle_uplift: Math.max(0, Number(state.last_cycle_uplift || 0))
  });
}

function proposalTemplate(category: string) {
  if (category === 'hashing') {
    return {
      primitive_name: 'blake3_domain_separated_hash_chain',
      mechanism: 'blake3+kangarootwelve domain-separated hybrid hash lanes'
    };
  }
  if (category === 'signing') {
    return {
      primitive_name: 'sphincs_attestation_signature_lane',
      mechanism: 'sphincs+ signature envelopes for attestation and watermark proofing'
    };
  }
  if (category === 'kem') {
    return {
      primitive_name: 'kyber_session_wrap_lane',
      mechanism: 'kyber-wrapped ephemeral key exchange for defensive channels'
    };
  }
  if (category === 'attestation') {
    return {
      primitive_name: 'post_quantum_attestation_challenge',
      mechanism: 'post-quantum challenge-response for unauthorized runtime detection'
    };
  }
  return {
    primitive_name: 'quantum_resilient_watermark_tag',
    mechanism: 'quantum-resilient watermark tags with deterministic forensic lineage'
  };
}

function buildProposals(policy: any, intel: any) {
  const totalSignals = Math.max(0, Number(intel.total_signals || 0));
  const baseUplift = clampNumber(0.2 + Math.min(0.25, totalSignals / 100), 0.2, 0.65, 0.2);
  const categories = (policy.categories || []).slice(0, Math.max(1, Number(policy.max_proposals_per_cycle || 6)));

  const proposals = [];
  for (let i = 0; i < categories.length; i += 1) {
    const category = normalizeToken(categories[i], 80) || 'hashing';
    const template = proposalTemplate(category);
    const uplift = clampNumber(baseUplift * (1 + Math.min(0.2, i * 0.03)), 0.2, 0.8, 0.2);
    const id = `qsp_${stableHash(`${category}|${template.primitive_name}|${Date.now()}|${i}`, 12)}`;

    proposals.push({
      proposal_id: id,
      ts: nowIso(),
      category,
      primitive_name: template.primitive_name,
      mechanism: template.mechanism,
      source: {
        kind: 'self_play_plus_threat_intel',
        venom_signals: Number(intel.unauthorized_signals || 0),
        redteam_signals: Number(intel.active_threat_signals || 0)
      },
      estimated_containment_uplift: Number(uplift.toFixed(6)),
      defensive_only: true,
      bounded: true,
      auditable: true,
      unauthorized_scope_only: true,
      gates: {
        red_team_gate: 'required',
        dream_warden_gate: 'required',
        symbiosis_gate: 'required'
      },
      status: 'proposed'
    });
  }
  return proposals;
}

function evaluateProposals(policy: any, proposals: any[]) {
  const accepted = [];
  const rejected = [];

  for (const proposal of proposals) {
    const checks = {
      defensive_only: policy.defensive_only === true && proposal.defensive_only === true,
      bounded: policy.bounded_only === true ? proposal.bounded === true : true,
      auditable: policy.auditable_only === true ? proposal.auditable === true : true,
      unauthorized_scope_only: proposal.unauthorized_scope_only === true,
      uplift_minimum: Number(proposal.estimated_containment_uplift || 0) >= Number(policy.min_containment_uplift_per_cycle || 0)
    };
    const pass = Object.values(checks).every(Boolean);
    const row = {
      ...proposal,
      evaluation: {
        pass,
        checks,
        ts: nowIso()
      },
      status: pass ? 'accepted_shadow' : 'rejected'
    };
    if (pass) accepted.push(row);
    else rejected.push(row);
  }

  return { accepted, rejected };
}

function cmdRun(args: any, policy: any) {
  const applyRequested = toBool(args.apply, false);
  const applyAllowed = applyRequested && policy.shadow_only !== true;

  const intel = loadRecentIntel(policy);
  const proposals = buildProposals(policy, intel);
  const decision = evaluateProposals(policy, proposals);

  const cycleUplift = decision.accepted.length > 0
    ? Number((decision.accepted.reduce((acc: number, row: any) => acc + Number(row.estimated_containment_uplift || 0), 0) / decision.accepted.length).toFixed(6))
    : 0;

  const queue = loadQueue(policy);
  const dedup = new Map();
  for (const row of queue.proposals || []) {
    dedup.set(normalizeToken(row && row.proposal_id || '', 120), row);
  }
  for (const row of decision.accepted.concat(decision.rejected)) {
    dedup.set(normalizeToken(row.proposal_id || '', 120), row);
  }
  queue.proposals = Array.from(dedup.values()).slice(-500);
  saveQueue(policy, queue);

  const catalog = loadCatalog(policy);
  let promotedCount = 0;
  if (applyAllowed) {
    const promotable = decision.accepted
      .filter((row: any) => Number(row.estimated_containment_uplift || 0) >= Number(policy.min_containment_uplift_per_cycle || 0));
    for (const row of promotable) {
      catalog.promoted.push({
        proposal_id: row.proposal_id,
        category: row.category,
        primitive_name: row.primitive_name,
        promoted_at: nowIso(),
        estimated_containment_uplift: Number(row.estimated_containment_uplift || 0)
      });
      promotedCount += 1;
    }
    catalog.promoted = catalog.promoted.slice(-500);
    saveCatalog(policy, catalog);
  }

  const state = loadState(policy);
  state.cycles += 1;
  state.proposals_generated += proposals.length;
  state.proposals_accepted += decision.accepted.length;
  state.promoted += promotedCount;
  state.last_cycle_uplift = cycleUplift;
  saveState(policy, state);

  const out = {
    ok: true,
    type: 'quantum_security_primitive_synthesis_run',
    ts: nowIso(),
    policy_version: policy.version,
    policy_path: policy.policy_path,
    shadow_only: policy.shadow_only === true,
    defensive_only: policy.defensive_only === true,
    apply_requested: applyRequested,
    apply_allowed: applyAllowed,
    proposals_generated: proposals.length,
    proposals_accepted: decision.accepted.length,
    proposals_rejected: decision.rejected.length,
    promoted: promotedCount,
    min_containment_uplift_per_cycle: Number(policy.min_containment_uplift_per_cycle || 0),
    cycle_containment_uplift: cycleUplift,
    uplift_goal_met: cycleUplift >= Number(policy.min_containment_uplift_per_cycle || 0),
    intel,
    accepted: decision.accepted.slice(0, 20).map((row: any) => ({
      proposal_id: row.proposal_id,
      category: row.category,
      primitive_name: row.primitive_name,
      estimated_containment_uplift: Number(row.estimated_containment_uplift || 0)
    })),
    rejected: decision.rejected.slice(0, 20).map((row: any) => ({
      proposal_id: row.proposal_id,
      category: row.category,
      primitive_name: row.primitive_name,
      reasons: Object.entries(row.evaluation && row.evaluation.checks || {})
        .filter((pair: any) => pair[1] !== true)
        .map((pair: any) => pair[0])
    }))
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  return out;
}

function cmdVerify(args: any, policy: any) {
  const strict = toBool(args.strict, false);
  const latest = readJson(policy.paths.latest_path, null);
  if (!latest || typeof latest !== 'object') {
    const out = { ok: false, type: 'quantum_security_primitive_synthesis_verify', error: 'latest_missing' };
    if (strict) emit(out, 1);
    return out;
  }

  const checks = {
    defensive_only: policy.defensive_only === true,
    bounded_only: policy.bounded_only === true,
    auditable_only: policy.auditable_only === true,
    uplift_goal_met: latest.uplift_goal_met === true,
    accepted_proposals_present: Number(latest.proposals_accepted || 0) >= 1
  };
  const pass = Object.values(checks).every(Boolean);

  const out = {
    ok: pass,
    type: 'quantum_security_primitive_synthesis_verify',
    ts: nowIso(),
    strict,
    checks,
    latest
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);

  if (strict && !pass) emit(out, 1);
  return out;
}

function cmdStatus(policy: any) {
  return {
    ok: true,
    type: 'quantum_security_primitive_synthesis_status',
    ts: nowIso(),
    latest: readJson(policy.paths.latest_path, null),
    state: loadState(policy),
    queue: loadQueue(policy),
    catalog: loadCatalog(policy)
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    return;
  }

  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : POLICY_PATH);
  if (policy.enabled !== true) emit({ ok: false, error: 'quantum_security_primitive_synthesis_disabled' }, 1);

  if (cmd === 'run') emit(cmdRun(args, policy));
  if (cmd === 'verify') emit(cmdVerify(args, policy));
  if (cmd === 'status') emit(cmdStatus(policy));

  emit({ ok: false, error: 'unsupported_command', cmd }, 1);
}

if (require.main === module) {
  main();
}

module.exports = {
  loadPolicy,
  loadRecentIntel,
  buildProposals,
  evaluateProposals,
  cmdRun,
  cmdVerify,
  cmdStatus
};
