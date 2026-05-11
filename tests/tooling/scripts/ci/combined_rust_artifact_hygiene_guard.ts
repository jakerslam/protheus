#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const ROOT = process.cwd();
type Violation = { kind: string; path: string; detail: string };
function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 ? process.argv[idx + 1] : fallback;
}
function boolFlag(name: string, fallback = false): boolean { const raw = flag(name, fallback ? '1' : '0'); return raw === '1' || raw === 'true'; }
function abs(rel: string): string { return path.join(ROOT, rel); }
function readJson(rel: string): any { return JSON.parse(fs.readFileSync(abs(rel), 'utf8')); }
function ensureDir(rel: string): void { fs.mkdirSync(path.dirname(abs(rel)), { recursive: true }); }
function gitFiles(): string[] { return execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' }).split(/\r?\n/).filter(Boolean); }
function isCombined(p: string): boolean { return p.endsWith('000-combined.rs') || p.includes('.combined_parts/') || p.includes('.combined_parts'); }
function validateRows(rows: any[], required: string[], rel: string, violations: Violation[]): void {
  rows.forEach((row, idx) => {
    for (const field of required) if (!(field in row)) violations.push({ kind: 'combined_row_field_missing', path: rel, detail: `row ${idx} missing ${field}` });
  });
}
function main(): void {
  const strict = boolFlag('strict', true);
  const policyPath = flag('policy', 'validation/conformance/contracts/combined_rust_artifact_hygiene_policy.json');
  const outJson = flag('out-json', 'core/local/artifacts/combined_rust_artifact_hygiene_guard_current.json');
  const outMd = flag('out-markdown', 'local/workspace/reports/COMBINED_RUST_ARTIFACT_HYGIENE_GUARD_CURRENT.md');
  const policy = readJson(policyPath);
  const invPath = policy.baseline_artifacts?.inventory_json;
  const mapPath = policy.baseline_artifacts?.reference_map_json;
  const inventory = readJson(invPath);
  const refMap = readJson(mapPath);
  const current = gitFiles().filter(isCombined).sort();
  const inventoryPaths = new Set((inventory.artifacts || []).map((row: any) => String(row.path)));
  const mapPaths = new Set((refMap.references || []).map((row: any) => String(row.path)));
  const violations: Violation[] = [];
  if (policy.type !== 'combined_rust_artifact_hygiene_policy') violations.push({ kind: 'combined_policy_type_invalid', path: policyPath, detail: 'Wrong policy type.' });
  validateRows(inventory.artifacts || [], policy.inventory_required_fields || [], invPath, violations);
  validateRows(refMap.references || [], policy.reference_map_required_fields || [], mapPath, violations);
  for (const p of current) {
    if (!inventoryPaths.has(p)) violations.push({ kind: 'combined_inventory_missing_current_artifact', path: invPath, detail: p });
    if (!mapPaths.has(p)) violations.push({ kind: 'combined_reference_map_missing_current_artifact', path: mapPath, detail: p });
  }
  for (const p of inventoryPaths) if (!current.includes(p)) violations.push({ kind: 'combined_inventory_stale_artifact', path: invPath, detail: String(p) });
  const allowed = new Set(policy.allowed_reference_classes || []);
  for (const row of refMap.references || []) if (!allowed.has(String(row.reference_class))) violations.push({ kind: 'combined_reference_class_invalid', path: mapPath, detail: `${row.path}: ${row.reference_class}` });
  const payload = {
    ok: violations.length === 0,
    type: 'combined_rust_artifact_hygiene_guard',
    generated_at: new Date().toISOString(),
    strict,
    current_artifact_count: current.length,
    inventory_artifact_count: inventory.artifact_count,
    reference_map_artifact_count: refMap.artifact_count,
    reference_summary: refMap.reference_summary,
    violations,
  };
  ensureDir(outJson);
  fs.writeFileSync(abs(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMd);
  fs.writeFileSync(abs(outMd), `# Combined Rust Artifact Hygiene Guard\n\n- ok: ${payload.ok}\n- current_artifact_count: ${current.length}\n- violations: ${violations.length}\n\n${violations.map((v) => `- ${v.kind}: ${v.detail}`).join('\n') || '- none'}\n`);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !payload.ok) process.exit(1);
}
main();
