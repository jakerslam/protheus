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
const setupResults = [];

function compactTail(value) {
  return String(value || '').slice(-2400);
}

function parseJsonLine(raw) {
  const lines = String(raw || '').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    try {
      return JSON.parse(lines[idx]);
    } catch {
      // try previous line
    }
  }
  return null;
}

function runSetup(command, timeoutMs) {
  const started = Date.now();
  const [cmd, ...args] = command;
  const run = spawnSync(cmd, args, {
    cwd: root,
    encoding: 'utf8',
    timeout: timeoutMs,
    maxBuffer: 1024 * 1024,
    env: process.env,
  });
  const timedOut = Boolean(run.error && run.error.code === 'ETIMEDOUT');
  return {
    id: 'build_source_ops_binary',
    kind: 'setup',
    command,
    duration_ms: Date.now() - started,
    exit_code: typeof run.status === 'number' ? run.status : null,
    signal: run.signal || null,
    timed_out: timedOut,
    ok: !timedOut && run.status === 0,
    stdout_tail: compactTail(run.stdout),
    stderr_tail: compactTail(run.stderr),
  };
}

function resolveOpsBinary() {
  const envBinary = process.env.INFRING_GATEWAY_REPLAY_OPS_BINARY;
  const candidates = [
    ...(envBinary ? [envBinary] : []),
    ...((policy.ops_binary_candidates || []).map(String)),
  ];
  for (const candidate of candidates) {
    const full = path.isAbsolute(candidate) ? candidate : path.join(root, candidate);
    try {
      const stat = fs.statSync(full);
      if (stat.isFile()) return full;
    } catch {
      // try next candidate
    }
  }
  return null;
}

if (apply && policy.build_before_apply === true) {
  setupResults.push(runSetup([
    'cargo',
    'build',
    '--quiet',
    '--manifest-path',
    'core/layer0/ops/Cargo.toml',
    '--bin',
    'infring-ops',
  ], Number(policy.build_timeout_ms || 120000)));
}

const opsBinary = resolveOpsBinary();

function resolveCommand(command) {
  return (command || []).map((part) => part === '${INFRING_OPS}' ? opsBinary : part);
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
  const command = resolveCommand(step.command);
  if (command.some((part) => !part)) {
    return {
      id: step.id,
      kind: step.kind,
      mutates: Boolean(step.mutates),
      diagnostic_only: step.diagnostic_only === true,
      skipped: false,
      command: step.command,
      resolved_command: command,
      duration_ms: Date.now() - started,
      exit_code: null,
      signal: null,
      timed_out: false,
      exit_code_ok: false,
      output_token_match_count: 0,
      matched_tokens: [],
      success_token_match_count: 0,
      success_tokens: [],
      forbidden_token_match_count: 0,
      forbidden_tokens: [],
      ok: false,
      stdout_tail: '',
      stderr_tail: 'source authoritative infring-ops binary could not be resolved',
    };
  }
  const [cmd, ...args] = command;
  const childEnv = {
    ...process.env,
    ...(opsBinary ? { INFRING_DAEMON_EXPECTED_BINARY: opsBinary } : {}),
  };
  const run = spawnSync(cmd, args, {
    cwd: root,
    encoding: 'utf8',
    timeout: policy.timeout_ms,
    maxBuffer: 1024 * 1024,
    env: childEnv,
  });
  const combined = `${run.stdout || ''}\n${run.stderr || ''}`.toLowerCase();
  const parsedJson = parseJsonLine(run.stdout);
  const matched = (policy.required_output_tokens_any || []).filter((token) => combined.includes(String(token).toLowerCase()));
  const successTokens = (step.success_tokens_any || []).filter((token) => combined.includes(String(token).toLowerCase()));
  const forbiddenTokens = (step.forbidden_tokens || []).filter((token) => combined.includes(String(token).toLowerCase()));
  const timedOut = Boolean(run.error && run.error.code === 'ETIMEDOUT');
  const exitCode = typeof run.status === 'number' ? run.status : null;
  const acceptedExitCodes = Array.isArray(step.accept_exit_codes) ? step.accept_exit_codes : [0];
  const exitCodeOk = exitCode !== null && acceptedExitCodes.includes(exitCode);
  const diagnosticOnly = step.diagnostic_only === true;
  const receiptOk = parsedJson && parsedJson.ok === true;
  const dashboardRunning = Boolean(parsedJson?.dashboard?.running || parsedJson?.dashboard?.started?.running);
  const receiptOkSatisfied = step.require_receipt_ok === true ? receiptOk : true;
  const dashboardRunningSatisfied = step.require_dashboard_running === true ? dashboardRunning : true;
  const stepOk = diagnosticOnly
    ? matched.length > 0 || exitCodeOk || timedOut
    : !timedOut && exitCodeOk && receiptOkSatisfied && dashboardRunningSatisfied && successTokens.length > 0 && forbiddenTokens.length === 0;
  return {
    id: step.id,
    kind: step.kind,
    mutates: Boolean(step.mutates),
    diagnostic_only: diagnosticOnly,
    skipped: false,
    command: step.command,
    resolved_command: command,
    duration_ms: Date.now() - started,
    exit_code: exitCode,
    signal: run.signal || null,
    timed_out: timedOut,
    exit_code_ok: exitCodeOk,
    receipt_ok: receiptOk,
    dashboard_running: dashboardRunning,
    receipt_ok_satisfied: receiptOkSatisfied,
    dashboard_running_satisfied: dashboardRunningSatisfied,
    output_token_match_count: matched.length,
    matched_tokens: matched,
    success_token_match_count: successTokens.length,
    success_tokens: successTokens,
    forbidden_token_match_count: forbiddenTokens.length,
    forbidden_tokens: forbiddenTokens,
    ok: stepOk,
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
const setupFailures = setupResults.filter((row) => row.ok !== true);
const failures = [...setupFailures, ...results.filter((row) => row.ok !== true)];
const liveRequiredSteps = (policy.steps || []).filter((step) => step.requires_apply && !step.always_cleanup);
const liveRequiredIds = liveRequiredSteps.map((step) => step.id);
const liveCompletedIds = executed.filter((row) => liveRequiredIds.includes(row.id) && row.ok).map((row) => row.id);
const liveProofComplete = apply && liveRequiredIds.every((id) => liveCompletedIds.includes(id));
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
  ops_binary: opsBinary,
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
  setup_results: setupResults,
  live_proof_required_for_completion: Boolean(policy.live_proof_required_for_completion),
  live_proof_complete: liveProofComplete,
  live_required_step_ids: liveRequiredIds,
  live_completed_step_ids: liveCompletedIds,
  skipped_dry_run_plan_step_count: skippedDryRunPlan.length,
  skipped_mutating_step_count: skippedMutating.length,
  failure_count: failures.length,
  results,
};
fs.mkdirSync(path.dirname(path.join(root, policy.report_path)), { recursive: true });
fs.writeFileSync(path.join(root, policy.report_path), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
process.exit(payload.ok ? 0 : 1);
