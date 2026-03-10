#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::model-router (authoritative)
// Client wrapper routes to Rust lane; JS keeps compatibility helpers only.

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

const bridge = createOpsLaneBridge(__dirname, 'model_router', 'model-router');
const ROOT = path.resolve(__dirname, '..', '..');
const CONFIG_PATH = process.env.ROUTER_CONFIG_PATH || path.join(ROOT, 'config', 'agent_routing_rules.json');
const STATE_DIR = process.env.ROUTER_STATE_DIR || path.join(ROOT, 'state', 'routing');
const HEALTH_PATH = path.join(STATE_DIR, 'model_health.json');
const BANS_PATH = path.join(STATE_DIR, 'banned_models.json');

process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '60000';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
process.env.PROTHEUS_CONDUIT_COMPAT_FALLBACK = '0';

function readJson(filePath, fallback = {}) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, payload) {
  try {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
    return true;
  } catch {
    return false;
  }
}

function parseJsonPayload(raw) {
  const text = String(raw || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {
      // keep scanning
    }
  }
  return null;
}

function parseArgs(argv = []) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '').trim();
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx > 2) {
      out[token.slice(2, idx)] = token.slice(idx + 1);
      continue;
    }
    const key = token.slice(2);
    const next = String(argv[i + 1] || '');
    if (next && !next.startsWith('--')) {
      out[key] = next;
      i += 1;
      continue;
    }
    out[key] = '1';
  }
  return out;
}

function isLocalOllamaModel(modelId) {
  const model = String(modelId || '').trim();
  return model.startsWith('ollama/') && !model.includes(':cloud');
}

function bansMap() {
  const raw = readJson(BANS_PATH, {});
  return raw && typeof raw === 'object' ? raw : {};
}

function isBanned(modelId) {
  const model = String(modelId || '').trim();
  if (!model) return false;
  const bans = bansMap();
  const row = bans[model];
  if (!row || typeof row !== 'object') return false;
  const expiresMs = Number(row.expires_ms || 0);
  if (Number.isFinite(expiresMs) && expiresMs > 0 && Date.now() > expiresMs) {
    return false;
  }
  return true;
}

function health(modelId, force = false) {
  const model = String(modelId || '').trim();
  if (!model) {
    return {
      model_id: model,
      available: null,
      latency_ms: null,
      follows_instructions: null,
      generic_hits: null,
      sample: '',
      source: 'model_health_file',
      forced: force === true
    };
  }

  const raw = readJson(HEALTH_PATH, {});
  const row = raw && typeof raw === 'object' ? raw[model] : null;
  if (!row || typeof row !== 'object') {
    return {
      model_id: model,
      available: null,
      latency_ms: null,
      follows_instructions: null,
      generic_hits: null,
      sample: '',
      source: 'model_health_file',
      forced: force === true
    };
  }

  return {
    model_id: model,
    available: row.available === true,
    latency_ms: Number.isFinite(Number(row.latency_ms)) ? Number(row.latency_ms) : null,
    follows_instructions: row.follows_instructions === true,
    generic_hits: Number.isFinite(Number(row.generic_hits)) ? Number(row.generic_hits) : null,
    sample: String(row.sample || '').slice(0, 220),
    reason: row.reason || null,
    ts: row.ts || null,
    source: 'model_health_file',
    forced: force === true
  };
}

function coreRouteArgs(argv = []) {
  const parsed = parseArgs(argv);
  const cmd = String(parsed._[0] || 'status').toLowerCase();
  const tail = argv.slice(1);

  if (cmd === 'route') return ['infer', ...tail];
  if (cmd === 'infer' || cmd === 'run' || cmd === 'status' || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    return [cmd === 'run' ? 'infer' : cmd === '--help' || cmd === '-h' ? 'help' : cmd, ...tail];
  }
  return null;
}

function runCore(args = []) {
  try {
    return bridge.run(Array.isArray(args) ? args : []);
  } catch (error) {
    return {
      status: 1,
      stdout: '',
      stderr: String(error && error.message ? error.message : error),
      payload: {
        ok: false,
        type: 'model_router_core_error',
        error: String(error && error.message ? error.message : error)
      }
    };
  }
}

function printOut(out) {
  if (!out) return;
  if (out.stdout) {
    process.stdout.write(out.stdout);
  } else if (out.payload) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  if (out.stderr) process.stderr.write(String(out.stderr));
}

function readAllowlistLocals() {
  const config = readJson(CONFIG_PATH, {});
  const allow = Array.isArray(config && config.routing && config.routing.spawn_model_allowlist)
    ? config.routing.spawn_model_allowlist
    : [];
  return allow.map((row) => String(row || '').trim()).filter((row) => isLocalOllamaModel(row));
}

function runCompatCommand(argv = []) {
  const parsed = parseArgs(argv);
  const cmd = String(parsed._[0] || 'status').toLowerCase();

  if (cmd === 'probe') {
    const model = String(parsed.model || '').trim();
    const payload = {
      ok: true,
      type: 'model_router_probe',
      model,
      banned: isBanned(model),
      health: health(model, true)
    };
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return 0;
  }

  if (cmd === 'probe-all') {
    const locals = readAllowlistLocals();
    const payload = {
      ok: true,
      type: 'model_router_probe_all',
      count: locals.length,
      models: locals.map((model) => ({
        model,
        banned: isBanned(model),
        health: health(model, true)
      }))
    };
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return 0;
  }

  if (cmd === 'bans') {
    process.stdout.write(`${JSON.stringify({ ok: true, type: 'model_router_bans', bans: bansMap() })}\n`);
    return 0;
  }

  if (cmd === 'unban') {
    const model = String(parsed.model || '').trim();
    const bans = bansMap();
    if (model) delete bans[model];
    writeJson(BANS_PATH, bans);
    process.stdout.write(`${JSON.stringify({ ok: true, type: 'model_router_unban', model, bans })}\n`);
    return 0;
  }

  if (cmd === 'stats' || cmd === 'doctor' || cmd === 'cache-summary' || cmd === 'hardware-plan' || cmd === 'warmup') {
    const out = runCore(['status']);
    printOut(out);
    return Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  }

  process.stdout.write(`${JSON.stringify({ ok: false, type: 'model_router_error', error: `unknown_command:${cmd}` })}\n`);
  return 2;
}

if (require.main === module) {
  const raw = process.argv.slice(2);
  const mapped = coreRouteArgs(raw);
  if (mapped) {
    const out = runCore(mapped);
    printOut(out);
    process.exit(Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1);
  }
  process.exit(runCompatCommand(raw));
}

module.exports = {
  lane: bridge.lane,
  run: (args = []) => {
    const mapped = coreRouteArgs(Array.isArray(args) ? args : []);
    return mapped ? runCore(mapped) : runCore(['status']);
  },
  health,
  isLocalOllamaModel,
  isBanned
};
