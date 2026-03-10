#!/usr/bin/env node
'use strict';

const path = require('path');

const runtimeHelper = require(path.join(
  __dirname,
  '..',
  '..',
  '..',
  'runtime',
  'lib',
  'legacy_retired_wrapper.js'
));

function normalizeLaneId(raw) {
  return runtimeHelper.normalizeLaneId(raw, 'MEMORY-TEST-LEGACY-RETIRED');
}

function createTestModule(scriptDir, scriptName, laneId) {
  return runtimeHelper.createLegacyRetiredModule(
    scriptDir,
    scriptName,
    normalizeLaneId(laneId)
  );
}

module.exports = {
  createTestModule,
  runAsMain: runtimeHelper.runAsMain,
  normalizeLaneId
};
