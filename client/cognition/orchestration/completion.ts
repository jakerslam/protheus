#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: orchestration (control-plane cognition); this file is a thin bridge.
const path = require('node:path');
const impl = require(
  path.join(__dirname, '..', '..', '..', 'orchestration', 'scripts', 'cognition', 'completion.ts'),
);

module.exports = impl;
