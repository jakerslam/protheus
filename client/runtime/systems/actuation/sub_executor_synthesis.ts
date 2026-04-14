#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops (authoritative runtime-systems execution); this file is a thin bridge wrapper.

const { bindRuntimeSystemModule } = require('../../../../adapters/runtime/runtime_system_bridge.ts');

module.exports = bindRuntimeSystemModule(
  __dirname,
  'sub_executor_synthesis',
  'SYSTEMS-ACTUATION-SUB_EXECUTOR_SYNTHESIS',
  module,
);
