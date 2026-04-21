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
    '',
    '| module | status | category | matched_source_files | execution_unit_ok | boundary_ok |',
    '| --- | --- | --- | ---: | --- | --- |',
  ];
  for (const moduleRow of report.modules) {
    lines.push(
      `| ${moduleRow.id} | ${moduleRow.status} | ${moduleRow.category} | ${moduleRow.matched_source_files} | ${moduleRow.execution_unit_ok} | ${moduleRow.scheduler_boundary_ok} |`,
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
  const failures: Array<{ id: string; detail: string }> = [];

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

    return {
      id,
      path_prefix: prefix,
      category,
      status,
      owner,
      matched_source_files: matchedSources.length,
      execution_unit_ok: executionUnitOk,
      scheduler_boundary_ok: schedulerBoundaryOk,
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
