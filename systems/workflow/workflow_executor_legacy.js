#!/usr/bin/env node
'use strict';

const payload = {
  ok: false,
  retired: true,
  error: 'legacy_retired:workflow_executor',
  replacement: 'protheus-ops workflow-executor'
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
