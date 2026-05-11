#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (Rust crate cargo-check evidence report)

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/rust_crate_cargo_check_evidence_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const execute = process.argv.includes('--execute=1') || process.argv.includes('--execute');
const rows = [];

for (const row of policy.sample_manifests || []) {
  const manifestPath = path.join(root, row.path);
  const command = ['cargo', 'check', '--manifest-path', row.path, '--quiet'];
  if (!execute) {
    rows.push({
      path: row.path,
      reason: row.reason,
      evidence_status: 'planned_not_executed',
      command,
      duration_ms: 0,
      exit_code: null,
      stdout_tail: '',
      stderr_tail: '',
    });
    continue;
  }
  const started = Date.now();
  const run = fs.existsSync(manifestPath)
    ? spawnSync(command[0], command.slice(1), { cwd: root, encoding: 'utf8', timeout: policy.timeout_ms, maxBuffer: 1024 * 1024 })
    : { status: null, stdout: '', stderr: 'manifest missing', error: null, signal: null };
  const timedOut = Boolean(run.error && run.error.code === 'ETIMEDOUT');
  const status = !fs.existsSync(manifestPath)
    ? 'manifest_missing'
    : timedOut
      ? 'cargo_check_timeout'
      : run.status === 0
        ? 'cargo_check_passed'
        : 'cargo_check_failed';
  rows.push({
    path: row.path,
    reason: row.reason,
    evidence_status: status,
    command,
    duration_ms: Date.now() - started,
    exit_code: typeof run.status === 'number' ? run.status : null,
    signal: run.signal || null,
    stdout_tail: String(run.stdout || '').slice(-2000),
    stderr_tail: String(run.stderr || '').slice(-2000),
  });
}

const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'rust_crate_cargo_check_evidence_report',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  execute,
  sample_count: rows.length,
  observed_count: rows.filter((row) => policy.accepted_evidence_statuses.includes(row.evidence_status)).length,
  pass_count: rows.filter((row) => row.evidence_status === 'cargo_check_passed').length,
  fail_count: rows.filter((row) => row.evidence_status === 'cargo_check_failed').length,
  timeout_count: rows.filter((row) => row.evidence_status === 'cargo_check_timeout').length,
  rows,
};
const reportPath = path.join(root, policy.report_path);
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({
  ok: true,
  type: payload.type,
  execute,
  sample_count: payload.sample_count,
  observed_count: payload.observed_count,
  pass_count: payload.pass_count,
  fail_count: payload.fail_count,
  timeout_count: payload.timeout_count,
  report_path: policy.report_path,
}, null, 2));
