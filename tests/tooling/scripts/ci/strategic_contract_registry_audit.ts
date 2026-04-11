#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { invokeTsModuleSync } from '../../../../client/runtime/lib/in_process_ts_delegate.ts';

const ROOT = resolve('.');
const ACTIONABLE_MAP_PATH = resolve('core/local/artifacts/srs_actionable_map_current.json');
const CONTRACT_SOURCE_PATH = resolve('core/layer0/ops/src/runtime_system_contracts.rs');
const SRS_PATH = resolve('docs/workspace/SRS.md');
const OUT_PATH = resolve('core/local/artifacts/strategic_contract_registry_audit_current.json');

function fail(msg, payload = {}) {
  const out = {
    ok: false,
    type: 'strategic_contract_registry_audit',
    error: msg,
    ...payload
  };
  mkdirSync(resolve('core/local/artifacts'), { recursive: true });
  writeFileSync(OUT_PATH, `${JSON.stringify(out, null, 2)}\n`, 'utf8');
  console.error(JSON.stringify(out, null, 2));
  process.exit(1);
}

function ensureActionableMap() {
  if (existsSync(ACTIONABLE_MAP_PATH)) return;
  const run = invokeTsModuleSync(resolve('tests/tooling/scripts/ci/srs_actionable_map.ts'), {
    cwd: ROOT,
    exportName: 'run',
  });
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  if (status !== 0) {
    fail('srs_actionable_map_generation_failed', {
      status,
      stdout: String(run.stdout || '').slice(-4000),
      stderr: String(run.stderr || '').slice(-4000)
    });
  }
}

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

function main() {
  const args = parseArgs(process.argv);
  const strict = String(args.get('strict') || '0') === '1';

  ensureActionableMap();
  if (!existsSync(CONTRACT_SOURCE_PATH)) {
    fail('runtime_system_contracts_source_missing');
  }
  if (!existsSync(SRS_PATH)) {
    fail('srs_missing');
  }

  const actionable = JSON.parse(readFileSync(ACTIONABLE_MAP_PATH, 'utf8'));
  const rows = Array.isArray(actionable.rows) ? actionable.rows : [];
  const mustCoverIds = rows
    .filter((row) => ['queued', 'in_progress'].includes(String(row.status || '').trim()))
    .map((row) => String(row.id || '').trim())
    .filter(Boolean)
    .sort();

  const source = readFileSync(CONTRACT_SOURCE_PATH, 'utf8');
  const idsInRegistryArr = [...source.matchAll(/"((?:V[0-9A-Z]+-[0-9A-Z-]+(?:\.[0-9]+)?))"/g)]
    .map((m) => m[1])
    .filter((id) => /^V[0-9A-Z]+-[0-9A-Z-]+(?:\.[0-9]+)?$/.test(id))
    .filter((id) => !id.endsWith('-'))
    .filter((id) => /-\d/.test(id));
  const idsInRegistry = new Set(idsInRegistryArr);

  const missing = mustCoverIds.filter((id) => !idsInRegistry.has(id));

  const srsRows = readFileSync(SRS_PATH, 'utf8')
    .split('\n')
    .filter((line) => line.startsWith('|'))
    .map((line) => line.split('|').slice(1, -1).map((c) => c.trim()))
    .filter((cells) => cells.length >= 2 && /^V[0-9A-Z]+-/.test(cells[0]))
    .map((cells) => ({ id: cells[0], status: String(cells[1] || '').toLowerCase() }));
  const srsStatusById = new Map(srsRows.map((row) => [row.id, row.status]));
  const registryMissingFromSrs = idsInRegistryArr.filter((id) => !srsStatusById.has(id));
  const registryNotDone = idsInRegistryArr.filter((id) => srsStatusById.get(id) !== 'done');

  const out = {
    ok: missing.length === 0 && registryMissingFromSrs.length === 0,
    type: 'strategic_contract_registry_audit',
    strict,
    source_actionable_map: 'core/local/artifacts/srs_actionable_map_current.json',
    source_runtime_registry: 'core/layer0/ops/src/runtime_system_contracts.rs',
    source_srs: 'docs/workspace/SRS.md',
    scanned_actionable_ids: mustCoverIds.length,
    registry_ids: idsInRegistry.size,
    missing_ids: missing
    ,
    registry_missing_from_srs: registryMissingFromSrs,
    registry_not_done: registryNotDone
  };

  mkdirSync(resolve('core/local/artifacts'), { recursive: true });
  writeFileSync(OUT_PATH, `${JSON.stringify(out, null, 2)}\n`, 'utf8');

  if (strict && (missing.length > 0 || registryMissingFromSrs.length > 0)) {
    fail('strategic_contract_registry_missing_actionable_ids', out);
  }

  console.log(JSON.stringify(out, null, 2));
}

main();
