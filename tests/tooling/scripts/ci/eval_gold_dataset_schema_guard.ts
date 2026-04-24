#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type Taxonomy = {
  classes?: Array<{ id?: string; critical?: boolean }>;
  severity_scale?: string[];
};

const DEFAULT_DATASET_PATH = 'tests/tooling/fixtures/eval_gold_dataset_seed.jsonl';
const DEFAULT_SCHEMA_PATH = 'tests/tooling/schemas/eval_gold_dataset.schema.json';
const DEFAULT_TAXONOMY_PATH = 'tests/tooling/config/eval_issue_taxonomy.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_gold_dataset_schema_guard_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_gold_dataset_schema_guard_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_GOLD_DATASET_SCHEMA_GUARD_CURRENT.md';
const SENSITIVE_PATTERNS = [
  /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/i,
  /\bhttps?:\/\/\S+/i,
  /\bwww\.\S+/i,
  /\b(?:sk|rk|pk|ghp|xoxb|xoxp)[-_A-Za-z0-9]{8,}\b/,
  /\b\d{3}[-.\s]?\d{2}[-.\s]?\d{4}\b/,
  /\b\d{12,19}\b/,
];

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    datasetPath: cleanText(readFlag(argv, 'dataset') || DEFAULT_DATASET_PATH, 500),
    schemaPath: cleanText(readFlag(argv, 'schema') || DEFAULT_SCHEMA_PATH, 500),
    taxonomyPath: cleanText(readFlag(argv, 'taxonomy') || DEFAULT_TAXONOMY_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function readJsonLines(filePath: string): any[] {
  try {
    const body = fs.readFileSync(filePath, 'utf8');
    return body
      .split(/\r?\n/)
      .filter((line) => line.trim().length > 0)
      .map((line) => {
        try {
          return JSON.parse(line);
        } catch {
          return { __parse_error: line };
        }
      });
  } catch {
    return [];
  }
}

function validateDatasetRows(
  rows: any[],
  validClasses: Set<string>,
  validSeverities: Set<string>,
): Array<{ row: number; reason: string }> {
  const failures: Array<{ row: number; reason: string }> = [];
  const requiredTop = ['id', 'source_event_id', 'ts', 'prompt', 'assistant_text', 'labels'];
  const requiredLabels = ['issue_class', 'severity', 'is_failure', 'expected_fix'];

  rows.forEach((row, index) => {
    const rowNum = index + 1;
    if (!row || typeof row !== 'object' || row.__parse_error) {
      failures.push({ row: rowNum, reason: 'row_parse_error' });
      return;
    }
    for (const field of requiredTop) {
      if (!(field in row)) failures.push({ row: rowNum, reason: `missing_${field}` });
    }
    const labels = row.labels;
    if (!labels || typeof labels !== 'object') {
      failures.push({ row: rowNum, reason: 'missing_labels_object' });
      return;
    }
    for (const field of requiredLabels) {
      if (!(field in labels)) failures.push({ row: rowNum, reason: `missing_labels_${field}` });
    }
    const issueClass = cleanText(labels.issue_class, 120);
    if (!validClasses.has(issueClass)) failures.push({ row: rowNum, reason: `invalid_issue_class:${issueClass || 'empty'}` });
    const severity = cleanText(labels.severity, 40);
    if (!validSeverities.has(severity)) failures.push({ row: rowNum, reason: `invalid_severity:${severity || 'empty'}` });
    if (typeof labels.is_failure !== 'boolean') failures.push({ row: rowNum, reason: 'labels_is_failure_not_boolean' });
    if (cleanText(labels.expected_fix, 500).length < 5) failures.push({ row: rowNum, reason: 'labels_expected_fix_too_short' });

    const safetyFields = [
      cleanText(row.prompt, 12000),
      cleanText(row.assistant_text, 12000),
      ...(Array.isArray(row.tool_trace) ? row.tool_trace.map((entry: unknown) => cleanText(entry, 500)) : []),
    ];
    for (const value of safetyFields) {
      for (const pattern of SENSITIVE_PATTERNS) {
        if (pattern.test(value)) {
          failures.push({ row: rowNum, reason: 'unredacted_sensitive_content_detected' });
          break;
        }
      }
    }
  });

  return failures;
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Gold Dataset Schema Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- dataset_rows: ${Number(report.summary?.dataset_rows || 0)}`);
  lines.push(`- critical_class_coverage: ${Number(report.summary?.critical_class_coverage || 0)}`);
  lines.push(`- required_critical_classes: ${Number(report.summary?.required_critical_classes || 0)}`);
  lines.push(`- failure_count: ${Number(report.summary?.failure_count || 0)}`);
  lines.push('');
  lines.push('## Failures');
  const failures = Array.isArray(report.failures) ? report.failures : [];
  if (failures.length === 0) {
    lines.push('- none');
  } else {
    failures.slice(0, 30).forEach((row) => {
      lines.push(`- row ${Number(row.row || 0)}: ${cleanText(row.reason || '', 240)}`);
    });
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const datasetAbs = path.resolve(root, args.datasetPath);
  const schemaAbs = path.resolve(root, args.schemaPath);
  const taxonomyAbs = path.resolve(root, args.taxonomyPath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const nowIso = new Date().toISOString();

  const schemaExists = fs.existsSync(schemaAbs);
  const schema = readJson(schemaAbs);
  const taxonomy = (readJson(taxonomyAbs) || {}) as Taxonomy;
  const rows = readJsonLines(datasetAbs);
  const validClasses = new Set(
    (Array.isArray(taxonomy.classes) ? taxonomy.classes : [])
      .map((row) => cleanText(row?.id || '', 120))
      .filter(Boolean),
  );
  const criticalClasses = new Set(
    (Array.isArray(taxonomy.classes) ? taxonomy.classes : [])
      .filter((row) => Boolean(row?.critical))
      .map((row) => cleanText(row?.id || '', 120))
      .filter(Boolean),
  );
  const validSeverities = new Set(
    (Array.isArray(taxonomy.severity_scale) ? taxonomy.severity_scale : [])
      .map((row) => cleanText(row, 40))
      .filter(Boolean),
  );
  const failures = validateDatasetRows(rows, validClasses, validSeverities);

  const presentClasses = new Set(
    rows
      .filter((row) => row && typeof row === 'object' && row.labels)
      .map((row) => cleanText(row.labels.issue_class, 120))
      .filter(Boolean),
  );
  const missingCriticalClasses = Array.from(criticalClasses).filter(
    (issueClass) => !presentClasses.has(issueClass),
  );
  for (const issueClass of missingCriticalClasses) {
    failures.push({ row: 0, reason: `missing_critical_class:${issueClass}` });
  }

  const checks = [
    { id: 'schema_exists', ok: schemaExists, detail: args.schemaPath },
    { id: 'schema_parseable', ok: Boolean(schema && typeof schema === 'object'), detail: args.schemaPath },
    { id: 'taxonomy_classes_present', ok: validClasses.size > 0, detail: `class_count=${validClasses.size}` },
    { id: 'taxonomy_severities_present', ok: validSeverities.size > 0, detail: `severity_count=${validSeverities.size}` },
    { id: 'dataset_rows_present', ok: rows.length > 0, detail: `rows=${rows.length}` },
    { id: 'dataset_schema_valid', ok: failures.length === 0, detail: `failure_count=${failures.length}` },
    {
      id: 'critical_class_coverage',
      ok: missingCriticalClasses.length === 0,
      detail: `required=${criticalClasses.size};missing=${missingCriticalClasses.length}`,
    },
  ];

  const report = {
    type: 'eval_gold_dataset_schema_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    summary: {
      dataset_rows: rows.length,
      critical_class_coverage: criticalClasses.size - missingCriticalClasses.length,
      required_critical_classes: criticalClasses.size,
      failure_count: failures.length,
    },
    failures,
    sources: {
      dataset: args.datasetPath,
      schema: args.schemaPath,
      taxonomy: args.taxonomyPath,
    },
  };

  writeJsonArtifact(outLatestAbs, report);
  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));
