#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (CI tier manifest generator)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const workflowDir = path.join(root, '.github/workflows');
const outPath = path.join(root, 'validation/conformance/contracts/ci_workflow_tier_manifest.json');
const files = fs.readdirSync(workflowDir).filter((name) => name.endsWith('.yml') || name.endsWith('.yaml')).sort();
function classify(name, body) {
  const text = `${name}\n${body}`.toLowerCase();
  const lowerName = name.toLowerCase();
  if (lowerName.includes('nightly') || lowerName.includes('dream')) return 'nightly_maintenance';
  if (text.includes('security') || text.includes('audit') || text.includes('codeql') || text.includes('supply')) return 'security_gate';
  if (text.includes('release') || text.includes('version') || text.includes('publish') || text.includes('sbom')) return 'release_gate';
  if (text.includes('sentinel') || text.includes('observability') || text.includes('telemetry')) return 'observability_guard';
  if (text.includes('validation') || text.includes('test') || text.includes('coverage') || text.includes('smoke')) return 'validation_guard';
  return 'advisory_guard';
}
const workflows = files.map((file) => {
  const full = path.join(workflowDir, file);
  const body = fs.readFileSync(full, 'utf8');
  const nameMatch = body.match(/^name:\s*(.+)$/m);
  const tier = classify(file, body);
  return {
    file: `.github/workflows/${file}`,
    name: nameMatch ? nameMatch[1].replace(/^['"]|['"]$/g, '') : file,
    tier,
    required_for_release: ['release_gate', 'security_gate', 'validation_guard'].includes(tier),
    allowed_to_be_advisory: ['advisory_guard', 'nightly_maintenance', 'observability_guard'].includes(tier)
  };
});
const payload = {
  trace_id: `validation:${new Date().toISOString()}:${process.pid}`,
  span_id: `span:ci_workflow_tier_manifest:${process.pid}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'ci_workflow_tier_manifest',
  schema_version: 1,
  generated_at: new Date().toISOString(),
  workflow_count: workflows.length,
  workflows,
  governance: {
    required_tiers: ['release_gate', 'security_gate', 'validation_guard'],
    advisory_tiers: ['advisory_guard', 'nightly_maintenance', 'observability_guard'],
    recommendation: 'Use this manifest as the durable source for required/advisory/nightly workflow ownership before changing branch protection.'
  }
};
fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: 'ci_workflow_tier_manifest_generate', output_path: outPath, workflow_count: workflows.length }, null, 2));
