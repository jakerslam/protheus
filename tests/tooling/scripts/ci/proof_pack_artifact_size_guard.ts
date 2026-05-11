#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/release_gates/policies/proof_pack_artifact_size_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(ROOT, policyPath), 'utf8'));
const proofRoots = ['validation/release_gates/proof_packs', 'releases/proof-packs'];
type Violation = { kind: string; path: string; detail: string };
const violations: Violation[] = [];

function walk(dir: string): string[] {
  if (!fs.existsSync(dir)) return [];
  const out: string[] = [];
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, ent.name);
    if (ent.isDirectory()) out.push(...walk(full));
    else if (ent.isFile()) out.push(full);
  }
  return out;
}
function rel(file: string): string { return path.relative(ROOT, file).replace(/\\/g, '/'); }
function compactedRef(file: string): boolean {
  if (!file.endsWith('.json')) return false;
  try {
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8'));
    return Boolean(parsed && typeof parsed === 'object' && String(parsed.type || '').includes('compacted') && parsed.previous_artifact);
  } catch {
    return false;
  }
}

const rootReports = [];
for (const proofRootRel of proofRoots) {
  const proofRoot = path.join(ROOT, proofRootRel);
  const files = walk(proofRoot);
  let rootBytes = 0;
  let compactedRefCount = 0;
  for (const file of files) {
    const relative = rel(file);
    const stat = fs.statSync(file);
    rootBytes += stat.size;
    const isCompactedRef = compactedRef(file);
    if (isCompactedRef) compactedRefCount += 1;
    const allowedLarge = policy.allowed_large_artifact_patterns.some((p: string) => relative.endsWith(p));
    if (stat.size > policy.policy.max_single_artifact_bytes && !allowedLarge && !isCompactedRef) {
      violations.push({ kind: 'oversized_proof_pack_artifact', path: relative, detail: `bytes=${stat.size}` });
    }
    if (relative.endsWith('.json') && !isCompactedRef) {
      const lines = fs.readFileSync(file, 'utf8').split('\n').length;
      if (lines > policy.policy.max_single_json_lines) violations.push({ kind: 'oversized_json_line_count', path: relative, detail: `lines=${lines}` });
    }
  }
  const packReports = [];
  for (const pack of fs.existsSync(proofRoot) ? fs.readdirSync(proofRoot) : []) {
    const full = path.join(proofRoot, pack);
    if (!fs.statSync(full).isDirectory()) continue;
    let bytes = 0;
    for (const file of walk(full)) bytes += fs.statSync(file).size;
    if (bytes > policy.policy.max_proof_pack_bytes) violations.push({ kind: 'oversized_proof_pack_directory', path: rel(full), detail: `bytes=${bytes}` });
    packReports.push({ path: rel(full), bytes });
  }
  rootReports.push({
    path: proofRootRel,
    exists: fs.existsSync(proofRoot),
    file_count: files.length,
    total_bytes: rootBytes,
    compacted_ref_count: compactedRefCount,
    packs: packReports,
  });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'proof_pack_artifact_size_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  roots: rootReports,
  max_single_artifact_bytes: policy.policy.max_single_artifact_bytes,
  max_single_json_lines: policy.policy.max_single_json_lines,
  max_proof_pack_bytes: policy.policy.max_proof_pack_bytes,
  violation_count: violations.length,
  violations,
};
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/proof_pack_artifact_size_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
