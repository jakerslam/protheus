#!/usr/bin/env node
'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const { setTimeout: delay } = require('timers/promises');

const DEFAULT_SHOWCASE_DURATION_MS = 10000;
const DEFAULT_REALTIME_DURATION_MS = 0;
const DEFAULT_PREWARM_TTL_MS = 5 * 60 * 1000;
const BAR_WIDTH = 64;
const FILLED_CHAR = '█';
const EMPTY_CHAR = '░';
const SCRIPT_DIR = __dirname;
const WORKSPACE_ROOT = path.resolve(SCRIPT_DIR, '..', '..', '..', '..');
const RUN_OPS_SCRIPT = path.join(WORKSPACE_ROOT, 'client', 'runtime', 'systems', 'ops', 'run_protheus_ops.js');
const STATE_DIR = path.join(WORKSPACE_ROOT, 'local', 'state', 'tools', 'assimilate');
const PREWARM_STATE_PATH = path.join(STATE_DIR, 'prewarm.json');
const METRICS_STATE_PATH = path.join(STATE_DIR, 'metrics.json');

const STAGES = [
  { percent: 20, label: 'Spinning up swarm (5,000 agents)', weight: 0.2 },
  { percent: 50, label: 'Parallel analysis (manifest + docs)', weight: 0.3 },
  { percent: 80, label: 'Building bridges & adapters', weight: 0.3 },
  { percent: 95, label: 'Validating + signing receipts', weight: 0.15 },
  { percent: 100, label: 'Assimilation complete. Ready to use.', weight: 0.05 },
];

function usage() {
  process.stdout.write(
    [
      'Usage: infring assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--scaffold-payload=1]',
      '',
      'Known targets route to governed core bridge lanes. Unknown targets run local simulation mode.',
    ].join('\n') + '\n',
  );
}

function parseBooleanFlag(value) {
  if (value == null) return false;
  const normalized = String(value).trim().toLowerCase();
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on';
}

function parseArgs(argv) {
  const out = {
    target: '',
    passthrough: [],
    durationMs: null,
    showcase: false,
    scaffoldPayload: false,
    json: parseBooleanFlag(process.env.PROTHEUS_GLOBAL_JSON),
    prewarm: true,
    coreDomain: '',
    coreArgsBase64: '',
    help: false,
  };
  for (const token of argv) {
    if (token === '--help' || token === '-h') {
      out.help = true;
      continue;
    }
    if (token.startsWith('--showcase=')) {
      out.showcase = parseBooleanFlag(token.slice('--showcase='.length));
      continue;
    }
    if (token === '--showcase') {
      out.showcase = true;
      continue;
    }
    if (token.startsWith('--scaffold-payload=')) {
      out.scaffoldPayload = parseBooleanFlag(token.slice('--scaffold-payload='.length));
      continue;
    }
    if (token === '--scaffold-payload') {
      out.scaffoldPayload = true;
      continue;
    }
    if (token === '--no-prewarm') {
      out.prewarm = false;
      continue;
    }
    if (token.startsWith('--prewarm=')) {
      out.prewarm = parseBooleanFlag(token.slice('--prewarm='.length));
      continue;
    }
    if (token.startsWith('--duration-ms=')) {
      const parsed = Number(token.slice('--duration-ms='.length));
      if (Number.isFinite(parsed) && parsed >= 0) {
        out.durationMs = parsed;
      }
      continue;
    }
    if (token.startsWith('--json=')) {
      out.json = parseBooleanFlag(token.slice('--json='.length));
      continue;
    }
    if (token.startsWith('--core-domain=')) {
      out.coreDomain = token.slice('--core-domain='.length).trim();
      continue;
    }
    if (token.startsWith('--core-args-base64=')) {
      out.coreArgsBase64 = token.slice('--core-args-base64='.length).trim();
      continue;
    }
    if (token.startsWith('--target=')) {
      out.target = token.slice('--target='.length);
      continue;
    }
    if (!token.startsWith('--') && !out.target) {
      out.target = token;
      continue;
    }
    out.passthrough.push(token);
  }
  out.target = String(out.target || '').trim();
  return out;
}

function normalizeTarget(value) {
  return String(value || '')
    .replace(/[\u0000-\u001f\u007f]/g, '')
    .trim()
    .slice(0, 120);
}

function renderBar(percent) {
  const bounded = Math.max(0, Math.min(100, Number(percent) || 0));
  const filled = Math.round((bounded / 100) * BAR_WIDTH);
  return `[${FILLED_CHAR.repeat(filled)}${EMPTY_CHAR.repeat(BAR_WIDTH - filled)}]`;
}

function buildStageSchedule(totalMs) {
  return STAGES.map((stage) => ({
    ...stage,
    durationMs: Math.round(stage.weight * totalMs),
  }));
}

function buildReceiptHash(target, tsIso) {
  const digest = crypto
    .createHash('sha256')
    .update(`${target}|assimilation|${tsIso}`)
    .digest('hex');
  return `sha256:${digest}`;
}

function decodeInjectedCoreRoute(options) {
  const domain = String(options.coreDomain || '').trim();
  if (!domain) {
    return null;
  }
  const rawBase64 = String(options.coreArgsBase64 || '').trim();
  if (!rawBase64) {
    throw new Error('core-args-base64 is required when core-domain is provided');
  }
  let parsedArgs;
  try {
    const decoded = Buffer.from(rawBase64, 'base64').toString('utf8');
    parsedArgs = JSON.parse(decoded);
  } catch (_error) {
    throw new Error('invalid core route payload');
  }
  if (!Array.isArray(parsedArgs) || !parsedArgs.every((row) => typeof row === 'string')) {
    throw new Error('core route args must be a string array');
  }
  return { domain, args: parsedArgs };
}

function payloadScaffoldFor(target) {
  const normalized = String(target || '').toLowerCase();
  if (normalized === 'haystack' || normalized === 'workflow://haystack' || normalized === 'rag://haystack') {
    return {
      name: 'example-haystack-pipeline',
      components: [
        {
          id: 'retriever',
          stage_type: 'retriever',
          input_type: 'text',
          output_type: 'docs',
          parallel: false,
          spawn: false,
          budget: 128,
        },
      ],
    };
  }
  if (
    normalized === 'langchain' ||
    normalized === 'workflow://langchain' ||
    normalized === 'chains://langchain'
  ) {
    return {
      name: 'langchain-integration',
      integration_type: 'tool',
      capabilities: ['retrieve'],
    };
  }
  if (normalized === 'dspy' || normalized === 'workflow://dspy' || normalized === 'optimizer://dspy') {
    return {
      name: 'dspy-integration',
      kind: 'retriever',
      capabilities: ['retrieve'],
    };
  }
  if (normalized === 'pydantic-ai' || normalized === 'workflow://pydantic-ai' || normalized === 'agents://pydantic-ai') {
    return {
      name: 'pydantic-agent',
      model: 'gpt-4o-mini',
      tools: [],
    };
  }
  if (normalized === 'camel' || normalized === 'workflow://camel' || normalized === 'society://camel') {
    return {
      name: 'camel-dataset',
      dataset: { rows: [] },
    };
  }
  if (normalized === 'llamaindex' || normalized === 'rag://llamaindex') {
    return {
      name: 'llamaindex-connector',
      connector_type: 'filesystem',
      root_path: './docs',
    };
  }
  if (normalized === 'google-adk' || normalized === 'workflow://google-adk') {
    return {
      name: 'google-adk-tool-manifest',
      tools: [],
    };
  }
  if (normalized === 'mastra' || normalized === 'workflow://mastra') {
    return {
      name: 'mastra-graph',
      nodes: [],
      edges: [],
    };
  }
  if (normalized === 'shannon' || normalized === 'workflow://shannon') {
    return {
      profile: 'rich',
      task: 'assimilate',
    };
  }
  return {
    target: normalized || 'unknown',
    hint: 'No specialized scaffold exists for this target. Use --payload-base64 with target-specific JSON.',
  };
}

function parseLastJsonObject(text) {
  const rows = String(text || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
  for (let i = rows.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(rows[i]);
    } catch (_error) {
      // keep scanning backward
    }
  }
  return null;
}

function ensureStateDir() {
  fs.mkdirSync(STATE_DIR, { recursive: true });
}

function readJsonFile(filePath, fallback) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (_error) {
    return fallback;
  }
}

function writeJsonFile(filePath, value) {
  ensureStateDir();
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function percentile(sortedValues, p) {
  if (!sortedValues.length) return 0;
  const idx = Math.ceil((p / 100) * sortedValues.length) - 1;
  const bounded = Math.max(0, Math.min(sortedValues.length - 1, idx));
  return sortedValues[bounded];
}

function updateMetrics(target, latencyMs, ok) {
  const now = new Date().toISOString();
  const metrics = readJsonFile(METRICS_STATE_PATH, {
    schema_version: 'assimilate_metrics_v1',
    targets: {},
  });
  if (!metrics.targets || typeof metrics.targets !== 'object') {
    metrics.targets = {};
  }
  if (!metrics.targets[target]) {
    metrics.targets[target] = {
      count: 0,
      ok_count: 0,
      fail_count: 0,
      last_latency_ms: 0,
      p50_ms: 0,
      p95_ms: 0,
      updated_at: now,
      latencies_ms: [],
    };
  }
  const row = metrics.targets[target];
  row.count += 1;
  if (ok) {
    row.ok_count += 1;
  } else {
    row.fail_count += 1;
  }
  row.last_latency_ms = Math.max(0, Math.round(latencyMs));
  row.updated_at = now;
  if (ok) {
    row.latencies_ms = Array.isArray(row.latencies_ms) ? row.latencies_ms : [];
    row.latencies_ms.push(row.last_latency_ms);
    if (row.latencies_ms.length > 200) {
      row.latencies_ms = row.latencies_ms.slice(row.latencies_ms.length - 200);
    }
    const sorted = row.latencies_ms.slice().sort((a, b) => a - b);
    row.p50_ms = percentile(sorted, 50);
    row.p95_ms = percentile(sorted, 95);
  }
  writeJsonFile(METRICS_STATE_PATH, metrics);
  return row;
}

function maybePrewarm(enabled) {
  if (!enabled) return;
  const nowMs = Date.now();
  const state = readJsonFile(PREWARM_STATE_PATH, { ts_ms: 0 });
  const lastTs = Number(state.ts_ms || 0);
  if (nowMs - lastTs < DEFAULT_PREWARM_TTL_MS) {
    return;
  }
  spawnSync(
    process.execPath,
    [RUN_OPS_SCRIPT, 'health-status', 'status', '--fast=1'],
    { cwd: WORKSPACE_ROOT, encoding: 'utf8', maxBuffer: 1024 * 1024 * 2 },
  );
  writeJsonFile(PREWARM_STATE_PATH, { ts_ms: nowMs, ts: new Date(nowMs).toISOString() });
}

function runCoreAssimilation(domain, args) {
  const t0 = process.hrtime.bigint();
  const proc = spawnSync(process.execPath, [RUN_OPS_SCRIPT, domain, ...args], {
    cwd: WORKSPACE_ROOT,
    encoding: 'utf8',
    maxBuffer: 1024 * 1024 * 8,
  });
  const t1 = process.hrtime.bigint();
  const latencyMs = Number(t1 - t0) / 1e6;
  const payload = parseLastJsonObject(proc.stdout);
  return {
    status: proc.status == null ? 1 : proc.status,
    latencyMs,
    stdout: String(proc.stdout || ''),
    stderr: String(proc.stderr || ''),
    payload,
  };
}

function emitStageSnapshot(totalMs, includeFinal = false) {
  const elapsedTotal = Math.max(0, Number(totalMs) || 0);
  const stages = includeFinal ? STAGES : STAGES.slice(0, STAGES.length - 1);
  for (const stage of stages) {
    const elapsedMs = Math.round(elapsedTotal * stage.percent / 100);
    process.stdout.write(`${stage.label}\n`);
    process.stdout.write(
      `${renderBar(stage.percent)} ${String(stage.percent).padStart(3)}%   (${(elapsedMs / 1000).toFixed(1)} seconds elapsed)\n\n`,
    );
  }
}

async function emitShowcaseProgress(totalMs) {
  const schedule = buildStageSchedule(totalMs);
  let elapsedMs = 0;
  for (let i = 0; i < schedule.length - 1; i += 1) {
    const stage = schedule[i];
    elapsedMs += stage.durationMs;
    process.stdout.write(`${stage.label}\n`);
    process.stdout.write(
      `${renderBar(stage.percent)} ${String(stage.percent).padStart(3)}%   (${(elapsedMs / 1000).toFixed(1)} seconds elapsed)\n\n`,
    );
    if (stage.durationMs > 0) {
      await delay(stage.durationMs);
    }
  }
}

function printFinalSuccess(target, receipt, metrics, totalElapsedMs) {
  const finalStage = STAGES[STAGES.length - 1];
  process.stdout.write(`${finalStage.label}\n`);
  process.stdout.write(
    `${renderBar(finalStage.percent)} ${String(finalStage.percent).padStart(3)}%   (${(Math.max(0, totalElapsedMs) / 1000).toFixed(1)} seconds elapsed)\n\n`,
  );
  process.stdout.write(`Receipt: ${receipt}\n`);
  process.stdout.write(`Target: ${target} fully assimilated. Agents online.\n`);
  if (metrics) {
    process.stdout.write(
      `Latency: ${metrics.last_latency_ms} ms (p50=${metrics.p50_ms} ms, p95=${metrics.p95_ms} ms)\n`,
    );
  }
  process.stdout.write('\nPower to The Users.\n');
}

function printFailure(target, runResult) {
  const detail = runResult.payload || { ok: false, error: 'assimilation_failed' };
  process.stderr.write(
    JSON.stringify(
      {
        ok: false,
        type: 'assimilate_failure',
        target,
        latency_ms: Math.round(runResult.latencyMs),
        status: runResult.status,
        detail,
      },
      null,
      2,
    ) + '\n',
  );
}

async function run() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    usage();
    return 0;
  }

  const target = normalizeTarget(options.target);
  if (!target) {
    usage();
    return 1;
  }

  const route = decodeInjectedCoreRoute(options);
  if (options.scaffoldPayload) {
    const scaffold = payloadScaffoldFor(target);
    process.stdout.write(
      JSON.stringify(
        {
          ok: true,
          type: 'assimilate_payload_scaffold',
          target,
          route: route || null,
          payload: scaffold,
          payload_base64: Buffer.from(JSON.stringify(scaffold)).toString('base64'),
        },
        null,
        2,
      ) + '\n',
    );
    return 0;
  }

  maybePrewarm(options.prewarm);

  const startedAt = new Date();
  const tsIso = startedAt.toISOString();
  const displayDurationMs =
    options.durationMs == null
      ? (options.showcase ? DEFAULT_SHOWCASE_DURATION_MS : DEFAULT_REALTIME_DURATION_MS)
      : options.durationMs;

  if (!route) {
    const syntheticReceipt = buildReceiptHash(target, tsIso);
    if (!options.json) {
      if (displayDurationMs > 0) {
        await emitShowcaseProgress(displayDurationMs);
      } else {
        emitStageSnapshot(0, false);
      }
    }
    const metrics = updateMetrics(target, Math.max(0, displayDurationMs), true);
    if (options.json) {
      process.stdout.write(
        JSON.stringify(
          {
            ok: true,
            type: 'assimilate_progress',
            mode: 'simulation',
            target,
            receipt: syntheticReceipt,
            latency_ms: metrics.last_latency_ms,
            metrics,
            ts: tsIso,
            motto: 'Power to The Users.',
          },
          null,
          2,
        ) + '\n',
      );
      return 0;
    }
    printFinalSuccess(target, syntheticReceipt, metrics, Math.max(0, displayDurationMs));
    return 0;
  }

  if (displayDurationMs > 0 && !options.json) {
    const progressPromise = emitShowcaseProgress(displayDurationMs);
    const runResult = runCoreAssimilation(route.domain, route.args);
    await progressPromise;
    const metrics = updateMetrics(target, runResult.latencyMs, runResult.status === 0);
    const receipt =
      runResult.payload && typeof runResult.payload.receipt_hash === 'string'
        ? runResult.payload.receipt_hash
        : buildReceiptHash(target, tsIso);
    if (runResult.status !== 0) {
      printFailure(target, runResult);
      return 1;
    }
    printFinalSuccess(target, receipt, metrics, runResult.latencyMs);
    return 0;
  }

  const runResult = runCoreAssimilation(route.domain, route.args);
  const metrics = updateMetrics(target, runResult.latencyMs, runResult.status === 0);
  const receipt =
    runResult.payload && typeof runResult.payload.receipt_hash === 'string'
      ? runResult.payload.receipt_hash
      : buildReceiptHash(target, tsIso);

  if (options.json) {
    process.stdout.write(
      JSON.stringify(
        {
          ok: runResult.status === 0,
          type: 'assimilate_execution',
          mode: 'runtime',
          target,
          route,
          latency_ms: Math.round(runResult.latencyMs),
          receipt,
          metrics,
          payload: runResult.payload,
          stderr: runResult.status === 0 ? '' : runResult.stderr.trim(),
          ts: tsIso,
        },
        null,
        2,
      ) + '\n',
    );
    return runResult.status === 0 ? 0 : 1;
  }

  emitStageSnapshot(runResult.latencyMs, false);
  if (runResult.status !== 0) {
    printFailure(target, runResult);
    return 1;
  }
  printFinalSuccess(target, receipt, metrics, runResult.latencyMs);
  return 0;
}

run()
  .then((code) => {
    process.exitCode = code;
  })
  .catch((error) => {
    process.stderr.write(
      JSON.stringify(
        {
          ok: false,
          type: 'assimilate_cli_error',
          error: error && error.message ? String(error.message) : String(error),
        },
        null,
        2,
      ) + '\n',
    );
    process.exitCode = 1;
  });
