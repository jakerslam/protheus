#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/reports (Sentinel full-run timeout report)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'observability/sentinel/sentinel_full_run_timeout_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const artifactPath = path.join(root, policy.artifact_path);
const outPath = path.join(root, policy.report_path);
let artifact = null;
try { artifact = JSON.parse(fs.readFileSync(artifactPath, 'utf8')); } catch {}
const artifactBytes = artifact && fs.existsSync(artifactPath) ? fs.statSync(artifactPath).size : 0;
const timeoutObserved = artifact?.artifact_kind === 'diagnostic' && (artifact?.status === 'timeout' || artifact?.operator_summary?.failure_kind === 'sentinel_auto_timeout');
const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: artifact?.trace_id || null,
  source_domain: 'observability',
  type: 'sentinel_full_run_timeout_report',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  artifact_path: policy.artifact_path,
  timeout_observed: timeoutObserved,
  artifact_bytes: artifactBytes,
  compact_artifact_ok: artifactBytes <= policy.budgets.max_timeout_artifact_bytes,
  observed_status: artifact?.status || null,
  observed_failure_kind: artifact?.operator_summary?.failure_kind || artifact?.failure_kind || null,
  observed_max_runtime_ms: artifact?.max_runtime_ms || null,
  observed_elapsed_ms: artifact?.elapsed_ms || null,
  severity: timeoutObserved ? 'yellow' : 'pass',
  root_cause_hypothesis: timeoutObserved
    ? 'Kernel Sentinel full dream/release self-study currently exceeds its bounded run budget; likely needs stage splitting, cached evidence reuse, or a larger explicit dream budget.'
    : 'No current Sentinel full-run timeout diagnostic observed.',
  next_actions: timeoutObserved ? [
    'Split expensive Sentinel full-run stages into resumable collector/report/self-study phases.',
    'Persist partial stage timings before timeout so future runs can localize the expensive stage.',
    'Keep heartbeat/maintenance Sentinel on lightweight freshness path; reserve full self-study for dream/release cadence.'
  ] : []
};
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: payload.type, timeout_observed: timeoutObserved, severity: payload.severity, report_path: policy.report_path }, null, 2));
