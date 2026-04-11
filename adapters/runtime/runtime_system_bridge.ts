#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('./ops_lane_bridge.ts');

function normalizeArgs(args = []) {
  return Array.isArray(args) ? args.map((value) => String(value)) : [];
}

function writeBridgeOutput(out) {
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
}

function bridgeStatusCode(out) {
  const parsed = Number(out && out.status);
  return Number.isFinite(parsed) ? parsed : 1;
}

function createRuntimeSystemModule(scriptDir, scriptName, systemId, options = {}) {
  const bridge = createOpsLaneBridge(
    scriptDir,
    scriptName,
    String(options.domain || 'runtime-systems'),
    {
      inheritStdio: options.inheritStdio !== false,
      preferLocalCore: options.preferLocalCore === true,
    },
  );

  function run(args = process.argv.slice(2)) {
    const out = bridge.run([`--system-id=${String(systemId)}`].concat(normalizeArgs(args)));
    writeBridgeOutput(out);
    return out;
  }

  return {
    lane: bridge.lane,
    systemId: String(systemId),
    run,
    statusCode: bridgeStatusCode,
  };
}

function bindRuntimeSystemModule(
  scriptDir,
  scriptName,
  systemId,
  currentModule,
  argv = process.argv.slice(2),
  options = {},
) {
  const mod = createRuntimeSystemModule(scriptDir, scriptName, systemId, options);
  if (currentModule && require.main === currentModule) {
    process.exit(mod.statusCode(mod.run(argv)));
  }
  return mod;
}

module.exports = {
  bindRuntimeSystemModule,
  bridgeStatusCode,
  createRuntimeSystemModule,
  writeBridgeOutput,
};
