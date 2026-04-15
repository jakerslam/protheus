#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: apps/agent-holo-viz (authoritative)

const fs = require('fs');
const path = require('path');

const CANONICAL_VISUALIZER_CLIENT_REL = 'client/runtime/local/workspaces/agent-holo-viz/client/app.ts';

function resolveCanonicalVisualizerClientPath(workspaceRoot = process.env.INFRING_WORKSPACE || process.cwd()) {
  const root = String(workspaceRoot || process.cwd()).trim() || process.cwd();
  return path.resolve(root, CANONICAL_VISUALIZER_CLIENT_REL);
}

function hasCanonicalVisualizerClient(workspaceRoot = process.env.INFRING_WORKSPACE || process.cwd()) {
  return fs.existsSync(resolveCanonicalVisualizerClientPath(workspaceRoot));
}

function visualizerCompatibilityContract(workspaceRoot = process.env.INFRING_WORKSPACE || process.cwd()) {
  return {
    canonical_client_path: resolveCanonicalVisualizerClientPath(workspaceRoot),
    canonical_client_present: hasCanonicalVisualizerClient(workspaceRoot),
  };
}

module.exports = {
  CANONICAL_VISUALIZER_CLIENT_REL,
  hasCanonicalVisualizerClient,
  resolveCanonicalVisualizerClientPath,
  visualizerCompatibilityContract,
};
