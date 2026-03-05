#!/usr/bin/env node
'use strict';

const payload = {
  ok: false,
  retired: true,
  error: 'legacy_retired:spine',
  replacement: 'protheus-ops spine'
};

process.stderr.write(`${JSON.stringify(payload)}\n`);
process.exit(2);
