#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
type Violation = { kind: string; path: string; detail: string };
function flag(name: string, fallback = ''): string { const prefix = `--${name}=`; const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix)); if (direct) return direct.slice(prefix.length); const idx = process.argv.indexOf(`--${name}`); return idx >= 0 ? process.argv[idx + 1] : fallback; }
function boolFlag(name: string, fallback = false): boolean { const raw = flag(name, fallback ? '1' : '0'); return raw === '1' || raw === 'true'; }
function abs(rel: string): string { return path.join(ROOT, rel); }
function read(rel: string): string { return fs.readFileSync(abs(rel), 'utf8'); }
function exists(rel: string): boolean { return fs.existsSync(abs(rel)); }
function ensureDir(rel: string): void { fs.mkdirSync(path.dirname(abs(rel)), { recursive: true }); }
function main(): void {
  const strict = boolFlag('strict', true);
  const outJson = flag('out-json', 'core/local/artifacts/digital_dna_foundation_graduation_guard_current.json');
  const outMd = flag('out-markdown', 'local/workspace/reports/DIGITAL_DNA_FOUNDATION_GRADUATION_GUARD_CURRENT.md');
  const audit = 'docs/workspace/reports/DNA_FOUNDATION_AUDIT_2026-05-09.md';
  const lock = 'docs/workspace/reports/DNA_SRS_FOUNDATION_LOCK_2026-05-09.md';
  const srs = 'docs/workspace/SRS.md';
  const integrity = 'tests/tooling/scripts/ci/digital_dna_integrity_gate.ts';
  const violations: Violation[] = [];
  for (const rel of [audit, lock, srs, integrity, 'core/layer0/ops/src/metakernel_parts/057-digital-dna-foundation.rs', 'core/layer0/ops/src/metakernel_parts/058-hybrid-digital-dna-v2.rs']) {
    if (!exists(rel)) violations.push({ kind: 'dna_required_artifact_missing', path: rel, detail: 'Required DNA artifact missing.' });
  }
  const auditText = exists(audit) ? read(audit) : '';
  const lockText = exists(lock) ? read(lock) : '';
  const srsText = exists(srs) ? read(srs) : '';
  for (const token of ['not yet graduated', 'Graduation criteria', 'Every instance', 'Critical actions emit receipts', 'Sentinel checks DNA integrity']) {
    if (!auditText.includes(token)) violations.push({ kind: 'dna_audit_graduation_token_missing', path: audit, detail: token });
  }
  for (const token of ['Canonical status', 'V6-FOUNDATION-DNA-001', 'V6-FOUNDATION-DNA-002', 'V13-DNA-INTEGRITY-GATE-001', 'runtime substrate proof']) {
    if (!lockText.includes(token)) violations.push({ kind: 'dna_lock_token_missing', path: lock, detail: token });
  }
  for (const token of ['V6-FOUNDATION-DNA-001', 'V6-FOUNDATION-DNA-002', 'V13-DNA-INTEGRITY-GATE-001']) {
    if (!srsText.includes(token)) violations.push({ kind: 'dna_srs_row_missing', path: srs, detail: token });
  }
  const payload = { ok: violations.length === 0, type: 'digital_dna_foundation_graduation_guard', generated_at: new Date().toISOString(), strict, violations };
  ensureDir(outJson); fs.writeFileSync(abs(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMd); fs.writeFileSync(abs(outMd), `# Digital DNA Foundation Graduation Guard\n\n- ok: ${payload.ok}\n- violations: ${violations.length}\n\n${violations.map((v) => `- ${v.kind}: ${v.path} ${v.detail}`).join('\n') || '- none'}\n`);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !payload.ok) process.exit(1);
}
main();
