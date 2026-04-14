#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/tech_debt_report_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/TECH_DEBT_REPORT_CURRENT.md';
const DEFAULT_LEDGER = 'docs/workspace/reports/TECH_DEBT_LEDGER.json';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  ledgerPath: string;
};

function resolveArgs(argv: string[]): ScriptArgs {
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    outJson: readFlag(argv, 'out-json') || DEFAULT_OUT_JSON,
    outMarkdown: readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD,
    ledgerPath: readFlag(argv, 'ledger') || DEFAULT_LEDGER,
  };
}

function readJsonMaybe<T>(filePath: string, fallback: T): T {
  const abs = path.resolve(ROOT, filePath);
  if (!fs.existsSync(abs)) return fallback;
  try {
    return JSON.parse(fs.readFileSync(abs, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function countBy<T>(rows: T[], keyFn: (row: T) => string) {
  const counts: Record<string, number> = {};
  for (const row of rows) {
    const key = keyFn(row);
    counts[key] = (counts[key] || 0) + 1;
  }
  return Object.fromEntries(Object.entries(counts).sort((a, b) => b[1] - a[1]));
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Tech Debt Report');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Policy green but debt remaining: ${payload.summary.policy_green_but_debt_remaining}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- ledger_items: ${payload.summary.ledger_items}`);
  lines.push(`- open_items: ${payload.summary.open_items}`);
  lines.push(`- blocked_items: ${payload.summary.blocked_items}`);
  lines.push(`- size_exceptions: ${payload.summary.size_exceptions}`);
  lines.push(`- classic_asset_files: ${payload.summary.classic_asset_files}`);
  lines.push('');
  lines.push('## Open Debt Items');
  for (const row of payload.open_items) {
    lines.push(`- ${row.id} [${row.category}] ${row.summary} (${row.status}, owner ${row.owner}, batch ${row.planned_batch_date})`);
  }
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = resolveArgs(argv);
  const ledger = readJsonMaybe<any>(args.ledgerPath, { items: [] });
  const policyDebt = readJsonMaybe<any>('core/local/artifacts/policy_debt_summary_current.json', null);
  const classicDashboard = readJsonMaybe<any>('core/local/artifacts/classic_dashboard_debt_inventory_current.json', null);
  const clientLegacy = readJsonMaybe<any>('core/local/artifacts/client_legacy_debt_report_current.json', null);
  const adapterFallback = readJsonMaybe<any>('core/local/artifacts/orchestration_adapter_fallback_guard_current.json', null);

  const items = Array.isArray(ledger.items) ? ledger.items : [];
  const openItems = items.filter((row) => row.status !== 'done');
  const blockedItems = items.filter((row) => row.status === 'blocked');

  const payload = {
    ok: true,
    type: 'tech_debt_report',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
      ledger_path: args.ledgerPath,
    },
    summary: {
      ledger_items: items.length,
      open_items: openItems.length,
      blocked_items: blockedItems.length,
      by_category: countBy(openItems, (row: any) => String(row.category || 'unknown')),
      size_exceptions: Number(policyDebt?.debt?.size?.exception_count ?? 0),
      classic_asset_files: Number(classicDashboard?.summary?.classic_asset_files ?? 0),
      legacy_client_files: Number(clientLegacy?.summary?.total_files ?? 0),
      policy_green_but_debt_remaining:
        Boolean(policyDebt?.ok) &&
        (openItems.length > 0 ||
          Number(policyDebt?.debt?.size?.exempted ?? 0) > 0 ||
          Number(classicDashboard?.summary?.classic_asset_files ?? 0) > 0),
      adapter_fallback_guard_pass: Boolean(adapterFallback?.ok),
    },
    open_items: openItems,
    blocked_items: blockedItems,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
