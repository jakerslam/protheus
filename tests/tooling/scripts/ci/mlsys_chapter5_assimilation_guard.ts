#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'observability/research/mlsys_chapter5_workflow_assimilation.json';
const DEFAULT_NOTE = 'observability/research/MLSYS_CHAPTER5_ASSIMILATION.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/mlsys_chapter5_assimilation_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/MLSYS_CHAPTER5_ASSIMILATION_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  contractPath: string;
  notePath: string;
  outJson: string;
  outMarkdown: string;
  includeControlledViolation: boolean;
};

type Violation = {
  kind: string;
  path: string;
  detail: string;
};

const REQUIRED_LESSONS = [
  'feedback_loops_are_primary',
  'workflow_is_not_linear',
  'validation_is_continuous',
  'production_differs_from_development',
  'systems_thinking_controls_transfer',
];

const REQUIRED_DIMENSIONS = [
  'workload_awareness',
  'confidence_routing',
  'resource_budgeting',
  'dam_diagnosis',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function readJson<T>(relPath: string): T {
  return JSON.parse(readText(relPath)) as T;
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    contractPath: cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600),
    notePath: cleanText(readFlag(argv, 'note') || DEFAULT_NOTE, 600),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function requireRows(violations: Violation[], pathName: string, kind: string, actual: Set<string>, expected: string[]): void {
  for (const id of expected) {
    if (!actual.has(id)) {
      violations.push({ kind, path: pathName, detail: `Missing required id: ${id}` });
    }
  }
}

function validateContract(contract: any, contractPath: string, violations: Violation[]): void {
  if (contract.type !== 'mlsys_chapter5_workflow_assimilation') {
    violations.push({ kind: 'mlsys5_contract_type_invalid', path: contractPath, detail: 'Contract type mismatch.' });
  }
  if (!String(contract.source?.url || '').includes('mlsysbook.ai/contents/core/workflow/workflow.html')) {
    violations.push({ kind: 'mlsys5_source_url_missing', path: contractPath, detail: 'Contract must cite the Chapter 5 workflow page.' });
  }
  const lessons = new Set((contract.source_lessons || []).map((row: any) => String(row.id || '')));
  requireRows(violations, contractPath, 'mlsys5_lesson_missing', lessons, REQUIRED_LESSONS);
  const dimensions = new Set((contract.infring_assimilation_dimensions || []).map((row: any) => String(row.id || '')));
  requireRows(violations, contractPath, 'mlsys5_dimension_missing', dimensions, REQUIRED_DIMENSIONS);
  for (const row of contract.infring_assimilation_dimensions || []) {
    const id = String(row.id || '<missing>');
    if (!row.required_behavior || !row.owned_by) {
      violations.push({ kind: 'mlsys5_dimension_shape_invalid', path: contractPath, detail: `${id} requires required_behavior and owned_by.` });
    }
    const evidence = Array.isArray(row.evidence_refs) ? row.evidence_refs.map(String) : [];
    if (evidence.length === 0) {
      violations.push({ kind: 'mlsys5_dimension_evidence_missing', path: contractPath, detail: `${id} needs evidence_refs.` });
    }
    for (const ref of evidence) {
      if (!fs.existsSync(abs(ref))) {
        violations.push({ kind: 'mlsys5_dimension_evidence_missing_file', path: contractPath, detail: `${id} evidence does not exist: ${ref}` });
      }
    }
  }
  const guardrails = Array.isArray(contract.guardrails) ? contract.guardrails : [];
  if (guardrails.length < 5) {
    violations.push({ kind: 'mlsys5_guardrails_incomplete', path: contractPath, detail: 'At least five guardrails are required.' });
  }
}

function validateNote(notePath: string, text: string, violations: Violation[]): void {
  for (const token of ['Feedback loops are primary', 'Validation is continuous', 'Production differs from development', 'Diagnose -> Act -> Monitor']) {
    if (!text.includes(token)) {
      violations.push({ kind: 'mlsys5_note_missing_token', path: notePath, detail: `Missing note token: ${token}` });
    }
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# MLSys Chapter 5 Assimilation Guard');
  lines.push('');
  lines.push(`- Generated at: ${payload.generated_at}`);
  lines.push(`- Revision: ${payload.revision}`);
  lines.push(`- Pass: ${payload.ok}`);
  lines.push(`- Contract: ${payload.contract_path}`);
  lines.push('');
  lines.push('## Summary');
  for (const [key, value] of Object.entries(payload.summary)) lines.push(`- ${key}: ${value}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) lines.push(`- ${violation.kind}: ${violation.path} ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const contract = readJson<any>(args.contractPath);
  const note = readText(args.notePath);
  const violations: Violation[] = [];
  validateContract(contract, args.contractPath, violations);
  validateNote(args.notePath, note, violations);
  if (args.includeControlledViolation) {
    violations.push({
      kind: 'controlled_mlsys5_assimilation_violation',
      path: args.contractPath,
      detail: 'Controlled failure proves strict mode rejects incomplete chapter assimilation.',
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'mlsys_chapter5_assimilation_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    note_path: args.notePath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      source_lessons: (contract.source_lessons || []).length,
      assimilation_dimensions: (contract.infring_assimilation_dimensions || []).length,
      guardrails: (contract.guardrails || []).length,
      violations: violations.length,
    },
    violations,
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: args.outJson });
  if (args.strict && !payload.ok) process.exitCode = 1;
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
