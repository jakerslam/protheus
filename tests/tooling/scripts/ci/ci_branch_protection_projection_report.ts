#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/release_gates (CI branch protection projection)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'validation/release_gates/policies/ci_branch_protection_projection_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const manifest = JSON.parse(fs.readFileSync(path.join(root, policy.manifest_path), 'utf8'));
const workflows = Array.isArray(manifest.workflows) ? manifest.workflows : [];

function hasToken(row, tokens) {
  const text = `${row.name || ''}\n${row.file || ''}\n${row.tier || ''}`.toLowerCase();
  return (tokens || []).some((token) => text.includes(String(token).toLowerCase()));
}

function contextName(row) {
  return String(row.name || path.basename(row.file || '').replace(/\.ya?ml$/, ''));
}

function priority(row) {
  let score = 0;
  if (hasToken(row, policy.always_required_name_tokens)) score += 100;
  if (row.tier === 'security_gate') score += 50;
  if (row.tier === 'release_gate') score += 30;
  if (row.tier === 'validation_guard') score += 10;
  if (hasToken(row, policy.advisory_name_tokens)) score -= 80;
  return score;
}

const requiredBudget = policy.required_context_budget || {};
const buckets = new Map();
for (const row of workflows) {
  const tier = row.tier || 'unclassified';
  buckets.set(tier, [...(buckets.get(tier) || []), row]);
}
const selected = [];
for (const [tier, rows] of buckets.entries()) {
  const budget = Number(requiredBudget[tier] || 0);
  if (budget <= 0) continue;
  selected.push(...rows.sort((a, b) => priority(b) - priority(a) || contextName(a).localeCompare(contextName(b))).slice(0, budget));
}
const selectedIds = new Set(selected.map((row) => row.file || row.name));
let required = selected.slice(0, policy.target_required_max);
if (required.length > policy.target_required_max) required = required.slice(0, policy.target_required_max);
const requiredIds = new Set(required.map((row) => row.file || row.name));
const advisory = workflows.filter((row) => !requiredIds.has(row.file || row.name) && row.required_for_release !== false);
const nightly = workflows.filter((row) => row.required_for_release === false);
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const projection = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'ci_branch_protection_projection',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  ok: required.length <= policy.target_required_max,
  target_required_max: policy.target_required_max,
  current_workflow_count: workflows.length,
  projected_required_count: required.length,
  projected_advisory_count: advisory.length,
  projected_nightly_count: nightly.length,
  requires_human_review_before_apply: true,
  required_contexts: required.map((row) => ({
    context: contextName(row),
    workflow_file: row.file,
    tier: row.tier,
    reason: hasToken(row, policy.always_required_name_tokens) ? 'always_required_token' : 'tier_budget',
  })),
  advisory_contexts: advisory.map((row) => ({
    context: contextName(row),
    workflow_file: row.file,
    tier: row.tier,
    reason: hasToken(row, policy.advisory_name_tokens) ? 'advisory_token' : selectedIds.has(row.file || row.name) ? 'over_target_budget' : 'outside_required_budget',
  })),
  nightly_contexts: nightly.map((row) => ({
    context: contextName(row),
    workflow_file: row.file,
    tier: row.tier,
  })),
  next_actions: [
    'Review required_contexts for correctness before applying branch protection changes.',
    'Use the GitHub UI/API to mark required_contexts required and keep advisory/nightly checks visible but non-blocking.',
    'Re-run this projection after workflow renames or required-check policy changes.',
  ],
};
for (const outputPath of [policy.report_path, policy.artifact_path]) {
  const full = path.join(root, outputPath);
  fs.mkdirSync(path.dirname(full), { recursive: true });
  fs.writeFileSync(full, `${JSON.stringify(projection, null, 2)}\n`);
}
console.log(JSON.stringify({
  ok: projection.ok,
  type: projection.type,
  projected_required_count: projection.projected_required_count,
  projected_advisory_count: projection.projected_advisory_count,
  projected_nightly_count: projection.projected_nightly_count,
  report_path: policy.report_path,
}, null, 2));
