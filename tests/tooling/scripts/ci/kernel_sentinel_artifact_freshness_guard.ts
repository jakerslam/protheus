#!/usr/bin/env node
import fs from 'fs';
import path from 'path';

type Json = Record<string, any>;
const root = process.cwd();
const strict = process.argv.includes('--strict=1') || process.argv.includes('--strict');
const policyRel = 'observability/sentinel/sentinel_artifact_freshness_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), 'utf8')) as Json;
const sourcePolicyRel = String(policy.source_policy || 'observability/sentinel/sentinel_anti_entropy_observer_policy.json');
const sourcePolicy = JSON.parse(fs.readFileSync(path.join(root, sourcePolicyRel), 'utf8')) as Json;
const outRel = String(policy.output_path || 'core/local/artifacts/kernel_sentinel_artifact_freshness_guard_current.json');
const artifactDir = path.join(root, 'core/local/artifacts');

function ageHours(file: string): number | null {
  try { return (Date.now() - fs.statSync(file).mtimeMs) / 3_600_000; } catch { return null; }
}
function readJson(file: string): Json | null {
  try { return JSON.parse(fs.readFileSync(file, 'utf8')) as Json; } catch { return null; }
}

const rows = Object.entries(sourcePolicy.freshness_budgets_hours || {}).map(([name, budgetRaw]) => {
  const file = path.join(artifactDir, name);
  const age = ageHours(file);
  const budget = Number(budgetRaw || 168);
  const payload = readJson(file);
  const exists = age !== null;
  const fresh = exists && age <= budget;
  return {
    name,
    path: path.relative(root, file),
    exists,
    fresh,
    age_hours: age,
    freshness_budget_hours: budget,
    ok_field: payload?.ok ?? null,
    type: payload?.type ?? null,
    generated_at: payload?.generated_at ?? null,
  };
});
const violations = rows.flatMap((row) => {
  if (!row.exists) return [{ kind: 'missing_sentinel_source_artifact', path: row.path, next_action: `Run or wire producer for ${row.name}.` }];
  if (!row.fresh) return [{ kind: 'stale_sentinel_source_artifact', path: row.path, age_hours: row.age_hours, freshness_budget_hours: row.freshness_budget_hours, next_action: `Refresh ${row.name} during dream maintenance.` }];
  return [];
});
const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-artifact-freshness`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'observability',
  type: 'kernel_sentinel_artifact_freshness_guard',
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  strict,
  policy_path: policyRel,
  source_policy: sourcePolicyRel,
  checked_count: rows.length,
  fresh_count: rows.filter((row) => row.exists && row.fresh).length,
  missing_count: rows.filter((row) => !row.exists).length,
  stale_count: rows.filter((row) => row.exists && !row.fresh).length,
  violations,
  rows,
};
const outPath = path.join(root, outRel);
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
if (strict && !report.ok) process.exit(1);
