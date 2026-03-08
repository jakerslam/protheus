#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const {
  loadMechSuitModePolicy,
  resolvePolicyPath,
  resolveStatePath,
  updateMechSuitStatus
} = require('../../../lib/mech_suit_mode');

const ROOT = path.resolve(__dirname, '..', '..');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out: Record<string, any> = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '');
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx >= 0) {
      out[token.slice(2, idx)] = token.slice(idx + 1);
      continue;
    }
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function parseModeValue(v: unknown) {
  const raw = cleanText(v, 24).toLowerCase();
  if (!raw) return null;
  if (['1', 'true', 'yes', 'on', 'enable', 'enabled', 'active'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off', 'disable', 'disabled', 'inactive'].includes(raw)) return false;
  return null;
}

function usage() {
  console.log('Usage:');
  console.log('  node client/runtime/systems/ops/mech_suit_mode_control.ts status');
  console.log('  node client/runtime/systems/ops/mech_suit_mode_control.ts on');
  console.log('  node client/runtime/systems/ops/mech_suit_mode_control.ts off');
  console.log('  node client/runtime/systems/ops/mech_suit_mode_control.ts set --enabled=1|0');
}

function readPolicyRaw(policyPath: string) {
  try {
    if (!fs.existsSync(policyPath)) return {};
    const parsed = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function writePolicyRaw(policyPath: string, raw: Record<string, any>) {
  fs.mkdirSync(path.dirname(policyPath), { recursive: true });
  fs.writeFileSync(policyPath, `${JSON.stringify(raw, null, 2)}\n`, 'utf8');
}

function emitStatusPayload(type: string, policy: any, changed: boolean, note: string) {
  const root = policy && policy._root ? policy._root : ROOT;
  const policyPath = policy && policy._policy_path ? policy._policy_path : resolvePolicyPath(root);
  const payload: any = {
    ok: true,
    type,
    ts: nowIso(),
    ambient_mode_active: policy && policy.enabled === true,
    changed,
    note: cleanText(note, 180) || null,
    policy_path: path.relative(root, policyPath).replace(/\\/g, '/'),
    status_path: path.relative(root, resolveStatePath(policy, policy.state.status_path)).replace(/\\/g, '/'),
    history_path: path.relative(root, resolveStatePath(policy, policy.state.history_path)).replace(/\\/g, '/')
  };
  payload.receipt_hash = crypto.createHash('sha256')
    .update(JSON.stringify({
      type: payload.type,
      ambient_mode_active: payload.ambient_mode_active,
      changed: payload.changed,
      policy_path: payload.policy_path,
      status_path: payload.status_path
    }), 'utf8')
    .digest('hex');
  return payload;
}

function setMode(enabled: boolean, root: string) {
  const policyPath = resolvePolicyPath(root);
  const raw = readPolicyRaw(policyPath);
  const before = !!raw.enabled;
  raw.enabled = !!enabled;
  writePolicyRaw(policyPath, raw);
  const policy = loadMechSuitModePolicy({ root });
  updateMechSuitStatus('control_plane', {
    ambient_mode_active: policy.enabled === true,
    changed: before !== policy.enabled,
    reason: 'mech_suit_mode_control'
  }, { policy });
  return emitStatusPayload('mech_suit_mode_set', policy, before !== policy.enabled, before !== policy.enabled ? 'updated' : 'no_change');
}

function status(root: string) {
  const policy = loadMechSuitModePolicy({ root });
  return emitStatusPayload('mech_suit_mode_status', policy, false, 'read_only');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 40).toLowerCase();
  const root = path.resolve(cleanText(args.root || process.env.OPENCLAW_WORKSPACE || ROOT, 400) || ROOT);

  if (cmd === 'help' || cmd === '-h' || cmd === '--help') {
    usage();
    process.exit(0);
  }

  if (cmd === 'status') {
    process.stdout.write(`${JSON.stringify(status(root))}\n`);
    process.exit(0);
    return;
  }

  if (cmd === 'on' || cmd === 'off' || cmd === 'set') {
    let enabled: boolean | null = null;
    if (cmd === 'on') enabled = true;
    if (cmd === 'off') enabled = false;
    if (cmd === 'set') enabled = parseModeValue(args.enabled != null ? args.enabled : args['mech-suit-mode']);
    if (enabled == null) {
      process.stdout.write(`${JSON.stringify({
        ok: false,
        type: 'mech_suit_mode_set',
        ts: nowIso(),
        reason: 'invalid_enabled_value',
        hint: '--enabled=1|0'
      })}\n`);
      process.exit(2);
      return;
    }
    process.stdout.write(`${JSON.stringify(setMode(enabled, root))}\n`);
    process.exit(0);
    return;
  }

  usage();
  process.exit(2);
}

if (require.main === module) {
  main().catch((err) => {
    process.stdout.write(`${JSON.stringify({
      ok: false,
      type: 'mech_suit_mode_control',
      ts: nowIso(),
      error: cleanText(err && err.message ? err.message : err, 240)
    })}\n`);
    process.exit(1);
  });
}

