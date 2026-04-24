#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type GemPolicy = {
  version?: number;
  providers?: Record<
    string,
    {
      typed_capability?: string;
      required_probe_key?: string;
    }
  >;
  bootstrap_contract?: {
    required_env?: string[];
    required_skip_reasons?: string[];
  };
  rate_limit_contract?: {
    per_provider_burst_max?: number;
    sustained_rps_max?: number;
    on_budget_exhausted?: string;
  };
  circuit_breaker_contract?: {
    states?: string[];
    quarantine_transition_required?: boolean;
    cooldown_seconds_min?: number;
  };
  cache_contract?: {
    skip_reason_required_when_skipped?: boolean;
    stale_age_seconds_field_required?: boolean;
    write_block_on_provider_failure?: boolean;
  };
  diagnostics_contract?: {
    required_fields?: string[];
    boilerplate_denied_phrases?: string[];
  };
  provider_failure_reason_codes?: string[];
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/gem_feedback_closure_policy.json');
const LIVE_SMOKE_PATH = path.join(ROOT, 'core/local/artifacts/gem_live_provider_smoke_current.json');
const MEMORY_PATH = path.join(ROOT, 'core/local/artifacts/gem_memory_durability_current.json');
const SUBAGENT_PATH = path.join(ROOT, 'core/local/artifacts/gem_subagent_route_contract_current.json');
const WEB_SOAK_PATH = path.join(ROOT, 'artifacts/web_tooling_context_soak_report_latest.json');
const DEFAULT_OUT_PATH = 'core/local/artifacts/gem_feedback_closure_guard_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/gem_feedback_closure_guard_latest.json';
const DEFAULT_STATE_LATEST_PATH = 'local/state/ops/gem_feedback_closure_guard/latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/GEM_FEEDBACK_CLOSURE_GUARD_CURRENT.md';

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

function readJson(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function hasBoilerplate(value: string, denied: string[]): boolean {
  const lowered = cleanText(value, 8000).toLowerCase();
  return denied.some((needle) => lowered.includes(cleanText(needle, 200).toLowerCase()));
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# GEM Feedback Closure Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## GEM Coverage');
  for (const [id, pass] of Object.entries(report.gem_coverage || {})) {
    lines.push(`- ${id}: ${pass === true ? 'pass' : 'fail'}`);
  }
  lines.push('');
  lines.push('## Checks');
  for (const row of Array.isArray(report.checks) ? report.checks : []) {
    lines.push(
      `- ${cleanText((row as any).id || 'unknown', 120)}: ${(row as any).ok === true ? 'pass' : 'fail'} (${cleanText((row as any).detail || '', 240)})`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const policy = (readJson(POLICY_PATH) || {}) as GemPolicy;
  const live = readJson(LIVE_SMOKE_PATH);
  const memory = readJson(MEMORY_PATH);
  const subagent = readJson(SUBAGENT_PATH);
  const webSoak = readJson(WEB_SOAK_PATH);

  const providers = policy.providers || {};
  const providerRows = Object.entries(providers);
  const requiredReasonCodes = new Set(
    asArray(policy.provider_failure_reason_codes).map((row) => cleanText(row, 120)).filter(Boolean),
  );
  const deniedPhrases = asArray(policy.diagnostics_contract?.boilerplate_denied_phrases)
    .map((row) => cleanText(row, 200))
    .filter(Boolean);

  const failedProviderRows = asArray(live?.providers).filter((row: any) => row?.ok !== true);
  const diagnosticFields = asArray(policy.diagnostics_contract?.required_fields)
    .map((row) => cleanText(row, 120))
    .filter(Boolean);
  const diagnosticsFieldsOk = failedProviderRows.every((row: any) =>
    diagnosticFields.every((field) => cleanText(row?.[field], 200).length > 0),
  );
  const reasonCodesOk = failedProviderRows.every((row: any) => {
    const reason = cleanText(row?.provider_failure_reason, 120);
    return reason.length > 0 && (requiredReasonCodes.size === 0 || requiredReasonCodes.has(reason));
  });
  const deniedPhraseDetected = failedProviderRows.some((row: any) =>
    hasBoilerplate(
      [row?.route_failure_reason, row?.probe_failure_reason, row?.provider_failure_reason, row?.next_fix_hint]
        .map((value) => cleanText(value, 400))
        .join(' '),
      deniedPhrases,
    ),
  );

  const webTaxonomyContract = webSoak?.taxonomy_contract || {};
  const cacheSkipReasonMissingCount = Number(webTaxonomyContract.cache_skip_reason_missing_count || 0);
  const cacheWriteGateViolationCount = Number(webTaxonomyContract.cache_write_gate_violation_count || 0);
  const cacheStaleAgeMissingCount = Number(webTaxonomyContract.cache_stale_age_missing_count || 0);
  const cacheContractOk =
    cacheSkipReasonMissingCount === 0 &&
    cacheWriteGateViolationCount === 0 &&
    cacheStaleAgeMissingCount === 0;

  const checks = [
    {
      id: 'gem_policy_loaded',
      ok: Number(policy.version || 0) >= 1,
      detail: `policy_version=${Number(policy.version || 0)}`,
    },
    {
      id: 'gem_typed_provider_routes_present',
      ok:
        providerRows.length >= 2 &&
        cleanText(providers.web_search?.typed_capability, 120) === 'web_search' &&
        cleanText(providers.web_search?.required_probe_key, 120) === 'web_search' &&
        cleanText(providers.web_fetch?.typed_capability, 120) === 'web_fetch' &&
        cleanText(providers.web_fetch?.required_probe_key, 120) === 'web_fetch',
      detail: `provider_rows=${providerRows.length};required=web_search+web_fetch`,
    },
    {
      id: 'gem_live_provider_smoke_artifact_present',
      ok: live?.type === 'gem_live_provider_smoke',
      detail: `live_smoke_type=${cleanText(live?.type, 120) || 'missing'}`,
    },
    {
      id: 'gem_bootstrap_preflight_contract',
      ok: asArray(live?.checks).some((row: any) => cleanText(row?.id, 160) === 'gem_bootstrap_required_env_contract'),
      detail: 'live smoke report includes deterministic bootstrap preflight check',
    },
    {
      id: 'gem_diagnostics_fields_actionable',
      ok: diagnosticsFieldsOk,
      detail: `failed_provider_rows=${failedProviderRows.length};required_fields=${diagnosticFields.join(',')}`,
    },
    {
      id: 'gem_provider_failure_reason_codes_canonical',
      ok: reasonCodesOk,
      detail: `required_reason_codes=${Array.from(requiredReasonCodes).join(',')}`,
    },
    {
      id: 'gem_failure_guidance_no_workflow_loop_boilerplate',
      ok: !deniedPhraseDetected,
      detail: `denied_phrases=${deniedPhrases.length};detected=${deniedPhraseDetected}`,
    },
    {
      id: 'gem_web_cache_contract_enforced',
      ok: cacheContractOk,
      detail: `cache_skip_reason_missing_count=${cacheSkipReasonMissingCount};cache_write_gate_violation_count=${cacheWriteGateViolationCount};cache_stale_age_missing_count=${cacheStaleAgeMissingCount}`,
    },
    {
      id: 'gem_memory_durability_lane_artifact_present_and_passed',
      ok: memory?.type === 'gem_memory_durability_lane' && memory?.ok === true,
      detail: `memory_artifact_type=${cleanText(memory?.type, 120) || 'missing'};ok=${memory?.ok === true}`,
    },
    {
      id: 'gem_subagent_route_contract_artifact_present_and_passed',
      ok: subagent?.type === 'gem_subagent_route_contract_guard' && subagent?.ok === true,
      detail: `subagent_artifact_type=${cleanText(subagent?.type, 120) || 'missing'};ok=${subagent?.ok === true}`,
    },
    {
      id: 'gem_rate_limit_contract_fail_closed',
      ok:
        Number(policy.rate_limit_contract?.per_provider_burst_max || 0) > 0 &&
        Number(policy.rate_limit_contract?.sustained_rps_max || 0) > 0 &&
        cleanText(policy.rate_limit_contract?.on_budget_exhausted, 120) === 'fail_closed',
      detail: `burst=${Number(policy.rate_limit_contract?.per_provider_burst_max || 0)};rps=${Number(policy.rate_limit_contract?.sustained_rps_max || 0)};on_budget_exhausted=${cleanText(policy.rate_limit_contract?.on_budget_exhausted, 120)}`,
    },
    {
      id: 'gem_circuit_breaker_contract_full',
      ok: (() => {
        const states = new Set(
          asArray(policy.circuit_breaker_contract?.states)
            .map((row) => cleanText(row, 60))
            .filter(Boolean),
        );
        return (
          states.has('open') &&
          states.has('half_open') &&
          states.has('closed') &&
          policy.circuit_breaker_contract?.quarantine_transition_required === true &&
          Number(policy.circuit_breaker_contract?.cooldown_seconds_min || 0) > 0
        );
      })(),
      detail: `states=${asArray(policy.circuit_breaker_contract?.states).map((row) => cleanText(row, 60)).join('|')}`,
    },
  ];

  const failed = checks.filter((row) => !row.ok);
  const coverage = {
    'GEM-001': checks[1].ok,
    'GEM-002': checks[5].ok,
    'GEM-003': checks[8].ok,
    'GEM-004': checks[9].ok,
    'GEM-005': checks[4].ok,
    'GEM-006': checks[10].ok,
    'GEM-007': checks[11].ok,
    'GEM-008': checks[7].ok,
    'GEM-009': checks[2].ok,
    'GEM-010': checks[8].ok,
    'GEM-011': checks[6].ok,
    'GEM-012': checks[3].ok,
  };

  const report = {
    type: 'gem_feedback_closure_guard',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    ok: failed.length === 0,
    checks,
    failed_ids: failed.map((row) => row.id),
    gem_coverage: coverage,
    sources: {
      policy: path.relative(ROOT, POLICY_PATH),
      live_provider_smoke: path.relative(ROOT, LIVE_SMOKE_PATH),
      memory_durability: path.relative(ROOT, MEMORY_PATH),
      subagent_route_contract: path.relative(ROOT, SUBAGENT_PATH),
      web_soak: path.relative(ROOT, WEB_SOAK_PATH),
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
