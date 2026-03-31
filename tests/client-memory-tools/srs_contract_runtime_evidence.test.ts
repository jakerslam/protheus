#!/usr/bin/env node
/* eslint-disable no-console */
const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '../..');
const SRS_FILE = path.join(ROOT, 'docs/workspace/SRS.md');

function readDoneIdsFromSrs() {
  if (!fs.existsSync(SRS_FILE)) {
    throw new Error(`missing SRS source: ${SRS_FILE}`);
  }
  const done = new Set();
  const lines = fs.readFileSync(SRS_FILE, 'utf8').split(/\r?\n/);
  for (const line of lines) {
    if (!line.startsWith('|')) continue;
    const cells = line
      .split('|')
      .slice(1, -1)
      .map((cell) => cell.trim());
    if (cells.length < 2) continue;
    const id = String(cells[0] || '').trim().toUpperCase();
    const status = String(cells[1] || '').trim().toLowerCase();
    if (!/^V[0-9A-Z]+-/.test(id)) continue;
    if (status !== 'done') continue;
    done.add(id);
  }
  return done;
}

function readRuntimeManifestIds() {
  const out = execFileSync(
    'cargo',
    ['run', '-q', '-p', 'protheus-ops-core', '--bin', 'protheus-ops', '--', 'runtime-systems', 'manifest', '--json=1'],
    {
      cwd: ROOT,
      encoding: 'utf8',
      maxBuffer: 1024 * 1024 * 64,
    },
  );
  const payload = JSON.parse(out);
  if (!Array.isArray(payload.contracts)) {
    throw new Error('runtime manifest missing contracts array');
  }
  return new Set(
    payload.contracts
      .map((row) => String((row && row.id) || '').trim().toUpperCase())
      .filter(Boolean),
  );
}

function deriveRuntimeDoneIds() {
  const doneIds = readDoneIdsFromSrs();
  const runtimeIds = readRuntimeManifestIds();
  return [...runtimeIds].filter((id) => doneIds.has(id)).sort();
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function readLatestSuccessfulFromHistory(historyPath, id) {
  if (!fs.existsSync(historyPath)) return null;
  const lines = fs
    .readFileSync(historyPath, 'utf8')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    let parsed;
    try {
      parsed = JSON.parse(lines[idx]);
    } catch {
      continue;
    }
    const parsedId =
      parsed.id ||
      parsed.system_id ||
      (parsed.contract_profile && parsed.contract_profile.id) ||
      (parsed.contract_execution && parsed.contract_execution.contract_id);
    if (String(parsedId || '').toUpperCase() !== id) continue;
    if (parsed.ok !== true) continue;
    if (!(typeof parsed.receipt_hash === 'string' && parsed.receipt_hash.length > 10)) continue;
    return parsed;
  }
  return null;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function main() {
  const ids = deriveRuntimeDoneIds();
  assert(ids.length > 0, 'runtime done id selection is empty');
  const failures = [];

  for (const id of ids) {
    const legacyLatestPath = path.join(ROOT, 'local/state/ops/srs_contract_runtime', id, 'latest.json');
    const runtimeLatestPath = path.join(ROOT, 'client/local/state/runtime_systems', id, 'latest.json');
    const runtimeExists = fs.existsSync(runtimeLatestPath);
    const legacyExists = fs.existsSync(legacyLatestPath);
    try {
      assert(runtimeExists || legacyExists, `missing runtime receipt for ${id}`);
      let latestPath = runtimeExists ? runtimeLatestPath : legacyLatestPath;
      let latest = readJson(latestPath);
      // Fallback for stale pre-contract runtime artifacts that do not carry strict contract receipts.
      if (
        latestPath === runtimeLatestPath &&
        (!latest || latest.ok !== true) &&
        legacyExists
      ) {
        latestPath = legacyLatestPath;
        latest = readJson(latestPath);
      }
      if (!latest || latest.ok !== true) {
        const fallback =
          readLatestSuccessfulFromHistory(
            path.join(ROOT, 'client/local/state/runtime_systems', id, 'history.jsonl'),
            id,
          ) ||
          readLatestSuccessfulFromHistory(
            path.join(ROOT, 'local/state/ops/srs_contract_runtime', id, 'history.jsonl'),
            id,
          );
        if (fallback) {
          latest = fallback;
        }
      }
      const latestId =
        latest.id ||
        latest.system_id ||
        (latest.contract_profile && latest.contract_profile.id) ||
        (latest.contract_execution && latest.contract_execution.contract_id);
      assert(String(latestId || '').toUpperCase() === id, `runtime receipt id mismatch for ${id}`);
      assert(latest.ok === true, `runtime receipt not ok for ${id}`);
      assert(
        typeof latest.receipt_hash === 'string' && latest.receipt_hash.length > 10,
        `missing deterministic receipt_hash for ${id}`,
      );
    } catch (error) {
      failures.push({ id, error: String(error && error.message ? error.message : error) });
    }
  }

  if (failures.length > 0) {
    console.error(
      JSON.stringify(
        {
          ok: false,
          type: 'srs_contract_runtime_evidence_test',
          ids_scanned: ids.length,
          failures,
        },
        null,
        2,
      ),
    );
    process.exit(1);
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'srs_contract_runtime_evidence_test',
        ids_scanned: ids.length,
      },
      null,
      2,
    ),
  );
}

main();
