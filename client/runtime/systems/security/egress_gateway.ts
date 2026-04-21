#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::egress-gateway-kernel (authoritative).

const { bindCompatibilityBridgeModule } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindCompatibilityBridgeModule('../../../lib/egress_gateway.ts', module);
