#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson } = require('./cli_shared.ts');
const { ROOT, invokeInfringOpsViaBridge } = require('../run_infring_ops.ts');

function parseJsonOutput(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    const lines = text.split('\n');
    for (let index = 0; index < lines.length; index += 1) {
      const candidate = lines.slice(index).join('\n').trim();
      if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
      try {
        return JSON.parse(candidate);
      } catch {}
    }
  }
  return null;
}

function normalizeBridgePayload(out) {
  if (!out || typeof out !== 'object') return null;
  const normalized = { ...out };
  if (typeof normalized.ok === 'boolean') {
    if (!normalized.type) normalized.type = normalized.ok ? 'orchestration_bridge_result' : 'orchestration_bridge_error';
    if (!normalized.reason_code && normalized.reason) normalized.reason_code = String(normalized.reason);
    return normalized;
  }
  return normalized;
}

function invokeOrchestrationWithBridge(op, payload = {}, options = {}, invokeBridge = invokeInfringOpsViaBridge) {
  const safePayload = payload && typeof payload === 'object' ? payload : {};
  const args = [
    'orchestration',
    'invoke',
    `--op=${String(op || '').trim()}`,
    `--payload-json=${JSON.stringify(safePayload)}`,
  ];

  const proc = invokeBridge(args, {
    unknownDomainFallback: false,
    env: { INFRING_ROOT: ROOT, ...(options.env || {}) },
  });
  if (!proc) {
    return {
      ok: false,
      type: 'orchestration_bridge_error',
      reason_code: 'bridge_unavailable',
    };
  }

  if (proc.payload && typeof proc.payload === 'object') {
    const normalized = normalizeBridgePayload(proc.payload);
    if (normalized) return normalized;
  }

  const parsed = parseJsonOutput(proc.stdout) || parseJsonOutput(proc.stderr);
  if (parsed && typeof parsed === 'object') {
    const normalized = normalizeBridgePayload(parsed);
    if (normalized) return normalized;
  }

  return {
    ok: false,
    type: 'orchestration_bridge_error',
    reason_code: `invoke_failed:${Number.isFinite(proc.status) ? proc.status : 1}`,
    stderr: String(proc.stderr || '').trim() || null,
  };
}

function invokeOrchestration(op, payload = {}, options = {}) {
  return invokeOrchestrationWithBridge(op, payload, options, invokeInfringOpsViaBridge);
}

module.exports = {
  ROOT,
  parseArgs,
  parseJson,
  parseJsonOutput,
  normalizeBridgePayload,
  invokeOrchestrationWithBridge,
  invokeOrchestration,
};
