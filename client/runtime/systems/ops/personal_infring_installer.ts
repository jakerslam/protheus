#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: client/runtime/systems/ops/infring_setup_wizard.ts (allowed operator runtime utility); this file is a thin CLI bridge.

const path = require('path');
const { installTsRequireHook } = require('../../lib/ts_bootstrap.ts');

const target = path.resolve(__dirname, 'infring_setup_wizard.ts');

installTsRequireHook();
const impl = require(target);

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

async function main(argv = process.argv.slice(2)) {
  const outcome = await Promise.resolve(impl.main(normalizeArgs(argv)));
  return Number.isFinite(Number(outcome)) ? Number(outcome) : 1;
}

if (require.main === module) {
  Promise.resolve(main(process.argv.slice(2)))
    .then((code) => process.exit(code))
    .catch((error) => {
      process.stderr.write(
        `${JSON.stringify({
          ok: false,
          type: 'personal_infring_installer',
          error: String(error && error.message ? error.message : error),
        })}\n`
      );
      process.exit(1);
    });
}

module.exports = {
  ...impl,
  normalizeArgs,
  main,
};
