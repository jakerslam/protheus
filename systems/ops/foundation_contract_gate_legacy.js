#!/usr/bin/env node
'use strict';

const payload = {
  ok: false,
  retired: true,
  error: 'legacy_retired:foundation_contract_gate',
  replacement: 'protheus-ops foundation-contract-gate'
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
