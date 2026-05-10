#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/conformance/contracts/command_registry_metadata_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(ROOT, policyPath), 'utf8'));
const registry = JSON.parse(fs.readFileSync(path.join(ROOT, policy.registry_path), 'utf8'));
const entries = Array.isArray(registry.entries) ? registry.entries : [];
const violations: any[] = [];
for (const entry of entries) {
  for (const field of policy.required_entry_fields) {
    if (entry[field] === undefined || entry[field] === null || String(entry[field]).trim() === '') violations.push({ kind: 'missing_command_metadata', id: entry.id || 'unknown', field });
  }
  if (entry.work_gate && !policy.allowed_work_gates.includes(entry.work_gate)) violations.push({ kind: 'invalid_work_gate', id: entry.id, work_gate: entry.work_gate });
  if (entry.lifecycle === 'compatibility_alias' && !entry.owner) violations.push({ kind: 'compatibility_alias_missing_owner', id: entry.id });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'command_registry_metadata_guard', generated_at: new Date().toISOString(), entry_count: entries.length, violations };
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/command_registry_metadata_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
