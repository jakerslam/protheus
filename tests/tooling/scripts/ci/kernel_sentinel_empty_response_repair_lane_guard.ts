#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'observability/reports/kernel_sentinel_empty_response_repair_lane.json';
const DEFAULT_NOTE = 'observability/reports/KERNEL_SENTINEL_EMPTY_RESPONSE_REPAIR_LANE.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/kernel_sentinel_empty_response_repair_lane_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/KERNEL_SENTINEL_EMPTY_RESPONSE_REPAIR_LANE_GUARD_CURRENT.md';

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_SIGNATURES = [
  'empty_assistant_response',
  'final_llm_empty',
  'tool_pending_without_final_synthesis',
  'system_fallback_suppressed_without_operator_diagnostic',
];

const REQUIRED_COMMAND_TOKENS = [
  'workflow_visibility_diagnostics_classify_empty_llm_reply_without_system_injection',
  'workflow_self_play_empty_reply_keeps_trace_diagnostic_from_beginning_to_end',
  'ops:ksent:empty-response-repair:guard',
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

function parseArgs(argv: string[]) {
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

function requireValues(violations: Violation[], relPath: string, kind: string, values: unknown, expected: string[]): string[] {
  const actual = Array.isArray(values) ? values.map(String) : [];
  for (const id of expected) {
    if (!actual.includes(id)) violations.push({ kind, path: relPath, detail: `Missing required value: ${id}` });
  }
  return actual;
}

function validateContract(contract: any, contractPath: string, violations: Violation[]): void {
  if (contract.type !== 'kernel_sentinel_empty_response_repair_lane') {
    violations.push({ kind: 'empty_response_contract_type_invalid', path: contractPath, detail: 'Contract type mismatch.' });
  }
  if (contract.owner_domain !== 'observability') {
    violations.push({ kind: 'empty_response_owner_invalid', path: contractPath, detail: 'Owner domain must be observability.' });
  }
  if (contract.source_todo !== 'KSENT-ISSUE-EMPTY-RESPONSES') {
    violations.push({ kind: 'empty_response_source_todo_invalid', path: contractPath, detail: 'Source TODO must be KSENT-ISSUE-EMPTY-RESPONSES.' });
  }
  if (contract.policy?.human_review_required !== true || contract.policy?.safe_to_auto_apply_patch !== false) {
    violations.push({ kind: 'empty_response_policy_invalid', path: contractPath, detail: 'Repair lane must require review and forbid auto-apply.' });
  }
  if (contract.policy?.system_authored_visible_fallback_allowed !== false) {
    violations.push({ kind: 'empty_response_visible_fallback_policy_invalid', path: contractPath, detail: 'Visible system fallback must remain forbidden.' });
  }
  requireValues(violations, contractPath, 'empty_response_signature_missing', contract.symptom_signatures, REQUIRED_SIGNATURES);
  const lanes = Array.isArray(contract.repair_lanes) ? contract.repair_lanes : [];
  if (lanes.length === 0) violations.push({ kind: 'empty_response_repair_lane_missing', path: contractPath, detail: 'At least one repair lane is required.' });
  for (const lane of lanes) {
    const laneId = String(lane.id || '<missing>');
    for (const field of ['owner', 'failure_signature', 'root_cause_hypothesis', 'concrete_next_action']) {
      if (!String(lane[field] || '').trim()) violations.push({ kind: 'empty_response_lane_field_missing', path: contractPath, detail: `${laneId} missing ${field}.` });
    }
    const acceptance = Array.isArray(lane.acceptance_criteria) ? lane.acceptance_criteria.map(String) : [];
    if (acceptance.length < 4) violations.push({ kind: 'empty_response_acceptance_incomplete', path: contractPath, detail: `${laneId} needs at least four acceptance criteria.` });
    const commands = Array.isArray(lane.replay_validation_commands) ? lane.replay_validation_commands.map(String) : [];
    for (const token of REQUIRED_COMMAND_TOKENS) {
      if (!commands.some((command) => command.includes(token))) {
        violations.push({ kind: 'empty_response_command_missing', path: contractPath, detail: `${laneId} missing validation command token: ${token}` });
      }
    }
    const refs = Array.isArray(lane.evidence_refs) ? lane.evidence_refs.map(String) : [];
    if (refs.length < 4) violations.push({ kind: 'empty_response_evidence_refs_incomplete', path: contractPath, detail: `${laneId} needs at least four evidence refs.` });
    for (const ref of refs) {
      if (!fs.existsSync(abs(ref))) violations.push({ kind: 'empty_response_evidence_ref_missing', path: contractPath, detail: `${laneId} evidence ref missing: ${ref}` });
    }
  }
}

function validateNote(notePath: string, note: string, violations: Violation[]): void {
  for (const token of ['Release policy', 'Symptom signatures', 'Acceptance criteria', 'Validation commands']) {
    if (!note.includes(token)) violations.push({ kind: 'empty_response_note_token_missing', path: notePath, detail: `Missing section token: ${token}` });
  }
  for (const token of REQUIRED_SIGNATURES) {
    if (!note.includes(token)) violations.push({ kind: 'empty_response_note_signature_missing', path: notePath, detail: `Missing signature token: ${token}` });
  }
}

function markdown(payload: any): string {
  const lines = [
    '# Kernel Sentinel Empty-Response Repair Lane Guard',
    '',
    `- Generated at: ${payload.generated_at}`,
    `- Revision: ${payload.revision}`,
    `- Pass: ${payload.ok}`,
    `- Contract: ${payload.contract_path}`,
    '',
    '## Summary',
  ];
  for (const [key, value] of Object.entries(payload.summary)) lines.push(`- ${key}: ${value}`);
  lines.push('', '## Violations');
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
    violations.push({ kind: 'controlled_empty_response_repair_lane_violation', path: args.contractPath, detail: 'Controlled failure proves strict mode rejects incomplete empty-response repair lanes.' });
  }
  const payload = {
    ok: violations.length === 0,
    type: 'kernel_sentinel_empty_response_repair_lane_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    note_path: args.notePath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      symptom_signatures: (contract.symptom_signatures || []).length,
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
