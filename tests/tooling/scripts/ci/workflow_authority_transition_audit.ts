#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const OUT = 'core/local/artifacts/workflow_authority_transition_audit_current.json';

type AuthorityClass =
  | 'llm_submission'
  | 'system_guard'
  | 'telemetry_only'
  | 'forbidden_auto_decision';

type Failure = { id: string; detail: string };
type TransitionSpec = {
  id: string;
  authority_class: AuthorityClass;
  source_path: string;
  required_tokens: string[];
  rationale: string;
};

const TRANSITIONS: TransitionSpec[] = [
  {
    id: 'gate_1_need_tool_access',
    authority_class: 'llm_submission',
    source_path: 'core/layer0/ops/src/app_plane_parts/030-run-chat-ui_parts/070-turn-tool-decision-tree.rs',
    required_tokens: [
      'gate_1_decision_source = "pending_llm_submission"',
      '"gate_submission"',
      '"llm_submission"',
      '"accepted"',
      '"resume_token"',
    ],
    rationale: 'Gate 1 must wait for an LLM menu submission instead of defaulting to a Shell/system no-tool route.',
  },
  {
    id: 'tool_family_selection',
    authority_class: 'llm_submission',
    source_path: 'core/layer0/ops/src/app_plane_parts/030-run-chat-ui_parts/070-turn-tool-decision-tree.rs',
    required_tokens: [
      'let selected_tool_family = "unselected"',
      'let manual_tool_selection = true',
      'let tool_selection_authority = "llm_submitted_menu_or_text_input"',
      '"tool_family_selection_required": true',
    ],
    rationale: 'Tool-family selection must stay unselected until LLM menu/text input supplies it.',
  },
  {
    id: 'tool_payload_entry',
    authority_class: 'llm_submission',
    source_path: 'core/layer0/ops/src/app_plane_parts/030-run-chat-ui_parts/070-turn-tool-decision-tree.rs',
    required_tokens: [
      '"request_payload_entry_required": true',
      '"system_may_select_tools": false',
      '"tool_recommendations_allowed": false',
      '"automatic_tool_calls_allowed": automatic_tool_calls_allowed',
    ],
    rationale: 'Tool payloads must be entered through the LLM/user workflow instead of inferred by Shell code.',
  },
  {
    id: 'response_workflow_trace_export',
    authority_class: 'llm_submission',
    source_path: 'core/layer0/ops/src/app_plane_parts/030-run-chat-ui_parts/480-build-response-workflow-trace.rs',
    required_tokens: [
      'gate_submission',
      '"selection_authority": "llm_submission_only"',
      '"observed_true_without_gate_submission"',
      '"observed_tool_execution_without_gate_submission"',
    ],
    rationale: 'Workflow traces must report LLM-submission provenance and flag tool execution without a gate submission.',
  },
  {
    id: 'post_synthesis_visibility_guard',
    authority_class: 'system_guard',
    source_path: 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/047a-visible-response-provenance-guards.rs',
    required_tokens: [
      'response_workflow["system_chat_injection_used"] = json!(false)',
      'response_finalization["system_chat_injection_used"] = json!(false)',
      '"chat_injection_allowed": false',
    ],
    rationale: 'Post-synthesis guards may block or annotate unsafe output, but may not become user-visible chat authors.',
  },
  {
    id: 'live_eval_monitor_visibility',
    authority_class: 'telemetry_only',
    source_path: 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/081-part.rs',
    required_tokens: [
      'live_eval_monitor',
      '"chat_injection_allowed": false',
      '"attention_key"',
    ],
    rationale: 'Live eval monitoring observes finalized turns and emits attention telemetry only.',
  },
];

const FORBIDDEN_SCAN_PATHS = [
  'core/layer0/ops/src/app_plane_parts/030-run-chat-ui_parts/070-turn-tool-decision-tree.rs',
  'core/layer0/ops/src/app_plane_parts/030-run-chat-ui_parts/480-build-response-workflow-trace.rs',
  'core/layer0/ops/src/app_plane_parts/030-run-chat-ui_parts/010-run-chat-ui_parts/030-run-turn-persist-and-build-output.expr.rs',
  'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046-turn-workflow-library.rs',
  'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/050-message-finalization-and-payload_parts/050-finalize-message-finalization-and-payload_parts/000-combined.rs',
];

const FORBIDDEN_PATTERNS = [
  { id: 'auto_tool_calls_enabled', pattern: /\bautomatic_tool_calls_allowed\b\s*[:=]\s*true/g },
  { id: 'auto_decisions_enabled', pattern: /\bauto_decisions_disabled\b\s*[:=]\s*false/g },
  { id: 'system_may_select_tools', pattern: /"system_may_select_tools"\s*:\s*true/g },
  { id: 'semantic_classifier_active', pattern: /"semantic_route_classifier_active"\s*:\s*true/g },
  { id: 'info_classifier_active', pattern: /"info_task_route_classifier_active"\s*:\s*true/g },
  { id: 'workflow_classifier_active', pattern: /"workflow_route_classifier_active"\s*:\s*true/g },
  { id: 'system_selection_authority', pattern: /"selection_authority"\s*:\s*"system_auto"/g },
  { id: 'backend_selection_authority', pattern: /"selection_authority"\s*:\s*"backend_auto"/g },
];

function readText(rel: string): string {
  return fs.readFileSync(path.resolve(ROOT, rel), 'utf8');
}

function writeJson(rel: string, payload: unknown) {
  const abs = path.resolve(ROOT, rel);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
}

function forbiddenMatches(source: string, sourcePath: string) {
  const matches: Array<{ id: string; source_path: string; match: string }> = [];
  for (const item of FORBIDDEN_PATTERNS) {
    for (const match of source.matchAll(item.pattern)) {
      matches.push({
        id: item.id,
        source_path: sourcePath,
        match: String(match[0]).slice(0, 160),
      });
    }
  }
  return matches;
}

function run() {
  const strict = process.argv.includes('--strict') || process.argv.includes('--strict=1');
  const failures: Failure[] = [];
  const transition_rows = [];

  for (const spec of TRANSITIONS) {
    const source = readText(spec.source_path);
    const missing = spec.required_tokens.filter((token) => !source.includes(token));
    if (missing.length) {
      failures.push({ id: spec.id, detail: `missing_tokens=${missing.join('|')}` });
    }
    transition_rows.push({
      id: spec.id,
      authority_class: spec.authority_class,
      source_path: spec.source_path,
      status: missing.length ? 'missing_evidence' : 'classified',
      required_evidence_count: spec.required_tokens.length,
      missing,
      rationale: spec.rationale,
    });
  }

  const forbidden = FORBIDDEN_SCAN_PATHS.flatMap((sourcePath) => forbiddenMatches(readText(sourcePath), sourcePath));
  for (const match of forbidden) {
    failures.push({ id: match.id, detail: `${match.source_path}:${match.match}` });
  }
  transition_rows.push({
    id: 'forbidden_auto_decision_sweep',
    authority_class: 'forbidden_auto_decision' as AuthorityClass,
    source_path: FORBIDDEN_SCAN_PATHS.join(','),
    status: forbidden.length ? 'present' : 'absent',
    required_evidence_count: FORBIDDEN_PATTERNS.length,
    missing: [],
    forbidden_match_count: forbidden.length,
    rationale: 'Forbidden auto-decision sources must stay absent from workflow transition authority paths.',
  });

  const selfTestForbiddenDetection =
    forbiddenMatches('"system_may_select_tools": true\nlet auto_decisions_disabled = false;', 'self_test').length === 2;
  if (!selfTestForbiddenDetection) {
    failures.push({ id: 'self_test.forbidden_detection', detail: 'expected forbidden detector to catch synthetic auto-decision source' });
  }

  const counts = transition_rows.reduce<Record<string, number>>((acc, row: any) => {
    acc[row.authority_class] = (acc[row.authority_class] || 0) + 1;
    return acc;
  }, {});
  const payload = {
    ok: failures.length === 0,
    type: 'workflow_authority_transition_audit',
    generated_at: new Date().toISOString(),
    strict,
    summary: {
      transition_count: transition_rows.length,
      authority_class_counts: counts,
      forbidden_match_count: forbidden.length,
      failures: failures.length,
      self_test_forbidden_detection: selfTestForbiddenDetection,
    },
    transition_rows,
    forbidden_matches: forbidden,
    failures,
  };
  writeJson(OUT, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && failures.length) process.exit(1);
}

run();
