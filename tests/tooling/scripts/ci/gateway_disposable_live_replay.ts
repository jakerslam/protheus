#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/regression (disposable Gateway live replay)

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const root = process.cwd();
const policyPath = path.join(root, 'validation/regression/fixtures/gateway_idempotence/gateway_disposable_live_replay_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const apply = process.env[policy.apply_env] === '1' || policy.default_apply === true;
const results = [];

function compactTail(value) {
  return String(value || '').slice(-2400);
}

function runStep(step) {
  const dryRunSkip = !apply && step.execute_in_dry_run !== true;
  if (dryRunSkip) {
    return {
      id: step.id,
      kind: step.kind,
      mutates: Boolean(step.mutates),
      skipped: true,
      reason: 'dry_run_plan_only',
      ok: true,
    };
  }
  const shouldSkip = step.requires_apply && !apply;
  if (shouldSkip) {
    return {
      id: step.id,
      kind: step.kind,
      mutates: Boolean(step.mutates),
      skipped: true,
      reason: 'requires_explicit_apply',
      ok: true,
    };
  }
  const started = Date.now();
  const [cmd, ...args] = step.command;
  const run = spawnSync(cmd, args, {
    cwd: root,
    encoding: 'utf8',
    timeout: policy.timeout_ms,
    maxBuffer: 1024 * 1024,
    env: process.env,
  });
  const combined = `${run.stdout || ''}\n${run.stderr || ''}`.toLowerCase();
  const matched = (policy.required_output_tokens_any || []).filter((token) => combined.includes(String(token).toLowerCase()));
  const timedOut = Boolean(run.error && run.error.code === 'ETIMEDOUT');
  return {
    id: step.id,
    kind: step.kind,
    mutates: Boolean(step.mutates),
    skipped: false,
    command: step.command,
    duration_ms: Date.now() - started,
    exit_code: typeof run.status === 'number' ? run.status : null,
    signal: run.signal || null,
    timed_out: timedOut,
    output_token_match_count: matched.length,
    matched_tokens: matched,
    ok: !timedOut && (matched.length > 0 || step.kind === 'cleanup'),
    stdout_tail: compactTail(run.stdout),
    stderr_tail: compactTail(run.stderr),
  };
}

try {
  for (const step of policy.steps || []) {
    if (step.always_cleanup) continue;
    results.push(runStep(step));
  }
} finally {
  if (apply) {
    for (const step of (policy.steps || []).filter((row) => row.always_cleanup)) {
      results.push(runStep(step));
    }
  } else {
    for (const step of (policy.steps || []).filter((row) => row.always_cleanup)) {
      results.push({
        id: step.id,
        kind: step.kind,
        mutates: Boolean(step.mutates),
        skipped: true,
        reason: 'cleanup_not_needed_in_dry_run',
        ok: true,
      });
    }
  }
}

const executed = results.filter((row) => !row.skipped);
const skippedMutating = results.filter((row) => row.skipped && row.mutates);
const skippedDryRunPlan = results.filter((row) => row.skipped && row.reason === 'dry_run_plan_only');
const failures = results.filter((row) => row.ok !== true);
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'gateway_disposable_live_replay',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  apply,
  dry_run: !apply,
  host: policy.host,
  port: policy.port,
  ok: failures.length === 0,
  diagnostic: apply
    ? failures.length === 0
      ? 'gateway_disposable_live_replay_passed'
      : 'gateway_disposable_live_replay_failed'
    : 'gateway_disposable_live_replay_dry_run_ready',
  root_cause_hypothesis: apply
    ? failures.length === 0
      ? 'Disposable Gateway start/status/restart/cleanup completed within bounded replay rules.'
      : 'At least one disposable Gateway live replay step failed or timed out.'
    : 'Disposable Gateway live replay is installed but mutating steps require explicit apply to avoid surprising local runtime changes.',
  next_actions: apply
    ? failures.length === 0
      ? []
      : ['Inspect failed step tails and rerun the replay after fixing the bounded Gateway lifecycle issue.']
    : [`Run ${policy.apply_env}=1 node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/gateway_disposable_live_replay.ts when a disposable live start/restart proof is desired.`],
  executed_step_count: executed.length,
  skipped_dry_run_plan_step_count: skippedDryRunPlan.length,
  skipped_mutating_step_count: skippedMutating.length,
  failure_count: failures.length,
  results,
};
fs.mkdirSync(path.dirname(path.join(root, policy.report_path)), { recursive: true });
fs.writeFileSync(path.join(root, policy.report_path), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
process.exit(payload.ok ? 0 : 1);
