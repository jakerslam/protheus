#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const DEFAULT_ACTIONABLE_MAP = 'core/local/artifacts/srs_actionable_map_current.json';
const LANE_REGISTRY_PATH = 'client/runtime/config/lane_command_registry.json';

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), 'utf8'));
}

function laneNameForId(id) {
  return `lane:${String(id).toLowerCase().replace(/_/g, '-')}:run`;
}

function laneCommand(id) {
  return `node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/srs_repair_lane_runner.ts --id=${id} --strict=1`;
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
  const actionablePath = String(args.get('actionable') || DEFAULT_ACTIONABLE_MAP);
  const dryRun = String(args.get('dry-run') || '0') === '1';

  const actionable = readJson(actionablePath);
  const registry = readJson(LANE_REGISTRY_PATH);
  const runRegistry = { ...(registry.run || {}) };

  const rows = Array.isArray(actionable.rows) ? actionable.rows : [];
  const repairRows = rows.filter(
    (row) => row && row.todoBucket === 'repair_lane' && String(row.status || '') !== 'blocked',
  );

  const added = [];
  const alreadyCorrect = [];
  const replaced = [];

  for (const row of repairRows) {
    const id = String(row.id || '').trim().toUpperCase();
    if (!id) continue;
    const key = laneNameForId(id);
    const next = laneCommand(id);
    const existing = runRegistry[id] && runRegistry[id].command;
    if (!existing) {
      runRegistry[id] = { id, command: next, source_script: key };
      added.push(id);
      continue;
    }
    if (existing === next) {
      alreadyCorrect.push(id);
      continue;
    }
    runRegistry[id] = { id, command: next, source_script: key };
    replaced.push(id);
  }

  if (!dryRun && (added.length > 0 || replaced.length > 0)) {
    registry.run = runRegistry;
    writeFileSync(resolve(LANE_REGISTRY_PATH), `${JSON.stringify(registry, null, 2)}\n`);
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'sync_repair_lane_scripts',
        actionablePath,
        laneRegistryPath: LANE_REGISTRY_PATH,
        repair_rows: repairRows.length,
        added: added.length,
        replaced: replaced.length,
        already_correct: alreadyCorrect.length,
        dry_run: dryRun,
      },
      null,
      2,
    ),
  );
}

main();
