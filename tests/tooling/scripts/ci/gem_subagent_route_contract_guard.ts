#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type GemPolicy = {
  subagent_contract?: {
    route_id?: string;
    scheduling_authority?: string;
    fallback_mode?: string;
    required_runtime_file?: string;
  };
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/gem_feedback_closure_policy.json');
const DEFAULT_OUT_PATH = 'core/local/artifacts/gem_subagent_route_contract_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/gem_subagent_route_contract_latest.json';
const DEFAULT_STATE_LATEST_PATH = 'local/state/ops/gem_subagent_route_contract/latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/GEM_SUBAGENT_ROUTE_CONTRACT_CURRENT.md';

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

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function hasAny(content: string, needles: string[]): boolean {
  const lowered = cleanText(content, 8_000_000).toLowerCase();
  return needles.some((needle) => lowered.includes(needle.toLowerCase()));
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# GEM Subagent Route Contract (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- runtime_file: ${cleanText(report.runtime_route?.path || '', 220)}`);
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
  const subagent = policy.subagent_contract || {};
  const routeId = cleanText(subagent.route_id || 'spawn_subagents', 120);
  const schedulingAuthority = cleanText(subagent.scheduling_authority || 'kernel', 80).toLowerCase();
  const fallbackMode = cleanText(subagent.fallback_mode || 'fail_closed', 80).toLowerCase();
  const runtimeFile = cleanText(
    subagent.required_runtime_file || 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/131-part.rs',
    500,
  );
  const runtimeAbs = path.resolve(ROOT, runtimeFile);
  const runtimePresent = fs.existsSync(runtimeAbs);
  const runtimeSource = runtimePresent ? fs.readFileSync(runtimeAbs, 'utf8') : '';

  const checks = [
    {
      id: 'gem_subagent_route_id_present',
      ok: routeId === 'spawn_subagents',
      detail: `route_id=${routeId}`,
    },
    {
      id: 'gem_subagent_runtime_file_present',
      ok: runtimePresent,
      detail: `required_runtime_file=${runtimeFile};present=${runtimePresent}`,
    },
    {
      id: 'gem_subagent_route_implemented_in_runtime_file',
      ok: runtimePresent && hasAny(runtimeSource, [routeId, 'spawn_subagents']),
      detail: `runtime file must expose ${routeId} route`,
    },
    {
      id: 'gem_subagent_authority_bound_scheduling_contract',
      ok:
        schedulingAuthority === 'kernel'
          ? runtimePresent
            && hasAny(runtimeSource, [
              'spawn_guard_policy',
              'max_descendants_per_parent',
              'depth_limit',
              'spawn_budget_cap',
            ])
          : false,
      detail: `scheduling_authority=${schedulingAuthority};required_keywords=spawn_guard_policy|max_descendants_per_parent|depth_limit|spawn_budget_cap`,
    },
    {
      id: 'gem_subagent_deterministic_receipt_contract',
      ok:
        runtimePresent
        && hasAny(runtimeSource, ['deterministic_receipt_hash', 'receipt_hash', '"receipt"']),
      detail: 'runtime route must emit deterministic receipt lineage fields',
    },
    {
      id: 'gem_subagent_fail_closed_fallback_contract',
      ok:
        fallbackMode === 'fail_closed'
          ? runtimePresent
            && hasAny(runtimeSource, ['spawn_budget_exceeded', 'error', 'return json!'])
          : false,
      detail: `fallback_mode=${fallbackMode};expected fail-closed error surface tokens`,
    },
  ];

  const failed = checks.filter((row) => !row.ok);
  const report = {
    type: 'gem_subagent_route_contract_guard',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    ok: failed.length === 0,
    policy_path: path.relative(ROOT, POLICY_PATH),
    checks,
    failed_ids: failed.map((row) => row.id),
    runtime_route: {
      path: path.relative(ROOT, runtimeAbs),
      present: runtimePresent,
      route_id: routeId,
      scheduling_authority: schedulingAuthority,
      fallback_mode: fallbackMode,
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
