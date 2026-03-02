#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-031
 * Legion Geas Protocol
 */

const path = require('path');
const { ROOT } = require('../../lib/queued_backlog_runtime');
const { runLaneCli } = require('../../lib/backlog_lane_cli');

const POLICY_PATH = process.env.LEGION_GEAS_PROTOCOL_POLICY_PATH
  ? path.resolve(process.env.LEGION_GEAS_PROTOCOL_POLICY_PATH)
  : path.join(ROOT, 'config/legion_geas_protocol_policy.json');

runLaneCli({
  lane_id: 'V3-RACE-031',
  title: 'Legion Geas Protocol',
  type: 'legion_geas_protocol',
  default_action: 'enforce',
  script_label: 'systems/security/legion_geas_protocol.js',
  policy_path: POLICY_PATH,
  default_policy: {
    version: '1.0',
    enabled: true,
    strict_default: true,
    checks: [
    {
        "id": "cryptographic_lease_manager",
        "description": "Short-lived cryptographic lease manager active"
    },
    {
        "id": "behavior_continuity_validation",
        "description": "Three-factor validation enforced"
    },
    {
        "id": "self_destruct_on_violation",
        "description": "Hard self-destruct on lease breach wired"
    },
    {
        "id": "phoenix_handoff_contract",
        "description": "Phoenix handoff resumes inherited tactical state"
    }
],
    paths: {
      state_path: 'state/security/legion_geas_protocol/state.json',
      latest_path: 'state/security/legion_geas_protocol/latest.json',
      receipts_path: 'state/security/legion_geas_protocol/receipts.jsonl',
      history_path: 'state/security/legion_geas_protocol/history.jsonl'
    }
  }
});
