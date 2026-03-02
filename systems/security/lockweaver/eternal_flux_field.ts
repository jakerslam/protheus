#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-DEF-026
 * Lockweaver Eternal Flux Field
 */

const path = require('path');
const { ROOT } = require('../../../lib/queued_backlog_runtime');
const { runLaneCli } = require('../../../lib/backlog_lane_cli');

const POLICY_PATH = process.env.LOCKWEAVER_ETERNAL_FLUX_FIELD_POLICY_PATH
  ? path.resolve(process.env.LOCKWEAVER_ETERNAL_FLUX_FIELD_POLICY_PATH)
  : path.join(ROOT, 'config/lockweaver_eternal_flux_field_policy.json');

runLaneCli({
  lane_id: 'V3-RACE-DEF-026',
  title: 'Lockweaver Eternal Flux Field',
  type: 'lockweaver_eternal_flux_field',
  default_action: 'flux',
  script_label: 'systems/security/lockweaver/eternal_flux_field.js',
  policy_path: POLICY_PATH,
  default_policy: {
    version: '1.0',
    enabled: true,
    strict_default: true,
    checks: [
    {
        "id": "origin_lock_verification",
        "description": "Origin lock verify and reseed loop active",
        "file_must_exist": "systems/security/lockweaver/README.md"
    },
    {
        "id": "mutation_cycle_receipts",
        "description": "Cycle receipts publish to authoritative stream"
    },
    {
        "id": "fractal_rate_controller",
        "description": "Threat adaptive cadence controller configured"
    },
    {
        "id": "scope_exclusion_invariant",
        "description": "Open platform/habits/skills exclusion enforced"
    }
],
    paths: {
      state_path: 'state/security/lockweaver_eternal_flux_field/state.json',
      latest_path: 'state/security/lockweaver_eternal_flux_field/latest.json',
      receipts_path: 'state/security/lockweaver_eternal_flux_field/receipts.jsonl',
      history_path: 'state/security/lockweaver_eternal_flux_field/history.jsonl'
    }
  }
});
