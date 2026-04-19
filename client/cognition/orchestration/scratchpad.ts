#!/usr/bin/env node
'use strict';

const impl = require('../../../surface/orchestration/scripts/cognition/scratchpad.ts');

if (require.main === module && typeof impl.run === 'function') {
  const out = impl.run(process.argv.slice(2));
  if (typeof out === 'number') {
    process.exit(out);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out && out.ok ? 0 : 1);
}

module.exports = impl;
