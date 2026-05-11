#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (command operator surface guard)

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/command_operator_surface_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const registry = JSON.parse(fs.readFileSync(path.join(root, policy.registry_path), 'utf8'));
const runner = fs.readFileSync(path.join(root, policy.runner_path), 'utf8');
const entries = Array.isArray(registry.entries) ? registry.entries : [];
const surface = entries.filter((entry) => entry.operator_surface === true);
const curatedCanonical = surface.filter((entry) => entry.metadata_curated && entry.lifecycle === 'canonical_entrypoint');
const violations = [];

if (surface.length !== policy.required_operator_surface_count) {
  violations.push({ kind: 'operator_surface_count_mismatch', expected: policy.required_operator_surface_count, actual: surface.length });
}
if (curatedCanonical.length < policy.minimum_curated_canonical_count) {
  violations.push({ kind: 'operator_surface_missing_curated_canonical_commands', expected_minimum: policy.minimum_curated_canonical_count, actual: curatedCanonical.length });
}
for (const entry of surface) {
  if (!Number.isInteger(entry.operator_surface_rank) || entry.operator_surface_rank < 1) violations.push({ kind: 'operator_surface_missing_rank', id: entry.id });
  if (!entry.operator_surface_reason) violations.push({ kind: 'operator_surface_missing_reason', id: entry.id });
  for (const term of policy.forbidden_promoted_terms || []) {
    const haystack = `${entry.id}\n${entry.command}`.toLowerCase();
    if (!entry.metadata_curated && haystack.includes(String(term).toLowerCase())) {
      violations.push({ kind: 'operator_surface_forbidden_promoted_term', id: entry.id, term });
    }
  }
}
if (!runner.includes('operatorSurface')) violations.push({ kind: 'command_runner_missing_operator_surface_filter' });
if (!runner.includes('--operator-surface=0')) violations.push({ kind: 'command_runner_missing_operator_surface_usage' });

const listed = spawnSync('node', ['client/runtime/lib/ts_entrypoint.ts', policy.runner_path, 'list'], { cwd: root, encoding: 'utf8', timeout: 10000, maxBuffer: 1024 * 1024 });
let defaultCount = null;
try {
  defaultCount = JSON.parse(listed.stdout || '{}').count;
} catch {
  violations.push({ kind: 'command_runner_default_list_not_json' });
}
if (typeof defaultCount === 'number' && defaultCount > policy.maximum_default_command_count) {
  violations.push({ kind: 'command_runner_default_list_too_large', maximum: policy.maximum_default_command_count, actual: defaultCount });
}

const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'command_operator_surface_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  operator_surface_count: surface.length,
  curated_canonical_count: curatedCanonical.length,
  default_list_count: defaultCount,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, policy.report_path), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
