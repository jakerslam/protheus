#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/sentinel (Sentinel staged timing sample capture)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'observability/sentinel/sentinel_timing_trend_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const sourceRel = process.argv.find((arg) => arg.startsWith('--source='))?.slice('--source='.length) || 'observability/reports/sentinel_full_run_stage_runner_current.json';
const cadence = process.argv.find((arg) => arg.startsWith('--cadence='))?.slice('--cadence='.length) || 'dream';
const sourcePath = path.join(root, sourceRel);
const source = JSON.parse(fs.readFileSync(sourcePath, 'utf8'));
const phases = Array.isArray(source.phase_results) ? source.phase_results : [];
const stageTimings = phases.map((phase) => ({
  stage: String(phase.id || 'unknown'),
  elapsed_ms: Number(phase.duration_ms || 0),
  ok: phase.ok === true,
}));
const sourceTrace = String(source.trace_id || '');
const sample = {
  trace_id: `observability:${new Date().toISOString()}:${process.pid}:sentinel-timing-sample`,
  parent_span_id: sourceTrace || null,
  source_domain: 'observability',
  type: 'sentinel_timing_sample',
  generated_at: new Date().toISOString(),
  source_report: sourceRel,
  source_trace_id: sourceTrace,
  cadence,
  artifact_kind: 'staged_sentinel_run',
  stage_count: stageTimings.length,
  total_elapsed_ms: stageTimings.reduce((sum, row) => sum + row.elapsed_ms, 0),
  stage_timings: stageTimings,
};
if (stageTimings.length < 1) {
  console.error(JSON.stringify({ ok: false, type: 'sentinel_timing_sample_capture', reason: 'missing_stage_timings', source: sourceRel }, null, 2));
  process.exit(1);
}
const storePath = path.join(root, policy.sample_store_path);
fs.mkdirSync(path.dirname(storePath), { recursive: true });
const existing = fs.existsSync(storePath) ? fs.readFileSync(storePath, 'utf8').split(/\r?\n/).filter(Boolean) : [];
const duplicate = existing.some((line) => {
  try {
    const row = JSON.parse(line);
    return row.source_trace_id === sample.source_trace_id && row.cadence === sample.cadence;
  } catch {
    return false;
  }
});
if (!duplicate) fs.appendFileSync(storePath, `${JSON.stringify(sample)}\n`);
const latestPath = path.join(root, policy.latest_sample_path || 'core/local/artifacts/sentinel_timing_sample_capture_current.json');
fs.mkdirSync(path.dirname(latestPath), { recursive: true });
fs.writeFileSync(latestPath, `${JSON.stringify({ ...sample, appended: !duplicate }, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: 'sentinel_timing_sample_capture', appended: !duplicate, cadence, stage_count: stageTimings.length, store_path: policy.sample_store_path }, null, 2));
