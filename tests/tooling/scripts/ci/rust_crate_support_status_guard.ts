#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const manifestPath = 'validation/conformance/contracts/rust_crate_support_status_manifest.json';
const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, manifestPath), 'utf8'));
const violations: any[] = [];
for (const row of manifest.crates || []) {
  if (!row.path || !fs.existsSync(path.join(ROOT, row.path))) violations.push({ kind: 'crate_manifest_missing', path: row.path });
  if (!row.support_status) violations.push({ kind: 'crate_support_status_missing', path: row.path });
  if (!row.owner) violations.push({ kind: 'crate_owner_missing', path: row.path });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'rust_crate_support_status_guard', generated_at: new Date().toISOString(), manifest_path: manifestPath, crate_count: (manifest.crates || []).length, statuses: manifest.statuses || {}, violations };
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/rust_crate_support_status_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
