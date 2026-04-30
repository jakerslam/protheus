#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const OUT = 'core/local/artifacts/assurance_placement_guard_current.json';

type Failure = { id: string; detail: string };

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

function includes(value: unknown, needle: string): boolean {
  return Array.isArray(value) && value.includes(needle);
}

function run() {
  const strict = process.argv.includes('--strict') || process.argv.includes('--strict=1');
  const validationPath = readFlag('validation-registry') || 'validation/conformance/contracts/assurance_validation_registry.json';
  const scorecardPath = readFlag('scorecard-contract') || 'validation/scorecards/contracts/assurance_scorecard_derivation_contract.json';
  const consumerPath = readFlag('consumer-contract') || 'validation/conformance/contracts/assurance_consumer_boundary_contract.json';
  const validation: any = readJson(validationPath);
  const scorecard: any = readJson(scorecardPath);
  const consumer: any = readJson(consumerPath);
  const failures: Failure[] = [];

  for (const entry of validation.entries || []) {
    if (entry.validation_kind === 'eval_suite' && entry.domain !== 'validation') {
      failures.push({ id: entry.id, detail: 'eval_suite_not_owned_by_validation' });
    }
    if (entry.validation_kind === 'benchmark' && entry.domain !== 'validation') {
      failures.push({ id: entry.id, detail: 'benchmark_not_owned_by_validation' });
    }
    if (entry.validation_kind === 'harness_only') {
      if (entry.authority_class !== 'harness_only') failures.push({ id: entry.id, detail: 'harness_authority_not_harness_only' });
      if (entry.lifecycle_state === 'release_gate') failures.push({ id: entry.id, detail: 'harness_only_marked_release_gate' });
      if (entry.signal_class === 'hard_gate') failures.push({ id: entry.id, detail: 'harness_only_marked_hard_gate' });
    }
    if (/surface\/orchestration|orchestration/i.test(String(entry.owner_of_truth || '')) && entry.validation_kind === 'eval_suite' && !entry.harness_only) {
      failures.push({ id: entry.id, detail: 'orchestration_owns_eval_truth' });
    }
  }

  const scorecardOutput = scorecard.scorecard_output;
  if (!scorecardOutput) failures.push({ id: 'governance.scorecard', detail: 'missing_scorecard_output' });
  if (scorecardOutput && scorecardOutput.must_reference_evidence !== true) {
    failures.push({ id: scorecardOutput.id, detail: 'scorecard_output_not_evidence_derived' });
  }

  const consumers = new Map((consumer.consumers || []).map((row: any) => [row.id, row]));
  const orchestration: any = consumers.get('consumer.orchestration');
  const shell: any = consumers.get('consumer.shell');
  const gateway: any = consumers.get('consumer.gateway');
  const kernel: any = consumers.get('consumer.kernel');

  if (!orchestration || !includes(orchestration.forbidden_actions, 'own_eval_definition')) failures.push({ id: 'consumer.orchestration', detail: 'missing_eval_ownership_prohibition' });
  if (!orchestration || !includes(orchestration.forbidden_actions, 'own_release_gate_definition')) failures.push({ id: 'consumer.orchestration', detail: 'missing_release_gate_ownership_prohibition' });
  if (!shell || !includes(shell.forbidden_actions, 'infer_health_truth')) failures.push({ id: 'consumer.shell', detail: 'missing_health_truth_prohibition' });
  if (!gateway || !includes(gateway.forbidden_actions, 'decide_assurance_verdict')) failures.push({ id: 'consumer.gateway', detail: 'missing_verdict_decision_prohibition' });
  if (!kernel || !includes(kernel.forbidden_actions, 'own_fuzzy_eval_scoring')) failures.push({ id: 'consumer.kernel', detail: 'missing_fuzzy_eval_scoring_prohibition' });

  const payload = {
    ok: failures.length === 0,
    type: 'assurance_placement_guard',
    generated_at: new Date().toISOString(),
    strict,
    summary: {
      validation_registry_path: validationPath,
      scorecard_contract_path: scorecardPath,
      consumer_contract_path: consumerPath,
      validation_entries: (validation.entries || []).length,
      scorecard_contract_loaded: scorecard.type === 'validation_scorecard_derivation_contract',
      consumers: (consumer.consumers || []).length,
      failures: failures.length,
    },
    failures,
  };
  writeJson(OUT, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && failures.length) process.exit(1);
}

run();
