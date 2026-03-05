#!/usr/bin/env node
'use strict';

const payload = {
  ok: false,
  retired: true,
  error: 'legacy_retired:rust_memory_transition_lane',
  replacement: 'systems/memory/rust/bin rust_memory_transition_lane'
};

if (require.main === module) {
  process.stderr.write(`${JSON.stringify(payload)}\n`);
  process.exit(2);
}

function run() {
  return { ...payload };
}

module.exports = {
  ...payload,
  run
};
