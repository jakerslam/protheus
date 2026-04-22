#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type Layer3PolicyModule = {
  id: string;
  path_prefix: string;
  category: string;
  status: string;
  owner: string;
  timeout_semantics: string;
  retry_semantics: string;
  receipt_requirements: string[];
  parity_test_path: string;
  scheduler_boundary?: {
    layer2_interface?: string;
    authority?: string;
  };
  boundary_alignment?: {
    layer2_authority_boundary?: string;
    layer3_scope?: string;
    gateway_boundary?: string;
  };
  execution_unit?: {
    id?: string;
    lifecycle?: string[];
    budget?: Record<string, unknown>;
    dependencies?: string[];
    receipts?: string[];
  };
};

type Layer3Policy = {
  schema_id?: string;
  schema_version?: string;
  layer3_root?: string;
  source_extensions?: string[];
  allowed_categories?: string[];
  allowed_statuses?: string[];
  placement_boundaries?: {
    layer2?: {
      owns?: string[];
      forbidden_in_layer3?: string[];
    };
    layer3?: {
      owns?: string[];
      must_consume_layer2_receipts?: boolean;
    };
    gateways?: {
      owns?: string[];
      forbidden_in_layer3?: string[];
    };
  };
  require_module_boundary_alignment?: boolean;
  reject_module_without_boundary_alignment?: boolean;
  fail_on_dependency_boundary_violation?: boolean;
  allowed_dependency_prefixes?: string[];
  forbidden_dependency_prefixes?: string[];
  fail_on_unmapped_source_file?: boolean;
  modules?: Layer3PolicyModule[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/layer3_contract_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out-json') || readFlag(argv, 'out') || common.out || '', 400),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/LAYER3_CONTRACT_GUARD_CURRENT.md',
      400,
    ),
    policyPath: cleanText(
      readFlag(argv, 'policy') || 'tests/tooling/config/layer3_contract_policy.json',
      400,
    ),
  };
}

function readJsonStrict(filePath: string): any {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function walkFiles(rootPath: string, out: string[] = []): string[] {
  if (!fs.existsSync(rootPath)) return out;
  for (const entry of fs.readdirSync(rootPath, { withFileTypes: true })) {
    if (entry.name === '.git' || entry.name === 'target' || entry.name === 'node_modules') continue;
    const abs = path.join(rootPath, entry.name);
    if (entry.isDirectory()) {
      walkFiles(abs, out);
    } else if (entry.isFile()) {
      out.push(abs);
    }
  }
  return out;
}

function rel(root: string, filePath: string): string {
  return cleanText(path.relative(root, filePath).replace(/\\/g, '/'), 500);
}

function fileMatchesModule(filePath: string, modulePathPrefix: string): boolean {
  const normalizedFile = cleanText(filePath.replace(/\\/g, '/'), 500);
  const normalizedPrefix = cleanText(modulePathPrefix.replace(/\\/g, '/'), 500);
  return normalizedFile === normalizedPrefix || normalizedFile.startsWith(`${normalizedPrefix}/`);
}

function isNonEmptyString(value: unknown): boolean {
  return typeof value === 'string' && cleanText(value, 200).length > 0;
}

function toMarkdown(report: any): string {
  const lines = [
    '# Layer3 Contract Guard',
    '',
    `- policy: ${report.policy_path}`,
    `- layer3_root: ${report.layer3_root}`,
    `- overall_status: ${report.summary.overall_status}`,
    `- module_count: ${report.summary.module_count}`,
    `- source_file_count: ${report.summary.source_file_count}`,
    `- unmapped_source_file_count: ${report.summary.unmapped_source_file_count}`,
    `- placement_policy_failure_count: ${report.summary.placement_policy_failure_count}`,
    `- boundary_alignment_failure_count: ${report.summary.boundary_alignment_failure_count}`,
    `- dependency_boundary_failure_count: ${report.summary.dependency_boundary_failure_count}`,
    '',
    '| module | status | category | matched_source_files | execution_unit_ok | scheduler_boundary_ok | boundary_alignment_ok | dependency_boundary_ok |',
    '| --- | --- | --- | ---: | --- | --- | --- | --- |',
  ];
  for (const moduleRow of report.modules) {
    lines.push(
      `| ${moduleRow.id} | ${moduleRow.status} | ${moduleRow.category} | ${moduleRow.matched_source_files} | ${moduleRow.execution_unit_ok} | ${moduleRow.scheduler_boundary_ok} | ${moduleRow.boundary_alignment_ok} | ${moduleRow.dependency_boundary_ok} |`,
    );
  }
  lines.push('');
  lines.push('## Unmapped Source Files');
  if (Array.isArray(report.unmapped_source_files) && report.unmapped_source_files.length > 0) {
    for (const filePath of report.unmapped_source_files) {
      lines.push(`- ${filePath}`);
    }
  } else {
    lines.push('- none');
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  let policy: Layer3Policy | null = null;
  try {
    policy = readJsonStrict(path.resolve(root, args.policyPath)) as Layer3Policy;
  } catch (error) {
    const payload = {
      ok: false,
      type: 'layer3_contract_guard',
      error: 'layer3_contract_policy_unavailable',
      detail: cleanText(error instanceof Error ? error.message : String(error), 320),
      policy_path: args.policyPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const layer3Root = cleanText(policy.layer3_root || 'core/layer3', 300);
  const layer3RootAbs = path.resolve(root, layer3Root);
  const sourceExtensions =
    Array.isArray(policy.source_extensions) && policy.source_extensions.length > 0
      ? policy.source_extensions.map((value) => cleanText(String(value), 20).toLowerCase())
      : ['.rs', '.toml'];
  const allowedCategories = new Set(
    (Array.isArray(policy.allowed_categories) ? policy.allowed_categories : []).map((value) =>
      cleanText(String(value), 60),
    ),
  );
  const allowedStatuses = new Set(
    (Array.isArray(policy.allowed_statuses) ? policy.allowed_statuses : []).map((value) =>
      cleanText(String(value), 60),
    ),
  );
  const modules = Array.isArray(policy.modules) ? policy.modules : [];
  const placementBoundaries = policy.placement_boundaries || {};
  const requireModuleBoundaryAlignment = policy.require_module_boundary_alignment !== false;
  const rejectModuleWithoutBoundaryAlignment = policy.reject_module_without_boundary_alignment !== false;
  const failOnDependencyBoundaryViolation = policy.fail_on_dependency_boundary_violation !== false;
  const allowedDependencyPrefixes =
    Array.isArray(policy.allowed_dependency_prefixes) && policy.allowed_dependency_prefixes.length > 0
      ? policy.allowed_dependency_prefixes.map((value) => cleanText(String(value || ''), 200)).filter(Boolean)
      : ['core/layer2/', 'core/layer3/'];
  const forbiddenDependencyPrefixes =
    Array.isArray(policy.forbidden_dependency_prefixes) && policy.forbidden_dependency_prefixes.length > 0
      ? policy.forbidden_dependency_prefixes.map((value) => cleanText(String(value || ''), 200)).filter(Boolean)
      : ['adapters/', 'surface/orchestration/', 'client/'];
  const failures: Array<{ id: string; detail: string }> = [];

  const placementLayer2Owns = Array.isArray(placementBoundaries.layer2?.owns)
    ? placementBoundaries.layer2?.owns || []
    : [];
  const placementLayer3Owns = Array.isArray(placementBoundaries.layer3?.owns)
    ? placementBoundaries.layer3?.owns || []
    : [];
  const placementGatewaysOwns = Array.isArray(placementBoundaries.gateways?.owns)
    ? placementBoundaries.gateways?.owns || []
    : [];
  if (placementLayer2Owns.length === 0) {
    failures.push({ id: 'layer3_policy_placement_layer2_owns_missing', detail: 'placement_boundaries.layer2.owns' });
  }
  if (placementLayer3Owns.length === 0) {
    failures.push({ id: 'layer3_policy_placement_layer3_owns_missing', detail: 'placement_boundaries.layer3.owns' });
  }
  if (placementGatewaysOwns.length === 0) {
    failures.push({
      id: 'layer3_policy_placement_gateways_owns_missing',
      detail: 'placement_boundaries.gateways.owns',
    });
  }
  if (placementBoundaries.layer3?.must_consume_layer2_receipts !== true) {
    failures.push({
      id: 'layer3_policy_must_consume_layer2_receipts_false',
      detail: 'placement_boundaries.layer3.must_consume_layer2_receipts',
    });
  }

  const sourceFiles = walkFiles(layer3RootAbs)
    .map((abs) => rel(root, abs))
    .filter((relativePath) => sourceExtensions.some((ext) => relativePath.endsWith(ext)));

  const moduleRows = modules.map((moduleRow) => {
    const id = cleanText(moduleRow.id || '', 120);
    const prefix = cleanText(moduleRow.path_prefix || '', 300);
    const category = cleanText(moduleRow.category || '', 60);
    const status = cleanText(moduleRow.status || '', 60);
    const owner = cleanText(moduleRow.owner || '', 80);
    const matchedSources = sourceFiles.filter((filePath) => fileMatchesModule(filePath, prefix));

    if (!id) failures.push({ id: 'layer3_module_id_missing', detail: prefix || 'unknown_prefix' });
    if (!prefix) failures.push({ id: 'layer3_module_path_prefix_missing', detail: id || 'unknown_module' });
    if (!allowedCategories.has(category)) {
      failures.push({ id: 'layer3_module_category_invalid', detail: `${id}:${category}` });
    }
    if (!allowedStatuses.has(status)) {
      failures.push({ id: 'layer3_module_status_invalid', detail: `${id}:${status}` });
    }
    if (!isNonEmptyString(owner)) {
      failures.push({ id: 'layer3_module_owner_missing', detail: id });
    }
    if (!isNonEmptyString(moduleRow.timeout_semantics)) {
      failures.push({ id: 'layer3_module_timeout_semantics_missing', detail: id });
    }
    if (!isNonEmptyString(moduleRow.retry_semantics)) {
      failures.push({ id: 'layer3_module_retry_semantics_missing', detail: id });
    }
    if (!Array.isArray(moduleRow.receipt_requirements) || moduleRow.receipt_requirements.length === 0) {
      failures.push({ id: 'layer3_module_receipt_requirements_missing', detail: id });
    }
    if (!isNonEmptyString(moduleRow.parity_test_path)) {
      failures.push({ id: 'layer3_module_parity_test_path_missing', detail: id });
    }
    if (matchedSources.length === 0) {
      failures.push({ id: 'layer3_module_no_source_match', detail: id });
    }

    const schedulerBoundary = moduleRow.scheduler_boundary || {};
    const schedulerBoundaryOk =
      isNonEmptyString(schedulerBoundary.layer2_interface) && isNonEmptyString(schedulerBoundary.authority);
    if (!schedulerBoundaryOk) {
      failures.push({ id: 'layer3_module_scheduler_boundary_invalid', detail: id });
    }
    if (schedulerBoundary.authority !== 'layer2_owns_scheduling') {
      failures.push({
        id: 'layer3_module_scheduler_authority_must_be_layer2',
        detail: `${id}:${cleanText(schedulerBoundary.authority || '', 80)}`,
      });
    }
    if (!String(schedulerBoundary.layer2_interface || '').toLowerCase().includes('layer2')) {
      failures.push({
        id: 'layer3_module_scheduler_interface_not_layer2_scoped',
        detail: `${id}:${cleanText(schedulerBoundary.layer2_interface || '', 120)}`,
      });
    }

    const boundaryAlignment = moduleRow.boundary_alignment || {};
    const boundaryAlignmentOk =
      isNonEmptyString(boundaryAlignment.layer2_authority_boundary) &&
      isNonEmptyString(boundaryAlignment.layer3_scope) &&
      isNonEmptyString(boundaryAlignment.gateway_boundary);
    if (requireModuleBoundaryAlignment && rejectModuleWithoutBoundaryAlignment && !boundaryAlignmentOk) {
      failures.push({ id: 'layer3_module_boundary_alignment_missing', detail: id });
    }

    const executionUnit = moduleRow.execution_unit || {};
    const executionUnitOk =
      isNonEmptyString(executionUnit.id) &&
      Array.isArray(executionUnit.lifecycle) &&
      executionUnit.lifecycle.length > 0 &&
      !!executionUnit.budget &&
      typeof executionUnit.budget === 'object' &&
      Array.isArray(executionUnit.dependencies) &&
      executionUnit.dependencies.length > 0 &&
      Array.isArray(executionUnit.receipts) &&
      executionUnit.receipts.length > 0;
    if (!executionUnitOk) {
      failures.push({ id: 'layer3_module_execution_unit_invalid', detail: id });
    }

    const dependencyRows = Array.isArray(executionUnit.dependencies)
      ? executionUnit.dependencies.map((value) => cleanText(String(value || ''), 300)).filter(Boolean)
      : [];
    const dependencyBoundaryViolations = dependencyRows.filter((dep) => {
      const normalized = dep.replace(/\\/g, '/');
      if (forbiddenDependencyPrefixes.some((prefix) => normalized.startsWith(prefix))) return true;
      return !allowedDependencyPrefixes.some((prefix) => normalized.startsWith(prefix));
    });
    const dependencyBoundaryOk = dependencyBoundaryViolations.length === 0;
    if (failOnDependencyBoundaryViolation && !dependencyBoundaryOk) {
      for (const violation of dependencyBoundaryViolations) {
        failures.push({ id: 'layer3_module_dependency_boundary_violation', detail: `${id}:${violation}` });
      }
    }

    return {
      id,
      path_prefix: prefix,
      category,
      status,
      owner,
      matched_source_files: matchedSources.length,
      execution_unit_ok: executionUnitOk,
      scheduler_boundary_ok: schedulerBoundaryOk,
      boundary_alignment_ok: boundaryAlignmentOk,
      dependency_boundary_ok: dependencyBoundaryOk,
    };
  });

  const modulePrefixes = modules
    .map((moduleRow) => cleanText(moduleRow.path_prefix || '', 300))
    .filter(Boolean);
  const unmappedSourceFiles = sourceFiles.filter(
    (filePath) => !modulePrefixes.some((prefix) => fileMatchesModule(filePath, prefix)),
  );
  if ((policy.fail_on_unmapped_source_file ?? true) && unmappedSourceFiles.length > 0) {
    for (const filePath of unmappedSourceFiles) {
      failures.push({ id: 'layer3_unmapped_source_file', detail: filePath });
    }
  }

  const report = {
    ok: failures.length === 0,
    type: 'layer3_contract_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    policy_path: args.policyPath,
    layer3_root: layer3Root,
    summary: {
      pass: failures.length === 0,
      overall_status: failures.length === 0 ? 'pass' : 'fail',
      module_count: moduleRows.length,
      source_file_count: sourceFiles.length,
      unmapped_source_file_count: unmappedSourceFiles.length,
      placement_policy_failure_count: failures.filter((row) => row.id.startsWith('layer3_policy_')).length,
      boundary_alignment_failure_count: failures.filter(
        (row) => row.id === 'layer3_module_boundary_alignment_missing',
      ).length,
      dependency_boundary_failure_count: failures.filter(
        (row) => row.id === 'layer3_module_dependency_boundary_violation',
      ).length,
      failed_count: failures.length,
    },
    modules: moduleRows,
    source_files: sourceFiles,
    unmapped_source_files: unmappedSourceFiles,
    failures,
  };

  if (args.markdownOutPath) {
    writeTextArtifact(args.markdownOutPath, toMarkdown(report));
  }

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
