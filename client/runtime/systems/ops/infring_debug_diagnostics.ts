#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::infring-control-plane (authoritative)
// Thin TypeScript launcher wrapper only.

const { runInfringOps, invokeInfringOpsViaBridge } = require('./run_infring_ops.ts');

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv)
    ? argv.map((token) => String(token || '').trim()).filter(Boolean)
    : [];
}

function normalizeSubcommand(args = []) {
  const sub = String(args[0] || 'status').trim().toLowerCase();
  if (sub === 'status' || sub === 'health') return 'status';
  if (sub === 'run' || sub === 'diagnose' || sub === 'diag') return 'run';
  return 'run';
}

function mappedOpsArgs(args = []) {
  const sub = normalizeSubcommand(args);
  return sub === 'status'
    ? ['infring-control-plane', 'status', ...args.slice(1)]
    : ['infring-control-plane', 'run', ...args];
}

function normalizeBridgePayload(out, command) {
  const payload = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  if (payload && payload.payload && typeof payload.payload === 'object') {
    return {
      status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1,
      command,
      ...payload.payload,
    };
  }
  if (payload && typeof payload === 'object') {
    return {
      status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1,
      command,
      ...payload,
    };
  }
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  return {
    ok: status === 0,
    type: 'infring_debug_diagnostics',
    command,
    status,
    error:
      out && out.stderr
        ? String(out.stderr).trim() || 'infring_debug_diagnostics_bridge_failed'
        : 'infring_debug_diagnostics_bridge_failed'
  };
}

function runDetailed(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const mapped = mappedOpsArgs(args);
  const command = String(mapped[1] || 'run');
  const options = {
    env: {
      INFRING_OPS_USE_PREBUILT: '0',
      INFRING_OPS_LOCAL_TIMEOUT_MS: '120000',
    },
    unknownDomainFallback: false,
  };
  const bridgeOut = invokeInfringOpsViaBridge(mapped, options);
  if (bridgeOut && typeof bridgeOut === 'object') {
    return normalizeBridgePayload(bridgeOut, command);
  }
  const status = Number(runInfringOps(mapped, options));
  return {
    ok: status === 0,
    type: 'infring_debug_diagnostics',
    command,
    status,
  };
}

function run(argv = process.argv.slice(2)) {
  const result = runDetailed(argv);
  const status = Number(result && result.status);
  return Number.isFinite(status) ? status : (result && result.ok ? 0 : 1);
}

function runCli(argv = process.argv.slice(2)) {
  const result = runDetailed(argv);
  process.stdout.write(`${JSON.stringify(result)}\n`);
  const status = Number(result && result.status);
  return Number.isFinite(status) ? status : (result && result.ok ? 0 : 1);
}

if (require.main === module) {
  process.exit(runCli(process.argv.slice(2)));
}

module.exports = {
  run,
  runCli,
  runDetailed,
  normalizeArgs,
  normalizeSubcommand,
};
