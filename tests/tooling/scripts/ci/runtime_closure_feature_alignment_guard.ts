#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type ParsedTable = {
  headers: string[];
  rows: string[][];
};

function isCanonicalBucketToken(value: string): boolean {
  return /^[a-z0-9_]+$/.test(cleanText(value, 120));
}

function isCanonicalOwnerToken(value: string): boolean {
  return /^[a-z0-9_-]+$/.test(cleanText(value, 120));
}

function isCanonicalGateToken(value: string): boolean {
  return /^ops:[a-z0-9:-]+$/.test(cleanText(value, 200));
}

function isCanonicalArtifactToken(value: string): boolean {
  return /^core\/local\/artifacts\/[a-z0-9_./-]+_current\.json$/.test(cleanText(value, 260));
}

function isIsoUtcTimestamp(value: string): boolean {
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(cleanText(value, 80));
}

function isCanonicalRepoRelativePathToken(
  value: string,
  requiredPrefix = '',
  requiredSuffix = '',
): boolean {
  const token = cleanText(value, 600);
  if (!token) return false;
  if (path.isAbsolute(token)) return false;
  if (token.includes('\\')) return false;
  if (token.includes('..')) return false;
  if (token.includes('//')) return false;
  if (/\s/.test(token)) return false;
  if (requiredPrefix && !token.startsWith(requiredPrefix)) return false;
  if (requiredSuffix && !token.endsWith(requiredSuffix)) return false;
  return true;
}

function placeholderToken(value: string): boolean {
  return /^(tbd|todo|n\/a|-+|pending|unknown)$/i.test(cleanText(value, 120));
}

function duplicateValues(values: string[]): string[] {
  const normalized = values.map((value) => cleanText(value, 200)).filter(Boolean);
  return normalized.filter((value, index, arr) => arr.indexOf(value) !== index);
}

function casefoldDuplicateValues(values: string[]): string[] {
  const normalized = values
    .map((value) => cleanText(value, 200).toLowerCase())
    .filter(Boolean);
  return normalized.filter((value, index, arr) => arr.indexOf(value) !== index);
}

function parseDelimitedTokens(raw: string): string[] {
  const cleaned = cleanText(raw, 2000);
  if (!cleaned) return [];
  return cleaned
    .split(/[,\n;]/)
    .map((token) => cleanText(token, 260))
    .filter(Boolean);
}

function uniqueTokens(values: string[]): string[] {
  const out: string[] = [];
  for (const value of values) {
    if (!out.includes(value)) out.push(value);
  }
  return out;
}

function extractValidationAnchors(raw: string): string[] {
  const text = cleanText(raw, 4000);
  if (!text) return [];
  const gateAnchors = text.match(/ops:[a-z0-9:-]+/g) || [];
  const artifactAnchors =
    text.match(/core\/local\/artifacts\/[a-z0-9_./-]+_current\.json/g) || [];
  return uniqueTokens([...gateAnchors, ...artifactAnchors]);
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_closure_feature_alignment_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') ||
        'local/workspace/reports/RUNTIME_CLOSURE_FEATURE_ALIGNMENT_GUARD_CURRENT.md',
      400,
    ),
    templatePath: cleanText(
      readFlag(argv, 'template') || '.github/pull_request_template.md',
      400,
    ),
    boardPath: cleanText(
      readFlag(argv, 'board') || 'client/runtime/config/runtime_closure_board.json',
      400,
    ),
    gateRegistryPath: cleanText(
      readFlag(argv, 'gate-registry') || 'tests/tooling/config/tooling_gate_registry.json',
      400,
    ),
  };
}

function readTextBestEffort(filePath: string): string {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
  }
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function sectionBody(markdown: string, heading: string): string {
  const token = `## ${heading}`;
  const start = markdown.indexOf(token);
  if (start < 0) return '';
  const bodyStart = start + token.length;
  const rest = markdown.slice(bodyStart);
  const nextHeadingOffset = rest.search(/\n##\s+/);
  if (nextHeadingOffset < 0) return rest.trim();
  return rest.slice(0, nextHeadingOffset).trim();
}

function parseMarkdownTable(section: string): ParsedTable | null {
  const lines = section
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith('|'));
  if (lines.length < 2) return null;
  const splitRow = (line: string): string[] =>
    line
      .split('|')
      .slice(1, -1)
      .map((cell) => cleanText(cell, 240));
  const headers = splitRow(lines[0]);
  const rows = lines
    .slice(2)
    .map(splitRow)
    .filter((row) => row.some((cell) => cell.length > 0));
  return { headers, rows };
}

function indexOfHeader(headers: string[], name: string): number {
  const target = cleanText(name, 120).toLowerCase();
  return headers.findIndex((header) => cleanText(header, 120).toLowerCase() === target);
}

function renderMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Closure Feature Alignment Guard');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`- pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push(`- github_event_name: ${cleanText(payload?.summary?.github_event_name || '', 80) || 'unknown'}`);
  lines.push(`- pr_body_checked: ${payload?.summary?.pr_body_checked === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- template_marker_failures: ${Number(payload?.summary?.template_marker_failures || 0)}`);
  lines.push(`- major_row_failures: ${Number(payload?.summary?.major_row_failures || 0)}`);
  lines.push(`- capability_row_failures: ${Number(payload?.summary?.capability_row_failures || 0)}`);
  lines.push(`- failure_count: ${Number(payload?.summary?.failure_count || 0)}`);
  lines.push('');
  const failures = Array.isArray(payload?.failures) ? payload.failures : [];
  if (failures.length > 0) {
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(
        `- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(failure?.detail || '', 260)}`,
      );
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function writeMarkdown(filePath: string, body: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body, 'utf8');
}

function countOccurrences(text: string, needle: string): number {
  if (!needle) return 0;
  const source = cleanText(text, 120_000);
  let count = 0;
  let offset = 0;
  while (offset >= 0 && offset < source.length) {
    const next = source.indexOf(needle, offset);
    if (next < 0) break;
    count += 1;
    offset = next + needle.length;
  }
  return count;
}

function parseIsoUtcMillis(value: string): number | null {
  const cleaned = cleanText(value, 120);
  if (!cleaned) return null;
  const millis = Date.parse(cleaned);
  if (!Number.isFinite(millis)) return null;
  return millis;
}

function sortedTokens(values: string[]): string[] {
  return [...values].sort((left, right) => left.localeCompare(right));
}

function hasSignal(raw: string, signals: string[]): boolean {
  const text = cleanText(raw, 2000).toLowerCase();
  if (!text) return false;
  return signals.some((signal) => text.includes(signal.toLowerCase()));
}

function wordCount(raw: string): number {
  return cleanText(raw, 4000)
    .split(/\s+/)
    .map((token) => cleanText(token, 120))
    .filter(Boolean).length;
}

function lexicalTokens(raw: string): string[] {
  const stopwords = new Set([
    'the',
    'and',
    'for',
    'with',
    'this',
    'that',
    'from',
    'into',
    'onto',
    'over',
    'under',
    'major',
    'minor',
    'runtime',
    'closure',
    'feature',
    'capability',
  ]);
  return cleanText(raw, 4000)
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .map((token) => cleanText(token, 120))
    .filter((token) => token.length >= 3 && !stopwords.has(token));
}

function hasLexicalOverlap(left: string, right: string): boolean {
  const leftTokens = new Set(lexicalTokens(left));
  if (leftTokens.size === 0) return false;
  const rightTokens = lexicalTokens(right);
  return rightTokens.some((token) => leftTokens.has(token));
}

function anchorLinkHints(anchors: string[]): string[] {
  const hints: string[] = [];
  for (const anchor of anchors) {
    const normalized = cleanText(anchor, 400).toLowerCase();
    if (!normalized) continue;
    if (normalized.startsWith('ops:')) {
      hints.push(
        ...normalized
          .split(':')
          .map((token) => cleanText(token, 120))
          .filter((token) => token.length >= 4 && token !== 'ops'),
      );
      continue;
    }
    if (normalized.startsWith('core/local/artifacts/')) {
      const basename = normalized.slice(normalized.lastIndexOf('/') + 1).replace(/_current\.json$/, '');
      hints.push(
        ...basename
          .split(/[_-]+/)
          .map((token) => cleanText(token, 120))
          .filter((token) => token.length >= 4),
      );
    }
  }
  return uniqueTokens(hints);
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const outPathToken = cleanText(args.outPath || '', 500);
  const markdownPathToken = cleanText(args.markdownPath || '', 500);
  const templatePathToken = cleanText(args.templatePath || '', 500);
  const boardPathToken = cleanText(args.boardPath || '', 500);
  const gateRegistryPathToken = cleanText(args.gateRegistryPath || '', 500);
  const templatePath = path.resolve(root, args.templatePath);
  const boardPath = path.resolve(root, args.boardPath);
  const gateRegistryPath = path.resolve(root, args.gateRegistryPath);
  const template = readTextBestEffort(templatePath);
  const board = readJsonBestEffort(boardPath);
  const gateRegistry = readJsonBestEffort(gateRegistryPath);
  const bucketIds = new Set<string>(
    Array.isArray(board?.buckets)
      ? board.buckets
          .map((row: any) => cleanText(row?.id || '', 80))
          .filter(Boolean)
      : [],
  );
  const knownRegistryGates = new Set<string>(Object.keys(gateRegistry?.gates || {}));
  const failures: Array<{ id: string; detail: string }> = [];
  const expectedBoardSchemaId = 'runtime_closure_board';
  const expectedBoardSchemaVersion = 1;
  const scopeAllowlist = new Set(['major', 'minor']);
  const bucketStatusAllowlist = new Set(['active', 'experimental', 'blocked']);
  const expectedScopeOrder = ['major', 'minor'];
  const expectedBucketStatusOrder = ['active', 'experimental', 'blocked'];
  const expectedBoardTopLevelKeys = ['schema_id', 'schema_version', 'updated_at', 'buckets'];
  const expectedBucketRowKeys = [
    'id',
    'label',
    'owner',
    'status',
    'validation_gates',
    'evidence_artifacts',
  ];
  const shareableCrossBucketGateAllowlist = new Set(['ops:runtime-proof:verify']);
  const expectedBucketOrder = [
    'layer2_parity',
    'production_gateways',
    'boundedness',
    'dashboard_truth',
    'auto_heal_backpressure',
  ];
  const expectedBucketSet = new Set(expectedBucketOrder);
  const nowMillis = Date.now();
  const maxFutureSkewMs = 10 * 60 * 1000;
  const maxBoardStalenessMs = 30 * 24 * 60 * 60 * 1000;
  const uniqueInputPathTokens = new Set([
    outPathToken,
    markdownPathToken,
    templatePathToken,
    boardPathToken,
    gateRegistryPathToken,
  ]);
  const expectedOwnerByBucket: Record<string, string> = {
    layer2_parity: 'kernel-runtime',
    production_gateways: 'gateway-runtime',
    boundedness: 'runtime-proof',
    dashboard_truth: 'shell-authority',
    auto_heal_backpressure: 'runtime-recovery',
  };
  const expectedOwnerKeys = Object.keys(expectedOwnerByBucket);
  const boardBuckets = Array.isArray(board?.buckets) ? board.buckets : [];
  const boardBucketIdList = boardBuckets
    .map((row: any) => cleanText(row?.id || '', 80))
    .filter(Boolean);
  const boardBucketIdsUnique = new Set(boardBucketIdList).size === boardBucketIdList.length;
  const boardBucketIdsCanonical = boardBucketIdList.every((bucketId) => isCanonicalBucketToken(bucketId));
  const shareableGateAllowlist = Array.from(shareableCrossBucketGateAllowlist.values());
  const shareableGateAllowlistUnique = new Set(shareableGateAllowlist).size === shareableGateAllowlist.length;
  const shareableGateAllowlistCanonical = shareableGateAllowlist.every((gateId) => isCanonicalGateToken(gateId));
  const templateLower = cleanText(template, 120_000).toLowerCase();
  const requiredTemplateSections = ['## major features', '## capability features'];
  const missingTemplateSections = requiredTemplateSections.filter(
    (token) => !templateLower.includes(token),
  );

  if (outPathToken !== 'core/local/artifacts/runtime_closure_feature_alignment_guard_current.json') {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_out_path_exact_contract',
      detail: outPathToken || 'missing',
    });
  }
  if (
    markdownPathToken
    !== 'local/workspace/reports/RUNTIME_CLOSURE_FEATURE_ALIGNMENT_GUARD_CURRENT.md'
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_markdown_path_exact_contract',
      detail: markdownPathToken || 'missing',
    });
  }
  if (templatePathToken !== '.github/pull_request_template.md') {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_template_path_exact_contract',
      detail: templatePathToken || 'missing',
    });
  }
  if (gateRegistryPathToken !== 'tests/tooling/config/tooling_gate_registry.json') {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_gate_registry_path_exact_contract',
      detail: gateRegistryPathToken || 'missing',
    });
  }
  if (placeholderToken(outPathToken)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_out_path_not_placeholder_contract',
      detail: outPathToken || 'missing',
    });
  }
  if (placeholderToken(markdownPathToken)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_markdown_path_not_placeholder_contract',
      detail: markdownPathToken || 'missing',
    });
  }
  if (placeholderToken(templatePathToken)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_template_path_not_placeholder_contract',
      detail: templatePathToken || 'missing',
    });
  }
  if (placeholderToken(boardPathToken)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_board_path_not_placeholder_contract',
      detail: boardPathToken || 'missing',
    });
  }
  if (placeholderToken(gateRegistryPathToken)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_gate_registry_path_not_placeholder_contract',
      detail: gateRegistryPathToken || 'missing',
    });
  }
  if (!fs.existsSync(templatePath)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_template_file_exists_contract',
      detail: templatePathToken || 'missing',
    });
  }
  if (!fs.existsSync(boardPath)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_board_file_exists_contract',
      detail: boardPathToken || 'missing',
    });
  }
  if (!fs.existsSync(gateRegistryPath)) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_gate_registry_file_exists_contract',
      detail: gateRegistryPathToken || 'missing',
    });
  }
  if (cleanText(template, 120_000).length < 400) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_template_minimum_length_contract',
      detail: `${cleanText(template, 120_000).length}`,
    });
  }
  if (missingTemplateSections.length > 0) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_template_required_sections_contract',
      detail: missingTemplateSections.join(','),
    });
  }
  if (shareableGateAllowlist.length === 0) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_shareable_gate_allowlist_nonempty_contract',
      detail: 'empty',
    });
  }
  if (!shareableGateAllowlistCanonical) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_shareable_gate_allowlist_token_contract',
      detail: shareableGateAllowlist.join(','),
    });
  }
  if (!shareableGateAllowlistUnique) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_shareable_gate_allowlist_unique_contract',
      detail: shareableGateAllowlist.join(','),
    });
  }
  if (expectedBucketSet.size !== expectedBucketOrder.length) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_expected_bucket_set_size_contract',
      detail: `${expectedBucketSet.size}|${expectedBucketOrder.length}`,
    });
  }
  if (!boardBucketIdsUnique) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_board_bucket_ids_unique_contract',
      detail: boardBucketIdList.join(','),
    });
  }
  if (!boardBucketIdsCanonical) {
    failures.push({
      id: 'runtime_closure_feature_alignment_dp_board_bucket_ids_canonical_contract',
      detail: boardBucketIdList.join(','),
    });
  }

  if (!isCanonicalRepoRelativePathToken(outPathToken, 'core/local/artifacts/', '_current.json')) {
    failures.push({
      id: 'runtime_closure_feature_alignment_out_path_noncanonical',
      detail: outPathToken || 'missing',
    });
  }
  if (!isCanonicalRepoRelativePathToken(markdownPathToken, 'local/workspace/reports/', '.md')) {
    failures.push({
      id: 'runtime_closure_feature_alignment_markdown_path_noncanonical',
      detail: markdownPathToken || 'missing',
    });
  }
  if (!isCanonicalRepoRelativePathToken(templatePathToken, '.github/', '.md')) {
    failures.push({
      id: 'runtime_closure_feature_alignment_template_path_noncanonical',
      detail: templatePathToken || 'missing',
    });
  }
  if (
    !isCanonicalRepoRelativePathToken(boardPathToken, 'client/runtime/config/', '.json')
    || boardPathToken !== 'client/runtime/config/runtime_closure_board.json'
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_board_path_noncanonical',
      detail: boardPathToken || 'missing',
    });
  }
  if (!isCanonicalRepoRelativePathToken(gateRegistryPathToken, 'tests/tooling/config/', '.json')) {
    failures.push({
      id: 'runtime_closure_feature_alignment_gate_registry_path_noncanonical',
      detail: gateRegistryPathToken || 'missing',
    });
  }
  if (path.relative(root, templatePath).startsWith('..') || path.isAbsolute(path.relative(root, templatePath))) {
    failures.push({
      id: 'runtime_closure_feature_alignment_template_path_out_of_repo',
      detail: templatePathToken || 'missing',
    });
  }
  if (path.relative(root, boardPath).startsWith('..') || path.isAbsolute(path.relative(root, boardPath))) {
    failures.push({
      id: 'runtime_closure_feature_alignment_board_path_out_of_repo',
      detail: boardPathToken || 'missing',
    });
  }
  if (
    path.relative(root, gateRegistryPath).startsWith('..')
    || path.isAbsolute(path.relative(root, gateRegistryPath))
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_gate_registry_path_out_of_repo',
      detail: gateRegistryPathToken || 'missing',
    });
  }
  if (uniqueInputPathTokens.size !== 5) {
    failures.push({
      id: 'runtime_closure_feature_alignment_input_path_collision',
      detail: `${outPathToken}|${markdownPathToken}|${templatePathToken}|${boardPathToken}|${gateRegistryPathToken}`,
    });
  }
  if (
    duplicateValues(expectedBoardTopLevelKeys).length > 0
    || expectedBoardTopLevelKeys.length !== 4
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_expected_board_top_level_keys_invalid',
      detail: expectedBoardTopLevelKeys.join(','),
    });
  }
  if (
    duplicateValues(expectedBucketRowKeys).length > 0
    || expectedBucketRowKeys.length !== 6
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_expected_bucket_row_keys_invalid',
      detail: expectedBucketRowKeys.join(','),
    });
  }
  if (expectedOwnerKeys.some((bucketId) => !isCanonicalBucketToken(bucketId))) {
    failures.push({
      id: 'runtime_closure_feature_alignment_expected_owner_map_bucket_token_invalid',
      detail: expectedOwnerKeys.join(','),
    });
  }
  const scopeOrderActual = Array.from(scopeAllowlist.values());
  if (
    scopeOrderActual.length !== expectedScopeOrder.length
    || scopeOrderActual.some((value, index) => value !== expectedScopeOrder[index])
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_scope_allowlist_contract_invalid',
      detail: scopeOrderActual.join(','),
    });
  }
  const statusOrderActual = Array.from(bucketStatusAllowlist.values());
  if (
    statusOrderActual.length !== expectedBucketStatusOrder.length
    || statusOrderActual.some((value, index) => value !== expectedBucketStatusOrder[index])
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_bucket_status_allowlist_contract_invalid',
      detail: statusOrderActual.join(','),
    });
  }
  if (cleanText(gateRegistry?.version || '', 40) !== '1.0') {
    failures.push({
      id: 'runtime_closure_feature_alignment_gate_registry_version_invalid',
      detail: cleanText(gateRegistry?.version || '', 40) || 'missing',
    });
  }
  if (gateRegistry?.gates && typeof gateRegistry.gates === 'object' && Object.keys(gateRegistry.gates).length === 0) {
    failures.push({
      id: 'runtime_closure_feature_alignment_gate_registry_empty',
      detail: gateRegistryPathToken || 'missing',
    });
  }
  if (knownRegistryGates.size > 0 && Array.from(knownRegistryGates).some((gateId) => !isCanonicalGateToken(gateId))) {
    failures.push({
      id: 'runtime_closure_feature_alignment_gate_registry_gate_id_noncanonical',
      detail: Array.from(knownRegistryGates).filter((gateId) => !isCanonicalGateToken(gateId)).join(','),
    });
  }
  if (
    expectedOwnerKeys.length !== expectedBucketOrder.length
    || expectedOwnerKeys.some((bucketId) => !expectedBucketSet.has(bucketId))
  ) {
    failures.push({
      id: 'runtime_closure_board_expected_owner_map_set_drift',
      detail: expectedOwnerKeys.join(','),
    });
  }
  const duplicateExpectedBuckets = duplicateValues(expectedBucketOrder);
  if (duplicateExpectedBuckets.length > 0) {
    failures.push({
      id: 'runtime_closure_board_expected_bucket_order_duplicate',
      detail: Array.from(new Set(duplicateExpectedBuckets)).join(','),
    });
  }
  const invalidExpectedBuckets = expectedBucketOrder.filter((bucketId) => !isCanonicalBucketToken(bucketId));
  if (invalidExpectedBuckets.length > 0) {
    failures.push({
      id: 'runtime_closure_board_expected_bucket_order_noncanonical',
      detail: invalidExpectedBuckets.join(','),
    });
  }
  const duplicateExpectedOwnersCasefold = casefoldDuplicateValues(Object.values(expectedOwnerByBucket));
  if (duplicateExpectedOwnersCasefold.length > 0) {
    failures.push({
      id: 'runtime_closure_board_expected_owner_map_owner_duplicate_casefold',
      detail: Array.from(new Set(duplicateExpectedOwnersCasefold)).join(','),
    });
  }
  const invalidExpectedOwners = Object.entries(expectedOwnerByBucket)
    .filter(([, owner]) => !isCanonicalOwnerToken(owner))
    .map(([bucketId, owner]) => `${bucketId}:${owner}`);
  if (invalidExpectedOwners.length > 0) {
    failures.push({
      id: 'runtime_closure_board_expected_owner_map_owner_token_invalid',
      detail: invalidExpectedOwners.join(','),
    });
  }
  const duplicateShareableGatesCasefold = casefoldDuplicateValues(
    Array.from(shareableCrossBucketGateAllowlist),
  );
  if (duplicateShareableGatesCasefold.length > 0) {
    failures.push({
      id: 'runtime_closure_board_shareable_gate_allowlist_duplicate_casefold',
      detail: Array.from(new Set(duplicateShareableGatesCasefold)).join(','),
    });
  }
  const invalidShareableGates = Array.from(shareableCrossBucketGateAllowlist)
    .filter((gateId) => !isCanonicalGateToken(gateId));
  if (invalidShareableGates.length > 0) {
    failures.push({
      id: 'runtime_closure_board_shareable_gate_allowlist_noncanonical',
      detail: invalidShareableGates.join(','),
    });
  }
  const unknownShareableGates = Array.from(shareableCrossBucketGateAllowlist)
    .filter((gateId) => !knownRegistryGates.has(gateId));
  if (unknownShareableGates.length > 0) {
    failures.push({
      id: 'runtime_closure_board_shareable_gate_allowlist_unknown_registry',
      detail: unknownShareableGates.join(','),
    });
  }

  if (!board) {
    failures.push({
      id: 'runtime_closure_board_missing',
      detail: args.boardPath,
    });
  } else {
    const boardKeys = Object.keys(board || {});
    const missingBoardKeys = expectedBoardTopLevelKeys.filter((key) => !boardKeys.includes(key));
    if (missingBoardKeys.length > 0) {
      failures.push({
        id: 'runtime_closure_board_top_level_key_missing',
        detail: missingBoardKeys.join(','),
      });
    }
    const unexpectedBoardKeys = boardKeys.filter((key) => !expectedBoardTopLevelKeys.includes(key));
    if (unexpectedBoardKeys.length > 0) {
      failures.push({
        id: 'runtime_closure_board_top_level_key_unexpected',
        detail: unexpectedBoardKeys.join(','),
      });
    }
    const schemaId = cleanText(board?.schema_id || '', 80);
    const schemaVersion = Number(board?.schema_version || 0);
    if (schemaId !== expectedBoardSchemaId) {
      failures.push({
        id: 'runtime_closure_board_schema_id_invalid',
        detail: schemaId || 'missing',
      });
    }
    if (schemaVersion !== expectedBoardSchemaVersion) {
      failures.push({
        id: 'runtime_closure_board_schema_version_invalid',
        detail: Number.isFinite(schemaVersion) ? String(schemaVersion) : 'missing',
      });
    }
    if (!Number.isInteger(schemaVersion) || schemaVersion <= 0) {
      failures.push({
        id: 'runtime_closure_board_schema_version_noninteger_or_nonpositive',
        detail: Number.isFinite(schemaVersion) ? String(schemaVersion) : 'missing',
      });
    }
    const updatedAt = cleanText(board?.updated_at || '', 80);
    if (!isIsoUtcTimestamp(updatedAt)) {
      failures.push({
        id: 'runtime_closure_board_updated_at_invalid',
        detail: updatedAt || 'missing',
      });
    } else {
      const updatedAtMillis = parseIsoUtcMillis(updatedAt);
      if (updatedAtMillis === null) {
        failures.push({
          id: 'runtime_closure_board_updated_at_unparseable',
          detail: updatedAt,
        });
      } else {
        if (updatedAtMillis > nowMillis + maxFutureSkewMs) {
          failures.push({
            id: 'runtime_closure_board_updated_at_future_skew',
            detail: updatedAt,
          });
        }
        if (updatedAtMillis < nowMillis - maxBoardStalenessMs) {
          failures.push({
            id: 'runtime_closure_board_updated_at_stale',
            detail: updatedAt,
          });
        }
      }
    }
  }
  if (!gateRegistry) {
    failures.push({
      id: 'runtime_closure_board_gate_registry_missing',
      detail: args.gateRegistryPath,
    });
  } else if (!gateRegistry.gates || typeof gateRegistry.gates !== 'object') {
    failures.push({
      id: 'runtime_closure_board_gate_registry_gates_missing',
      detail: args.gateRegistryPath,
    });
  }

  const bucketRowsRaw = Array.isArray(board?.buckets) ? board.buckets : [];
  if (board && !Array.isArray(board?.buckets)) {
    failures.push({
      id: 'runtime_closure_board_bucket_rows_type_invalid',
      detail: typeof board?.buckets,
    });
  }
  const bucketIdsRaw = bucketRowsRaw
    .map((row: any) => cleanText(row?.id || '', 80))
    .filter(Boolean);
  const duplicateBucketIds = bucketIdsRaw.filter((id, index, arr) => arr.indexOf(id) !== index);
  if (duplicateBucketIds.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_id_duplicate',
      detail: Array.from(new Set(duplicateBucketIds)).join(','),
    });
  }
  const invalidBucketIds = bucketIdsRaw.filter((id) => !/^[a-z0-9_]+$/.test(id));
  if (invalidBucketIds.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_id_noncanonical',
      detail: invalidBucketIds.join(','),
    });
  }
  if (bucketIds.size === 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_ids_missing',
      detail: args.boardPath,
    });
  }
  if (bucketRowsRaw.length !== expectedBucketOrder.length) {
    failures.push({
      id: 'runtime_closure_board_bucket_count_drift',
      detail: `actual=${bucketRowsRaw.length};expected=${expectedBucketOrder.length}`,
    });
  }
  const missingExpectedBuckets = expectedBucketOrder.filter((bucketId) => !bucketIds.has(bucketId));
  if (missingExpectedBuckets.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_set_missing_expected',
      detail: missingExpectedBuckets.join(','),
    });
  }
  const unknownBuckets = bucketIdsRaw.filter((bucketId) => !expectedBucketSet.has(bucketId));
  if (unknownBuckets.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_set_unknown',
      detail: unknownBuckets.join(','),
    });
  }
  const bucketOrderMatches =
    bucketIdsRaw.length === expectedBucketOrder.length
    && bucketIdsRaw.every((bucketId, index) => bucketId === expectedBucketOrder[index]);
  if (!bucketOrderMatches) {
    failures.push({
      id: 'runtime_closure_board_bucket_order_drift',
      detail: bucketIdsRaw.join(',') || 'missing',
    });
  }

  const knownValidationTargets = new Set<string>();
  const bucketLabels: string[] = [];
  const bucketOwners: string[] = [];
  const allEvidenceArtifacts: string[] = [];
  const gateToBuckets = new Map<string, Set<string>>();
  const bucketToValidationTargets = new Map<string, Set<string>>();
  for (const row of bucketRowsRaw) {
    if (!row || typeof row !== 'object' || Array.isArray(row)) {
      failures.push({
        id: 'runtime_closure_board_bucket_row_not_object',
        detail: typeof row,
      });
      continue;
    }
    for (const gateId of Array.isArray(row?.validation_gates) ? row.validation_gates : []) {
      const normalized = cleanText(gateId || '', 160);
      if (normalized) knownValidationTargets.add(normalized);
    }
    for (const artifactPath of Array.isArray(row?.evidence_artifacts) ? row.evidence_artifacts : []) {
      const normalized = cleanText(artifactPath || '', 260);
      if (normalized) knownValidationTargets.add(normalized);
    }
  }
  for (const row of bucketRowsRaw) {
    if (!row || typeof row !== 'object' || Array.isArray(row)) continue;
    const bucketId = cleanText(row?.id || '', 80);
    if (!isCanonicalBucketToken(bucketId)) continue;

    const rowKeys = Object.keys((row && typeof row === 'object' ? row : {}) as Record<string, unknown>);
    const missingRowKeys = expectedBucketRowKeys.filter((key) => !rowKeys.includes(key));
    if (missingRowKeys.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_row_key_missing',
        detail: `${bucketId}:${missingRowKeys.join(',')}`,
      });
    }
    const unexpectedRowKeys = rowKeys.filter((key) => !expectedBucketRowKeys.includes(key));
    if (unexpectedRowKeys.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_row_key_unexpected',
        detail: `${bucketId}:${unexpectedRowKeys.join(',')}`,
      });
    }

    const label = cleanText(row?.label || '', 160);
    if (!label) {
      failures.push({
        id: 'runtime_closure_board_bucket_label_missing',
        detail: bucketId,
      });
    } else {
      bucketLabels.push(label);
      if (placeholderToken(label)) {
        failures.push({
          id: 'runtime_closure_board_bucket_label_placeholder',
          detail: `${bucketId}:${label}`,
        });
      } else if (wordCount(label) < 2) {
        failures.push({
          id: 'runtime_closure_board_bucket_label_too_short',
          detail: `${bucketId}:${label}`,
        });
      } else if (!hasLexicalOverlap(label, bucketId.replace(/_/g, ' '))) {
        failures.push({
          id: 'runtime_closure_board_bucket_label_bucket_id_mismatch',
          detail: `${bucketId}:${label}`,
        });
      }
    }

    const owner = cleanText(row?.owner || '', 120);
    if (owner) bucketOwners.push(owner);
    if (!isCanonicalOwnerToken(owner)) {
      failures.push({
        id: 'runtime_closure_board_bucket_owner_invalid',
        detail: `${bucketId}:${owner || 'missing'}`,
      });
    } else if (placeholderToken(owner)) {
      failures.push({
        id: 'runtime_closure_board_bucket_owner_placeholder',
        detail: `${bucketId}:${owner}`,
      });
    } else if (!owner.includes('-')) {
      failures.push({
        id: 'runtime_closure_board_bucket_owner_shape_invalid',
        detail: `${bucketId}:${owner}`,
      });
    }
    const expectedOwner = cleanText(expectedOwnerByBucket[bucketId] || '', 120);
    if (expectedOwner && owner && owner !== expectedOwner) {
      failures.push({
        id: 'runtime_closure_board_bucket_owner_expected_mismatch',
        detail: `${bucketId}:${owner};expected=${expectedOwner}`,
      });
    }
    if (label && owner && label.toLowerCase() === owner.toLowerCase()) {
      failures.push({
        id: 'runtime_closure_board_bucket_label_owner_identical',
        detail: `${bucketId}:${label}`,
      });
    }

    const statusRaw = cleanText(row?.status || '', 80);
    const status = statusRaw.toLowerCase();
    if (!bucketStatusAllowlist.has(status)) {
      failures.push({
        id: 'runtime_closure_board_bucket_status_invalid',
        detail: `${bucketId}:${status || 'missing'}`,
      });
    } else if (status !== 'active') {
      failures.push({
        id: 'runtime_closure_board_bucket_status_nonactive',
        detail: `${bucketId}:${status}`,
      });
    }
    if (statusRaw && statusRaw !== status) {
      failures.push({
        id: 'runtime_closure_board_bucket_status_casing_invalid',
        detail: `${bucketId}:${statusRaw}`,
      });
    }

    if (!Array.isArray(row?.validation_gates)) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gates_type_invalid',
        detail: `${bucketId}:${typeof row?.validation_gates}`,
      });
    }
    const validationGates = Array.isArray(row?.validation_gates)
      ? row.validation_gates.map((gate: unknown) => cleanText(gate, 160)).filter(Boolean)
      : [];
    if (validationGates.length === 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gates_missing',
        detail: bucketId,
      });
    }
    const duplicateGates = duplicateValues(validationGates);
    if (duplicateGates.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gates_duplicate',
        detail: `${bucketId}:${Array.from(new Set(duplicateGates)).join(',')}`,
      });
    }
    const duplicateGatesCasefold = casefoldDuplicateValues(validationGates);
    if (duplicateGatesCasefold.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gates_duplicate_casefold',
        detail: `${bucketId}:${Array.from(new Set(duplicateGatesCasefold)).join(',')}`,
      });
    }
    const invalidGateTokens = validationGates.filter((gateId) => !isCanonicalGateToken(gateId));
    if (invalidGateTokens.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gate_noncanonical',
        detail: `${bucketId}:${invalidGateTokens.join(',')}`,
      });
    }
    for (const gateId of validationGates) {
      if (!gateToBuckets.has(gateId)) gateToBuckets.set(gateId, new Set<string>());
      gateToBuckets.get(gateId)?.add(bucketId);
    }
    const unknownRegistryGateTokens = validationGates.filter((gateId) => !knownRegistryGates.has(gateId));
    if (unknownRegistryGateTokens.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gate_unknown_registry',
        detail: `${bucketId}:${unknownRegistryGateTokens.join(',')}`,
      });
    }
    const sortedValidationGates = [...validationGates].sort();
    if (sortedValidationGates.join('|') !== validationGates.join('|')) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gates_unsorted',
        detail: bucketId,
      });
    }

    if (!Array.isArray(row?.evidence_artifacts)) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifacts_type_invalid',
        detail: `${bucketId}:${typeof row?.evidence_artifacts}`,
      });
    }
    const evidenceArtifacts = Array.isArray(row?.evidence_artifacts)
      ? row.evidence_artifacts.map((artifact: unknown) => cleanText(artifact, 260)).filter(Boolean)
      : [];
    if (evidenceArtifacts.length === 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifacts_missing',
        detail: bucketId,
      });
    }
    const duplicateArtifacts = duplicateValues(evidenceArtifacts);
    if (duplicateArtifacts.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifacts_duplicate',
        detail: `${bucketId}:${Array.from(new Set(duplicateArtifacts)).join(',')}`,
      });
    }
    const duplicateArtifactsCasefold = casefoldDuplicateValues(evidenceArtifacts);
    if (duplicateArtifactsCasefold.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifacts_duplicate_casefold',
        detail: `${bucketId}:${Array.from(new Set(duplicateArtifactsCasefold)).join(',')}`,
      });
    }
    const invalidArtifactTokens = evidenceArtifacts.filter(
      (artifactPath) =>
        artifactPath.includes('..')
        || artifactPath.startsWith('/')
        || /\s/.test(artifactPath),
    );
    if (invalidArtifactTokens.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifact_noncanonical',
        detail: `${bucketId}:${invalidArtifactTokens.join(',')}`,
      });
    }
    const outsideCoreLocal = evidenceArtifacts.filter(
      (artifactPath) => !artifactPath.startsWith('core/local/artifacts/'),
    );
    if (outsideCoreLocal.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifact_outside_core_local',
        detail: `${bucketId}:${outsideCoreLocal.join(',')}`,
      });
    }
    const artifactSuffixInvalid = evidenceArtifacts.filter(
      (artifactPath) => !isCanonicalArtifactToken(artifactPath),
    );
    if (artifactSuffixInvalid.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifact_suffix_invalid',
        detail: `${bucketId}:${artifactSuffixInvalid.join(',')}`,
      });
    }
    const sortedEvidenceArtifacts = [...evidenceArtifacts].sort();
    if (sortedEvidenceArtifacts.join('|') !== evidenceArtifacts.join('|')) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifacts_unsorted',
        detail: bucketId,
      });
    }
    const overlappingValidationAndArtifacts = validationGates.filter((gateId) =>
      evidenceArtifacts.some((artifactPath) => artifactPath === gateId),
    );
    if (overlappingValidationAndArtifacts.length > 0) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_artifact_overlap',
        detail: `${bucketId}:${overlappingValidationAndArtifacts.join(',')}`,
      });
    }
    allEvidenceArtifacts.push(...evidenceArtifacts);
    bucketToValidationTargets.set(bucketId, new Set<string>([...validationGates, ...evidenceArtifacts]));
    if (validationGates.length > 6) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gate_too_many',
        detail: `${bucketId}:${validationGates.length}`,
      });
    }
    if (evidenceArtifacts.length > 8) {
      failures.push({
        id: 'runtime_closure_board_bucket_evidence_artifact_too_many',
        detail: `${bucketId}:${evidenceArtifacts.length}`,
      });
    }
    const traceabilitySurface = [...validationGates, ...evidenceArtifacts].join(' ').toLowerCase();
    const bucketIdTokens = bucketId.split('_').map((token) => cleanText(token, 120)).filter((token) => token.length >= 4);
    if (bucketIdTokens.length > 0 && !bucketIdTokens.some((token) => traceabilitySurface.includes(token.toLowerCase()))) {
      failures.push({
        id: 'runtime_closure_board_bucket_traceability_missing',
        detail: bucketId,
      });
    }
  }
  const duplicateLabels = duplicateValues(bucketLabels);
  if (duplicateLabels.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_label_duplicate',
      detail: Array.from(new Set(duplicateLabels)).join(','),
    });
  }
  const duplicateEvidenceArtifactsGlobal = duplicateValues(allEvidenceArtifacts);
  if (duplicateEvidenceArtifactsGlobal.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_evidence_artifact_duplicate_global',
      detail: Array.from(new Set(duplicateEvidenceArtifactsGlobal)).join(','),
    });
  }
  const duplicateLabelsCasefold = casefoldDuplicateValues(bucketLabels);
  if (duplicateLabelsCasefold.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_label_duplicate_casefold',
      detail: Array.from(new Set(duplicateLabelsCasefold)).join(','),
    });
  }
  const duplicateOwnersCasefold = casefoldDuplicateValues(bucketOwners);
  if (duplicateOwnersCasefold.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_owner_duplicate_casefold',
      detail: Array.from(new Set(duplicateOwnersCasefold)).join(','),
    });
  }
  const duplicateEvidenceArtifactsGlobalCasefold = casefoldDuplicateValues(allEvidenceArtifacts);
  if (duplicateEvidenceArtifactsGlobalCasefold.length > 0) {
    failures.push({
      id: 'runtime_closure_board_bucket_evidence_artifact_duplicate_casefold_global',
      detail: Array.from(new Set(duplicateEvidenceArtifactsGlobalCasefold)).join(','),
    });
  }
  for (const [gateId, bucketSet] of gateToBuckets.entries()) {
    if (bucketSet.size > 1 && !shareableCrossBucketGateAllowlist.has(gateId)) {
      failures.push({
        id: 'runtime_closure_board_bucket_validation_gate_cross_bucket_unapproved',
        detail: `${gateId}:${Array.from(bucketSet).join(',')}`,
      });
    }
  }
  if (knownValidationTargets.size === 0) {
    failures.push({
      id: 'runtime_closure_board_validation_targets_missing',
      detail: args.boardPath,
    });
  }

  const templateMarkers = [
    '## Runtime Closure Feature Alignment (required for major surface features)',
    '| Feature Surface | Scope (`major`/`minor`) | Runtime Closure Bucket | Validation Artifact / Gate |',
    'If any feature scope is `major`, each major feature maps to a runtime-closure bucket and directly validates it with a linked proof artifact, replay fixture, or release gate.',
    'Every visible capability change links to at least one proof artifact, replay fixture, or release gate.',
    '- [ ] No exterior capability expansion without verifiable runtime truth increase.',
    '- [ ] Every visible capability change links to at least one proof artifact, replay fixture, or release gate.',
  ];
  for (const marker of templateMarkers) {
    if (!template.includes(marker)) {
      failures.push({
        id: 'runtime_closure_template_marker_missing',
        detail: marker,
      });
    } else if (countOccurrences(template, marker) > 1) {
      failures.push({
        id: 'runtime_closure_template_marker_duplicate',
        detail: marker,
      });
    }
  }
  const capabilitySectionHeading = '## Capability Proof Burden (required for new or expanded capabilities)';
  if (!template.includes(capabilitySectionHeading)) {
    failures.push({
      id: 'runtime_closure_template_capability_section_heading_missing',
      detail: capabilitySectionHeading,
    });
  } else if (countOccurrences(template, capabilitySectionHeading) > 1) {
    failures.push({
      id: 'runtime_closure_template_capability_section_heading_duplicate',
      detail: capabilitySectionHeading,
    });
  }
  const capabilityTableHeader =
    '| Capability | Proof Artifact / Replay Fixture / Gate | Invariant | Failure Mode | Receipt Surface | Recovery Behavior | Verifiable Runtime Truth Increase |';
  if (template.includes(capabilityTableHeader) && countOccurrences(template, capabilityTableHeader) > 1) {
    failures.push({
      id: 'runtime_closure_template_capability_table_header_duplicate',
      detail: capabilityTableHeader,
    });
  }
  const alignmentSectionHeading = '## Runtime Closure Feature Alignment (required for major surface features)';
  if (template.includes(alignmentSectionHeading) && countOccurrences(template, alignmentSectionHeading) > 1) {
    failures.push({
      id: 'runtime_closure_template_alignment_section_heading_duplicate',
      detail: alignmentSectionHeading,
    });
  }
  const alignmentTableHeader =
    '| Feature Surface | Scope (`major`/`minor`) | Runtime Closure Bucket | Validation Artifact / Gate |';
  if (template.includes(alignmentTableHeader) && countOccurrences(template, alignmentTableHeader) > 1) {
    failures.push({
      id: 'runtime_closure_template_alignment_table_header_duplicate',
      detail: alignmentTableHeader,
    });
  }
  const majorScopeCheckbox =
    '- [ ] If any feature scope is `major`, each major feature maps to a runtime-closure bucket and directly validates it with a linked proof artifact, replay fixture, or release gate.';
  if (template.includes(majorScopeCheckbox) && countOccurrences(template, majorScopeCheckbox) > 1) {
    failures.push({
      id: 'runtime_closure_template_major_scope_checkbox_duplicate',
      detail: majorScopeCheckbox,
    });
  }
  const noExteriorCheckbox = '- [ ] No exterior capability expansion without verifiable runtime truth increase.';
  if (template.includes(noExteriorCheckbox) && countOccurrences(template, noExteriorCheckbox) > 1) {
    failures.push({
      id: 'runtime_closure_template_capability_no_exterior_checkbox_duplicate',
      detail: noExteriorCheckbox,
    });
  }
  const capabilityLinkCheckbox =
    '- [ ] Every visible capability change links to at least one proof artifact, replay fixture, or release gate.';
  const validationSectionHeading = '## Validation';
  const evidenceSectionHeading = '## Evidence';
  const alignmentSectionHeadingCount = countOccurrences(template, alignmentSectionHeading);
  if (alignmentSectionHeadingCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_alignment_section_heading_count_invalid',
      detail: String(alignmentSectionHeadingCount),
    });
  }
  const alignmentTableHeaderCount = countOccurrences(template, alignmentTableHeader);
  if (alignmentTableHeaderCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_alignment_table_header_count_invalid',
      detail: String(alignmentTableHeaderCount),
    });
  }
  const majorScopeCheckboxCount = countOccurrences(template, majorScopeCheckbox);
  if (majorScopeCheckboxCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_major_scope_checkbox_count_invalid',
      detail: String(majorScopeCheckboxCount),
    });
  }
  const noExteriorCheckboxCount = countOccurrences(template, noExteriorCheckbox);
  if (noExteriorCheckboxCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_capability_no_exterior_checkbox_count_invalid',
      detail: String(noExteriorCheckboxCount),
    });
  }
  const capabilityLinkCheckboxCount = countOccurrences(template, capabilityLinkCheckbox);
  if (capabilityLinkCheckboxCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_capability_link_checkbox_count_invalid',
      detail: String(capabilityLinkCheckboxCount),
    });
  }
  const validationSectionCount = countOccurrences(template, validationSectionHeading);
  if (validationSectionCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_validation_section_count_invalid',
      detail: String(validationSectionCount),
    });
  }
  const evidenceSectionCount = countOccurrences(template, evidenceSectionHeading);
  if (evidenceSectionCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_evidence_section_count_invalid',
      detail: String(evidenceSectionCount),
    });
  }
  const alignmentSectionIndex = template.indexOf(alignmentSectionHeading);
  const capabilitySectionIndex = template.indexOf(capabilitySectionHeading);
  const evidenceSectionIndex = template.indexOf(evidenceSectionHeading);
  if (
    alignmentSectionIndex >= 0
    && capabilitySectionIndex >= 0
    && capabilitySectionIndex < alignmentSectionIndex
  ) {
    failures.push({
      id: 'runtime_closure_template_section_order_alignment_capability_invalid',
      detail: `${alignmentSectionIndex}:${capabilitySectionIndex}`,
    });
  }
  if (
    capabilitySectionIndex >= 0
    && evidenceSectionIndex >= 0
    && evidenceSectionIndex < capabilitySectionIndex
  ) {
    failures.push({
      id: 'runtime_closure_template_section_order_capability_evidence_invalid',
      detail: `${capabilitySectionIndex}:${evidenceSectionIndex}`,
    });
  }
  const alignmentSectionBody = sectionBody(template, 'Runtime Closure Feature Alignment');
  const capabilitySectionBody = sectionBody(template, 'Capability Proof Burden');
  const alignmentTableHeaderToken =
    '| Feature Surface | Scope (`major`/`minor`) | Runtime Closure Bucket | Validation Artifact / Gate |';
  const capabilityTableHeaderToken =
    '| Capability | Proof Artifact / Replay Fixture / Gate | Invariant | Failure Mode | Receipt Surface | Recovery Behavior | Verifiable Runtime Truth Increase |';
  const alignmentTableCount = countOccurrences(alignmentSectionBody, alignmentTableHeaderToken);
  if (alignmentTableCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_alignment_table_count_invalid',
      detail: String(alignmentTableCount),
    });
  }
  if (alignmentSectionBody && alignmentTableCount === 0) {
    failures.push({
      id: 'runtime_closure_template_alignment_table_missing',
      detail: 'Runtime Closure Feature Alignment',
    });
  }
  const capabilityTableCount = countOccurrences(capabilitySectionBody, capabilityTableHeaderToken);
  if (capabilityTableCount !== 1) {
    failures.push({
      id: 'runtime_closure_template_capability_table_count_invalid',
      detail: String(capabilityTableCount),
    });
  }
  if (capabilitySectionBody && capabilityTableCount === 0) {
    failures.push({
      id: 'runtime_closure_template_capability_table_missing',
      detail: 'Capability Proof Burden',
    });
  }

  const eventName = cleanText(process.env.GITHUB_EVENT_NAME || '', 80);
  const eventPath = cleanText(process.env.GITHUB_EVENT_PATH || '', 400);
  const eventPayload =
    eventPath.length > 0 && fs.existsSync(eventPath) ? readJsonBestEffort(eventPath) : null;
  const prBody = cleanText(eventPayload?.pull_request?.body || '', 40_000);
  const shouldCheckPrBody = eventName === 'pull_request' || eventName === 'pull_request_target';
  if (shouldCheckPrBody && eventPath.length > 0 && !eventPayload) {
    failures.push({
      id: 'runtime_closure_feature_alignment_event_payload_parse_failed',
      detail: eventPath,
    });
  }
  if (shouldCheckPrBody && eventPayload && (typeof eventPayload !== 'object' || Array.isArray(eventPayload))) {
    failures.push({
      id: 'runtime_closure_feature_alignment_event_payload_type_invalid',
      detail: typeof eventPayload,
    });
  }
  if (
    shouldCheckPrBody
    && eventPayload
    && (typeof eventPayload?.pull_request !== 'object' || Array.isArray(eventPayload?.pull_request))
  ) {
    failures.push({
      id: 'runtime_closure_feature_alignment_event_pull_request_payload_missing',
      detail: eventPath || 'missing',
    });
  }

  let majorRowsChecked = 0;
  let capabilityRowsChecked = 0;
  let alignmentRowsChecked = 0;
  const majorFeatureSurfaces: string[] = [];
  const majorBucketOwners = new Map<string, string>();
  const capabilityCorpus: string[] = [];
  const alignmentFeatureSurfaces: string[] = [];
  const alignmentBucketTokens: string[] = [];
  const alignmentValidationAnchors = new Set<string>();

  if (shouldCheckPrBody) {
    if (!prBody) {
      failures.push({
        id: 'runtime_closure_pr_body_missing',
        detail: 'pull_request.body is empty',
      });
    } else {
      const closureSection = sectionBody(prBody, 'Runtime Closure Feature Alignment');
      const closureTable = parseMarkdownTable(closureSection);
      if (!closureTable) {
        failures.push({
          id: 'runtime_closure_pr_alignment_table_missing',
          detail: 'Runtime Closure Feature Alignment table not found in PR body',
        });
      } else {
        if (closureTable.rows.length === 0) {
          failures.push({
            id: 'runtime_closure_pr_alignment_rows_missing',
            detail: 'Runtime Closure Feature Alignment table has no data rows',
          });
        }
        if (closureTable.rows.length > 40) {
          failures.push({
            id: 'runtime_closure_pr_alignment_rows_excessive',
            detail: String(closureTable.rows.length),
          });
        }
        const headers = closureTable.headers;
        const expectedAlignmentHeaders = [
          'Feature Surface',
          'Scope (`major`/`minor`)',
          'Runtime Closure Bucket',
          'Validation Artifact / Gate',
        ].map((header) => cleanText(header, 120).toLowerCase());
        const normalizedAlignmentHeaders = headers.map((header) => cleanText(header, 120).toLowerCase());
        const duplicateAlignmentHeaders = duplicateValues(normalizedAlignmentHeaders);
        if (duplicateAlignmentHeaders.length > 0) {
          failures.push({
            id: 'runtime_closure_pr_alignment_header_duplicate',
            detail: Array.from(new Set(duplicateAlignmentHeaders)).join(','),
          });
        }
        if (headers.length !== expectedAlignmentHeaders.length) {
          failures.push({
            id: 'runtime_closure_pr_alignment_header_count_invalid',
            detail: `actual=${headers.length};expected=${expectedAlignmentHeaders.length}`,
          });
        }
        if (
          normalizedAlignmentHeaders.length === expectedAlignmentHeaders.length
          && normalizedAlignmentHeaders.some((header, index) => header !== expectedAlignmentHeaders[index])
        ) {
          failures.push({
            id: 'runtime_closure_pr_alignment_headers_unexpected',
            detail: headers.join('|'),
          });
        }
        const surfaceIx = indexOfHeader(headers, 'Feature Surface');
        const scopeIx = indexOfHeader(headers, 'Scope (`major`/`minor`)');
        const bucketIx = indexOfHeader(headers, 'Runtime Closure Bucket');
        const validationIx = indexOfHeader(headers, 'Validation Artifact / Gate');
        if ([surfaceIx, scopeIx, bucketIx, validationIx].some((ix) => ix < 0)) {
          failures.push({
            id: 'runtime_closure_pr_alignment_headers_missing',
            detail: headers.join('|'),
          });
        } else {
          const seenFeatureSurfaces = new Set<string>();
          const seenFeatureSurfacesCasefold = new Set<string>();
          const seenFeatureSurfacesNormalized = new Set<string>();
          const expectedWidth = headers.length;
          for (const [rowIndex, row] of closureTable.rows.entries()) {
            if (row.length !== expectedWidth) {
              failures.push({
                id: 'runtime_closure_pr_alignment_row_shape_invalid',
                detail: `row=${rowIndex + 1};width=${row.length};expected=${expectedWidth}`,
              });
            }
            const featureSurface = cleanText(row[surfaceIx] || '', 200);
            const featureSurfaceCasefold = featureSurface.toLowerCase();
            const featureSurfaceNormalized = featureSurfaceCasefold.replace(/[^a-z0-9]+/g, ' ').trim();
            const scopeRaw = cleanText(row[scopeIx] || '', 80);
            const scope = scopeRaw.toLowerCase();
            const bucket = cleanText(row[bucketIx] || '', 120);
            const validation = cleanText(row[validationIx] || '', 260);
            const rowHasData = row.some((cell) => cleanText(cell || '', 240).length > 0);
            if (!featureSurface) {
              if (rowHasData) {
                failures.push({
                  id: 'runtime_closure_pr_alignment_feature_surface_missing',
                  detail: `row=${rowIndex + 1}`,
                });
              }
              continue;
            }
            alignmentRowsChecked += 1;
            alignmentFeatureSurfaces.push(featureSurface);
            if (seenFeatureSurfaces.has(featureSurface)) {
              failures.push({
                id: 'runtime_closure_pr_alignment_feature_surface_duplicate',
                detail: featureSurface,
              });
            }
            seenFeatureSurfaces.add(featureSurface);
            if (featureSurfaceCasefold) {
              if (seenFeatureSurfacesCasefold.has(featureSurfaceCasefold)) {
                failures.push({
                  id: 'runtime_closure_pr_alignment_feature_surface_duplicate_casefold',
                  detail: featureSurface,
                });
              }
              seenFeatureSurfacesCasefold.add(featureSurfaceCasefold);
            }
            if (featureSurfaceNormalized) {
              if (seenFeatureSurfacesNormalized.has(featureSurfaceNormalized)) {
                failures.push({
                  id: 'runtime_closure_pr_alignment_feature_surface_duplicate_normalized',
                  detail: featureSurface,
                });
              }
              seenFeatureSurfacesNormalized.add(featureSurfaceNormalized);
            }
            if (!scopeAllowlist.has(scope)) {
              failures.push({
                id: 'runtime_closure_pr_scope_invalid',
                detail: `${featureSurface}:${scope || 'missing'}`,
              });
            }
            if (scopeRaw && scopeRaw !== scope) {
              failures.push({
                id: 'runtime_closure_pr_scope_casing_invalid',
                detail: `${featureSurface}:${scopeRaw}`,
              });
            }
            if (featureSurface.length < 3) {
              failures.push({
                id: 'runtime_closure_pr_alignment_feature_surface_too_short',
                detail: featureSurface,
              });
            }
            if (wordCount(featureSurface) < 2) {
              failures.push({
                id: 'runtime_closure_pr_alignment_feature_surface_wordcount_low',
                detail: featureSurface,
              });
            }
            if (bucket && !hasLexicalOverlap(featureSurface, bucket.replace(/_/g, ' '))) {
              failures.push({
                id: 'runtime_closure_pr_alignment_feature_surface_bucket_overlap_missing',
                detail: `${featureSurface}:${bucket}`,
              });
            }
            if (
              featureSurface
              && !/^[a-z0-9]/i.test(featureSurface)
              || featureSurface
              && !/[a-z0-9]$/i.test(featureSurface)
            ) {
              failures.push({
                id: 'runtime_closure_pr_alignment_feature_surface_edge_punctuation',
                detail: featureSurface,
              });
            }
            if (placeholderToken(featureSurface)) {
              failures.push({
                id: 'runtime_closure_pr_feature_surface_placeholder',
                detail: featureSurface,
              });
            }
            if (bucket && !isCanonicalBucketToken(bucket)) {
              failures.push({
                id: 'runtime_closure_pr_bucket_noncanonical',
                detail: `${featureSurface}:${bucket}`,
              });
            }
            if (bucket && cleanText(bucket, 120).toLowerCase() === featureSurfaceCasefold) {
              failures.push({
                id: 'runtime_closure_pr_alignment_feature_surface_identical_bucket',
                detail: `${featureSurface}:${bucket}`,
              });
            }
            if (placeholderToken(bucket)) {
              failures.push({
                id: 'runtime_closure_pr_bucket_placeholder',
                detail: `${featureSurface}:${bucket}`,
              });
            }
            if (bucket) alignmentBucketTokens.push(bucket);
            if (placeholderToken(validation)) {
              failures.push({
                id: 'runtime_closure_pr_validation_placeholder',
                detail: `${featureSurface}:${validation}`,
              });
            }
            const validationAnchors = extractValidationAnchors(validation);
            for (const anchor of validationAnchors) alignmentValidationAnchors.add(anchor);
            const validationAnchorList = parseDelimitedTokens(validation)
              .filter((token) => token.startsWith('ops:') || token.startsWith('core/local/artifacts/'));
            const duplicateValidationAnchors = duplicateValues(validationAnchorList);
            if (duplicateValidationAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_validation_anchor_duplicate',
                detail: `${featureSurface}:${Array.from(new Set(duplicateValidationAnchors)).join(',')}`,
              });
            }
            const duplicateValidationAnchorsCasefold = casefoldDuplicateValues(validationAnchorList);
            if (duplicateValidationAnchorsCasefold.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_validation_anchor_duplicate_casefold',
                detail: `${featureSurface}:${Array.from(new Set(duplicateValidationAnchorsCasefold)).join(',')}`,
              });
            }
            const gateAnchors = validationAnchors.filter((anchor) => anchor.startsWith('ops:'));
            const artifactAnchors = validationAnchors.filter((anchor) =>
              anchor.startsWith('core/local/artifacts/'),
            );
            if (validationAnchors.length > 0 && validationAnchors.length < 2) {
              failures.push({
                id: 'runtime_closure_pr_validation_anchor_too_few',
                detail: `${featureSurface}:${validationAnchors.length}`,
              });
            }
            const sortedValidationAnchorList = sortedTokens(validationAnchorList);
            if (validationAnchorList.join('|') !== sortedValidationAnchorList.join('|')) {
              failures.push({
                id: 'runtime_closure_pr_validation_anchor_order_unsorted',
                detail: featureSurface,
              });
            }
            const invalidGateAnchors = gateAnchors.filter((anchor) => !isCanonicalGateToken(anchor));
            if (invalidGateAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_validation_gate_anchor_noncanonical',
                detail: `${featureSurface}:${invalidGateAnchors.join(',')}`,
              });
            }
            const invalidArtifactAnchors = artifactAnchors.filter(
              (anchor) => !isCanonicalArtifactToken(anchor),
            );
            if (invalidArtifactAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_validation_artifact_anchor_noncanonical',
                detail: `${featureSurface}:${invalidArtifactAnchors.join(',')}`,
              });
            }
            if (validation && validationAnchors.length === 0) {
              failures.push({
                id: 'runtime_closure_pr_validation_anchor_missing',
                detail: `${featureSurface}:${validation}`,
              });
            }
            if (validation && wordCount(validation) < 2) {
              failures.push({
                id: 'runtime_closure_pr_alignment_validation_too_short',
                detail: `${featureSurface}:${validation}`,
              });
            }
            if (validation && wordCount(validation) < 3) {
              failures.push({
                id: 'runtime_closure_pr_alignment_validation_too_short_strict',
                detail: `${featureSurface}:${validation}`,
              });
            }
            if (validation && cleanText(validation, 260).toLowerCase() === featureSurfaceCasefold) {
              failures.push({
                id: 'runtime_closure_pr_alignment_validation_identical_feature_surface',
                detail: featureSurface,
              });
            }
            if (validation && bucket && cleanText(validation, 260).toLowerCase() === bucket.toLowerCase()) {
              failures.push({
                id: 'runtime_closure_pr_alignment_validation_identical_bucket',
                detail: `${featureSurface}:${bucket}`,
              });
            }
            if (validationAnchors.length > 12) {
              failures.push({
                id: 'runtime_closure_pr_alignment_validation_anchor_excessive',
                detail: `${featureSurface}:${validationAnchors.length}`,
              });
            }
            const unknownValidationAnchors = validationAnchors.filter(
              (anchor) => !knownValidationTargets.has(anchor),
            );
            if (unknownValidationAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_validation_anchor_unknown',
                detail: `${featureSurface}:${unknownValidationAnchors.join(',')}`,
              });
            }
            if (scope.includes('major')) {
              majorRowsChecked += 1;
              majorFeatureSurfaces.push(featureSurface);
              if (!bucket) {
                failures.push({
                  id: 'runtime_closure_pr_major_bucket_missing',
                  detail: featureSurface,
                });
              } else if (!bucketIds.has(bucket)) {
                failures.push({
                  id: 'runtime_closure_pr_major_bucket_unknown',
                  detail: `${featureSurface}:${bucket}`,
                });
              } else {
                const priorOwner = majorBucketOwners.get(bucket);
                if (priorOwner && priorOwner !== featureSurface) {
                  failures.push({
                    id: 'runtime_closure_pr_major_bucket_duplicate',
                    detail: `${bucket}:${priorOwner},${featureSurface}`,
                  });
                } else if (!priorOwner) {
                  majorBucketOwners.set(bucket, featureSurface);
                }
              }
              if (!validation) {
                failures.push({
                  id: 'runtime_closure_pr_major_validation_missing',
                  detail: featureSurface,
                });
              } else if (validationAnchors.length === 0) {
                failures.push({
                  id: 'runtime_closure_pr_major_validation_unanchored',
                  detail: `${featureSurface}:${validation}`,
                });
              } else if (gateAnchors.length === 0) {
                failures.push({
                  id: 'runtime_closure_pr_major_validation_gate_anchor_missing',
                  detail: `${featureSurface}:${validation}`,
                });
              } else if (artifactAnchors.length === 0) {
                failures.push({
                  id: 'runtime_closure_pr_major_validation_artifact_anchor_missing',
                  detail: `${featureSurface}:${validation}`,
                });
              } else if (!Array.from(knownValidationTargets).some((token) => validation.includes(token))) {
                failures.push({
                  id: 'runtime_closure_pr_major_validation_unmapped',
                  detail: `${featureSurface}:${validation}`,
                });
              }
            } else if (scope === 'minor') {
              if (!bucket) {
                failures.push({
                  id: 'runtime_closure_pr_minor_bucket_missing',
                  detail: featureSurface,
                });
              } else if (!bucketIds.has(bucket)) {
                failures.push({
                  id: 'runtime_closure_pr_minor_bucket_unknown',
                  detail: `${featureSurface}:${bucket}`,
                });
              }
              if (!validation) {
                failures.push({
                  id: 'runtime_closure_pr_minor_validation_missing',
                  detail: featureSurface,
                });
              } else if (validationAnchors.length === 0) {
                failures.push({
                  id: 'runtime_closure_pr_minor_validation_unanchored',
                  detail: `${featureSurface}:${validation}`,
                });
              } else if (gateAnchors.length === 0) {
                failures.push({
                  id: 'runtime_closure_pr_minor_validation_gate_anchor_missing',
                  detail: `${featureSurface}:${validation}`,
                });
              } else if (artifactAnchors.length === 0) {
                failures.push({
                  id: 'runtime_closure_pr_minor_validation_artifact_anchor_missing',
                  detail: `${featureSurface}:${validation}`,
                });
              } else if (!Array.from(knownValidationTargets).some((token) => validation.includes(token))) {
                failures.push({
                  id: 'runtime_closure_pr_minor_validation_unmapped',
                  detail: `${featureSurface}:${validation}`,
                });
              }
            }
            if (bucket && validationAnchors.length > 0) {
              const expectedTargets = bucketToValidationTargets.get(bucket) || new Set<string>();
              if (expectedTargets.size > 0) {
                const hasBucketAlignedAnchor = validationAnchors.some((anchor) =>
                  expectedTargets.has(anchor),
                );
                if (!hasBucketAlignedAnchor) {
                  failures.push({
                    id: 'runtime_closure_pr_alignment_validation_anchor_bucket_mismatch',
                    detail: `${featureSurface}:${bucket}`,
                  });
                }
              }
            }
          }
        }
      }

      const capabilitySection = sectionBody(prBody, 'Capability Proof Burden');
      const capabilityTable = parseMarkdownTable(capabilitySection);
      if (!capabilityTable) {
        failures.push({
          id: 'runtime_closure_pr_capability_table_missing',
          detail: 'Capability Proof Burden table not found in PR body',
        });
      } else {
        if (capabilityTable.rows.length === 0) {
          failures.push({
            id: 'runtime_closure_pr_capability_rows_missing',
            detail: 'Capability Proof Burden table has no data rows',
          });
        }
        if (capabilityTable.rows.length > 60) {
          failures.push({
            id: 'runtime_closure_pr_capability_rows_excessive',
            detail: String(capabilityTable.rows.length),
          });
        }
        const capabilityIx = indexOfHeader(capabilityTable.headers, 'Capability');
        const proofIx = indexOfHeader(
          capabilityTable.headers,
          'Proof Artifact / Replay Fixture / Gate',
        );
        const invariantIx = indexOfHeader(capabilityTable.headers, 'Invariant');
        const failureModeIx = indexOfHeader(capabilityTable.headers, 'Failure Mode');
        const receiptIx = indexOfHeader(capabilityTable.headers, 'Receipt Surface');
        const recoveryIx = indexOfHeader(capabilityTable.headers, 'Recovery Behavior');
        const truthIx = indexOfHeader(capabilityTable.headers, 'Verifiable Runtime Truth Increase');
        const expectedCapabilityHeaders = [
          'Capability',
          'Proof Artifact / Replay Fixture / Gate',
          'Invariant',
          'Failure Mode',
          'Receipt Surface',
          'Recovery Behavior',
          'Verifiable Runtime Truth Increase',
        ].map((header) => cleanText(header, 120).toLowerCase());
        const normalizedCapabilityHeaders = capabilityTable.headers
          .map((header) => cleanText(header, 120).toLowerCase());
        const duplicateCapabilityHeaders = duplicateValues(normalizedCapabilityHeaders);
        if (duplicateCapabilityHeaders.length > 0) {
          failures.push({
            id: 'runtime_closure_pr_capability_header_duplicate',
            detail: Array.from(new Set(duplicateCapabilityHeaders)).join(','),
          });
        }
        if (capabilityTable.headers.length !== expectedCapabilityHeaders.length) {
          failures.push({
            id: 'runtime_closure_pr_capability_header_count_invalid',
            detail: `actual=${capabilityTable.headers.length};expected=${expectedCapabilityHeaders.length}`,
          });
        }
        if (
          normalizedCapabilityHeaders.length === expectedCapabilityHeaders.length
          && normalizedCapabilityHeaders.some((header, index) => header !== expectedCapabilityHeaders[index])
        ) {
          failures.push({
            id: 'runtime_closure_pr_capability_headers_unexpected',
            detail: capabilityTable.headers.join('|'),
          });
        }
        if (
          capabilityIx < 0
          || proofIx < 0
          || invariantIx < 0
          || failureModeIx < 0
          || receiptIx < 0
          || recoveryIx < 0
          || truthIx < 0
        ) {
          failures.push({
            id: 'runtime_closure_pr_capability_headers_missing',
            detail: capabilityTable.headers.join('|'),
          });
        } else {
          const seenCapabilities = new Set<string>();
          const seenCapabilitiesCasefold = new Set<string>();
          const seenCapabilitiesNormalized = new Set<string>();
          const seenCapabilitySignatures = new Map<string, string>();
          const expectedWidth = capabilityTable.headers.length;
          for (const [rowIndex, row] of capabilityTable.rows.entries()) {
            if (row.length !== expectedWidth) {
              failures.push({
                id: 'runtime_closure_pr_capability_row_shape_invalid',
                detail: `row=${rowIndex + 1};width=${row.length};expected=${expectedWidth}`,
              });
            }
            const capability = cleanText(row[capabilityIx] || '', 200);
            const capabilityCasefold = capability.toLowerCase();
            const capabilityNormalized = capabilityCasefold.replace(/[^a-z0-9]+/g, ' ').trim();
            const proof = cleanText(row[proofIx] || '', 260);
            const invariant = cleanText(row[invariantIx] || '', 200);
            const failureMode = cleanText(row[failureModeIx] || '', 200);
            const receiptSurface = cleanText(row[receiptIx] || '', 200);
            const recovery = cleanText(row[recoveryIx] || '', 200);
            const truthIncrease = cleanText(row[truthIx] || '', 200);
            const rowHasData = row.some((cell) => cleanText(cell || '', 240).length > 0);
            if (!capability) {
              if (rowHasData) {
                failures.push({
                  id: 'runtime_closure_pr_capability_missing_for_populated_row',
                  detail: `row=${rowIndex + 1}`,
                });
              }
              continue;
            }
            if (seenCapabilities.has(capability)) {
              failures.push({
                id: 'runtime_closure_pr_capability_duplicate',
                detail: capability,
              });
            }
            seenCapabilities.add(capability);
            if (capability.length < 3) {
              failures.push({
                id: 'runtime_closure_pr_capability_too_short',
                detail: capability,
              });
            }
            if (capabilityCasefold) {
              if (seenCapabilitiesCasefold.has(capabilityCasefold)) {
                failures.push({
                  id: 'runtime_closure_pr_capability_duplicate_casefold',
                  detail: capability,
                });
              }
              seenCapabilitiesCasefold.add(capabilityCasefold);
            }
            if (capabilityNormalized) {
              if (seenCapabilitiesNormalized.has(capabilityNormalized)) {
                failures.push({
                  id: 'runtime_closure_pr_capability_duplicate_normalized',
                  detail: capability,
                });
              }
              seenCapabilitiesNormalized.add(capabilityNormalized);
            }
            capabilityRowsChecked += 1;
            capabilityCorpus.push(
              [capability, proof, invariant, failureMode, receiptSurface, recovery, truthIncrease]
                .map((value) => cleanText(value, 300))
                .filter(Boolean)
                .join(' '),
            );
            if (
              alignmentFeatureSurfaces.length > 0
              && !alignmentFeatureSurfaces.some((feature) => hasLexicalOverlap(capability, feature))
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_feature_surface_overlap_missing',
                detail: capability,
              });
            }
            if (
              alignmentBucketTokens.length > 0
              && !alignmentBucketTokens.some((bucketToken) =>
                hasLexicalOverlap(capability, bucketToken.replace(/_/g, ' ')),
              )
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_bucket_overlap_missing',
                detail: capability,
              });
            }
            if (placeholderToken(capability)) {
              failures.push({
                id: 'runtime_closure_pr_capability_placeholder',
                detail: capability,
              });
            }
            if (
              capability
              && !/^[a-z0-9]/i.test(capability)
              || capability
              && !/[a-z0-9]$/i.test(capability)
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_edge_punctuation',
                detail: capability,
              });
            }
            const proofAnchors = extractValidationAnchors(proof);
            const proofAnchorList = parseDelimitedTokens(proof)
              .filter((token) => token.startsWith('ops:') || token.startsWith('core/local/artifacts/'));
            const duplicateProofAnchors = duplicateValues(proofAnchorList);
            if (duplicateProofAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_duplicate',
                detail: `${capability}:${Array.from(new Set(duplicateProofAnchors)).join(',')}`,
              });
            }
            const duplicateProofAnchorsCasefold = casefoldDuplicateValues(proofAnchorList);
            if (duplicateProofAnchorsCasefold.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_duplicate_casefold',
                detail: `${capability}:${Array.from(new Set(duplicateProofAnchorsCasefold)).join(',')}`,
              });
            }
            const proofGateAnchors = proofAnchors.filter((anchor) => anchor.startsWith('ops:'));
            const proofArtifactAnchors = proofAnchors.filter((anchor) =>
              anchor.startsWith('core/local/artifacts/'),
            );
            if (
              proofAnchors.length > 0
              && alignmentValidationAnchors.size > 0
              && !proofAnchors.some((anchor) => alignmentValidationAnchors.has(anchor))
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_alignment_overlap_missing',
                detail: `${capability}:${proof}`,
              });
            }
            if (proofAnchors.length > 0 && proofAnchors.length < 2) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_too_few',
                detail: `${capability}:${proofAnchors.length}`,
              });
            }
            const sortedProofAnchorList = sortedTokens(proofAnchorList);
            if (proofAnchorList.join('|') !== sortedProofAnchorList.join('|')) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_order_unsorted',
                detail: capability,
              });
            }
            const invalidProofGateAnchors = proofGateAnchors.filter(
              (anchor) => !isCanonicalGateToken(anchor),
            );
            if (invalidProofGateAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_gate_anchor_noncanonical',
                detail: `${capability}:${invalidProofGateAnchors.join(',')}`,
              });
            }
            const invalidProofArtifactAnchors = proofArtifactAnchors.filter(
              (anchor) => !isCanonicalArtifactToken(anchor),
            );
            if (invalidProofArtifactAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_artifact_anchor_noncanonical',
                detail: `${capability}:${invalidProofArtifactAnchors.join(',')}`,
              });
            }
            if (proof && proofAnchors.length === 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_missing',
                detail: `${capability}:${proof}`,
              });
            }
            if (proofAnchors.length > 12) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_excessive',
                detail: `${capability}:${proofAnchors.length}`,
              });
            }
            if (proof && proofAnchors.length > 0 && proofGateAnchors.length === 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_gate_anchor_missing',
                detail: `${capability}:${proof}`,
              });
            }
            if (proof && proofAnchors.length > 0 && proofArtifactAnchors.length === 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_artifact_anchor_missing',
                detail: `${capability}:${proof}`,
              });
            }
            const unknownProofAnchors = proofAnchors.filter((anchor) => !knownValidationTargets.has(anchor));
            if (unknownProofAnchors.length > 0) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_anchor_unknown',
                detail: `${capability}:${unknownProofAnchors.join(',')}`,
              });
            }
            if (!proof) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_link_missing',
                detail: capability,
              });
            } else if (!Array.from(knownValidationTargets).some((token) => proof.includes(token))) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_unmapped',
                detail: `${capability}:${proof}`,
              });
            }
            if (placeholderToken(proof)) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_placeholder',
                detail: capability,
              });
            }
            if (proof && wordCount(proof) < 2) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_too_short',
                detail: `${capability}:${proof}`,
              });
            }
            if (proof && cleanText(proof, 260).toLowerCase() === capabilityCasefold) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_identical_capability',
                detail: capability,
              });
            }
            if (proofAnchors.length > 0) {
              const hints = anchorLinkHints(proofAnchors);
              if (hints.length > 0) {
                const capabilityLower = capability.toLowerCase();
                const proofOverlap = hints.some((hint) => capabilityLower.includes(hint));
                if (!proofOverlap) {
                  failures.push({
                    id: 'runtime_closure_pr_capability_proof_capability_overlap_missing',
                    detail: `${capability}:${proof}`,
                  });
                }
              }
            }
            if (!invariant) {
              failures.push({
                id: 'runtime_closure_pr_capability_invariant_missing',
                detail: capability,
              });
            }
            if (placeholderToken(invariant)) {
              failures.push({
                id: 'runtime_closure_pr_capability_invariant_placeholder',
                detail: capability,
              });
            } else if (!hasSignal(invariant, ['must', 'always', 'never', 'fail-closed', 'deterministic', 'bounded'])) {
              failures.push({
                id: 'runtime_closure_pr_capability_invariant_signal_missing',
                detail: `${capability}:${invariant}`,
              });
            } else if (wordCount(invariant) < 3) {
              failures.push({
                id: 'runtime_closure_pr_capability_invariant_too_short',
                detail: `${capability}:${invariant}`,
              });
            }
            if (!failureMode) {
              failures.push({
                id: 'runtime_closure_pr_capability_failure_mode_missing',
                detail: capability,
              });
            }
            if (placeholderToken(failureMode)) {
              failures.push({
                id: 'runtime_closure_pr_capability_failure_mode_placeholder',
                detail: capability,
              });
            } else if (
              !hasSignal(failureMode, [
                'fail',
                'error',
                'timeout',
                'denied',
                'degraded',
                'stale',
                'drift',
                'overflow',
                'exhaust',
              ])
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_failure_mode_signal_missing',
                detail: `${capability}:${failureMode}`,
              });
            } else if (wordCount(failureMode) < 3) {
              failures.push({
                id: 'runtime_closure_pr_capability_failure_mode_too_short',
                detail: `${capability}:${failureMode}`,
              });
            }
            if (!receiptSurface) {
              failures.push({
                id: 'runtime_closure_pr_capability_receipt_surface_missing',
                detail: capability,
              });
            }
            if (placeholderToken(receiptSurface)) {
              failures.push({
                id: 'runtime_closure_pr_capability_receipt_surface_placeholder',
                detail: capability,
              });
            } else if (
              !hasSignal(receiptSurface, [
                'receipt',
                'artifact',
                'event',
                'log',
                'telemetry',
                'trace',
                'proof',
              ])
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_receipt_surface_signal_missing',
                detail: `${capability}:${receiptSurface}`,
              });
            } else if (wordCount(receiptSurface) < 3) {
              failures.push({
                id: 'runtime_closure_pr_capability_receipt_surface_too_short',
                detail: `${capability}:${receiptSurface}`,
              });
            }
            if (!recovery) {
              failures.push({
                id: 'runtime_closure_pr_capability_recovery_behavior_missing',
                detail: capability,
              });
            }
            if (placeholderToken(recovery)) {
              failures.push({
                id: 'runtime_closure_pr_capability_recovery_behavior_placeholder',
                detail: capability,
              });
            } else if (
              !hasSignal(recovery, [
                'retry',
                'rollback',
                'quarantine',
                'escalat',
                'fail-closed',
                'replay',
                'fallback',
                'backoff',
                'restart',
                'recover',
              ])
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_recovery_behavior_signal_missing',
                detail: `${capability}:${recovery}`,
              });
            } else if (wordCount(recovery) < 3) {
              failures.push({
                id: 'runtime_closure_pr_capability_recovery_behavior_too_short',
                detail: `${capability}:${recovery}`,
              });
            }
            if (!truthIncrease || placeholderToken(truthIncrease)) {
              failures.push({
                id: 'runtime_closure_pr_capability_truth_increase_missing',
                detail: capability,
              });
            }
            const truthRuntimeSignals = ['runtime', 'proof', 'receipt', 'gate', 'artifact', 'replay'];
            if (
              truthIncrease
              && !placeholderToken(truthIncrease)
              && !truthRuntimeSignals.some((token) => truthIncrease.toLowerCase().includes(token))
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_truth_increase_unverifiable',
                detail: `${capability}:${truthIncrease}`,
              });
            } else if (truthIncrease && !placeholderToken(truthIncrease) && wordCount(truthIncrease) < 3) {
              failures.push({
                id: 'runtime_closure_pr_capability_truth_increase_too_short',
                detail: `${capability}:${truthIncrease}`,
              });
            }
            if (
              truthIncrease
              && invariant
              && cleanText(truthIncrease, 200).toLowerCase() === cleanText(invariant, 200).toLowerCase()
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_truth_increase_identical_invariant',
                detail: capability,
              });
            }
            if (truthIncrease && proofAnchors.length > 0) {
              const truthHints = anchorLinkHints(proofAnchors);
              const lowerTruth = cleanText(truthIncrease, 400).toLowerCase();
              if (truthHints.length > 0 && !truthHints.some((hint) => lowerTruth.includes(hint))) {
                failures.push({
                  id: 'runtime_closure_pr_capability_truth_increase_proof_link_missing',
                  detail: `${capability}:${truthIncrease}`,
                });
              }
            }
            if (invariant && failureMode && cleanText(invariant, 200).toLowerCase() === cleanText(failureMode, 200).toLowerCase()) {
              failures.push({
                id: 'runtime_closure_pr_capability_invariant_failure_identical',
                detail: capability,
              });
            }
            if (failureMode && recovery && cleanText(failureMode, 200).toLowerCase() === cleanText(recovery, 200).toLowerCase()) {
              failures.push({
                id: 'runtime_closure_pr_capability_failure_recovery_identical',
                detail: capability,
              });
            }
            if (
              invariant
              && recovery
              && cleanText(invariant, 200).toLowerCase() === cleanText(recovery, 200).toLowerCase()
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_invariant_recovery_identical',
                detail: capability,
              });
            }
            if (
              invariant
              && truthIncrease
              && cleanText(invariant, 200).toLowerCase() === cleanText(truthIncrease, 200).toLowerCase()
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_invariant_truth_identical',
                detail: capability,
              });
            }
            if (
              failureMode
              && truthIncrease
              && cleanText(failureMode, 200).toLowerCase() === cleanText(truthIncrease, 200).toLowerCase()
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_failure_truth_identical',
                detail: capability,
              });
            }
            if (receiptSurface && proofAnchors.length > 0) {
              const hints = anchorLinkHints(proofAnchors);
              const lowerReceipt = cleanText(receiptSurface, 400).toLowerCase();
              if (hints.length > 0 && !hints.some((hint) => lowerReceipt.includes(hint))) {
                failures.push({
                  id: 'runtime_closure_pr_capability_receipt_surface_proof_link_missing',
                  detail: `${capability}:${receiptSurface}`,
                });
              }
            }
            if (
              receiptSurface
              && recovery
              && cleanText(receiptSurface, 200).toLowerCase() === cleanText(recovery, 200).toLowerCase()
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_receipt_recovery_identical',
                detail: capability,
              });
            }
            if (
              truthIncrease
              && recovery
              && cleanText(truthIncrease, 200).toLowerCase() === cleanText(recovery, 200).toLowerCase()
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_truth_recovery_identical',
                detail: capability,
              });
            }
            if (
              truthIncrease
              && receiptSurface
              && cleanText(truthIncrease, 200).toLowerCase() === cleanText(receiptSurface, 200).toLowerCase()
            ) {
              failures.push({
                id: 'runtime_closure_pr_capability_truth_receipt_identical',
                detail: capability,
              });
            }
            const signature = [
              cleanText(invariant, 200).toLowerCase(),
              cleanText(failureMode, 200).toLowerCase(),
              cleanText(receiptSurface, 200).toLowerCase(),
              cleanText(recovery, 200).toLowerCase(),
              cleanText(truthIncrease, 200).toLowerCase(),
            ].join('|');
            if (signature.replace(/\|/g, '').length > 0) {
              const priorCapability = seenCapabilitySignatures.get(signature);
              if (priorCapability && priorCapability !== capability) {
                failures.push({
                  id: 'runtime_closure_pr_capability_signature_duplicate',
                  detail: `${priorCapability},${capability}`,
                });
              } else if (!priorCapability) {
                seenCapabilitySignatures.set(signature, capability);
              }
            }
          }
        }
      }

      if (majorRowsChecked > 0 && capabilityRowsChecked === 0) {
        failures.push({
          id: 'runtime_closure_pr_capability_rows_missing_for_major_scope',
          detail: 'major_rows_present_without_capability_rows',
        });
      } else if (majorRowsChecked > capabilityRowsChecked) {
        failures.push({
          id: 'runtime_closure_pr_capability_rows_below_major_feature_count',
          detail: `major_rows=${majorRowsChecked};capability_rows=${capabilityRowsChecked}`,
        });
      }
      if (alignmentRowsChecked > 0 && capabilityRowsChecked === 0) {
        failures.push({
          id: 'runtime_closure_pr_alignment_rows_without_capability_rows',
          detail: `alignment_rows=${alignmentRowsChecked};capability_rows=${capabilityRowsChecked}`,
        });
      }
      if (capabilityRowsChecked > 0 && alignmentRowsChecked === 0) {
        failures.push({
          id: 'runtime_closure_pr_capability_rows_without_alignment_rows',
          detail: `capability_rows=${capabilityRowsChecked};alignment_rows=${alignmentRowsChecked}`,
        });
      }
      if (majorRowsChecked > alignmentRowsChecked) {
        failures.push({
          id: 'runtime_closure_pr_major_rows_exceed_alignment_rows',
          detail: `major_rows=${majorRowsChecked};alignment_rows=${alignmentRowsChecked}`,
        });
      }
      if (alignmentRowsChecked > 0 && capabilityRowsChecked > alignmentRowsChecked * 8) {
        failures.push({
          id: 'runtime_closure_pr_capability_rows_excessive_vs_alignment_rows',
          detail: `capability_rows=${capabilityRowsChecked};alignment_rows=${alignmentRowsChecked}`,
        });
      }
      if (capabilityRowsChecked > 0 && alignmentRowsChecked > capabilityRowsChecked * 8) {
        failures.push({
          id: 'runtime_closure_pr_alignment_rows_excessive_vs_capability_rows',
          detail: `alignment_rows=${alignmentRowsChecked};capability_rows=${capabilityRowsChecked}`,
        });
      }
      if (alignmentRowsChecked > 0 && alignmentValidationAnchors.size === 0) {
        failures.push({
          id: 'runtime_closure_pr_alignment_validation_anchor_set_empty_when_rows_present',
          detail: `alignment_rows=${alignmentRowsChecked};validation_anchors=${alignmentValidationAnchors.size}`,
        });
      }
      if (majorRowsChecked > 0 && capabilityCorpus.length > 0) {
        for (const majorFeature of majorFeatureSurfaces) {
          const linked = capabilityCorpus.some((entry) => hasLexicalOverlap(majorFeature, entry));
          if (!linked) {
            failures.push({
              id: 'runtime_closure_pr_major_feature_missing_capability_linkage',
              detail: majorFeature,
            });
          }
        }
      }
      if (majorRowsChecked > expectedBucketOrder.length) {
        failures.push({
          id: 'runtime_closure_pr_major_rows_exceed_bucket_capacity',
          detail: `major_rows=${majorRowsChecked};bucket_capacity=${expectedBucketOrder.length}`,
        });
      }
      if (majorRowsChecked > 0 && majorBucketOwners.size !== majorRowsChecked) {
        failures.push({
          id: 'runtime_closure_pr_major_bucket_owner_cardinality_mismatch',
          detail: `major_rows=${majorRowsChecked};unique_major_buckets=${majorBucketOwners.size}`,
        });
      }
    }
  }

  const payload = {
    ok: failures.length === 0,
    type: 'runtime_closure_feature_alignment_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    template_path: args.templatePath,
    board_path: args.boardPath,
    summary: {
      pass: failures.length === 0,
      github_event_name: eventName,
      pr_body_checked: shouldCheckPrBody,
      runtime_closure_bucket_count: bucketIds.size,
      major_rows_checked: majorRowsChecked,
      capability_rows_checked: capabilityRowsChecked,
      alignment_rows_checked: alignmentRowsChecked,
      template_marker_failures: failures.filter(
        (row) => row.id === 'runtime_closure_template_marker_missing',
      ).length,
      board_failures: failures.filter((row) => row.id.startsWith('runtime_closure_board_')).length,
      alignment_row_failures: failures.filter((row) =>
        row.id.startsWith('runtime_closure_pr_alignment_')
        || row.id.startsWith('runtime_closure_pr_major_')
        || row.id.startsWith('runtime_closure_pr_minor_')
        || row.id.startsWith('runtime_closure_pr_scope_')
        || row.id.startsWith('runtime_closure_pr_validation_')
        || row.id.startsWith('runtime_closure_pr_bucket_')
        || row.id.startsWith('runtime_closure_pr_feature_surface_'),
      ).length,
      major_row_failures: failures.filter((row) => row.id.includes('runtime_closure_pr_major_'))
        .length,
      capability_row_failures: failures.filter((row) =>
        row.id.includes('runtime_closure_pr_capability_'),
      ).length,
      failure_count: failures.length,
    },
    failures,
  };

  writeMarkdown(path.resolve(root, args.markdownPath), renderMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
