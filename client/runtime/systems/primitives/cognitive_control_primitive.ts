#!/usr/bin/env node
// @ts-nocheck
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// TypeScript compatibility shim only.
const mod = require('./cognitive_control_primitive.js');
if (require.main === module) mod.run(process.argv.slice(2));
module.exports = mod;

const { createConduitLaneModule } = require("../../lib/direct_conduit_lane_bridge.js");
const __directConduitLane = createConduitLaneModule("SYSTEMS_PRIMITIVES_COGNITIVE_CONTROL_PRIMITIVE");
void __directConduitLane;
