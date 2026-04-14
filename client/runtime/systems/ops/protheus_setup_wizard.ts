#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::protheus-setup-wizard (authoritative operator UX bridge).

const mod = require('../../../../adapters/runtime/protheus_setup_wizard.ts');

function normalizeExitCode(value, fallback = 1) {
  if (Number.isFinite(Number(value))) return Number(value);
  return fallback;
}

async function run(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv) ? argv.map((token) => String(token || '')) : [];
  return normalizeExitCode(await mod.main(args), 0);
}

if (require.main === module) {
  Promise.resolve(run(process.argv.slice(2)))
    .then((code) => process.exit(Number.isFinite(code) ? code : 0))
    .catch((err) => {
      process.stderr.write(
        `${JSON.stringify({
          ok: false,
          type: 'protheus_setup_wizard',
          error: mod.cleanText(err && err.message ? err.message : err, 220)
        })}\n`
      );
      process.exit(1);
    });
}

module.exports = {
  ...mod,
  run
};
