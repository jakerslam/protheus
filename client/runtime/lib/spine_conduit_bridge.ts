'use strict';

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

function findRepoRoot(startDir) {
  let dir = path.resolve(startDir || process.cwd());
  while (true) {
    const cargo = path.join(dir, 'Cargo.toml');
    const coreOps = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    const legacyOps = path.join(dir, 'crates', 'ops', 'Cargo.toml');
    if (fs.existsSync(cargo) && (fs.existsSync(coreOps) || fs.existsSync(legacyOps))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) return process.cwd();
    dir = parent;
  }
}

function asArray(args) {
  return Array.isArray(args) ? args.map((value) => String(value)) : [];
}

function normalizeSpineArgs(args) {
  const rows = asArray(args);
  if (!rows.length) return ['status'];
  const head = String(rows[0] || '').trim().toLowerCase();
  if (head !== 'run') return rows;

  const modeRaw = String(rows[1] || '').trim().toLowerCase();
  const mode = modeRaw === 'eyes' ? 'eyes' : 'daily';
  const dateToken = String(rows[2] || '').trim();
  const normalized = [mode];
  const hasDate = /^\d{4}-\d{2}-\d{2}$/.test(dateToken);
  if (hasDate) normalized.push(dateToken);
  const restStart = hasDate ? 3 : 2;
  for (let i = restStart; i < rows.length; i += 1) {
    normalized.push(rows[i]);
  }
  return normalized;
}

function buildErrorPayload(type, reason, routedVia = 'core_local') {
  return {
    ok: false,
    type,
    reason: String(reason || '').slice(0, 320) || 'bridge_error',
    routed_via: routedVia
  };
}

function toBridgeResult(out, errorType, fallbackReason = 'bridge_result_unavailable') {
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  const routedVia = out && out.routed_via ? String(out.routed_via) : 'core_local';
  const payload = out && out.payload && typeof out.payload === 'object'
    ? { ...out.payload }
    : buildErrorPayload(errorType, fallbackReason, routedVia);

  if (!payload.type) payload.type = errorType;
  if (!payload.routed_via) payload.routed_via = routedVia;

  if (status !== 0) {
    payload.ok = false;
    if (!payload.reason) {
      payload.reason = String(out && out.stderr ? out.stderr : fallbackReason).slice(0, 320);
    }
  } else if (typeof payload.ok !== 'boolean') {
    payload.ok = true;
  }

  return {
    ok: status === 0 && payload.ok !== false,
    status,
    payload,
    detail: payload,
    response: null,
    routed_via: routedVia,
    stdout: String(out && out.stdout ? out.stdout : ''),
    stderr: String(out && out.stderr ? out.stderr : '')
  };
}

function runDomainBridge(domain, commandArgs) {
  const lane = `spine_conduit_bridge_${String(domain || '').replace(/[^a-zA-Z0-9_-]/g, '_')}`;
  const bridge = createOpsLaneBridge(__dirname, lane, String(domain || '').trim(), {
    preferLocalCore: true,
    inheritStdio: false
  });
  return bridge.run(asArray(commandArgs));
}

async function runSpineCommand(commandArgs, _opts = {}) {
  const out = runDomainBridge('spine', normalizeSpineArgs(commandArgs));
  return toBridgeResult(out, 'spine_conduit_bridge_error', 'spine_bridge_failed');
}

async function runSpineCommandCli(commandArgs, opts = {}) {
  const out = await runSpineCommand(commandArgs, opts);
  if (opts.echoPayload !== false && out.payload) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  if (opts.echoStderr === true && out.stderr) {
    process.stderr.write(out.stderr.endsWith('\n') ? out.stderr : `${out.stderr}\n`);
  }
  process.exit(Number.isFinite(out.status) ? out.status : 1);
}

async function runAttentionCommand(commandArgs, _opts = {}) {
  const out = runDomainBridge('attention-queue', commandArgs);
  return toBridgeResult(out, 'attention_conduit_bridge_error', 'attention_bridge_failed');
}

async function runPersonaAmbientCommand(commandArgs, _opts = {}) {
  const out = runDomainBridge('persona-ambient', commandArgs);
  return toBridgeResult(out, 'persona_ambient_conduit_bridge_error', 'persona_ambient_bridge_failed');
}

async function runDopamineAmbientCommand(commandArgs, _opts = {}) {
  const out = runDomainBridge('dopamine-ambient', commandArgs);
  return toBridgeResult(out, 'dopamine_ambient_conduit_bridge_error', 'dopamine_ambient_bridge_failed');
}

async function runMemoryAmbientCommand(commandArgs, _opts = {}) {
  const out = runDomainBridge('memory-ambient', commandArgs);
  return toBridgeResult(out, 'memory_ambient_conduit_bridge_error', 'memory_ambient_bridge_failed');
}

async function runOpsDomainCommand(domain, commandArgs, _opts = {}) {
  const normalizedDomain = String(domain || '').trim();
  if (!normalizedDomain) {
    return {
      ok: false,
      status: 1,
      payload: buildErrorPayload('ops_domain_conduit_bridge_error', 'missing_domain'),
      detail: null,
      response: null,
      routed_via: 'core_local',
      stdout: '',
      stderr: 'missing_domain'
    };
  }
  const out = runDomainBridge(normalizedDomain, commandArgs);
  return toBridgeResult(out, 'ops_domain_conduit_bridge_error', 'ops_domain_bridge_failed');
}

module.exports = {
  findRepoRoot,
  runAttentionCommand,
  runDopamineAmbientCommand,
  runMemoryAmbientCommand,
  runOpsDomainCommand,
  runPersonaAmbientCommand,
  runSpineCommand,
  runSpineCommandCli
};
