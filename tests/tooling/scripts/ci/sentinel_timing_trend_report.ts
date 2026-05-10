#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/reports (Sentinel timing trend report)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'observability/sentinel/sentinel_timing_trend_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const outPath = path.join(root, policy.report_path);
function walk(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const name of fs.readdirSync(dir)) {
    const full = path.join(dir, name);
    const stat = fs.statSync(full);
    if (stat.isDirectory()) walk(full, out);
    else if (name === 'kernel_sentinel_auto_run_current.json') out.push(full);
  }
  return out;
}
const candidates = new Set([
  path.join(root, 'core/local/artifacts/kernel_sentinel_auto_run_current.json'),
  path.join(root, 'core/local/state/kernel_sentinel/kernel_sentinel_auto_run_current.json'),
  ...walk(path.join(root, 'validation/release_gates/proof_packs')),
]);
const samples = [];
for (const file of candidates) {
  if (!fs.existsSync(file)) continue;
  try {
    const artifact = JSON.parse(fs.readFileSync(file, 'utf8'));
    const stages = Array.isArray(artifact.stage_timings) ? artifact.stage_timings : [];
    if (!stages.length) continue;
    samples.push({
      path: path.relative(root, file),
      generated_at: artifact.generated_at || '',
      trace_id: artifact.trace_id || '',
      cadence: artifact.cadence || '',
      artifact_kind: artifact.artifact_kind || 'full_auto_run',
      stage_timings: stages.map((stage) => ({ stage: stage.stage || 'unknown', elapsed_ms: Number(stage.elapsed_ms) || 0 }))
    });
  } catch {}
}
const byStage = {};
for (const sample of samples) {
  for (const stage of sample.stage_timings) {
    const row = byStage[stage.stage] || { count: 0, max_ms: 0, total_ms: 0 };
    row.count += 1;
    row.max_ms = Math.max(row.max_ms, stage.elapsed_ms);
    row.total_ms += stage.elapsed_ms;
    byStage[stage.stage] = row;
  }
}
for (const row of Object.values(byStage)) row.avg_ms = row.count ? Math.round(row.total_ms / row.count) : 0;
const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'observability',
  type: 'sentinel_timing_trend_report',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  sample_count: samples.length,
  status: samples.length >= (policy.budgets?.warn_if_samples_below || 2) ? 'trend_ready' : 'insufficient_samples',
  by_stage: byStage,
  samples: samples.slice(0, 20),
  next_action: samples.length >= (policy.budgets?.warn_if_samples_below || 2)
    ? 'compare stage max/avg across dream and release runs for regressions'
    : 'collect at least one additional full dream or release Sentinel run with stage_timings'
};
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: 'sentinel_timing_trend_report', report_path: policy.report_path, sample_count: samples.length, status: payload.status }, null, 2));
