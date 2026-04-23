#!/usr/bin/env node
/* eslint-disable no-console */
const { spawnSync } = require('node:child_process');
const path = require('node:path');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '../..');

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function runOps(args) {
  return spawnSync(
    'cargo',
    ['run', '-q', '-p', 'infring-ops-core', '--bin', 'infring-ops', '--', 'security-plane', 'truth-seeking-gate', ...args],
    {
      cwd: ROOT,
      encoding: 'utf8',
    },
  );
}

function parseJson(raw) {
  try {
    return JSON.parse(String(raw || '{}'));
  } catch {
    return null;
  }
}

function main() {
  const failures = [];

  try {
    const deny = runOps([
      'evaluate',
      '--claim=I agree with this change',
      '--persona-id=truth_guard',
      '--evidence=',
    ]);
    assert(deny.status === 1, `expected deny exit=1, got ${deny.status}`);
    const payload = parseJson(deny.stdout);
    assert(payload && payload.type === 'truth_seeking_gate_evaluate', 'deny type mismatch');
    assert(payload.decision === 'deny', 'deny decision mismatch');
    assertNoPlaceholderOrPromptLeak(payload, 'truth_gate_security_plane_boundary_test:deny');\n    assertStableToolingEnvelope(payload, 'truth_gate_security_plane_boundary_test:deny');\n    const reasons = Array.isArray(payload.deny_reasons) ? payload.deny_reasons : [];
    assert(reasons.includes('agreement_without_verification_denied'), 'missing deny reason');
  } catch (error) {
    failures.push({ case: 'deny_without_evidence', error: String(error && error.message ? error.message : error) });
  }

  try {
    const allow = runOps([
      'evaluate',
      '--claim=I agree with this change because receipt evidence exists',
      '--persona-id=truth_guard',
      '--evidence=receipt:demo123',
    ]);
    assert(allow.status === 0, `expected allow exit=0, got ${allow.status}`);
    const payload = parseJson(allow.stdout);
    assert(payload && payload.type === 'truth_seeking_gate_evaluate', 'allow type mismatch');
    assert(payload.decision === 'allow', 'allow decision mismatch');
    assert(payload.ok === true, 'allow ok mismatch');\n    assertNoPlaceholderOrPromptLeak(payload, 'truth_gate_security_plane_boundary_test:allow');\n    assertStableToolingEnvelope(payload, 'truth_gate_security_plane_boundary_test:allow');
  } catch (error) {
    failures.push({ case: 'allow_with_evidence', error: String(error && error.message ? error.message : error) });
  }

  if (failures.length > 0) {
    console.error(
      JSON.stringify(
        {
          ok: false,
          type: 'truth_gate_security_plane_boundary_test',
          failures,
        },
        null,
        2,
      ),
    );
    process.exit(1);
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'truth_gate_security_plane_boundary_test',
      },
      null,
      2,
    ),
  );
}

main();
