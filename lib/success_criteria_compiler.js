// @ts-check
'use strict';

const fs = require('fs');
const path = require('path');

const REPO_ROOT = path.resolve(__dirname, '..');
const SUCCESS_CRITERIA_CAPABILITY_MAP_PATH = process.env.SUCCESS_CRITERIA_CAPABILITY_MAP_PATH
  ? path.resolve(process.env.SUCCESS_CRITERIA_CAPABILITY_MAP_PATH)
  : path.join(REPO_ROOT, 'config', 'success_criteria_capability_map.json');

/**
 * @typedef {{
 *   source: string,
 *   metric: string,
 *   target: string,
 *   horizon: string,
 *   measurable: boolean
 * }} CompiledSuccessCriteria
 */

const ALL_KNOWN_METRICS = new Set([
  'execution_success',
  'postconditions_ok',
  'queue_outcome_logged',
  'artifact_count',
  'entries_count',
  'revenue_actions_count',
  'token_usage',
  'duration_ms',
  'outreach_artifact',
  'reply_or_interview_count'
]);

const DEFAULT_METRIC_ALIASES = {
  validation_metric: 'postconditions_ok',
  validation_check: 'postconditions_ok',
  verification_metric: 'postconditions_ok',
  verification_check: 'postconditions_ok',
  collector_failure_streak: 'queue_outcome_logged',
  collector_success_runs: 'artifact_count',
  hypothesis_signal_lift: 'artifact_count',
  experiment_artifact: 'artifact_count',
  offer_draft_count: 'outreach_artifact',
  proposal_draft_count: 'outreach_artifact',
  outreach_artifact_count: 'outreach_artifact',
  reply_count: 'reply_or_interview_count',
  interview_count: 'reply_or_interview_count',
  outreach_reply_count: 'reply_or_interview_count',
  outreach_interview_count: 'reply_or_interview_count'
};

const DEFAULT_CAPABILITY_REWRITE = {
  proposal_generic: {
    outreach_artifact: 'artifact_count',
    reply_or_interview_count: 'artifact_count'
  },
  proposal_outreach: {},
  actuation: {},
  generic: {}
};

let CAPABILITY_MAP_CACHE = undefined;

function normalizeText(v) {
  return String(v == null ? '' : v).trim();
}

function normalizeSpaces(v) {
  return normalizeText(v).replace(/\s+/g, ' ');
}

function normalizeMetricName(v) {
  return normalizeSpaces(v).toLowerCase().replace(/[\s-]+/g, '_');
}

function normalizeCapabilityFamily(capabilityKey) {
  const key = normalizeSpaces(capabilityKey).toLowerCase();
  if (!key) return 'generic';
  if (key.startsWith('actuation:')) return 'actuation';
  if (key.startsWith('proposal:')) {
    if (/\b(opportunity|outreach|lead|sales|bizdev|revenue|freelance|contract|gig)\b/.test(key)) {
      return 'proposal_outreach';
    }
    return 'proposal_generic';
  }
  return 'generic';
}

function readJsonSafe(filePath, fallback) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function normalizeCapabilityMetricMap(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  const aliasesRaw = src.metric_aliases && typeof src.metric_aliases === 'object'
    ? src.metric_aliases
    : {};
  const aliases = { ...DEFAULT_METRIC_ALIASES };
  for (const [k, v] of Object.entries(aliasesRaw)) {
    const key = normalizeMetricName(k);
    const target = normalizeMetricName(v);
    if (!key || !target || !ALL_KNOWN_METRICS.has(target)) continue;
    aliases[key] = target;
  }

  const rewriteRaw = src.capability_rewrite && typeof src.capability_rewrite === 'object'
    ? src.capability_rewrite
    : {};
  const rewrite = {};
  for (const family of Object.keys(DEFAULT_CAPABILITY_REWRITE)) {
    const familySrc = rewriteRaw[family] && typeof rewriteRaw[family] === 'object'
      ? rewriteRaw[family]
      : {};
    const merged = { ...DEFAULT_CAPABILITY_REWRITE[family] };
    for (const [k, v] of Object.entries(familySrc)) {
      const key = normalizeMetricName(k);
      const target = normalizeMetricName(v);
      if (!key || !target || !ALL_KNOWN_METRICS.has(target)) continue;
      merged[key] = target;
    }
    rewrite[family] = merged;
  }

  return {
    version: normalizeText(src.version) || '1.0',
    metric_aliases: aliases,
    capability_rewrite: rewrite
  };
}

function loadCapabilityMetricMap() {
  if (CAPABILITY_MAP_CACHE !== undefined) return CAPABILITY_MAP_CACHE;
  CAPABILITY_MAP_CACHE = normalizeCapabilityMetricMap(readJsonSafe(SUCCESS_CRITERIA_CAPABILITY_MAP_PATH, null));
  return CAPABILITY_MAP_CACHE;
}

function rewriteMetricForCapability(metric, capabilityKey, capabilityMap) {
  const metricNorm = normalizeMetricName(metric);
  if (!metricNorm) return 'execution_success';
  const map = capabilityMap && typeof capabilityMap === 'object'
    ? capabilityMap
    : loadCapabilityMetricMap();
  const aliases = map.metric_aliases && typeof map.metric_aliases === 'object'
    ? map.metric_aliases
    : DEFAULT_METRIC_ALIASES;
  const family = normalizeCapabilityFamily(capabilityKey);
  const familyRewrite = map.capability_rewrite
    && typeof map.capability_rewrite === 'object'
    && map.capability_rewrite[family]
    && typeof map.capability_rewrite[family] === 'object'
    ? map.capability_rewrite[family]
    : {};
  let out = aliases[metricNorm] || metricNorm;
  if (familyRewrite[out]) out = normalizeMetricName(familyRewrite[out]);
  if (!ALL_KNOWN_METRICS.has(out)) return 'execution_success';
  return out;
}

function applyCapabilityMetricMap(rows, capabilityKey) {
  const input = Array.isArray(rows) ? rows : [];
  if (!input.length) return [];
  const map = loadCapabilityMetricMap();
  const out = [];
  const seen = new Set();
  for (const row of input) {
    const metric = rewriteMetricForCapability(row && row.metric, capabilityKey, map);
    const source = normalizeText(row && row.source) || 'success_criteria';
    const target = normalizeSpaces(row && row.target || '') || 'execution success';
    const horizon = normalizeSpaces(row && row.horizon || '');
    const key = `${metric}|${target}|${horizon}|${source}`.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push({
      source,
      metric,
      target,
      horizon,
      measurable: row && row.measurable === true
    });
  }
  return out;
}

function clampNumber(v, lo, hi, fallback) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  if (n < lo) return lo;
  if (n > hi) return hi;
  return n;
}

function parseFirstInt(text, fallback) {
  const m = String(text || '').match(/\b(\d+)\b/);
  if (!m) return fallback;
  const n = Number(m[1]);
  return Number.isFinite(n) ? n : fallback;
}

function parseComparator(text, fallback) {
  const t = String(text || '').toLowerCase();
  if (/(?:<=|≤|\bat most\b|\bwithin\b|\bunder\b|\bbelow\b|\bmax(?:imum)?\b|\bless than\b)/.test(t)) return 'lte';
  if (/(?:>=|≥|\bat least\b|\bover\b|\babove\b|\bminimum\b|\bmin\b|\bmore than\b)/.test(t)) return 'gte';
  return fallback;
}

function parseDurationLimitMs(text) {
  const t = String(text || '').toLowerCase();
  const m = t.match(/(\d+(?:\.\d+)?)\s*(ms|msec|millisecond(?:s)?|s|sec|secs|second(?:s)?|m|min|mins|minute(?:s)?)/);
  if (!m) return null;
  let value = Number(m[1]);
  if (!Number.isFinite(value)) return null;
  const unit = String(m[2] || '');
  if (unit === 'm' || unit === 'min' || unit === 'mins' || unit.startsWith('minute')) value *= 60 * 1000;
  else if (unit === 's' || unit === 'sec' || unit === 'secs' || unit.startsWith('second')) value *= 1000;
  return Math.round(value);
}

function parseTokenLimit(text) {
  const t = String(text || '').toLowerCase();
  const mA = t.match(/(\d+(?:\.\d+)?)\s*(k|m)?\s*tokens?/);
  const mB = t.match(/tokens?\s*(?:<=|≥|>=|≤|<|>|=|at most|at least|under|over|below|above|within|max(?:imum)?|min(?:imum)?)?\s*(\d+(?:\.\d+)?)(?:\s*(k|m))?/);
  const m = mA || mB;
  if (!m) return null;
  let value = Number(m[1]);
  if (!Number.isFinite(value)) return null;
  const suffix = String(m[2] || '').toLowerCase();
  if (suffix === 'k') value *= 1000;
  else if (suffix === 'm') value *= 1000000;
  return Math.round(value);
}

function parseHorizon(text) {
  const t = String(text || '').toLowerCase();
  const m = t.match(/\b(\d+\s*(?:h|hr|hour|hours|d|day|days|w|week|weeks|min|mins|minute|minutes|run|runs))\b/);
  if (m) return normalizeSpaces(m[1]);
  if (/\bnext\s+run\b/.test(t)) return 'next run';
  if (/\bnext\s+2\s+runs?\b/.test(t)) return '2 runs';
  if (/\b24h\b/.test(t)) return '24h';
  if (/\b48h\b/.test(t)) return '48h';
  if (/\b7d\b/.test(t)) return '7d';
  return '';
}

function normalizeTarget(metric, targetText, horizonText) {
  const text = normalizeSpaces(`${targetText} ${horizonText}`.toLowerCase());
  if (metric === 'execution_success') return 'execution success';
  if (metric === 'postconditions_ok') return 'postconditions pass';
  if (metric === 'queue_outcome_logged') return 'outcome receipt logged';
  if (metric === 'artifact_count') {
    const comparator = parseComparator(text, 'gte');
    const threshold = parseFirstInt(text, 1);
    return `${comparator === 'lte' ? '<=' : '>='}${threshold} artifact`;
  }
  if (metric === 'outreach_artifact') {
    const comparator = parseComparator(text, 'gte');
    const threshold = parseFirstInt(text, 1);
    return `${comparator === 'lte' ? '<=' : '>='}${threshold} outreach artifact`;
  }
  if (metric === 'reply_or_interview_count') {
    const comparator = parseComparator(text, 'gte');
    const threshold = parseFirstInt(text, 1);
    return `${comparator === 'lte' ? '<=' : '>='}${threshold} reply/interview signal`;
  }
  if (metric === 'entries_count') {
    const comparator = parseComparator(text, 'gte');
    const threshold = parseFirstInt(text, 1);
    return `${comparator === 'lte' ? '<=' : '>='}${threshold} entries`;
  }
  if (metric === 'revenue_actions_count') {
    const comparator = parseComparator(text, 'gte');
    const threshold = parseFirstInt(text, 1);
    return `${comparator === 'lte' ? '<=' : '>='}${threshold} revenue actions`;
  }
  if (metric === 'token_usage') {
    const comparator = parseComparator(text, 'lte');
    const limit = parseTokenLimit(text) != null ? parseTokenLimit(text) : 1200;
    return `tokens ${comparator === 'gte' ? '>=' : '<='}${limit}`;
  }
  if (metric === 'duration_ms') {
    const comparator = parseComparator(text, 'lte');
    const limitMs = parseDurationLimitMs(text) != null ? parseDurationLimitMs(text) : 15000;
    return `duration ${comparator === 'gte' ? '>=' : '<='}${limitMs}ms`;
  }
  return normalizeSpaces(targetText || 'execution success') || 'execution success';
}

function classifyMetric(metricText, targetText, sourceText) {
  const metric = normalizeSpaces(metricText).toLowerCase();
  const text = normalizeSpaces(`${metricText} ${targetText} ${sourceText}`).toLowerCase();

  if (!metric && /\b(reply|interview)\b/.test(text)) return 'reply_or_interview_count';
  if (!metric && /\boutreach\b/.test(text) && /\b(artifact|draft|offer|proposal)\b/.test(text)) return 'outreach_artifact';

  if (metric === 'validation_metric' || metric === 'validation_check' || metric === 'verification_metric' || metric === 'verification_check') return 'postconditions_ok';
  if (metric === 'outreach_artifact') return 'outreach_artifact';
  if (metric === 'reply_or_interview_count' || metric === 'reply_count' || metric === 'interview_count' || metric === 'outreach_reply_count' || metric === 'outreach_interview_count') return 'reply_or_interview_count';
  if (metric === 'artifact_count' || metric === 'experiment_artifact' || metric === 'collector_success_runs' || metric === 'hypothesis_signal_lift' || metric === 'outreach_artifact_count' || metric === 'offer_draft_count' || metric === 'proposal_draft_count') return 'artifact_count';
  if (metric === 'verification_checks_passed' || metric === 'postconditions_ok') return 'postconditions_ok';
  if (metric === 'collector_failure_streak' || metric === 'queue_outcome_logged') return 'queue_outcome_logged';
  if (metric === 'entries_count') return 'entries_count';
  if (metric === 'revenue_actions_count') return 'revenue_actions_count';
  if (metric === 'token_usage') return 'token_usage';
  if (metric === 'duration_ms') return 'duration_ms';
  if (metric === 'execution_success') return 'execution_success';

  if (/\b(reply|interview)\b/.test(text)) return 'reply_or_interview_count';
  if (/\boutreach\b/.test(text) && /\b(artifact|draft|offer|proposal)\b/.test(text)) return 'outreach_artifact';
  if (/\b(artifact|draft|experiment|patch|plan|deliverable)\b/.test(text)) return 'artifact_count';
  if (/\b(postcondition|contract|verify|verification|check(?:s)? pass)\b/.test(text)) return 'postconditions_ok';
  if (/\b(receipt|evidence|queue[\s_-]?outcome|logged?)\b/.test(text)) return 'queue_outcome_logged';
  if (/\brevenue\b/.test(text)) return 'revenue_actions_count';
  if (/\b(entries|entry|notes?)\b/.test(text)) return 'entries_count';
  if (/\btoken(?:s)?\b/.test(text)) return 'token_usage';
  if (/\b(latency|duration|time|ms|msec|millisecond|second|sec|min|minute)\b/.test(text)) return 'duration_ms';
  if (/\b(execut(e|ed|ion)|run|runnable|success)\b/.test(text)) return 'execution_success';
  return 'execution_success';
}

function normalizeInputRows(rows, source) {
  const out = [];
  const src = normalizeText(source) || 'success_criteria';
  for (const row of Array.isArray(rows) ? rows : []) {
    if (typeof row === 'string') {
      const target = normalizeSpaces(row);
      if (!target) continue;
      out.push({ source: src, metric: '', target, horizon: '' });
      continue;
    }
    if (!row || typeof row !== 'object') continue;
    const metric = normalizeSpaces(row.metric || row.name || '');
    const target = normalizeSpaces(row.target || row.threshold || row.description || row.goal || '');
    const horizon = normalizeSpaces(row.horizon || row.window || row.by || '');
    if (!metric && !target && !horizon) continue;
    out.push({ source: src, metric, target, horizon });
  }
  return out;
}

/**
 * @param {unknown} rows
 * @param {{ source?: string }} [opts]
 * @returns {CompiledSuccessCriteria[]}
 */
function compileSuccessCriteriaRows(rows, opts = {}) {
  const rawRows = normalizeInputRows(rows, opts.source || 'success_criteria');
  const out = [];
  const seen = new Set();
  for (const row of rawRows) {
    const metric = classifyMetric(row.metric, row.target, row.source);
    const horizon = row.horizon || parseHorizon(row.target);
    const target = normalizeTarget(metric, row.target, horizon);
    const key = `${metric}|${target}|${horizon}|${row.source}`.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push({
      source: row.source,
      metric,
      target,
      horizon,
      measurable: true
    });
  }
  return out;
}

/**
 * @param {Record<string, any>} proposal
 * @param {{ include_verify?: boolean, include_validation?: boolean, allow_fallback?: boolean, capability_key?: string }} [opts]
 * @returns {CompiledSuccessCriteria[]}
 */
function compileProposalSuccessCriteria(proposal, opts = {}) {
  const p = proposal && typeof proposal === 'object' ? proposal : {};
  const actionSpec = p.action_spec && typeof p.action_spec === 'object' ? p.action_spec : {};
  const includeVerify = opts.include_verify !== false;
  const includeValidation = opts.include_validation !== false;

  const compiled = [];
  // Backward compatibility: some proposals persist success_criteria at top level.
  compiled.push(...compileSuccessCriteriaRows(p.success_criteria, { source: 'success_criteria' }));
  compiled.push(...compileSuccessCriteriaRows(actionSpec.success_criteria, { source: 'action_spec.success_criteria' }));
  if (includeVerify) compiled.push(...compileSuccessCriteriaRows(actionSpec.verify, { source: 'action_spec.verify' }));
  if (includeValidation) compiled.push(...compileSuccessCriteriaRows(p.validation, { source: 'validation' }));

  if (!compiled.length && opts.allow_fallback !== false) {
    compiled.push({
      source: 'compiler_fallback',
      metric: 'execution_success',
      target: 'execution success',
      horizon: '',
      measurable: true
    });
  }
  return applyCapabilityMetricMap(compiled, opts.capability_key || '');
}

/**
 * @param {CompiledSuccessCriteria[]} compiledRows
 * @returns {{ metric: string, target: string, horizon: string }[]}
 */
function toActionSpecRows(compiledRows) {
  const rows = Array.isArray(compiledRows) ? compiledRows : [];
  return rows.map((row) => ({
    metric: String(row.metric || 'execution_success'),
    target: String(row.target || 'execution success'),
    horizon: normalizeSpaces(row.horizon || '')
  }));
}

module.exports = {
  compileSuccessCriteriaRows,
  compileProposalSuccessCriteria,
  toActionSpecRows,
  normalizeCapabilityFamily,
  rewriteMetricForCapability
};
