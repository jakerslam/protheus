'use strict';

const fs = require('fs');
const path = require('path');

const REPO_ROOT = path.resolve(__dirname, '..');
const DEFAULT_STRATEGY_DIR = path.join(REPO_ROOT, 'config', 'strategies');
const THRESHOLD_KEYS = new Set([
  'min_signal_quality',
  'min_sensory_signal_score',
  'min_sensory_relevance_score',
  'min_directive_fit',
  'min_actionability_score',
  'min_eye_score_ema',
  'min_composite_eligibility'
]);
const VALID_RISK_LEVELS = new Set(['low', 'medium', 'high']);
const ALLOWED_TOP_KEYS = new Set([
  'version',
  'id',
  'name',
  'status',
  'tags',
  'objective',
  'risk_policy',
  'allowed_risks',
  'admission_policy',
  'ranking_weights',
  'budget_policy',
  'exploration_policy',
  'stop_policy',
  'promotion_policy',
  'execution_policy',
  'threshold_overrides'
]);

function asString(v) {
  return String(v == null ? '' : v).trim();
}

function asStringArray(v) {
  if (!Array.isArray(v)) return [];
  const out = [];
  for (const item of v) {
    const s = asString(item);
    if (s) out.push(s);
  }
  return Array.from(new Set(out));
}

function readJsonSafe(filePath, fallback) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function normalizeThresholdOverrides(raw) {
  const out = {};
  const src = raw && typeof raw === 'object' ? raw : {};
  for (const [key, value] of Object.entries(src)) {
    if (!THRESHOLD_KEYS.has(key)) continue;
    const n = Number(value);
    if (!Number.isFinite(n)) continue;
    out[key] = n;
  }
  return out;
}

function normalizeRiskPolicy(rawRisk, rawAllowed, warnings) {
  const riskSrc = rawRisk && typeof rawRisk === 'object' ? rawRisk : {};
  const fromRisk = rawRisk && typeof rawRisk === 'object'
    ? asStringArray(rawRisk.allowed_risks).map(x => x.toLowerCase())
    : [];
  const fromRoot = asStringArray(rawAllowed).map(x => x.toLowerCase());
  const combined = Array.from(new Set([...fromRisk, ...fromRoot]));
  const invalid = combined.filter(x => !VALID_RISK_LEVELS.has(x));
  const allowed = combined.filter(x => VALID_RISK_LEVELS.has(x));
  if (invalid.length && Array.isArray(warnings)) {
    for (const item of invalid) warnings.push(`risk_policy_invalid_risk_filtered:${item}`);
  }
  const maxPerAction = Number(riskSrc.max_risk_per_action);
  const max_risk_per_action = Number.isFinite(maxPerAction)
    ? Math.max(0, Math.min(100, Math.round(maxPerAction)))
    : null;
  return { allowed_risks: allowed, max_risk_per_action, invalid_risks: invalid };
}

function normalizeStatus(raw) {
  const s = asString(raw).toLowerCase();
  if (s === 'disabled' || s === 'off' || s === 'paused') return 'disabled';
  return 'active';
}

function clampNumber(v, lo, hi, fallback) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return n;
}

function normalizeInteger(v, lo, hi, fallback, allowNull = false) {
  if (allowNull && (v == null || String(v).trim() === '')) return null;
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  const x = Math.round(n);
  if (x < lo) return lo;
  if (x > hi) return hi;
  return x;
}

function normalizeAdmissionPolicy(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  return {
    allowed_types: asStringArray(src.allowed_types).map(x => x.toLowerCase()),
    blocked_types: asStringArray(src.blocked_types).map(x => x.toLowerCase()),
    max_remediation_depth: normalizeInteger(src.max_remediation_depth, 0, 12, null, true),
    duplicate_window_hours: normalizeInteger(src.duplicate_window_hours, 1, 168, 24)
  };
}

function normalizeRankingWeights(raw, errors) {
  const defaults = {
    composite: 0.35,
    actionability: 0.2,
    directive_fit: 0.15,
    signal_quality: 0.15,
    expected_value: 0.1,
    time_to_value: 0,
    risk_penalty: 0.05
  };
  const src = raw && typeof raw === 'object' ? raw : {};
  const merged = { ...defaults };
  for (const [key, val] of Object.entries(src)) {
    if (!(key in defaults)) continue;
    const n = Number(val);
    if (!Number.isFinite(n) || n < 0) continue;
    merged[key] = n;
  }
  const total = Object.values(merged).reduce((a, b) => a + Number(b || 0), 0);
  if (total <= 0) {
    errors.push('ranking_weights_sum_zero');
    return defaults;
  }
  const normalized = {};
  for (const [k, v] of Object.entries(merged)) {
    normalized[k] = Number((Number(v) / total).toFixed(6));
  }
  return normalized;
}

function normalizeBudgetPolicy(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  const caps = src.per_capability_caps && typeof src.per_capability_caps === 'object'
    ? src.per_capability_caps
    : {};
  const perCaps = {};
  for (const [k, v] of Object.entries(caps)) {
    const key = asString(k).toLowerCase();
    if (!key) continue;
    const n = Number(v);
    if (!Number.isFinite(n) || n < 0) continue;
    perCaps[key] = Math.round(n);
  }
  return {
    daily_runs_cap: normalizeInteger(src.daily_runs_cap, 1, 500, null, true),
    daily_token_cap: normalizeInteger(src.daily_token_cap, 100, 1000000, null, true),
    max_tokens_per_action: normalizeInteger(src.max_tokens_per_action, 50, 1000000, null, true),
    per_capability_caps: perCaps
  };
}

function normalizeExplorationPolicy(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  return {
    fraction: Number(clampNumber(src.fraction, 0.05, 0.8, 0.25).toFixed(3)),
    every_n: normalizeInteger(src.every_n, 1, 20, 3),
    min_eligible: normalizeInteger(src.min_eligible, 2, 20, 3)
  };
}

function normalizeStopPolicy(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  const cb = src.circuit_breakers && typeof src.circuit_breakers === 'object' ? src.circuit_breakers : {};
  const rc = src.recursion && typeof src.recursion === 'object' ? src.recursion : {};
  return {
    circuit_breakers: {
      http_429_cooldown_hours: normalizeInteger(cb.http_429_cooldown_hours, 1, 168, 12),
      http_5xx_cooldown_hours: normalizeInteger(cb.http_5xx_cooldown_hours, 1, 168, 6),
      dns_error_cooldown_hours: normalizeInteger(cb.dns_error_cooldown_hours, 1, 168, 6)
    },
    recursion: {
      max_consecutive_remediation: normalizeInteger(rc.max_consecutive_remediation, 0, 12, 2),
      max_duplicate_proposals_24h: normalizeInteger(rc.max_duplicate_proposals_24h, 1, 200, 3)
    }
  };
}

function normalizePromotionPolicy(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  return {
    min_days: normalizeInteger(src.min_days, 1, 90, 7),
    min_attempted: normalizeInteger(src.min_attempted, 0, 10000, 12),
    min_verified_rate: Number(clampNumber(src.min_verified_rate, 0, 1, 0.5).toFixed(3)),
    max_reverted_rate: Number(clampNumber(src.max_reverted_rate, 0, 1, 0.35).toFixed(3)),
    max_stop_ratio: Number(clampNumber(src.max_stop_ratio, 0, 1, 0.75).toFixed(3)),
    min_shipped: normalizeInteger(src.min_shipped, 0, 10000, 1)
  };
}

function normalizeExecutionPolicy(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  const modeRaw = asString(src.mode).toLowerCase();
  const mode = modeRaw === 'execute' ? 'execute' : 'score_only';
  return { mode };
}

function normalizeObjective(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  return {
    primary: asString(src.primary),
    secondary: asStringArray(src.secondary),
    fitness_metric: asString(src.fitness_metric) || 'verified_progress_rate',
    target_window_days: normalizeInteger(src.target_window_days, 1, 90, 14)
  };
}

function pushValidationChecks(normalized, warnings, errors) {
  if (!normalized || typeof normalized !== 'object') return;

  const allowedTypes = Array.isArray(normalized.admission_policy && normalized.admission_policy.allowed_types)
    ? normalized.admission_policy.allowed_types
    : [];
  const blockedTypes = Array.isArray(normalized.admission_policy && normalized.admission_policy.blocked_types)
    ? normalized.admission_policy.blocked_types
    : [];
  const blockedSet = new Set(blockedTypes);
  for (const t of allowedTypes) {
    if (blockedSet.has(t)) errors.push(`admission_policy_type_conflict:${t}`);
  }

  const duplicateWindow = Number(normalized.admission_policy && normalized.admission_policy.duplicate_window_hours);
  if (Number.isFinite(duplicateWindow) && duplicateWindow < 1) {
    errors.push('admission_policy_duplicate_window_invalid');
  }

  const maxRisk = Number(normalized.risk_policy && normalized.risk_policy.max_risk_per_action);
  if (Number.isFinite(maxRisk) && maxRisk < 15) {
    warnings.push('risk_policy_max_risk_per_action_very_low');
  }
  if (
    Number.isFinite(maxRisk)
    && Array.isArray(normalized.risk_policy && normalized.risk_policy.allowed_risks)
    && normalized.risk_policy.allowed_risks.includes('high')
    && maxRisk < 70
  ) {
    warnings.push('risk_policy_high_allowed_but_max_risk_low');
  }

  const promo = normalized.promotion_policy && typeof normalized.promotion_policy === 'object'
    ? normalized.promotion_policy
    : {};
  if (Number(promo.min_shipped || 0) > Number(promo.min_attempted || 0)) {
    errors.push('promotion_policy_min_shipped_gt_min_attempted');
  }
}

function collectSchemaWarnings(src, warnings) {
  for (const key of Object.keys(src || {})) {
    if (!ALLOWED_TOP_KEYS.has(key)) warnings.push(`unknown_top_level_key:${key}`);
  }
}

function normalizeStrategy(raw, filePath) {
  const fileName = path.basename(filePath, path.extname(filePath));
  const src = raw && typeof raw === 'object' ? raw : {};
  const warnings = [];
  const errors = [];
  collectSchemaWarnings(src, warnings);
  const id = asString(src.id) || fileName;
  const name = asString(src.name) || id;
  const status = normalizeStatus(src.status);
  const objective = normalizeObjective(src.objective);
  const tags = asStringArray(src.tags).map(x => x.toLowerCase());
  const risk_policy = normalizeRiskPolicy(src.risk_policy, src.allowed_risks, warnings);
  const admission_policy = normalizeAdmissionPolicy(src.admission_policy);
  const ranking_weights = normalizeRankingWeights(src.ranking_weights, errors);
  const budget_policy = normalizeBudgetPolicy(src.budget_policy);
  const exploration_policy = normalizeExplorationPolicy(src.exploration_policy);
  const stop_policy = normalizeStopPolicy(src.stop_policy);
  const promotion_policy = normalizePromotionPolicy(src.promotion_policy);
  const execution_policy = normalizeExecutionPolicy(src.execution_policy);
  const threshold_overrides = normalizeThresholdOverrides(src.threshold_overrides);
  if (!objective.primary) warnings.push('objective_primary_missing');
  if (!risk_policy.allowed_risks.length) errors.push('risk_policy_allowed_risks_empty');
  const normalized = {
    id,
    name,
    status,
    file: filePath,
    version: asString(src.version) || '1.0',
    objective,
    tags,
    risk_policy,
    admission_policy,
    ranking_weights,
    budget_policy,
    exploration_policy,
    stop_policy,
    promotion_policy,
    execution_policy,
    threshold_overrides
  };
  pushValidationChecks(normalized, warnings, errors);
  return {
    ...normalized,
    validation: {
      strict_ok: errors.length === 0,
      errors,
      warnings
    }
  };
}

function listStrategies(options = {}) {
  const strategyDir = path.resolve(String(options.dir || process.env.AUTONOMY_STRATEGY_DIR || DEFAULT_STRATEGY_DIR));
  if (!fs.existsSync(strategyDir)) return [];
  const files = fs.readdirSync(strategyDir)
    .filter(f => f.endsWith('.json'))
    .sort();
  const out = [];
  for (const f of files) {
    const fp = path.join(strategyDir, f);
    const raw = readJsonSafe(fp, null);
    if (!raw || typeof raw !== 'object') continue;
    out.push(normalizeStrategy(raw, fp));
  }
  return out.sort((a, b) => a.id.localeCompare(b.id));
}

function loadActiveStrategy(options = {}) {
  const allowMissing = options.allowMissing === true;
  const strict = options.strict === true || String(process.env.AUTONOMY_STRATEGY_STRICT || '') === '1';
  const requestedId = asString(options.id || process.env.AUTONOMY_STRATEGY_ID);
  const strategies = listStrategies(options);
  if (!strategies.length) {
    if (allowMissing) return null;
    throw new Error('no strategy profiles found');
  }

  if (requestedId) {
    const hit = strategies.find(s => s.id === requestedId);
    if (!hit) {
      if (allowMissing) return null;
      throw new Error(`strategy not found: ${requestedId}`);
    }
    if (strict && hit.validation && hit.validation.strict_ok === false) {
      throw new Error(`strategy_invalid:${requestedId}:${(hit.validation.errors || []).join(',')}`);
    }
    return hit;
  }

  const active = strategies.filter(s => s.status === 'active');
  if (active.length) {
    const pick = active[0];
    if (strict && pick.validation && pick.validation.strict_ok === false) {
      throw new Error(`strategy_invalid:${pick.id}:${(pick.validation.errors || []).join(',')}`);
    }
    return pick;
  }
  if (allowMissing) return null;
  throw new Error('no active strategy profile');
}

function effectiveAllowedRisks(defaultSet, strategy) {
  const defaults = defaultSet instanceof Set
    ? Array.from(defaultSet).map(x => asString(x).toLowerCase()).filter(Boolean)
    : [];
  const fromStrategy = strategy
    && strategy.risk_policy
    && Array.isArray(strategy.risk_policy.allowed_risks)
      ? strategy.risk_policy.allowed_risks.map(x => asString(x).toLowerCase()).filter(Boolean)
      : [];
  const chosen = fromStrategy.length ? fromStrategy : defaults;
  return new Set(chosen);
}

function applyThresholdOverrides(baseThresholds, strategy) {
  const base = baseThresholds && typeof baseThresholds === 'object' ? { ...baseThresholds } : {};
  const overrides = strategy && strategy.threshold_overrides && typeof strategy.threshold_overrides === 'object'
    ? strategy.threshold_overrides
    : {};
  for (const [key, value] of Object.entries(overrides)) {
    if (!THRESHOLD_KEYS.has(key)) continue;
    const n = Number(value);
    if (!Number.isFinite(n)) continue;
    base[key] = n;
  }
  return base;
}

function strategyExecutionMode(strategy, fallback = 'execute') {
  const mode = strategy
    && strategy.execution_policy
    && asString(strategy.execution_policy.mode).toLowerCase() === 'score_only'
    ? 'score_only'
    : (strategy && strategy.execution_policy && asString(strategy.execution_policy.mode).toLowerCase() === 'execute'
      ? 'execute'
      : fallback);
  return mode === 'score_only' ? 'score_only' : 'execute';
}

function strategyBudgetCaps(strategy, defaults = {}) {
  const defaultRuns = Number(defaults.daily_runs_cap);
  const defaultTokens = Number(defaults.daily_token_cap);
  const defaultPerAction = Number(defaults.max_tokens_per_action);
  const runs = strategy && strategy.budget_policy && Number.isFinite(Number(strategy.budget_policy.daily_runs_cap))
    ? Number(strategy.budget_policy.daily_runs_cap)
    : (Number.isFinite(defaultRuns) ? defaultRuns : null);
  const tokens = strategy && strategy.budget_policy && Number.isFinite(Number(strategy.budget_policy.daily_token_cap))
    ? Number(strategy.budget_policy.daily_token_cap)
    : (Number.isFinite(defaultTokens) ? defaultTokens : null);
  const perAction = strategy
    && strategy.budget_policy
    && strategy.budget_policy.max_tokens_per_action != null
    && String(strategy.budget_policy.max_tokens_per_action).trim() !== ''
    && Number.isFinite(Number(strategy.budget_policy.max_tokens_per_action))
    ? Number(strategy.budget_policy.max_tokens_per_action)
    : (Number.isFinite(defaultPerAction) ? defaultPerAction : null);
  return {
    daily_runs_cap: runs,
    daily_token_cap: tokens,
    max_tokens_per_action: perAction,
    per_capability_caps: strategy
      && strategy.budget_policy
      && strategy.budget_policy.per_capability_caps
      && typeof strategy.budget_policy.per_capability_caps === 'object'
      ? { ...strategy.budget_policy.per_capability_caps }
      : {}
  };
}

function strategyExplorationPolicy(strategy, defaults = {}) {
  const base = {
    fraction: Number.isFinite(Number(defaults.fraction)) ? Number(defaults.fraction) : 0.25,
    every_n: Number.isFinite(Number(defaults.every_n)) ? Number(defaults.every_n) : 3,
    min_eligible: Number.isFinite(Number(defaults.min_eligible)) ? Number(defaults.min_eligible) : 3
  };
  if (!strategy || !strategy.exploration_policy) return base;
  return {
    fraction: Number(strategy.exploration_policy.fraction),
    every_n: Number(strategy.exploration_policy.every_n),
    min_eligible: Number(strategy.exploration_policy.min_eligible)
  };
}

function strategyRankingWeights(strategy) {
  if (!strategy || !strategy.ranking_weights || typeof strategy.ranking_weights !== 'object') {
    return normalizeRankingWeights({}, []);
  }
  return strategy.ranking_weights;
}

function strategyAllowsProposalType(strategy, proposalType) {
  if (!strategy || !strategy.admission_policy) return true;
  const type = asString(proposalType).toLowerCase();
  const allowed = Array.isArray(strategy.admission_policy.allowed_types)
    ? strategy.admission_policy.allowed_types
    : [];
  const blocked = Array.isArray(strategy.admission_policy.blocked_types)
    ? strategy.admission_policy.blocked_types
    : [];
  if (!type) return allowed.length === 0;
  if (blocked.includes(type)) return false;
  if (allowed.length === 0) return true;
  return allowed.includes(type);
}

function strategyPromotionPolicy(strategy, defaults = {}) {
  const base = normalizePromotionPolicy(defaults);
  if (!strategy || !strategy.promotion_policy || typeof strategy.promotion_policy !== 'object') return base;
  return normalizePromotionPolicy({ ...base, ...strategy.promotion_policy });
}

function strategyMaxRiskPerAction(strategy, fallback = null) {
  const raw = strategy && strategy.risk_policy ? strategy.risk_policy.max_risk_per_action : null;
  if (raw != null && String(raw).trim() !== '') {
    const v = Number(raw);
    if (Number.isFinite(v)) return Math.max(0, Math.min(100, Math.round(v)));
  }
  const fv = Number(fallback);
  if (Number.isFinite(fv)) return Math.max(0, Math.min(100, Math.round(fv)));
  return null;
}

function strategyDuplicateWindowHours(strategy, fallback = 24) {
  const v = strategy
    && strategy.admission_policy
    ? Number(strategy.admission_policy.duplicate_window_hours)
    : NaN;
  if (Number.isFinite(v)) return Math.max(1, Math.min(168, Math.round(v)));
  const fv = Number(fallback);
  if (Number.isFinite(fv)) return Math.max(1, Math.min(168, Math.round(fv)));
  return 24;
}

module.exports = {
  DEFAULT_STRATEGY_DIR,
  THRESHOLD_KEYS,
  listStrategies,
  loadActiveStrategy,
  effectiveAllowedRisks,
  applyThresholdOverrides,
  strategyExecutionMode,
  strategyBudgetCaps,
  strategyExplorationPolicy,
  strategyRankingWeights,
  strategyAllowsProposalType,
  strategyPromotionPolicy,
  strategyMaxRiskPerAction,
  strategyDuplicateWindowHours
};
