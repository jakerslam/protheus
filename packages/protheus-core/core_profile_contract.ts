#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-169
 * Core profile contract lane for user defaults.
 */

const path = require('path');
const { normalizeToken } = require('../../client/runtime/lib/queued_backlog_runtime');
const { runStandardLane } = require('../../client/runtime/lib/upgrade_lane_runtime');
const { sanitizeBridgeArg } = require('../../client/runtime/lib/runtime_system_entrypoint.ts');

const ROOT = path.join(__dirname, '..', '..');
const DEFAULT_POLICY_PATH = path.join(ROOT, 'client', 'runtime', 'config', 'protheus_core_profile_policy.json');

function isPathInsideRoot(candidate, root) {
  const resolvedRoot = path.resolve(root);
  const resolvedTarget = path.resolve(candidate);
  const relative = path.relative(resolvedRoot, resolvedTarget);
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative));
}

function resolvePolicyPath() {
  const envPath = sanitizeBridgeArg(process.env.PROTHEUS_CORE_PROFILE_POLICY_PATH || '', 1024);
  if (!envPath) return DEFAULT_POLICY_PATH;
  const resolved = path.resolve(envPath);
  if (!isPathInsideRoot(resolved, ROOT)) return DEFAULT_POLICY_PATH;
  return resolved;
}

const POLICY_PATH = resolvePolicyPath();

function usage() {
  console.log('Usage:');
  console.log('  node client/runtime/lib/ts_entrypoint.ts packages/protheus-core/core_profile_contract.ts configure --owner=<owner_id> [--mode=lite]');
  console.log('  node client/runtime/lib/ts_entrypoint.ts packages/protheus-core/core_profile_contract.ts bootstrap --owner=<owner_id> [--mode=lite]');
  console.log('  node client/runtime/lib/ts_entrypoint.ts packages/protheus-core/core_profile_contract.ts status [--owner=<owner_id>]');
}

runStandardLane({
  lane_id: 'V3-RACE-169',
  script_rel: 'packages/protheus-core/core_profile_contract.js',
  policy_path: POLICY_PATH,
  stream: 'core.profiles',
  paths: {
    memory_dir: 'client/runtime/local/memory/core_profiles',
    adaptive_index_path: 'client/cognition/adaptive/core_profiles/index.json',
    events_path: 'client/runtime/local/state/core/profiles/events.jsonl',
    latest_path: 'client/runtime/local/state/core/profiles/latest.json',
    receipts_path: 'client/runtime/local/state/core/profiles/receipts.jsonl'
  },
  usage,
  handlers: {
    bootstrap(policy: any, args: any, ctx: any) {
      const mode = normalizeToken(args.mode || 'lite', 40) || 'lite';
      return ctx.cmdRecord(policy, {
        ...args,
        event: 'core_profile_bootstrap',
        payload_json: JSON.stringify({ mode, one_command_starter: true, optional_heavy_layers: false })
      });
    }
  }
});
