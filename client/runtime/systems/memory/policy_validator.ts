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

function validateMemoryPolicy(args = [], options = {}) {
  const out = invoke('validate', {
    args,
    options: options && typeof options === 'object' ? options : {}
  });
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : {
        ok: false,
        type: 'memory_policy_validation',
        reason_code: 'policy_validation_failed',
        details: {}
      };
}

function guardFailureResult(validation, context = {}) {
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
          reason: 'policy_validation_failed',
          layer: 'client_runtime_memory_guard',
          fail_closed: true
        })}\n`,
        stderr: 'memory_policy_guard_reject:policy_validation_failed\n',
        payload: {
          ok: false,
          type: 'memory_policy_guard_reject',
          reason: 'policy_validation_failed',
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
