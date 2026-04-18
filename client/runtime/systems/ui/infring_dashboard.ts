#!/usr/bin/env tsx
// Thin dashboard UI host compatibility shim only.

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::infring-dashboard (authoritative dashboard host/runtime surface).

const {
  createCompatModuleExportBridge
} = require('../../lib/compat_target_bridge.ts');

const bridge = createCompatModuleExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../adapters/runtime/infring_dashboard.ts',
  loadError: 'infring_dashboard_load_failed',
  invalidError: 'infring_dashboard_invalid'
});
const mod = bridge.exported;

function stderrLine(message) {
  process.stderr.write(String(message || '') + '\n');
}

function cleanStderrMessage(errorLike) {
  if (mod && mod.ok === false) return JSON.stringify(mod);
  const raw = String(errorLike && errorLike.message ? errorLike.message : errorLike || 'unknown_error');
  if (mod && typeof mod.cleanText === 'function') return mod.cleanText(raw, 280);
  return raw.slice(0, 280);
}

if (require.main === module) {
  if (mod && mod.ok === false) {
    stderrLine(cleanStderrMessage(mod));
    process.exitCode = 1;
  } else {
  process.on('uncaughtException', (error) => {
    if (mod.isTransientSocketError && mod.isTransientSocketError(error)) {
      stderrLine(cleanStderrMessage(`dashboard_host_socket:${error.code || 'unknown'}`));
      return;
    }
    stderrLine(cleanStderrMessage(error));
    process.exitCode = 1;
  });
  Promise.resolve(mod.run(process.argv.slice(2)))
    .then((exitCode) => {
      if (typeof exitCode === 'number') process.exitCode = exitCode;
    })
    .catch((error) => {
      stderrLine(cleanStderrMessage(error));
      process.exitCode = 1;
    });
  }
}

module.exports = mod;
