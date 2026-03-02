#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-032
 * Complexity Warden Meta-Organ
 */

const path = require('path');
const { ROOT } = require('../../../lib/queued_backlog_runtime');
const { runLaneCli } = require('../../../lib/backlog_lane_cli');

const POLICY_PATH = process.env.COMPLEXITY_WARDEN_META_ORGAN_POLICY_PATH
  ? path.resolve(process.env.COMPLEXITY_WARDEN_META_ORGAN_POLICY_PATH)
  : path.join(ROOT, 'config/complexity_warden_meta_organ_policy.json');

runLaneCli({
  lane_id: 'V3-RACE-032',
  title: 'Complexity Warden Meta-Organ',
  type: 'complexity_warden_meta_organ',
  default_action: 'score',
  script_label: 'systems/fractal/warden/complexity_warden_meta_organ.js',
  policy_path: POLICY_PATH,
  default_policy: {
    version: '1.0',
    enabled: true,
    strict_default: true,
    checks: [
    {
        "id": "warden_scoring_core",
        "description": "Complexity scoring core computes normalized dimensions",
        "file_must_exist": "systems/fractal/warden/README.md"
    },
    {
        "id": "complexity_budget_enforcement",
        "description": "Complexity budget and soul-tax enforcement active"
    },
    {
        "id": "organ_contract_validation",
        "description": "Fractal contract validation lane active"
    },
    {
        "id": "weekly_simplification_cycle",
        "description": "Scheduled simplification sprint lane active"
    }
],
    paths: {
      state_path: 'state/fractal/complexity_warden_meta_organ/state.json',
      latest_path: 'state/fractal/complexity_warden_meta_organ/latest.json',
      receipts_path: 'state/fractal/complexity_warden_meta_organ/receipts.jsonl',
      history_path: 'state/fractal/complexity_warden_meta_organ/history.jsonl'
    }
  }
});
