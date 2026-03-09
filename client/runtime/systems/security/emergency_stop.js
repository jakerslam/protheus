#!/usr/bin/env node
'use strict';

/**
 * emergency_stop.js
 *
 * Layer ownership: core/layer1/security::emergency-stop (authoritative)
 * Client wrapper is core-first with compatibility fallback.
 */

const {
  VALID_SCOPES,
  getStopState,
  engageEmergencyStop,
  releaseEmergencyStop
} = require('../../lib/emergency_stop');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '1500';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '2000';

const COMMAND = 'emergency-stop';
const bridge = createOpsLaneBridge(__dirname, 'emergency_stop', 'security-plane');

function usage() {
  console.log('Usage:');
  console.log('  node systems/security/emergency_stop.js status');
  console.log('  node systems/security/emergency_stop.js engage [--scope=all|autonomy|routing|actuation|spine[,..]] --approval-note="..."');
  console.log('  node systems/security/emergency_stop.js release --approval-note="..."');
  console.log('  node systems/security/emergency_stop.js --help');
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

function requireApprovalNote(note) {
  const s = String(note || '').trim();
  if (s.length >= 10) return s;
  process.stdout.write(JSON.stringify({
    ok: false,
    error: 'approval_note_too_short',
    min_len: 10
  }) + '\n');
  process.exit(2);
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
  const cmd = String(args._[0] || '');
  if (!cmd || cmd === '--help' || cmd === '-h' || cmd === 'help' || args.help) {
    usage();
    process.exit(0);
  }

  if (cmd === 'status') {
    process.stdout.write(JSON.stringify({
      ok: true,
      ts: new Date().toISOString(),
      state: getStopState()
    }, null, 2) + '\n');
    process.exit(0);
  }

  if (cmd === 'engage') {
    const note = requireApprovalNote(args['approval-note'] || args.approval_note);
    const scopeRaw = String(args.scope || 'all');
    const next = engageEmergencyStop({
      scopes: scopeRaw,
      approval_note: note,
      actor: args.actor,
      reason: args.reason
    });
    process.stdout.write(JSON.stringify({
      ok: true,
      result: 'engaged',
      ts: new Date().toISOString(),
      valid_scopes: Array.from(VALID_SCOPES),
      state: next
    }, null, 2) + '\n');
    process.exit(0);
  }

  if (cmd === 'release') {
    const note = requireApprovalNote(args['approval-note'] || args.approval_note);
    const next = releaseEmergencyStop({
      approval_note: note,
      actor: args.actor,
      reason: args.reason
    });
    process.stdout.write(JSON.stringify({
      ok: true,
      result: 'released',
      ts: new Date().toISOString(),
      state: next
    }, null, 2) + '\n');
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
