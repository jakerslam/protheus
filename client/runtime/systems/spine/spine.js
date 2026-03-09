#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::spine (authoritative)
// Client wrapper only routes commands through the conduit lane.
const { runSpineCommand } = require('../../lib/spine_conduit_bridge');
const tsBootstrap = require('../../lib/ts_bootstrap');

if (require.main === module) {
  // Harden against startup probe flakiness in manual CLI usage.
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE = '0';
  process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS =
    process.env.PROTHEUS_CONDUIT_STARTUP_PROBE_TIMEOUT_MS || '8000';
  runSpineCommand(process.argv.slice(2), {
    runContext: 'spine_wrapper',
    stdioTimeoutMs: Number(process.env.PROTHEUS_SPINE_STDIO_TIMEOUT_MS || 25000)
  }).then((out) => {
    if (out && out.payload) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
    if (out && out.stderr) process.stderr.write(out.stderr.endsWith('\n') ? out.stderr : `${out.stderr}\n`);
    const status = Number.isFinite(out && out.status) ? Number(out.status) : 1;
    if (status !== 0) {
      const reason = String((out && out.payload && out.payload.reason) || out && out.stderr || '');
      if (/conduit_/i.test(reason) || /timeout/i.test(reason)) {
        process.env.PROTHEUS_SPINE_TS_COMPAT = '1';
        process.env.PROTHEUS_SPINE_TS_LOCAL_COMPAT = '1';
        tsBootstrap.bootstrap(__filename, module);
        return;
      }
    }
    process.exit(status);
  }).catch((error) => {
    const payload = {
      ok: false,
      type: 'spine_wrapper_error',
      error: String(error && error.message ? error.message : error)
    };
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    process.exit(1);
  });
}

module.exports = {
  run: (args = [], opts = {}) =>
    runSpineCommand(args, {
      runContext: 'spine_wrapper',
      ...opts
    })
};
