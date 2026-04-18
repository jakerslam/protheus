#!/usr/bin/env node
'use strict';

const { createCompatWorkflowExportBridge } = require('../../lib/compat_target_bridge.ts');
const bridge = createCompatWorkflowExportBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../lib/workflow_graph_bridge.ts',
  loadError: 'workflow_graph_bridge_target_load_failed',
  invalidError: 'workflow_graph_bridge_target_invalid',
  framework: 'workflow_graph',
  bridgePath: 'client/runtime/lib/workflow_graph_bridge.ts',
  bridgeTarget: 'surface/orchestration/scripts/workflow_graph_bridge.ts',
});
bridge.exitIfMain(module);

module.exports = bridge.exported;
