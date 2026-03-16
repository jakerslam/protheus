#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '../..');
const IDS_FILE = path.join(ROOT, 'tests/fixtures/srs_contract_runtime_done_ids.txt');

function readIds() {
  if (!fs.existsSync(IDS_FILE)) {
    throw new Error(`missing ids fixture: ${IDS_FILE}`);
  }
  return fs
    .readFileSync(IDS_FILE, 'utf8')
    .split(/\r?\n/)
    .map((line) => line.trim().toUpperCase())
    .filter(Boolean);
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function main() {
  const ids = readIds();
  assert(ids.length > 0, 'ids fixture is empty');
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
