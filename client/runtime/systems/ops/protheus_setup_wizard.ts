#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::protheus-setup-wizard (authoritative operator UX bridge).

const mod = require('../../../../adapters/runtime/protheus_setup_wizard.ts');

if (require.main === module) {
  Promise.resolve(mod.main(process.argv.slice(2)))
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

module.exports = mod;
