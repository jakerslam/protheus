#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/sentinel (Sentinel staged timing sample capture guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'observability/sentinel/sentinel_timing_trend_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const latestPath = path.join(root, policy.latest_sample_path || 'core/local/artifacts/sentinel_timing_sample_capture_current.json');
const storePath = path.join(root, policy.sample_store_path || '');
const latest = fs.existsSync(latestPath) ? JSON.parse(fs.readFileSync(latestPath, 'utf8')) : null;
const lines = fs.existsSync(storePath) ? fs.readFileSync(storePath, 'utf8').split(/\r?\n/).filter(Boolean) : [];
const violations = [];
if (!latest) violations.push({ kind: 'sentinel_timing_sample_latest_missing' });
if (!lines.length) violations.push({ kind: 'sentinel_timing_sample_store_empty' });
if (latest && latest.source_domain !== 'observability') violations.push({ kind: 'sentinel_timing_sample_wrong_source_domain', actual: latest.source_domain });
if (latest && Number(latest.stage_count || 0) < 1) violations.push({ kind: 'sentinel_timing_sample_missing_stages' });
if (latest && !Array.isArray(latest.stage_timings)) violations.push({ kind: 'sentinel_timing_sample_missing_stage_timings' });
const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: latest?.trace_id || null,
  source_domain: 'observability',
  type: 'sentinel_timing_sample_capture_guard',
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  policy_path: policyPath,
  sample_count: lines.length,
  latest_sample_path: policy.latest_sample_path,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/sentinel_timing_sample_capture_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
