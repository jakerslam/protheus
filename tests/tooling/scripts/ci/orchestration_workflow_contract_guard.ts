#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'client/runtime/config/orchestration_workflow_contract_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_workflow_contract_guard_current.json';
const DEFAULT_OUT_MARKDOWN =
  'local/workspace/reports/ORCHESTRATION_WORKFLOW_CONTRACT_GUARD_CURRENT.md';

type RequiredEntry = {
  path?: string;
  required_phrases?: string[];
};

type Policy = {
  required_docs?: RequiredEntry[];
  required_rust_files?: RequiredEntry[];
};

type Violation = {
  check_id: string;
  file: string;
  reason: string;
  detail: string;
};

type Args = {
  strict: boolean;
  policy: string;
  outJson: string;
  outMarkdown: string;
};

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function isCanonicalRelativePath(value: string): boolean {
  if (!value) return false;
  if (value.startsWith('/') || value.startsWith('\\')) return false;
  if (value.includes('..') || value.includes('\\') || value.includes('//')) return false;
  return /^[A-Za-z0-9._/\-]+$/.test(value);
}

function hasCaseInsensitiveSuffix(value: string, suffix: string): boolean {
  return value.toLowerCase().endsWith(suffix.toLowerCase());
}

function isNonEmptyString(value: unknown): boolean {
  return typeof value === 'string' && cleanText(value, 500).length > 0;
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: common.strict,
    policy: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function requiredPhraseViolations(entries: RequiredEntry[], checkId: string): Violation[] {
  const violations: Violation[] = [];
  for (const entry of entries) {
    const targetPath = cleanText(entry.path || '', 400);
    if (!targetPath) {
      violations.push({
        check_id: checkId,
        file: '(policy)',
        reason: 'missing_required_path',
        detail: JSON.stringify(entry),
      });
      continue;
    }
    const abs = path.resolve(ROOT, targetPath);
    if (!fs.existsSync(abs)) {
      violations.push({
        check_id: checkId,
        file: targetPath,
        reason: 'required_file_missing',
        detail: 'file_not_found',
      });
      continue;
    }
    const source = fs.readFileSync(abs, 'utf8');
    const phrases = Array.isArray(entry.required_phrases) ? entry.required_phrases : [];
    for (const phrase of phrases) {
      const normalized = cleanText(phrase, 300);
      if (!normalized) continue;
      if (!source.includes(normalized)) {
        violations.push({
          check_id: checkId,
          file: targetPath,
          reason: 'required_phrase_missing',
          detail: normalized,
        });
      }
    }
  }
  return violations;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Orchestration Workflow Contract Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push(`Policy: ${payload.inputs.policy}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- check_count: ${payload.summary.check_count}`);
  lines.push(`- check_failure_count: ${payload.summary.check_failure_count}`);
  lines.push(`- docs_checked: ${payload.summary.docs_checked}`);
  lines.push(`- rust_files_checked: ${payload.summary.rust_files_checked}`);
  lines.push(`- violation_count: ${payload.summary.violation_count}`);
  lines.push('');
  lines.push('## Checks');
  if (Array.isArray(payload.checks) && payload.checks.length > 0) {
    for (const row of payload.checks) {
      lines.push(`- [${row.ok ? 'x' : ' '}] ${row.id} :: ${row.detail}`);
    }
  } else {
    lines.push('- none');
  }
  lines.push('');
  if (Array.isArray(payload.violations) && payload.violations.length > 0) {
    lines.push('## Violations');
    for (const row of payload.violations) {
      lines.push(`- ${row.check_id} :: ${row.file} :: ${row.reason} :: ${row.detail}`);
    }
  } else {
    lines.push('## Violations');
    lines.push('- none');
  }
  return `${lines.join('\n')}\n`;
}

function main(argv: string[]): number {
  const args = parseArgs(argv);
  const checks: CheckRow[] = [];
  const policyPath = path.resolve(ROOT, args.policy);
  const policyRel = rel(policyPath);
  const policyExists = fs.existsSync(policyPath);
  let policy: Policy = {};
  let policyParseError = '';
  if (policyExists) {
    try {
      policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Policy;
    } catch (error) {
      policyParseError = cleanText(error instanceof Error ? error.message : String(error), 500);
      policy = {};
    }
  }

  checks.push({
    id: 'orchestration_workflow_contract_policy_path_canonical_contract',
    ok: isCanonicalRelativePath(args.policy),
    detail: args.policy,
  });
  checks.push({
    id: 'orchestration_workflow_contract_out_json_path_canonical_contract',
    ok: isCanonicalRelativePath(args.outJson),
    detail: args.outJson,
  });
  checks.push({
    id: 'orchestration_workflow_contract_out_markdown_path_canonical_contract',
    ok: isCanonicalRelativePath(args.outMarkdown),
    detail: args.outMarkdown,
  });
  checks.push({
    id: 'orchestration_workflow_contract_policy_suffix_contract',
    ok: hasCaseInsensitiveSuffix(args.policy, '.json'),
    detail: args.policy,
  });
  checks.push({
    id: 'orchestration_workflow_contract_out_json_current_suffix_contract',
    ok: hasCaseInsensitiveSuffix(args.outJson, '_current.json'),
    detail: args.outJson,
  });
  checks.push({
    id: 'orchestration_workflow_contract_out_markdown_current_suffix_contract',
    ok: hasCaseInsensitiveSuffix(args.outMarkdown, '_current.md'),
    detail: args.outMarkdown,
  });
  checks.push({
    id: 'orchestration_workflow_contract_output_paths_distinct_contract',
    ok: new Set([args.policy, args.outJson, args.outMarkdown]).size === 3,
    detail: `${args.policy}|${args.outJson}|${args.outMarkdown}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_policy_file_exists_contract',
    ok: policyExists,
    detail: policyRel,
  });
  checks.push({
    id: 'orchestration_workflow_contract_policy_parse_contract',
    ok: policyExists && !policyParseError,
    detail: policyParseError || 'ok',
  });
  const requiredDocs = Array.isArray(policy.required_docs) ? policy.required_docs : [];
  const requiredRustFiles = Array.isArray(policy.required_rust_files)
    ? policy.required_rust_files
    : [];
  const requiredDocsPaths = requiredDocs.map((entry) => cleanText(entry.path || '', 400)).filter(Boolean);
  const requiredRustPaths = requiredRustFiles
    .map((entry) => cleanText(entry.path || '', 400))
    .filter(Boolean);

  checks.push({
    id: 'orchestration_workflow_contract_required_docs_nonempty_contract',
    ok: requiredDocs.length > 0,
    detail: `count=${requiredDocs.length}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_required_rust_files_nonempty_contract',
    ok: requiredRustFiles.length > 0,
    detail: `count=${requiredRustFiles.length}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_required_docs_paths_unique_canonical_contract',
    ok: requiredDocsPaths.length === requiredDocs.length
      && new Set(requiredDocsPaths).size === requiredDocsPaths.length
      && requiredDocsPaths.every((entryPath) => isCanonicalRelativePath(entryPath)),
    detail: `count=${requiredDocs.length};paths=${requiredDocsPaths.length};unique=${new Set(requiredDocsPaths).size}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_required_rust_paths_unique_canonical_contract',
    ok: requiredRustPaths.length === requiredRustFiles.length
      && new Set(requiredRustPaths).size === requiredRustPaths.length
      && requiredRustPaths.every((entryPath) => isCanonicalRelativePath(entryPath)),
    detail: `count=${requiredRustFiles.length};paths=${requiredRustPaths.length};unique=${new Set(requiredRustPaths).size}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_required_docs_phrases_nonempty_contract',
    ok: requiredDocs.every((entry) => Array.isArray(entry.required_phrases) && entry.required_phrases.length > 0),
    detail: `count=${requiredDocs.length}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_required_rust_phrases_nonempty_contract',
    ok: requiredRustFiles.every((entry) => Array.isArray(entry.required_phrases) && entry.required_phrases.length > 0),
    detail: `count=${requiredRustFiles.length}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_required_docs_phrase_tokens_contract',
    ok: requiredDocs.every((entry) => {
      const phrases = Array.isArray(entry.required_phrases) ? entry.required_phrases : [];
      return phrases.every((phrase) => isNonEmptyString(phrase));
    }),
    detail: `count=${requiredDocs.length}`,
  });
  checks.push({
    id: 'orchestration_workflow_contract_required_rust_phrase_tokens_contract',
    ok: requiredRustFiles.every((entry) => {
      const phrases = Array.isArray(entry.required_phrases) ? entry.required_phrases : [];
      return phrases.every((phrase) => isNonEmptyString(phrase));
    }),
    detail: `count=${requiredRustFiles.length}`,
  });

  const violations = [
    ...requiredPhraseViolations(requiredDocs, 'required_docs'),
    ...requiredPhraseViolations(requiredRustFiles, 'required_rust_files'),
  ];
  const violationsShapeValid = violations.every((row) => {
    return (
      isNonEmptyString(row.check_id)
      && (isCanonicalRelativePath(cleanText(row.file || '', 500)) || row.file === '(policy)')
      && isNonEmptyString(row.reason)
      && isNonEmptyString(row.detail)
    );
  });
  checks.push({
    id: 'orchestration_workflow_contract_violation_shape_contract',
    ok: violationsShapeValid,
    detail: `count=${violations.length}`,
  });

  const artifactPaths = [args.outJson, args.outMarkdown];
  checks.push({
    id: 'orchestration_workflow_contract_artifact_paths_unique_canonical_contract',
    ok: new Set(artifactPaths).size === artifactPaths.length
      && artifactPaths.every((artifactPath) => isCanonicalRelativePath(artifactPath)),
    detail: artifactPaths.join('|'),
  });

  const summaryParity = requiredDocs.length === requiredDocsPaths.length
    && requiredRustFiles.length === requiredRustPaths.length;
  checks.push({
    id: 'orchestration_workflow_contract_summary_cardinality_parity_contract',
    ok: summaryParity,
    detail: `docs=${requiredDocs.length}/${requiredDocsPaths.length};rust=${requiredRustFiles.length}/${requiredRustPaths.length}`,
  });

  const checkFailureCount = checks.filter((row) => !row.ok).length;
  const ok = violations.length === 0 && checkFailureCount === 0;
  const payload = {
    ok,
    type: 'orchestration_workflow_contract_guard',
    generated_at: new Date().toISOString(),
    owner: 'ops',
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      policy: policyRel,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    summary: {
      check_count: checks.length,
      check_failure_count: checkFailureCount,
      docs_checked: requiredDocs.length,
      rust_files_checked: requiredRustFiles.length,
      violation_count: violations.length,
      pass: ok,
    },
    checks,
    artifact_paths: artifactPaths,
    violations,
  };
  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(main(process.argv.slice(2)));
