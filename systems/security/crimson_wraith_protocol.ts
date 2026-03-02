#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-031B
 * Crimson Wraith Protocol
 */

const path = require('path');
const { ROOT } = require('../../lib/queued_backlog_runtime');
const { runLaneCli } = require('../../lib/backlog_lane_cli');

const POLICY_PATH = process.env.CRIMSON_WRAITH_PROTOCOL_POLICY_PATH
  ? path.resolve(process.env.CRIMSON_WRAITH_PROTOCOL_POLICY_PATH)
  : path.join(ROOT, 'config/crimson_wraith_protocol_policy.json');

runLaneCli({
  lane_id: 'V3-RACE-DEF-031B',
  title: 'Crimson Wraith Protocol',
  type: 'crimson_wraith_protocol',
  default_action: 'mission',
  script_label: 'systems/security/crimson_wraith_protocol.js',
  policy_path: POLICY_PATH,
  default_policy: {
    version: '1.0',
    enabled: true,
    strict_default: true,
    checks: [
    {
        "id": "one_shot_spawn_type",
        "description": "Single-mission crimson_wraith spawn type available"
    },
    {
        "id": "hard_timeout_enforced",
        "description": "Mission timeout and TTL contracts enforced"
    },
    {
        "id": "decoy_trap_templates",
        "description": "Decoy and trap mission templates available"
    },
    {
        "id": "irreversible_termination",
        "description": "No-lineage respawn behavior enforced"
    }
],
    paths: {
      state_path: 'state/security/crimson_wraith_protocol/state.json',
      latest_path: 'state/security/crimson_wraith_protocol/latest.json',
      receipts_path: 'state/security/crimson_wraith_protocol/receipts.jsonl',
      history_path: 'state/security/crimson_wraith_protocol/history.jsonl'
    }
  }
});
