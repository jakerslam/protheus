#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/cross_layer_import_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/CROSS_LAYER_IMPORT_GUARD_CURRENT.md';
const SCAN_ROOTS = ['client', 'surface/orchestration', 'adapters'];
const EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.mjs', '.cjs']);
const ROOT_SPEC_PREFIXES = ['client/', 'core/', 'surface/', 'adapters/', 'tests/'];
const BOUNDARY_RULES = [
  {
    rule_id: 'client_orchestration_internal_import_forbidden',
    reason_id: 'shell_to_control_plane_internal_contract_violation',
    source_layer: 'shell',
    blocked_target: 'surface/orchestration internals',
    policy_reference: 'docs/workspace/orchestration_ownership_policy.md#shell',
  },
  {
    rule_id: 'orchestration_kernel_authority_import_forbidden',
    reason_id: 'control_plane_to_kernel_authority_violation',
    source_layer: 'control_plane',
    blocked_target: 'kernel policy/admission/scheduler/receipt authority',
    policy_reference: 'docs/workspace/orchestration_ownership_policy.md#control-plane',
  },
  {
    rule_id: 'adapter_scheduler_admission_import_forbidden',
    reason_id: 'gateway_to_kernel_or_control_plane_scheduler_authority_violation',
    source_layer: 'gateway',
    blocked_target: 'scheduler/admission authority',
    policy_reference: 'docs/workspace/orchestration_ownership_policy.md#gateways',
  },
] as const;
const IGNORED_DIR_NAMES = new Set([
  'node_modules',
  '.git',
  '.next',
  '.svelte-kit',
  'dist',
  'build',
  'coverage',
]);

type Violation = {
  rule_id: string;
  reason_id: string;
  file: string;
  spec: string;
  resolved_target: string;
  source_layer: string;
  target_layer: string;
  edge: string;
  policy_reference: string;
  detail: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

function parseArgs(argv: string[]): Args {
  const parsed = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: parsed.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || parsed.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function isCanonicalRelativePath(value: string): boolean {
  if (!value) return false;
  if (value.startsWith('/') || value.startsWith('\\')) return false;
  if (value.includes('..') || value.includes('\\') || value.includes('//')) return false;
  return /^[A-Za-z0-9._+/\-]+$/.test(value);
}

function hasCaseInsensitiveSuffix(value: string, suffix: string): boolean {
  return value.toLowerCase().endsWith(suffix.toLowerCase());
}

function isCanonicalToken(value: string): boolean {
  return /^[a-z0-9][a-z0-9_-]*$/.test(value);
}

function isCanonicalLayerToken(value: string): boolean {
  return ['shell', 'control_plane', 'kernel', 'gateway', 'other'].includes(value);
}

function isCanonicalEdgeToken(value: string): boolean {
  return /^(shell|control_plane|kernel|gateway|other)->(shell|control_plane|kernel|gateway|other)$/.test(value);
}

function walk(scanRoot: string): string[] {
  const rootPath = path.resolve(ROOT, scanRoot);
  if (!fs.existsSync(rootPath)) return [];
  const out: string[] = [];
  const stack = [rootPath];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        if (IGNORED_DIR_NAMES.has(entry.name) || entry.name.startsWith('.')) continue;
        stack.push(path.join(current, entry.name));
        continue;
      }
      if (!entry.isFile()) continue;
      const abs = path.join(current, entry.name);
      if (EXTENSIONS.has(path.extname(abs).toLowerCase())) out.push(abs);
    }
  }
  return out.sort();
}

function parseImportSpecs(source: string): string[] {
  const specs: string[] = [];
  const re = /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null = null;
  while ((match = re.exec(source)) != null) {
    specs.push(cleanText(match[1] || '', 600));
  }
  return specs.filter(Boolean);
}

function resolveRelativeImport(fromFile: string, spec: string): string | null {
  const base = path.resolve(path.dirname(fromFile), spec);
  const baseExt = path.extname(base).toLowerCase();
  const stem = baseExt ? base.slice(0, -baseExt.length) : base;
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.tsx`,
    `${base}.js`,
    `${base}.mjs`,
    `${base}.cjs`,
    path.join(base, 'index.ts'),
    path.join(base, 'index.tsx'),
    path.join(base, 'index.js'),
    path.join(base, 'index.mjs'),
    path.join(base, 'index.cjs'),
    stem,
    `${stem}.ts`,
    `${stem}.tsx`,
    `${stem}.js`,
    `${stem}.mjs`,
    `${stem}.cjs`,
    path.join(stem, 'index.ts'),
    path.join(stem, 'index.tsx'),
    path.join(stem, 'index.js'),
    path.join(stem, 'index.mjs'),
    path.join(stem, 'index.cjs'),
  ];
  for (const candidate of candidates) {
    if (!fs.existsSync(candidate)) continue;
    if (!fs.statSync(candidate).isFile()) continue;
    return rel(candidate);
  }
  return null;
}

function resolveImportTarget(fromFile: string, spec: string): string | null {
  if (!spec) return null;
  if (spec.startsWith('.')) {
    if (spec === './$types' || spec.endsWith('/$types')) return null;
    return resolveRelativeImport(fromFile, spec);
  }
  const normalized = spec.replace(/\\/g, '/');
  for (const prefix of ROOT_SPEC_PREFIXES) {
    if (normalized.startsWith(prefix)) return normalized;
  }
  return null;
}

function isOrchestrationContractPath(target: string): boolean {
  return (
    target === 'surface/orchestration/src/contracts.rs' ||
    target.startsWith('surface/orchestration/contracts/') ||
    target.startsWith('surface/orchestration/scripts/')
  );
}

function isClientImportingOrchestrationInternals(source: string, target: string): boolean {
  if (!source.startsWith('client/')) return false;
  if (!target.startsWith('surface/orchestration/')) return false;
  if (isOrchestrationContractPath(target)) return false;
  return target.startsWith('surface/orchestration/src/') || target.startsWith('surface/orchestration/tests/');
}

function isKernelPolicyAuthorityPath(target: string): boolean {
  if (!target.startsWith('core/')) return false;
  const lower = target.toLowerCase();
  return (
    lower.includes('/policy') ||
    lower.includes('policy_') ||
    lower.includes('/admission') ||
    lower.includes('admission_') ||
    lower.includes('/receipt') ||
    lower.includes('receipt_') ||
    lower.includes('/scheduler') ||
    lower.includes('scheduler_')
  );
}

function isOrchestrationImportingKernelPolicyAuthority(source: string, target: string): boolean {
  if (!source.startsWith('surface/orchestration/')) return false;
  return isKernelPolicyAuthorityPath(target);
}

function isAdaptersImportingSchedulerAdmissionAuthority(source: string, target: string): boolean {
  if (!source.startsWith('adapters/')) return false;
  if (!(target.startsWith('core/') || target.startsWith('surface/orchestration/src/'))) return false;
  const lower = target.toLowerCase();
  return (
    lower.includes('/scheduler') ||
    lower.includes('scheduler_') ||
    lower.includes('/admission') ||
    lower.includes('admission_')
  );
}

function mapOwnershipLayer(filePath: string): string {
  if (filePath.startsWith('client/')) return 'shell';
  if (filePath.startsWith('surface/orchestration/')) return 'control_plane';
  if (filePath.startsWith('core/')) return 'kernel';
  if (filePath.startsWith('adapters/')) return 'gateway';
  return 'other';
}

function toMarkdown(payload: {
  generated_at: string;
  revision: string;
  summary: {
    scanned_files: number;
    scanned_imports: number;
    violation_count: number;
    check_count: number;
    check_failure_count: number;
    pass: boolean;
  };
  checks: CheckRow[];
  violations: Violation[];
}): string {
  const lines: string[] = [];
  lines.push('# Cross-Layer Import Guard');
  lines.push('');
  lines.push(`- Generated: ${payload.generated_at}`);
  lines.push(`- Revision: ${payload.revision}`);
  lines.push(`- Pass: ${payload.summary.pass ? 'yes' : 'no'}`);
  lines.push(`- Scanned files: ${payload.summary.scanned_files}`);
  lines.push(`- Scanned imports: ${payload.summary.scanned_imports}`);
  lines.push(`- Checks: ${payload.summary.check_count}`);
  lines.push(`- Check failures: ${payload.summary.check_failure_count}`);
  lines.push(`- Violations: ${payload.summary.violation_count}`);
  lines.push('');
  if (payload.checks.length > 0) {
    lines.push('## Contract checks');
    lines.push('');
    for (const check of payload.checks) {
      lines.push(`- [${check.ok ? 'ok' : 'fail'}] ${check.id} :: ${check.detail}`);
    }
    lines.push('');
  }
  if (payload.violations.length === 0) {
    lines.push('No violations detected.');
    lines.push('');
    return `${lines.join('\n')}\n`;
  }
  lines.push('## Violations');
  lines.push('');
  for (const violation of payload.violations) {
    lines.push(
      `- [${violation.rule_id}] edge=${violation.edge} reason=${violation.reason_id} policy=${violation.policy_reference} :: ${violation.file} -> ${violation.spec} (${violation.resolved_target}) :: ${violation.detail}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const files = SCAN_ROOTS.flatMap((root) => walk(root)).sort((a, b) => a.localeCompare(b));
  const revision = currentRevision(ROOT);
  const violations: Violation[] = [];
  const edgeViolationCounts = new Map<string, number>();
  const reasonCounts = new Map<string, number>();
  let scannedImports = 0;
  let unresolvedRelativeImports = 0;
  const scanRootsUniqueCount = new Set(SCAN_ROOTS).size;
  const rootSpecPrefixesUniqueCount = new Set(ROOT_SPEC_PREFIXES).size;
  const extensionList = Array.from(EXTENSIONS).sort();
  const ignoredDirList = Array.from(IGNORED_DIR_NAMES).sort();

  for (const filePath of files) {
    const sourcePath = rel(filePath);
    const source = fs.readFileSync(filePath, 'utf8');
    const specs = parseImportSpecs(source);
    scannedImports += specs.length;
    for (const spec of specs) {
      const target = resolveImportTarget(filePath, spec);
      if (!target && spec.startsWith('.') && spec !== './$types' && !spec.endsWith('/$types')) {
        unresolvedRelativeImports += 1;
      }
      if (!target) continue;
      const sourceLayer = mapOwnershipLayer(sourcePath);
      const targetLayer = mapOwnershipLayer(target);
      const edge = `${sourceLayer}->${targetLayer}`;
      if (isClientImportingOrchestrationInternals(sourcePath, target)) {
        const reasonId = 'shell_to_control_plane_internal_contract_violation';
        violations.push({
          rule_id: 'client_orchestration_internal_import_forbidden',
          reason_id: reasonId,
          file: sourcePath,
          spec,
          resolved_target: target,
          source_layer: sourceLayer,
          target_layer: targetLayer,
          edge,
          policy_reference: 'docs/workspace/orchestration_ownership_policy.md#shell',
          detail: 'client imports orchestration internals (only contracts/scripts are allowed)',
        });
        edgeViolationCounts.set(edge, (edgeViolationCounts.get(edge) || 0) + 1);
        reasonCounts.set(reasonId, (reasonCounts.get(reasonId) || 0) + 1);
      }
      if (isOrchestrationImportingKernelPolicyAuthority(sourcePath, target)) {
        const reasonId = 'control_plane_to_kernel_authority_violation';
        violations.push({
          rule_id: 'orchestration_kernel_authority_import_forbidden',
          reason_id: reasonId,
          file: sourcePath,
          spec,
          resolved_target: target,
          source_layer: sourceLayer,
          target_layer: targetLayer,
          edge,
          policy_reference: 'docs/workspace/orchestration_ownership_policy.md#control-plane',
          detail: 'orchestration imports kernel policy/admission/scheduler/receipt authority path',
        });
        edgeViolationCounts.set(edge, (edgeViolationCounts.get(edge) || 0) + 1);
        reasonCounts.set(reasonId, (reasonCounts.get(reasonId) || 0) + 1);
      }
      if (isAdaptersImportingSchedulerAdmissionAuthority(sourcePath, target)) {
        const reasonId = 'gateway_to_kernel_or_control_plane_scheduler_authority_violation';
        violations.push({
          rule_id: 'adapter_scheduler_admission_import_forbidden',
          reason_id: reasonId,
          file: sourcePath,
          spec,
          resolved_target: target,
          source_layer: sourceLayer,
          target_layer: targetLayer,
          edge,
          policy_reference: 'docs/workspace/orchestration_ownership_policy.md#gateways',
          detail: 'gateway adapter imports scheduler/admission authority path',
        });
        edgeViolationCounts.set(edge, (edgeViolationCounts.get(edge) || 0) + 1);
        reasonCounts.set(reasonId, (reasonCounts.get(reasonId) || 0) + 1);
      }
    }
  }

  const filesSorted = files.every((filePath, index) => index === 0 || filePath.localeCompare(files[index - 1]) >= 0);
  const filesUnique = new Set(files).size === files.length;
  const scanRootsCanonical = SCAN_ROOTS.every((root) => isCanonicalRelativePath(root) && !root.endsWith('/'));
  const rootSpecPrefixesCanonical = ROOT_SPEC_PREFIXES.every((prefix) => isCanonicalRelativePath(prefix) && prefix.endsWith('/'));
  const extensionAllowlistCanonical = extensionList.every((ext) => /^\.[a-z0-9]+$/.test(ext));
  const ignoredDirNamesCanonical = ignoredDirList.every((dirName) => /^[A-Za-z0-9._-]+$/.test(dirName));
  const violationRuleIdsCanonical = violations.every((row) => isCanonicalToken(cleanText(row.rule_id || '', 160)));
  const violationReasonIdsCanonical = violations.every((row) => isCanonicalToken(cleanText(row.reason_id || '', 160)));
  const violationShapeCanonical = violations.every((row) => {
    const file = cleanText(row.file || '', 400);
    const target = cleanText(row.resolved_target || '', 400);
    const policy = cleanText(row.policy_reference || '', 400);
    return (
      isCanonicalLayerToken(cleanText(row.source_layer || '', 120)) &&
      isCanonicalLayerToken(cleanText(row.target_layer || '', 120)) &&
      isCanonicalEdgeToken(cleanText(row.edge || '', 120)) &&
      isCanonicalRelativePath(file) &&
      isCanonicalRelativePath(target) &&
      isCanonicalRelativePath(policy) &&
      hasCaseInsensitiveSuffix(policy, '.md') &&
      Boolean(cleanText(row.detail || '', 400))
    );
  });
  const outJsonArtifactsPrefixCanonical = args.outJson.startsWith('core/local/artifacts/');
  const outMarkdownReportsPrefixCanonical = args.outMarkdown.startsWith('local/workspace/reports/');
  const outMarkdownContractPathExact =
    args.outMarkdown === DEFAULT_OUT_MARKDOWN ||
    args.outMarkdown === 'local/workspace/reports/BOUNDARY_GUARD_CURRENT.md';
  const scanRootsExpectedOrder =
    SCAN_ROOTS.length === 3 &&
    SCAN_ROOTS[0] === 'client' &&
    SCAN_ROOTS[1] === 'surface/orchestration' &&
    SCAN_ROOTS[2] === 'adapters';
  const scanRootsExist = SCAN_ROOTS.every((root) => fs.existsSync(path.resolve(ROOT, root)));
  const rootSpecPrefixesCoverScanRoots = SCAN_ROOTS.every((root) =>
    ROOT_SPEC_PREFIXES.some((prefix) => `${root}/`.startsWith(prefix)),
  );
  const extensionAllowlistUnique = extensionList.length === EXTENSIONS.size;
  const extensionAllowlistContainsTsTsx =
    extensionList.includes('.ts') && extensionList.includes('.tsx');
  const extensionAllowlistNoWhitespace = extensionList.every((ext) => ext === ext.trim());
  const ignoredDirNamesNonempty = ignoredDirList.length > 0;
  const ignoredDirNamesUnique = ignoredDirList.length === IGNORED_DIR_NAMES.size;
  const ignoredDirNamesNoWhitespace = ignoredDirList.every((name) => name === name.trim());
  const scannedFilePathsCanonical = files.every((filePath) =>
    isCanonicalRelativePath(rel(filePath)),
  );
  const scannedFilePathsUnderScanRoots = files.every((filePath) => {
    const fileRel = rel(filePath);
    return SCAN_ROOTS.some((root) => fileRel.startsWith(`${root}/`));
  });
  const violationEdgeLayerConsistency = violations.every(
    (row) => `${row.source_layer}->${row.target_layer}` === row.edge,
  );
  const violationRuleReasonPairValid = violations.every((row) => {
    const expected = {
      client_orchestration_internal_import_forbidden:
        'shell_to_control_plane_internal_contract_violation',
      orchestration_kernel_authority_import_forbidden:
        'control_plane_to_kernel_authority_violation',
      adapter_scheduler_admission_import_forbidden:
        'gateway_to_kernel_or_control_plane_scheduler_authority_violation',
    }[cleanText(row.rule_id || '', 200)];
    return Boolean(expected) && expected === cleanText(row.reason_id || '', 200);
  });
  const violationPolicyReferenceWorkspacePrefix = violations.every((row) =>
    cleanText(row.policy_reference || '', 400).startsWith('docs/workspace/'),
  );
  const violationSpecNonempty = violations.every((row) => Boolean(cleanText(row.spec || '', 400)));
  const violationSourceLayerConsistency = violations.every(
    (row) => mapOwnershipLayer(cleanText(row.file || '', 400)) === row.source_layer,
  );
  const violationTargetLayerConsistency = violations.every(
    (row) => mapOwnershipLayer(cleanText(row.resolved_target || '', 400)) === row.target_layer,
  );

  const checks: CheckRow[] = [
    {
      id: 'cross_layer_import_guard_out_json_path_canonical_contract',
      ok: isCanonicalRelativePath(args.outJson),
      detail: args.outJson,
    },
    {
      id: 'cross_layer_import_guard_out_markdown_path_canonical_contract',
      ok: isCanonicalRelativePath(args.outMarkdown),
      detail: args.outMarkdown,
    },
    {
      id: 'cross_layer_import_guard_out_json_current_suffix_contract',
      ok: hasCaseInsensitiveSuffix(args.outJson, '_current.json'),
      detail: args.outJson,
    },
    {
      id: 'cross_layer_import_guard_out_markdown_current_suffix_contract',
      ok: hasCaseInsensitiveSuffix(args.outMarkdown, '_current.md'),
      detail: args.outMarkdown,
    },
    {
      id: 'cross_layer_import_guard_output_paths_distinct_contract',
      ok: args.outJson !== args.outMarkdown,
      detail: `${args.outJson}|${args.outMarkdown}`,
    },
    {
      id: 'cross_layer_import_guard_out_json_artifacts_prefix_contract',
      ok: outJsonArtifactsPrefixCanonical,
      detail: args.outJson,
    },
    {
      id: 'cross_layer_import_guard_out_markdown_reports_prefix_contract',
      ok: outMarkdownReportsPrefixCanonical,
      detail: args.outMarkdown,
    },
    {
      id: 'cross_layer_import_guard_out_markdown_contract_path_exact',
      ok: outMarkdownContractPathExact,
      detail: args.outMarkdown,
    },
    {
      id: 'cross_layer_import_guard_scan_roots_expected_order_contract',
      ok: scanRootsExpectedOrder,
      detail: SCAN_ROOTS.join(','),
    },
    {
      id: 'cross_layer_import_guard_scan_roots_exist_contract',
      ok: scanRootsExist,
      detail: SCAN_ROOTS.join(','),
    },
    {
      id: 'cross_layer_import_guard_root_spec_prefixes_cover_scan_roots_contract',
      ok: rootSpecPrefixesCoverScanRoots,
      detail: ROOT_SPEC_PREFIXES.join(','),
    },
    {
      id: 'cross_layer_import_guard_extension_allowlist_unique_contract',
      ok: extensionAllowlistUnique,
      detail: `count=${extensionList.length};unique=${EXTENSIONS.size}`,
    },
    {
      id: 'cross_layer_import_guard_extension_allowlist_contains_ts_tsx_contract',
      ok: extensionAllowlistContainsTsTsx,
      detail: extensionList.join(','),
    },
    {
      id: 'cross_layer_import_guard_extension_allowlist_no_whitespace_contract',
      ok: extensionAllowlistNoWhitespace,
      detail: extensionList.join(','),
    },
    {
      id: 'cross_layer_import_guard_ignored_dir_names_nonempty_contract',
      ok: ignoredDirNamesNonempty,
      detail: `count=${ignoredDirList.length}`,
    },
    {
      id: 'cross_layer_import_guard_ignored_dir_names_unique_contract',
      ok: ignoredDirNamesUnique,
      detail: `count=${ignoredDirList.length};unique=${IGNORED_DIR_NAMES.size}`,
    },
    {
      id: 'cross_layer_import_guard_ignored_dir_names_no_whitespace_contract',
      ok: ignoredDirNamesNoWhitespace,
      detail: ignoredDirList.join(','),
    },
    {
      id: 'cross_layer_import_guard_scanned_file_paths_canonical_contract',
      ok: scannedFilePathsCanonical,
      detail: `count=${files.length}`,
    },
    {
      id: 'cross_layer_import_guard_scanned_file_paths_under_scan_roots_contract',
      ok: scannedFilePathsUnderScanRoots,
      detail: SCAN_ROOTS.join(','),
    },
    {
      id: 'cross_layer_import_guard_violation_edge_layer_consistency_contract',
      ok: violationEdgeLayerConsistency,
      detail: `count=${violations.length}`,
    },
    {
      id: 'cross_layer_import_guard_violation_rule_reason_pair_contract',
      ok: violationRuleReasonPairValid,
      detail: `count=${violations.length}`,
    },
    {
      id: 'boundary_guard_client_orchestration_internal_rule_published',
      ok: BOUNDARY_RULES.some((row) => row.rule_id === 'client_orchestration_internal_import_forbidden'),
      detail: 'client_orchestration_internal_import_forbidden',
    },
    {
      id: 'boundary_guard_orchestration_kernel_authority_rule_published',
      ok: BOUNDARY_RULES.some((row) => row.rule_id === 'orchestration_kernel_authority_import_forbidden'),
      detail: 'orchestration_kernel_authority_import_forbidden',
    },
    {
      id: 'boundary_guard_gateway_scheduler_admission_rule_published',
      ok: BOUNDARY_RULES.some((row) => row.rule_id === 'adapter_scheduler_admission_import_forbidden'),
      detail: 'adapter_scheduler_admission_import_forbidden',
    },
    {
      id: 'cross_layer_import_guard_violation_policy_reference_workspace_prefix_contract',
      ok: violationPolicyReferenceWorkspacePrefix,
      detail: `count=${violations.length}`,
    },
    {
      id: 'cross_layer_import_guard_violation_spec_nonempty_contract',
      ok: violationSpecNonempty,
      detail: `count=${violations.length}`,
    },
    {
      id: 'cross_layer_import_guard_violation_source_layer_consistency_contract',
      ok: violationSourceLayerConsistency,
      detail: `count=${violations.length}`,
    },
    {
      id: 'cross_layer_import_guard_violation_target_layer_consistency_contract',
      ok: violationTargetLayerConsistency,
      detail: `count=${violations.length}`,
    },
    {
      id: 'cross_layer_import_guard_scan_roots_nonempty_contract',
      ok: SCAN_ROOTS.length > 0,
      detail: `count=${SCAN_ROOTS.length}`,
    },
    {
      id: 'cross_layer_import_guard_scan_roots_unique_contract',
      ok: scanRootsUniqueCount === SCAN_ROOTS.length,
      detail: `count=${SCAN_ROOTS.length};unique=${scanRootsUniqueCount}`,
    },
    {
      id: 'cross_layer_import_guard_scan_roots_canonical_contract',
      ok: scanRootsCanonical,
      detail: SCAN_ROOTS.join(','),
    },
    {
      id: 'cross_layer_import_guard_extension_allowlist_nonempty_contract',
      ok: extensionList.length > 0,
      detail: `count=${extensionList.length}`,
    },
    {
      id: 'cross_layer_import_guard_extension_allowlist_token_contract',
      ok: extensionAllowlistCanonical,
      detail: extensionList.join(','),
    },
    {
      id: 'cross_layer_import_guard_root_spec_prefixes_nonempty_contract',
      ok: ROOT_SPEC_PREFIXES.length > 0,
      detail: `count=${ROOT_SPEC_PREFIXES.length}`,
    },
    {
      id: 'cross_layer_import_guard_root_spec_prefixes_unique_contract',
      ok: rootSpecPrefixesUniqueCount === ROOT_SPEC_PREFIXES.length,
      detail: `count=${ROOT_SPEC_PREFIXES.length};unique=${rootSpecPrefixesUniqueCount}`,
    },
    {
      id: 'cross_layer_import_guard_root_spec_prefixes_canonical_contract',
      ok: rootSpecPrefixesCanonical,
      detail: ROOT_SPEC_PREFIXES.join(','),
    },
    {
      id: 'cross_layer_import_guard_ignored_dir_names_token_contract',
      ok: ignoredDirNamesCanonical,
      detail: ignoredDirList.join(','),
    },
    {
      id: 'cross_layer_import_guard_scanned_files_sorted_contract',
      ok: filesSorted,
      detail: `count=${files.length}`,
    },
    {
      id: 'cross_layer_import_guard_scanned_files_unique_contract',
      ok: filesUnique,
      detail: `count=${files.length};unique=${new Set(files).size}`,
    },
    {
      id: 'cross_layer_import_guard_unresolved_relative_imports_zero_contract',
      ok: unresolvedRelativeImports === 0,
      detail: `count=${unresolvedRelativeImports}`,
    },
    {
      id: 'cross_layer_import_guard_violation_rule_id_token_contract',
      ok: violationRuleIdsCanonical,
      detail: `count=${violations.length}`,
    },
    {
      id: 'cross_layer_import_guard_violation_reason_id_token_contract',
      ok: violationReasonIdsCanonical,
      detail: `count=${violations.length}`,
    },
    {
      id: 'cross_layer_import_guard_violation_shape_contract',
      ok: violationShapeCanonical,
      detail: `count=${violations.length}`,
    },
  ];

  const checkFailureCount = checks.filter((row) => !row.ok).length;
  const overallPass = violations.length === 0 && checkFailureCount === 0;

  const payload = {
    type: 'cross_layer_import_guard',
    generated_at: new Date().toISOString(),
    revision,
    summary: {
      scanned_files: files.length,
      scanned_imports: scannedImports,
      check_count: checks.length,
      check_failure_count: checkFailureCount,
      violation_count: violations.length,
      pass: overallPass,
    },
    checks,
    edge_violation_counts: Object.fromEntries(
      Array.from(edgeViolationCounts.entries()).sort((a, b) => a[0].localeCompare(b[0])),
    ),
    violation_reason_counts: Object.fromEntries(
      Array.from(reasonCounts.entries()).sort((a, b) => a[0].localeCompare(b[0])),
    ),
    boundary_rules: BOUNDARY_RULES,
    violations,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: overallPass,
  });
}

process.exit(main());
