#!/usr/bin/env tsx
// Thin dashboard UI host compatibility shim only.

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::infring-dashboard (authoritative dashboard host/runtime surface).

const mod = require('../../../../adapters/runtime/infring_dashboard.ts');

if (require.main === module) {
  process.on('uncaughtException', (error) => {
    if (mod.isTransientSocketError && mod.isTransientSocketError(error)) {
      console.error(mod.cleanText(`dashboard_host_socket:${error.code || 'unknown'}`, 280));
      return;
    }
    console.error(mod.cleanText(error && error.message ? error.message : String(error), 280));
    process.exitCode = 1;
  });
  Promise.resolve(mod.run(process.argv.slice(2)))
    .then((exitCode) => {
      if (typeof exitCode === 'number') process.exitCode = exitCode;
    })
    .catch((error) => {
      console.error(mod.cleanText(error && error.message ? error.message : String(error), 280));
      process.exitCode = 1;
    });
}

module.exports = mod;
