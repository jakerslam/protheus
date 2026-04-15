#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::protheus-setup-wizard (authoritative operator UX bridge).

const impl = require('../../../../adapters/runtime/protheus_setup_wizard.ts');

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

async function main(argv = process.argv.slice(2)) {
  const status = await Promise.resolve(impl.main(normalizeArgs(argv)));
  return Number.isFinite(Number(status)) ? Number(status) : 1;
}

if (require.main === module) {
  Promise.resolve(main(process.argv.slice(2)))
    .then((code) => process.exit(Number.isFinite(Number(code)) ? Number(code) : 0))
    .catch((err) => {
      process.stderr.write(
        `${JSON.stringify({
          ok: false,
          type: 'protheus_setup_wizard',
          error: String(err && err.message ? err.message : err),
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
