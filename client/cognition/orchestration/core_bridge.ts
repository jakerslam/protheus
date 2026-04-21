#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (control-plane cognition); this file is a thin bridge.
const path = require('node:path');
const impl = require(
  path.join(__dirname, '..', '..', '..', 'surface', 'orchestration', 'scripts', 'cognition', 'core_bridge.ts'),
);

module.exports = impl;
