#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin protocol bridge over workflow_graph-bridge)

const bridge = require('../../client/runtime/systems/workflow/workflow_graph_bridge.ts');

function recordTrace(payload = {}) {
  return bridge.recordTrace({
    bridge_path: 'adapters/protocol/workflow_graph_trace_bridge.ts',
    ...payload,
  });
}

function streamGraph(payload = {}) {
  return bridge.streamGraph(payload);
}

module.exports = {
  recordTrace,
  streamGraph,
};
