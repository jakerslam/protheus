#!/usr/bin/env tsx
// Thin TS entrypoint wrapper; authoritative dashboard runtime lives in Rust core.
const dashboard = require('./infring_dashboard.js');
module.exports = dashboard;

if (require.main === module && dashboard && typeof dashboard.run === 'function') {
  const exitCode = dashboard.run(process.argv.slice(2));
  if (typeof exitCode === 'number') {
    process.exitCode = exitCode;
  }
}
