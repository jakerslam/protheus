#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const OUT_JSON = 'core/local/artifacts/digital_dna_integrity_gate_current.json';
const OUT_MD = 'local/workspace/reports/DIGITAL_DNA_INTEGRITY_GATE_CURRENT.md';

type Check = {
  id: string;
  ok: boolean;
  severity: 'hard' | 'advisory';
  detail: string;
};

function flag(name: string): string | undefined {
  const prefix = `--${name}=`;
  const direct = process.argv.slice(2).find((arg) => arg.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 ? process.argv[idx + 1] : undefined;
}

function boolFlag(name: string, fallback = false): boolean {
  const value = flag(name);
  if (value === undefined) return fallback;
  return value === '1' || value === 'true';
}

function repo(rel: string): string {
  return path.join(ROOT, rel);
}

function read(rel: string): string {
  try {
    return fs.readFileSync(repo(rel), 'utf8');
  } catch {
    return '';
  }
}

function exists(rel: string): boolean {
  return fs.existsSync(repo(rel));
}

function has(rel: string, needles: string[]): boolean {
  const text = read(rel);
  return needles.every((needle) => text.includes(needle));
}

function check(id: string, ok: boolean, detail: string, severity: 'hard' | 'advisory' = 'hard'): Check {
  return { id, ok, severity, detail };
}

function srsStatus(id: string): string | null {
  const match = read('docs/workspace/SRS.md').match(
    new RegExp(`^\\|\\s*${id}\\s*\\|\\s*([^|]+?)\\s*\\|`, 'm'),
  );
  return match ? match[1].trim() : null;
}

function todoHasYellowFlag(): boolean {
  const registry = read('docs/workspace/todo/todo_registry.json');
  return (
    registry.includes('"id": "DNA-FOUNDATION-AUDIT"') &&
    registry.includes('"section": "yellow"') &&
    registry.includes('Digital DNA Foundation Graduation Audit')
  );
}

function buildChecks(): Check[] {
  const foundation = 'core/layer0/ops/src/metakernel_parts/057-digital-dna-foundation.rs';
  const hybrid = 'core/layer0/ops/src/metakernel_parts/058-hybrid-digital-dna-v2.rs';
  const foundationParts = 'core/layer0/ops/src/metakernel_parts/057-digital-dna-foundation_parts';
  const hybridParts = 'core/layer0/ops/src/metakernel_parts/058-hybrid-digital-dna-v2.rs.parts';
  const run = 'core/layer0/ops/src/metakernel_parts/060-run_parts/020-run.rs';
  const tests = 'core/layer0/ops/src/metakernel_parts/060-run_parts/030-mod-tests.rs';
  const usage = 'core/layer0/ops/src/ops_main_usage.rs';

  const status1 = srsStatus('V6-FOUNDATION-DNA-001');
  const status2 = srsStatus('V6-FOUNDATION-DNA-002');
  const graduated = [status1, status2].every((status) =>
    status === 'done' || status === 'existing-coverage-validated',
  );
  const yellowFlag = todoHasYellowFlag();

  return [
    check('dna.foundation.wrapper.exists', exists(foundation), `${foundation} must exist`),
    check('dna.hybrid.wrapper.exists', exists(hybrid), `${hybrid} must exist`),
    check(
      'dna.foundation.parts.exist',
      ['010-use-serde-deserialize-serialize.rs', '020-repair-letter-with-complement-check.rs', '030-run-digital-dna-create.rs', '040-run-digital-dna-status.rs'].every((file) =>
        exists(`${foundationParts}/${file}`),
      ),
      'Digital DNA v1 part files must remain present',
    ),
    check(
      'dna.hybrid.parts.exist',
      ['010-segment.rs', '020-segment.rs', '030-segment.rs', '040-segment.rs'].every((file) =>
        exists(`${hybridParts}/${file}`),
      ),
      'Hybrid DNA v2 part files must remain present',
    ),
    check(
      'dna.metakernel.includes',
      has('core/layer0/ops/src/metakernel.rs', [
        '057-digital-dna-foundation.rs',
        '058-hybrid-digital-dna-v2.rs',
      ]),
      'Metakernel must include v1 and v2 DNA modules',
    ),
    check(
      'dna.foundation.primitives',
      has(`${foundationParts}/010-use-serde-deserialize-serialize.rs`, [
        'struct Quark',
        'struct Baryon',
        'struct Letter',
        'struct Codon',
        'struct Gene',
        'struct InstanceDna',
        'fn derive_verity',
        'fn validate_instance_dna',
      ]),
      'Primitive DNA ladder and verity validation must remain intact',
    ),
    check(
      'dna.foundation.receipts',
      has(`${foundationParts}/020-repair-letter-with-complement-check.rs`, [
        'fn write_digital_dna_receipt',
        '"instance_dna_ref"',
        '"digital_dna_receipt"',
      ]),
      'Digital DNA receipts must include instance_dna_ref',
    ),
    check(
      'dna.subservience.lock',
      has(`${foundationParts}/020-repair-letter-with-complement-check.rs`, [
        'fn evaluate_subservience',
        'parent_signature_mismatch',
        'metakernel_judicial_lock',
      ]),
      'Subservience mismatch must remain fail-closed through judicial lock',
    ),
    check(
      'dna.hybrid.integrity',
      has(`${hybridParts}/010-segment.rs`, [
        'fn gene_merkle_root',
        'fn validate_commit_link',
        'HYBRID_COMMIT_WORM_SUPERSESSION',
      ]),
      'Hybrid DNA must keep Merkle, commit-chain, and WORM integrity primitives',
    ),
    check(
      'dna.commands.routed',
      has(run, [
        'dna-status',
        'dna-create',
        'dna-mutate',
        'dna-enforce-subservience',
        'dna-hybrid-status',
        'dna-hybrid-commit',
        'dna-hybrid-verify',
        'dna-hybrid-worm-supersede',
        'dna-hybrid-worm-mutate',
        'microkernel-safety',
        'evaluate_subservience',
      ]),
      'Metakernel command router must expose DNA and enforce subservience on critical safety calls',
    ),
    check(
      'dna.usage.surface',
      has(usage, ['dna-status', 'dna-create', 'dna-hybrid-verify', 'dna-hybrid-protected-lineage']),
      'Operator usage surface must list DNA command family',
    ),
    check(
      'dna.tests.present',
      has(tests, [
        'metakernel_run_dispatches_digital_dna_commands',
        'microkernel_safety_enforces_subservience_when_parent_signature_is_supplied',
        'metakernel_dispatches_hybrid_dna_v2_commands',
      ]) &&
        has(`${foundationParts}/040-run-digital-dna-status.rs`, [
          'letter_validation_rejects_invalid_verity',
          'subservience_mismatch_triggers_judicial_lock',
        ]) &&
        has(`${hybridParts}/040-segment.rs`, [
          'hybrid_valid_commit_chain_example',
          'hybrid_invalid_commit_chain_example',
          'hybrid_judicial_lock_invalid_worm_mutation_example',
        ]),
      'DNA regression and sovereignty tests must remain present',
    ),
    check(
      'dna.srs.rows.present',
      Boolean(status1 && status2),
      `SRS rows must exist; current statuses: ${status1 ?? 'missing'}, ${status2 ?? 'missing'}`,
    ),
    check(
      'dna.graduation.debt.visible',
      graduated || yellowFlag,
      `DNA graduation debt must be resolved or yellow-flagged; statuses: ${status1 ?? 'missing'}, ${status2 ?? 'missing'}`,
    ),
    check(
      'dna.universal.substrate.advisory',
      graduated,
      'Digital DNA is implemented as metakernel capability but not yet proven as universal unavoidable substrate',
      'advisory',
    ),
  ];
}

function ensureDir(rel: string): void {
  fs.mkdirSync(path.dirname(repo(rel)), { recursive: true });
}

function writeArtifacts(payload: Record<string, unknown>, checks: Check[], outJson: string, outMd: string): void {
  ensureDir(outJson);
  fs.writeFileSync(repo(outJson), `${JSON.stringify(payload, null, 2)}\n`);
  ensureDir(outMd);
  const lines = [
    '# Digital DNA Integrity Gate',
    '',
    `- ok: ${payload.ok}`,
    `- hard_failures: ${payload.hard_failures}`,
    `- advisory_failures: ${payload.advisory_failures}`,
    '',
    '## Checks',
    ...checks.map((row) => `- ${row.ok ? 'PASS' : 'FAIL'} ${row.id} (${row.severity}) - ${row.detail}`),
    '',
  ];
  fs.writeFileSync(repo(outMd), lines.join('\n'));
}

function main(): void {
  const strict = boolFlag('strict', true);
  const outJson = flag('out-json') || OUT_JSON;
  const outMd = flag('out-markdown') || OUT_MD;
  const checks = buildChecks();
  const hardFailures = checks.filter((row) => row.severity === 'hard' && !row.ok);
  const advisoryFailures = checks.filter((row) => row.severity === 'advisory' && !row.ok);
  const payload = {
    ok: hardFailures.length === 0,
    type: 'digital_dna_integrity_gate',
    strict,
    hard_failures: hardFailures.length,
    advisory_failures: advisoryFailures.length,
    checks,
    artifacts: { out_json: outJson, out_markdown: outMd },
  };
  writeArtifacts(payload, checks, outJson, outMd);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && hardFailures.length > 0) process.exit(1);
}

main();
