#!/usr/bin/env node
'use strict';

const DEFAULT_POLICY = Object.freeze({
  index_first_required: true,
  max_burn_slo_tokens: 200,
  max_recall_top: 50,
  max_max_files: 20,
  max_expand_lines: 300,
  bootstrap_hydration_token_cap: 48,
  block_stale_override: true
});

const NON_EXECUTING_COMMANDS = new Set(['status', 'verify', 'health', 'help']);
const INDEX_BYPASS_FLAGS = ['bypass', 'bypass-index', 'allow-direct-file', 'allow_full_scan', 'allow-full-scan'];
const DIRECT_READ_FLAGS = ['file', 'path', 'full-file', 'full_file', 'direct-file', 'direct_file'];
const STALE_OVERRIDE_FLAGS = ['allow-stale', 'allow_stale', 'stale-ok', 'stale_ok'];

function parseCliArgs(args = []) {
  const positional = [];
  const flags = {};
  for (const raw of Array.isArray(args) ? args : []) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token.startsWith('--')) {
      const body = token.slice(2);
      const eq = body.indexOf('=');
      if (eq >= 0) {
        const key = body.slice(0, eq).trim();
        const value = body.slice(eq + 1).trim();
        flags[key] = value;
      } else {
        flags[body] = '1';
      }
      continue;
    }
    positional.push(token);
  }
  return { positional, flags };
}

function readNumeric(flags, names, fallback) {
  for (const name of names) {
    if (!(name in flags)) continue;
    const value = Number(flags[name]);
    if (Number.isFinite(value)) {
      return Math.floor(value);
    }
  }
  return fallback;
}

function readBoolean(flags, names, fallback = false) {
  for (const name of names) {
    if (!(name in flags)) continue;
    const raw = String(flags[name]).trim().toLowerCase();
    if (raw === '' || raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on') return true;
    if (raw === '0' || raw === 'false' || raw === 'no' || raw === 'off') return false;
  }
  return fallback;
}

function readJson(flags, names) {
  for (const name of names) {
    if (!(name in flags)) continue;
    try {
      return JSON.parse(String(flags[name]));
    } catch {
      return null;
    }
  }
  return null;
}

function severityRank(raw) {
  const value = String(raw || '').toLowerCase();
  if (value === 'critical') return 4;
  if (value === 'high') return 3;
  if (value === 'medium') return 2;
  if (value === 'low') return 1;
  return 0;
}

function validateDescendingRanking(scores = [], ids = []) {
  if (!Array.isArray(scores) || !Array.isArray(ids)) {
    return { ok: false, reason_code: 'ranking_shape_invalid' };
  }
  if (scores.length !== ids.length) {
    return { ok: false, reason_code: 'ranking_shape_mismatch' };
  }
  for (let idx = 0; idx < scores.length; idx += 1) {
    const score = Number(scores[idx]);
    if (!Number.isFinite(score)) {
      return { ok: false, reason_code: 'ranking_non_finite_score' };
    }
    if (idx === 0) continue;
    const prevScore = Number(scores[idx - 1]);
    if (score > prevScore) {
      return { ok: false, reason_code: 'ranking_not_descending' };
    }
    if (score === prevScore && String(ids[idx]) < String(ids[idx - 1])) {
      return { ok: false, reason_code: 'ranking_tie_not_stable' };
    }
  }
  return { ok: true, reason_code: 'ranking_descending_stable' };
}

function validateLensMapAnnotation(annotation) {
  if (annotation == null) {
    return { ok: true, reason_code: 'lensmap_annotation_not_provided' };
  }
  if (typeof annotation !== 'object' || Array.isArray(annotation)) {
    return { ok: false, reason_code: 'lensmap_annotation_invalid_type' };
  }

  const nodeId = String(annotation.node_id || annotation.nodeId || '').trim();
  if (!nodeId) {
    return { ok: false, reason_code: 'lensmap_annotation_missing_node_id' };
  }

  const tags = Array.isArray(annotation.tags) ? annotation.tags : [];
  const jots = Array.isArray(annotation.jots) ? annotation.jots : [];
  if (!tags.length && !jots.length) {
    return { ok: false, reason_code: 'lensmap_annotation_missing_tags_or_jots' };
  }

  const normalized = new Set();
  for (const tag of tags) {
    const value = String(tag || '').trim().toLowerCase();
    if (!value) {
      return { ok: false, reason_code: 'lensmap_annotation_empty_tag' };
    }
    if (normalized.has(value)) {
      return { ok: false, reason_code: 'lensmap_annotation_duplicate_tag' };
    }
    normalized.add(value);
  }

  return { ok: true, reason_code: 'lensmap_annotation_valid' };
}

function commandNameFromArgs(args = [], fallback = 'status') {
  const parsed = parseCliArgs(args);
  const token = parsed.positional[0] || fallback;
  return String(token || fallback).trim().toLowerCase();
}

function buildFailure(reasonCode, details = {}) {
  return {
    ok: false,
    type: 'memory_policy_validation',
    reason_code: reasonCode,
    details
  };
}

function validateMemoryPolicy(args = [], options = {}) {
  const policy = Object.assign({}, DEFAULT_POLICY, options.policy || {});
  const parsed = parseCliArgs(args);
  const command = String(options.command || parsed.positional[0] || 'status').trim().toLowerCase();

  if (NON_EXECUTING_COMMANDS.has(command)) {
    return {
      ok: true,
      type: 'memory_policy_validation',
      reason_code: 'policy_not_required_for_status_command',
      policy
    };
  }

  if (policy.index_first_required) {
    if (readBoolean(parsed.flags, INDEX_BYPASS_FLAGS, false)) {
      return buildFailure('index_first_bypass_forbidden', { command });
    }
    if (DIRECT_READ_FLAGS.some((flag) => typeof parsed.flags[flag] === 'string' && parsed.flags[flag].trim() !== '')) {
      return buildFailure('direct_file_read_forbidden', { command });
    }
  }

  const bootstrap = readBoolean(parsed.flags, ['bootstrap'], false);
  const lazyHydration = readBoolean(parsed.flags, ['lazy-hydration', 'lazy_hydration'], true);
  const hydrationTokens = readNumeric(
    parsed.flags,
    ['estimated-hydration-tokens', 'estimated_hydration_tokens'],
    0
  );
  if (bootstrap && !lazyHydration) {
    return buildFailure('bootstrap_requires_lazy_hydration', { command });
  }
  if (bootstrap && hydrationTokens > policy.bootstrap_hydration_token_cap) {
    return buildFailure('bootstrap_hydration_token_cap_exceeded', {
      cap: policy.bootstrap_hydration_token_cap,
      hydration_tokens: hydrationTokens
    });
  }

  const burnThreshold = readNumeric(
    parsed.flags,
    ['burn-threshold', 'burn_threshold', 'burn-slo-threshold', 'burn_slo_threshold'],
    policy.max_burn_slo_tokens
  );
  if (burnThreshold > policy.max_burn_slo_tokens) {
    return buildFailure('burn_slo_threshold_exceeded', {
      configured_threshold: burnThreshold,
      max_burn_slo_tokens: policy.max_burn_slo_tokens
    });
  }

  const failClosed = readBoolean(parsed.flags, ['fail-closed', 'fail_closed'], true);
  if (!failClosed) {
    return buildFailure('fail_closed_required', { command });
  }

  const top = readNumeric(parsed.flags, ['top', 'recall-top', 'recall_top'], 5);
  const maxFiles = readNumeric(parsed.flags, ['max-files', 'max_files'], 1);
  const expandLines = readNumeric(parsed.flags, ['expand-lines', 'expand_lines'], 0);
  if (top > policy.max_recall_top || maxFiles > policy.max_max_files || expandLines > policy.max_expand_lines) {
    return buildFailure('recall_budget_exceeded', {
      top,
      max_files: maxFiles,
      expand_lines: expandLines,
      policy: {
        max_recall_top: policy.max_recall_top,
        max_max_files: policy.max_max_files,
        max_expand_lines: policy.max_expand_lines
      }
    });
  }

  if (policy.block_stale_override && readBoolean(parsed.flags, STALE_OVERRIDE_FLAGS, false)) {
    return buildFailure('stale_override_forbidden', { command });
  }

  const scores = readJson(parsed.flags, ['scores-json', 'scores_json']);
  const ids = readJson(parsed.flags, ['ids-json', 'ids_json']);
  if (scores != null || ids != null) {
    const rank = validateDescendingRanking(scores || [], ids || []);
    if (!rank.ok) {
      return buildFailure(rank.reason_code, { command });
    }
  }

  const annotation = readJson(parsed.flags, ['lensmap-annotation-json', 'lensmap_annotation_json']);
  if (annotation != null) {
    const lensMapValidation = validateLensMapAnnotation(annotation);
    if (!lensMapValidation.ok) {
      return buildFailure(lensMapValidation.reason_code, { command });
    }
  }

  return {
    ok: true,
    type: 'memory_policy_validation',
    reason_code: 'policy_ok',
    command,
    policy,
    effective_budget: {
      top,
      max_files: maxFiles,
      expand_lines: expandLines,
      burn_threshold: burnThreshold
    }
  };
}

function guardFailureResult(validation, context = {}) {
  const payload = Object.assign(
    {
      ok: false,
      type: 'memory_policy_guard_reject',
      reason: validation && validation.reason_code ? validation.reason_code : 'policy_validation_failed',
      layer: 'client_runtime_memory_guard',
      fail_closed: true
    },
    context && typeof context === 'object' ? context : {}
  );
  return {
    ok: false,
    status: 2,
    stdout: `${JSON.stringify(payload)}\n`,
    stderr: `memory_policy_guard_reject:${payload.reason}\n`,
    payload
  };
}

module.exports = {
  DEFAULT_POLICY,
  parseCliArgs,
  commandNameFromArgs,
  validateMemoryPolicy,
  guardFailureResult,
  validateDescendingRanking,
  validateLensMapAnnotation,
  severityRank
};
