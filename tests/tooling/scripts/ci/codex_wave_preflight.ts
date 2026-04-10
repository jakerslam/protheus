#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const OUT_JSON = 'core/local/artifacts/codex_wave_preflight_current.json';
const OUT_MD = 'local/workspace/reports/CODEX_WAVE_PREFLIGHT_CURRENT.md';
const DEFAULT_LEDGER = 'local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.json';

function parseArgs(argv: string[]) {
  const strict = argv.includes('--strict=1') || argv.includes('--strict');
  const runChurnGuard = !(
    argv.includes('--skip-churn=1') || argv.includes('--skip-churn')
  );
  const ledgerArg = argv.find((row) => row.startsWith('--ledger='));
  const ordersArg = argv.find((row) => row.startsWith('--orders='));
  const minArg = argv.find((row) => row.startsWith('--min-wave-size='));
  const maxArg = argv.find((row) => row.startsWith('--max-wave-size='));

  const ledgerPath = resolve(
    process.cwd(),
    ledgerArg ? ledgerArg.slice('--ledger='.length).trim() : DEFAULT_LEDGER,
  );
  const ordersSpec = ordersArg ? ordersArg.slice('--orders='.length).trim() : '';
  const minWaveSize = Number(minArg ? minArg.slice('--min-wave-size='.length) : '4');
  const maxWaveSize = Number(maxArg ? maxArg.slice('--max-wave-size='.length) : '8');
  return {
    strict,
    runChurnGuard,
    ledgerPath,
    ordersSpec,
    minWaveSize: Number.isFinite(minWaveSize) ? minWaveSize : 4,
    maxWaveSize: Number.isFinite(maxWaveSize) ? maxWaveSize : 8,
  };
}

function parseOrders(spec: string): number[] {
  const rows = new Set<number>();
  for (const rawToken of spec.split(',').map((token) => token.trim()).filter(Boolean)) {
    if (rawToken.includes('-')) {
      const [startRaw, endRaw] = rawToken.split('-', 2);
      const start = Number(startRaw);
      const end = Number(endRaw);
      if (!Number.isInteger(start) || !Number.isInteger(end) || start <= 0 || end <= 0) {
        continue;
      }
      const lo = Math.min(start, end);
      const hi = Math.max(start, end);
      for (let value = lo; value <= hi; value += 1) {
        rows.add(value);
      }
      continue;
    }
    const value = Number(rawToken);
    if (Number.isInteger(value) && value > 0) {
      rows.add(value);
    }
  }
  return [...rows].sort((a, b) => a - b);
}

function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(path, 'utf8')) as T;
}

function parseGitStatusRows() {
  const raw = execSync('git status --porcelain=v1 -uall', { encoding: 'utf8' });
  return raw
    .split('\n')
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .map((line) => ({
      status: line.slice(0, 2),
      path: line.slice(3).trim(),
    }));
}

function runChurnGuard() {
  try {
    const output = execSync('npm -s run ops:churn:guard', { encoding: 'utf8' });
    const parsed = JSON.parse(output);
    return {
      ok: Boolean(parsed?.ok),
      summary: parsed?.summary ?? null,
      raw: output.trim(),
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return {
      ok: false,
      summary: null,
      raw: message,
    };
  }
}

function writeArtifact(path: string, data: string) {
  const abs = resolve(process.cwd(), path);
  const folder = dirname(abs);
  if (!existsSync(folder)) mkdirSync(folder, { recursive: true });
  writeFileSync(abs, data);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const dirtyRows = parseGitStatusRows();
  const selectedOrders = parseOrders(args.ordersSpec);
  const churn = args.runChurnGuard ? runChurnGuard() : { ok: true, summary: null, raw: 'skipped' };

  const failures: string[] = [];
  if (!existsSync(args.ledgerPath)) {
    failures.push(`ledger_missing:${args.ledgerPath}`);
  }
  if (dirtyRows.length > 0) {
    failures.push('workspace_not_clean');
  }
  if (!churn.ok) {
    failures.push('churn_guard_failed');
  }
  if (selectedOrders.length === 0) {
    failures.push('orders_missing_or_invalid');
  }
  if (selectedOrders.length > 0 && selectedOrders.length < args.minWaveSize) {
    failures.push(`wave_size_below_min:${selectedOrders.length}<${args.minWaveSize}`);
  }
  if (selectedOrders.length > args.maxWaveSize) {
    failures.push(`wave_size_above_max:${selectedOrders.length}>${args.maxWaveSize}`);
  }

  let selectedRows: any[] = [];
  let missingOrders: number[] = [];
  let nonQueuedOrders: Array<{ order: number; status: string }> = [];
  let duplicateFiles: string[] = [];
  if (existsSync(args.ledgerPath)) {
    const ledger = readJson<{ rows?: any[]; [k: string]: any }>(args.ledgerPath);
    const rows = Array.isArray(ledger.rows) ? ledger.rows : [];
    const byOrder = new Map<number, any>();
    for (const row of rows) {
      if (Number.isInteger(row?.order)) {
        byOrder.set(row.order, row);
      }
    }
    for (const order of selectedOrders) {
      const row = byOrder.get(order);
      if (!row) {
        missingOrders.push(order);
        continue;
      }
      selectedRows.push({
        order,
        status: String(row.status ?? '').toLowerCase(),
        file: String(row.file ?? ''),
        language: String(row.language ?? ''),
        loc: Number(row.loc ?? 0),
      });
    }
    nonQueuedOrders = selectedRows
      .filter((row) => row.status !== 'queued')
      .map((row) => ({ order: row.order, status: row.status }));
    const fileCount = new Map<string, number>();
    for (const row of selectedRows) {
      fileCount.set(row.file, (fileCount.get(row.file) ?? 0) + 1);
    }
    duplicateFiles = [...fileCount.entries()]
      .filter(([, count]) => count > 1)
      .map(([file]) => file);
  }

  if (missingOrders.length > 0) {
    failures.push(`orders_missing_in_ledger:${missingOrders.join(',')}`);
  }
  if (nonQueuedOrders.length > 0) {
    failures.push(
      `orders_not_queued:${nonQueuedOrders
        .map((row) => `${row.order}:${row.status}`)
        .join(',')}`,
    );
  }
  if (duplicateFiles.length > 0) {
    failures.push(`duplicate_file_shards:${duplicateFiles.join(',')}`);
  }

  const pass = failures.length === 0;
  const out = {
    ok: pass,
    type: 'codex_wave_preflight',
    strict: args.strict,
    ledger_path: args.ledgerPath,
    selected_orders: selectedOrders,
    selected_rows: selectedRows,
    summary: {
      selected_count: selectedOrders.length,
      selected_rows_found: selectedRows.length,
      workspace_clean: dirtyRows.length === 0,
      churn_guard_ok: churn.ok,
      disjoint_files: duplicateFiles.length === 0,
      all_queued: nonQueuedOrders.length === 0,
      pass,
    },
    failures,
    dirty_rows: dirtyRows,
    churn_guard: {
      ok: churn.ok,
      summary: churn.summary,
    },
    out_json: OUT_JSON,
    out_markdown: OUT_MD,
  };

  writeArtifact(OUT_JSON, `${JSON.stringify(out, null, 2)}\n`);
  const md = [
    '# Codex Wave Preflight',
    '',
    `- pass: \`${pass}\``,
    `- strict: \`${args.strict}\``,
    `- selected orders: \`${selectedOrders.join(',') || '(none)'}\``,
    `- selected rows found: \`${selectedRows.length}\``,
    `- workspace clean: \`${dirtyRows.length === 0}\``,
    `- churn guard ok: \`${churn.ok}\``,
    `- disjoint files: \`${duplicateFiles.length === 0}\``,
    `- all queued: \`${nonQueuedOrders.length === 0}\``,
    '',
    '## Failures',
    ...(failures.length > 0 ? failures.map((row) => `- ${row}`) : ['- none']),
  ];
  writeArtifact(OUT_MD, `${md.join('\n')}\n`);
  console.log(JSON.stringify(out, null, 2));
  if (args.strict && !pass) {
    process.exit(1);
  }
}

main();
