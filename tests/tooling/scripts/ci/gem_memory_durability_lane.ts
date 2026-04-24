#!/usr/bin/env tsx

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type GemPolicy = {
  memory_contract?: {
    store_path?: string;
    max_read_rows?: number;
    required_route_file?: string;
  };
};

type MemoryEntry = {
  id: string;
  key: string;
  value: string;
  created_at: string;
  previous_hash: string;
  hash: string;
};

type MemoryStore = {
  schema_id: 'gem_memory_store_v1';
  schema_version: 1;
  updated_at: string;
  entries: MemoryEntry[];
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/gem_feedback_closure_policy.json');
const DEFAULT_OUT_PATH = 'core/local/artifacts/gem_memory_durability_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/gem_memory_durability_latest.json';
const DEFAULT_STATE_LATEST_PATH = 'local/state/ops/gem_memory_durability/latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/GEM_MEMORY_DURABILITY_CURRENT.md';

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    stateLatestPath: cleanText(readFlag(argv, 'state-latest') || DEFAULT_STATE_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function entryHash(entry: Omit<MemoryEntry, 'hash'>): string {
  return crypto
    .createHash('sha256')
    .update(
      [
        entry.id,
        entry.key,
        entry.value,
        entry.created_at,
        entry.previous_hash,
      ].join('|'),
      'utf8',
    )
    .digest('hex');
}

function createEntry(id: string, key: string, value: string, previousHash: string): MemoryEntry {
  const createdAt = new Date().toISOString();
  const base = {
    id,
    key,
    value,
    created_at: createdAt,
    previous_hash: previousHash,
  };
  return { ...base, hash: entryHash(base) };
}

function verifyLineage(entries: MemoryEntry[]): {
  ok: boolean;
  violation_count: number;
  first_violation_id: string;
} {
  let previous = '';
  let violations = 0;
  let first = '';
  for (const row of entries) {
    const expected = entryHash({
      id: row.id,
      key: row.key,
      value: row.value,
      created_at: row.created_at,
      previous_hash: row.previous_hash,
    });
    if (row.previous_hash !== previous || row.hash !== expected) {
      violations += 1;
      if (!first) first = row.id;
    }
    previous = row.hash;
  }
  return {
    ok: violations === 0,
    violation_count: violations,
    first_violation_id: first,
  };
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# GEM Memory Durability (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- store_path: ${cleanText(report.memory_store?.path || '', 220)}`);
  lines.push(`- entries_total: ${Number(report.memory_store?.entries_total || 0)}`);
  lines.push(`- recall_rows: ${Number(report.memory_store?.recall_rows || 0)}`);
  lines.push('');
  lines.push('## Checks');
  for (const row of Array.isArray(report.checks) ? report.checks : []) {
    lines.push(
      `- ${cleanText(row.id || 'unknown', 120)}: ${row.ok === true ? 'pass' : 'fail'} (${cleanText(row.detail || '', 240)})`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const policy = readJson<GemPolicy>(POLICY_PATH, {});
  const memoryContract = policy.memory_contract || {};
  const storeRel = cleanText(memoryContract.store_path || '', 500);
  const storePath = path.resolve(ROOT, storeRel || 'local/state/ops/gem_memory_backend/store.json');
  const maxReadRows = Math.max(1, Number(memoryContract.max_read_rows || 256));
  const requiredRouteFile = cleanText(memoryContract.required_route_file || '', 500);
  const routePath = requiredRouteFile ? path.resolve(ROOT, requiredRouteFile) : '';
  const routePresent = routePath ? fs.existsSync(routePath) : false;

  const loaded = readJson<MemoryStore | null>(storePath, null);
  const baselineEntries = Array.isArray(loaded?.entries) ? loaded!.entries : [];
  const previousHash =
    baselineEntries.length > 0 ? cleanText(baselineEntries[baselineEntries.length - 1].hash, 128) : '';

  const seed = Date.now().toString(36);
  const newEntries: MemoryEntry[] = [];
  for (let index = 0; index < 3; index += 1) {
    const prior = index === 0 ? previousHash : newEntries[index - 1].hash;
    newEntries.push(
      createEntry(
        `gem-memory-${seed}-${index}`,
        `memory_store.gem.synthetic.${index}`,
        `durability_probe_value_${seed}_${index}`,
        prior,
      ),
    );
  }

  const updated: MemoryStore = {
    schema_id: 'gem_memory_store_v1',
    schema_version: 1,
    updated_at: new Date().toISOString(),
    entries: [...baselineEntries, ...newEntries],
  };
  fs.mkdirSync(path.dirname(storePath), { recursive: true });
  fs.writeFileSync(storePath, `${JSON.stringify(updated, null, 2)}\n`, 'utf8');

  const reloaded = readJson<MemoryStore>(storePath, {
    schema_id: 'gem_memory_store_v1',
    schema_version: 1,
    updated_at: '',
    entries: [],
  });
  const replayEntries = Array.isArray(reloaded.entries) ? reloaded.entries : [];
  const recallRows = replayEntries.slice(-maxReadRows);
  const lineage = verifyLineage(replayEntries);
  const replayPresenceOk = newEntries.every((entry) =>
    replayEntries.some((existing) => existing.id === entry.id && existing.hash === entry.hash),
  );
  const boundedReadbackOk = recallRows.length <= maxReadRows;

  const checks = [
    {
      id: 'gem_memory_contract_path_present',
      ok: storeRel.length > 0,
      detail: `store_path=${storeRel || 'missing'}`,
    },
    {
      id: 'gem_memory_contract_max_read_rows_positive',
      ok: maxReadRows > 0,
      detail: `max_read_rows=${maxReadRows}`,
    },
    {
      id: 'gem_memory_store_route_file_present',
      ok: requiredRouteFile.length === 0 || routePresent,
      detail:
        requiredRouteFile.length === 0
          ? 'memory route file not configured in policy'
          : `required_route_file=${requiredRouteFile};present=${routePresent}`,
    },
    {
      id: 'gem_memory_durable_write_readback',
      ok: replayPresenceOk,
      detail: `new_entries=${newEntries.length};persisted=${replayPresenceOk}`,
    },
    {
      id: 'gem_memory_bounded_readback_contract',
      ok: boundedReadbackOk,
      detail: `recall_rows=${recallRows.length};max_read_rows=${maxReadRows}`,
    },
    {
      id: 'gem_memory_receipt_lineage_contract',
      ok: lineage.ok,
      detail: `violations=${lineage.violation_count};first_violation_id=${lineage.first_violation_id || 'none'}`,
    },
  ];

  const failed = checks.filter((row) => !row.ok);
  const report = {
    type: 'gem_memory_durability_lane',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    ok: failed.length === 0,
    policy_path: path.relative(ROOT, POLICY_PATH),
    checks,
    failed_ids: failed.map((row) => row.id),
    memory_store: {
      path: path.relative(ROOT, storePath),
      entries_total: replayEntries.length,
      recall_rows: recallRows.length,
      max_read_rows: maxReadRows,
      appended_entry_ids: newEntries.map((entry) => entry.id),
      appended_receipt_hashes: newEntries.map((entry) => entry.hash),
      lineage: {
        ok: lineage.ok,
        violation_count: lineage.violation_count,
        first_violation_id: lineage.first_violation_id,
      },
    },
  };

  const outAbs = path.resolve(ROOT, args.outPath || DEFAULT_OUT_PATH);
  const outLatestAbs = path.resolve(ROOT, args.outLatestPath || DEFAULT_OUT_LATEST_PATH);
  const stateLatestAbs = path.resolve(ROOT, args.stateLatestPath || DEFAULT_STATE_LATEST_PATH);
  const markdownAbs = path.resolve(ROOT, args.markdownPath || DEFAULT_MARKDOWN_PATH);
  writeJsonArtifact(outLatestAbs, report);
  writeJsonArtifact(stateLatestAbs, report);
  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: outAbs,
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));
