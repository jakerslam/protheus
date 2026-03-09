'use strict';

// Layer ownership: core/layer1/security (authoritative)

const { createOpsLaneBridge } = require('./rust_lane_bridge');

function normalizeTool(tool) {
  return String(tool == null ? '' : tool).trim().toLowerCase();
}

function runSecurityPlane(tool, args = []) {
  const lane = createOpsLaneBridge(__dirname, 'security_plane', 'security-plane');
  const normalizedTool = normalizeTool(tool);
  return lane.run([normalizedTool, ...args]);
}

function runSecurityPlaneCli(tool, args = []) {
  const lane = createOpsLaneBridge(__dirname, 'security_plane', 'security-plane');
  const normalizedTool = normalizeTool(tool);
  lane.runCli([normalizedTool, ...args]);
}

module.exports = {
  runSecurityPlane,
  runSecurityPlaneCli
};
