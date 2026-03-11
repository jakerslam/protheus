'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

function normalizeTool(tool: unknown) {
  return String(tool == null ? '' : tool).trim().toLowerCase();
}

function runSecurityPlane(tool: unknown, args: string[] = []) {
  const lane = createOpsLaneBridge(__dirname, 'security_plane', 'security-plane');
  const normalizedTool = normalizeTool(tool);
  return lane.run([normalizedTool, ...args]);
}

function runSecurityPlaneCli(tool: unknown, args: string[] = []) {
  const lane = createOpsLaneBridge(__dirname, 'security_plane', 'security-plane');
  const normalizedTool = normalizeTool(tool);
  lane.runCli([normalizedTool, ...args]);
}

module.exports = {
  runSecurityPlane,
  runSecurityPlaneCli
};
