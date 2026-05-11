#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/regression (Gateway status live replay guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/regression/fixtures/gateway_idempotence/gateway_status_live_replay_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];
if (!report) violations.push({ kind: 'gateway_status_live_replay_report_missing', path: policy.report_path });
if (report && report.duration_ms > policy.timeout_ms + 1000) violations.push({ kind: 'gateway_status_live_replay_exceeded_timeout_budget', duration_ms: report.duration_ms, timeout_ms: policy.timeout_ms });
if (report && !Array.isArray(report.command)) violations.push({ kind: 'gateway_status_live_replay_missing_command' });
if (report && String(report.command || '').includes('restart')) violations.push({ kind: 'gateway_status_live_replay_not_read_only' });
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: report?.trace_id || null, source_domain: 'validation', ok: violations.length === 0, type: 'gateway_status_live_replay_guard', generated_at: new Date().toISOString(), policy_path: policyPath, report_path: policy.report_path, replay_ok: !!report?.ok, replay_diagnostic: report?.diagnostic || null, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/gateway_status_live_replay_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
