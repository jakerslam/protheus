#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import {
  SHELL_SOCKET_ROUTES,
  ShellSocketCapabilityId,
  ShellSocketGatewayClient,
} from '../client/shell_socket_gateway_client.ts';

const ROOT = process.cwd();
const DEFAULT_SOCKET_CONTRACT = 'shell/socket/contract/shell_socket_contract.json';
const DEFAULT_ROUTE_CONTRACT = 'validation/conformance/contracts/shell_socket_gateway_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_socket_headless_probe_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_SOCKET_HEADLESS_PROBE_CURRENT.md';

type CallRecord = {
  capabilityId: ShellSocketCapabilityId;
  method: string;
  path: string;
  response: Record<string, unknown>;
};

type ProbeOptions = {
  strict: boolean;
  includeControlledViolation: boolean;
  socketContractPath: string;
  routeContractPath: string;
  outJson: string;
  outMarkdown: string;
};

function readArg(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const found = process.argv.find((arg) => arg.startsWith(prefix));
  return found ? found.slice(prefix.length) : fallback;
}

function readBool(name: string): boolean {
  return process.argv.some((arg) => arg === `--${name}` || arg === `--${name}=1` || arg === `--${name}=true`);
}

function abs(rel: string): string {
  return path.resolve(ROOT, rel);
}

function readJson(rel: string): any {
  return JSON.parse(fs.readFileSync(abs(rel), 'utf8'));
}

function writeFile(relOrAbs: string, content: string): void {
  const target = path.isAbsolute(relOrAbs) ? relOrAbs : abs(relOrAbs);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content);
}

function response(payload: Record<string, unknown>, status = 200): any {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: async () => JSON.stringify(payload),
  };
}

function parseBody(init?: Record<string, unknown>): Record<string, unknown> {
  const raw = typeof init?.body === 'string' ? init.body : '{}';
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function makeIngressAck(kind: string): Record<string, unknown> {
  return {
    accepted: true,
    rejected: false,
    reason_code: 'accepted_by_headless_gateway_fixture',
    receipt_ref: `receipt:${kind}:probe`,
    follow_up_ref: `followup:${kind}:probe`,
    correlation_id: `probe-${kind}`,
  };
}

function makeHeadlessGatewayFetch(calls: CallRecord[], includeControlledViolation: boolean) {
  return async (input: string, init?: Record<string, unknown>): Promise<any> => {
    const url = new URL(input, 'http://shell-socket.local');
    const method = String(init?.method || 'GET').toUpperCase();
    const pathName = url.pathname;
    const route = `${method} ${pathName}`;
    const body = parseBody(init);
    let capabilityId: ShellSocketCapabilityId | null = null;
    let payload: Record<string, unknown> | null = null;

    if (route === 'GET /api/shell-socket/runtime-status') {
      capabilityId = 'get_runtime_status';
      payload = {
        state: 'ready',
        label: 'Headless Gateway fixture ready',
        source: 'shell_socket_headless_probe',
        source_sequence: 1,
        age_seconds: 0,
        stale: false,
        receipt_ref: 'receipt:runtime-status:probe',
        correlation_id: 'probe-runtime-status',
      };
      if (includeControlledViolation) payload.raw_runtime_state = { leaked: true };
    } else if (route === 'GET /api/shell-socket/agents') {
      capabilityId = 'list_agents';
      payload = {
        agents: [{ id: 'agent-probe', label: 'Probe Agent', status: 'ready', detail_ref: 'detail:agent:probe' }],
        agent_ids: ['agent-probe'],
        active_agent_id: 'agent-probe',
        labels: { 'agent-probe': 'Probe Agent' },
        status_counts: { ready: 1 },
        last_activity_preview: { 'agent-probe': 'Headless socket fixture active' },
        next_cursor: null,
        detail_refs: ['detail:agent:probe'],
        receipt_ref: 'receipt:agents:probe',
        correlation_id: 'probe-agents',
      };
    } else if (method === 'GET' && /^\/api\/shell-socket\/agents\/[^/]+\/sessions$/.test(pathName)) {
      capabilityId = 'list_sessions';
      payload = {
        sessions: [{ id: 'session-probe', agent_id: 'agent-probe', title: 'Headless socket session' }],
        session_ids: ['session-probe'],
        active_session_id: 'session-probe',
        last_message_previews: { 'session-probe': 'Socket fixture message preview' },
        message_counts: { 'session-probe': 2 },
        next_cursor: null,
        detail_refs: ['detail:session:probe'],
        receipt_ref: 'receipt:sessions:probe',
        correlation_id: 'probe-sessions',
      };
    } else if (method === 'GET' && /^\/api\/shell-socket\/sessions\/[^/]+\/messages$/.test(pathName)) {
      capabilityId = 'get_message_window';
      payload = {
        ok: true,
        agent_id: 'agent-probe',
        session_id: 'session-probe',
        active_session_id: 'session-probe',
        message_window: [
          { id: 'msg-user-probe', role: 'user', text_preview: 'Run a headless socket probe.', detail_ref: 'detail:message:user-probe' },
          { id: 'msg-agent-probe', role: 'assistant', text_preview: 'Probe completed through Gateway-shaped routes.', detail_ref: 'detail:message:agent-probe' },
        ],
        message_count: 2,
        total_count: 2,
        has_more: false,
        before_cursor: null,
        after_cursor: null,
        detail_refs: ['detail:message:user-probe', 'detail:message:agent-probe'],
        receipt_ref: 'receipt:message-window:probe',
        correlation_id: 'probe-message-window',
      };
    } else if (method === 'GET' && /^\/api\/shell-socket\/details\/[^/]+$/.test(pathName)) {
      capabilityId = 'get_message_detail';
      payload = {
        detail_id: decodeURIComponent(pathName.split('/').pop() || ''),
        detail_kind: 'message',
        requested_view: url.searchParams.get('view') || 'summary',
        detail_projection: {
          id: 'msg-agent-probe',
          text_preview: 'Probe completed through Gateway-shaped routes.',
          line_count: 1,
          tool_summaries: [{ name: 'headless_probe', status: 'complete' }],
        },
        size_bound: { max_response_bytes: 65536 },
        next_cursor: null,
        receipt_ref: 'receipt:detail:probe',
        correlation_id: 'probe-detail',
      };
    } else if (route === 'POST /api/shell-socket/input') {
      capabilityId = 'submit_input';
      payload = makeIngressAck(String(body.kind || 'input'));
    } else if (method === 'GET' && /^\/api\/shell-socket\/sessions\/[^/]+\/events$/.test(pathName)) {
      capabilityId = 'subscribe_events';
      payload = {
        event_id: 'event-probe-1',
        event_kind: 'message_projected',
        agent_id: 'agent-probe',
        session_id: 'session-probe',
        display_projection: { text_preview: 'Headless event projection' },
        status_label: 'projected',
        cursor_refs: { next: null },
        detail_refs: ['detail:event:probe-1'],
        receipt_refs: ['receipt:event:probe-1'],
        correlation_id: 'probe-events',
      };
    } else if (route === 'GET /api/shell-socket/search') {
      capabilityId = 'search';
      payload = {
        query_id: 'query-probe',
        hits: [{ id: 'hit-probe-1', label: 'Headless socket hit', detail_ref: 'detail:search:probe-1' }],
        hit_ids: ['hit-probe-1'],
        snippets: { 'hit-probe-1': 'Projected search snippet only' },
        labels: { 'hit-probe-1': 'Headless socket hit' },
        counts: { total: 1 },
        next_cursor: null,
        detail_refs: ['detail:search:probe-1'],
        receipt_ref: 'receipt:search:probe',
        correlation_id: 'probe-search',
      };
    } else if (route === 'POST /api/shell-socket/issues') {
      capabilityId = 'submit_issue';
      payload = makeIngressAck('issue');
    } else if (method === 'POST' && /^\/api\/shell-socket\/approvals\/[^/]+\/decision$/.test(pathName)) {
      capabilityId = 'submit_approval_decision';
      payload = makeIngressAck('approval');
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/model$/.test(pathName)) {
      capabilityId = 'set_model';
      payload = makeIngressAck('model');
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/git-tree$/.test(pathName)) {
      capabilityId = 'set_git_tree';
      payload = makeIngressAck('git-tree');
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/fresh-session$/.test(pathName)) {
      capabilityId = 'fresh_session';
      payload = makeIngressAck('fresh-session');
    } else if (route === 'POST /api/shell-socket/terminal/commands') {
      capabilityId = 'submit_terminal_command';
      payload = makeIngressAck('terminal-command');
    }

    if (!capabilityId || !payload) return response({ rejected: true, reason_code: 'unknown_headless_fixture_route' }, 404);
    calls.push({ capabilityId, method, path: pathName, response: payload });
    return response(payload);
  };
}

function projectionByName(contract: any): Map<string, any> {
  return new Map((contract.projection_types || []).map((row: any) => [row.name, row]));
}

function mappingByCapability(routeContract: any): Map<string, any> {
  return new Map((routeContract.route_mappings || []).map((row: any) => [row.capability_id, row]));
}

function collectForbiddenFields(value: unknown, forbidden: Set<string>, trail = '$'): string[] {
  if (!value || typeof value !== 'object') return [];
  const rows: string[] = [];
  if (Array.isArray(value)) {
    value.forEach((item, index) => rows.push(...collectForbiddenFields(item, forbidden, `${trail}[${index}]`)));
    return rows;
  }
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    const nextTrail = `${trail}.${key}`;
    if (forbidden.has(key)) rows.push(nextTrail);
    rows.push(...collectForbiddenFields(child, forbidden, nextTrail));
  }
  return rows;
}

function sourceDependencyViolations(): string[] {
  const tokens = [
    ['A', 'lpine'].join(''),
    ['S', 'velte'].join(''),
    ['local', 'Storage'].join(''),
    ['session', 'Storage'].join(''),
    ['docu', 'ment.'].join(''),
    ['win', 'dow.'].join(''),
    ['infring', '_static'].join(''),
    ['dash', 'board'].join(''),
  ];
  const files = [
    'shell/socket/client/shell_socket_gateway_client.ts',
    'shell/socket/probe/shell_socket_headless_probe.ts',
  ];
  return files.flatMap((file) => {
    const source = fs.readFileSync(abs(file), 'utf8');
    return tokens.filter((token) => source.includes(token)).map((token) => `${file}:${token}`);
  });
}

function validateCalls(calls: CallRecord[], socketContract: any, routeContract: any): string[] {
  const violations: string[] = [];
  const projectionMap = projectionByName(socketContract);
  const routeMap = mappingByCapability(routeContract);
  const forbiddenFields = new Set<string>(socketContract.forbidden_default_payload_fields || []);
  const expectedIds = new Set(SHELL_SOCKET_ROUTES.map((route) => route.capabilityId));
  const calledIds = new Set(calls.map((call) => call.capabilityId));
  for (const expected of expectedIds) {
    if (!calledIds.has(expected)) violations.push(`missing_call:${expected}`);
  }
  for (const call of calls) {
    if (!call.path.startsWith('/api/shell-socket/')) violations.push(`non_gateway_socket_path:${call.capabilityId}:${call.path}`);
    const route = routeMap.get(call.capabilityId);
    if (!route) {
      violations.push(`missing_route_mapping:${call.capabilityId}`);
      continue;
    }
    const projection = projectionMap.get(route.default_response_projection);
    if (!projection) {
      violations.push(`missing_projection:${call.capabilityId}:${route.default_response_projection}`);
      continue;
    }
    const allowed = new Set<string>(projection.allowed_top_level_fields || []);
    for (const key of Object.keys(call.response)) {
      if (!allowed.has(key)) violations.push(`unexpected_top_level_field:${call.capabilityId}:${key}`);
    }
    for (const fieldPath of collectForbiddenFields(call.response, forbiddenFields)) {
      violations.push(`forbidden_field:${call.capabilityId}:${fieldPath}`);
    }
    const bytes = Buffer.byteLength(JSON.stringify(call.response), 'utf8');
    if (bytes > Number(projection.max_response_bytes || 0)) violations.push(`payload_too_large:${call.capabilityId}:${bytes}`);
    if (projection.requires_cursor) {
      const hasCursor = ['next_cursor', 'before_cursor', 'after_cursor', 'cursor_refs'].some((key) => key in call.response);
      if (!hasCursor) violations.push(`missing_cursor:${call.capabilityId}`);
    }
    if (projection.requires_detail_refs) {
      const hasRefs = ['detail_refs', 'receipt_ref', 'follow_up_ref'].some((key) => key in call.response);
      if (!hasRefs) violations.push(`missing_ref:${call.capabilityId}`);
    }
  }
  violations.push(...sourceDependencyViolations().map((row) => `forbidden_source_dependency:${row}`));
  return violations;
}

async function runProbe(options: ProbeOptions): Promise<Record<string, unknown>> {
  const socketContract = readJson(options.socketContractPath);
  const routeContract = readJson(options.routeContractPath);
  const calls: CallRecord[] = [];
  const client = new ShellSocketGatewayClient({
    baseUrl: 'http://shell-socket.local',
    fetchImpl: makeHeadlessGatewayFetch(calls, options.includeControlledViolation),
  });

  const runtime = await client.getRuntimeStatus<any>();
  const agents = await client.listAgents<any>({ limit: 10 });
  const agentId = String((agents.agent_ids || [])[0] || 'agent-probe');
  const sessions = await client.listSessions<any>(agentId, { limit: 10 });
  const sessionId = String((sessions.session_ids || [])[0] || 'session-probe');
  const windowProjection = await client.getMessageWindow<any>(sessionId, { limit: 80 });
  const detailRef = String((windowProjection.detail_refs || [])[0] || 'detail:message:agent-probe');
  const detail = await client.getMessageDetail<any>(detailRef, { view: 'summary', limit: 1 });
  const inputAck = await client.submitInput<any>({ kind: 'operator-message', session_id: sessionId, text: 'Probe input' });
  const event = await client.subscribeEvents<any>(sessionId, { cursor: 'event-cursor-0' });
  const search = await client.search<any>({ q: 'probe', scope: sessionId, limit: 10 });
  const issueAck = await client.submitIssue<any>({ kind: 'internal-eval', detail_refs: [detailRef] });
  const approvalAck = await client.submitApprovalDecision<any>('approval-probe', { decision: 'approve' });
  const modelAck = await client.setModel<any>(agentId, { model_ref: 'model:auto' });
  const gitAck = await client.setGitTree<any>(agentId, { tree_ref: 'git-tree:current' });
  const freshSessionAck = await client.freshSession<any>(agentId, { reason: 'headless_probe' });
  const terminalAck = await client.submitTerminalCommand<any>({ command_ref: 'terminal:probe-command' });

  const violations = validateCalls(calls, socketContract, routeContract);
  const ok = violations.length === 0 && calls.length === SHELL_SOCKET_ROUTES.length;
  return {
    ok,
    type: 'shell_socket_headless_probe',
    strict: options.strict,
    controlled_violation: options.includeControlledViolation,
    route_count: SHELL_SOCKET_ROUTES.length,
    exercised_capability_count: new Set(calls.map((call) => call.capabilityId)).size,
    call_count: calls.length,
    capabilities: calls.map((call) => call.capabilityId),
    sample_projection_refs: {
      runtime_label: runtime.label,
      agent_id: agentId,
      session_id: sessionId,
      detail_id: detail.detail_id,
      input_receipt: inputAck.receipt_ref,
      event_id: event.event_id,
      search_query_id: search.query_id,
      issue_receipt: issueAck.receipt_ref,
      approval_receipt: approvalAck.receipt_ref,
      model_receipt: modelAck.receipt_ref,
      git_receipt: gitAck.receipt_ref,
      fresh_session_receipt: freshSessionAck.receipt_ref,
      terminal_receipt: terminalAck.receipt_ref,
    },
    violations,
  };
}

function markdownFor(result: Record<string, unknown>): string {
  const violations = Array.isArray(result.violations) ? result.violations : [];
  return [
    '# Shell Socket Headless Probe',
    '',
    `- ok: \`${String(result.ok)}\``,
    `- route_count: \`${String(result.route_count)}\``,
    `- exercised_capability_count: \`${String(result.exercised_capability_count)}\``,
    `- call_count: \`${String(result.call_count)}\``,
    `- controlled_violation: \`${String(result.controlled_violation)}\``,
    `- violations: \`${violations.length}\``,
    '',
    '## Capabilities',
    '',
    ...((result.capabilities as string[]) || []).map((row) => `- \`${row}\``),
    '',
  ].join('\n');
}

async function main(): Promise<void> {
  const options: ProbeOptions = {
    strict: readBool('strict'),
    includeControlledViolation: readBool('include-controlled-violation'),
    socketContractPath: readArg('socket-contract', DEFAULT_SOCKET_CONTRACT),
    routeContractPath: readArg('route-contract', DEFAULT_ROUTE_CONTRACT),
    outJson: readArg('out-json', DEFAULT_OUT_JSON),
    outMarkdown: readArg('out-markdown', DEFAULT_OUT_MARKDOWN),
  };
  const result = await runProbe(options);
  writeFile(options.outJson, `${JSON.stringify(result, null, 2)}\n`);
  writeFile(options.outMarkdown, markdownFor(result));
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  process.exitCode = result.ok ? 0 : 1;
}

main().catch((error) => {
  const result = {
    ok: false,
    type: 'shell_socket_headless_probe',
    error: error instanceof Error ? error.message : String(error),
  };
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  process.exitCode = 1;
});
