#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { invokeKernelPayload } = require('../../lib/protheus_kernel_bridge.ts');
function invoke(command, payload = {}, opts = {}) {
  return invokeKernelPayload(
    'memory-policy-kernel',
    command,
    payload,
    {
      throwOnError: opts.throwOnError,
      fallbackError: 'memory_policy_kernel_bridge_failed',
    }
  );
}

function parseCliArgs(args = [], options = {}) {
  const out = invoke(
    'parse-cli',
    { args, options: options && typeof options === 'object' ? options : {} },
    { throwOnError: false }
  );
  return out.parsed && typeof out.parsed === 'object'
    ? out.parsed
    : { positional: [], flags: {} };
}

function commandNameFromArgs(args = [], fallback = 'status') {
  const out = invoke('command-name', { args, fallback }, { throwOnError: false });
  return String(out.command || fallback).trim().toLowerCase();
}

function validateDescendingRanking(scores = [], ids = []) {
  const out = invoke('validate-ranking', { scores, ids }, { throwOnError: false });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, reason_code: 'ranking_validation_failed' };
}

function validateLensMapAnnotation(annotation) {
  const out = invoke('validate-lensmap', { annotation }, { throwOnError: false });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, reason_code: 'lensmap_annotation_invalid' };
}

function severityRank(raw) {
  const out = invoke('severity-rank', { value: raw }, { throwOnError: false });
  return Number.isFinite(Number(out.rank)) ? Number(out.rank) : 0;
}

function validateMemoryPolicy(args = [], options = {}) {
  const out = invoke(
    'validate',
    {
      args: Array.isArray(args) ? args : [],
      options: options && typeof options === 'object' ? options : {}
    },
    { throwOnError: false }
  );
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : { ok: false, type: 'memory_policy_validation', reason_code: 'policy_validation_failed' };
}

function guardFailureResult(validation, context = {}) {
  const out = invoke(
    'guard-failure',
    {
      validation: validation && typeof validation === 'object' ? validation : {},
      context: context && typeof context === 'object' ? context : {}
    },
    { throwOnError: false }
  );
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

const DEFAULT_POLICY = (() => {
  const status = invoke('status', {}, { throwOnError: false });
  const candidate = status.default_policy && typeof status.default_policy === 'object'
    ? status.default_policy
    : {};
  return Object.freeze({
    index_first_required: candidate.index_first_required !== false,
    max_burn_slo_tokens: Number.isFinite(Number(candidate.max_burn_slo_tokens))
      ? Number(candidate.max_burn_slo_tokens)
      : 200,
    max_recall_top: Number.isFinite(Number(candidate.max_recall_top))
      ? Number(candidate.max_recall_top)
      : 50,
    max_max_files: Number.isFinite(Number(candidate.max_max_files))
      ? Number(candidate.max_max_files)
      : 20,
    max_expand_lines: Number.isFinite(Number(candidate.max_expand_lines))
      ? Number(candidate.max_expand_lines)
      : 300,
    bootstrap_hydration_token_cap: Number.isFinite(Number(candidate.bootstrap_hydration_token_cap))
      ? Number(candidate.bootstrap_hydration_token_cap)
      : 48,
    block_stale_override: candidate.block_stale_override !== false
  });
})();

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
