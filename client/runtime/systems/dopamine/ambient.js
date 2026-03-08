#!/usr/bin/env node
'use strict';

const path = require('path');
const { runDopamineAmbientCommand } = require('../../lib/spine_conduit_bridge');

const ROOT = path.resolve(__dirname, '..', '..');

function normalizeGateDegraded(out) {
  if (!out || !out.payload || out.payload.gate_active !== true) return out;
  return {
    ...out,
    ok: true,
    status: 0,
    payload: {
      ok: true,
      blocked: true,
      type: 'dopamine_ambient_status',
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
  const routed = Array.isArray(args) && args.length > 0 ? args : ['status'];
  const command = String(routed[0] || 'status').trim().toLowerCase();
  const timeoutMs = Math.max(
    1000,
    Number(
      opts.timeoutMs
      || process.env.PROTHEUS_DOPAMINE_AMBIENT_TIMEOUT_MS
      || (command === 'status' ? 15000 : 60000)
    ) || (command === 'status' ? 15000 : 60000)
  );
  const stdioTimeoutMs = Math.max(
    1000,
    Number(
      opts.stdioTimeoutMs
      || process.env.PROTHEUS_DOPAMINE_AMBIENT_STDIO_TIMEOUT_MS
      || process.env.PROTHEUS_CONDUIT_STDIO_TIMEOUT_MS
      || (command === 'status' ? 8000 : 12000)
    ) || (command === 'status' ? 8000 : 12000)
  );
  const out = await runDopamineAmbientCommand(routed, {
    cwdHint: opts.cwdHint || ROOT,
    timeoutMs,
    stdioTimeoutMs: Math.min(timeoutMs, stdioTimeoutMs)
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
