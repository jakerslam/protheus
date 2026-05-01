#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'observability/reports/kernel_sentinel_boundedness_repair_lane.json';
const DEFAULT_NOTE = 'observability/reports/KERNEL_SENTINEL_BOUNDEDNESS_REPAIR_LANE.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/kernel_sentinel_boundedness_repair_lane_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/KERNEL_SENTINEL_BOUNDEDNESS_REPAIR_LANE_GUARD_CURRENT.md';

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

const REQUIRED_DIMENSIONS = [
  'max_rss',
  'queue_depth_p95',
  'queue_depth_max',
  'stale_surface_count',
  'recovery_time_ms',
  'report_size_bytes',
];

const REQUIRED_COMMAND_TOKENS = [
  'workspace-tooling:context-soak',
  'workspace-tooling:release-proof',
  'kernel_sentinel::report_budget',
  'ops:ksent:boundedness-repair:guard',
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

function requireStringArray(
  violations: Violation[],
  relPath: string,
  kind: string,
  values: unknown,
  expected: string[],
): string[] {
  const actual = Array.isArray(values) ? values.map(String) : [];
  for (const id of expected) {
    if (!actual.includes(id)) {
      violations.push({ kind, path: relPath, detail: `Missing required value: ${id}` });
    }
  }
  return actual;
}

function validateContract(contract: any, contractPath: string, violations: Violation[]): void {
  if (contract.type !== 'kernel_sentinel_boundedness_repair_lane') {
    violations.push({ kind: 'boundedness_contract_type_invalid', path: contractPath, detail: 'Contract type mismatch.' });
  }
  if (contract.owner_domain !== 'observability') {
    violations.push({ kind: 'boundedness_owner_invalid', path: contractPath, detail: 'Owner domain must be observability.' });
  }
  if (contract.source_todo !== 'KSENT-ISSUE-BOUNDEDNESS') {
    violations.push({ kind: 'boundedness_source_todo_invalid', path: contractPath, detail: 'Source TODO must be KSENT-ISSUE-BOUNDEDNESS.' });
  }
  if (contract.policy?.human_review_required !== true || contract.policy?.safe_to_auto_apply_patch !== false) {
    violations.push({ kind: 'boundedness_policy_invalid', path: contractPath, detail: 'Repair lane must require review and forbid auto-apply.' });
  }
  if (contract.policy?.raw_evidence_embedded !== false) {
    violations.push({ kind: 'boundedness_raw_evidence_policy_invalid', path: contractPath, detail: 'Raw evidence must not be embedded.' });
  }
  requireStringArray(violations, contractPath, 'boundedness_dimension_missing', contract.boundedness_dimensions, REQUIRED_DIMENSIONS);
  const lanes = Array.isArray(contract.repair_lanes) ? contract.repair_lanes : [];
  if (lanes.length === 0) {
    violations.push({ kind: 'boundedness_repair_lane_missing', path: contractPath, detail: 'At least one repair lane is required.' });
  }
  for (const lane of lanes) {
    const laneId = String(lane.id || '<missing>');
    for (const field of ['owner', 'failure_signature', 'root_cause_hypothesis', 'concrete_next_action']) {
      if (!String(lane[field] || '').trim()) {
        violations.push({ kind: 'boundedness_lane_field_missing', path: contractPath, detail: `${laneId} missing ${field}.` });
      }
    }
    const acceptance = Array.isArray(lane.acceptance_criteria) ? lane.acceptance_criteria.map(String) : [];
    if (acceptance.length < 4) {
      violations.push({ kind: 'boundedness_acceptance_incomplete', path: contractPath, detail: `${laneId} needs at least four acceptance criteria.` });
    }
    const commands = Array.isArray(lane.replay_validation_commands) ? lane.replay_validation_commands.map(String) : [];
    for (const token of REQUIRED_COMMAND_TOKENS) {
      if (!commands.some((command) => command.includes(token))) {
        violations.push({ kind: 'boundedness_command_missing', path: contractPath, detail: `${laneId} missing validation command token: ${token}` });
      }
    }
    const refs = Array.isArray(lane.evidence_refs) ? lane.evidence_refs.map(String) : [];
    if (refs.length < 3) {
      violations.push({ kind: 'boundedness_evidence_refs_incomplete', path: contractPath, detail: `${laneId} needs at least three evidence refs.` });
    }
    for (const ref of refs) {
      if (!fs.existsSync(abs(ref))) {
        violations.push({ kind: 'boundedness_evidence_ref_missing', path: contractPath, detail: `${laneId} evidence ref is missing: ${ref}` });
      }
    }
  }
}

function validateNote(notePath: string, note: string, violations: Violation[]): void {
  for (const token of ['Release policy', 'Required boundedness dimensions', 'Acceptance criteria', 'Validation commands']) {
    if (!note.includes(token)) {
      violations.push({ kind: 'boundedness_note_token_missing', path: notePath, detail: `Missing section token: ${token}` });
    }
  }
  for (const token of REQUIRED_DIMENSIONS) {
    if (!note.includes(token)) {
      violations.push({ kind: 'boundedness_note_dimension_missing', path: notePath, detail: `Missing boundedness dimension: ${token}` });
    }
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Kernel Sentinel Boundedness Repair Lane Guard');
  lines.push('');
  lines.push(`- Generated at: ${payload.generated_at}`);
  lines.push(`- Revision: ${payload.revision}`);
  lines.push(`- Pass: ${payload.ok}`);
  lines.push(`- Contract: ${payload.contract_path}`);
  lines.push(`- Note: ${payload.note_path}`);
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
      kind: 'controlled_boundedness_repair_lane_violation',
      path: args.contractPath,
      detail: 'Controlled failure proves strict mode rejects incomplete boundedness repair lanes.',
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'kernel_sentinel_boundedness_repair_lane_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    note_path: args.notePath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      dimensions: (contract.boundedness_dimensions || []).length,
      repair_lanes: (contract.repair_lanes || []).length,
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
