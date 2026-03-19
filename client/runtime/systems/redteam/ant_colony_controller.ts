#!/usr/bin/env node
'use strict';

const path = require('path');
const runtimeHelper = require('../../lib/legacy_retired_wrapper.ts');

const laneId = runtimeHelper.laneIdFromRuntimePath(__filename);
const mod = runtimeHelper.createLegacyRetiredModule(__dirname, path.basename(__filename), laneId);

if (require.main === module) runtimeHelper.runAsMain(mod, process.argv.slice(2));
module.exports = mod;
