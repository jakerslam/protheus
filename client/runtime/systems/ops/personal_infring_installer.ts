#!/usr/bin/env node
'use strict';

const path = require('path');
const { invokeTsModuleAsync } = require('../../lib/in_process_ts_delegate.ts');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const SETUP_WIZARD = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'infring_setup_wizard.ts');

async function main(argv = process.argv.slice(2)) {
  const run = await invokeTsModuleAsync(SETUP_WIZARD, {
    argv,
    cwd: ROOT,
    exportName: 'main',
    teeStdout: true,
    teeStderr: true,
  });
  return Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
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
  main,
};
