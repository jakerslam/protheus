#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const OUT = 'core/local/artifacts/assurance_envelope_guard_current.json';
const REQUIRED = ['type', 'schema_version', 'generated_at', 'domain', 'source', 'source_kind', 'authority_class', 'signal_class', 'subject', 'status', 'evidence', 'freshness'];
const DOMAINS = new Set(['validation', 'observability', 'governance']);
const SIGNALS = new Set(['hard_gate', 'advisory', 'diagnostic']);

function readJson<T>(rel: string): T {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, rel), 'utf8')) as T;
}

function readFlag(name: string): string | undefined {
  const prefix = `--${name}=`;
  const value = process.argv.find((arg) => arg.startsWith(prefix));
  return value ? value.slice(prefix.length) : undefined;
}

function writeJson(rel: string, payload: unknown) {
  const abs = path.resolve(ROOT, rel);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
}

function validateEnvelope(row: any): string[] {
  const failures: string[] = [];
  for (const key of REQUIRED) if (!(key in row)) failures.push(`missing_${key}`);
  if (row.type !== 'assurance_evidence') failures.push('invalid_type');
  if (row.schema_version !== 1) failures.push('invalid_schema_version');
  if (!DOMAINS.has(row.domain)) failures.push('invalid_domain');
  if (!SIGNALS.has(row.signal_class)) failures.push('invalid_signal_class');
  if (!Array.isArray(row.evidence) || row.evidence.length === 0) failures.push('missing_evidence_refs');
  if (!row.source || typeof row.source !== 'object' || !row.source.id) failures.push('missing_source_id');
  if (!row.freshness || typeof row.freshness !== 'object') failures.push('missing_freshness_object');
  if (row.freshness && typeof row.freshness.observed !== 'boolean') failures.push('missing_freshness_observed');
  if (row.freshness && typeof row.freshness.stale !== 'boolean') failures.push('missing_freshness_stale');
  return failures;
}

function run() {
  const strict = process.argv.includes('--strict') || process.argv.includes('--strict=1');
  const schemaPath = readFlag('schema') || 'observability/evidence_normalization/assurance_evidence_envelope.schema.json';
  const schema: any = readJson(schemaPath);
  const failures: Array<{ id: string; detail: string }> = [];
  for (const key of REQUIRED) {
    if (!schema.required.includes(key)) failures.push({ id: 'schema.required', detail: `schema_missing_required_${key}` });
  }
  const positive = {
    type: 'assurance_evidence',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    domain: 'observability',
    source_kind: 'sentinel_stream',
    source: { id: 'kernel_sentinel.runtime_observations' },
    authority_class: 'deterministic_kernel_authority',
    signal_class: 'hard_gate',
    subject: 'sample',
    status: 'fail',
    evidence: ['local/state/kernel_sentinel/evidence/runtime_observations.jsonl'],
    freshness: { observed: true, stale: false, age_seconds: 0 },
  };
  const positiveFailures = validateEnvelope(positive);
  if (positiveFailures.length) failures.push({ id: 'positive_fixture', detail: positiveFailures.join(',') });
  for (const key of ['authority_class', 'signal_class', 'evidence', 'freshness']) {
    const negative = { ...positive } as any;
    delete negative[key];
    if (validateEnvelope(negative).length === 0) failures.push({ id: `negative_fixture.${key}`, detail: 'missing_required_field_was_accepted' });
  }
  const payload = {
    ok: failures.length === 0,
    type: 'assurance_envelope_guard',
    generated_at: new Date().toISOString(),
    strict,
    summary: {
      schema_path: schemaPath,
      required_fields: REQUIRED.length,
      failures: failures.length,
    },
    failures,
  };
  writeJson(OUT, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && failures.length) process.exit(1);
}

run();
