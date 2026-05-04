#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'validation/regression/contracts/shell_runtime_payload_budget_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_runtime_payload_budget_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_RUNTIME_PAYLOAD_BUDGET_GUARD_CURRENT.md';

type RuntimeCase = {
  id: string;
  endpoint_budget: string;
  endpoint_pattern: string;
  sample_row_count: number;
  required_top_level_fields?: string[];
};

type Contract = {
  type?: string;
  owner?: string;
  status?: string;
  interface_payload_budget_contract_path: string;
  policy_refs?: string[];
  policy_doc_required_tokens?: string[];
  required_endpoint_budgets?: string[];
  runtime_cases?: RuntimeCase[];
};

type EndpointBudget = {
  name: string;
  endpoint_patterns?: string[];
  max_response_bytes: number;
  max_array_items: number;
  max_object_depth: number;
  max_string_chars: number;
  max_top_level_fields: number;
  allowed_top_level_fields?: string[];
  requires_cursor: boolean;
  cursor_fields?: string[];
  requires_detail_refs: boolean;
  required_ref_fields?: string[];
};

type PayloadReport = {
  case_id: string;
  endpoint_budget: string;
  endpoint_pattern: string;
  response_bytes: number;
  top_level_fields: number;
  max_depth: number;
  max_array_items_seen: number;
  max_string_chars_seen: number;
};

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_CASES = [
  'session_list_runtime_projection',
  'chat_message_window_runtime_projection',
  'runtime_event_runtime_projection',
];

const FORBIDDEN_KEY_PATTERN = /(^|_)(raw|root|full_state|all_messages|conversation_tree|raw_payload|tool_input|tool_result|trace_body|decision_trace|plan_graph|workflow_graph|execution_observation|runtime_quality|eval_payload|artifact_body|file_output|folder_tree|policy_decision|authorization_state)(_|$)/;

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readJson<T>(relPath: string): T {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8')) as T;
}

function exists(relPath: string): boolean {
  return fs.existsSync(abs(relPath));
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function valueAtPath(value: any, dottedPath: string): any {
  return dottedPath.split('.').reduce((current, key) => (current == null ? undefined : current[key]), value);
}

function maxDepth(value: any): number {
  if (value == null || typeof value !== 'object') return 0;
  if (Array.isArray(value)) return 1 + Math.max(0, ...value.map((row) => maxDepth(row)));
  return 1 + Math.max(0, ...Object.values(value).map((row) => maxDepth(row)));
}

function scanValue(value: any, forbiddenFields: Set<string>, pathParts: string[], violations: Violation[], pathRel: string, stats: { maxArray: number; maxString: number }): void {
  if (typeof value === 'string') {
    stats.maxString = Math.max(stats.maxString, value.length);
    return;
  }
  if (Array.isArray(value)) {
    stats.maxArray = Math.max(stats.maxArray, value.length);
    value.forEach((row, index) => scanValue(row, forbiddenFields, [...pathParts, String(index)], violations, pathRel, stats));
    return;
  }
  if (!value || typeof value !== 'object') return;
  for (const [key, nested] of Object.entries(value)) {
    if (forbiddenFields.has(key) || FORBIDDEN_KEY_PATTERN.test(key)) {
      push(violations, 'runtime_payload_forbidden_field', pathRel, `${pathParts.concat(key).join('.')} uses forbidden default payload key ${key}.`);
    }
    scanValue(nested, forbiddenFields, [...pathParts, key], violations, pathRel, stats);
  }
}

function buildPayload(testCase: RuntimeCase, controlledViolation: boolean): any {
  if (testCase.id === 'session_list_runtime_projection') {
    const sessions = Array.from({ length: testCase.sample_row_count }, (_, index) => ({
      id: `session-${index}`,
      title_preview: `Session ${index}`,
      status: index % 2 === 0 ? 'active' : 'idle',
      updated_at: '2026-05-01T00:00:00.000Z',
      detail_ref: `detail://session/${index}`,
    }));
    return {
      sessions,
      session_ids: sessions.map((row) => row.id),
      active_session_id: sessions[0]?.id || null,
      last_message_previews: sessions.map((row) => ({ session_id: row.id, content_preview: 'bounded preview' })),
      message_counts: sessions.map((row) => ({ session_id: row.id, count: 3 })),
      next_cursor: 'cursor-session-next',
      detail_refs: sessions.map((row) => row.detail_ref),
      receipt_ref: 'receipt://session-list',
      correlation_id: 'trace-runtime-budget',
    };
  }
  if (testCase.id === 'chat_message_window_runtime_projection') {
    const rows = Array.from({ length: testCase.sample_row_count }, (_, index) => ({
      id: `message-${index}`,
      origin_kind: index % 2 === 0 ? 'user' : 'assistant',
      timestamp: '2026-05-01T00:00:00.000Z',
      status: 'complete',
      content_preview: `Bounded message preview ${index}`,
      line_count: 1,
      detail_ref: `detail://message/${index}`,
      tool_summary_count: index % 10 === 0 ? 1 : 0,
    }));
    const payload: any = {
      ok: true,
      agent_id: 'agent-runtime-budget',
      active_session_id: 'session-runtime-budget',
      message_window: {
        rows,
        window_start_id: rows[0]?.id,
        window_end_id: rows[rows.length - 1]?.id,
        before_cursor: 'cursor-before',
        after_cursor: 'cursor-after',
      },
      message_count: rows.length,
      total_count: 10000,
      has_more: true,
      detail_refs: rows.map((row) => row.detail_ref),
      receipt_ref: 'receipt://message-window',
      correlation_id: 'trace-runtime-budget',
    };
    if (controlledViolation) payload.all_messages = rows.concat(rows);
    return payload;
  }
  const projectionRows = Array.from({ length: testCase.sample_row_count }, (_, index) => ({
    id: `event-row-${index}`,
    content_preview: `Runtime event projection ${index}`,
    detail_ref: `detail://event/${index}`,
  }));
  const payload: any = {
    event_id: 'event-runtime-budget',
    event_kind: 'message_delta',
    agent_id: 'agent-runtime-budget',
    session_id: 'session-runtime-budget',
    display_projection: {
      rows: projectionRows,
      status: 'running',
      content_preview: 'bounded event projection',
    },
    status_label: 'Running',
    cursor_refs: ['cursor-event-next'],
    detail_refs: projectionRows.map((row) => row.detail_ref),
    receipt_refs: ['receipt://runtime-event'],
    correlation_id: 'trace-runtime-budget',
  };
  if (controlledViolation) payload.display_projection.raw_tool_result = { secret: 'raw payload should fail' };
  return payload;
}

function validatePayload(testCase: RuntimeCase, budget: EndpointBudget, forbiddenFields: Set<string>, payload: any, violations: Violation[], contractPath: string): PayloadReport {
  const bytes = Buffer.byteLength(JSON.stringify(payload), 'utf8');
  const topFields = Object.keys(payload).length;
  const depth = maxDepth(payload);
  const stats = { maxArray: 0, maxString: 0 };
  scanValue(payload, forbiddenFields, [testCase.id], violations, contractPath, stats);
  const allowed = new Set(budget.allowed_top_level_fields || []);
  for (const field of Object.keys(payload)) {
    if (!allowed.has(field)) push(violations, 'runtime_payload_top_level_field_not_allowed', contractPath, `${testCase.id} emits top-level field ${field} outside ${budget.name}.`);
  }
  for (const field of testCase.required_top_level_fields || []) {
    if (!(field in payload)) push(violations, 'runtime_payload_required_field_missing', contractPath, `${testCase.id} missing required field ${field}.`);
  }
  if (bytes > budget.max_response_bytes) push(violations, 'runtime_payload_response_bytes_exceeded', contractPath, `${testCase.id} emitted ${bytes} bytes over ${budget.max_response_bytes}.`);
  if (topFields > budget.max_top_level_fields) push(violations, 'runtime_payload_top_level_budget_exceeded', contractPath, `${testCase.id} emitted ${topFields} top-level fields over ${budget.max_top_level_fields}.`);
  if (depth > budget.max_object_depth) push(violations, 'runtime_payload_depth_exceeded', contractPath, `${testCase.id} depth ${depth} exceeds ${budget.max_object_depth}.`);
  if (stats.maxArray > budget.max_array_items) push(violations, 'runtime_payload_array_budget_exceeded', contractPath, `${testCase.id} array length ${stats.maxArray} exceeds ${budget.max_array_items}.`);
  if (stats.maxString > budget.max_string_chars) push(violations, 'runtime_payload_string_budget_exceeded', contractPath, `${testCase.id} string length ${stats.maxString} exceeds ${budget.max_string_chars}.`);
  if (budget.requires_cursor && !(budget.cursor_fields || []).some((field) => valueAtPath(payload, field) != null)) {
    push(violations, 'runtime_payload_cursor_missing', contractPath, `${testCase.id} must expose at least one declared cursor/window field.`);
  }
  if (budget.requires_detail_refs) {
    for (const refField of budget.required_ref_fields || []) {
      if (valueAtPath(payload, refField) == null) push(violations, 'runtime_payload_ref_missing', contractPath, `${testCase.id} missing required ref field ${refField}.`);
    }
  }
  return {
    case_id: testCase.id,
    endpoint_budget: budget.name,
    endpoint_pattern: testCase.endpoint_pattern,
    response_bytes: bytes,
    top_level_fields: topFields,
    max_depth: depth,
    max_array_items_seen: stats.maxArray,
    max_string_chars_seen: stats.maxString,
  };
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Shell Runtime Payload Budget Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `cases: ${report.case_reports.length}`,
    `violations: ${report.violations.length}`,
    '',
    '## Runtime Cases',
  ];
  for (const row of report.case_reports) {
    lines.push(`- ${row.case_id}: bytes=${row.response_bytes}; depth=${row.max_depth}; max_array=${row.max_array_items_seen}; budget=${row.endpoint_budget}`);
  }
  lines.push('', '## Violations');
  if (!report.violations.length) lines.push('- none');
  for (const violation of report.violations) lines.push(`- ${violation.kind} at \`${violation.path}\`: ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
const contractPath = cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const contract = readJson<Contract>(contractPath);
const interfaceBudget = readJson<any>(contract.interface_payload_budget_contract_path);
const endpointBudgets = new Map((interfaceBudget.default_endpoint_budgets || []).map((row: EndpointBudget) => [row.name, row]));
const forbiddenFields = new Set([...(interfaceBudget.forbidden_default_payload_fields || [])]);
const violations: Violation[] = [];

if (contract.type !== 'shell_runtime_payload_budget_contract') push(violations, 'wrong_contract_type', contractPath, 'Expected shell_runtime_payload_budget_contract.');
if (contract.owner !== 'assurance.validation') push(violations, 'wrong_contract_owner', contractPath, 'Owner must be assurance.validation.');
if (contract.status !== 'enforced') push(violations, 'wrong_contract_status', contractPath, 'Status must be enforced.');
for (const ref of contract.policy_refs || []) if (!exists(ref)) push(violations, 'missing_policy_ref', ref, 'Runtime payload budget contract references a missing policy.');
for (const token of contract.policy_doc_required_tokens || []) {
  const policy = fs.readFileSync(abs('docs/workspace/interface_payload_budget_policy.md'), 'utf8');
  if (!policy.includes(token)) push(violations, 'payload_budget_policy_missing_token', 'docs/workspace/interface_payload_budget_policy.md', `Missing token: ${token}`);
}
for (const required of contract.required_endpoint_budgets || []) {
  if (!endpointBudgets.has(required)) push(violations, 'runtime_payload_budget_missing', contract.interface_payload_budget_contract_path, `${required} is not declared in interface payload budget contract.`);
}
const cases = contract.runtime_cases || [];
for (const requiredCase of REQUIRED_CASES) {
  if (!cases.some((row) => row.id === requiredCase)) push(violations, 'runtime_payload_case_missing', contractPath, `Missing runtime case ${requiredCase}.`);
}
const caseReports: PayloadReport[] = [];
for (const testCase of cases) {
  const budget = endpointBudgets.get(testCase.endpoint_budget) as EndpointBudget | undefined;
  if (!budget) continue;
  if (!(budget.endpoint_patterns || []).includes(testCase.endpoint_pattern)) {
    push(violations, 'runtime_payload_endpoint_pattern_not_budgeted', contractPath, `${testCase.id} pattern ${testCase.endpoint_pattern} is not budgeted by ${budget.name}.`);
  }
  const payload = buildPayload(testCase, includeControlledViolation);
  caseReports.push(validatePayload(testCase, budget, forbiddenFields, payload, violations, contractPath));
}

const report = {
  ok: violations.length === 0,
  type: 'shell_runtime_payload_budget_guard',
  revision: currentRevision(ROOT),
  contract_path: contractPath,
  interface_payload_budget_contract_path: contract.interface_payload_budget_contract_path,
  case_reports: caseReports,
  violations,
};

writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict: common.strict, ok: report.ok });
