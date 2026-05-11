#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
type Violation = { kind: string; path: string; detail: string };
function flag(name: string, fallback = ''): string { const prefix = `--${name}=`; const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix)); if (direct) return direct.slice(prefix.length); const idx = process.argv.indexOf(`--${name}`); return idx >= 0 ? process.argv[idx + 1] : fallback; }
function boolFlag(name: string, fallback = false): boolean { const raw = flag(name, fallback ? '1' : '0'); return raw === '1' || raw === 'true'; }
function abs(rel: string): string { return path.join(ROOT, rel); }
function exists(rel: string): boolean { return fs.existsSync(abs(rel)); }
function read(rel: string): string { return fs.readFileSync(abs(rel), 'utf8'); }
function json(rel: string): any { return JSON.parse(read(rel)); }
function ensureDir(rel: string): void { fs.mkdirSync(path.dirname(abs(rel)), { recursive: true }); }
function ageMs(value: unknown): number | null { const t = Date.parse(String(value || '')); return Number.isFinite(t) ? Date.now() - t : null; }
function main(): void {
  const strict = boolFlag('strict', true);
  const maxAgeMs = Number(flag('max-age-ms', String(24 * 60 * 60 * 1000)));
  const outJson = flag('out-json', 'core/local/artifacts/kernel_sentinel_fresh_evidence_guard_current.json');
  const outMd = flag('out-markdown', 'local/workspace/reports/KERNEL_SENTINEL_FRESH_EVIDENCE_GUARD_CURRENT.md');
  const policyPath = 'observability/freshness/kernel_sentinel_deterministic_evidence_refresh_policy.md';
  const sourcePath = 'core/layer0/ops/src/kernel_sentinel/evidence.rs';
  const testPath = 'validation/tests/rust/kernel_sentinel/evidence/tests.rs';
  const artifacts = [
    'core/local/artifacts/kernel_sentinel_auto_run_current.json',
    'local/state/kernel_sentinel/kernel_sentinel_final_report_current.json',
  ];
  const violations: Violation[] = [];
  if (!exists(policyPath)) violations.push({ kind: 'freshness_policy_missing', path: policyPath, detail: 'Refresh policy missing.' });
  if (!exists(sourcePath)) violations.push({ kind: 'sentinel_evidence_source_missing', path: sourcePath, detail: 'Sentinel evidence source missing.' });
  const policy = exists(policyPath) ? read(policyPath) : '';
  for (const token of ['Required fresh evidence classes', 'Freshness rule', 'Stale evidence rule', 'stale_historical_evidence_failure']) {
    if (!policy.includes(token)) violations.push({ kind: 'freshness_policy_token_missing', path: policyPath, detail: token });
  }
  const source = exists(sourcePath) ? read(sourcePath) : '';
  for (const token of ['EVIDENCE_FINDING_MAX_AGE_MS', 'raw_record_age_ms', 'stale_historical_evidence_failure', 'freshness://age_seconds']) {
    if (!source.includes(token)) violations.push({ kind: 'freshness_source_token_missing', path: sourcePath, detail: token });
  }
  const tests = exists(testPath) ? read(testPath) : '';
  for (const token of ['stale_generated_at_failures_are_historical_not_current_receipt_blockers', 'stale_historical_evidence_failure', 'freshness://age_seconds']) {
    if (!tests.includes(token)) violations.push({ kind: 'freshness_test_token_missing', path: testPath, detail: token });
  }
  const artifactSummaries: any[] = [];
  for (const artifact of artifacts) {
    if (!exists(artifact)) {
      artifactSummaries.push({ path: artifact, exists: false });
      continue;
    }
    const payload = json(artifact);
    const generatedAge = ageMs(payload.generated_at);
    const stale = generatedAge === null || generatedAge > maxAgeMs;
    const ok = payload.ok === true || payload.status === 'ok' || payload.verdict === 'pass';
    artifactSummaries.push({ path: artifact, exists: true, generated_at: payload.generated_at || null, age_ms: generatedAge, stale, ok, verdict: payload.verdict || payload.status || null });
    if (stale && ok) violations.push({ kind: 'stale_sentinel_artifact_marked_ok', path: artifact, detail: `age_ms=${generatedAge}` });
    if (generatedAge === null) violations.push({ kind: 'sentinel_artifact_missing_generated_at', path: artifact, detail: 'Artifact cannot be freshness-ranked.' });
  }
  const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
  const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'observability', ok: violations.length === 0, type: 'kernel_sentinel_fresh_evidence_guard', generated_at: new Date().toISOString(), strict, max_age_ms: maxAgeMs, artifact_summaries: artifactSummaries, violations };
  ensureDir(outJson); fs.writeFileSync(abs(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMd); fs.writeFileSync(abs(outMd), `# Kernel Sentinel Fresh Evidence Guard\n\n- ok: ${payload.ok}\n- violations: ${violations.length}\n- max_age_ms: ${maxAgeMs}\n\n## Artifacts\n${artifactSummaries.map((a) => `- ${a.path}: exists=${a.exists} stale=${a.stale ?? 'n/a'} ok=${a.ok ?? 'n/a'} age_ms=${a.age_ms ?? 'n/a'}`).join('\n')}\n\n## Violations\n${violations.map((v) => `- ${v.kind}: ${v.path} ${v.detail}`).join('\n') || '- none'}\n`);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !payload.ok) process.exit(1);
}
main();
