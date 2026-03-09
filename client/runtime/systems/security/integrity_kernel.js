#!/usr/bin/env node
'use strict';

/**
 * integrity_kernel.js
 *
 * Layer ownership: core/layer1/security::integrity-kernel (authoritative)
 * Client wrapper is core-first with compatibility fallback.
 */

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');
const path = require('path');
const {
  DEFAULT_POLICY_PATH,
  verifyIntegrity,
  sealIntegrity
} = require('../../lib/security_integrity');

process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '1500';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '2000';

const COMMAND = 'integrity-kernel';
const bridge = createOpsLaneBridge(__dirname, 'integrity_kernel', 'security-plane');

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/integrity_kernel.js run [--policy=/abs/path.json]');
  console.log('  node systems/security/integrity_kernel.js status [--policy=/abs/path.json]');
  console.log('  node systems/security/integrity_kernel.js seal [--policy=/abs/path.json] --approval-note="..."');
  console.log('  node systems/security/integrity_kernel.js --help');
}

function parseArgs(argv) {
  const out = { _: [] };
  for (const arg of argv) {
    if (!arg.startsWith('--')) {
      out._.push(arg);
      continue;
    }
    const eq = arg.indexOf('=');
    if (eq === -1) out[arg.slice(2)] = true;
    else out[arg.slice(2, eq)] = arg.slice(eq + 1);
  }
  return out;
}

function runCore(args = []) {
  try {
    return bridge.run([COMMAND, ...(Array.isArray(args) ? args : [])]);
  } catch {
    return null;
  }
}

function coreResultUsable(out) {
  if (!out || Number(out.status) !== 0) return false;
  if (!out.payload || typeof out.payload !== 'object') return false;
  if (out.payload.ok === false) return false;
  return true;
}

function printCore(out) {
  if (!out) return;
  if (out.stdout) process.stdout.write(out.stdout);
  else if (out.payload) process.stdout.write(`${JSON.stringify(out.payload, null, 2)}\n`);
  if (out.stderr) process.stderr.write(out.stderr);
}

function runLegacy(argv) {
  const args = parseArgs(argv);
  const cmd = String(args._[0] || '').trim().toLowerCase();
  if (!cmd || cmd === '--help' || cmd === '-h' || cmd === 'help' || args.help) {
    usage();
    process.exit(0);
  }
  const policyPath = path.resolve(String(args.policy || DEFAULT_POLICY_PATH));
  if (cmd === 'run' || cmd === 'status') {
    const result = verifyIntegrity(policyPath);
    process.stdout.write(JSON.stringify(result, null, 2) + '\n');
    if (!result.ok) process.exit(1);
    process.exit(0);
  }
  if (cmd === 'seal') {
    const approvalNote = String(args['approval-note'] || args.approval_note || '').trim();
    if (approvalNote.length < 10) {
      process.stdout.write(JSON.stringify({
        ok: false,
        error: 'approval_note_too_short',
        min_len: 10
      }) + '\n');
      process.exit(2);
    }
    const result = sealIntegrity(policyPath, {
      approval_note: approvalNote,
      sealed_by: process.env.USER || 'unknown'
    });
    process.stdout.write(JSON.stringify(result, null, 2) + '\n');
    if (!result.ok) process.exit(1);
    process.exit(0);
  }
  usage();
  process.exit(2);
}

if (require.main === module) {
  const argv = process.argv.slice(2);
  const out = runCore(argv);
  if (coreResultUsable(out)) {
    printCore(out);
    process.exit(0);
  }
  runLegacy(argv);
}

module.exports = {
  lane: bridge.lane,
  run: (args = []) => {
    const out = runCore(args);
    if (coreResultUsable(out)) return out;
    return null;
  }
};
