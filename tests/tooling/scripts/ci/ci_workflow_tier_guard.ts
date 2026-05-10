#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/conformance/contracts/ci_workflow_tier_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(ROOT, policyPath), 'utf8'));
const manifestPath = policy.manifest_path || 'validation/conformance/contracts/ci_workflow_tier_manifest.json';
const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, manifestPath), 'utf8'));
const workflowRoot = path.join(ROOT, policy.workflow_root);
const workflowFiles = fs.existsSync(workflowRoot)
  ? fs.readdirSync(workflowRoot).filter((f) => f.endsWith('.yml') || f.endsWith('.yaml')).sort()
  : [];
const live = new Set(workflowFiles.map((file) => `${policy.workflow_root}/${file}`));
const manifestRows = Array.isArray(manifest.workflows) ? manifest.workflows : [];
const byFile = new Map(manifestRows.map((row: any) => [String(row.file), row]));
const allowedTiers = new Set(policy.tiers || []);
const requiredTiers = new Set(policy.required_tiers || []);
const advisoryTiers = new Set(policy.advisory_tiers || []);
const violations: any[] = [];
if (manifest.workflow_count !== workflowFiles.length) {
  violations.push({ kind: 'workflow_count_mismatch', manifest_count: manifest.workflow_count, live_count: workflowFiles.length });
}
for (const file of live) {
  if (!byFile.has(file)) violations.push({ kind: 'workflow_missing_from_manifest', file });
}
for (const row of manifestRows) {
  const file = String(row.file || '');
  const tier = String(row.tier || '');
  if (!live.has(file)) violations.push({ kind: 'stale_workflow_manifest_entry', file });
  if (!allowedTiers.has(tier)) violations.push({ kind: 'invalid_workflow_tier', file, tier });
  const expectedRequired = requiredTiers.has(tier);
  if (Boolean(row.required_for_release) !== expectedRequired) {
    violations.push({ kind: 'required_for_release_mismatch', file, tier, expected: expectedRequired, actual: Boolean(row.required_for_release) });
  }
  const expectedAdvisory = advisoryTiers.has(tier);
  if (Boolean(row.allowed_to_be_advisory) !== expectedAdvisory) {
    violations.push({ kind: 'allowed_to_be_advisory_mismatch', file, tier, expected: expectedAdvisory, actual: Boolean(row.allowed_to_be_advisory) });
  }
}
const tierCounts = manifestRows.reduce((acc: any, row: any) => {
  const tier = String(row.tier || 'unclassified');
  acc[tier] = (acc[tier] || 0) + 1;
  return acc;
}, {});
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'ci_workflow_tier_guard', generated_at: new Date().toISOString(), policy_path: policyPath, manifest_path: manifestPath, workflow_count: workflowFiles.length, tiers: tierCounts, violations, classified: manifestRows };
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/ci_workflow_tier_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
