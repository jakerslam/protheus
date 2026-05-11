#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (installer module dispatch guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const reportPath = path.join(root, 'core/local/artifacts/installer_module_dispatch_current.json');
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];
if (!report) violations.push({ kind: 'installer_module_dispatch_report_missing' });
if (report && report.source_domain !== 'validation') violations.push({ kind: 'installer_module_dispatch_wrong_source_domain', actual: report.source_domain });
for (const row of (report?.rows || [])) {
  if (!['referenced', 'mirror_only'].includes(row.status)) {
    violations.push({ kind: 'installer_module_dispatch_unwired', installer: row.installer, module: row.module, status: row.status });
  }
  if (row.status !== 'referenced' && !row.next_action) {
    violations.push({ kind: 'installer_module_dispatch_missing_next_action', installer: row.installer, module: row.module });
  }
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: report?.trace_id || null,
  source_domain: 'validation',
  type: 'installer_module_dispatch_guard',
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: path.relative(root, reportPath),
  referenced_count: report?.referenced_count ?? null,
  mirror_only_count: report?.mirror_only_count ?? null,
  unwired_count: report?.unwired_count ?? null,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/installer_module_dispatch_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
