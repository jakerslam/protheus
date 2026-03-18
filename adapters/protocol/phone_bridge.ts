#!/usr/bin/env node
'use strict';

// thin desktop shell only

const bridge = require('../../client/runtime/lib/phone_runtime_bridge.ts');

function sensorIntake(payload = {}) {
  return bridge.sensorIntake(payload);
}

function interactionMode(payload = {}) {
  return bridge.interactionMode(payload);
}

function backgroundDaemon(payload = {}) {
  return bridge.backgroundDaemon(payload);
}

module.exports = {
  sensorIntake,
  interactionMode,
  backgroundDaemon,
};
