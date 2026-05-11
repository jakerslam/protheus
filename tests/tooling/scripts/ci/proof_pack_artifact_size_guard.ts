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
function hasOwn(row: Record<string, unknown>, field: string): boolean {
  return Object.prototype.hasOwnProperty.call(row, field);
}
function compactedRef(file: string, requiredFields: string[]): { ok: boolean; missing_required_fields: string[] } {
  if (!file.endsWith('.json')) return { ok: false, missing_required_fields: [] };
  try {
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8')) as Record<string, unknown>;
    const ok = Boolean(parsed && typeof parsed === 'object' && String(parsed.type || '').includes('compacted') && parsed.previous_artifact);
    if (!ok) return { ok: false, missing_required_fields: [] };
    return {
      ok: true,
      missing_required_fields: requiredFields.filter((field) => !hasOwn(parsed, field) || parsed[field] == null || parsed[field] === ''),
    };
  } catch {
    return { ok: false, missing_required_fields: [] };
  }
}

const rootReports = [];
const compactableArtifacts: Array<{ path: string; bytes: number; action: string }> = [];
const blockedArtifacts: Array<{ path: string; detail: string; action: string }> = [];
const requiredCompactedRefFields = Array.isArray(policy.policy.compacted_ref_required_fields)
  ? policy.policy.compacted_ref_required_fields.map(String)
  : [];
const releaseGateEnforcement = policy.policy.release_gate_enforcement && typeof policy.policy.release_gate_enforcement === 'object'
  ? policy.policy.release_gate_enforcement
  : {};
for (const field of [
  'pre_assembly_guard_required',
  'post_assembly_guard_required',
  'assembler_must_compact_oversized_source_artifacts',
  'oversized_existing_proof_pack_artifacts_block_release',
  'oversized_existing_proof_pack_artifacts_may_be_replaced_by_compacted_refs',
  'oversized_proof_pack_directories_block_release',
]) {
  if (releaseGateEnforcement[field] !== true) {
    violations.push({
      kind: 'proof_pack_release_gate_enforcement_missing',
      path: policyPath,
      detail: field,
    });
  }
}
for (const proofRootRel of proofRoots) {
  const proofRoot = path.join(ROOT, proofRootRel);
  const files = walk(proofRoot);
  let rootBytes = 0;
  let compactedRefCount = 0;
  for (const file of files) {
    const relative = rel(file);
    const stat = fs.statSync(file);
    rootBytes += stat.size;
    const compactedRefCheck = compactedRef(file, requiredCompactedRefFields);
    const isCompactedRef = compactedRefCheck.ok;
    if (isCompactedRef) compactedRefCount += 1;
    if (isCompactedRef && compactedRefCheck.missing_required_fields.length > 0) {
      violations.push({
        kind: 'compacted_ref_missing_required_fields',
        path: relative,
        detail: `missing=${compactedRefCheck.missing_required_fields.join(',')}`,
      });
      blockedArtifacts.push({
        path: relative,
        detail: `missing=${compactedRefCheck.missing_required_fields.join(',')}`,
        action: 'block_release_until_compacted_ref_contract_is_fixed',
      });
    }
    const allowedLarge = policy.allowed_large_artifact_patterns.some((p: string) => relative.endsWith(p));
    if (stat.size > policy.policy.max_single_artifact_bytes && !allowedLarge && !isCompactedRef) {
      violations.push({ kind: 'oversized_proof_pack_artifact', path: relative, detail: `bytes=${stat.size}` });
      compactableArtifacts.push({
        path: relative,
        bytes: stat.size,
        action: 'replace_artifact_body_with_proof_pack_compacted_artifact_ref_or_remove_from_release_pack',
      });
    }
    if (relative.endsWith('.json') && !isCompactedRef) {
      const lines = fs.readFileSync(file, 'utf8').split('\n').length;
      if (lines > policy.policy.max_single_json_lines) {
        violations.push({ kind: 'oversized_json_line_count', path: relative, detail: `lines=${lines}` });
        compactableArtifacts.push({
          path: relative,
          bytes: stat.size,
          action: 'replace_large_json_with_compacted_ref_before_release',
        });
      }
    }
  }
  const packReports = [];
  for (const pack of fs.existsSync(proofRoot) ? fs.readdirSync(proofRoot) : []) {
    const full = path.join(proofRoot, pack);
    if (!fs.statSync(full).isDirectory()) continue;
    let bytes = 0;
    for (const file of walk(full)) bytes += fs.statSync(file).size;
    if (bytes > policy.policy.max_proof_pack_bytes) {
      violations.push({ kind: 'oversized_proof_pack_directory', path: rel(full), detail: `bytes=${bytes}` });
      blockedArtifacts.push({
        path: rel(full),
        detail: `bytes=${bytes}`,
        action: 'block_release_until_pack_is_split_or_compacted_under_total_budget',
      });
    }
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
  release_gate_enforcement: {
    pre_assembly_guard_required: releaseGateEnforcement.pre_assembly_guard_required === true,
    post_assembly_guard_required: releaseGateEnforcement.post_assembly_guard_required === true,
    assembler_must_compact_oversized_source_artifacts: releaseGateEnforcement.assembler_must_compact_oversized_source_artifacts === true,
    oversized_existing_proof_pack_artifacts_block_release: releaseGateEnforcement.oversized_existing_proof_pack_artifacts_block_release === true,
    oversized_existing_proof_pack_artifacts_may_be_replaced_by_compacted_refs:
      releaseGateEnforcement.oversized_existing_proof_pack_artifacts_may_be_replaced_by_compacted_refs === true,
    oversized_proof_pack_directories_block_release: releaseGateEnforcement.oversized_proof_pack_directories_block_release === true,
  },
  compactable_artifact_count: compactableArtifacts.length,
  compactable_artifacts: compactableArtifacts.slice(0, 100),
  blocked_artifact_count: blockedArtifacts.length,
  blocked_artifacts: blockedArtifacts.slice(0, 100),
  violation_count: violations.length,
  violations,
};
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/proof_pack_artifact_size_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
