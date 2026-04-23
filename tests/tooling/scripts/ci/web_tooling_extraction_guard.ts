#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type ToolMode = 'web_search' | 'web_fetch';
type InferredMode = ToolMode | 'ambiguous' | 'unknown';

type ContractCase = {
  id: string;
  mode: ToolMode;
  raw_input: string;
  expected_contains: string[];
  forbidden_contains: string[];
  sources: string[];
};

type CaseResult = {
  id: string;
  mode: ToolMode;
  inferred_mode: InferredMode;
  mode_match: boolean;
  expected_missing: string[];
  forbidden_present: string[];
  source_attribution_ok: boolean;
  source_attribution_issues: string[];
  extraction_ok: boolean;
  ok: boolean;
};

type ReliabilityCounters = {
  total_cases: number;
  passed_cases: number;
  failed_cases: number;
  extraction_fidelity_fail_count: number;
  parse_drift_count: number;
  ambiguous_fetch_search_parse_count: number;
  chrome_leak_count: number;
  source_attribution_failure_count: number;
  provider_failure_contract_violation_count: number;
  cache_skip_reason_missing_count: number;
  cache_write_gate_violation_count: number;
  cache_stale_age_missing_count: number;
  benchmark_instruction_leak_count: number;
  benchmark_intent_overlap_hygiene_violation_count: number;
  metadata_card_leak_count: number;
  metadata_card_line_shape_leak_count: number;
  raw_function_scaffold_leak_count: number;
  tool_trace_blocked_scaffold_leak_count: number;
  runtime_capability_template_leak_count: number;
  workflow_retry_template_leak_count: number;
  workflow_unexpected_state_template_leak_count: number;
  source_receipt_scaffold_leak_count: number;
  tool_routing_diagnosis_template_leak_count: number;
  ingress_policy_preamble_template_leak_count: number;
  tool_block_input_result_scaffold_leak_count: number;
  workflow_loop_leak_contract_violation_count: number;
  file_tool_route_misdirection_contract_violation_count: number;
};

const DEFAULT_FIXTURE_PATH = 'tests/tooling/fixtures/web_tooling_extraction_contract_matrix.json';
const DEFAULT_SOAK_PATH = 'artifacts/web_tooling_context_soak_report_latest.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/web_tooling_reliability_current.json';
const DEFAULT_OUT_LATEST = 'artifacts/web_tooling_reliability_latest.json';
const DEFAULT_STATE_PATH = 'local/state/ops/web_tooling_reliability/latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/WEB_TOOLING_RELIABILITY_CURRENT.md';

const DEFAULT_FORBIDDEN_TOKENS = [
  '<style',
  '<script',
  'font-family',
  'display:flex',
  'margin:',
  'padding:',
  'search tools',
  'images videos maps news',
  'next actions:',
  'run one targeted tool call',
  'return a concise answer from current context',
  'ops:benchmark:refresh',
  'ops:benchmark:sanity',
  'ops:benchmark:public-audit',
  'ops:benchmark:repro',
  'i completed the workflow gate, but the final workflow state was unexpected',
  'final reply did not render',
  'final workflow state was unexpected',
  'i completed the run, but the final reply did not render',
  'please retry so i can rerun the chain cleanly',
  'ask me to continue and i will synthesize from the recorded workflow state',
  'ask me to continue and i will synthesize from recorded workflow state',
  'title:',
  'excerpt:',
  'originalurl:',
  'featuredcontent:',
  '<function=',
  '</function>',
  'tool trace complete',
  'done · 1 blocked',
  'file list blocked',
  'ingress delivery policy',
  'lease_denied:client_ingress_domain_boundary',
  'this is a policy gate, not a web-provider outage',
  'file_list was blocked by ingress delivery policy in this runtime lane',
  'i can access runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces in this session',
  'i can access runtime telemetry',
  'approved command surfaces in this session',
  'tell me what you want me to check and i will run it now',
  '[source:local_context]',
  '[source:tool_receipt:',
  'the system is still automatically triggering tool calls without my conscious selection',
  'fundamental misclassification error',
  'tool routing mechanism is clearly malfunctioning',
  'requires recalibration to properly distinguish between internal system operations and external data retrieval requests',
  'the first gate ("workflow_route") is still classifying this as an "info" route rather than a "task" route',
  'the first gate ("task_or_info_route") is still classifying this as an "info" route rather than a "task" route',
  'the first gate ("workflow_route") is a binary classification',
  'the first gate ("task_or_info_route") is a binary classification',
  "it's not a true/false decision i control",
  'automated classification based on semantic analysis',
  'otherwise, it defaults to info',
  'the system needs explicit tool-related phrasing to trigger the task classification path',
  'it\'s still seeing this as a conversational exchange rather than a tool operation request',
  'it is still seeing this as a conversational exchange rather than a tool operation request',
  '[source:workflow_gate]',
  'the file list step was blocked before i could finish the answer',
  'tool trace complete1 done · 1 blocked',
  'result `file_list` was blocked by ingress delivery policy in this runtime lane',
];

const PROVIDER_FAILURE_MODE_ALLOWLIST = new Set([
  'provider_registry_missing',
  'provider_registry_empty',
  'provider_auth_missing',
  'provider_unreachable',
  'provider_partial_degradation',
  'search_provider_unavailable',
  'fetch_provider_unavailable',
  'provider_timeout',
  'provider_rate_limited',
]);

function readJson<T>(pathname: string): T | null {
  try {
    return JSON.parse(fs.readFileSync(pathname, 'utf8')) as T;
  } catch {
    return null;
  }
}

function writeJson(pathname: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeMarkdown(pathname: string, body: string): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, body, 'utf8');
}

function sanitizeWebBody(raw: string): string {
  let out = String(raw || '');
  out = out.replace(/<script[\s\S]*?<\/script>/gi, ' ');
  out = out.replace(/<style[\s\S]*?<\/style>/gi, ' ');
  out = out.replace(/<noscript[\s\S]*?<\/noscript>/gi, ' ');
  out = out.replace(/<function=[\s\S]*?<\/function>/gi, ' ');
  out = out.replace(/<function=[^\n>]*>/gi, ' ');
  out = out.replace(/<\/function>/gi, ' ');
  out = out.replace(/tool trace complete[^\n]*/gi, ' ');
  out = out.replace(/(^|\n)\s*file list\s*blocked[^\n]*/gi, ' ');
  out = out.replace(/this is a policy gate,\s*not a web-provider outage\.?/gi, ' ');
  out = out.replace(
    /result\s*`?file_list`?\s*was blocked by ingress delivery policy[^\n]*/gi,
    ' ',
  );
  out = out.replace(
    /`?file_list`?\s*was blocked by ingress delivery policy in this runtime lane[^\n]*/gi,
    ' ',
  );
  out = out.replace(
    /i can access runtime telemetry, persistent memory, workspace files, channels, and approved command surfaces in this session\.?/gi,
    ' ',
  );
  out = out.replace(
    /tell me what you want me to check and i will run it now\.?/gi,
    ' ',
  );
  out = out.replace(/lease_denied:client_ingress_domain_boundary/gi, ' ');
  out = out.replace(/^\s*input\s*\{[\s\S]*?\}\s*$/gim, ' ');
  out = out.replace(
    /(^|\n)\s*input\s*\{[\s\S]{0,1500}?\}\s*(?=\n\s*(result|tool|source|$))/gim,
    ' ',
  );
  out = out.replace(
    /(^|\n)\s*(title|excerpt|originalurl|original_url|type|featuredcontent|publisheddatetime|provider|images?)\s*:\s*[^\n]*/gi,
    ' ',
  );
  out = out.replace(
    /i completed the workflow gate,[^\n]*final workflow state was unexpected[^\n]*/gi,
    ' ',
  );
  out = out.replace(
    /i completed the run,[^\n]*final reply did not render[^\n]*/gi,
    ' ',
  );
  out = out.replace(
    /please retry so i can rerun the chain cleanly\.?/gi,
    ' ',
  );
  out = out.replace(
    /ask me to continue and i will synthesize from (the )?recorded workflow state\.?/gi,
    ' ',
  );
  out = out.replace(/\[source:[^\]]+\]/gi, ' ');
  out = out.replace(/source:tool_receipt:[^\s\]]+/gi, ' ');
  out = out.replace(
    /the system is still automatically triggering tool calls without my conscious selection\.?/gi,
    ' ',
  );
  out = out.replace(/fundamental misclassification error\.?/gi, ' ');
  out = out.replace(/tool routing mechanism is clearly malfunctioning\.?/gi, ' ');
  out = out.replace(
    /requires recalibration to properly distinguish between internal system operations and external data retrieval requests\.?/gi,
    ' ',
  );
  out = out.replace(
    /the first gate\s*\("(workflow_route|task_or_info_route)"\)\s*is still classifying this as an\s*"info"\s*route rather than a\s*"task"\s*route\.?/gi,
    ' ',
  );
  out = out.replace(
    /the first gate\s*\("(workflow_route|task_or_info_route)"\)\s*is a binary classification[\s\S]{0,500}?(?:defaults to info|routes to task)\.?/gi,
    ' ',
  );
  out = out.replace(
    /it'?s not a true\/false decision i control\s*-\s*it'?s an automated classification based on semantic analysis of the user'?s input\.?/gi,
    ' ',
  );
  out = out.replace(
    /the system needs explicit tool-related phrasing to trigger the task classification path\.?/gi,
    ' ',
  );
  out = out.replace(
    /it'?s still seeing this as a conversational exchange rather than a tool operation request\.?/gi,
    ' ',
  );
  out = out.replace(
    /the file list step was blocked before i could finish the answer:?/gi,
    ' ',
  );
  out = out.replace(/tool trace complete\d*\s*done\s*[·•]\s*\d+\s*blocked/gi, ' ');
  out = out.replace(
    /result\s*`?file_list`?\s*was blocked by ingress delivery policy in this runtime lane[^\n]*/gi,
    ' ',
  );
  out = out.replace(
    /next actions:\s*1\)[\s\S]*?return a concise answer from current context/gi,
    ' ',
  );
  out = out.replace(
    /next actions:\s*run one targeted tool call,\s*then return a concise answer from current context\.?/gi,
    ' ',
  );
  out = out.replace(/\[source:workflow_gate\]/gi, ' ');
  out = out.replace(/npm\s+run\s+-s\s+ops:benchmark:[a-z0-9-]+/gi, ' ');
  out = out.replace(/ops:benchmark:(refresh|sanity|public-audit|repro)/gi, ' ');
  out = out.replace(/<[^>]+>/g, ' ');
  out = out.replace(/&nbsp;/gi, ' ');
  out = out.replace(/&amp;/gi, '&');
  out = out.replace(/[a-z-]+\s*:\s*[^;{}]+;/gi, ' ');
  out = out.replace(/\s+/g, ' ').trim();
  return out;
}

function inferMode(raw: string): InferredMode {
  const lower = String(raw || '').toLowerCase();
  const searchSignal =
    /search result|query:|serp|result\s+\d|snippet|top stories|related searches/.test(lower);
  const fetchSignal =
    /<html|<body|<article|content-type:\s*text\/html|http\/\d\.\d\s+200|<!doctype html/.test(lower);
  if (searchSignal && fetchSignal) return 'ambiguous';
  if (searchSignal) return 'web_search';
  if (fetchSignal) return 'web_fetch';
  return 'unknown';
}

function textHasAnyPattern(raw: string, patterns: RegExp[]): boolean {
  return patterns.some((pattern) => pattern.test(raw));
}

function sourceAttributionIssues(sources: string[]): string[] {
  const rows = Array.isArray(sources) ? sources.map((value) => cleanText(value || '', 400)) : [];
  const issues: string[] = [];
  if (rows.length === 0) issues.push('source_count_missing');
  if (rows.some((value) => !/^https?:\/\//i.test(value))) issues.push('source_url_invalid');
  const duplicates = rows.filter((value, idx, arr) => arr.indexOf(value) !== idx);
  if (duplicates.length > 0) issues.push('source_duplicates_present');
  return issues;
}

function asObject(value: unknown): Record<string, unknown> {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
}

function readBooleanLike(value: unknown): boolean | null {
  if (typeof value === 'boolean') return value;
  if (typeof value === 'number') return value !== 0;
  if (typeof value === 'string') {
    const lowered = cleanText(value, 20).toLowerCase();
    if (['true', '1', 'yes', 'y', 'on'].includes(lowered)) return true;
    if (['false', '0', 'no', 'n', 'off'].includes(lowered)) return false;
  }
  return null;
}

function readNumberLike(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string') {
    const parsed = Number.parseFloat(cleanText(value, 80));
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

function evaluateProviderCacheContracts(soak: any) {
  const taxonomy = asObject(soak?.taxonomy);
  const cache = asObject(taxonomy.cache);
  const cacheSkip = asObject(taxonomy.cache_skip);
  const providerRows = Array.isArray(taxonomy.provider_failures)
    ? taxonomy.provider_failures
    : Array.isArray(taxonomy.provider_failure_events)
      ? taxonomy.provider_failure_events
      : Array.isArray(taxonomy.failures)
        ? taxonomy.failures
        : [];
  const providerFailureModes = new Set<string>();
  let providerFailureContractViolationCount = 0;
  for (const rawRow of providerRows) {
    const row = asObject(rawRow);
    const mode = cleanText(
      String(
        row.failure_mode ?? row.reason ?? row.code ?? row.kind ?? row.status ?? row.type ?? '',
      ),
      80,
    ).toLowerCase();
    if (!mode) {
      providerFailureContractViolationCount += 1;
      continue;
    }
    providerFailureModes.add(mode);
    if (!PROVIDER_FAILURE_MODE_ALLOWLIST.has(mode)) {
      providerFailureContractViolationCount += 1;
    }
  }
  const explicitProviderModes = [
    taxonomy.provider_failure_mode,
    taxonomy.search_provider_failure_mode,
    taxonomy.fetch_provider_failure_mode,
    cache.provider_failure_mode,
    cache.search_provider_failure_mode,
    cache.fetch_provider_failure_mode,
    cacheSkip.provider_failure_mode,
    cacheSkip.search_provider_failure_mode,
    cacheSkip.fetch_provider_failure_mode,
  ]
    .map((value) => cleanText(String(value ?? ''), 80).toLowerCase())
    .filter(Boolean);
  for (const mode of explicitProviderModes) {
    providerFailureModes.add(mode);
    if (!PROVIDER_FAILURE_MODE_ALLOWLIST.has(mode)) {
      providerFailureContractViolationCount += 1;
    }
  }

  const providerFailureCount = providerFailureModes.size;
  const cacheSkipped =
    readBooleanLike(taxonomy.cache_skipped) ??
    readBooleanLike(cache.skipped) ??
    readBooleanLike(cacheSkip.skipped) ??
    false;
  const cacheSkipReason = cleanText(
    String(
      taxonomy.cache_skip_reason ??
        cache.skip_reason ??
        cacheSkip.reason ??
        cache.reason ??
        '',
    ),
    160,
  );
  const cacheSkipReasonMissingCount = cacheSkipped && !cacheSkipReason ? 1 : 0;
  const cacheWriteAllowed =
    readBooleanLike(taxonomy.cache_write_allowed) ??
    readBooleanLike(cache.write_allowed) ??
    readBooleanLike(cacheSkip.write_allowed) ??
    false;
  const cacheWriteAttempted =
    readBooleanLike(taxonomy.cache_write_attempted) ??
    readBooleanLike(cache.write_attempted) ??
    false;
  const cacheStaleAgeSeconds =
    readNumberLike(taxonomy.cache_stale_age_seconds) ??
    readNumberLike(cache.stale_age_seconds) ??
    readNumberLike(cache.age_seconds) ??
    readNumberLike(taxonomy.age_seconds) ??
    null;
  const cacheStaleAgeRequired = cacheSkipped || providerFailureCount > 0 || cacheWriteAttempted;
  const cacheStaleAgeMissingCount =
    cacheStaleAgeRequired && (cacheStaleAgeSeconds == null || cacheStaleAgeSeconds < 0) ? 1 : 0;
  const cacheWriteGateViolationCount =
    providerFailureCount > 0 && (cacheWriteAllowed || cacheWriteAttempted) ? 1 : 0;
  return {
    provider_failure_count: providerFailureCount,
    provider_failure_modes: Array.from(providerFailureModes).sort(),
    provider_failure_contract_violation_count: providerFailureContractViolationCount,
    cache_skipped: cacheSkipped,
    cache_skip_reason: cacheSkipReason,
    cache_skip_reason_missing_count: cacheSkipReasonMissingCount,
    cache_write_allowed: cacheWriteAllowed,
    cache_write_attempted: cacheWriteAttempted,
    cache_stale_age_seconds: cacheStaleAgeSeconds,
    cache_stale_age_required: cacheStaleAgeRequired,
    cache_stale_age_missing_count: cacheStaleAgeMissingCount,
    cache_write_gate_violation_count: cacheWriteGateViolationCount,
  };
}

function evaluateSoakTaxonomyContracts(soak: any) {
  const taxonomyContract = asObject(soak?.taxonomy_contract);
  const taxonomy = asObject(soak?.taxonomy);
  const qualityTelemetry = asObject(taxonomy.quality_telemetry ?? taxonomy.telemetry ?? taxonomy.metrics);
  const contractFailures = Array.isArray(taxonomyContract.failures)
    ? taxonomyContract.failures.map((value) => cleanText(String(value || ''), 120).toLowerCase())
    : [];
  const workflowLoopLeakCount = Math.max(
    0,
    Math.trunc(
      readNumberLike(taxonomyContract.workflow_loop_leak_count)
      ?? readNumberLike(taxonomy.workflow_loop_leak_count)
      ?? readNumberLike(taxonomy.workflow_retry_loop_detected_count)
      ?? readNumberLike(qualityTelemetry.workflow_unexpected_state_loop_count)
      ?? readNumberLike(qualityTelemetry.unexpected_state_loop_count)
      ?? 0,
    ),
  );
  const fileToolRouteMisdirectionCount = Math.max(
    0,
    Math.trunc(
      readNumberLike(taxonomyContract.file_tool_route_misdirection_count)
      ?? readNumberLike(taxonomy.file_tool_route_misdirection_count)
      ?? readNumberLike(taxonomy.route_misdirection_count)
      ?? readNumberLike(qualityTelemetry.file_tool_route_misdirection_count)
      ?? readNumberLike(qualityTelemetry.route_misdirection_count)
      ?? 0,
    ),
  );
  const workflowLoopFailureTagged = contractFailures.includes('workflow_loop_leak_detected');
  const fileRouteFailureTagged = contractFailures.includes('file_tool_route_misdirection_detected');
  const workflowLoopContractViolationCount =
    workflowLoopLeakCount > 0 || workflowLoopFailureTagged ? Math.max(1, workflowLoopLeakCount) : 0;
  const fileRouteContractViolationCount =
    fileToolRouteMisdirectionCount > 0 || fileRouteFailureTagged
      ? Math.max(1, fileToolRouteMisdirectionCount)
      : 0;
  return {
    workflow_loop_leak_count: workflowLoopLeakCount,
    file_tool_route_misdirection_count: fileToolRouteMisdirectionCount,
    workflow_loop_leak_contract_violation_count: workflowLoopContractViolationCount,
    file_tool_route_misdirection_contract_violation_count: fileRouteContractViolationCount,
    failures: contractFailures,
  };
}

function renderMarkdown(report: any): string {
  const counters = report?.counters || {};
  const lines: string[] = [];
  lines.push('# Web Tooling Reliability (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report?.generated_at || '', 80)}`);
  lines.push(`- strict_mode: ${report?.strict_mode === true ? 'true' : 'false'}`);
  lines.push(`- ok: ${report?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Counters');
  lines.push(`- total_cases: ${Number(counters.total_cases || 0)}`);
  lines.push(`- failed_cases: ${Number(counters.failed_cases || 0)}`);
  lines.push(`- extraction_fidelity_fail_count: ${Number(counters.extraction_fidelity_fail_count || 0)}`);
  lines.push(`- parse_drift_count: ${Number(counters.parse_drift_count || 0)}`);
  lines.push(
    `- ambiguous_fetch_search_parse_count: ${Number(counters.ambiguous_fetch_search_parse_count || 0)}`,
  );
  lines.push(`- chrome_leak_count: ${Number(counters.chrome_leak_count || 0)}`);
  lines.push(
    `- source_attribution_failure_count: ${Number(counters.source_attribution_failure_count || 0)}`,
  );
  lines.push(
    `- provider_failure_contract_violation_count: ${Number(counters.provider_failure_contract_violation_count || 0)}`,
  );
  lines.push(
    `- cache_skip_reason_missing_count: ${Number(counters.cache_skip_reason_missing_count || 0)}`,
  );
  lines.push(
    `- cache_write_gate_violation_count: ${Number(counters.cache_write_gate_violation_count || 0)}`,
  );
  lines.push(
    `- cache_stale_age_missing_count: ${Number(counters.cache_stale_age_missing_count || 0)}`,
  );
  lines.push(
    `- benchmark_instruction_leak_count: ${Number(counters.benchmark_instruction_leak_count || 0)}`,
  );
  lines.push(
    `- benchmark_intent_overlap_hygiene_violation_count: ${Number(counters.benchmark_intent_overlap_hygiene_violation_count || 0)}`,
  );
  lines.push(`- metadata_card_leak_count: ${Number(counters.metadata_card_leak_count || 0)}`);
  lines.push(
    `- metadata_card_line_shape_leak_count: ${Number(counters.metadata_card_line_shape_leak_count || 0)}`,
  );
  lines.push(
    `- raw_function_scaffold_leak_count: ${Number(counters.raw_function_scaffold_leak_count || 0)}`,
  );
  lines.push(
    `- tool_trace_blocked_scaffold_leak_count: ${Number(counters.tool_trace_blocked_scaffold_leak_count || 0)}`,
  );
  lines.push(
    `- runtime_capability_template_leak_count: ${Number(counters.runtime_capability_template_leak_count || 0)}`,
  );
  lines.push(
    `- workflow_retry_template_leak_count: ${Number(counters.workflow_retry_template_leak_count || 0)}`,
  );
  lines.push(
    `- workflow_unexpected_state_template_leak_count: ${Number(counters.workflow_unexpected_state_template_leak_count || 0)}`,
  );
  lines.push(
    `- source_receipt_scaffold_leak_count: ${Number(counters.source_receipt_scaffold_leak_count || 0)}`,
  );
  lines.push(
    `- tool_routing_diagnosis_template_leak_count: ${Number(counters.tool_routing_diagnosis_template_leak_count || 0)}`,
  );
  lines.push(
    `- ingress_policy_preamble_template_leak_count: ${Number(counters.ingress_policy_preamble_template_leak_count || 0)}`,
  );
  lines.push(
    `- tool_block_input_result_scaffold_leak_count: ${Number(counters.tool_block_input_result_scaffold_leak_count || 0)}`,
  );
  lines.push(
    `- workflow_loop_leak_contract_violation_count: ${Number(counters.workflow_loop_leak_contract_violation_count || 0)}`,
  );
  lines.push(
    `- file_tool_route_misdirection_contract_violation_count: ${Number(counters.file_tool_route_misdirection_contract_violation_count || 0)}`,
  );
  const failures = Array.isArray(report?.failed_case_ids) ? report.failed_case_ids : [];
  lines.push('');
  lines.push(`- failed_case_ids: ${failures.join(',') || 'none'}`);
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 400),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST, 400),
    statePath: cleanText(readFlag(argv, 'state') || DEFAULT_STATE_PATH, 400),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 400),
    fixturePath: cleanText(readFlag(argv, 'fixture') || DEFAULT_FIXTURE_PATH, 400),
    soakPath: cleanText(readFlag(argv, 'soak') || DEFAULT_SOAK_PATH, 400),
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const fixtureAbs = path.resolve(root, args.fixturePath);
  const soakAbs = path.resolve(root, args.soakPath);
  const fixture = readJson<{ cases?: ContractCase[]; schema_id?: string; schema_version?: number }>(
    fixtureAbs,
  );
  const soak = readJson<any>(soakAbs);

  const failures: Array<{ id: string; detail: string }> = [];
  if (!fixture) {
    failures.push({ id: 'fixture_missing', detail: args.fixturePath });
  } else {
    if (cleanText(fixture.schema_id || '', 120) !== 'web_tooling_extraction_contract_matrix') {
      failures.push({ id: 'fixture_schema_id_invalid', detail: cleanText(fixture.schema_id || 'missing', 120) });
    }
    if (Number(fixture.schema_version || 0) !== 1) {
      failures.push({
        id: 'fixture_schema_version_invalid',
        detail: cleanText(String(fixture.schema_version ?? 'missing'), 40),
      });
    }
  }

  const cases = Array.isArray(fixture?.cases) ? fixture.cases : [];
  const caseResults: CaseResult[] = [];
  const counters: ReliabilityCounters = {
    total_cases: cases.length,
    passed_cases: 0,
    failed_cases: 0,
    extraction_fidelity_fail_count: 0,
    parse_drift_count: 0,
    ambiguous_fetch_search_parse_count: 0,
    chrome_leak_count: 0,
    source_attribution_failure_count: 0,
    provider_failure_contract_violation_count: 0,
    cache_skip_reason_missing_count: 0,
    cache_write_gate_violation_count: 0,
    cache_stale_age_missing_count: 0,
    benchmark_instruction_leak_count: 0,
    benchmark_intent_overlap_hygiene_violation_count: 0,
    metadata_card_leak_count: 0,
    metadata_card_line_shape_leak_count: 0,
    raw_function_scaffold_leak_count: 0,
    tool_trace_blocked_scaffold_leak_count: 0,
    runtime_capability_template_leak_count: 0,
    workflow_retry_template_leak_count: 0,
    workflow_unexpected_state_template_leak_count: 0,
    source_receipt_scaffold_leak_count: 0,
    tool_routing_diagnosis_template_leak_count: 0,
    ingress_policy_preamble_template_leak_count: 0,
    tool_block_input_result_scaffold_leak_count: 0,
    workflow_loop_leak_contract_violation_count: 0,
    file_tool_route_misdirection_contract_violation_count: 0,
  };

  for (const row of cases) {
    const raw = cleanText(row.raw_input || '', 20_000);
    const sanitized = sanitizeWebBody(raw).toLowerCase();
    const expected = (Array.isArray(row.expected_contains) ? row.expected_contains : []).map((value) =>
      cleanText(value || '', 400).toLowerCase(),
    );
    const forbidden = Array.from(
      new Set(
        [...DEFAULT_FORBIDDEN_TOKENS, ...(Array.isArray(row.forbidden_contains) ? row.forbidden_contains : [])]
          .map((value) => cleanText(value || '', 200).toLowerCase())
          .filter(Boolean),
      ),
    );
    const inferred = inferMode(raw);
    const modeMatch = inferred === row.mode;
    const expectedMissing = expected.filter((value) => !sanitized.includes(value));
    const forbiddenPresent = forbidden.filter((value) => sanitized.includes(value));
    const benchmarkInstructionLeak =
      forbiddenPresent.some(
        (value) =>
          value.includes('ops:benchmark:')
          || value.includes('next actions:')
          || value.includes('targeted tool call'),
      )
      || textHasAnyPattern(sanitized, [
        /npm\s+run\s+-s\s+ops:benchmark:[a-z0-9-]+/i,
        /ops:benchmark:(refresh|sanity|public-audit|repro)/i,
        /next actions:\s*1\)[\s\S]*targeted tool call[\s\S]*concise answer from current context/i,
      ]);
    const benchmarkIntentOverlapHygieneViolation =
      (
        forbiddenPresent.some(
          (value) =>
            value.includes('ops:benchmark:')
            || value.includes('benchmark matrix')
            || value.includes('benchmark:'),
        )
        || textHasAnyPattern(sanitized, [
          /ops:benchmark:(refresh|sanity|public-audit|repro)/i,
          /npm\s+run\s+-s\s+ops:benchmark:[a-z0-9-]+/i,
          /\bbenchmark matrix\b/i,
        ])
      )
      && (
        forbiddenPresent.some(
          (value) =>
            value.includes('next actions:')
            || value.includes('targeted tool call')
            || value.includes('concise answer from current context'),
        )
        || textHasAnyPattern(sanitized, [
          /next actions:\s*1\)/i,
          /targeted tool call/i,
          /concise answer from current context/i,
        ])
      );
    const metadataCardLeak =
      forbiddenPresent.some(
        (value) =>
          value === 'title:'
          || value === 'excerpt:'
          || value === 'originalurl:'
          || value === 'featuredcontent:',
      )
      || textHasAnyPattern(sanitized, [
        /title:\s*\S+/i,
        /excerpt:\s*\S+/i,
        /originalurl:\s*\S+/i,
        /featuredcontent:\s*\S+/i,
      ]);
    const metadataCardLineShapeLeak = [
      /title:\s*\S+/i,
      /excerpt:\s*\S+/i,
      /originalurl:\s*\S+/i,
      /featuredcontent:\s*\S+/i,
      /publisheddatetime:\s*\S+/i,
      /provider:\s*\S*/i,
      /images?:\s*\S*/i,
    ].filter((pattern) => pattern.test(sanitized)).length >= 2;
    const rawFunctionScaffoldLeak =
      forbiddenPresent.some((value) => value.includes('<function=') || value.includes('</function>'))
      || textHasAnyPattern(sanitized, [/<function\s*=[^>\n]*>/i, /<\/function>/i, /\bfunction\s*=\s*[a-z_]/i]);
    const toolTraceBlockedScaffoldLeak = forbiddenPresent.some(
      (value) =>
        value.includes('tool trace complete')
        || value.includes('done · 1 blocked')
        || value.includes('file list blocked')
        || value.includes('ingress delivery policy')
        || value.includes('lease_denied:client_ingress_domain_boundary')
        || value.includes('policy gate, not a web-provider outage')
        || value.includes('file_list was blocked by ingress delivery policy in this runtime lane'),
    );
    const runtimeCapabilityTemplateLeak = forbiddenPresent.some(
      (value) =>
        value.includes('i can access runtime telemetry')
        || value.includes('approved command surfaces in this session')
        || value.includes('tell me what you want me to check and i will run it now'),
    );
    const workflowRetryTemplateLeak = forbiddenPresent.some(
      (value) =>
        value.includes('please retry so i can rerun the chain cleanly')
        || value.includes('ask me to continue and i will synthesize from the recorded workflow state')
        || value.includes('ask me to continue and i will synthesize from recorded workflow state'),
    );
    const workflowUnexpectedStateTemplateLeak = forbiddenPresent.some(
      (value) =>
        value.includes('i completed the workflow gate, but the final workflow state was unexpected')
        || value.includes('final workflow state was unexpected')
        || value.includes('i completed the run, but the final reply did not render')
        || value.includes('final reply did not render'),
    );
    const sourceReceiptScaffoldLeak = forbiddenPresent.some(
      (value) =>
        value.includes('[source:local_context]')
        || value.includes('[source:tool_receipt:'),
    );
    const toolRoutingDiagnosisTemplateLeak = forbiddenPresent.some(
      (value) =>
        value.includes('the system is still automatically triggering tool calls without my conscious selection')
        || value.includes('fundamental misclassification error')
        || value.includes('tool routing mechanism is clearly malfunctioning')
        || value.includes('the first gate ("workflow_route") is still classifying this as an "info" route rather than a "task" route')
        || value.includes('the first gate ("task_or_info_route") is still classifying this as an "info" route rather than a "task" route')
        || value.includes('the first gate ("workflow_route") is a binary classification')
        || value.includes('the first gate ("task_or_info_route") is a binary classification')
        || value.includes("it's not a true/false decision i control")
        || value.includes('automated classification based on semantic analysis')
        || value.includes('otherwise, it defaults to info')
        || value.includes('the system needs explicit tool-related phrasing to trigger the task classification path')
        || value.includes('it\'s still seeing this as a conversational exchange rather than a tool operation request')
        || value.includes('it is still seeing this as a conversational exchange rather than a tool operation request')
        || value.includes('[source:workflow_gate]')
        || value.includes(
          'requires recalibration to properly distinguish between internal system operations and external data retrieval requests',
        ),
    );
    const ingressPolicyPreambleTemplateLeak = forbiddenPresent.some(
      (value) =>
        value.includes('the file list step was blocked before i could finish the answer')
        || value.includes('policy gate, not a web-provider outage'),
    );
    const toolBlockInputResultScaffoldLeak = forbiddenPresent.some(
      (value) =>
        value.includes('tool trace complete1 done · 1 blocked')
        || value.includes('result `file_list` was blocked by ingress delivery policy in this runtime lane')
        || value.includes('result file_list was blocked by ingress delivery policy in this runtime lane'),
    );
    const sourceIssues = sourceAttributionIssues(Array.isArray(row.sources) ? row.sources : []);
    const sourceOk = sourceIssues.length === 0;
    const extractionOk = modeMatch && expectedMissing.length === 0 && forbiddenPresent.length === 0;
    const ok = extractionOk && sourceOk && inferred !== 'ambiguous' && inferred !== 'unknown';

    if (!modeMatch || inferred === 'unknown') counters.parse_drift_count += 1;
    if (inferred === 'ambiguous') counters.ambiguous_fetch_search_parse_count += 1;
    if (forbiddenPresent.length > 0) counters.chrome_leak_count += 1;
    if (benchmarkInstructionLeak) counters.benchmark_instruction_leak_count += 1;
    if (benchmarkIntentOverlapHygieneViolation) {
      counters.benchmark_intent_overlap_hygiene_violation_count += 1;
    }
    if (metadataCardLeak) counters.metadata_card_leak_count += 1;
    if (metadataCardLineShapeLeak) counters.metadata_card_line_shape_leak_count += 1;
    if (rawFunctionScaffoldLeak) counters.raw_function_scaffold_leak_count += 1;
    if (toolTraceBlockedScaffoldLeak) counters.tool_trace_blocked_scaffold_leak_count += 1;
    if (runtimeCapabilityTemplateLeak) counters.runtime_capability_template_leak_count += 1;
    if (workflowRetryTemplateLeak) counters.workflow_retry_template_leak_count += 1;
    if (workflowUnexpectedStateTemplateLeak) {
      counters.workflow_unexpected_state_template_leak_count += 1;
    }
    if (sourceReceiptScaffoldLeak) counters.source_receipt_scaffold_leak_count += 1;
    if (toolRoutingDiagnosisTemplateLeak) {
      counters.tool_routing_diagnosis_template_leak_count += 1;
    }
    if (ingressPolicyPreambleTemplateLeak) {
      counters.ingress_policy_preamble_template_leak_count += 1;
    }
    if (toolBlockInputResultScaffoldLeak) {
      counters.tool_block_input_result_scaffold_leak_count += 1;
    }
    if (!sourceOk) counters.source_attribution_failure_count += 1;
    if (!extractionOk) counters.extraction_fidelity_fail_count += 1;
    if (ok) counters.passed_cases += 1;
    else counters.failed_cases += 1;

    caseResults.push({
      id: cleanText(row.id || '', 120),
      mode: row.mode,
      inferred_mode: inferred,
      mode_match: modeMatch,
      expected_missing: expectedMissing,
      forbidden_present: forbiddenPresent,
      source_attribution_ok: sourceOk,
      source_attribution_issues: sourceIssues,
      extraction_ok: extractionOk,
      ok,
    });
  }

  const soakParseError = cleanText(soak?.taxonomy?.parse_error || '', 120);
  if (soak && soakParseError) {
    counters.parse_drift_count += 1;
    failures.push({ id: 'web_tooling_context_soak_taxonomy_parse_error', detail: soakParseError });
  }
  const providerCacheContract = evaluateProviderCacheContracts(soak);
  const soakTaxonomyContract = evaluateSoakTaxonomyContracts(soak);
  counters.provider_failure_contract_violation_count +=
    providerCacheContract.provider_failure_contract_violation_count;
  counters.cache_skip_reason_missing_count += providerCacheContract.cache_skip_reason_missing_count;
  counters.cache_write_gate_violation_count += providerCacheContract.cache_write_gate_violation_count;
  counters.cache_stale_age_missing_count += providerCacheContract.cache_stale_age_missing_count;
  counters.workflow_loop_leak_contract_violation_count +=
    soakTaxonomyContract.workflow_loop_leak_contract_violation_count;
  counters.file_tool_route_misdirection_contract_violation_count +=
    soakTaxonomyContract.file_tool_route_misdirection_contract_violation_count;
  if (providerCacheContract.provider_failure_contract_violation_count > 0) {
    failures.push({
      id: 'web_provider_failure_contract_violation',
      detail: cleanText(
        `modes=${providerCacheContract.provider_failure_modes.join('|') || 'none'}; violations=${providerCacheContract.provider_failure_contract_violation_count}`,
        280,
      ),
    });
  }
  if (providerCacheContract.cache_skip_reason_missing_count > 0) {
    failures.push({
      id: 'web_cache_skip_reason_missing',
      detail: 'cache skipped without explicit reason',
    });
  }
  if (providerCacheContract.cache_write_gate_violation_count > 0) {
    failures.push({
      id: 'web_cache_write_gate_violation',
      detail: 'cache write attempted/allowed during provider failure mode',
    });
  }
  if (providerCacheContract.cache_stale_age_missing_count > 0) {
    failures.push({
      id: 'web_cache_stale_age_missing',
      detail: 'cache stale-age metadata missing when cache/provider-failure contract requires it',
    });
  }
  if (soakTaxonomyContract.workflow_loop_leak_contract_violation_count > 0) {
    failures.push({
      id: 'web_workflow_loop_leak_contract_violation',
      detail: cleanText(
        `workflow_loop_leak_count=${soakTaxonomyContract.workflow_loop_leak_count};contract_failures=${soakTaxonomyContract.failures.join('|') || 'none'}`,
        280,
      ),
    });
  }
  if (soakTaxonomyContract.file_tool_route_misdirection_contract_violation_count > 0) {
    failures.push({
      id: 'web_file_tool_route_misdirection_contract_violation',
      detail: cleanText(
        `file_tool_route_misdirection_count=${soakTaxonomyContract.file_tool_route_misdirection_count};contract_failures=${soakTaxonomyContract.failures.join('|') || 'none'}`,
        280,
      ),
    });
  }

  const thresholds = {
    extraction_fidelity_fail_max: 0,
    parse_drift_max: 0,
    ambiguous_fetch_search_parse_max: 0,
    chrome_leak_max: 0,
    source_attribution_failure_max: 0,
    provider_failure_contract_violation_max: 0,
    cache_skip_reason_missing_max: 0,
    cache_write_gate_violation_max: 0,
    cache_stale_age_missing_max: 0,
    benchmark_instruction_leak_max: 0,
    benchmark_intent_overlap_hygiene_violation_max: 0,
    metadata_card_leak_max: 0,
    metadata_card_line_shape_leak_max: 0,
    raw_function_scaffold_leak_max: 0,
    tool_trace_blocked_scaffold_leak_max: 0,
    runtime_capability_template_leak_max: 0,
    workflow_retry_template_leak_max: 0,
    workflow_unexpected_state_template_leak_max: 0,
    source_receipt_scaffold_leak_max: 0,
    tool_routing_diagnosis_template_leak_max: 0,
    ingress_policy_preamble_template_leak_max: 0,
    tool_block_input_result_scaffold_leak_max: 0,
    workflow_loop_leak_contract_violation_max: 0,
    file_tool_route_misdirection_contract_violation_max: 0,
  };

  const gateChecks = [
    {
      id: 'web_extraction_fidelity',
      ok: counters.extraction_fidelity_fail_count <= thresholds.extraction_fidelity_fail_max,
      detail: `value=${counters.extraction_fidelity_fail_count} max=${thresholds.extraction_fidelity_fail_max}`,
    },
    {
      id: 'web_parse_drift',
      ok: counters.parse_drift_count <= thresholds.parse_drift_max,
      detail: `value=${counters.parse_drift_count} max=${thresholds.parse_drift_max}`,
    },
    {
      id: 'web_ambiguous_fetch_search_parse',
      ok: counters.ambiguous_fetch_search_parse_count <= thresholds.ambiguous_fetch_search_parse_max,
      detail: `value=${counters.ambiguous_fetch_search_parse_count} max=${thresholds.ambiguous_fetch_search_parse_max}`,
    },
    {
      id: 'web_chrome_leakage',
      ok: counters.chrome_leak_count <= thresholds.chrome_leak_max,
      detail: `value=${counters.chrome_leak_count} max=${thresholds.chrome_leak_max}`,
    },
    {
      id: 'web_source_attribution_integrity',
      ok: counters.source_attribution_failure_count <= thresholds.source_attribution_failure_max,
      detail: `value=${counters.source_attribution_failure_count} max=${thresholds.source_attribution_failure_max}`,
    },
    {
      id: 'web_provider_failure_contract',
      ok:
        counters.provider_failure_contract_violation_count
        <= thresholds.provider_failure_contract_violation_max,
      detail: `value=${counters.provider_failure_contract_violation_count} max=${thresholds.provider_failure_contract_violation_max}`,
    },
    {
      id: 'web_cache_skip_reason_contract',
      ok: counters.cache_skip_reason_missing_count <= thresholds.cache_skip_reason_missing_max,
      detail: `value=${counters.cache_skip_reason_missing_count} max=${thresholds.cache_skip_reason_missing_max}`,
    },
    {
      id: 'web_cache_write_fail_closed',
      ok: counters.cache_write_gate_violation_count <= thresholds.cache_write_gate_violation_max,
      detail: `value=${counters.cache_write_gate_violation_count} max=${thresholds.cache_write_gate_violation_max}`,
    },
    {
      id: 'web_cache_stale_age_contract',
      ok: counters.cache_stale_age_missing_count <= thresholds.cache_stale_age_missing_max,
      detail: `value=${counters.cache_stale_age_missing_count} max=${thresholds.cache_stale_age_missing_max}`,
    },
    {
      id: 'web_benchmark_instruction_snippet_suppression',
      ok: counters.benchmark_instruction_leak_count <= thresholds.benchmark_instruction_leak_max,
      detail: `value=${counters.benchmark_instruction_leak_count} max=${thresholds.benchmark_instruction_leak_max}`,
    },
    {
      id: 'web_benchmark_intent_overlap_hygiene',
      ok:
        counters.benchmark_intent_overlap_hygiene_violation_count
        <= thresholds.benchmark_intent_overlap_hygiene_violation_max,
      detail: `value=${counters.benchmark_intent_overlap_hygiene_violation_count} max=${thresholds.benchmark_intent_overlap_hygiene_violation_max}`,
    },
    {
      id: 'web_metadata_card_scaffold_suppression',
      ok: counters.metadata_card_leak_count <= thresholds.metadata_card_leak_max,
      detail: `value=${counters.metadata_card_leak_count} max=${thresholds.metadata_card_leak_max}`,
    },
    {
      id: 'web_metadata_card_line_shape_suppression',
      ok:
        counters.metadata_card_line_shape_leak_count
        <= thresholds.metadata_card_line_shape_leak_max,
      detail: `value=${counters.metadata_card_line_shape_leak_count} max=${thresholds.metadata_card_line_shape_leak_max}`,
    },
    {
      id: 'web_raw_function_scaffold_suppression',
      ok:
        counters.raw_function_scaffold_leak_count
        <= thresholds.raw_function_scaffold_leak_max,
      detail: `value=${counters.raw_function_scaffold_leak_count} max=${thresholds.raw_function_scaffold_leak_max}`,
    },
    {
      id: 'web_tool_trace_blocked_scaffold_suppression',
      ok:
        counters.tool_trace_blocked_scaffold_leak_count
        <= thresholds.tool_trace_blocked_scaffold_leak_max,
      detail: `value=${counters.tool_trace_blocked_scaffold_leak_count} max=${thresholds.tool_trace_blocked_scaffold_leak_max}`,
    },
    {
      id: 'web_runtime_capability_template_suppression',
      ok:
        counters.runtime_capability_template_leak_count
        <= thresholds.runtime_capability_template_leak_max,
      detail: `value=${counters.runtime_capability_template_leak_count} max=${thresholds.runtime_capability_template_leak_max}`,
    },
    {
      id: 'web_workflow_retry_template_suppression',
      ok:
        counters.workflow_retry_template_leak_count
        <= thresholds.workflow_retry_template_leak_max,
      detail: `value=${counters.workflow_retry_template_leak_count} max=${thresholds.workflow_retry_template_leak_max}`,
    },
    {
      id: 'web_workflow_unexpected_state_template_suppression',
      ok:
        counters.workflow_unexpected_state_template_leak_count
        <= thresholds.workflow_unexpected_state_template_leak_max,
      detail: `value=${counters.workflow_unexpected_state_template_leak_count} max=${thresholds.workflow_unexpected_state_template_leak_max}`,
    },
    {
      id: 'web_source_receipt_scaffold_suppression',
      ok:
        counters.source_receipt_scaffold_leak_count
        <= thresholds.source_receipt_scaffold_leak_max,
      detail: `value=${counters.source_receipt_scaffold_leak_count} max=${thresholds.source_receipt_scaffold_leak_max}`,
    },
    {
      id: 'web_tool_routing_diagnosis_template_suppression',
      ok:
        counters.tool_routing_diagnosis_template_leak_count
        <= thresholds.tool_routing_diagnosis_template_leak_max,
      detail: `value=${counters.tool_routing_diagnosis_template_leak_count} max=${thresholds.tool_routing_diagnosis_template_leak_max}`,
    },
    {
      id: 'web_ingress_policy_preamble_template_suppression',
      ok:
        counters.ingress_policy_preamble_template_leak_count
        <= thresholds.ingress_policy_preamble_template_leak_max,
      detail: `value=${counters.ingress_policy_preamble_template_leak_count} max=${thresholds.ingress_policy_preamble_template_leak_max}`,
    },
    {
      id: 'web_tool_block_input_result_scaffold_suppression',
      ok:
        counters.tool_block_input_result_scaffold_leak_count
        <= thresholds.tool_block_input_result_scaffold_leak_max,
      detail: `value=${counters.tool_block_input_result_scaffold_leak_count} max=${thresholds.tool_block_input_result_scaffold_leak_max}`,
    },
    {
      id: 'web_workflow_loop_leak_contract',
      ok:
        counters.workflow_loop_leak_contract_violation_count
        <= thresholds.workflow_loop_leak_contract_violation_max,
      detail: `value=${counters.workflow_loop_leak_contract_violation_count} max=${thresholds.workflow_loop_leak_contract_violation_max}`,
    },
    {
      id: 'web_file_tool_route_misdirection_contract',
      ok:
        counters.file_tool_route_misdirection_contract_violation_count
        <= thresholds.file_tool_route_misdirection_contract_violation_max,
      detail: `value=${counters.file_tool_route_misdirection_contract_violation_count} max=${thresholds.file_tool_route_misdirection_contract_violation_max}`,
    },
  ];

  for (const row of caseResults.filter((caseRow) => !caseRow.ok)) {
    failures.push({
      id: `case:${row.id}`,
      detail: cleanText(
        `inferred=${row.inferred_mode}; expected_missing=${row.expected_missing.join('|') || 'none'}; forbidden_present=${row.forbidden_present.join('|') || 'none'}; source_issues=${row.source_attribution_issues.join('|') || 'none'}`,
        600,
      ),
    });
  }

  const allChecksPass = failures.length === 0 && gateChecks.every((row) => row.ok);
  const report = {
    type: 'web_tooling_reliability',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    strict_mode: args.strict,
    ok: allChecksPass,
    fixture_path: args.fixturePath,
    soak_path: args.soakPath,
    soak_context: {
      present: !!soak,
      ok: soak?.ok === true,
      taxonomy_parse_error: soakParseError || '',
    },
    provider_cache_contract: providerCacheContract,
    soak_taxonomy_contract: soakTaxonomyContract,
    counters,
    thresholds,
    gate_checks: gateChecks,
    failed_case_ids: caseResults.filter((row) => !row.ok).map((row) => row.id),
    case_results: caseResults,
    failures,
  };

  const outAbs = path.resolve(root, args.outPath);
  const latestAbs = path.resolve(root, args.outLatestPath);
  const stateAbs = path.resolve(root, args.statePath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  writeJson(outAbs, report);
  writeJson(latestAbs, report);
  writeJson(stateAbs, report);
  writeMarkdown(markdownAbs, renderMarkdown(report));

  const exitCode = args.strict ? (allChecksPass ? 0 : 1) : 0;
  emitStructuredResult(
    {
      ok: allChecksPass,
      report_path: args.outPath,
      latest_path: args.outLatestPath,
      markdown_path: args.markdownPath,
      failures: failures.length,
    },
    { outPath: args.outPath },
  );
  return exitCode;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exit(run());
}
