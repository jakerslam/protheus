#!/usr/bin/env node
/**
 * Runtime telemetry (opt-in only).
 * Emits anonymous usage/crash/perf events when explicitly enabled by policy.
 */
import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

type JsonMap = Record<string, any>;

const CLIENT_ROOT = path.resolve(__dirname, '..', '..');
const ROOT = path.resolve(CLIENT_ROOT, '..');
const DEFAULT_POLICY_PATH = path.join(CLIENT_ROOT, 'config', 'runtime_telemetry_policy.json');

function nowIso(): string {
  return new Date().toISOString();
}

function rel(p: string): string {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function ensureParent(filePath: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function readJson(filePath: string, fallback: any): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath: string, payload: any): void {
  ensureParent(filePath);
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function appendJsonl(filePath: string, payload: any): void {
  ensureParent(filePath);
  fs.appendFileSync(filePath, `${JSON.stringify(payload)}\n`);
}

function parseArgs(argv: string[]): { command: string; params: JsonMap } {
  const args = argv.slice(2);
  const command = args.length > 0 && !args[0].startsWith('--') ? args[0] : 'status';
  const params: JsonMap = {};
  for (const raw of args) {
    if (!raw.startsWith('--')) continue;
    const [key, value = '1'] = raw.slice(2).split('=');
    params[key] = value;
  }
  return { command, params };
}

function stableHash(input: string): string {
  return crypto.createHash('sha256').update(input).digest('hex');
}

function sanitizeToken(v: string, max = 64): string {
  return String(v || '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]/g, '_')
    .slice(0, max);
}

function toNumber(v: any, fallback = 0): number {
  const n = Number(v);
  return Number.isFinite(n) ? n : fallback;
}

function defaultPolicy(): JsonMap {
  return {
    schema_id: 'runtime_telemetry_policy',
    schema_version: '1.0.0',
    enabled: false,
    paths: {
      events_path: 'client/runtime/local/state/observability/runtime_telemetry.jsonl',
      aggregate_path: 'client/runtime/local/state/observability/runtime_telemetry_latest.json'
    },
    privacy: {
      salt_env_var: 'PROTHEUS_TELEMETRY_SALT',
      allow_raw_identifiers: false
    }
  };
}

function loadPolicy(policyPath: string): JsonMap {
  const base = defaultPolicy();
  const raw = readJson(policyPath, base);
  return {
    ...base,
    ...raw,
    paths: {
      ...base.paths,
      ...(raw && raw.paths && typeof raw.paths === 'object' ? raw.paths : {})
    },
    privacy: {
      ...base.privacy,
      ...(raw && raw.privacy && typeof raw.privacy === 'object' ? raw.privacy : {})
    }
  };
}

function hostFingerprint(policy: JsonMap): string {
  const saltEnv = String(policy.privacy?.salt_env_var || 'PROTHEUS_TELEMETRY_SALT');
  const salt = String(process.env[saltEnv] || process.env.PROTHEUS_TELEMETRY_SALT || 'telemetry-default-salt');
  const host = `${process.platform}|${process.arch}|${process.version}|${process.pid}`;
  return stableHash(`${salt}|${host}`).slice(0, 24);
}

function eventsPath(policy: JsonMap): string {
  return path.resolve(ROOT, String(policy.paths.events_path || defaultPolicy().paths.events_path));
}

function aggregatePath(policy: JsonMap): string {
  return path.resolve(ROOT, String(policy.paths.aggregate_path || defaultPolicy().paths.aggregate_path));
}

function emitDisabled(policy: JsonMap): void {
  process.stdout.write(
    `${JSON.stringify(
      {
        schema_id: 'runtime_telemetry_emit_result',
        schema_version: '1.0.0',
        ts: nowIso(),
        ok: false,
        emitted: false,
        reason: 'telemetry_disabled',
        policy_path: rel(path.resolve(ROOT, process.env.RUNTIME_TELEMETRY_POLICY_PATH || DEFAULT_POLICY_PATH))
      },
      null,
      2
    )}\n`
  );
}

function updateAggregate(policy: JsonMap, event: JsonMap): JsonMap {
  const outPath = aggregatePath(policy);
  const prev = readJson(outPath, {
    schema_id: 'runtime_telemetry_aggregate',
    schema_version: '1.0.0',
    generated_at: nowIso(),
    usage: {},
    crash_count: 0,
    perf: {}
  });
  const next: JsonMap = {
    ...prev,
    generated_at: nowIso()
  };

  if (event.kind === 'usage') {
    const key = sanitizeToken(event.counter || 'unknown');
    const before = toNumber(next.usage[key], 0);
    next.usage[key] = before + toNumber(event.value, 1);
  } else if (event.kind === 'crash') {
    next.crash_count = toNumber(next.crash_count, 0) + 1;
  } else if (event.kind === 'perf') {
    const metric = sanitizeToken(event.metric || 'latency_ms');
    const component = sanitizeToken(event.component || 'unknown');
    const composite = `${component}.${metric}`;
    const prevStat = next.perf[composite] || { samples: 0, avg: 0 };
    const samples = toNumber(prevStat.samples, 0) + 1;
    const avg = ((toNumber(prevStat.avg, 0) * (samples - 1)) + toNumber(event.value, 0)) / Math.max(samples, 1);
    next.perf[composite] = { samples, avg: Number(avg.toFixed(3)) };
  }

  writeJson(outPath, next);
  return next;
}

function emitEvent(kind: 'usage' | 'crash' | 'perf', params: JsonMap, policy: JsonMap): void {
  if (policy.enabled !== true) {
    emitDisabled(policy);
    return;
  }

  const event: JsonMap = {
    schema_id: 'runtime_telemetry_event',
    schema_version: '1.0.0',
    ts: nowIso(),
    event_id: stableHash(`${Date.now()}|${Math.random()}`).slice(0, 24),
    kind,
    host_fingerprint: hostFingerprint(policy)
  };

  if (kind === 'usage') {
    event.counter = sanitizeToken(params.counter || params.name || 'unknown');
    event.value = Math.max(1, Math.round(toNumber(params.value, 1)));
  } else if (kind === 'crash') {
    event.component = sanitizeToken(params.component || 'unknown');
    event.reason = sanitizeToken(params.reason || 'unknown', 128);
    event.severity = sanitizeToken(params.severity || 'error', 32);
  } else if (kind === 'perf') {
    event.component = sanitizeToken(params.component || 'unknown');
    event.metric = sanitizeToken(params.metric || 'latency_ms', 64);
    event.value = Number(toNumber(params.value, 0).toFixed(4));
    event.unit = sanitizeToken(params.unit || 'ms', 16);
  }

  const outEvents = eventsPath(policy);
  appendJsonl(outEvents, event);
  const aggregate = updateAggregate(policy, event);

  process.stdout.write(
    `${JSON.stringify(
      {
        schema_id: 'runtime_telemetry_emit_result',
        schema_version: '1.0.0',
        ts: nowIso(),
        ok: true,
        emitted: true,
        event,
        events_path: rel(outEvents),
        aggregate_path: rel(aggregatePath(policy)),
        aggregate_snapshot: aggregate
      },
      null,
      2
    )}\n`
  );
}

function status(policy: JsonMap): void {
  const out = {
    schema_id: 'runtime_telemetry_status',
    schema_version: '1.0.0',
    ts: nowIso(),
    enabled: policy.enabled === true,
    events_path: rel(eventsPath(policy)),
    aggregate_path: rel(aggregatePath(policy)),
    events_exists: fs.existsSync(eventsPath(policy)),
    aggregate_exists: fs.existsSync(aggregatePath(policy)),
    privacy: policy.privacy
  };
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function main(): void {
  const { command, params } = parseArgs(process.argv);
  const policyPath = path.resolve(ROOT, String(process.env.RUNTIME_TELEMETRY_POLICY_PATH || DEFAULT_POLICY_PATH));
  const policy = loadPolicy(policyPath);
  if (command === 'status') return status(policy);
  if (command === 'emit-usage') return emitEvent('usage', params, policy);
  if (command === 'emit-crash') return emitEvent('crash', params, policy);
  if (command === 'emit-perf') return emitEvent('perf', params, policy);
  process.stderr.write(
    'usage: node client/lib/ts_entrypoint.js client/runtime/systems/observability/runtime_telemetry_optin.ts <status|emit-usage|emit-crash|emit-perf> [--counter=.. --value=..]\n'
  );
  process.exit(1);
}

main();
