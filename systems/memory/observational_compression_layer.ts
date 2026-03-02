#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-164
 * observational_compression_layer lane.
 */

const path = require('path');
const { normalizeToken } = require(path.join(__dirname, '..', '..', 'lib', 'queued_backlog_runtime.js'));
const { runStandardLane } = require(path.join(__dirname, '..', '..', 'lib', 'upgrade_lane_runtime.js'));

const POLICY_PATH = process.env.V3_RACE_164_POLICY_PATH
  ? path.resolve(process.env.V3_RACE_164_POLICY_PATH)
  : path.join(__dirname, '..', '..', 'config', 'observational_compression_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/memory/observational_compression_layer.js configure --owner=<owner_id> [--profile=default]');
  console.log('  node systems/memory/observational_compression_layer.js execute --owner=<owner_id> [--task=default] [--risk-tier=2]');
  console.log('  node systems/memory/observational_compression_layer.js status [--owner=<owner_id>]');
}

runStandardLane({
  lane_id: 'V3-RACE-164',
  script_rel: 'systems/memory/observational_compression_layer.js',
  policy_path: POLICY_PATH,
  stream: 'memory.observational_compression',
  paths: {
    memory_dir: 'memory/observations',
    adaptive_index_path: 'adaptive/observations/index.json',
    events_path: 'state/memory\/observational_compression/events.jsonl',
    latest_path: 'state/memory\/observational_compression/latest.json',
    receipts_path: 'state/memory\/observational_compression/receipts.jsonl'
  },
  usage,
  handlers: {
    execute(policy, args, ctx) {
      const task = normalizeToken(args.task || args.mode || 'default', 120) || 'default';
      return ctx.cmdRecord(policy, {
        ...args,
        event: 'observational_compression_layer_execute',
        payload_json: JSON.stringify({
          lane_id: 'V3-RACE-164',
          task,
          guarded_execution: true,
          bounded_risk_tier: true,
          deterministic_receipts: true
        })
      });
    }
  }
});
