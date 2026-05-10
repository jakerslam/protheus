#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'observability/sentinel/sentinel_full_run_stage_timing_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(ROOT, policyPath), 'utf8'));
const sourcePath = 'core/layer0/ops/src/kernel_sentinel/auto_run.rs';
const source = fs.readFileSync(path.join(ROOT, sourcePath), 'utf8');
const violations: any[] = [];
if (!source.includes('stage_timings')) violations.push({ kind: 'stage_timings_missing', path: sourcePath });
for (const stage of policy.required_stage_names) {
  if (!source.includes(stage)) violations.push({ kind: 'required_stage_missing', path: sourcePath, stage });
}
if (!source.includes('fresh_lightweight_observation')) violations.push({ kind: 'lightweight_observation_missing', path: sourcePath });
for (const field of policy.required_trace_fields || []) {
  if (!source.includes(field)) violations.push({ kind: 'required_trace_field_missing', path: sourcePath, field });
}
const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'observability', ok: violations.length === 0, type: 'sentinel_stage_timing_policy_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/sentinel_stage_timing_policy_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
