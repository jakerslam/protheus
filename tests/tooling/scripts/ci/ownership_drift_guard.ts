#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type ImportBoundaryRule = {
  id?: string;
  scan_roots?: string[];
  extensions?: string[];
  forbidden_import_patterns?: string[];
  allow_import_patterns?: string[];
};

type SymbolBoundaryRule = {
  id?: string;
  scan_roots?: string[];
  extensions?: string[];
  forbidden_symbol_patterns?: string[];
  allow_file_patterns?: string[];
};

type PathBoundaryRule = {
  id?: string;
  scan_roots?: string[];
  extensions?: string[];
  forbidden_path_patterns?: string[];
  allow_path_patterns?: string[];
};

type Policy = {
  version?: string;
  import_boundaries?: ImportBoundaryRule[];
  symbol_boundaries?: SymbolBoundaryRule[];
  path_boundaries?: PathBoundaryRule[];
};

type DriftViolation = {
  check_id: 'import' | 'symbol' | 'path';
  boundary_id: string;
  file: string;
  detail: string;
};

type Args = {
  strict: boolean;
  policyPath: string;
  outJsonPath: string;
  outMarkdownPath: string;
};

const ROOT = process.cwd();
const DEFAULT_POLICY_PATH = 'client/runtime/config/ownership_drift_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/ownership_drift_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/OWNERSHIP_DRIFT_GUARD_CURRENT.md';

function rel(p: string): string {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function parseArgs(argv: string[]): Args {
  const strictOut = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: strictOut.strict,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 400),
    outJsonPath: cleanText(readFlag(argv, 'out-json') || strictOut.out || DEFAULT_OUT_JSON, 400),
    outMarkdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function listFiles(roots: string[], extensions: string[]): string[] {
  const extSet = new Set(extensions.map((value) => cleanText(value, 32).toLowerCase()).filter(Boolean));
  const files: string[] = [];
  const stack = roots
    .map((row) => path.resolve(ROOT, cleanText(row, 400)))
    .filter((absRoot) => fs.existsSync(absRoot));
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(absPath);
        continue;
      }
      if (!entry.isFile()) continue;
      const ext = path.extname(entry.name).toLowerCase();
      if (extSet.has(ext)) files.push(absPath);
    }
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function parseImportSpecs(source: string): string[] {
  const specs: string[] = [];
  const importRegex =
    /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null = null;
  while ((match = importRegex.exec(source)) !== null) {
    specs.push(cleanText(String(match[1] || ''), 500));
  }
  return specs;
}

function matchesPattern(input: string, pattern: string): boolean {
  const normalized = cleanText(pattern, 500);
  if (!normalized) return false;
  if (normalized.startsWith('re:')) {
    try {
      return new RegExp(normalized.slice(3), 'm').test(input);
    } catch {
      return false;
    }
  }
  return input.includes(normalized);
}

function matchesAny(input: string, patterns: string[]): boolean {
  return patterns.some((pattern) => matchesPattern(input, pattern));
}

function duplicateValues(values: string[]): string[] {
  return values.filter((value, index, arr) => arr.indexOf(value) !== index);
}

function isCanonicalRelativePath(value: string): boolean {
  const normalized = cleanText(value || '', 400);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (normalized.endsWith('/')) return false;
  if (normalized.includes(' ')) return false;
  return true;
}

function patternsValid(patterns: string[]): boolean {
  for (const raw of patterns) {
    const value = cleanText(raw || '', 260);
    if (!value) return false;
    if (value !== value.trim()) return false;
    if (value.startsWith('re:')) {
      try {
        // eslint-disable-next-line no-new
        new RegExp(value.slice(3), 'm');
      } catch {
        return false;
      }
    }
  }
  return true;
}

function isCanonicalToken(value: string, max = 120): boolean {
  const normalized = cleanText(value || '', max);
  if (!normalized) return false;
  if (normalized !== normalized.trim()) return false;
  return /^[a-z0-9_]+$/.test(normalized);
}

function isCanonicalExtensionToken(value: string): boolean {
  const normalized = cleanText(value || '', 40).toLowerCase();
  if (!normalized) return false;
  return /^\.[a-z0-9][a-z0-9_-]*$/.test(normalized);
}

function runPolicyContracts(policy: Policy): DriftViolation[] {
  const violations: DriftViolation[] = [];
  const pushContract = (
    checkId: 'import' | 'symbol' | 'path',
    boundaryId: string,
    detail: string,
  ): void => {
    violations.push({
      check_id: checkId,
      boundary_id: boundaryId,
      file: DEFAULT_POLICY_PATH,
      detail: cleanText(detail, 260),
    });
  };

  const version = cleanText(policy.version || '', 80);
  if (!version) {
    pushContract(
      'path',
      'ownership_policy_version_present_contract',
      'policy.version must be declared',
    );
  } else if (!/^v?[0-9]+(?:\.[0-9]+){0,2}$/.test(version)) {
    pushContract(
      'path',
      'ownership_policy_version_canonical_contract',
      `policy.version must be canonical semver-like token (found: ${version})`,
    );
  }

  const importRules = Array.isArray(policy.import_boundaries) ? policy.import_boundaries : [];
  const symbolRules = Array.isArray(policy.symbol_boundaries) ? policy.symbol_boundaries : [];
  const pathRules = Array.isArray(policy.path_boundaries) ? policy.path_boundaries : [];

  if (importRules.length === 0) {
    pushContract(
      'import',
      'ownership_import_boundaries_nonempty_contract',
      'policy.import_boundaries must be non-empty',
    );
  }
  if (symbolRules.length === 0) {
    pushContract(
      'symbol',
      'ownership_symbol_boundaries_nonempty_contract',
      'policy.symbol_boundaries must be non-empty',
    );
  }
  if (pathRules.length === 0) {
    pushContract(
      'path',
      'ownership_path_boundaries_nonempty_contract',
      'policy.path_boundaries must be non-empty',
    );
  }

  const importIds = importRules.map((row) => cleanText(row.id || '', 120)).filter(Boolean);
  const symbolIds = symbolRules.map((row) => cleanText(row.id || '', 120)).filter(Boolean);
  const pathIds = pathRules.map((row) => cleanText(row.id || '', 120)).filter(Boolean);

  if (duplicateValues(importIds).length > 0) {
    pushContract(
      'import',
      'ownership_import_boundary_ids_unique_contract',
      'import boundary ids must be unique',
    );
  }
  if (duplicateValues(symbolIds).length > 0) {
    pushContract(
      'symbol',
      'ownership_symbol_boundary_ids_unique_contract',
      'symbol boundary ids must be unique',
    );
  }
  if (duplicateValues(pathIds).length > 0) {
    pushContract('path', 'ownership_path_boundary_ids_unique_contract', 'path boundary ids must be unique');
  }

  for (const [index, rule] of importRules.entries()) {
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    if (roots.length === 0 || roots.some((root) => !isCanonicalRelativePath(root))) {
      pushContract(
        'import',
        'ownership_import_boundary_scan_roots_nonempty_canonical_contract',
        `import boundary scan_roots must be canonical (index ${index})`,
      );
    }
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0 ? rule.extensions : ['.ts', '.tsx', '.rs'];
    if (extensions.some((token) => !isCanonicalExtensionToken(token))) {
      pushContract(
        'import',
        'ownership_import_boundary_extensions_token_contract',
        `import boundary extensions must be canonical (index ${index})`,
      );
    }
    const forbidden = Array.isArray(rule.forbidden_import_patterns) ? rule.forbidden_import_patterns : [];
    if (forbidden.length === 0 || !patternsValid(forbidden)) {
      pushContract(
        'import',
        'ownership_import_boundary_forbidden_patterns_nonempty_valid_contract',
        `import boundary forbidden_import_patterns must be non-empty and valid (index ${index})`,
      );
    }
    const allow = Array.isArray(rule.allow_import_patterns) ? rule.allow_import_patterns : [];
    if (allow.length > 0 && !patternsValid(allow)) {
      pushContract(
        'import',
        'ownership_import_boundary_allow_patterns_valid_contract',
        `import boundary allow_import_patterns must be valid when present (index ${index})`,
      );
    }
  }

  for (const [index, rule] of symbolRules.entries()) {
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    if (roots.length === 0 || roots.some((root) => !isCanonicalRelativePath(root))) {
      pushContract(
        'symbol',
        'ownership_symbol_boundary_scan_roots_nonempty_canonical_contract',
        `symbol boundary scan_roots must be canonical (index ${index})`,
      );
    }
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0 ? rule.extensions : ['.ts', '.tsx', '.rs'];
    if (extensions.some((token) => !isCanonicalExtensionToken(token))) {
      pushContract(
        'symbol',
        'ownership_symbol_boundary_extensions_token_contract',
        `symbol boundary extensions must be canonical (index ${index})`,
      );
    }
    const forbidden = Array.isArray(rule.forbidden_symbol_patterns) ? rule.forbidden_symbol_patterns : [];
    if (forbidden.length === 0 || !patternsValid(forbidden)) {
      pushContract(
        'symbol',
        'ownership_symbol_boundary_forbidden_patterns_nonempty_valid_contract',
        `symbol boundary forbidden_symbol_patterns must be non-empty and valid (index ${index})`,
      );
    }
    const allowFiles = Array.isArray(rule.allow_file_patterns) ? rule.allow_file_patterns : [];
    if (allowFiles.length > 0 && !patternsValid(allowFiles)) {
      pushContract(
        'symbol',
        'ownership_symbol_boundary_allow_files_valid_contract',
        `symbol boundary allow_file_patterns must be valid when present (index ${index})`,
      );
    }
  }

  for (const [index, rule] of pathRules.entries()) {
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    if (roots.length === 0 || roots.some((root) => !isCanonicalRelativePath(root))) {
      pushContract(
        'path',
        'ownership_path_boundary_scan_roots_nonempty_canonical_contract',
        `path boundary scan_roots must be canonical (index ${index})`,
      );
    }
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0 ? rule.extensions : ['.ts', '.tsx', '.rs'];
    if (extensions.some((token) => !isCanonicalExtensionToken(token))) {
      pushContract(
        'path',
        'ownership_path_boundary_extensions_token_contract',
        `path boundary extensions must be canonical (index ${index})`,
      );
    }
    const forbidden = Array.isArray(rule.forbidden_path_patterns) ? rule.forbidden_path_patterns : [];
    if (forbidden.length === 0 || !patternsValid(forbidden)) {
      pushContract(
        'path',
        'ownership_path_boundary_forbidden_patterns_nonempty_valid_contract',
        `path boundary forbidden_path_patterns must be non-empty and valid (index ${index})`,
      );
    }
    const allow = Array.isArray(rule.allow_path_patterns) ? rule.allow_path_patterns : [];
    if (allow.length > 0 && !patternsValid(allow)) {
      pushContract(
        'path',
        'ownership_path_boundary_allow_patterns_valid_contract',
        `path boundary allow_path_patterns must be valid when present (index ${index})`,
      );
    }
  }

  return violations;
}

function runImportBoundaries(policy: Policy): DriftViolation[] {
  const violations: DriftViolation[] = [];
  violations.push(...runPolicyContracts(policy));
  for (const rule of policy.import_boundaries || []) {
    const boundaryId = cleanText(rule.id || 'import_boundary', 120);
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0
        ? rule.extensions
        : ['.ts', '.tsx', '.rs'];
    const forbidden = Array.isArray(rule.forbidden_import_patterns)
      ? rule.forbidden_import_patterns
      : [];
    const allow = Array.isArray(rule.allow_import_patterns) ? rule.allow_import_patterns : [];
    const files = listFiles(roots, extensions);
    for (const filePath of files) {
      const source = fs.readFileSync(filePath, 'utf8');
      for (const specRaw of parseImportSpecs(source)) {
        const spec = specRaw.replace(/\\/g, '/');
        if (!matchesAny(spec, forbidden)) continue;
        if (allow.length > 0 && matchesAny(spec, allow)) continue;
        violations.push({
          check_id: 'import',
          boundary_id: boundaryId,
          file: rel(filePath),
          detail: spec,
        });
      }
    }
  }
  return violations;
}

function runSymbolBoundaries(policy: Policy): DriftViolation[] {
  const violations: DriftViolation[] = [];
  for (const rule of policy.symbol_boundaries || []) {
    const boundaryId = cleanText(rule.id || 'symbol_boundary', 120);
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0
        ? rule.extensions
        : ['.ts', '.tsx', '.rs'];
    const forbidden = Array.isArray(rule.forbidden_symbol_patterns)
      ? rule.forbidden_symbol_patterns
      : [];
    const allowFiles = Array.isArray(rule.allow_file_patterns) ? rule.allow_file_patterns : [];
    const files = listFiles(roots, extensions);
    for (const filePath of files) {
      const fileRel = rel(filePath);
      if (allowFiles.length > 0 && matchesAny(fileRel, allowFiles)) continue;
      const source = fs.readFileSync(filePath, 'utf8');
      for (const pattern of forbidden) {
        if (!matchesPattern(source, pattern)) continue;
        violations.push({
          check_id: 'symbol',
          boundary_id: boundaryId,
          file: fileRel,
          detail: cleanText(pattern, 260),
        });
      }
    }
  }
  return violations;
}

function runPathBoundaries(policy: Policy): DriftViolation[] {
  const violations: DriftViolation[] = [];
  for (const rule of policy.path_boundaries || []) {
    const boundaryId = cleanText(rule.id || 'path_boundary', 120);
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0
        ? rule.extensions
        : ['.ts', '.tsx', '.rs'];
    const forbidden = Array.isArray(rule.forbidden_path_patterns)
      ? rule.forbidden_path_patterns
      : [];
    const allow = Array.isArray(rule.allow_path_patterns) ? rule.allow_path_patterns : [];
    const files = listFiles(roots, extensions);
    for (const filePath of files) {
      const fileRel = rel(filePath);
      if (!matchesAny(fileRel, forbidden)) continue;
      if (allow.length > 0 && matchesAny(fileRel, allow)) continue;
      violations.push({
        check_id: 'path',
        boundary_id: boundaryId,
        file: fileRel,
        detail: 'forbidden_path_pattern_match',
      });
    }
  }
  return violations;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Ownership Drift Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Policy: ${payload.inputs.policy_path}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push('');
  lines.push(`- Path drift violations: ${payload.summary.path_violation_count}`);
  lines.push(`- Import drift violations: ${payload.summary.import_violation_count}`);
  lines.push(`- Symbol drift violations: ${payload.summary.symbol_violation_count}`);
  lines.push(`- Policy failures: ${payload.summary.policy_failure_count}`);
  lines.push(`- Total violations: ${payload.summary.total_violation_count}`);
  lines.push(`- Total issues: ${payload.summary.total_issue_count}`);
  lines.push('');
  lines.push('## Policy Failures');
  lines.push('');
  lines.push('| ID | Detail |');
  lines.push('| --- | --- |');
  const policyFailures = Array.isArray(payload.policy_failures) ? payload.policy_failures : [];
  if (policyFailures.length === 0) {
    lines.push('| (none) | - |');
  } else {
    for (const row of policyFailures.slice(0, 120)) {
      lines.push(`| ${String(row.id)} | ${String(row.detail).slice(0, 220)} |`);
    }
  }
  lines.push('');
  lines.push('## Violations');
  lines.push('');
  lines.push('| Check | Boundary | File | Detail |');
  lines.push('| --- | --- | --- | --- |');
  const rows = Array.isArray(payload.violations) ? payload.violations : [];
  if (rows.length === 0) {
    lines.push('| (none) | - | - | - |');
  } else {
    for (const row of rows.slice(0, 180)) {
      lines.push(
        `| ${String(row.check_id)} | ${String(row.boundary_id)} | ${String(
          row.file,
        )} | ${String(row.detail).slice(0, 180)} |`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const policyPath = path.resolve(ROOT, args.policyPath);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Policy;
  const policyFailures: Array<{ id: string; detail: string }> = [];

  const version = cleanText(policy.version || '', 64);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(version)) {
    policyFailures.push({
      id: 'ownership_drift_policy_version_invalid',
      detail: version || 'missing',
    });
  }

  const importBoundaries = Array.isArray(policy.import_boundaries) ? policy.import_boundaries : [];
  const symbolBoundaries = Array.isArray(policy.symbol_boundaries) ? policy.symbol_boundaries : [];
  const pathBoundaries = Array.isArray(policy.path_boundaries) ? policy.path_boundaries : [];

  if (importBoundaries.length === 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_import_boundaries_missing',
      detail: 'import_boundaries',
    });
  }
  if (symbolBoundaries.length === 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_symbol_boundaries_missing',
      detail: 'symbol_boundaries',
    });
  }
  if (pathBoundaries.length === 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_path_boundaries_missing',
      detail: 'path_boundaries',
    });
  }

  const ruleIdPattern = /^[a-z0-9_]+$/;
  const importIds = importBoundaries.map((rule) => cleanText(rule.id || '', 120)).filter(Boolean);
  const symbolIds = symbolBoundaries.map((rule) => cleanText(rule.id || '', 120)).filter(Boolean);
  const pathIds = pathBoundaries.map((rule) => cleanText(rule.id || '', 120)).filter(Boolean);
  const importIdDuplicates = duplicateValues(importIds);
  const symbolIdDuplicates = duplicateValues(symbolIds);
  const pathIdDuplicates = duplicateValues(pathIds);

  if (importIdDuplicates.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_import_boundary_ids_duplicate',
      detail: Array.from(new Set(importIdDuplicates)).join(','),
    });
  }
  if (symbolIdDuplicates.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_symbol_boundary_ids_duplicate',
      detail: Array.from(new Set(symbolIdDuplicates)).join(','),
    });
  }
  if (pathIdDuplicates.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_path_boundary_ids_duplicate',
      detail: Array.from(new Set(pathIdDuplicates)).join(','),
    });
  }

  const allIds = [...importIds, ...symbolIds, ...pathIds];
  const crossGroupDuplicates = duplicateValues(allIds);
  if (crossGroupDuplicates.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_boundary_ids_cross_group_duplicate',
      detail: Array.from(new Set(crossGroupDuplicates)).join(','),
    });
  }

  const importIdsNoncanonical = importIds.filter((value) => !ruleIdPattern.test(value));
  const symbolIdsNoncanonical = symbolIds.filter((value) => !ruleIdPattern.test(value));
  const pathIdsNoncanonical = pathIds.filter((value) => !ruleIdPattern.test(value));
  if (importIdsNoncanonical.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_import_boundary_id_noncanonical',
      detail: Array.from(new Set(importIdsNoncanonical)).join(','),
    });
  }
  if (symbolIdsNoncanonical.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_symbol_boundary_id_noncanonical',
      detail: Array.from(new Set(symbolIdsNoncanonical)).join(','),
    });
  }
  if (pathIdsNoncanonical.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_path_boundary_id_noncanonical',
      detail: Array.from(new Set(pathIdsNoncanonical)).join(','),
    });
  }

  const importScanRootsInvalid = importBoundaries
    .filter((rule) => {
      const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
      return roots.length === 0 || roots.some((root) => !isCanonicalRelativePath(cleanText(root || '', 400)));
    })
    .map((rule) => cleanText(rule.id || 'import_boundary', 120));
  const symbolScanRootsInvalid = symbolBoundaries
    .filter((rule) => {
      const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
      return roots.length === 0 || roots.some((root) => !isCanonicalRelativePath(cleanText(root || '', 400)));
    })
    .map((rule) => cleanText(rule.id || 'symbol_boundary', 120));
  const pathScanRootsInvalid = pathBoundaries
    .filter((rule) => {
      const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
      return roots.length === 0 || roots.some((root) => !isCanonicalRelativePath(cleanText(root || '', 400)));
    })
    .map((rule) => cleanText(rule.id || 'path_boundary', 120));
  if (importScanRootsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_import_scan_roots_noncanonical_or_missing',
      detail: Array.from(new Set(importScanRootsInvalid)).join(','),
    });
  }
  if (symbolScanRootsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_symbol_scan_roots_noncanonical_or_missing',
      detail: Array.from(new Set(symbolScanRootsInvalid)).join(','),
    });
  }
  if (pathScanRootsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_path_scan_roots_noncanonical_or_missing',
      detail: Array.from(new Set(pathScanRootsInvalid)).join(','),
    });
  }

  const extensionPattern = /^\.[a-z0-9]+$/;
  const extensionListValid = (extensions: string[]): boolean => {
    if (extensions.length === 0) return false;
    if (duplicateValues(extensions).length > 0) return false;
    return extensions.every((value) => extensionPattern.test(value) && value === value.toLowerCase());
  };
  const importExtensionsInvalid = importBoundaries
    .filter((rule) => !extensionListValid((Array.isArray(rule.extensions) ? rule.extensions : []).map((value) => cleanText(value || '', 32))))
    .map((rule) => cleanText(rule.id || 'import_boundary', 120));
  const symbolExtensionsInvalid = symbolBoundaries
    .filter((rule) => !extensionListValid((Array.isArray(rule.extensions) ? rule.extensions : []).map((value) => cleanText(value || '', 32))))
    .map((rule) => cleanText(rule.id || 'symbol_boundary', 120));
  const pathExtensionsInvalid = pathBoundaries
    .filter((rule) => !extensionListValid((Array.isArray(rule.extensions) ? rule.extensions : []).map((value) => cleanText(value || '', 32))))
    .map((rule) => cleanText(rule.id || 'path_boundary', 120));
  if (importExtensionsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_import_extensions_noncanonical',
      detail: Array.from(new Set(importExtensionsInvalid)).join(','),
    });
  }
  if (symbolExtensionsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_symbol_extensions_noncanonical',
      detail: Array.from(new Set(symbolExtensionsInvalid)).join(','),
    });
  }
  if (pathExtensionsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_path_extensions_noncanonical',
      detail: Array.from(new Set(pathExtensionsInvalid)).join(','),
    });
  }

  const importPatternsInvalid = importBoundaries
    .filter((rule) => {
      const forbidden = (Array.isArray(rule.forbidden_import_patterns) ? rule.forbidden_import_patterns : [])
        .map((value) => cleanText(value || '', 260));
      const allow = (Array.isArray(rule.allow_import_patterns) ? rule.allow_import_patterns : [])
        .map((value) => cleanText(value || '', 260));
      const overlap = forbidden.some((value) => allow.includes(value));
      return forbidden.length === 0 || !patternsValid(forbidden) || !patternsValid(allow) || overlap;
    })
    .map((rule) => cleanText(rule.id || 'import_boundary', 120));
  const symbolPatternsInvalid = symbolBoundaries
    .filter((rule) => {
      const forbidden = (Array.isArray(rule.forbidden_symbol_patterns) ? rule.forbidden_symbol_patterns : [])
        .map((value) => cleanText(value || '', 260));
      return forbidden.length === 0 || !patternsValid(forbidden);
    })
    .map((rule) => cleanText(rule.id || 'symbol_boundary', 120));
  const pathPatternsInvalid = pathBoundaries
    .filter((rule) => {
      const forbidden = (Array.isArray(rule.forbidden_path_patterns) ? rule.forbidden_path_patterns : [])
        .map((value) => cleanText(value || '', 260));
      const allow = (Array.isArray(rule.allow_path_patterns) ? rule.allow_path_patterns : [])
        .map((value) => cleanText(value || '', 260));
      return forbidden.length === 0 || !patternsValid(forbidden) || !patternsValid(allow);
    })
    .map((rule) => cleanText(rule.id || 'path_boundary', 120));
  if (importPatternsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_import_patterns_noncanonical_or_missing',
      detail: Array.from(new Set(importPatternsInvalid)).join(','),
    });
  }
  if (symbolPatternsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_symbol_patterns_noncanonical_or_missing',
      detail: Array.from(new Set(symbolPatternsInvalid)).join(','),
    });
  }
  if (pathPatternsInvalid.length > 0) {
    policyFailures.push({
      id: 'ownership_drift_policy_path_patterns_noncanonical_or_missing',
      detail: Array.from(new Set(pathPatternsInvalid)).join(','),
    });
  }

  const pathViolations = runPathBoundaries(policy);
  const importViolations = runImportBoundaries(policy);
  const symbolViolations = runSymbolBoundaries(policy);
  const violations = [...pathViolations, ...importViolations, ...symbolViolations];
  const allFailures = [
    ...policyFailures,
    ...violations.map((row) => ({
      id: `ownership_drift_${row.check_id}_violation`,
      detail: `${row.boundary_id}:${row.file}:${row.detail}`,
    })),
  ];
  const pass = violations.length === 0 && policyFailures.length === 0;

  const payload = {
    ok: pass,
    type: 'ownership_drift_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      policy_path: rel(policyPath),
      out_json: args.outJsonPath,
      out_markdown: args.outMarkdownPath,
    },
    summary: {
      pass,
      policy_failure_count: policyFailures.length,
      path_violation_count: pathViolations.length,
      import_violation_count: importViolations.length,
      symbol_violation_count: symbolViolations.length,
      total_violation_count: violations.length,
      total_issue_count: violations.length + policyFailures.length,
    },
    policy_failures: policyFailures,
    violations,
    failures: allFailures,
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdownPath), toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: path.resolve(ROOT, args.outJsonPath),
    strict: args.strict,
    ok: payload.ok,
  });
}

const exitCode = main();
if (exitCode !== 0) process.exit(exitCode);
