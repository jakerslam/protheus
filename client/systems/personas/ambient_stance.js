#!/usr/bin/env node
'use strict';

const path = require('path');
const fs = require('fs');
const crypto = require('crypto');
const { runPersonaAmbientCommand } = require('../../lib/spine_conduit_bridge');

const ROOT = path.resolve(__dirname, '..', '..');

function readJson(filePath, fallback = null) {
  try {
    if (!filePath || !fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function parseArgs(argv) {
  const out = { _: [] };
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

function toBool(v, fallback = false) {
  const raw = String(v == null ? '' : v).trim().toLowerCase();
  if (!raw) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function loadPolicy() {
  const raw = String(process.env.MECH_SUIT_MODE_POLICY_PATH || '').trim();
  const policyPath = raw
    ? (path.isAbsolute(raw) ? raw : path.join(ROOT, raw))
    : path.join(ROOT, 'config', 'mech_suit_mode_policy.json');
  const policy = readJson(policyPath, {}) || {};
  const personas = policy && policy.personas && typeof policy.personas === 'object' ? policy.personas : {};
  return {
    ambient_stance: personas.ambient_stance !== false,
    latest_path: String(personas.latest_path || path.join(ROOT, 'local', 'state', 'personas', 'ambient_stance', 'latest.json')),
    receipts_path: String(personas.receipts_path || path.join(ROOT, 'local', 'state', 'personas', 'ambient_stance', 'receipts.jsonl'))
  };
}

function hash(payload) {
  return crypto.createHash('sha256').update(JSON.stringify(payload)).digest('hex');
}

function parseStance(raw) {
  const text = String(raw == null ? '' : raw).trim();
  if (!text) return {};
  try {
    const parsed = JSON.parse(text);
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function localCompatRun(argv = []) {
  const args = parseArgs(argv);
  const command = String(args._[0] || 'status').trim().toLowerCase();
  const persona = String(args.persona || args._[1] || 'default').trim() || 'default';
  const policy = loadPolicy();
  const ambientMode = toBool(process.env.MECH_SUIT_MODE_FORCE, false) && policy.ambient_stance === true;

  if (command === 'apply') {
    const stance = parseStance(args['stance-json'] || args.stance_json);
    const payload = {
      ok: true,
      type: 'persona_ambient_apply',
      ts: new Date().toISOString(),
      persona,
      ambient_mode_active: ambientMode,
      delta_applied: true,
      reload: 'incremental',
      source: String(args.source || 'compat').trim() || 'compat',
      stance
    };
    payload.receipt_hash = hash(payload);
    writeJson(policy.latest_path, payload);
    appendJsonl(policy.receipts_path, payload);
    return { ok: true, status: 0, payload, stderr: '' };
  }

  const latest = readJson(policy.latest_path, {});
  const payload = {
    ok: true,
    type: 'persona_ambient_status',
    ts: new Date().toISOString(),
    persona,
    ambient_mode_active: ambientMode,
    delta_applied: latest && latest.delta_applied === true,
    reload: latest && latest.reload ? String(latest.reload) : 'incremental',
    receipt_hash: latest && latest.receipt_hash ? String(latest.receipt_hash) : null
  };
  return { ok: true, status: 0, payload, stderr: '' };
}

function normalizeGateDegraded(out) {
  if (!out || !out.payload || out.payload.gate_active !== true) return out;
  return {
    ...out,
    ok: true,
    status: 0,
    payload: {
      ok: true,
      blocked: true,
      type: 'persona_ambient_status',
      degraded: true,
      degraded_reason: 'conduit_runtime_gate_active',
      gate_active: true,
      gate_reason: String(out.payload.reason || '').slice(0, 240) || 'conduit_runtime_gate_active',
      routed_via: 'conduit'
    },
    stderr: ''
  };
}

async function run(args = [], opts = {}) {
  if (toBool(process.env.PROTHEUS_PERSONA_AMBIENT_LOCAL_COMPAT, false)) {
    return localCompatRun(args);
  }
  const routed = Array.isArray(args) && args.length > 0 ? args : ['status'];
  const out = await runPersonaAmbientCommand(routed, {
    cwdHint: opts.cwdHint || ROOT
  });
  return normalizeGateDegraded(out);
}

async function main() {
  const out = await run(process.argv.slice(2));
  if (out.payload) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  } else if (out.stdout) {
    process.stdout.write(String(out.stdout));
  }
  if (out.stderr) {
    process.stderr.write(String(out.stderr));
    if (!String(out.stderr).endsWith('\n')) process.stderr.write('\n');
  }
  process.exit(Number.isFinite(out.status) ? Number(out.status) : 1);
}

if (require.main === module) {
  main().catch((err) => {
    process.stderr.write(`${String(err && err.message ? err.message : err)}\n`);
    process.exit(1);
  });
}

module.exports = {
  run
};
