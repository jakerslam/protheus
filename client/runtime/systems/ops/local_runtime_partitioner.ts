#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const RESET_CONFIRM = 'RESET_LOCAL';

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
const bridge = createOpsLaneBridge(__dirname, 'local_runtime_partitioner', 'local-runtime-partitioner');

function parseArgValue(args, key) {
  const list = Array.isArray(args) ? args.map((arg) => String(arg)) : [];
  const inline = list.find((arg) => arg.startsWith(`${key}=`));
  if (inline) return inline.slice(key.length + 1).trim();
  const idx = list.findIndex((arg) => arg === key);
  if (idx >= 0 && idx + 1 < list.length) {
    return String(list[idx + 1]).trim();
  }
  return '';
}

function usagePayload(command) {
  return {
    ok: false,
    type: 'local_runtime_partitioner_usage',
    command: String(command || '').trim().toLowerCase() || 'status',
    usage: [
      'local_runtime_partitioner.ts status',
      'local_runtime_partitioner.ts init',
      `local_runtime_partitioner.ts reset --confirm=${RESET_CONFIRM}`
    ]
  };
}

function helpPayload() {
  return {
    ok: true,
    type: 'local_runtime_partitioner_help',
    usage: usagePayload('help').usage
  };
}

function normalizePayload(out, fallbackCommand) {
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  if (receipt && receipt.payload && typeof receipt.payload === 'object') {
    return receipt.payload;
  }
  if (receipt) return receipt;
  return {
    ok: false,
    type: 'local_runtime_partitioner',
    command: fallbackCommand || 'status',
    error: out && out.stderr ? String(out.stderr).trim() || 'local_runtime_partitioner_bridge_failed' : 'local_runtime_partitioner_bridge_failed',
    fail_closed: true
  };
}

function invoke(command, extraArgs = [], workspaceRoot = WORKSPACE_ROOT) {
  const args = [command];
  if (workspaceRoot) {
    args.push(`--workspace-root=${path.resolve(String(workspaceRoot))}`);
  }
  for (const arg of Array.isArray(extraArgs) ? extraArgs : []) {
    if (arg == null) continue;
    args.push(String(arg));
  }
  const out = bridge.run(args);
  return normalizePayload(out, command);
}

function continuityStatus(workspaceRoot = WORKSPACE_ROOT) {
  return invoke('status', [], workspaceRoot);
}

function initLocalRuntime(workspaceRoot = WORKSPACE_ROOT) {
  return invoke('init', [], workspaceRoot);
}

function resetLocalRuntime(args = [], workspaceRoot = WORKSPACE_ROOT) {
  const confirm = parseArgValue(args, '--confirm');
  if (confirm !== RESET_CONFIRM) {
    return {
      ok: false,
      type: 'local_runtime_partitioner',
      command: 'reset',
      reason: 'confirm_required',
      expected_confirm: RESET_CONFIRM,
      status: 2,
      fail_closed: true
    };
  }
  const extraArgs = [];
  extraArgs.push(`--confirm=${confirm}`);
  return invoke('reset', extraArgs, workspaceRoot);
}

function run(argv = [], options = {}) {
  const args = Array.isArray(argv) ? argv.map((arg) => String(arg)) : [];
  const workspaceRoot = options.workspaceRoot
    ? path.resolve(String(options.workspaceRoot))
    : WORKSPACE_ROOT;
  const command = (args[0] || 'status').trim().toLowerCase();
  if (command === 'help' || command === '--help' || command === '-h') {
    return helpPayload();
  }
  switch (command) {
    case 'init':
      return initLocalRuntime(workspaceRoot);
    case 'reset':
      return resetLocalRuntime(args.slice(1), workspaceRoot);
    case 'status':
    default:
      if (command !== 'status') {
        return usagePayload(command);
      }
      return continuityStatus(workspaceRoot);
  }
}

module.exports = {
  RESET_CONFIRM,
  run,
  continuityStatus,
  initLocalRuntime,
  resetLocalRuntime,
};

if (require.main === module) {
  const result = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  if (!result.ok) {
    process.exit(Number.isFinite(Number(result && result.status)) ? Number(result.status) : 1);
  }
}
