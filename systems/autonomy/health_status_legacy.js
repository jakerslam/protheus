#!/usr/bin/env node
'use strict';

const payload = {
  ok: false,
  retired: true,
  error: 'legacy_retired:health_status',
  replacement: 'protheus-ops health-status'
};

process.stderr.write(`${JSON.stringify(payload)}\n`);
process.exit(2);
