#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const OPS_MANIFEST = path.join(ROOT, 'core', 'layer0', 'ops', 'Cargo.toml');

function readJson(relPath) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, relPath), 'utf8'));
}

function runCargoRouteTest() {
  return spawnSync(
    'cargo',
    ['test', '--manifest-path', OPS_MANIFEST, 'route_edge_swarm_maps_correctly', '--', '--nocapture'],
    {
      cwd: ROOT,
      encoding: 'utf8',
    },
  );
}

function main() {
  const bridgePolicy = readJson('client/runtime/config/mobile_edge_swarm_bridge_policy.json');
  const topPolicy = readJson('client/runtime/config/mobile_ops_top_policy.json');
  const routeSource = fs.readFileSync(
    path.join(ROOT, 'core/layer0/ops/src/infringctl_parts/020-evaluate-dispatch-security.rs'),
    'utf8',
  );

  assert.equal(bridgePolicy.enabled, true);
  assert.equal(bridgePolicy.strict_default, true);
  assert.equal(bridgePolicy.require_provenance_attestation, true);
  assert.equal(bridgePolicy.event_stream.stream, 'spawn.mobile_edge');
  assert.equal(bridgePolicy.paths.latest_path, topPolicy.paths.swarm_latest_path);
  assert(
    routeSource.includes('client/runtime/systems/ops/run_infring_ops.ts'),
    'expected edge swarm route to delegate through the live ops bridge surface'
  );
  assert(
    routeSource.includes('"edge".to_string()') && routeSource.includes('"swarm".to_string()'),
    'expected edge swarm route to preserve domain and subcommand when delegating'
  );

  const routeTest = runCargoRouteTest();
  assert.equal(
    routeTest.status,
    0,
    `route_edge_swarm_maps_correctly failed\nstdout:\n${routeTest.stdout}\nstderr:\n${routeTest.stderr}`
  );

  console.log(JSON.stringify({ ok: true, type: 'mobile_edge_swarm_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
