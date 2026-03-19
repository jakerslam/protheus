#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const DEFAULT_POLICY = Object.freeze({
  index_first_required: true,
  max_burn_slo_tokens: 200,
  max_recall_top: 50,
  max_max_files: 20,
  max_expand_lines: 300,
  bootstrap_hydration_token_cap: 48,
  block_stale_override: true
});

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'policy_validator', 'memory-policy-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command, payload = {}) {
  const args = [command, `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`];
  const out = bridge.run(args);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (!payloadOut || typeof payloadOut !== 'object') {
    return {
      ok: false,
      error: out && out.stderr ? String(out.stderr).trim() || 'memory_policy_kernel_bridge_failed' : 'memory_policy_kernel_bridge_failed'
    };
  }
  return payloadOut;
}

function parseCliArgs(args = []) {
  const out = invoke('parse-cli', { args });
  return out.parsed && typeof out.parsed === 'object'
    ? out.parsed
    : { positional: [], flags: {} };
}

function commandNameFromArgs(args = [], fallback = 'status') {
  const normalizedArgs = Array.isArray(args) ? args.map((row) => String(row)) : [];
  const localCommand = normalizedArgs.find((token) => !token.startsWith('--'));
  if (localCommand) return String(localCommand).trim().toLowerCase();
  const out = invoke('command-name', { args, fallback });
  return String(out.command || fallback).trim().toLowerCase();
}

function validateDescendingRanking(scores = [], ids = []) {
  const out = invoke('validate-ranking', { scores, ids });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, reason_code: 'ranking_validation_failed' };
}

function validateLensMapAnnotation(annotation) {
  const out = invoke('validate-lensmap', { annotation });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, reason_code: 'lensmap_annotation_invalid' };
}

function severityRank(raw) {
  const out = invoke('severity-rank', { value: raw });
  return Number.isFinite(Number(out.rank)) ? Number(out.rank) : 0;
}

function parseArgsToFlags(args = []) {
  const flags = {};
  const positional = [];
  for (const token of Array.isArray(args) ? args : []) {
    const value = String(token || '');
    if (!value.startsWith('--')) {
      positional.push(value);
      continue;
    }
    const eq = value.indexOf('=');
    if (eq === -1) {
      flags[value.slice(2)] = '1';
      continue;
    }
    flags[value.slice(2, eq)] = value.slice(eq + 1);
  }
  return { flags, positional };
}

function truthyFlag(raw) {
  const value = String(raw == null ? '' : raw).trim().toLowerCase();
  return ['1', 'true', 'yes', 'on'].includes(value);
}

function parseJsonArray(raw) {
  if (raw == null || raw === '') return null;
  try {
    const parsed = JSON.parse(String(raw));
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function localValidateMemoryPolicy(args = []) {
  const { flags } = parseArgsToFlags(args);
  const details = {};

  if (truthyFlag(flags['bypass']) || truthyFlag(flags['allow-full-scan'])) {
    return { ok: false, type: 'memory_policy_validation', reason_code: 'index_first_bypass_forbidden', details };
  }
  if (
    (typeof flags['file'] === 'string' && flags['file'].trim() !== '') ||
    (typeof flags['path'] === 'string' && flags['path'].trim() !== '')
  ) {
    return { ok: false, type: 'memory_policy_validation', reason_code: 'direct_file_read_forbidden', details };
  }
  if (truthyFlag(flags['bootstrap']) && !truthyFlag(flags['lazy-hydration'])) {
    return { ok: false, type: 'memory_policy_validation', reason_code: 'bootstrap_requires_lazy_hydration', details };
  }

  const burnThreshold = Number(flags['burn-threshold']);
  if (Number.isFinite(burnThreshold) && burnThreshold > DEFAULT_POLICY.max_burn_slo_tokens) {
    return { ok: false, type: 'memory_policy_validation', reason_code: 'burn_slo_threshold_exceeded', details };
  }

  const top = Number(flags['top']);
  const maxFiles = Number(flags['max-files']);
  const expandLines = Number(flags['expand-lines']);
  if (
    (Number.isFinite(top) && top > DEFAULT_POLICY.max_recall_top) ||
    (Number.isFinite(maxFiles) && maxFiles > DEFAULT_POLICY.max_max_files) ||
    (Number.isFinite(expandLines) && expandLines > DEFAULT_POLICY.max_expand_lines)
  ) {
    return { ok: false, type: 'memory_policy_validation', reason_code: 'recall_budget_exceeded', details };
  }

  if (DEFAULT_POLICY.block_stale_override && truthyFlag(flags['allow-stale'])) {
    return { ok: false, type: 'memory_policy_validation', reason_code: 'stale_override_forbidden', details };
  }

  const scores = parseJsonArray(flags['scores-json']);
  const ids = parseJsonArray(flags['ids-json']);
  if (scores && ids && scores.length === ids.length && scores.length > 1) {
    for (let i = 1; i < scores.length; i += 1) {
      const prev = Number(scores[i - 1]);
      const curr = Number(scores[i]);
      if (Number.isFinite(prev) && Number.isFinite(curr) && curr > prev) {
        return { ok: false, type: 'memory_policy_validation', reason_code: 'ranking_not_descending', details };
      }
    }
  }

  if (typeof flags['lensmap-annotation-json'] === 'string') {
    try {
      const annotation = JSON.parse(flags['lensmap-annotation-json']);
      const tags = Array.isArray(annotation && annotation.tags) ? annotation.tags : [];
      const jots = Array.isArray(annotation && annotation.jots) ? annotation.jots : [];
      if (tags.length === 0 || jots.length === 0) {
        return {
          ok: false,
          type: 'memory_policy_validation',
          reason_code: 'lensmap_annotation_missing_tags_or_jots',
          details
        };
      }
    } catch {
      return {
        ok: false,
        type: 'memory_policy_validation',
        reason_code: 'lensmap_annotation_missing_tags_or_jots',
        details
      };
    }
  }

  return {
    ok: true,
    type: 'memory_policy_validation',
    reason_code: 'policy_ok',
    details
  };
}

function validateMemoryPolicy(args = [], options = {}) {
  void options;
  return localValidateMemoryPolicy(args);
}

function guardFailureResult(validation, context = {}) {
  if (validation && typeof validation.reason_code === 'string' && validation.reason_code.trim()) {
    const reason = validation.reason_code.trim();
    return {
      ok: false,
      status: 2,
      stdout: `${JSON.stringify({
        ok: false,
        type: 'memory_policy_guard_reject',
        reason,
        layer: 'client_runtime_memory_guard',
        fail_closed: true
      })}\n`,
      stderr: `memory_policy_guard_reject:${reason}\n`,
      payload: {
        ok: false,
        type: 'memory_policy_guard_reject',
        reason,
        layer: 'client_runtime_memory_guard',
        fail_closed: true
      }
    };
  }
  const out = invoke('guard-failure', {
    validation: validation && typeof validation === 'object' ? validation : {},
    context: context && typeof context === 'object' ? context : {}
  });
  return out.result && typeof out.result === 'object'
    ? out.result
    : {
        ok: false,
        status: 2,
        stdout: `${JSON.stringify({
          ok: false,
          type: 'memory_policy_guard_reject',
          reason: String(
            validation && typeof validation.reason_code === 'string'
              ? validation.reason_code
              : 'policy_validation_failed'
          ),
          layer: 'client_runtime_memory_guard',
          fail_closed: true
        })}\n`,
        stderr: `memory_policy_guard_reject:${String(
          validation && typeof validation.reason_code === 'string'
            ? validation.reason_code
            : 'policy_validation_failed'
        )}\n`,
        payload: {
          ok: false,
          type: 'memory_policy_guard_reject',
          reason: String(
            validation && typeof validation.reason_code === 'string'
              ? validation.reason_code
              : 'policy_validation_failed'
          ),
          layer: 'client_runtime_memory_guard',
          fail_closed: true
        }
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
