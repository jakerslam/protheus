#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const RECEIPT_PATH = process.argv[2] || 'client/local/state/ops/backlog_queue_executor/latest.json';
const TARGETS = ['docs/workspace/SRS.md', 'docs/workspace/UPGRADE_BACKLOG.md'];

function loadExecutedIds(path) {
  const raw = JSON.parse(readFileSync(resolve(path), 'utf8'));
  const rows = Array.isArray(raw.rows) ? raw.rows : [];
  return new Set(rows.filter((r) => r && r.status === 'executed' && typeof r.id === 'string').map((r) => r.id));
}

function promoteTableRows(markdown, ids) {
  const lines = markdown.split('\n');
  let changed = 0;
  const out = lines.map((line) => {
    if (!line.startsWith('|')) return line;
    const cells = line
      .split('|')
      .slice(1, -1)
      .map((c) => c.trim());
    if (cells.length < 2) return line;
    const id = cells[0];
    const status = (cells[1] || '').toLowerCase();
    if (!ids.has(id)) return line;
    if (!['queued', 'in_progress'].includes(status)) return line;
    cells[1] = 'done';
    changed += 1;
    return `| ${cells.join(' | ')} |`;
  });
  return { markdown: out.join('\n'), changed };
}

function main() {
  const ids = loadExecutedIds(RECEIPT_PATH);
  if (ids.size === 0) {
    console.log(
      JSON.stringify(
        {
          ok: false,
          type: 'promote_executed_receipt_ids',
          reason: 'no_executed_ids_in_receipt',
          receipt: RECEIPT_PATH,
        },
        null,
        2,
      ),
    );
    process.exitCode = 1;
    return;
  }

  const changes = [];
  for (const target of TARGETS) {
    const abs = resolve(target);
    const before = readFileSync(abs, 'utf8');
    const result = promoteTableRows(before, ids);
    if (result.changed > 0) {
      writeFileSync(abs, result.markdown, 'utf8');
    }
    changes.push({ file: target, changed: result.changed });
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'promote_executed_receipt_ids',
        receipt: RECEIPT_PATH,
        executed_ids: ids.size,
        changes,
      },
      null,
      2,
    ),
  );
}

main();
