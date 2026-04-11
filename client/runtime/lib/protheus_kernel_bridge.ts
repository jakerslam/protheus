#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::protheus-kernel-bridge (authoritative shared transport helper).

module.exports = require('../../../adapters/runtime/protheus_kernel_bridge.ts');
