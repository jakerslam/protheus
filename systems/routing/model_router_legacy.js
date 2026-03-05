#!/usr/bin/env node
'use strict';

const payload = {
  ok: false,
  retired: true,
  error: 'legacy_retired:model_router',
  replacement: 'protheus-ops model-router'
};

process.stderr.write(`${JSON.stringify(payload)}\n`);
process.exit(2);
