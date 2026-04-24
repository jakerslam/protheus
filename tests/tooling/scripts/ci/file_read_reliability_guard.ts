#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/file_read_reliability_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/FILE_READ_RELIABILITY_GUARD_CURRENT.md';
const DEFAULT_OUT_ALIAS = 'artifacts/file_read_reliability_guard_latest.json';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

type GuardCheck = {
  id: string;
  ok: boolean;
  detail: string;
};

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 500),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 500),
  };
}

function readSource(relativePath: string): string {
  return fs.readFileSync(path.resolve(ROOT, relativePath), 'utf8');
}

function includesAll(source: string, tokens: string[]): boolean {
  return tokens.every((token) => source.includes(token));
}

function isCanonicalRelativePath(value: string, requiredPrefix = ''): boolean {
  const normalized = cleanText(value || '', 500);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (normalized.endsWith('/')) return false;
  if (requiredPrefix && !normalized.startsWith(requiredPrefix)) return false;
  return true;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# FILE READ RELIABILITY GUARD');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- ok: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- check_count: ${payload.summary.check_count}`);
  lines.push(`- check_pass_count: ${payload.summary.check_pass_count}`);
  lines.push(`- check_fail_count: ${payload.summary.check_fail_count}`);
  lines.push('');
  lines.push('## Checks');
  for (const row of payload.checks || []) {
    lines.push(`- [${row.ok ? 'x' : ' '}] ${row.id}: ${row.detail}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = parseArgs(argv);

  const parserPath = 'surface/orchestration/src/ingress/parser.rs';
  const classifierPath = 'surface/orchestration/src/ingress/classifier.rs';
  const fileReadRoutesPath =
    'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/080-file-read-routes.rs';
  const fileReadContractTestsPath =
    'core/layer0/ops/src/dashboard_compat_api_parts/config_payload_tests_parts/030-memory-kv-http-routes-round-trip-and-feed-context.rs';

  const parserSource = readSource(parserPath);
  const classifierSource = readSource(classifierPath);
  const routesSource = readSource(fileReadRoutesPath);
  const testsSource = readSource(fileReadContractTestsPath);

  const checks: GuardCheck[] = [];

  checks.push({
    id: 'file_read_reliability_guard_out_json_path_canonical',
    ok: isCanonicalRelativePath(args.outJson, 'core/local/artifacts/'),
    detail: 'out-json path must be canonical and rooted under core/local/artifacts/',
  });

  checks.push({
    id: 'file_read_reliability_guard_out_json_current_suffix_contract',
    ok: cleanText(args.outJson, 500).endsWith('_current.json'),
    detail: 'out-json path must end with _current.json',
  });

  checks.push({
    id: 'file_read_reliability_guard_out_markdown_path_canonical',
    ok: isCanonicalRelativePath(args.outMarkdown, 'local/workspace/reports/'),
    detail: 'out-markdown path must be canonical and rooted under local/workspace/reports/',
  });

  checks.push({
    id: 'file_read_reliability_guard_out_markdown_contract',
    ok: cleanText(args.outMarkdown, 500) === DEFAULT_OUT_MARKDOWN,
    detail: `out-markdown path must match canonical contract ${DEFAULT_OUT_MARKDOWN}`,
  });

  checks.push({
    id: 'file_read_reliability_guard_out_alias_path_canonical',
    ok: isCanonicalRelativePath(DEFAULT_OUT_ALIAS, 'artifacts/'),
    detail: 'latest alias artifact path must be canonical and rooted under artifacts/',
  });

  checks.push({
    id: 'file_read_reliability_guard_out_alias_latest_suffix_contract',
    ok: DEFAULT_OUT_ALIAS.endsWith('_latest.json'),
    detail: 'latest alias artifact path must end with _latest.json',
  });

  checks.push({
    id: 'file_read_reliability_guard_output_paths_distinct',
    ok: new Set([args.outJson, args.outMarkdown, DEFAULT_OUT_ALIAS]).size === 3,
    detail: 'out-json, out-markdown, and latest alias targets must be distinct paths',
  });

  checks.push({
    id: 'file_read_reliability_guard_source_paths_canonical',
    ok:
      isCanonicalRelativePath(parserPath, 'surface/orchestration/src/ingress/')
      && isCanonicalRelativePath(classifierPath, 'surface/orchestration/src/ingress/')
      && isCanonicalRelativePath(fileReadRoutesPath, 'core/layer0/ops/src/')
      && isCanonicalRelativePath(fileReadContractTestsPath, 'core/layer0/ops/src/'),
    detail: 'parser/classifier/routes/tests source paths must remain canonical and rooted in expected subsystems',
  });

  checks.push({
    id: 'file_read_reliability_parser_local_intent_vocabulary_hardened',
    ok: includesAll(parserSource, [
      'payload_local_workspace_intent',
      'file tooling',
      'workspace tooling',
      'local file tooling',
      'working tree',
      'source tree',
      'cwd',
      'pwd',
    ]),
    detail:
      'parser local-workspace intent classifier must include explicit local file-tooling/workspace vocabulary and cwd/pwd tokens',
  });

  checks.push({
    id: 'file_read_reliability_parser_repo_and_cwd_alias_signal_contract',
    ok: includesAll(parserSource, [
      '"repo_path"',
      '"repo_root"',
      '"repository_path"',
      '"repository_root"',
      '"cwd_path"',
      '"pwd_path"',
      '"current_working_directory"',
      '"present_working_directory"',
    ]),
    detail:
      'parser workspace-signal extraction must include repo/repository + cwd/pwd alias keys for local file intent',
  });

  checks.push({
    id: 'file_read_reliability_parser_directory_alias_signal_contract',
    ok: includesAll(parserSource, [
      '"workspace_dir"',
      '"repo_dir"',
      '"repository_dir"',
      '"working_dir"',
      '"current_dir"',
      '"directory"',
      '"directories"',
      '"folder"',
      '"folders"',
    ]),
    detail:
      'parser workspace-signal extraction must include directory/folder alias keys for local file tooling routes',
  });

  checks.push({
    id: 'file_read_reliability_parser_tool_hint_alias_contract',
    ok: includesAll(parserSource, [
      '"file_read" | "read_file" => "workspace_read"',
      '"file_search" | "file_list" | "workspace_analyze" => "workspace_search"',
      '"web_lookup" => "web_search"',
    ]),
    detail:
      'parser tool-hint alias normalization must retain canonical mappings for workspace and web route hints',
  });

  checks.push({
    id: 'file_read_reliability_parser_local_workspace_intent_working_dir_terms_contract',
    ok: includesAll(parserSource, [
      'payload_local_workspace_intent',
      'current working directory',
      'present working directory',
      'working tree',
      'source tree',
    ]),
    detail:
      'parser local-workspace intent classifier must include working-directory and tree locality terminology',
  });

  checks.push({
    id: 'file_read_reliability_parser_workspace_alias_keys',
    ok: includesAll(parserSource, [
      '"workspace_root"',
      '"repo_root"',
      '"working_directory"',
      '"directory_path"',
      '"workspace_roots"',
      '"repo_roots"',
      '"directory_paths"',
      '"cwd_path"',
      '"pwd_path"',
    ]),
    detail:
      'parser target extraction must normalize workspace alias keys for object payloads and path-key variants',
  });

  checks.push({
    id: 'file_read_reliability_parser_object_payload_target_normalization',
    ok: includesAll(parserSource, [
      'extract_nested_target_scalar',
      '"value"',
      '"path"',
      '"workspace_path"',
      '"directory"',
      '"folder"',
    ]),
    detail:
      'parser must normalize object-shaped target payload values into canonical workspace descriptors',
  });

  checks.push({
    id: 'file_read_reliability_parser_local_vs_web_hint_guard',
    ok: includesAll(parserSource, [
      'payload_local_workspace_intent',
      'payload_web_intent',
      'hints.retain(|hint| hint != "web_search" && hint != "web_fetch")',
    ]),
    detail:
      'parser must retain local file-routing priority by stripping web hints when workspace-local intent is present',
  });

  checks.push({
    id: 'file_read_reliability_classifier_typed_probe_missing_diagnostics_contract',
    ok: includesAll(classifierSource, [
      'typed_probe_contract_missing:capability.{capability_key}',
      'typed_probe_contract_missing:field.{capability_key}.{field}',
      'typed_probe_contract_missing_total:{}',
      'typed_probe_contract_complete',
      'typed_probe_contract_expected:{}',
    ]),
    detail:
      'classifier typed-probe diagnostics must emit capability/field/expected/total contracts for fail-closed route triage',
  });

  checks.push({
    id: 'file_read_reliability_classifier_workspace_capability_mapping',
    ok: includesAll(classifierSource, [
      '"workspace_read" => Capability::WorkspaceRead',
      '"workspace_search" => Capability::WorkspaceSearch',
      'typed_probe_contract_missing:capability.{capability_key}',
    ]),
    detail:
      'classifier must keep explicit workspace_read/workspace_search capability mappings with capability-specific missing diagnostics',
  });

  checks.push({
    id: 'file_read_reliability_routes_follow_up_mode_contract',
    ok: includesAll(routesSource, [
      'fn file_read_routing_follow_up_mode(',
      '"auto_retry"',
      '"user_input"',
      'return "none";',
    ]),
    detail:
      'file-read routes must expose deterministic follow_up_mode transitions across none/auto_retry/user_input',
  });

  checks.push({
    id: 'file_read_reliability_routes_follow_up_task_contract',
    ok: includesAll(routesSource, [
      'fn file_read_routing_follow_up_task(',
      '"policy_repair"',
      '"path_repair"',
      '"binary_opt_in"',
      '"content_expansion"',
      '"mixed_follow_up"',
      '"partial_replay"',
    ]),
    detail:
      'file-read routes must expose deterministic follow_up_task contracts for policy/path/binary/content/mixed/partial recoveries',
  });

  checks.push({
    id: 'file_read_reliability_routes_recovery_class_contract',
    ok: includesAll(routesSource, [
      'fn file_read_recovery_class(action: &str) -> &\'static str {',
      '"path_repair"',
      '"binary_opt_in"',
      '"content_expansion"',
      '"policy_repair"',
      '"none"',
    ]),
    detail:
      'file-read recovery classification must preserve canonical class taxonomy for route remediation workflows',
  });

  checks.push({
    id: 'file_read_reliability_routes_follow_up_operator_input_contract',
    ok: includesAll(routesSource, [
      'fn file_read_recovery_requires_user_input(action: &str) -> bool {',
      'fn file_read_routing_follow_up_can_auto_retry(',
      'recovery_requires_user_input',
    ]),
    detail:
      'file-read routing must preserve user-input gating and auto-retry suppression contracts for interactive follow-up recovery',
  });

  checks.push({
    id: 'file_read_reliability_routes_grouped_binary_bucket_contract',
    ok: includesAll(routesSource, [
      '"groups": {',
      '"binary_opt_in_blocked": grouped_binary_opt_in_blocked',
      '"group_binary_opt_in_blocked": grouped_binary_opt_in_blocked.len()',
    ]),
    detail:
      'file_read_many payload must expose groups.binary_opt_in_blocked and aligned counts.group_binary_opt_in_blocked contracts',
  });

  checks.push({
    id: 'file_read_reliability_routes_follow_up_user_text_contract',
    ok: includesAll(routesSource, [
      '"follow_up_requires_user_text_input": routing_follow_up_requires_user_text_input',
      '"follow_up_user_text_input_kind": routing_follow_up_user_text_input_kind',
    ]),
    detail:
      'file_read_many routing contract must carry follow-up user-text input requirement and input-kind telemetry',
  });

  checks.push({
    id: 'file_read_reliability_tests_operator_input_helper_contract',
    ok: includesAll(testsSource, [
      'fn assert_follow_up_requires_operator_input_contract(payload: &Value) {',
      '.pointer("/routing/requires_follow_up")',
      '.pointer("/routing/follow_up_owner")',
      '.pointer("/routing/follow_up_requires_operator_input")',
    ]),
    detail:
      'file-read contract tests must preserve helper-level parity assertions for operator-input routing contracts',
  });

  checks.push({
    id: 'file_read_reliability_tests_follow_up_payload_contract',
    ok: includesAll(testsSource, [
      '.pointer("/routing/follow_up_mode")',
      '.pointer("/routing/follow_up_task")',
      '.pointer("/routing/follow_up_priority")',
      '.pointer("/routing/follow_up_blocking")',
      '.pointer("/routing/follow_up_bucket")',
      '.pointer("/routing/follow_up_sla_seconds")',
    ]),
    detail:
      'file-read contract tests must assert follow-up mode/task/priority/blocking/bucket/sla contracts for reliable routing recovery',
  });

  checks.push({
    id: 'file_read_reliability_tests_assert_follow_up_and_binary_group_contracts',
    ok: includesAll(testsSource, [
      '.pointer("/routing/follow_up_requires_user_text_input")',
      '.pointer("/routing/binary_opt_in_blocked_count")',
      '.pointer("/counts/group_binary_opt_in_blocked")',
      '.pointer("/groups/binary_opt_in_blocked")',
    ]),
    detail:
      'contract tests must assert follow-up user-text + grouped binary bucket counters to keep file-read reliability fail-closed',
  });

  const checkPassCount = checks.filter((row) => row.ok).length;
  const checkCount = checks.length;
  const checkFailCount = checkCount - checkPassCount;
  const ok = checkFailCount === 0;

  const payload = {
    ok,
    strict: args.strict,
    type: 'file_read_reliability_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      check_count: checkCount,
      check_pass_count: checkPassCount,
      check_fail_count: checkFailCount,
    },
    checks,
  };

  writeJsonArtifact(DEFAULT_OUT_ALIAS, payload);
  writeTextArtifact(args.outMarkdown, toMarkdown(payload));

  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
