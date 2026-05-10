#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import {
  SHELL_SOCKET_ROUTES,
  ShellSocketCapabilityId,
} from '../client/shell_socket_gateway_client.ts';

const ROOT = process.cwd();
const DEFAULT_BASE_URL = process.env.SHELL_SOCKET_LIVE_BASE_URL || 'http://127.0.0.1:5173';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_socket_live_probe_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_SOCKET_LIVE_PROBE_CURRENT.md';

type ProbeStep = {
  capability_id: ShellSocketCapabilityId;
  method: string;
  path: string;
  status: number;
  ok: boolean;
  required_ok: boolean;
  note: string;
  payload_bytes: number;
  required_fields: string[];
  missing_fields: string[];
  forbidden_fields: string[];
};

type RequestResult = {
  status: number;
  ok: boolean;
  payload: Record<string, unknown>;
  error: string;
};

const FORBIDDEN_DEFAULT_FIELDS = new Set([
  'raw',
  'root',
  'raw_payload',
  'raw_runtime_state',
  'all_messages',
  'conversation_tree',
  'raw_tool_result',
  'tool_result',
  'trace_body',
  'workflow_graph',
]);

const REQUIRED_LIVE_CAPABILITIES: ShellSocketCapabilityId[] = SHELL_SOCKET_ROUTES.map((route) => route.capabilityId);

function readArg(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const found = process.argv.find((arg) => arg.startsWith(prefix));
  return found ? found.slice(prefix.length) : fallback;
}

function readBool(name: string, fallback = false): boolean {
  const found = process.argv.find((arg) => arg === `--${name}` || arg.startsWith(`--${name}=`));
  if (!found) return fallback;
  if (found === `--${name}`) return true;
  const value = found.slice(`--${name}=`.length).trim().toLowerCase();
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function abs(rel: string): string {
  return path.resolve(ROOT, rel);
}

function writeFile(relOrAbs: string, content: string): void {
  const target = path.isAbsolute(relOrAbs) ? relOrAbs : abs(relOrAbs);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content);
}

function cleanUrl(raw: string): string {
  return String(raw || '').replace(/\/+$/, '');
}

function routePath(capabilityId: ShellSocketCapabilityId, params: Record<string, unknown> = {}): string {
  const route = SHELL_SOCKET_ROUTES.find((row) => row.capabilityId === capabilityId);
  if (!route) throw new Error(`unknown_socket_capability:${capabilityId}`);
  let out = route.path;
  for (const key of route.pathParams || []) {
    const value = params[key];
    if (value == null || value === '') throw new Error(`missing_path_param:${capabilityId}:${key}`);
    out = out.replace(`{${key}}`, encodeURIComponent(String(value)));
  }
  const query = new URLSearchParams();
  for (const key of route.queryParams || []) {
    const value = params[key];
    if (value == null || value === '') continue;
    query.set(key, String(value));
  }
  const qs = query.toString();
  return qs ? `${out}?${qs}` : out;
}

function payloadBytes(payload: unknown): number {
  return Buffer.byteLength(JSON.stringify(payload || {}), 'utf8');
}

function valueAtPath(payload: Record<string, unknown>, field: string): unknown {
  return field.split('.').reduce<unknown>((current, part) => {
    if (!current || typeof current !== 'object') return undefined;
    return (current as Record<string, unknown>)[part];
  }, payload);
}

function collectForbiddenFields(value: unknown, trail = '$'): string[] {
  if (!value || typeof value !== 'object') return [];
  if (Array.isArray(value)) {
    return value.flatMap((item, index) => collectForbiddenFields(item, `${trail}[${index}]`));
  }
  const out: string[] = [];
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    const next = `${trail}.${key}`;
    if (FORBIDDEN_DEFAULT_FIELDS.has(key)) out.push(next);
    out.push(...collectForbiddenFields(child, next));
  }
  return out;
}

async function requestJson(baseUrl: string, method: string, urlPath: string, body?: unknown, timeoutMs = 8000): Promise<RequestResult> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(`${baseUrl}${urlPath}`, {
      method,
      headers: method === 'POST' ? { accept: 'application/json', 'content-type': 'application/json' } : { accept: 'application/json' },
      body: method === 'POST' ? JSON.stringify(body || {}) : undefined,
      signal: controller.signal,
    });
    const text = await response.text();
    let payload: Record<string, unknown> = {};
    try {
      payload = text ? JSON.parse(text) : {};
    } catch {
      payload = { parse_error: 'non_json_response', text_preview: text.slice(0, 240) };
    }
    return { status: response.status, ok: response.ok, payload, error: '' };
  } catch (error) {
    const message = error && typeof error === 'object' && 'name' in error && (error as Error).name === 'AbortError'
      ? 'request_timeout'
      : String(error && typeof error === 'object' && 'message' in error ? (error as Error).message : error);
    return { status: 0, ok: false, payload: {}, error: message };
  } finally {
    clearTimeout(timer);
  }
}

async function step(
  baseUrl: string,
  capabilityId: ShellSocketCapabilityId,
  requiredFields: string[],
  params: Record<string, unknown> = {},
  body?: unknown,
  acceptStatus: (status: number, payload: Record<string, unknown>) => boolean = (status) => status >= 200 && status < 300,
): Promise<{ step: ProbeStep; payload: Record<string, unknown> }> {
  const route = SHELL_SOCKET_ROUTES.find((row) => row.capabilityId === capabilityId);
  if (!route) throw new Error(`unknown_socket_capability:${capabilityId}`);
  const urlPath = routePath(capabilityId, params);
  const result = await requestJson(baseUrl, route.method, urlPath, body);
  const missing = requiredFields.filter((field) => valueAtPath(result.payload, field) == null);
  const forbidden = collectForbiddenFields(result.payload);
  const requiredOk = acceptStatus(result.status, result.payload) && missing.length === 0 && forbidden.length === 0;
  return {
    payload: result.payload,
    step: {
      capability_id: capabilityId,
      method: route.method,
      path: urlPath,
      status: result.status,
      ok: result.ok,
      required_ok: requiredOk,
      note: result.error || String(result.payload.reason_code || result.payload.error || ''),
      payload_bytes: payloadBytes(result.payload),
      required_fields: requiredFields,
      missing_fields: missing,
      forbidden_fields: forbidden,
    },
  };
}

function firstString(...values: unknown[]): string {
  for (const value of values) {
    if (typeof value === 'string' && value.trim()) return value.trim();
    if (Array.isArray(value)) {
      const first = value.find((entry) => typeof entry === 'string' && entry.trim());
      if (first) return String(first).trim();
    }
  }
  return '';
}

function messageRows(payload: Record<string, unknown>): Record<string, unknown>[] {
  const direct = payload.message_window;
  if (Array.isArray(direct)) return direct.filter((row): row is Record<string, unknown> => !!row && typeof row === 'object');
  if (direct && typeof direct === 'object' && Array.isArray((direct as Record<string, unknown>).rows)) {
    return ((direct as Record<string, unknown>).rows as unknown[]).filter((row): row is Record<string, unknown> => !!row && typeof row === 'object');
  }
  return [];
}

function markdownReport(result: Record<string, unknown>, steps: ProbeStep[]): string {
  const missingCapabilities = Array.isArray(result.missing_live_capabilities)
    ? result.missing_live_capabilities
    : [];
  const passedCapabilities = Array.isArray(result.passed_live_capabilities)
    ? result.passed_live_capabilities
    : [];
  const failedCapabilities = Array.isArray(result.failed_live_capabilities)
    ? result.failed_live_capabilities
    : [];
  const lines = [
    '# Shell Socket Live Probe',
    '',
    `- ok: ${String(result.ok)}`,
    `- live_parity_complete: ${String(result.live_parity_complete)}`,
    `- live_available: ${String(result.live_available)}`,
    `- base_url: ${String(result.base_url)}`,
    `- require_live: ${String(result.require_live)}`,
    `- step_count: ${steps.length}`,
    `- required_live_capabilities: ${Array.isArray(result.required_live_capabilities) ? result.required_live_capabilities.length : 0}`,
    `- passed_live_capabilities: ${passedCapabilities.length}`,
    `- failed_live_capabilities: ${failedCapabilities.length}`,
    `- missing_live_capabilities: ${missingCapabilities.length}`,
    `- violation_count: ${Array.isArray(result.violations) ? result.violations.length : 0}`,
    '',
    '## Failed Live Capabilities',
  ];
  if (failedCapabilities.length === 0) lines.push('- none');
  for (const capability of failedCapabilities) lines.push(`- ${String(capability)}`);
  lines.push(
    '',
    '## Missing Live Capabilities',
  );
  if (missingCapabilities.length === 0) lines.push('- none');
  for (const capability of missingCapabilities) lines.push(`- ${String(capability)}`);
  lines.push(
    '',
    '## Passed Live Capabilities',
  );
  if (passedCapabilities.length === 0) lines.push('- none');
  for (const capability of passedCapabilities) lines.push(`- ${String(capability)}`);
  lines.push(
    '',
    '## Steps',
  );
  for (const row of steps) {
    lines.push(`- ${row.capability_id}: status=${row.status} required_ok=${row.required_ok} bytes=${row.payload_bytes} note=${row.note || 'none'}`);
  }
  return `${lines.join('\n')}\n`;
}

async function run(): Promise<Record<string, unknown>> {
  const baseUrl = cleanUrl(readArg('base-url', DEFAULT_BASE_URL));
  const outJson = readArg('out-json', DEFAULT_OUT_JSON);
  const outMarkdown = readArg('out-markdown', DEFAULT_OUT_MARKDOWN);
  const requireLive = readBool('require-live', false);
  const steps: ProbeStep[] = [];
  const violations: string[] = [];

  const runtime = await step(baseUrl, 'get_runtime_status', ['state', 'label', 'receipt_ref']);
  steps.push(runtime.step);
  const liveAvailable = runtime.step.required_ok;
  if (!liveAvailable) {
    violations.push(`live_socket_unavailable:${runtime.step.status || runtime.step.note || 'unknown'}`);
  }

  let agentId = '';
  let sessionId = '';
  if (liveAvailable) {
    const agents = await step(baseUrl, 'list_agents', ['agent_ids', 'receipt_ref'], { limit: 1 });
    steps.push(agents.step);
    agentId = firstString(
      agents.payload.active_agent_id,
      agents.payload.agent_ids,
      Array.isArray(agents.payload.agents) ? (agents.payload.agents[0] as Record<string, unknown> | undefined)?.id : '',
    );

    const search = await step(baseUrl, 'search', ['query_id', 'hits', 'receipt_ref'], { q: 'shell socket', limit: 1 });
    steps.push(search.step);

    const rejectedInput = await step(
      baseUrl,
      'submit_input',
      ['accepted', 'rejected', 'reason_code', 'receipt_ref'],
      {},
      {},
      (status, payload) => status === 400 && payload.rejected === true,
    );
    steps.push(rejectedInput.step);

    const rejectedIssue = await step(
      baseUrl,
      'submit_issue',
      ['accepted', 'rejected', 'reason_code', 'receipt_ref'],
      {},
      {},
      (status, payload) => status === 400 && payload.rejected === true,
    );
    steps.push(rejectedIssue.step);

    const rejectedTerminal = await step(
      baseUrl,
      'submit_terminal_command',
      ['accepted', 'rejected', 'reason_code', 'receipt_ref'],
      {},
      {},
      (status, payload) => status === 400 && payload.rejected === true,
    );
    steps.push(rejectedTerminal.step);

    const approval = await step(
      baseUrl,
      'submit_approval_decision',
      ['accepted', 'rejected', 'reason_code', 'receipt_ref'],
      { approval_id: 'probe-approval' },
      { decision: 'approve' },
      (status, payload) => status >= 400 && status < 500 && payload.rejected === true,
    );
    steps.push(approval.step);

    if (agentId) {
      const rejectedModel = await step(
        baseUrl,
        'set_model',
        ['accepted', 'rejected', 'reason_code', 'receipt_ref'],
        { agent_id: agentId },
        {},
        (_status, payload) => payload.rejected === true,
      );
      steps.push(rejectedModel.step);

      const rejectedGitTree = await step(
        baseUrl,
        'set_git_tree',
        ['accepted', 'rejected', 'reason_code', 'receipt_ref'],
        { agent_id: agentId },
        {},
        (_status, payload) => payload.rejected === true,
      );
      steps.push(rejectedGitTree.step);

      const sessions = await step(baseUrl, 'list_sessions', ['session_ids', 'receipt_ref'], { agent_id: agentId, limit: 1 });
      steps.push(sessions.step);
      sessionId = firstString(sessions.payload.active_session_id, sessions.payload.session_ids);
    }
    if (sessionId) {
      const messages = await step(baseUrl, 'get_message_window', ['message_window', 'receipt_ref'], { session_id: sessionId, limit: 2 });
      steps.push(messages.step);
      const rows = messageRows(messages.payload);
      const detailRef = firstString(...rows.map((row) => row.detail_ref), messages.payload.detail_refs) || 'probe-missing-detail';
      const events = await step(baseUrl, 'subscribe_events', ['event_id', 'event_kind', 'receipt_refs'], { session_id: sessionId });
      steps.push(events.step);
      const detail = await step(
        baseUrl,
        'get_message_detail',
        ['detail_id', 'detail_kind', 'detail_projection', 'receipt_ref'],
        { detail_ref: detailRef, view: 'summary', limit: 1 },
        undefined,
        (status) => status === 200 || status === 404,
      );
      steps.push(detail.step);
    }
  }

  for (const row of steps) {
    if (!row.required_ok && (liveAvailable || requireLive)) {
      violations.push(`step_failed:${row.capability_id}:${row.status || row.note || 'unknown'}`);
    }
  }
  const exercisedCapabilities = Array.from(new Set(steps.map((row) => row.capability_id)));
  const exercisedSet = new Set(exercisedCapabilities);
  const missingLiveCapabilities = REQUIRED_LIVE_CAPABILITIES.filter((capabilityId) => !exercisedSet.has(capabilityId));
  const failedLiveCapabilities = Array.from(new Set(steps.filter((row) => !row.required_ok).map((row) => row.capability_id)));
  const failedSet = new Set(failedLiveCapabilities);
  const passedLiveCapabilities = REQUIRED_LIVE_CAPABILITIES.filter((capabilityId) => exercisedSet.has(capabilityId) && !failedSet.has(capabilityId));
  const liveParityComplete = liveAvailable && missingLiveCapabilities.length === 0 && failedLiveCapabilities.length === 0;
  if (requireLive && missingLiveCapabilities.length > 0) {
    violations.push(`live_capabilities_not_exercised:${missingLiveCapabilities.join(',')}`);
  }
  const ok = violations.length === 0 || (!requireLive && !liveAvailable);
  const result = {
    ok,
    type: 'shell_socket_live_probe',
    base_url: baseUrl,
    require_live: requireLive,
    live_available: liveAvailable,
    live_parity_complete: liveParityComplete,
    step_count: steps.length,
    required_live_capabilities: REQUIRED_LIVE_CAPABILITIES,
    exercised_capabilities: exercisedCapabilities,
    passed_live_capabilities: passedLiveCapabilities,
    failed_live_capabilities: failedLiveCapabilities,
    missing_live_capabilities: missingLiveCapabilities,
    violations,
    steps,
    note: liveAvailable
      ? 'live_gateway_socket_routes_checked'
      : 'live_gateway_socket_unavailable_soft_pass_unless_require_live',
  };
  writeFile(outJson, `${JSON.stringify(result, null, 2)}\n`);
  writeFile(outMarkdown, markdownReport(result, steps));
  return result;
}

run()
  .then((result) => {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    process.exitCode = result.ok ? 0 : 1;
  })
  .catch((error) => {
    const result = {
      ok: false,
      type: 'shell_socket_live_probe',
      error: String(error && error.message ? error.message : error),
    };
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    process.exitCode = 1;
  });
