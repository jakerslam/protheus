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

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
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
  lines.push(`- docs_checked: ${payload.summary.docs_checked}`);
  lines.push(`- rust_files_checked: ${payload.summary.rust_files_checked}`);
  lines.push(`- violation_count: ${payload.summary.violation_count}`);
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
  const policyPath = path.resolve(ROOT, args.policy);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Policy;
  const requiredDocs = Array.isArray(policy.required_docs) ? policy.required_docs : [];
  const requiredRustFiles = Array.isArray(policy.required_rust_files)
    ? policy.required_rust_files
    : [];
  const violations = [
    ...requiredPhraseViolations(requiredDocs, 'required_docs'),
    ...requiredPhraseViolations(requiredRustFiles, 'required_rust_files'),
  ];
  const payload = {
    ok: violations.length === 0,
    type: 'orchestration_workflow_contract_guard',
    generated_at: new Date().toISOString(),
    owner: 'ops',
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      policy: rel(policyPath),
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    summary: {
      docs_checked: requiredDocs.length,
      rust_files_checked: requiredRustFiles.length,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    artifact_paths: [args.outJson, args.outMarkdown],
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
