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
  lines.push(`- error_violations: ${payload.summary.error_violations}`);
  lines.push(`- warning_violations: ${payload.summary.warning_violations}`);
  lines.push(`- pattern_hits: ${payload.summary.pattern_hits}`);
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
  const scanExtensions = new Set(
    (Array.isArray(policy.scan_extensions) ? policy.scan_extensions : ['.ts', '.js'])
      .map((v) => String(v).trim().toLowerCase()),
  );
  const ignoredNeedles = (Array.isArray(policy.ignore_path_contains) ? policy.ignore_path_contains : [])
    .map((v) => String(v).trim())
    .filter(Boolean);
  const patterns = (Array.isArray(policy.forbidden_patterns) ? policy.forbidden_patterns : []);
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
      if (!regex) continue;
      const allowPaths = (allowedPatternPaths[pattern.id] || []).map((v) => String(v).replace(/\\/g, '/'));
      if (allowPaths.includes(file)) continue;
      const matches = source.match(regex);
      if (!matches || !matches.length) continue;
      const row = {
        pattern_id: String(pattern.id || 'unknown_pattern'),
        severity: String(pattern.severity || 'warn') === 'error' ? 'error' : 'warn',
        description: String(pattern.description || '').trim(),
        file,
        matches: matches.length,
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
      error_violations: errorViolations.length,
      warning_violations: warningViolations.length,
      pattern_hits: patternHits.length,
      pass: coverageFailures.length === 0
        && enumFailures.length === 0
        && contractFileFailures.length === 0
        && errorViolations.length === 0,
    },
    scanned_files: scannedFiles,
    required_field_coverage: requiredFieldCoverage,
    enum_evidence: enumEvidence,
    contract_file_failures: contractFileFailures,
    error_violations: errorViolations,
    warning_violations: warningViolations,
    pattern_hits: patternHits,
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
