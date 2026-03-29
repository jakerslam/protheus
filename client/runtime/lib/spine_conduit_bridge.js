'use strict';

// Thin client wrapper only: authority moved into core/layer0/ops::spine_conduit_bridge_kernel.

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.js');

const bridge = createOpsLaneBridge(
  __dirname,
  'spine_conduit_bridge_kernel',
  'spine-conduit-bridge-kernel',
  {
    preferLocalCore: true,
    inheritStdio: false
  }
);

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

function runDomainBridge(domain, commandArgs, opts = {}) {
  const normalizedDomain = String(domain || '').trim();
  if (!normalizedDomain) {
    return {
      ok: false,
      status: 1,
      stdout: '',
      stderr: 'missing_domain',
      payload: buildErrorPayload('ops_domain_conduit_bridge_error', 'missing_domain', 'core_local'),
      routed_via: 'core_local'
    };
  }
  const passArgs = [
    'run-domain',
    `--domain=${normalizedDomain}`,
    ...(normalizedDomain.toLowerCase() === 'spine' ? ['--normalize-spine=1'] : []),
    '--',
    ...asArray(commandArgs)
  ];
  return bridge.run(passArgs);
}

async function runSpineCommand(commandArgs, _opts = {}) {
  const out = runDomainBridge('spine', commandArgs);
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

