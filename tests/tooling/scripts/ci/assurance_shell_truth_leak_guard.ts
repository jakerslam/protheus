#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const OUT = 'core/local/artifacts/assurance_shell_truth_leak_guard_current.json';

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

function has(values: unknown, value: string): boolean {
  return Array.isArray(values) && values.includes(value);
}

function run() {
  const strict = process.argv.includes('--strict') || process.argv.includes('--strict=1');
  const consumerPath = readFlag('consumer-contract') || 'validation/conformance/contracts/assurance_consumer_boundary_contract.json';
  const observabilityPath = readFlag('observability-registry') || 'observability/source_coverage/assurance_observability_registry.json';
  const consumer: any = readJson(consumerPath);
  const observability: any = readJson(observabilityPath);
  const failures: Array<{ id: string; detail: string }> = [];
  const shell = (consumer.consumers || []).find((row: any) => row.id === 'consumer.shell');
  if (!shell) failures.push({ id: 'consumer.shell', detail: 'missing_shell_consumer' });
  if (shell) {
    for (const forbidden of ['infer_health_truth', 'infer_release_readiness', 'branch_on_raw_assurance_payload', 'cache_full_assurance_evidence', 'waive_assurance_gate']) {
      if (!has(shell.forbidden_actions, forbidden)) failures.push({ id: 'consumer.shell', detail: `missing_forbidden_${forbidden}` });
    }
    for (const required of ['summary', 'signal_class', 'status', 'freshness_status', 'detail_ref']) {
      if (!has(shell.required_fields_when_consuming, required)) failures.push({ id: 'consumer.shell', detail: `missing_required_projection_${required}` });
    }
  }
  const shellTelemetry = (observability.entries || []).find((row: any) => row.source_class === 'presentation_telemetry_stream');
  if (!shellTelemetry) failures.push({ id: 'observability.shell_telemetry', detail: 'missing_presentation_telemetry_stream' });
  if (shellTelemetry) {
    if (shellTelemetry.can_open_finding) failures.push({ id: shellTelemetry.id, detail: 'shell_telemetry_can_open_finding' });
    if (shellTelemetry.can_block_release) failures.push({ id: shellTelemetry.id, detail: 'shell_telemetry_can_block_release' });
    if (!shellTelemetry.corroboration_required_for_finding) failures.push({ id: shellTelemetry.id, detail: 'missing_corroboration_requirement' });
    if (shellTelemetry.authority_class !== 'presentation_telemetry_only') failures.push({ id: shellTelemetry.id, detail: 'shell_telemetry_authority_not_presentation_only' });
  }
  const payload = {
    ok: failures.length === 0,
    type: 'assurance_shell_truth_leak_guard',
    generated_at: new Date().toISOString(),
    strict,
    summary: {
      consumer_contract_path: consumerPath,
      observability_registry_path: observabilityPath,
      shell_consumer_present: Boolean(shell),
      shell_telemetry_present: Boolean(shellTelemetry),
      failures: failures.length,
    },
    failures,
  };
  writeJson(OUT, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && failures.length) process.exit(1);
}

run();
