#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const RESET_CONFIRM = 'RESET_LOCAL';

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
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
  const extraArgs = [];
  if (confirm) {
    extraArgs.push(`--confirm=${confirm}`);
  }
  return invoke('reset', extraArgs, workspaceRoot);
}

function run(argv = [], options = {}) {
  const args = Array.isArray(argv) ? argv.map((arg) => String(arg)) : [];
  const workspaceRoot = options.workspaceRoot
    ? path.resolve(String(options.workspaceRoot))
    : WORKSPACE_ROOT;
  const command = (args[0] || 'status').trim().toLowerCase();
  switch (command) {
    case 'init':
      return initLocalRuntime(workspaceRoot);
    case 'reset':
      return resetLocalRuntime(args.slice(1), workspaceRoot);
    case 'status':
    default:
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
  if (!result.ok) process.exit(1);
}
