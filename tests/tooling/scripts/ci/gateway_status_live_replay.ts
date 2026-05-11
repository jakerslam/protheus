#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/regression (read-only Gateway status live replay)

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const root = process.cwd();
const policyPath = path.join(root, 'validation/regression/fixtures/gateway_idempotence/gateway_status_live_replay_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const started = Date.now();
const [cmd, ...args] = policy.command;
const run = spawnSync(cmd, args, { cwd: root, encoding: 'utf8', timeout: policy.timeout_ms, maxBuffer: 1024 * 1024 });
const durationMs = Date.now() - started;
const stdout = run.stdout || '';
const stderr = run.stderr || '';
const combined = `${stdout}\n${stderr}`.toLowerCase();
const matchedTokens = (policy.required_output_tokens_any || []).filter((token) => combined.includes(String(token).toLowerCase()));
const timedOut = Boolean(run.error && run.error.code === 'ETIMEDOUT');
const securityGateBlocked = combined.includes('security_gate_blocked') || combined.includes('embedded_security_checker_not_linked_use_cargo');
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'gateway_status_live_replay',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  command: policy.command,
  timeout_ms: policy.timeout_ms,
  duration_ms: durationMs,
  exit_code: typeof run.status === 'number' ? run.status : null,
  signal: run.signal || null,
  timed_out: timedOut,
  output_token_match_count: matchedTokens.length,
  matched_tokens: matchedTokens,
  ok: !timedOut && matchedTokens.length > 0,
  diagnostic: timedOut
    ? 'gateway_status_live_replay_timeout'
    : securityGateBlocked
      ? 'gateway_status_blocked_by_security_checker_linkage'
      : matchedTokens.length > 0
        ? 'gateway_status_live_replay_observed_actionable_output'
        : 'gateway_status_live_replay_unrecognized_output',
  severity: timedOut || securityGateBlocked ? 'yellow' : 'pass',
  root_cause_hypothesis: securityGateBlocked
    ? 'Gateway status is blocked before read-only diagnostics because the embedded security checker is not linked and cargo fallback is disabled.'
    : timedOut
      ? 'Gateway status did not return within the bounded replay budget.'
      : 'Gateway status returned recognizable operator output.',
  next_actions: securityGateBlocked ? [
    'Link the embedded security checker into the gateway status dispatch path, or explicitly allow the governed cargo fallback for status-only diagnostics.',
    'Keep gateway status read-only and bounded; do not require restart/start recovery just to inspect status.'
  ] : [],
  stdout_tail: stdout.slice(-4000),
  stderr_tail: stderr.slice(-4000)
};
fs.mkdirSync(path.dirname(path.join(root, policy.report_path)), { recursive: true });
fs.writeFileSync(path.join(root, policy.report_path), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
process.exit(0);
