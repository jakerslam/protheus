#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/sentinel (Sentinel timing trend guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'observability/sentinel/sentinel_timing_trend_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];
if (!report) violations.push({ kind: 'sentinel_timing_trend_report_missing', path: policy.report_path });
if (report && Buffer.byteLength(JSON.stringify(report), 'utf8') > policy.budgets.max_report_bytes) violations.push({ kind: 'sentinel_timing_trend_report_too_large', bytes: Buffer.byteLength(JSON.stringify(report), 'utf8'), max: policy.budgets.max_report_bytes });
if (report && !['trend_ready', 'insufficient_samples'].includes(report.status)) violations.push({ kind: 'sentinel_timing_trend_status_invalid', status: report.status });
const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'observability', ok: violations.length === 0, type: 'sentinel_timing_trend_guard', generated_at: new Date().toISOString(), policy_path: policyPath, report_path: policy.report_path, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/sentinel_timing_trend_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
