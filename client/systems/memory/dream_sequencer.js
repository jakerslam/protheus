#!/usr/bin/env node
'use strict';

const path = require('path');
const { spawnSync } = require('child_process');

const RUNTIME_ENTRY = path.join(__dirname, '..', '..', 'runtime', 'systems', 'memory', 'dream_sequencer.js');

if (require.main === module) {
  const out = spawnSync(process.execPath, [RUNTIME_ENTRY, ...process.argv.slice(2)], {
    stdio: 'inherit',
    env: process.env,
    cwd: process.cwd()
  });
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

module.exports = require(RUNTIME_ENTRY);
