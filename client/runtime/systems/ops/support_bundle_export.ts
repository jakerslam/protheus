#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

type CommandResult = {
  id: string;
  ok: boolean;
  status: number;
  command: string;
  args: string[];
  stdout: string;
  stderr: string;
  payload: unknown;
};

type ParsedArgs = {
  command: string;
  out: string;
  strict: boolean;
};

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const TS_ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/support_bundle_latest.json');

function clean(value: unknown, max = 500): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(value: string | undefined, fallback = false): boolean {
  const raw = clean(value, 32).toLowerCase();
  if (!raw) return fallback;
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function parseArgs(argv: string[]): ParsedArgs {
  const parsed: ParsedArgs = {
    command: 'run',
    out: DEFAULT_OUT,
    strict: false,
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 500);
    if (!token) continue;
    if (token === 'run' || token === 'status' || token === 'help') {
      parsed.command = token;
      continue;
    }
    if (token.startsWith('--out=')) {
      parsed.out = path.resolve(ROOT, clean(token.slice('--out='.length), 500));
      continue;
    }
    if (token.startsWith('--strict=')) {
      parsed.strict = parseBool(token.slice('--strict='.length), false);
      continue;
    }
  }
  return parsed;
}

function parseJsonLine(stdout: string): unknown {
  const lines = String(stdout || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function runTsCommand(id: string, scriptRelPath: string, args: string[] = []): CommandResult {
  const scriptAbs = path.join(ROOT, scriptRelPath);
  const out = spawnSync(process.execPath, [TS_ENTRYPOINT, scriptAbs].concat(args), {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env },
    maxBuffer: 16 * 1024 * 1024,
  });
  const status = Number.isFinite(out.status) ? out.status : 1;
  const stdout = String(out.stdout || '');
  const stderr = String(out.stderr || '');
  return {
    id,
    ok: status === 0,
    status,
    command: process.execPath,
    args: [TS_ENTRYPOINT, scriptAbs].concat(args),
    stdout,
    stderr,
    payload: parseJsonLine(stdout),
  };
}

function checkFile(pathRel: string) {
  const abs = path.join(ROOT, pathRel);
  return {
    path: pathRel,
    exists: fs.existsSync(abs),
  };
}

function buildBundle(outPath: string) {
  const checks: CommandResult[] = [
    runTsCommand('transport_topology', 'client/runtime/systems/ops/transport_topology_status.ts', [
      '--json=1',
    ]),
    runTsCommand('release_policy_gate', 'tests/tooling/scripts/ci/release_policy_gate.ts', [
      '--strict=0',
      '--out=core/local/artifacts/release_policy_gate_current.json',
    ]),
    runTsCommand('runtime_diagnostics', 'client/runtime/systems/ops/protheus_debug_diagnostics.ts', [
      '--help',
    ]),
  ];

  const files = [
    checkFile('core/local/artifacts/release_policy_gate_current.json'),
    checkFile('core/local/artifacts/transport_convergence_guard_current.json'),
    checkFile('core/local/artifacts/arch_boundary_conformance_current.json'),
  ];

  const report = {
    ok: checks.every((row) => row.ok),
    type: 'support_bundle',
    generated_at: new Date().toISOString(),
    host: {
      platform: process.platform,
      arch: process.arch,
      node: process.version,
      cwd: ROOT,
      hostname: os.hostname(),
    },
    checks,
    files,
  };

  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  return report;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const parsed = parseArgs(argv);
  if (parsed.command === 'help') {
    console.log('Usage: ops:support-bundle:export [run|status] [--out=<path>] [--strict=1|0]');
    return 0;
  }
  const outPath = parsed.out || DEFAULT_OUT;
  if (parsed.command === 'status') {
    if (!fs.existsSync(outPath)) {
      console.log(
        JSON.stringify({
          ok: false,
          type: 'support_bundle_status',
          error: 'support_bundle_missing',
          out: outPath,
        }),
      );
      return parsed.strict ? 1 : 0;
    }
    const payload = JSON.parse(fs.readFileSync(outPath, 'utf8'));
    console.log(JSON.stringify({ ok: true, type: 'support_bundle_status', out: outPath, payload }));
    return parsed.strict && payload.ok !== true ? 1 : 0;
  }
  const bundle = buildBundle(outPath);
  console.log(JSON.stringify({ ok: bundle.ok, type: 'support_bundle_run', out: outPath, bundle }));
  if (parsed.strict && bundle.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
