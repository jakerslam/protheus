#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (curated command metadata guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const overridesPath = path.join(root, 'validation/conformance/contracts/command_metadata_curated_overrides.json');
const registryPath = path.join(root, 'tools/commands/command_registry.json');
const overrides = JSON.parse(fs.readFileSync(overridesPath, 'utf8'));
const registry = JSON.parse(fs.readFileSync(registryPath, 'utf8'));
const entries = Array.isArray(registry.entries) ? registry.entries : [];
const byId = new Map(entries.map((entry) => [entry.id, entry]));
const violations = [];
for (const override of overrides.overrides || []) {
  const entry = byId.get(override.id);
  if (!entry) {
    violations.push({ kind: 'curated_command_missing', id: override.id });
    continue;
  }
  for (const field of ['domain', 'work_gate', 'lifecycle', 'owner', 'description']) {
    if (entry[field] !== override[field]) {
      violations.push({ kind: 'curated_command_metadata_mismatch', id: override.id, field, expected: override[field], actual: entry[field] });
    }
  }
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'command_metadata_curated_guard', generated_at: new Date().toISOString(), overrides_path: overridesPath, curated_count: (overrides.overrides || []).length, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/command_metadata_curated_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
