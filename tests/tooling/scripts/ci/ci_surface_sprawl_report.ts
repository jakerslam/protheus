#!/usr/bin/env node
import fs from 'fs';
import path from 'path';

type Json = Record<string, any>;
const root = process.cwd();
const policyRel = 'validation/conformance/contracts/ci_surface_sprawl_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), 'utf8')) as Json;
function readJson(rel: string): Json | null { try { return JSON.parse(fs.readFileSync(path.join(root, rel), 'utf8')) as Json; } catch { return null; } }
const workflowDir = path.join(root, String(policy.workflow_dir || '.github/workflows'));
const manifest = readJson(String(policy.tier_manifest || 'validation/conformance/contracts/ci_workflow_tier_manifest.json')) || {};
const manifestRows = Array.isArray(manifest.workflows) ? manifest.workflows : Array.isArray(manifest.entries) ? manifest.entries : [];
const manifestByName = new Map<string, Json>();
for (const row of manifestRows) {
  for (const key of [row.file, row.path, row.workflow, row.name, row.id].filter(Boolean)) manifestByName.set(String(key).split('/').pop() || String(key), row);
}
const workflowFiles = fs.existsSync(workflowDir) ? fs.readdirSync(workflowDir).filter((name) => /\.ya?ml$/i.test(name)).sort() : [];
const rows = workflowFiles.map((name) => {
  const rel = `${policy.workflow_dir || '.github/workflows'}/${name}`;
  const text = fs.readFileSync(path.join(workflowDir, name), 'utf8');
  const manifestRow = manifestByName.get(name) || manifestByName.get(rel) || null;
  const trigger = /schedule:/m.test(text) ? 'scheduled' : /pull_request:/m.test(text) ? 'pull_request' : /push:/m.test(text) ? 'push' : 'other';
  const tier = String(manifestRow?.tier || manifestRow?.classification || 'unknown');
  const required = manifestRow?.required === true || manifestRow?.branch_protection_required === true || manifestRow?.required_for_branch_protection === true;
  const requiredForRelease = manifestRow?.required_for_release === true || tier === 'release_gate' || tier === 'security_gate';
  const recommended = required
    ? 'keep_branch_protection_only_if_release_critical'
    : requiredForRelease
      ? 'release_required_but_not_branch_protection_required'
      : trigger === 'scheduled'
        ? 'scheduled_or_advisory'
        : tier === 'unknown'
          ? 'classify_in_tier_manifest'
          : 'keep_advisory';
  return { name, path: rel, trigger, tier, required, required_for_release: requiredForRelease, in_manifest: Boolean(manifestRow), recommended };
});
const summary = {
  total: rows.length,
  required: rows.filter((row) => row.required).length,
  required_for_release: rows.filter((row) => row.required_for_release).length,
  scheduled: rows.filter((row) => row.trigger === 'scheduled').length,
  unknown: rows.filter((row) => !row.in_manifest || row.tier === 'unknown').length,
  advisory: rows.filter((row) => !row.required).length,
};
const thresholds = policy.thresholds || {};
const violations = [];
if (summary.total > Number(thresholds.maximum_total_workflows || 45)) violations.push({ kind: 'workflow_count_above_threshold', actual: summary.total, maximum: Number(thresholds.maximum_total_workflows || 45), next_action: 'Consolidate duplicate workflows into reusable tier runners.' });
if (summary.required > Number(thresholds.maximum_required_workflows || 30)) violations.push({ kind: 'required_workflow_count_above_threshold', actual: summary.required, maximum: Number(thresholds.maximum_required_workflows || 30), next_action: 'Reduce branch-protection-required workflows to the release-critical set.' });
if (summary.unknown > Number(thresholds.maximum_unknown_workflows || 0)) violations.push({ kind: 'unknown_workflows_need_tier_classification', actual: summary.unknown, maximum: Number(thresholds.maximum_unknown_workflows || 0), next_action: 'Add unknown workflows to the CI tier manifest or retire them.' });
const traceId = `validation:${new Date().toISOString()}:ci-surface-sprawl`;
const report = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', type: 'ci_surface_sprawl_report', generated_at: new Date().toISOString(), ok: violations.length === 0, policy_path: policyRel, summary, violations, rows };
const outJson = path.join(root, String(policy.output_json));
fs.mkdirSync(path.dirname(outJson), { recursive: true });
fs.writeFileSync(outJson, `${JSON.stringify(report, null, 2)}\n`);
const md = ['# CI Surface Sprawl Report', '', `Generated: ${report.generated_at}`, '', `Total workflows: ${summary.total}`, `Required: ${summary.required}`, `Advisory: ${summary.advisory}`, `Scheduled: ${summary.scheduled}`, `Unknown: ${summary.unknown}`, '', '## Violations', ''];
if (!violations.length) md.push('- None'); else for (const v of violations) md.push(`- ${v.kind}: ${v.actual} > ${v.maximum}. Next: ${v.next_action}`);
const outMd = path.join(root, String(policy.output_markdown));
fs.mkdirSync(path.dirname(outMd), { recursive: true });
fs.writeFileSync(outMd, `${md.join('\n')}\n`);
console.log(JSON.stringify({ ok: report.ok, summary, violation_count: violations.length, report: path.relative(root, outJson), markdown: path.relative(root, outMd) }, null, 2));
