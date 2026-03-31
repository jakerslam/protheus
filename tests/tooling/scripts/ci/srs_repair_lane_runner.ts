#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

function parseArgs(argv) {
  const out = new Map();
  for (const token of argv.slice(2)) {
    if (!token.startsWith('--')) continue;
    const idx = token.indexOf('=');
    if (idx === -1) {
      out.set(token.slice(2), '1');
    } else {
      out.set(token.slice(2, idx), token.slice(idx + 1));
    }
  }
  return out;
}

function normalizeId(raw) {
  return String(raw || '').trim().toUpperCase();
}

function isValidId(id) {
  return /^V[0-9A-Z._-]+$/.test(id);
}

function fail(message, extra = {}) {
  const payload = {
    ok: false,
    type: 'srs_repair_lane_runner',
    error: message,
    ...extra,
  };
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}

function contractPath(id) {
  return resolve(`planes/contracts/srs/${id}.json`);
}

function loadContract(id) {
  const path = contractPath(id);
  if (!existsSync(path)) return null;
  try {
    return {
      path,
      json: JSON.parse(readFileSync(path, 'utf8')),
    };
  } catch (error) {
    fail('contract_parse_failed', {
      id,
      contractPath: path,
      detail: String(error && error.message ? error.message : error),
    });
  }
  return null;
}

function detectRoute(contractPayload) {
  const runtime = String(contractPayload?.execution_contract?.runtime || '').toLowerCase();
  const laneCommand = String(contractPayload?.validation?.lane_command || '').toLowerCase();
  if (runtime.includes('srs_contract_runtime') || laneCommand.includes('srs-contract-runtime')) {
    return 'srs_contract_runtime';
  }
  if (runtime.includes('runtime_systems') || laneCommand.includes('runtime-systems')) {
    return 'runtime_systems';
  }
  return 'runtime_systems';
}

function receiptPathFor({ id, route, contractPayload }) {
  const mutableStatePath = String(contractPayload?.execution_contract?.mutable_state_path || '').trim();
  if (mutableStatePath) return resolve(mutableStatePath);
  if (route === 'srs_contract_runtime') {
    return resolve(`local/state/ops/srs_contract_runtime/${id}/latest.json`);
  }
  return resolve(`client/local/state/runtime_systems/${id}/latest.json`);
}

function cargoArgsFor({ id, strict, route }) {
  if (route === 'srs_contract_runtime') {
    return [
      'run',
      '-q',
      '-p',
      'protheus-ops-core',
      '--bin',
      'protheus-ops',
      '--',
      'srs-contract-runtime',
      'run',
      `--id=${id}`,
    ];
  }
  return [
    'run',
    '-q',
    '-p',
    'protheus-ops-core',
    '--bin',
    'protheus-ops',
    '--',
    'runtime-systems',
    'run',
    `--system-id=${id}`,
    '--apply=1',
    `--strict=${strict}`,
  ];
}

function main() {
  const args = parseArgs(process.argv);
  const id = normalizeId(args.get('id'));
  if (!id || !isValidId(id)) {
    fail('invalid_or_missing_id', { received: String(args.get('id') || '') });
  }

  const strict = String(args.get('strict') || '1') === '0' ? '0' : '1';
  const dryRun = String(args.get('dry-run') || '0') === '1';
  const contract = loadContract(id);
  const route = detectRoute(contract?.json || null);

  const receiptPath = receiptPathFor({ id, route, contractPayload: contract?.json || null });
  if (dryRun) {
    console.log(
      JSON.stringify(
        {
          ok: true,
          type: 'srs_repair_lane_runner',
          mode: 'dry_run',
          id,
          route,
          strict: strict === '1',
          contractPath: contract?.path || null,
          receiptPath,
        },
        null,
        2,
      ),
    );
    return;
  }

  const cmd = cargoArgsFor({ id, strict, route });
  const child = spawnSync('cargo', cmd, {
    cwd: resolve('.'),
    stdio: 'inherit',
    env: process.env,
  });
  if (child.status !== 0) {
    fail('cargo_run_failed', { id, exitCode: child.status ?? 1 });
  }

  if (!existsSync(receiptPath)) {
    fail('missing_receipt_after_run', { id, receiptPath });
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'srs_repair_lane_runner',
        mode: 'run',
        id,
        route,
        strict: strict === '1',
        contractPath: contract?.path || null,
        receiptPath,
      },
      null,
      2,
    ),
  );
}

main();
