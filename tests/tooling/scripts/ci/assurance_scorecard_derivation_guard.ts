#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const OUT = 'core/local/artifacts/assurance_scorecard_derivation_guard_current.json';

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

function scorecardRowHasEvidence(row: any): boolean {
  return Array.isArray(row.evidence) && row.evidence.length > 0;
}

function run() {
  const strict = process.argv.includes('--strict') || process.argv.includes('--strict=1');
  const contractPath = readFlag('scorecard-contract') || 'validation/scorecards/contracts/assurance_scorecard_derivation_contract.json';
  const registry: any = readJson(contractPath);
  const failures: Array<{ id: string; detail: string }> = [];
  if (registry.scorecards_are_derived !== true) failures.push({ id: 'policy', detail: 'scorecards_are_derived_not_true' });
  if (registry.scorecards_must_reference_evidence !== true) failures.push({ id: 'policy', detail: 'scorecards_must_reference_evidence_not_true' });
  const scoreOutput = registry.scorecard_output;
  if (!scoreOutput) failures.push({ id: 'output.release_scorecard', detail: 'missing_scorecard_output' });
  if (scoreOutput && scoreOutput.must_reference_evidence !== true) failures.push({ id: scoreOutput.id, detail: 'scorecard_output_must_reference_evidence_false' });
  const requiredRules = new Set(['scorecard.rows_reference_evidence', 'scorecard.no_manual_truth', 'scorecard.domain_breakdown_required', 'scorecard.stale_or_missing_evidence_visible']);
  for (const row of registry.scorecard_derivation_rules || []) {
    if (row.required === true) requiredRules.delete(row.id);
  }
  for (const id of requiredRules) failures.push({ id, detail: 'missing_required_scorecard_derivation_rule' });
  const positive = { id: 'row.ok', evidence: ['core/local/artifacts/example.json'] };
  const negative = { id: 'row.bad' };
  if (!scorecardRowHasEvidence(positive)) failures.push({ id: 'positive_fixture', detail: 'evidence_row_rejected' });
  if (scorecardRowHasEvidence(negative)) failures.push({ id: 'negative_fixture', detail: 'scorecard_row_without_evidence_accepted' });
  const payload = {
    ok: failures.length === 0,
    type: 'assurance_scorecard_derivation_guard',
    generated_at: new Date().toISOString(),
    strict,
    summary: {
      scorecard_contract_path: contractPath,
      derivation_rules: (registry.scorecard_derivation_rules || []).length,
      scorecard_output_present: Boolean(scoreOutput),
      failures: failures.length,
    },
    failures,
  };
  writeJson(OUT, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && failures.length) process.exit(1);
}

run();
