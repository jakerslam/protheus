#!/usr/bin/env node
'use strict';

const payload = {
  ok: false,
  retired: true,
  error: 'legacy_retired:state_kernel',
  replacement: 'protheus-ops state-kernel'
};

process.stderr.write(`${JSON.stringify(payload)}\n`);
process.exit(2);
