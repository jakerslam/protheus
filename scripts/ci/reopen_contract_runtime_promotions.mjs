#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RECEIPT_PATH = process.argv[2] || 'client/local/state/ops/backlog_queue_executor/latest.json';
const TARGETS = ['docs/workspace/SRS.md', 'docs/workspace/UPGRADE_BACKLOG.md'];

function readJson(path) {
  return JSON.parse(readFileSync(resolve(path), 'utf8'));
}

function collectIds(receipt) {
  const rows = Array.isArray(receipt.rows) ? receipt.rows : [];
  const ids = new Set();
  for (const row of rows) {
    if (!row || row.status !== 'executed') continue;
    const id = String(row.id || '').trim();
    if (!id) continue;
    const laneRoute = String(row.lane_route || row.laneRoute || '').trim();
    const laneScript = String(row.lane_script || row.laneScript || '').trim();
    if (laneRoute === 'core_srs_contract_runtime' || laneScript.startsWith('core:srs_contract_runtime:')) {
      ids.add(id);
    }
  }
  return ids;
}

function reopenRows(markdown, ids) {
  let changed = 0;
  const lines = String(markdown || '').split('\n');
  const out = lines.map((line) => {
    if (!line.startsWith('|')) return line;
    const cells = line.split('|').slice(1, -1).map((c) => c.trim());
    if (cells.length < 2) return line;
    const id = cells[0];
    const status = String(cells[1] || '').toLowerCase();
    if (!ids.has(id)) return line;
    if (status !== 'done') return line;
    cells[1] = 'in_progress';
    changed += 1;
    return `| ${cells.join(' | ')} |`;
  });
  return { changed, markdown: `${out.join('\n')}\n` };
}

function main() {
  const receipt = readJson(RECEIPT_PATH);
  const ids = collectIds(receipt);
  const changes = [];
  for (const target of TARGETS) {
    const abs = resolve(target);
    const before = readFileSync(abs, 'utf8');
    const result = reopenRows(before, ids);
    if (result.changed > 0) {
      writeFileSync(abs, result.markdown, 'utf8');
    }
    changes.push({ file: target, changed: result.changed });
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'reopen_contract_runtime_promotions',
        receipt: RECEIPT_PATH,
        ids_found: ids.size,
        changes,
      },
      null,
      2,
    ),
  );
}

main();
