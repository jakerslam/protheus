#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';

type ParsedArgs = {
  command: string;
  counter: string;
  component: string;
  reason: string;
  metric: string;
  value: number;
};

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/runtime_telemetry_policy.json');

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseNumber(raw: string | undefined, fallback = 0): number {
  const value = Number(raw);
  return Number.isFinite(value) ? value : fallback;
}

function parseArgs(argv: string[]): ParsedArgs {
  const parsed: ParsedArgs = {
    command: 'status',
    counter: 'unknown_counter',
    component: 'unknown_component',
    reason: 'unspecified',
    metric: 'unknown_metric',
    value: 0,
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 400);
    if (!token) continue;
    if (token === 'status' || token === 'emit-usage' || token === 'emit-crash' || token === 'emit-perf') {
      parsed.command = token;
      continue;
    }
    if (token.startsWith('--counter=')) parsed.counter = clean(token.slice(10), 120);
    else if (token.startsWith('--component=')) parsed.component = clean(token.slice(12), 120);
    else if (token.startsWith('--reason=')) parsed.reason = clean(token.slice(9), 160);
    else if (token.startsWith('--metric=')) parsed.metric = clean(token.slice(9), 120);
    else if (token.startsWith('--value=')) parsed.value = parseNumber(token.slice(8), 0);
  }
  return parsed;
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function writeJson(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function loadPolicy() {
  return readJson(POLICY_PATH, {
    enabled: false,
    paths: {
      events_path: 'client/runtime/local/state/observability/runtime_telemetry.jsonl',
      aggregate_path: 'client/runtime/local/state/observability/runtime_telemetry_latest.json',
    },
    privacy: {
      salt_env_var: 'PROTHEUS_TELEMETRY_SALT',
      allow_raw_identifiers: false,
    },
  });
}

function policyPaths(policy: any) {
  return {
    events: path.join(ROOT, clean(policy?.paths?.events_path, 240)),
    aggregate: path.join(ROOT, clean(policy?.paths?.aggregate_path, 240)),
  };
}

function eventEnvelope(type: string, detail: Record<string, unknown>, policy: any) {
  return {
    ok: true,
    type,
    ts: new Date().toISOString(),
    enabled: policy?.enabled === true,
    privacy: {
      allow_raw_identifiers: policy?.privacy?.allow_raw_identifiers === true,
      salt_env_var: clean(policy?.privacy?.salt_env_var, 80),
    },
    detail,
  };
}

function commandStatus(policy: any) {
  const paths = policyPaths(policy);
  const latest = readJson<Record<string, unknown> | null>(paths.aggregate, null);
  return {
    ok: true,
    type: 'runtime_telemetry_status',
    enabled: policy?.enabled === true,
    policy_path: path.relative(ROOT, POLICY_PATH),
    events_path: path.relative(ROOT, paths.events),
    aggregate_path: path.relative(ROOT, paths.aggregate),
    event_log_present: fs.existsSync(paths.events),
    aggregate_present: fs.existsSync(paths.aggregate),
    latest,
  };
}

function commandEmit(parsed: ParsedArgs, policy: any) {
  const paths = policyPaths(policy);
  const detail =
    parsed.command === 'emit-usage'
      ? { counter: parsed.counter, value: parsed.value || 1 }
      : parsed.command === 'emit-crash'
        ? { component: parsed.component, reason: parsed.reason }
        : { component: parsed.component, metric: parsed.metric, value: parsed.value };
  const payload = eventEnvelope(parsed.command.replace('emit-', 'runtime_telemetry_'), detail, policy);
  if (policy?.enabled !== true) {
    return {
      ...payload,
      emitted: false,
      reason: 'telemetry_disabled',
    };
  }
  appendJsonl(paths.events, payload);
  writeJson(paths.aggregate, payload);
  return {
    ...payload,
    emitted: true,
  };
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const policy = loadPolicy();
  const payload =
    parsed.command === 'emit-usage' || parsed.command === 'emit-crash' || parsed.command === 'emit-perf'
      ? commandEmit(parsed, policy)
      : commandStatus(policy);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
