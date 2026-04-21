#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson } = require('./cli_shared.ts');
const { ROOT, invokeProtheusOpsViaBridge } = require('../run_protheus_ops.ts');

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

function invokeOrchestration(op, payload = {}, options = {}) {
  const safePayload = payload && typeof payload === 'object' ? payload : {};
  const args = [
    'orchestration',
    'invoke',
    `--op=${String(op || '').trim()}`,
    `--payload-json=${JSON.stringify(safePayload)}`,
  ];

  const proc = invokeProtheusOpsViaBridge(args, {
    unknownDomainFallback: false,
    env: { PROTHEUS_ROOT: ROOT, ...(options.env || {}) },
  });
  if (!proc) {
    return {
      ok: false,
      type: 'orchestration_bridge_error',
      reason_code: 'bridge_unavailable',
    };
  }

  if (proc.payload && typeof proc.payload === 'object') {
    return proc.payload;
  }

  const parsed = parseJsonOutput(proc.stdout) || parseJsonOutput(proc.stderr);
  if (parsed && typeof parsed === 'object') {
    return parsed;
  }

  return {
    ok: false,
    type: 'orchestration_bridge_error',
    reason_code: `invoke_failed:${Number.isFinite(proc.status) ? proc.status : 1}`,
    stderr: String(proc.stderr || '').trim() || null,
  };
}

module.exports = {
  ROOT,
  parseArgs,
  parseJson,
  invokeOrchestration,
};
