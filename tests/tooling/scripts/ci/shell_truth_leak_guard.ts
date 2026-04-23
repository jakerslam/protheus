#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision, trackedFiles } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY_PATH = 'client/runtime/config/shell_truth_leak_policy.json';
const DEFAULT_CONTRACT_PATH = 'client/runtime/config/shell_backend_state_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_truth_leak_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_TRUTH_LEAK_GUARD_CURRENT.md';

type PatternPolicy = {
  id: string;
  severity: 'error' | 'warn';
  description: string;
  regex: string;
};

type RequiredContractFile = {
  path: string;
  must_include: string[];
};

type ShellTruthLeakPolicy = {
  version?: string;
  scan_roots?: string[];
  scan_extensions?: string[];
  ignore_path_contains?: string[];
  required_pattern_ids?: string[];
  error_on_warning_pattern_ids?: string[];
  forbidden_patterns?: PatternPolicy[];
  allowed_pattern_paths?: Record<string, string[]>;
};

type ShellBackendStateContract = {
  version?: string;
  backend_state?: {
    fields?: string[];
    enum_values?: string[];
    enum_evidence_file?: string;
  };
  freshness?: {
    fields?: string[];
  };
  required_contract_files?: RequiredContractFile[];
};

const AUTHORITY_AUDIT_CATEGORIES: Record<string, string[]> = {
  retry_logic: ['client_retry_policy_branching'],
  health_inference: [
    'sidebar_status_state_fallback',
    'sidebar_status_label_fallback',
    'client_runtime_state_policy_branching',
  ],
  lane_selection: ['client_lane_selection_heuristic'],
  policy_branching: ['client_runtime_state_policy_branching'],
};

type Args = {
  policyPath: string;
  contractPath: string;
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

function rel(value: string): string {
  return path.relative(ROOT, value).replace(/\\/g, '/');
}

function parseArgs(argv: string[]): Args {
  const strictOut = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 600),
    contractPath: cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT_PATH, 600),
    strict: strictOut.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || strictOut.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
  };
}

function readJsonFile<T>(relativePath: string): T {
  const abs = path.resolve(ROOT, relativePath);
  return JSON.parse(fs.readFileSync(abs, 'utf8')) as T;
}

function safeRegex(pattern: string): RegExp | null {
  try {
    return new RegExp(pattern, 'g');
  } catch {
    return null;
  }
}

function fileMatchesScanRoots(file: string, scanRoots: string[]): boolean {
  if (!scanRoots.length) return true;
  return scanRoots.some((root) => file === root || file.startsWith(`${root}/`));
}

function fileMatchesIgnoredPatterns(file: string, ignored: string[]): boolean {
  return ignored.some((needle) => needle && file.includes(needle));
}

function formatMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Truth-Leak Guard');
  lines.push('');
  lines.push(`- Generated at: ${payload.generated_at}`);
  lines.push(`- Revision: ${payload.revision}`);
  lines.push(`- Strict mode: ${payload.strict ? 'true' : 'false'}`);
  lines.push(`- Pass: ${payload.ok ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- scanned_files: ${payload.summary.scanned_files}`);
  lines.push(`- contract_files_checked: ${payload.summary.contract_files_checked}`);
  lines.push(`- required_field_coverage_failures: ${payload.summary.required_field_coverage_failures}`);
  lines.push(`- enum_evidence_failures: ${payload.summary.enum_evidence_failures}`);
  lines.push(`- contract_file_failures: ${payload.summary.contract_file_failures}`);
  lines.push(`- invalid_regex_patterns: ${payload.summary.invalid_regex_patterns}`);
  lines.push(`- missing_scan_roots: ${payload.summary.missing_scan_roots}`);
  lines.push(`- duplicate_forbidden_pattern_ids: ${payload.summary.duplicate_forbidden_pattern_ids}`);
  lines.push(`- duplicate_required_pattern_ids: ${payload.summary.duplicate_required_pattern_ids}`);
  lines.push(`- duplicate_error_on_warning_pattern_ids: ${payload.summary.duplicate_error_on_warning_pattern_ids}`);
  lines.push(`- duplicate_scan_roots: ${payload.summary.duplicate_scan_roots}`);
  lines.push(`- duplicate_scan_extensions: ${payload.summary.duplicate_scan_extensions}`);
  lines.push(`- duplicate_ignore_path_needles: ${payload.summary.duplicate_ignore_path_needles}`);
  lines.push(`- unknown_error_on_warning_pattern_ids: ${payload.summary.unknown_error_on_warning_pattern_ids}`);
  lines.push(`- invalid_pattern_id_formats: ${payload.summary.invalid_pattern_id_formats}`);
  lines.push(`- invalid_required_pattern_id_formats: ${payload.summary.invalid_required_pattern_id_formats}`);
  lines.push(`- invalid_error_on_warning_pattern_id_formats: ${payload.summary.invalid_error_on_warning_pattern_id_formats}`);
  lines.push(`- required_pattern_ids_without_error_enforcement: ${payload.summary.required_pattern_ids_without_error_enforcement}`);
  lines.push(`- unknown_allowed_pattern_ids: ${payload.summary.unknown_allowed_pattern_ids}`);
  lines.push(`- missing_allowed_pattern_path_files: ${payload.summary.missing_allowed_pattern_path_files}`);
  lines.push(`- invalid_allowed_pattern_path_formats: ${payload.summary.invalid_allowed_pattern_path_formats}`);
  lines.push(`- allowed_pattern_paths_outside_scan_roots: ${payload.summary.allowed_pattern_paths_outside_scan_roots}`);
  lines.push(`- allowed_pattern_paths_under_ignored_needles: ${payload.summary.allowed_pattern_paths_under_ignored_needles}`);
  lines.push(`- missing_required_pattern_ids: ${payload.summary.missing_required_pattern_ids}`);
  lines.push(`- error_violations: ${payload.summary.error_violations}`);
  lines.push(`- warning_violations: ${payload.summary.warning_violations}`);
  lines.push(`- pattern_hits: ${payload.summary.pattern_hits}`);
  lines.push(`- authority_retry_logic_violations: ${payload.summary.authority_retry_logic_violations}`);
  lines.push(`- authority_health_inference_violations: ${payload.summary.authority_health_inference_violations}`);
  lines.push(`- authority_lane_selection_violations: ${payload.summary.authority_lane_selection_violations}`);
  lines.push(`- authority_policy_branching_violations: ${payload.summary.authority_policy_branching_violations}`);
  lines.push('');
  lines.push('## Authority Audit (RTG-019)');
  lines.push('| category | violations | matches | files |');
  lines.push('| --- | ---: | ---: | ---: |');
  for (const row of payload.authority_audit || []) {
    lines.push(`| ${row.category} | ${row.violations} | ${row.matches} | ${row.files} |`);
  }
  if (!(payload.authority_audit || []).length) {
    lines.push('| (none) | 0 | 0 | 0 |');
  }
  lines.push('');
  lines.push('## Missing Scan Roots');
  if (!(payload.missing_scan_roots || []).length) {
    lines.push('- none');
  } else {
    for (const row of payload.missing_scan_roots) {
      lines.push(`- ${row}`);
    }
  }
  lines.push('');
  lines.push('## Duplicate Pattern ID Buckets');
  lines.push(`- forbidden: ${(payload.duplicate_forbidden_pattern_ids || []).join(', ') || 'none'}`);
  lines.push(`- required: ${(payload.duplicate_required_pattern_ids || []).join(', ') || 'none'}`);
  lines.push(`- error_on_warning: ${(payload.duplicate_error_on_warning_pattern_ids || []).join(', ') || 'none'}`);
  lines.push(`- scan_roots: ${(payload.duplicate_scan_roots || []).join(', ') || 'none'}`);
  lines.push(`- scan_extensions: ${(payload.duplicate_scan_extensions || []).join(', ') || 'none'}`);
  lines.push(`- ignore_path_needles: ${(payload.duplicate_ignore_path_needles || []).join(', ') || 'none'}`);
  lines.push('');
  lines.push('## Pattern ID Governance');
  lines.push(`- unknown_error_on_warning_pattern_ids: ${(payload.unknown_error_on_warning_pattern_ids || []).join(', ') || 'none'}`);
  lines.push(`- invalid_pattern_id_formats: ${(payload.invalid_pattern_id_formats || []).join(', ') || 'none'}`);
  lines.push(`- invalid_required_pattern_id_formats: ${(payload.invalid_required_pattern_id_formats || []).join(', ') || 'none'}`);
  lines.push(`- invalid_error_on_warning_pattern_id_formats: ${(payload.invalid_error_on_warning_pattern_id_formats || []).join(', ') || 'none'}`);
  lines.push(`- required_pattern_ids_without_error_enforcement: ${(payload.required_pattern_ids_without_error_enforcement || []).join(', ') || 'none'}`);
  lines.push('');
  lines.push('## Allowed Pattern Path Coverage');
  lines.push(`- unknown_pattern_ids: ${(payload.unknown_allowed_pattern_ids || []).join(', ') || 'none'}`);
  lines.push(`- invalid_path_formats: ${(payload.invalid_allowed_pattern_path_formats || []).join(', ') || 'none'}`);
  lines.push(`- outside_scan_roots: ${(payload.allowed_pattern_paths_outside_scan_roots || []).join(', ') || 'none'}`);
  lines.push(`- under_ignored_needles: ${(payload.allowed_pattern_paths_under_ignored_needles || []).join(', ') || 'none'}`);
  if (!(payload.missing_allowed_pattern_path_files || []).length) {
    lines.push('- missing_path_files: none');
  } else {
    for (const row of payload.missing_allowed_pattern_path_files) {
      lines.push(`- missing_path_file: ${row.pattern_id} -> ${row.path}`);
    }
  }
  lines.push('');
  lines.push('## Required Field Coverage');
  lines.push('| field | category | hits | ok |');
  lines.push('| --- | --- | ---: | --- |');
  for (const row of payload.required_field_coverage || []) {
    lines.push(`| ${row.field} | ${row.category} | ${row.hits} | ${row.ok ? 'true' : 'false'} |`);
  }
  if (!(payload.required_field_coverage || []).length) {
    lines.push('| (none) | - | 0 | true |');
  }
  lines.push('');
  lines.push('## Enum Evidence');
  lines.push('| value | hits | ok |');
  lines.push('| --- | ---: | --- |');
  for (const row of payload.enum_evidence || []) {
    lines.push(`| ${row.value} | ${row.hits} | ${row.ok ? 'true' : 'false'} |`);
  }
  if (!(payload.enum_evidence || []).length) {
    lines.push('| (none) | 0 | true |');
  }
  lines.push('');
  lines.push('## Contract File Failures');
  if (!(payload.contract_file_failures || []).length) {
    lines.push('- none');
  } else {
    for (const row of payload.contract_file_failures) {
      lines.push(`- ${row.file}: ${row.reason}${row.missing_tokens?.length ? ` (${row.missing_tokens.join(', ')})` : ''}`);
    }
  }
  lines.push('');
  lines.push('## Invalid Regex Patterns');
  if (!(payload.invalid_regex_patterns || []).length) {
    lines.push('- none');
  } else {
    for (const row of payload.invalid_regex_patterns) {
      lines.push(`- ${row.pattern_id}: ${row.regex}`);
    }
  }
  lines.push('');
  lines.push('## Missing Required Pattern IDs');
  if (!(payload.missing_required_pattern_ids || []).length) {
    lines.push('- none');
  } else {
    for (const row of payload.missing_required_pattern_ids) {
      lines.push(`- ${row}`);
    }
  }
  lines.push('');
  lines.push('## Error Violations');
  if (!(payload.error_violations || []).length) {
    lines.push('- none');
  } else {
    for (const row of payload.error_violations) {
      lines.push(`- ${row.pattern_id} @ ${row.file} (matches=${row.matches})`);
    }
  }
  lines.push('');
  lines.push('## Warning Violations');
  if (!(payload.warning_violations || []).length) {
    lines.push('- none');
  } else {
    for (const row of payload.warning_violations.slice(0, 80)) {
      lines.push(`- ${row.pattern_id} @ ${row.file} (matches=${row.matches})`);
    }
  }
  lines.push('');
  lines.push('## Pattern Hits');
  lines.push('| pattern_id | severity | files | matches |');
  lines.push('| --- | --- | ---: | ---: |');
  for (const row of payload.pattern_hits || []) {
    lines.push(`| ${row.pattern_id} | ${row.severity} | ${row.files} | ${row.matches} |`);
  }
  if (!(payload.pattern_hits || []).length) {
    lines.push('| (none) | - | 0 | 0 |');
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const policy = readJsonFile<ShellTruthLeakPolicy>(args.policyPath);
  const contract = readJsonFile<ShellBackendStateContract>(args.contractPath);

  const scanRoots = (Array.isArray(policy.scan_roots) ? policy.scan_roots : [])
    .map((v) => String(v).replace(/\\/g, '/'));
  const duplicateScanRoots = Array.from(
    scanRoots.reduce((acc, root) => {
      const normalized = String(root || '').trim();
      if (!normalized) return acc;
      acc.set(normalized, (acc.get(normalized) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([root, count]) => `${root}:${count}`);
  const missingScanRoots = scanRoots
    .filter((root) => root.trim().length > 0)
    .filter((root) => !fs.existsSync(path.resolve(ROOT, root)));
  const scanExtensionRows = (Array.isArray(policy.scan_extensions) ? policy.scan_extensions : ['.ts', '.js'])
    .map((v) => String(v).trim().toLowerCase())
    .filter((v) => v.length > 0);
  const duplicateScanExtensions = Array.from(
    scanExtensionRows.reduce((acc, extension) => {
      acc.set(extension, (acc.get(extension) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([extension, count]) => `${extension}:${count}`);
  const scanExtensions = new Set(scanExtensionRows);
  const ignoredNeedles = (Array.isArray(policy.ignore_path_contains) ? policy.ignore_path_contains : [])
    .map((v) => String(v).trim())
    .filter(Boolean);
  const duplicateIgnorePathNeedles = Array.from(
    ignoredNeedles.reduce((acc, needle) => {
      acc.set(needle, (acc.get(needle) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([needle, count]) => `${needle}:${count}`);
  const patterns = (Array.isArray(policy.forbidden_patterns) ? policy.forbidden_patterns : []);
  const requiredPatternIds = (Array.isArray(policy.required_pattern_ids) ? policy.required_pattern_ids : [])
    .map((v) => String(v || '').trim())
    .filter(Boolean);
  const errorOnWarningPatternIdRows = (Array.isArray(policy.error_on_warning_pattern_ids)
    ? policy.error_on_warning_pattern_ids
    : [])
    .map((v) => String(v || '').trim())
    .filter(Boolean);
  const errorOnWarningPatternIds = new Set(errorOnWarningPatternIdRows);
  const allowedPatternPaths = policy.allowed_pattern_paths || {};

  const tracked = trackedFiles(ROOT);
  const scannedFiles = tracked
    .filter((file) => fileMatchesScanRoots(file, scanRoots))
    .filter((file) => !fileMatchesIgnoredPatterns(file, ignoredNeedles))
    .filter((file) => scanExtensions.has(path.extname(file).toLowerCase()))
    .sort((a, b) => a.localeCompare(b, 'en'));

  const backendFields = (contract.backend_state?.fields || []).map((v) => String(v).trim()).filter(Boolean);
  const freshnessFields = (contract.freshness?.fields || []).map((v) => String(v).trim()).filter(Boolean);
  const requiredFields = [
    ...backendFields.map((field) => ({ field, category: 'backend_state' })),
    ...freshnessFields.map((field) => ({ field, category: 'freshness' })),
  ];

  const fieldHitMap = new Map<string, number>();
  for (const row of requiredFields) {
    fieldHitMap.set(row.field, 0);
  }

  const errorViolations: Array<any> = [];
  const warningViolations: Array<any> = [];
  const invalidRegexPatterns: Array<{ pattern_id: string; regex: string }> = [];
  const patternHitMap = new Map<string, {
    pattern_id: string;
    severity: 'error' | 'warn';
    description: string;
    files: Set<string>;
    matches: number;
  }>();

  for (const file of scannedFiles) {
    const abs = path.resolve(ROOT, file);
    let source = '';
    try {
      source = fs.readFileSync(abs, 'utf8');
    } catch {
      continue;
    }

    for (const field of fieldHitMap.keys()) {
      const matches = source.match(new RegExp(`\\b${field}\\b`, 'g'));
      if (matches && matches.length) {
        fieldHitMap.set(field, Number(fieldHitMap.get(field) || 0) + matches.length);
      }
    }

    for (const pattern of patterns) {
      const regex = safeRegex(String(pattern.regex || ''));
      if (!regex) {
        invalidRegexPatterns.push({
          pattern_id: String(pattern.id || 'unknown_pattern'),
          regex: String(pattern.regex || ''),
        });
        continue;
      }
      const allowPaths = (allowedPatternPaths[pattern.id] || []).map((v) => String(v).replace(/\\/g, '/'));
      if (allowPaths.includes(file)) continue;
      const matches = source.match(regex);
      if (!matches || !matches.length) continue;
      const configuredSeverity = String(pattern.severity || 'warn') === 'error' ? 'error' : 'warn';
      const effectiveSeverity = configuredSeverity === 'warn' && errorOnWarningPatternIds.has(String(pattern.id || ''))
        ? 'error'
        : configuredSeverity;
      const row = {
        pattern_id: String(pattern.id || 'unknown_pattern'),
        severity: effectiveSeverity,
        description: String(pattern.description || '').trim(),
        file,
        matches: matches.length,
        configured_severity: configuredSeverity,
      };
      if (row.severity === 'error') errorViolations.push(row);
      else warningViolations.push(row);
      const existing = patternHitMap.get(row.pattern_id) || {
        pattern_id: row.pattern_id,
        severity: row.severity as 'error' | 'warn',
        description: row.description,
        files: new Set<string>(),
        matches: 0,
      };
      existing.files.add(file);
      existing.matches += row.matches;
      patternHitMap.set(row.pattern_id, existing);
    }
  }

  const contractFileFailures: Array<any> = [];
  const requiredContractFiles = Array.isArray(contract.required_contract_files)
    ? contract.required_contract_files
    : [];

  for (const entry of requiredContractFiles) {
    const file = String(entry.path || '').replace(/\\/g, '/');
    if (!file) continue;
    const abs = path.resolve(ROOT, file);
    if (!fs.existsSync(abs)) {
      contractFileFailures.push({
        file,
        reason: 'missing_contract_file',
        missing_tokens: [],
      });
      continue;
    }
    const source = fs.readFileSync(abs, 'utf8');
    const tokens = Array.isArray(entry.must_include)
      ? entry.must_include.map((v) => String(v).trim()).filter(Boolean)
      : [];
    const missingTokens = tokens.filter((token) => !source.includes(token));
    if (missingTokens.length) {
      contractFileFailures.push({
        file,
        reason: 'missing_contract_tokens',
        missing_tokens: missingTokens,
      });
    }
    for (const token of tokens) {
      if (!fieldHitMap.has(token)) continue;
      const matches = source.match(new RegExp(`\\b${token}\\b`, 'g'));
      if (matches && matches.length) {
        fieldHitMap.set(token, Number(fieldHitMap.get(token) || 0) + matches.length);
      }
    }
  }

  const requiredFieldCoverage = requiredFields.map((row) => {
    const hits = Number(fieldHitMap.get(row.field) || 0);
    return {
      field: row.field,
      category: row.category,
      hits,
      ok: hits > 0,
    };
  });

  const enumEvidence: Array<any> = [];
  const enumValues = (contract.backend_state?.enum_values || []).map((v) => String(v).trim()).filter(Boolean);
  const enumEvidenceFile = String(contract.backend_state?.enum_evidence_file || '').replace(/\\/g, '/');
  if (enumEvidenceFile && enumValues.length) {
    const abs = path.resolve(ROOT, enumEvidenceFile);
    if (!fs.existsSync(abs)) {
      for (const value of enumValues) {
        enumEvidence.push({ value, hits: 0, ok: false, reason: 'missing_enum_evidence_file' });
      }
      contractFileFailures.push({
        file: enumEvidenceFile,
        reason: 'missing_enum_evidence_file',
        missing_tokens: enumValues,
      });
    } else {
      const source = fs.readFileSync(abs, 'utf8');
      for (const value of enumValues) {
        const regex = new RegExp(`['\"]${value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}['\"]`, 'g');
        const matches = source.match(regex);
        enumEvidence.push({
          value,
          hits: matches ? matches.length : 0,
          ok: Boolean(matches && matches.length > 0),
          reason: matches && matches.length > 0 ? '' : 'missing_enum_literal',
        });
      }
    }
  }

  const configuredPatternIds = new Set(patterns.map((row) => String(row.id || '').trim()).filter(Boolean));
  const unknownErrorOnWarningPatternIds = errorOnWarningPatternIdRows
    .filter((patternId) => !configuredPatternIds.has(patternId));
  const invalidPatternIdFormats = patterns
    .map((row) => String(row.id || '').trim())
    .filter((patternId) => patternId.length > 0)
    .filter((patternId) => !/^[a-z0-9_]+$/.test(patternId));
  const invalidRequiredPatternIdFormats = requiredPatternIds
    .filter((patternId) => !/^[a-z0-9_]+$/.test(patternId));
  const invalidErrorOnWarningPatternIdFormats = errorOnWarningPatternIdRows
    .filter((patternId) => !/^[a-z0-9_]+$/.test(patternId));
  const configuredPatternSeverity = new Map<string, 'error' | 'warn'>(
    patterns
      .map((row) => {
        const patternId = String(row.id || '').trim();
        const severity: 'error' | 'warn' =
          String(row.severity || 'warn').trim() === 'error' ? 'error' : 'warn';
        return [patternId, severity] as const;
      })
      .filter((row) => row[0].length > 0),
  );
  const duplicateForbiddenPatternIds = Array.from(
    patterns.reduce((acc, row) => {
      const patternId = String(row.id || '').trim();
      if (!patternId) return acc;
      acc.set(patternId, (acc.get(patternId) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([patternId, count]) => `${patternId}:${count}`);
  const duplicateRequiredPatternIds = Array.from(
    requiredPatternIds.reduce((acc, patternId) => {
      acc.set(patternId, (acc.get(patternId) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([patternId, count]) => `${patternId}:${count}`);
  const duplicateErrorOnWarningPatternIds = Array.from(
    errorOnWarningPatternIdRows.reduce((acc, patternId) => {
      acc.set(patternId, (acc.get(patternId) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([patternId, count]) => `${patternId}:${count}`);
  const requiredPatternIdsWithoutErrorEnforcement = requiredPatternIds
    .filter((patternId) => configuredPatternIds.has(patternId))
    .filter((patternId) => {
      const severity = configuredPatternSeverity.get(patternId) || 'warn';
      return severity !== 'error' && !errorOnWarningPatternIds.has(patternId);
    });
  const unknownAllowedPatternIds = Object.keys(allowedPatternPaths)
    .map((v) => String(v || '').trim())
    .filter((patternId) => patternId.length > 0 && !configuredPatternIds.has(patternId));
  const allowedPatternPathRows = Object.entries(allowedPatternPaths)
    .flatMap(([patternId, paths]) => {
      const pathRows = Array.isArray(paths) ? paths : [];
      return pathRows.map((relativePath) => ({
        pattern_id: String(patternId || '').trim(),
        path: String(relativePath || '').replace(/\\/g, '/').trim(),
      }));
    })
    .filter((row) => row.pattern_id.length > 0 && row.path.length > 0);
  const invalidAllowedPatternPathFormats = allowedPatternPathRows
    .filter((row) => row.path.startsWith('/') || row.path.includes('..'))
    .map((row) => `${row.pattern_id}:${row.path}`);
  const allowedPatternPathsOutsideScanRoots = allowedPatternPathRows
    .filter((row) => !fileMatchesScanRoots(row.path, scanRoots))
    .map((row) => `${row.pattern_id}:${row.path}`);
  const allowedPatternPathsUnderIgnoredNeedles = allowedPatternPathRows
    .filter((row) => fileMatchesIgnoredPatterns(row.path, ignoredNeedles))
    .map((row) => `${row.pattern_id}:${row.path}`);
  const missingAllowedPatternPathFiles = Object.entries(allowedPatternPaths)
    .flatMap(([patternId, paths]) => {
      const pathRows = Array.isArray(paths) ? paths : [];
      return pathRows.map((relativePath) => ({
        pattern_id: String(patternId || '').trim(),
        path: String(relativePath || '').replace(/\\/g, '/').trim(),
      }));
    })
    .filter((row) => row.pattern_id.length > 0 && row.path.length > 0)
    .filter((row) => !fs.existsSync(path.resolve(ROOT, row.path)));
  const missingRequiredPatternIds = requiredPatternIds.filter((id) => !configuredPatternIds.has(id));
  const coverageFailures = requiredFieldCoverage.filter((row) => !row.ok);
  const enumFailures = enumEvidence.filter((row) => !row.ok);
  const patternHits = Array.from(patternHitMap.values())
    .map((row) => ({
      pattern_id: row.pattern_id,
      severity: row.severity,
      description: row.description,
      files: row.files.size,
      matches: row.matches,
    }))
    .sort((a, b) => a.pattern_id.localeCompare(b.pattern_id, 'en'));
  const authorityAudit = Object.entries(AUTHORITY_AUDIT_CATEGORIES).map(([category, ids]) => {
    const idSet = new Set(ids.map((id) => String(id || '').trim()).filter(Boolean));
    const matchedRows = [...errorViolations, ...warningViolations].filter((row) =>
      idSet.has(String(row.pattern_id || '')),
    );
    const fileSet = new Set(matchedRows.map((row) => String(row.file || '')).filter(Boolean));
    const matches = matchedRows.reduce((sum, row) => sum + Number(row.matches || 0), 0);
    return {
      category,
      violations: matchedRows.length,
      matches,
      files: fileSet.size,
      pattern_ids: Array.from(idSet),
    };
  });
  const authorityAuditMap = new Map(authorityAudit.map((row) => [row.category, row]));

  const report = {
    type: 'shell_truth_leak_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    policy_path: rel(path.resolve(ROOT, args.policyPath)),
    contract_path: rel(path.resolve(ROOT, args.contractPath)),
    summary: {
      scanned_files: scannedFiles.length,
      contract_files_checked: requiredContractFiles.length,
      required_field_coverage_failures: coverageFailures.length,
      enum_evidence_failures: enumFailures.length,
      contract_file_failures: contractFileFailures.length,
      invalid_regex_patterns: invalidRegexPatterns.length,
      missing_scan_roots: missingScanRoots.length,
      duplicate_forbidden_pattern_ids: duplicateForbiddenPatternIds.length,
      duplicate_required_pattern_ids: duplicateRequiredPatternIds.length,
      duplicate_error_on_warning_pattern_ids: duplicateErrorOnWarningPatternIds.length,
      duplicate_scan_roots: duplicateScanRoots.length,
      duplicate_scan_extensions: duplicateScanExtensions.length,
      duplicate_ignore_path_needles: duplicateIgnorePathNeedles.length,
      unknown_error_on_warning_pattern_ids: unknownErrorOnWarningPatternIds.length,
      invalid_pattern_id_formats: invalidPatternIdFormats.length,
      invalid_required_pattern_id_formats: invalidRequiredPatternIdFormats.length,
      invalid_error_on_warning_pattern_id_formats: invalidErrorOnWarningPatternIdFormats.length,
      required_pattern_ids_without_error_enforcement: requiredPatternIdsWithoutErrorEnforcement.length,
      unknown_allowed_pattern_ids: unknownAllowedPatternIds.length,
      missing_allowed_pattern_path_files: missingAllowedPatternPathFiles.length,
      invalid_allowed_pattern_path_formats: invalidAllowedPatternPathFormats.length,
      allowed_pattern_paths_outside_scan_roots: allowedPatternPathsOutsideScanRoots.length,
      allowed_pattern_paths_under_ignored_needles: allowedPatternPathsUnderIgnoredNeedles.length,
      missing_required_pattern_ids: missingRequiredPatternIds.length,
      error_violations: errorViolations.length,
      warning_violations: warningViolations.length,
      pattern_hits: patternHits.length,
      authority_retry_logic_violations:
        Number(authorityAuditMap.get('retry_logic')?.violations || 0),
      authority_health_inference_violations:
        Number(authorityAuditMap.get('health_inference')?.violations || 0),
      authority_lane_selection_violations:
        Number(authorityAuditMap.get('lane_selection')?.violations || 0),
      authority_policy_branching_violations:
        Number(authorityAuditMap.get('policy_branching')?.violations || 0),
      pass: coverageFailures.length === 0
        && enumFailures.length === 0
        && contractFileFailures.length === 0
        && invalidRegexPatterns.length === 0
        && missingScanRoots.length === 0
        && duplicateForbiddenPatternIds.length === 0
        && duplicateRequiredPatternIds.length === 0
        && duplicateErrorOnWarningPatternIds.length === 0
        && duplicateScanRoots.length === 0
        && duplicateScanExtensions.length === 0
        && duplicateIgnorePathNeedles.length === 0
        && unknownErrorOnWarningPatternIds.length === 0
        && invalidPatternIdFormats.length === 0
        && invalidRequiredPatternIdFormats.length === 0
        && invalidErrorOnWarningPatternIdFormats.length === 0
        && requiredPatternIdsWithoutErrorEnforcement.length === 0
        && unknownAllowedPatternIds.length === 0
        && missingAllowedPatternPathFiles.length === 0
        && invalidAllowedPatternPathFormats.length === 0
        && allowedPatternPathsOutsideScanRoots.length === 0
        && allowedPatternPathsUnderIgnoredNeedles.length === 0
        && missingRequiredPatternIds.length === 0
        && errorViolations.length === 0,
    },
    scanned_files: scannedFiles,
    missing_scan_roots: missingScanRoots,
    required_field_coverage: requiredFieldCoverage,
    enum_evidence: enumEvidence,
    contract_file_failures: contractFileFailures,
    invalid_regex_patterns: invalidRegexPatterns,
    duplicate_forbidden_pattern_ids: duplicateForbiddenPatternIds,
    duplicate_required_pattern_ids: duplicateRequiredPatternIds,
    duplicate_error_on_warning_pattern_ids: duplicateErrorOnWarningPatternIds,
    duplicate_scan_roots: duplicateScanRoots,
    duplicate_scan_extensions: duplicateScanExtensions,
    duplicate_ignore_path_needles: duplicateIgnorePathNeedles,
    unknown_error_on_warning_pattern_ids: unknownErrorOnWarningPatternIds,
    invalid_pattern_id_formats: invalidPatternIdFormats,
    invalid_required_pattern_id_formats: invalidRequiredPatternIdFormats,
    invalid_error_on_warning_pattern_id_formats: invalidErrorOnWarningPatternIdFormats,
    required_pattern_ids_without_error_enforcement: requiredPatternIdsWithoutErrorEnforcement,
    unknown_allowed_pattern_ids: unknownAllowedPatternIds,
    missing_allowed_pattern_path_files: missingAllowedPatternPathFiles,
    invalid_allowed_pattern_path_formats: invalidAllowedPatternPathFormats,
    allowed_pattern_paths_outside_scan_roots: allowedPatternPathsOutsideScanRoots,
    allowed_pattern_paths_under_ignored_needles: allowedPatternPathsUnderIgnoredNeedles,
    missing_required_pattern_ids: missingRequiredPatternIds,
    error_violations: errorViolations,
    warning_violations: warningViolations,
    pattern_hits: patternHits,
    authority_audit: authorityAudit,
  };

  const markdown = formatMarkdown({
    ...report,
    ok: report.summary.pass,
  });
  writeTextArtifact(path.resolve(ROOT, args.outMarkdown), markdown);

  process.exit(
    emitStructuredResult(report, {
      outPath: path.resolve(ROOT, args.outJson),
      strict: args.strict,
      ok: report.summary.pass,
    }),
  );
}

main();
