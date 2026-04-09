#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin config bridge over metagpt-bridge)

const bridge = require('../../client/runtime/systems/workflow/metagpt_bridge.ts');

function simulatePr(payload = {}) {
  return bridge.simulatePr({
    bridge_path: 'adapters/protocol/metagpt_config_bridge.ts',
    ...payload,
  });
}

function ingestConfig(payload = {}) {
  return bridge.ingestConfig({
    bridge_path: 'adapters/protocol/metagpt_config_bridge.ts',
    ...payload,
  });
}

module.exports = {
  simulatePr,
  ingestConfig,
};
