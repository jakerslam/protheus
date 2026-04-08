#!/usr/bin/env node
'use strict';

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const TS_ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const SETUP_WIZARD = path.join(ROOT, 'client', 'runtime', 'systems', 'ops', 'infring_setup_wizard.ts');

const run = spawnSync(process.execPath, [TS_ENTRYPOINT, SETUP_WIZARD, ...process.argv.slice(2)], {
  cwd: ROOT,
  env: { ...process.env },
  stdio: 'inherit'
});

const code = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
process.exit(code);
