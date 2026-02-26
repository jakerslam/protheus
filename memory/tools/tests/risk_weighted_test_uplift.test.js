#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const assert = require('assert');

const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');

const LANE_RULES = [
  {
    lane: 'admission',
    minFiles: 2,
    files: [
      'memory/tools/tests/proposal_queue.test.js',
      'memory/tools/tests/directive_gate.test.js',
      'memory/tools/tests/autonomy_policy_hold_classification.test.js'
    ],
    tokens: [/blocked/, /filtered/, /policy_hold/, /reject/, /admission/]
  },
  {
    lane: 'mutation_safety',
    minFiles: 2,
    files: [
      'memory/tools/tests/mutation_safety_kernel.test.js',
      'memory/tools/tests/quorum_validator.test.js',
      'memory/tools/tests/improvement_controller_two_phase.test.js'
    ],
    tokens: [/mutation/, /quorum/, /rollback/, /blocked/, /safety/]
  },
  {
    lane: 'rollback',
    minFiles: 2,
    files: [
      'memory/tools/tests/autonomy_actionability_rollback_guard.test.js',
      'memory/tools/tests/improvement_controller_two_phase.test.js',
      'memory/tools/tests/workflow_executor.test.js'
    ],
    tokens: [/rollback/, /revert/, /retry/, /timeout/, /fail/]
  },
  {
    lane: 'emergency_stop',
    minFiles: 2,
    files: [
      'memory/tools/tests/emergency_stop.test.js',
      'memory/tools/tests/emergency_stop_cli.test.js',
      'memory/tools/tests/route_task_emergency_stop.test.js'
    ],
    tokens: [/emergency_stop/, /engage/, /release/, /blocked/]
  },
  {
    lane: 'policy_root',
    minFiles: 2,
    files: [
      'memory/tools/tests/policy_rootd_lease.test.js',
      'memory/tools/tests/strategy_mode_governor_policy_root.test.js',
      'memory/tools/tests/improvement_controller_policy_root.test.js'
    ],
    tokens: [/policy_root/, /lease/, /denied/, /blocked/, /approved/]
  }
];

function readLower(relPath) {
  const absPath = path.join(REPO_ROOT, relPath);
  if (!fs.existsSync(absPath)) return null;
  return fs.readFileSync(absPath, 'utf8').toLowerCase();
}

function run() {
  for (const rule of LANE_RULES) {
    const contents = rule.files
      .map((f) => ({ file: f, body: readLower(f) }))
      .filter((row) => row.body != null);

    assert.ok(
      contents.length >= rule.minFiles,
      `${rule.lane}: expected at least ${rule.minFiles} test files, found ${contents.length}`
    );

    const tokenHits = rule.tokens.filter((token) =>
      contents.some((row) => token.test(row.body))
    ).length;

    assert.ok(
      tokenHits >= 2,
      `${rule.lane}: expected at least two lane tokens to be present across tests`
    );

    const denyBranch = contents.some((row) => /(blocked|denied|reject|fail|error)/.test(row.body));
    const allowBranch = contents.some((row) => /(allow|approved|pass|mode_changed|ok)/.test(row.body));

    assert.ok(denyBranch, `${rule.lane}: missing deny-path assertions/signals`);
    assert.ok(allowBranch, `${rule.lane}: missing allow-path assertions/signals`);
  }

  console.log('risk_weighted_test_uplift.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`risk_weighted_test_uplift.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
