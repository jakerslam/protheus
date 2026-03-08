#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');
const fs = require('fs');
const path = require('path');

// Autotest runs frequently trip transient startup probes in cold environments.
if (!process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS) {
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS = '30000';
}
if (!process.env.PROTHEUS_CONDUIT_STDIO_TIMEOUT_MS) {
  process.env.PROTHEUS_CONDUIT_STDIO_TIMEOUT_MS = '90000';
}
if (!process.env.PROTHEUS_CONDUIT_STARTUP_PROBE) {
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
}

const bridge = createOpsLaneBridge(__dirname, 'autotest_controller', 'autotest-controller');

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

function readCachedStatus() {
  const candidates = [
    path.join(process.cwd(), 'client', 'local', 'state', 'ops', 'autotest', 'status.json'),
    path.join(process.cwd(), 'local', 'state', 'ops', 'autotest', 'status.json')
  ];
  for (const candidate of candidates) {
    if (!fs.existsSync(candidate)) continue;
    try {
      const payload = JSON.parse(fs.readFileSync(candidate, 'utf8'));
      if (payload && typeof payload === 'object') {
        return {
          ok: true,
          payload: {
            ...payload,
            cached_status: true,
            status_source: path.relative(process.cwd(), candidate).replace(/\\/g, '/')
          }
        };
      }
    } catch {}
  }
  return { ok: false, payload: null };
}

function runCli(args) {
  const parsed = parseArgs(args);
  const cmd = String(parsed._[0] || '').trim().toLowerCase();
  const forceLive = String(parsed.live || parsed['force-live'] || '').trim() === '1';

  if (cmd === 'status' && !forceLive) {
    const cached = readCachedStatus();
    if (cached.ok) {
      process.stdout.write(`${JSON.stringify(cached.payload)}\n`);
      process.exit(0);
      return;
    }
  }

  const out = bridge.run(args);
  if (!out.ok && cmd === 'status') {
    const timedOut = out && out.payload
      && typeof out.payload.reason === 'string'
      && out.payload.reason.includes('conduit_stdio_timeout:');
    if (timedOut) {
      const cached = readCachedStatus();
      if (cached.ok) {
        cached.payload.degraded = true;
        cached.payload.live_reason = out.payload.reason;
        process.stdout.write(`${JSON.stringify(cached.payload)}\n`);
        process.exit(0);
        return;
      }
    }
  }

  if (out.stdout) process.stdout.write(out.stdout);
  if (out.stderr) process.stderr.write(out.stderr);
  process.exit(Number.isFinite(out.status) ? out.status : 1);
}

if (require.main === module) {
  runCli(process.argv.slice(2));
}

module.exports = {
  lane: bridge.lane,
  run: bridge.run
};
