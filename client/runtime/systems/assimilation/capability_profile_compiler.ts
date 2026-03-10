#!/usr/bin/env node
// @ts-nocheck
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// TypeScript compatibility shim only.
const mod = require('./capability_profile_compiler.js');
if (require.main === module) mod.run(process.argv.slice(2));
module.exports = mod;

const { createConduitLaneModule } = require("../../lib/direct_conduit_lane_bridge.js");
const __directConduitLane = createConduitLaneModule("SYSTEMS_ASSIMILATION_CAPABILITY_PROFILE_COMPILER");
void __directConduitLane;
