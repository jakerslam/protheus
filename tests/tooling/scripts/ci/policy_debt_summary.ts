#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/policy_debt_summary_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/POLICY_DEBT_SUMMARY_CURRENT.md';

type ScriptArgs = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

function parseArgs(argv: string[]): ScriptArgs {
  const out: ScriptArgs = {
    strict: false,
    outJson: DEFAULT_OUT_JSON,
    outMarkdown: DEFAULT_OUT_MD,
  };
  for (const raw of argv) {
    const arg = String(raw || '').trim();
    if (!arg) continue;
    if (arg === '--strict' || arg === '--strict=1') out.strict = true;
    else if (arg.startsWith('--strict=')) {
      const value = arg.slice('--strict='.length).trim().toLowerCase();
      out.strict = ['1', 'true', 'yes', 'on'].includes(value);
    } else if (arg.startsWith('--out-json=')) {
      out.outJson = arg.slice('--out-json='.length).trim() || out.outJson;
    } else if (arg.startsWith('--out-markdown=')) {
      out.outMarkdown = arg.slice('--out-markdown='.length).trim() || out.outMarkdown;
    }
  }
  return out;
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

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Policy Debt Summary');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push('');
  lines.push('## Gate Summary');
  for (const gate of payload.gates) {
    lines.push(`- ${gate.id}: ${gate.ok ? 'pass' : 'fail'}${gate.detail ? ` (${gate.detail})` : ''}`);
  }
  lines.push('');
  lines.push('## Debt Summary');
  lines.push(`- exception_count: ${payload.debt.size.exception_count}`);
  lines.push(`- exception_count_ceiling: ${payload.debt.size.exception_count_ceiling ?? 'n/a'}`);
  lines.push(`- oversized_files: ${payload.debt.size.oversized}`);
  lines.push(`- exempted_oversized_files: ${payload.debt.size.exempted}`);
  lines.push(`- expired_debt_rules: ${payload.debt.expiry.violation_count}`);
  lines.push(`- expiring_soon_rules: ${payload.debt.expiry.expiring_soon_count}`);
  lines.push('');
  lines.push('## Top Expiring Soon');
  if (payload.top_expiring_soon.length === 0) {
    lines.push('- none');
  } else {
    for (const row of payload.top_expiring_soon) {
      lines.push(`- ${row.file}: ${row.detail} (expires ${row.expires})`);
    }
  }
  lines.push('');
  lines.push('## Top Oversized');
  if (payload.top_oversized.length === 0) {
    lines.push('- none');
  } else {
    for (const row of payload.top_oversized) {
      lines.push(`- ${row.path}: ${row.lines} lines (cap ${row.cap}, ${row.status}, expires ${row.expires || 'n/a'})`);
    }
  }
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = parseArgs(argv);
  const srs = readJsonMaybe<any>('core/local/artifacts/srs_full_regression_current.json', null);
  const size = readJsonMaybe<any>('core/local/artifacts/repo_file_size_gate_current.json', null);
  const expiry = readJsonMaybe<any>('core/local/artifacts/debt_expiry_guard_current.json', null);
  const closure = readJsonMaybe<any>('core/local/artifacts/production_readiness_closure_gate_current.json', null);
  const arch = readJsonMaybe<any>('core/local/artifacts/arch_boundary_conformance_current.json', null);
  const registry = readJsonMaybe<any>('core/local/artifacts/tooling_registry_contract_guard_current.json', null);

  const gates = [
    {
      id: 'srs_full_regression',
      ok: srs?.summary?.regression?.fail === 0 && srs?.summary?.regression?.warn === 0,
      detail: srs ? `fail=${srs.summary.regression.fail}; warn=${srs.summary.regression.warn}` : 'missing_artifact',
    },
    {
      id: 'repo_file_size_gate',
      ok: size?.summary?.pass === true,
      detail: size ? `violations=${size.summary.violations}; exceptions=${size.summary.exception_count}` : 'missing_artifact',
    },
    {
      id: 'debt_expiry_guard',
      ok: expiry?.summary?.pass === true,
      detail: expiry ? `violations=${expiry.summary.violation_count}; expiring_soon=${expiry.summary.expiring_soon_count}` : 'missing_artifact',
    },
    {
      id: 'production_readiness_closure',
      ok: closure?.summary?.pass === true,
      detail: closure ? `failed=${closure.summary.failed_count || 0}` : 'missing_artifact',
    },
    {
      id: 'arch_boundary_conformance',
      ok: arch?.summary?.pass === true,
      detail: arch ? `violations=${arch.summary.violation_count}` : 'missing_artifact',
    },
    {
      id: 'tooling_registry_contract_guard',
      ok: registry?.summary?.pass === true,
      detail: registry ? `failures=${registry.summary.failure_count}` : 'missing_artifact',
    },
  ];

  const payload = {
    ok: gates.every((row) => row.ok),
    type: 'policy_debt_summary',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    gates,
    debt: {
      size: {
        exception_count: size?.summary?.exception_count ?? null,
        exception_count_ceiling: size?.summary?.exception_count_ceiling ?? null,
        oversized: size?.summary?.oversized ?? null,
        exempted: size?.summary?.exempted ?? null,
      },
      expiry: {
        violation_count: expiry?.summary?.violation_count ?? null,
        expiring_soon_count: expiry?.summary?.expiring_soon_count ?? null,
        warn_days: expiry?.summary?.warn_days ?? null,
      },
    },
    top_expiring_soon: Array.isArray(expiry?.expiring_soon) ? expiry.expiring_soon.slice(0, 10) : [],
    top_oversized: Array.isArray(size?.oversized_inventory) ? size.oversized_inventory.slice(0, 10) : [],
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
