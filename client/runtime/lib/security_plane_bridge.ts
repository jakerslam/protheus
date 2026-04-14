'use strict';
export {};

// Layer ownership: core/layer1/security (authoritative)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const BRIDGE_PATH = 'client/runtime/lib/security_plane_bridge.ts';
const LANE_ID = 'security-plane';

function normalizeTool(tool: unknown) {
  return String(tool == null ? '' : tool).trim().toLowerCase();
}

function normalizeArgs(args: string[] = []) {
  return Array.isArray(args) ? args.map((value) => String(value)) : [];
}

function runSecurityPlane(tool: unknown, args: string[] = []) {
  const lane = createOpsLaneBridge(__dirname, 'security_plane', LANE_ID);
  const normalizedTool = normalizeTool(tool);
  return lane.run([normalizedTool, ...normalizeArgs(args)]);
}

function runSecurityPlaneCli(tool: unknown, args: string[] = []) {
  const lane = createOpsLaneBridge(__dirname, 'security_plane', LANE_ID);
  const normalizedTool = normalizeTool(tool);
  lane.runCli([normalizedTool, ...normalizeArgs(args)]);
}

module.exports = {
  BRIDGE_PATH,
  LANE_ID,
  normalizeTool,
  normalizeArgs,
  runSecurityPlane,
  runSecurityPlaneCli
};
