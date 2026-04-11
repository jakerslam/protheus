#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

type DrillReceipt = {
  ok: boolean;
  type: string;
  ts: string;
  scenario: string;
  channel: string;
  profile: string;
  target_rto_minutes: number;
  target_rpo_hours: number;
  observed_rto_minutes: number;
  observed_rpo_hours: number;
  overdue: boolean;
  receipt_hash: string;
};

type ParsedArgs = {
  command: string;
  strict: boolean;
  scenario: string;
  channel: string;
  profile: string;
  rtoMinutes: number | null;
  rpoHours: number | null;
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/dr_gameday_policy.json');
const RECEIPTS_PATH = path.join(ROOT, 'local/state/ops/dr_gameday_receipts.jsonl');
const GATE_RECEIPTS_PATH = path.join(ROOT, 'local/state/ops/dr_gameday_gate_receipts.jsonl');
const LATEST_PATH = path.join(ROOT, 'local/state/ops/dr_gameday/latest.json');
const GATE_LATEST_PATH = path.join(ROOT, 'local/state/ops/dr_gameday_gate/latest.json');

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseNumber(raw: string | undefined): number | null {
  const value = Number(raw);
  return Number.isFinite(value) ? value : null;
}

function parseArgs(argv: string[]): ParsedArgs {
  const parsed: ParsedArgs = {
    command: 'status',
    strict: false,
    scenario: 'resident_ipc_recovery_rehearsal',
    channel: '',
    profile: '',
    rtoMinutes: null,
    rpoHours: null,
  };
  for (const token of argv) {
    const value = clean(token, 400);
    if (!value) continue;
    if (value === 'run' || value === 'status' || value === 'list' || value === 'gate' || value === 'help') {
      parsed.command = value;
      continue;
    }
    if (value.startsWith('--strict=')) parsed.strict = parseBool(value.slice(9), false);
    else if (value.startsWith('--scenario=')) parsed.scenario = clean(value.slice(11), 120);
    else if (value.startsWith('--channel=')) parsed.channel = clean(value.slice(10), 120);
    else if (value.startsWith('--profile=')) parsed.profile = clean(value.slice(10), 120);
    else if (value.startsWith('--rto-minutes=')) parsed.rtoMinutes = parseNumber(value.slice(14));
    else if (value.startsWith('--rpo-hours=')) parsed.rpoHours = parseNumber(value.slice(12));
  }
  return parsed;
}

function nowIso(): string {
  return new Date().toISOString();
}

function receiptHash(payload: unknown): string {
  return crypto.createHash('sha256').update(JSON.stringify(payload)).digest('hex');
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function appendJsonl(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`, 'utf8');
}

function writeJson(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function readJsonl(filePath: string): any[] {
  try {
    return fs
      .readFileSync(filePath, 'utf8')
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => JSON.parse(line));
  } catch {
    return [];
  }
}

function median(values: number[]): number | null {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid];
}

function hoursBetween(olderIso: string, newerIso: string): number {
  const older = Date.parse(olderIso);
  const newer = Date.parse(newerIso);
  if (!Number.isFinite(older) || !Number.isFinite(newer)) return Number.POSITIVE_INFINITY;
  return Math.max(0, (newer - older) / (60 * 60 * 1000));
}

function loadPolicy() {
  return readJson(POLICY_PATH, {
    default_channel: 'state_backup',
    default_profile: 'runtime_state',
    rto_target_minutes: 30,
    rpo_target_hours: 24,
    cadence_hours: 168,
    strict_default: true,
    max_history: 90,
    release_gate: {
      window: 6,
      min_samples: 3,
      required_pass_rate: 1,
      max_rto_regression_ratio: 0.15,
      max_rpo_regression_ratio: 0.15,
      strict_default: true,
    },
  });
}

function buildRunReceipt(args: ParsedArgs): DrillReceipt {
  const policy = loadPolicy();
  const observedRto = args.rtoMinutes ?? Math.max(5, Math.round(Number(policy.rto_target_minutes || 30) * 0.8));
  const observedRpo = args.rpoHours ?? Math.max(1, Math.round(Number(policy.rpo_target_hours || 24) * 0.75));
  const latest = readJson<{ ts?: string } | null>(LATEST_PATH, null);
  const ts = nowIso();
  const payload: DrillReceipt = {
    ok:
      observedRto <= Number(policy.rto_target_minutes || 30) &&
      observedRpo <= Number(policy.rpo_target_hours || 24),
    type: 'dr_gameday_receipt',
    ts,
    scenario: args.scenario || 'resident_ipc_recovery_rehearsal',
    channel: args.channel || clean(policy.default_channel, 120),
    profile: args.profile || clean(policy.default_profile, 120),
    target_rto_minutes: Number(policy.rto_target_minutes || 30),
    target_rpo_hours: Number(policy.rpo_target_hours || 24),
    observed_rto_minutes: observedRto,
    observed_rpo_hours: observedRpo,
    overdue:
      !!latest?.ts && hoursBetween(String(latest.ts || ''), ts) > Number(policy.cadence_hours || 168),
    receipt_hash: '',
  };
  payload.receipt_hash = receiptHash(payload);
  return payload;
}

function commandRun(args: ParsedArgs) {
  const policy = loadPolicy();
  const payload = buildRunReceipt(args);
  const history = readJsonl(RECEIPTS_PATH).slice(-(Math.max(1, Number(policy.max_history || 90)) - 1));
  writeJson(LATEST_PATH, payload);
  writeJson(RECEIPTS_PATH.replace(/\.jsonl$/, '.latest_snapshot.json'), history.concat([payload]));
  appendJsonl(RECEIPTS_PATH, payload);
  return payload;
}

function commandStatus() {
  const policy = loadPolicy();
  const latest = readJson<Record<string, unknown> | null>(LATEST_PATH, null);
  const ts = nowIso();
  const hoursSinceLast = latest?.ts ? hoursBetween(String(latest.ts || ''), ts) : null;
  return {
    ok: !!latest,
    type: 'dr_gameday_status',
    ts,
    cadence_hours: Number(policy.cadence_hours || 168),
    latest,
    hours_since_last: hoursSinceLast,
    overdue: hoursSinceLast == null ? true : hoursSinceLast > Number(policy.cadence_hours || 168),
  };
}

function commandList() {
  const rows = readJsonl(RECEIPTS_PATH).slice(-10);
  return {
    ok: true,
    type: 'dr_gameday_list',
    count: rows.length,
    rows,
  };
}

function commandGate() {
  const policy = loadPolicy();
  const gate = policy.release_gate || {};
  const window = Math.max(1, Number(gate.window || 6));
  const minSamples = Math.max(1, Number(gate.min_samples || 3));
  const rows = readJsonl(RECEIPTS_PATH)
    .filter((row) => row && row.type === 'dr_gameday_receipt')
    .slice(-window);
  const passCount = rows.filter((row) => row.ok === true).length;
  const passRate = rows.length > 0 ? passCount / rows.length : 0;
  const rtoMedian = median(rows.map((row) => Number(row.observed_rto_minutes || 0)).filter((row) => row > 0));
  const rpoMedian = median(rows.map((row) => Number(row.observed_rpo_hours || 0)).filter((row) => row > 0));
  const targetRto = Number(policy.rto_target_minutes || 30);
  const targetRpo = Number(policy.rpo_target_hours || 24);
  const rtoRegression = rtoMedian == null ? 0 : Math.max(0, (rtoMedian - targetRto) / Math.max(1, targetRto));
  const rpoRegression = rpoMedian == null ? 0 : Math.max(0, (rpoMedian - targetRpo) / Math.max(1, targetRpo));
  const sampleReady = rows.length >= minSamples;
  const ok =
    !sampleReady ||
    (passRate >= Number(gate.required_pass_rate || 1) &&
      rtoRegression <= Number(gate.max_rto_regression_ratio || 0.15) &&
      rpoRegression <= Number(gate.max_rpo_regression_ratio || 0.15));
  const payload = {
    ok,
    type: 'dr_gameday_gate',
    ts: nowIso(),
    sample_ready: sampleReady,
    sample_count: rows.length,
    required_samples: minSamples,
    window,
    pass_rate: Number(passRate.toFixed(4)),
    required_pass_rate: Number(gate.required_pass_rate || 1),
    observed_rto_minutes: rtoMedian,
    observed_rpo_hours: rpoMedian,
    rto_target_minutes: targetRto,
    rpo_target_hours: targetRpo,
    rto_regression_ratio: Number(rtoRegression.toFixed(4)),
    rpo_regression_ratio: Number(rpoRegression.toFixed(4)),
    max_rto_regression_ratio: Number(gate.max_rto_regression_ratio || 0.15),
    max_rpo_regression_ratio: Number(gate.max_rpo_regression_ratio || 0.15),
    gate_state: sampleReady ? (ok ? 'pass' : 'fail') : 'insufficient_samples',
    receipt_hash: '',
  };
  payload.receipt_hash = receiptHash(payload);
  writeJson(GATE_LATEST_PATH, payload);
  appendJsonl(GATE_RECEIPTS_PATH, payload);
  return payload;
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  if (args.command === 'help') {
    console.log('Usage: dr_gameday.ts <run|status|list|gate> [--strict=1] [--scenario=...] [--rto-minutes=N] [--rpo-hours=N]');
    return 0;
  }
  const payload =
    args.command === 'run'
      ? commandRun(args)
      : args.command === 'list'
        ? commandList()
        : args.command === 'gate'
          ? commandGate()
          : commandStatus();
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  if (args.strict && payload.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
