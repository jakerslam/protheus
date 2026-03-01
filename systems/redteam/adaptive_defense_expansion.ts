#!/usr/bin/env node
'use strict';
export {};

/**
 * adaptive_defense_expansion.js
 *
 * Implements:
 * - V3-RED-002 Open-Ended Defensive Tool Discovery + Evolution Lane
 * - V3-RED-ESC-001..004 Cost-escalation and evolutionary trainer
 * - V3-RED-HALL-001..003 Governed exemption lane + registry/audit
 * - V3-RED-NASTY-001..004 Defensive nasty profile generation (bounded)
 *
 * Defensive-only and shadow-first by policy.
 *
 * Usage:
 *   node systems/redteam/adaptive_defense_expansion.js run [--policy=/abs/path.json] [--state-root=/abs/path]
 *   node systems/redteam/adaptive_defense_expansion.js request-exemption --scope=<id> [--reason=text] [--duration-hours=24]
 *   node systems/redteam/adaptive_defense_expansion.js approve-exemption --id=<id> --approver=<id> --approval-note="note"
 *   node systems/redteam/adaptive_defense_expansion.js audit-exemptions [--strict=1]
 *   node systems/redteam/adaptive_defense_expansion.js status
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

type AnyObj = Record<string, any>;

const ROOT = path.resolve(__dirname, '..', '..');
const DEFAULT_POLICY_PATH = process.env.REDTEAM_ADAPTIVE_DEFENSE_POLICY_PATH
  ? path.resolve(process.env.REDTEAM_ADAPTIVE_DEFENSE_POLICY_PATH)
  : path.join(ROOT, 'config', 'redteam_adaptive_defense_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) {
      out._.push(tok);
      continue;
    }
    const eq = tok.indexOf('=');
    if (eq >= 0) {
      out[tok.slice(2, eq)] = tok.slice(eq + 1);
      continue;
    }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeToken(v: unknown, maxLen = 120) {
  return cleanText(v, maxLen)
    .toLowerCase()
    .replace(/[^a-z0-9_.:/-]+/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function toBool(v: unknown, fallback = false) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function clampInt(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  const i = Math.floor(n);
  if (i < lo) return lo;
  if (i > hi) return hi;
  return i;
}

function clampNumber(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return n;
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath: string, fallback: any = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    return parsed == null ? fallback : parsed;
  } catch {
    return fallback;
  }
}

function writeJsonAtomic(filePath: string, value: AnyObj) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: AnyObj) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function readJsonl(filePath: string) {
  try {
    if (!fs.existsSync(filePath)) return [];
    return String(fs.readFileSync(filePath, 'utf8') || '')
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        try { return JSON.parse(line); } catch { return null; }
      })
      .filter(Boolean);
  } catch {
    return [];
  }
}

function resolvePath(raw: unknown, fallbackRel: string) {
  const text = cleanText(raw, 520);
  if (!text) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(text) ? text : path.join(ROOT, text);
}

function relPath(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function parseIsoMs(v: unknown): number | null {
  const ms = Date.parse(String(v || ''));
  return Number.isFinite(ms) ? ms : null;
}

function hash12(v: unknown) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, 12);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    shadow_only: true,
    defensive_only: true,
    runtime_fingerprint_classes: ['unknown', 'desktop', 'cloud_vm', 'gpu_heavy', 'containerized'],
    categories: [
      'detection',
      'friction',
      'decoy',
      'verification',
      'containment',
      'temporal_deception',
      'adaptive_honeypot',
      'stack_pressure'
    ],
    limits: {
      max_tool_proposals_per_run: 24,
      max_category_proposals_per_run: 4,
      max_friction_delay_ms: 2200,
      max_rate_limit_per_minute: 40,
      max_resource_sink_factor: 3.5,
      max_children_per_incident: 4,
      minimum_uplift_target: 0.2
    },
    hall_pass: {
      enabled: true,
      default_duration_hours: 24,
      max_duration_hours: 168,
      non_exemptible: ['defensive_only_invariant', 'constitution_root', 'soul_token_binding', 'attestation_binding', 'legal_bounds']
    },
    nasty_profiles: {
      tease_trap_enabled: true,
      stack_resource_bias_enabled: true,
      psychological_decay_enabled: true,
      containment_children_enabled: true
    },
    paths: {
      state_root: 'state/security/red_team/adaptive_defense',
      latest_path: 'state/security/red_team/adaptive_defense/latest.json',
      history_path: 'state/security/red_team/adaptive_defense/history.jsonl',
      tool_catalog_path: 'state/security/red_team/adaptive_defense/tool_catalog.json',
      cost_profiles_path: 'state/security/red_team/adaptive_defense/cost_profiles.json',
      exemptions_path: 'state/security/red_team/adaptive_defense/exemptions.json',
      registry_audit_path: 'state/security/red_team/adaptive_defense/exemption_audit.jsonl',
      venom_history_path: 'state/security/venom_containment/history.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const limits = raw.limits && typeof raw.limits === 'object' ? raw.limits : {};
  const hall = raw.hall_pass && typeof raw.hall_pass === 'object' ? raw.hall_pass : {};
  const nasty = raw.nasty_profiles && typeof raw.nasty_profiles === 'object' ? raw.nasty_profiles : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: toBool(raw.enabled, true),
    shadow_only: toBool(raw.shadow_only, true),
    defensive_only: toBool(raw.defensive_only, true),
    runtime_fingerprint_classes: Array.isArray(raw.runtime_fingerprint_classes)
      ? raw.runtime_fingerprint_classes.map((v: unknown) => normalizeToken(v, 80)).filter(Boolean)
      : base.runtime_fingerprint_classes,
    categories: Array.isArray(raw.categories)
      ? raw.categories.map((v: unknown) => normalizeToken(v, 80)).filter(Boolean)
      : base.categories,
    limits: {
      max_tool_proposals_per_run: clampInt(limits.max_tool_proposals_per_run, 1, 200, base.limits.max_tool_proposals_per_run),
      max_category_proposals_per_run: clampInt(limits.max_category_proposals_per_run, 1, 40, base.limits.max_category_proposals_per_run),
      max_friction_delay_ms: clampInt(limits.max_friction_delay_ms, 100, 20000, base.limits.max_friction_delay_ms),
      max_rate_limit_per_minute: clampInt(limits.max_rate_limit_per_minute, 1, 10000, base.limits.max_rate_limit_per_minute),
      max_resource_sink_factor: clampNumber(limits.max_resource_sink_factor, 1, 20, base.limits.max_resource_sink_factor),
      max_children_per_incident: clampInt(limits.max_children_per_incident, 0, 50, base.limits.max_children_per_incident),
      minimum_uplift_target: clampNumber(limits.minimum_uplift_target, 0, 1, base.limits.minimum_uplift_target)
    },
    hall_pass: {
      enabled: toBool(hall.enabled, base.hall_pass.enabled),
      default_duration_hours: clampInt(hall.default_duration_hours, 1, 24 * 30, base.hall_pass.default_duration_hours),
      max_duration_hours: clampInt(hall.max_duration_hours, 1, 24 * 180, base.hall_pass.max_duration_hours),
      non_exemptible: Array.isArray(hall.non_exemptible)
        ? hall.non_exemptible.map((v: unknown) => normalizeToken(v, 100)).filter(Boolean)
        : base.hall_pass.non_exemptible
    },
    nasty_profiles: {
      tease_trap_enabled: toBool(nasty.tease_trap_enabled, base.nasty_profiles.tease_trap_enabled),
      stack_resource_bias_enabled: toBool(nasty.stack_resource_bias_enabled, base.nasty_profiles.stack_resource_bias_enabled),
      psychological_decay_enabled: toBool(nasty.psychological_decay_enabled, base.nasty_profiles.psychological_decay_enabled),
      containment_children_enabled: toBool(nasty.containment_children_enabled, base.nasty_profiles.containment_children_enabled)
    },
    paths: {
      state_root: resolvePath(paths.state_root, base.paths.state_root),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      tool_catalog_path: resolvePath(paths.tool_catalog_path, base.paths.tool_catalog_path),
      cost_profiles_path: resolvePath(paths.cost_profiles_path, base.paths.cost_profiles_path),
      exemptions_path: resolvePath(paths.exemptions_path, base.paths.exemptions_path),
      registry_audit_path: resolvePath(paths.registry_audit_path, base.paths.registry_audit_path),
      venom_history_path: resolvePath(paths.venom_history_path, base.paths.venom_history_path)
    }
  };
}

function defensiveStatus(policy: AnyObj) {
  return {
    ok: policy.defensive_only === true,
    reason: policy.defensive_only === true ? 'ok' : 'defensive_only_disabled'
  };
}

function runtimeClass(v: unknown, policy: AnyObj) {
  const tok = normalizeToken(v || 'unknown', 80) || 'unknown';
  const known = new Set((policy.runtime_fingerprint_classes || []).map((x: unknown) => normalizeToken(x, 80)));
  return known.has(tok) ? tok : 'unknown';
}

function defaultCatalog(policy: AnyObj) {
  return {
    schema_version: '1.0',
    updated_at: null,
    tools: [],
    categories: Array.isArray(policy.categories) ? policy.categories : []
  };
}

function defaultCostProfiles() {
  return {
    schema_version: '1.0',
    updated_at: null,
    fingerprint_profiles: {
      unknown: { challenge_multiplier: 1, friction_multiplier: 1, decoy_intensity: 1, rate_limit_per_minute: 40 },
      desktop: { challenge_multiplier: 1, friction_multiplier: 1, decoy_intensity: 1, rate_limit_per_minute: 40 },
      cloud_vm: { challenge_multiplier: 1.15, friction_multiplier: 1.2, decoy_intensity: 1.1, rate_limit_per_minute: 30 },
      gpu_heavy: { challenge_multiplier: 1.3, friction_multiplier: 1.25, decoy_intensity: 1.2, rate_limit_per_minute: 24 },
      containerized: { challenge_multiplier: 1.1, friction_multiplier: 1.15, decoy_intensity: 1.05, rate_limit_per_minute: 32 }
    },
    last_uplift: 0
  };
}

function loadCatalog(policy: AnyObj) {
  return readJson(policy.paths.tool_catalog_path, defaultCatalog(policy)) || defaultCatalog(policy);
}

function loadCostProfiles(policy: AnyObj) {
  return readJson(policy.paths.cost_profiles_path, defaultCostProfiles()) || defaultCostProfiles();
}

function loadExemptions(policy: AnyObj) {
  const fallback = {
    schema_version: '1.0',
    updated_at: null,
    exemptions: []
  };
  return readJson(policy.paths.exemptions_path, fallback) || fallback;
}

function saveCatalog(policy: AnyObj, value: AnyObj) {
  writeJsonAtomic(policy.paths.tool_catalog_path, value);
}

function saveCostProfiles(policy: AnyObj, value: AnyObj) {
  writeJsonAtomic(policy.paths.cost_profiles_path, value);
}

function saveExemptions(policy: AnyObj, value: AnyObj) {
  writeJsonAtomic(policy.paths.exemptions_path, value);
}

function parseVenomIncidents(policy: AnyObj, maxRows = 500) {
  return readJsonl(policy.paths.venom_history_path)
    .filter((row) => row && row.type === 'venom_containment_evaluation' && row.unauthorized === true)
    .slice(-Math.max(0, maxRows));
}

function proposeToolsFromIncidents(policy: AnyObj, incidents: AnyObj[], existingCatalog: AnyObj) {
  const existingTools = new Set((existingCatalog.tools || []).map((row: any) => normalizeToken(row.id || '', 120)).filter(Boolean));
  const categories = new Set((existingCatalog.categories || []).map((v: unknown) => normalizeToken(v, 80)).filter(Boolean));

  const stageCounts: Record<string, number> = {};
  const runtimeCounts: Record<string, number> = {};
  for (const row of incidents) {
    const stage = normalizeToken(row.stage || 'none', 40) || 'none';
    const klass = runtimeClass(row.runtime_class || 'unknown', policy);
    stageCounts[stage] = (stageCounts[stage] || 0) + 1;
    runtimeCounts[klass] = (runtimeCounts[klass] || 0) + 1;
  }

  const proposals = [];
  const pushTool = (category: string, name: string, params: AnyObj) => {
    if (proposals.length >= Number(policy.limits.max_tool_proposals_per_run || 24)) return;
    const id = `def_${normalizeToken(category, 40)}_${hash12(`${name}_${JSON.stringify(params)}`)}`;
    if (existingTools.has(id)) return;
    proposals.push({
      id,
      category,
      name,
      params,
      bounded: true,
      reversible: true,
      legal: true,
      contained_only: true,
      proposed_at: nowIso()
    });
    existingTools.add(id);
    categories.add(category);
  };

  const total = Math.max(1, incidents.length);
  const lockoutRatio = (stageCounts.lockout || 0) / total;
  const degradeRatio = (stageCounts.degrade || 0) / total;

  pushTool('detection', 'attestation_anomaly_classifier', {
    signal_window: Math.min(240, incidents.length),
    sensitivity: Number((0.45 + lockoutRatio * 0.4).toFixed(4))
  });

  pushTool('friction', 'adaptive_verification_gate', {
    base_delay_ms: clampInt(350 + Math.round(degradeRatio * 900), 150, policy.limits.max_friction_delay_ms, 350),
    max_rate_limit_per_minute: Number(policy.limits.max_rate_limit_per_minute || 40)
  });

  pushTool('decoy', 'watermarked_low_value_lane', {
    decoy_intensity: Number((1 + lockoutRatio * 0.5).toFixed(4)),
    watermark_required: true
  });

  pushTool('verification', 'runtime_fingerprint_differential', {
    fingerprints: Object.keys(runtimeCounts).filter(Boolean).slice(0, 12)
  });

  if ((runtimeCounts.gpu_heavy || 0) > 0) {
    pushTool('stack_pressure', 'gpu_bias_challenge_profile', {
      challenge_multiplier: Number((1.15 + (runtimeCounts.gpu_heavy / total) * 0.35).toFixed(4))
    });
  }
  if ((runtimeCounts.cloud_vm || 0) > 0) {
    pushTool('stack_pressure', 'cloud_latency_pressure_profile', {
      friction_multiplier: Number((1.1 + (runtimeCounts.cloud_vm / total) * 0.3).toFixed(4))
    });
  }

  const newCategoryCandidates = [];
  if ((stageCounts.challenge || 0) > 0) {
    newCategoryCandidates.push('temporal_deception');
  }
  if ((stageCounts.degrade || 0) > 0) {
    newCategoryCandidates.push('adaptive_honeypot');
  }
  if ((runtimeCounts.containerized || 0) > 0) {
    newCategoryCandidates.push('stack_pressure');
  }

  const newCategories = [];
  for (const cat of newCategoryCandidates) {
    if (newCategories.length >= Number(policy.limits.max_category_proposals_per_run || 4)) break;
    if (!categories.has(cat)) {
      newCategories.push(cat);
      categories.add(cat);
    }
  }

  return {
    proposals,
    new_categories: newCategories,
    stage_counts: stageCounts,
    runtime_counts: runtimeCounts
  };
}

function updateCostProfiles(policy: AnyObj, incidents: AnyObj[], prior: AnyObj) {
  const next = { ...prior };
  next.updated_at = nowIso();
  if (!next.fingerprint_profiles || typeof next.fingerprint_profiles !== 'object') {
    next.fingerprint_profiles = defaultCostProfiles().fingerprint_profiles;
  }

  const counts: Record<string, number> = {};
  for (const row of incidents) {
    const klass = runtimeClass(row.runtime_class || 'unknown', policy);
    counts[klass] = (counts[klass] || 0) + 1;
  }
  const total = Math.max(1, incidents.length);

  for (const klass of Object.keys(next.fingerprint_profiles)) {
    const ratio = (counts[klass] || 0) / total;
    const p = next.fingerprint_profiles[klass] || {};
    const challenge = clampNumber((Number(p.challenge_multiplier || 1) * 0.65) + (1 + ratio * 0.9) * 0.35, 1, 3.5, 1);
    const friction = clampNumber((Number(p.friction_multiplier || 1) * 0.65) + (1 + ratio * 1.1) * 0.35, 1, 4, 1);
    const decoy = clampNumber((Number(p.decoy_intensity || 1) * 0.65) + (1 + ratio * 0.75) * 0.35, 1, 3, 1);
    const rpmBase = Number(policy.limits.max_rate_limit_per_minute || 40);
    const rpm = clampInt(Math.round(rpmBase * (1 - Math.min(0.65, ratio * 0.7))), 1, rpmBase, rpmBase);
    next.fingerprint_profiles[klass] = {
      challenge_multiplier: Number(challenge.toFixed(4)),
      friction_multiplier: Number(friction.toFixed(4)),
      decoy_intensity: Number(decoy.toFixed(4)),
      rate_limit_per_minute: rpm
    };
  }

  const lockouts = incidents.filter((row) => String(row.stage || '') === 'lockout').length;
  const uplift = total > 0 ? lockouts / total : 0;
  next.last_uplift = Number(uplift.toFixed(4));
  return next;
}

function buildNastyProfiles(policy: AnyObj, costProfiles: AnyObj) {
  const fp = costProfiles && costProfiles.fingerprint_profiles && typeof costProfiles.fingerprint_profiles === 'object'
    ? costProfiles.fingerprint_profiles
    : defaultCostProfiles().fingerprint_profiles;

  return {
    tease_trap: {
      enabled: policy.nasty_profiles.tease_trap_enabled === true,
      normal_actions_before_trap: 2,
      trap_stage: 'challenge',
      deterministic: true
    },
    stack_resource_bias: {
      enabled: policy.nasty_profiles.stack_resource_bias_enabled === true,
      fingerprint_profiles: fp,
      bounded_resource_sink_factor: Number(policy.limits.max_resource_sink_factor || 3.5)
    },
    psychological_decay: {
      enabled: policy.nasty_profiles.psychological_decay_enabled === true,
      fake_glitch_probability: 0.22,
      bounded_extra_latency_ms: clampInt(Math.round(Number(policy.limits.max_friction_delay_ms || 2200) * 0.55), 100, Number(policy.limits.max_friction_delay_ms || 2200), 900)
    },
    containment_children: {
      enabled: policy.nasty_profiles.containment_children_enabled === true,
      max_children: Number(policy.limits.max_children_per_incident || 4),
      strategy: 'bounded_recursive_decoy_children'
    }
  };
}

function auditExemptions(policy: AnyObj, opts: AnyObj = {}) {
  const state = loadExemptions(policy);
  const rows = Array.isArray(state.exemptions) ? state.exemptions : [];
  const now = Date.now();
  const nonExemptible = new Set((policy.hall_pass.non_exemptible || []).map((v: unknown) => normalizeToken(v, 100)));

  const expired = [];
  const outOfScope = [];
  const active = [];
  for (const row of rows) {
    const expiresMs = parseIsoMs(row.expires_at);
    const scope = normalizeToken(row.scope || '', 120);
    const status = normalizeToken(row.status || 'pending', 40);
    if (nonExemptible.has(scope)) outOfScope.push(row);
    if (status === 'approved' && expiresMs != null && expiresMs < now) {
      expired.push(row);
      continue;
    }
    active.push(row);
  }

  const next = {
    ...state,
    updated_at: nowIso(),
    exemptions: active
  };
  saveExemptions(policy, next);

  const audit = {
    ok: expired.length === 0 && outOfScope.length === 0,
    type: 'redteam_exemption_audit',
    ts: nowIso(),
    expired_count: expired.length,
    out_of_scope_count: outOfScope.length,
    active_count: active.length
  };
  appendJsonl(policy.paths.registry_audit_path, audit);

  if (opts.strict === true && !audit.ok) {
    return {
      ...audit,
      error: 'exemption_audit_failed',
      expired,
      out_of_scope: outOfScope
    };
  }

  return {
    ...audit,
    expired,
    out_of_scope: outOfScope
  };
}

function requestExemption(policy: AnyObj, input: AnyObj = {}) {
  const status = defensiveStatus(policy);
  if (!status.ok || policy.hall_pass.enabled !== true) {
    return {
      ok: false,
      type: 'redteam_exemption_request',
      error: 'hall_pass_disabled_or_defensive_invariant_failed'
    };
  }

  const scope = normalizeToken(input.scope || '', 120);
  if (!scope) {
    return {
      ok: false,
      type: 'redteam_exemption_request',
      error: 'scope_required'
    };
  }

  const nonExemptible = new Set((policy.hall_pass.non_exemptible || []).map((v: unknown) => normalizeToken(v, 100)));
  if (nonExemptible.has(scope)) {
    return {
      ok: false,
      type: 'redteam_exemption_request',
      error: 'scope_non_exemptible',
      scope
    };
  }

  const hours = clampInt(input.duration_hours, 1, Number(policy.hall_pass.max_duration_hours || 168), Number(policy.hall_pass.default_duration_hours || 24));
  const id = `hall_${Date.now()}_${hash12(`${scope}_${input.reason || ''}`)}`;
  const createdAt = nowIso();
  const expiresAt = new Date(Date.now() + hours * 60 * 60 * 1000).toISOString();

  const state = loadExemptions(policy);
  const row = {
    id,
    scope,
    reason: cleanText(input.reason || 'redteam_defense_rnd', 220),
    status: 'pending',
    created_at: createdAt,
    expires_at: expiresAt,
    duration_hours: hours,
    approver: null,
    approval_note: null,
    rollback_metadata: {
      reversible: true,
      rollback_required: true
    }
  };

  state.updated_at = createdAt;
  state.exemptions = Array.isArray(state.exemptions) ? state.exemptions : [];
  state.exemptions.push(row);
  saveExemptions(policy, state);

  appendJsonl(policy.paths.registry_audit_path, {
    ts: createdAt,
    type: 'redteam_exemption_requested',
    id,
    scope,
    status: 'pending'
  });

  return {
    ok: true,
    type: 'redteam_exemption_request',
    ts: createdAt,
    request: row
  };
}

function approveExemption(policy: AnyObj, input: AnyObj = {}) {
  const id = normalizeToken(input.id || '', 180);
  const approver = normalizeToken(input.approver || '', 120);
  const note = cleanText(input.approval_note || '', 260);
  if (!id || !approver || !note) {
    return {
      ok: false,
      type: 'redteam_exemption_approve',
      error: 'id_approver_note_required'
    };
  }

  const state = loadExemptions(policy);
  const rows = Array.isArray(state.exemptions) ? state.exemptions : [];
  const idx = rows.findIndex((row: any) => normalizeToken(row.id || '', 180) === id);
  if (idx < 0) {
    return {
      ok: false,
      type: 'redteam_exemption_approve',
      error: 'exemption_not_found',
      id
    };
  }

  rows[idx] = {
    ...rows[idx],
    status: 'approved',
    approver,
    approval_note: note,
    approved_at: nowIso()
  };
  state.updated_at = nowIso();
  state.exemptions = rows;
  saveExemptions(policy, state);

  appendJsonl(policy.paths.registry_audit_path, {
    ts: nowIso(),
    type: 'redteam_exemption_approved',
    id,
    approver
  });

  return {
    ok: true,
    type: 'redteam_exemption_approve',
    ts: nowIso(),
    id,
    approver,
    status: 'approved'
  };
}

function runAdaptiveDefenseExpansion(input: AnyObj = {}, opts: AnyObj = {}) {
  const policy = opts.policy || loadPolicy(opts.policyPath || DEFAULT_POLICY_PATH);
  const def = defensiveStatus(policy);
  if (!def.ok) {
    return {
      ok: false,
      type: 'redteam_adaptive_defense_run',
      ts: nowIso(),
      error: 'defensive_only_invariant_failed'
    };
  }

  if (policy.enabled !== true) {
    return {
      ok: true,
      type: 'redteam_adaptive_defense_run',
      ts: nowIso(),
      enabled: false,
      reason: 'adaptive_defense_disabled'
    };
  }

  const incidents = parseVenomIncidents(policy, 600);
  const catalog = loadCatalog(policy);
  const profiles = loadCostProfiles(policy);

  const toolResult = proposeToolsFromIncidents(policy, incidents, catalog);
  const mergedTools = Array.isArray(catalog.tools) ? catalog.tools.slice(0) : [];
  for (const row of toolResult.proposals) mergedTools.push(row);

  const nextCatalog = {
    schema_version: '1.0',
    updated_at: nowIso(),
    categories: Array.from(new Set([...(catalog.categories || []), ...toolResult.new_categories])).slice(0, 128),
    tools: mergedTools.slice(-500)
  };

  const nextProfiles = updateCostProfiles(policy, incidents, profiles);
  const nastyProfiles = buildNastyProfiles(policy, nextProfiles);
  const exemptionAudit = auditExemptions(policy, { strict: false });

  const result = {
    ok: true,
    type: 'redteam_adaptive_defense_run',
    ts: nowIso(),
    shadow_only: policy.shadow_only === true,
    incidents_sampled: incidents.length,
    tool_proposals_added: toolResult.proposals.length,
    new_categories: toolResult.new_categories,
    stage_counts: toolResult.stage_counts,
    runtime_counts: toolResult.runtime_counts,
    cost_profiles_last_uplift: Number(nextProfiles.last_uplift || 0),
    uplift_target: Number(policy.limits.minimum_uplift_target || 0.2),
    uplift_met: Number(nextProfiles.last_uplift || 0) >= Number(policy.limits.minimum_uplift_target || 0.2),
    exemption_posture: {
      ok: exemptionAudit.ok,
      expired_count: Number(exemptionAudit.expired_count || 0),
      out_of_scope_count: Number(exemptionAudit.out_of_scope_count || 0)
    },
    nasty_profiles: nastyProfiles,
    defensive_only: true,
    legal_bounded: true,
    reversible: true
  };

  saveCatalog(policy, nextCatalog);
  saveCostProfiles(policy, nextProfiles);
  writeJsonAtomic(policy.paths.latest_path, result);
  appendJsonl(policy.paths.history_path, result);

  return {
    ...result,
    paths: {
      latest_path: relPath(policy.paths.latest_path),
      history_path: relPath(policy.paths.history_path),
      tool_catalog_path: relPath(policy.paths.tool_catalog_path),
      cost_profiles_path: relPath(policy.paths.cost_profiles_path),
      exemptions_path: relPath(policy.paths.exemptions_path)
    }
  };
}

function statusAdaptiveDefense(policyPath?: string) {
  const policy = loadPolicy(policyPath || DEFAULT_POLICY_PATH);
  const latest = readJson(policy.paths.latest_path, null);
  const exemptions = loadExemptions(policy);
  const audit = auditExemptions(policy, { strict: false });
  return {
    ok: true,
    type: 'redteam_adaptive_defense_status',
    ts: nowIso(),
    enabled: policy.enabled === true,
    shadow_only: policy.shadow_only === true,
    defensive_only: policy.defensive_only === true,
    latest: latest || null,
    exemptions_active: Array.isArray(exemptions.exemptions) ? exemptions.exemptions.length : 0,
    exemption_posture: {
      ok: audit.ok,
      expired_count: Number(audit.expired_count || 0),
      out_of_scope_count: Number(audit.out_of_scope_count || 0)
    },
    policy_path: relPath(policyPath || DEFAULT_POLICY_PATH)
  };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/redteam/adaptive_defense_expansion.js run [--policy=/abs/path.json] [--state-root=/abs/path]');
  console.log('  node systems/redteam/adaptive_defense_expansion.js request-exemption --scope=<id> [--reason=text] [--duration-hours=24]');
  console.log('  node systems/redteam/adaptive_defense_expansion.js approve-exemption --id=<id> --approver=<id> --approval-note="note"');
  console.log('  node systems/redteam/adaptive_defense_expansion.js audit-exemptions [--strict=1]');
  console.log('  node systems/redteam/adaptive_defense_expansion.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  const policyPath = args.policy || process.env.REDTEAM_ADAPTIVE_DEFENSE_POLICY_PATH || DEFAULT_POLICY_PATH;

  if (cmd === 'help' || cmd === '--help' || cmd === '-h' || args.help) {
    usage();
    return;
  }

  if (cmd === 'run') {
    const out = runAdaptiveDefenseExpansion({}, { policyPath });
    process.stdout.write(`${JSON.stringify(out)}\n`);
    return;
  }

  if (cmd === 'request-exemption') {
    const policy = loadPolicy(policyPath);
    const out = requestExemption(policy, {
      scope: args.scope,
      reason: args.reason,
      duration_hours: args['duration-hours'] || args.duration_hours
    });
    process.stdout.write(`${JSON.stringify(out)}\n`);
    if (!out.ok) process.exitCode = 1;
    return;
  }

  if (cmd === 'approve-exemption') {
    const policy = loadPolicy(policyPath);
    const out = approveExemption(policy, {
      id: args.id,
      approver: args.approver,
      approval_note: args['approval-note'] || args.approval_note
    });
    process.stdout.write(`${JSON.stringify(out)}\n`);
    if (!out.ok) process.exitCode = 1;
    return;
  }

  if (cmd === 'audit-exemptions') {
    const policy = loadPolicy(policyPath);
    const out = auditExemptions(policy, { strict: toBool(args.strict, false) });
    process.stdout.write(`${JSON.stringify(out)}\n`);
    if (!out.ok && toBool(args.strict, false)) process.exitCode = 1;
    return;
  }

  if (cmd === 'status') {
    process.stdout.write(`${JSON.stringify(statusAdaptiveDefense(policyPath))}\n`);
    return;
  }

  usage();
  process.exitCode = 2;
}

if (require.main === module) {
  try {
    main();
  } catch (err) {
    process.stdout.write(`${JSON.stringify({
      ok: false,
      type: 'redteam_adaptive_defense',
      error: cleanText(err && (err as AnyObj).message ? (err as AnyObj).message : err || 'adaptive_defense_failed', 260)
    })}\n`);
    process.exit(1);
  }
}

module.exports = {
  loadPolicy,
  runAdaptiveDefenseExpansion,
  statusAdaptiveDefense,
  requestExemption,
  approveExemption,
  auditExemptions
};
