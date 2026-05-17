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
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/message$/.test(pathName)) {
      capabilityId = 'submit_message_result';
      payload = {
        ok: true,
        response: 'Headless socket message response.',
        input_tokens: 4,
        output_tokens: 5,
        cost_usd: 0,
        iterations: 1,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        agent_name: 'Probe Agent',
        route: { provider: 'probe', model: 'auto', reason: 'fixture' },
        context_pressure: 'low',
        tools: [{ id: 'tool-probe', name: 'headless_probe', status: 'ok', result_preview: 'complete' }],
        detail_refs: ['detail:message-result:probe'],
        receipt_ref: 'receipt:message-result:probe',
        correlation_id: 'probe-message-result',
      };
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
    } else if (route === 'GET /api/shell-socket/models') {
      capabilityId = 'list_models';
      payload = {
        ok: true,
        models: [{ id: 'probe/auto', provider: 'probe', model: 'auto', display_name: 'Probe Auto', available: true }],
        model_ids: ['probe/auto'],
        providers: ['probe'],
        counts: { total: 1, available: 1 },
        next_cursor: null,
        detail_refs: ['detail:model:probe-auto'],
        receipt_ref: 'receipt:models:probe',
        correlation_id: 'probe-models',
      };
    } else if (route === 'POST /api/shell-socket/models/discover') {
      capabilityId = 'discover_models';
      payload = {
        ok: true,
        provider: 'probe',
        input_kind: String(body.input || '') === '__auto__' ? 'auto_discovery' : 'api_key',
        provider_count: 1,
        probed: [{ provider: 'probe', ok: true, status: 'ready' }],
        model_count: 1,
        available_model_count: 1,
        models: ['auto'],
        receipt_ref: 'receipt:model-discovery:probe',
        correlation_id: 'probe-model-discovery',
      };
    } else if (route === 'POST /api/shell-socket/models/download') {
      capabilityId = 'download_model';
      payload = {
        ok: true,
        method: 'fixture_download',
        download_path: '/tmp/infring-probe-model',
        provider: String(body.provider || 'probe'),
        model: String(body.model || 'probe/auto'),
        receipt_ref: 'receipt:model-download:probe',
        correlation_id: 'probe-model-download',
      };
    } else if (route === 'POST /api/shell-socket/models/custom') {
      capabilityId = 'upsert_custom_model';
      payload = makeIngressAck('custom-model');
    } else if (route === 'POST /api/shell-socket/models/custom/delete') {
      capabilityId = 'delete_custom_model';
      payload = makeIngressAck('custom-model-delete');
    } else if (route === 'POST /api/shell-socket/config/set') {
      capabilityId = 'set_config';
      payload = {
        ok: true,
        path: String(body.path || 'display.theme'),
        value: body.value ?? 'auto',
        provider: null,
        auth_status: null,
        switched_default: null,
        message: null,
        error: null,
        receipt_ref: 'receipt:config-set:probe',
        correlation_id: 'probe-config-set',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/providers\/[^/]+\/key$/.test(pathName)) {
      capabilityId = 'save_provider_key';
      payload = {
        ok: true,
        provider: decodeURIComponent(pathName.split('/')[4] || 'probe'),
        auth_status: 'configured',
        switched_default: false,
        message: null,
        error: null,
        receipt_ref: 'receipt:provider-key:probe',
        correlation_id: 'probe-provider-key',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/providers\/[^/]+\/key\/remove$/.test(pathName)) {
      capabilityId = 'remove_provider_key';
      payload = {
        ok: true,
        provider: decodeURIComponent(pathName.split('/')[4] || 'probe'),
        auth_status: 'not_set',
        error: null,
        receipt_ref: 'receipt:provider-key-remove:probe',
        correlation_id: 'probe-provider-key-remove',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/providers\/[^/]+\/test$/.test(pathName)) {
      capabilityId = 'test_provider';
      payload = {
        ok: true,
        status: 'ok',
        provider: decodeURIComponent(pathName.split('/')[4] || 'probe'),
        latency_ms: 1,
        error: null,
        receipt_ref: 'receipt:provider-test:probe',
        correlation_id: 'probe-provider-test',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/providers\/[^/]+\/url$/.test(pathName)) {
      capabilityId = 'set_provider_url';
      payload = {
        ok: true,
        provider: decodeURIComponent(pathName.split('/')[4] || 'probe'),
        reachable: true,
        latency_ms: 1,
        error: null,
        receipt_ref: 'receipt:provider-url:probe',
        correlation_id: 'probe-provider-url',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/providers\/[^/]+\/oauth\/start$/.test(pathName)) {
      capabilityId = 'start_provider_oauth';
      payload = {
        ok: true,
        provider: decodeURIComponent(pathName.split('/')[4] || 'github-copilot'),
        status: 'pending',
        poll_id: 'poll-probe',
        user_code: 'PROBE-CODE',
        verification_uri: 'https://github.com/login/device',
        interval: 5,
        expires_in: 900,
        receipt_ref: 'receipt:provider-oauth-start:probe',
        correlation_id: 'probe-provider-oauth-start',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/providers\/[^/]+\/oauth\/poll$/.test(pathName)) {
      capabilityId = 'poll_provider_oauth';
      payload = {
        ok: true,
        provider: decodeURIComponent(pathName.split('/')[4] || 'github-copilot'),
        status: 'complete',
        poll_id: String(body.poll_id || 'poll-probe'),
        interval: 5,
        error: null,
        receipt_ref: 'receipt:provider-oauth-poll:probe',
        correlation_id: 'probe-provider-oauth-poll',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/model$/.test(pathName)) {
      capabilityId = 'set_model';
      payload = makeIngressAck('model');
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/config$/.test(pathName)) {
      capabilityId = 'update_agent_config';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        rename_notice: null,
        receipt_ref: 'receipt:agent-config:probe',
        correlation_id: 'probe-agent-config',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/mode$/.test(pathName)) {
      capabilityId = 'update_agent_mode';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        mode: String(body.mode || 'normal'),
        receipt_ref: 'receipt:agent-mode:probe',
        correlation_id: 'probe-agent-mode',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/tools$/.test(pathName)) {
      capabilityId = 'update_agent_tools';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        tool_filters: {
          tool_allowlist: Array.isArray(body.tool_allowlist) ? body.tool_allowlist : [],
          tool_blocklist: Array.isArray(body.tool_blocklist) ? body.tool_blocklist : [],
        },
        receipt_ref: 'receipt:agent-tools:probe',
        correlation_id: 'probe-agent-tools',
      };
    } else if (route === 'POST /api/shell-socket/agents/create') {
      capabilityId = 'create_agent';
      payload = {
        ok: true,
        id: 'agent-created-probe',
        agent_id: 'agent-created-probe',
        name: 'Created Probe Agent',
        role: String(body.role || 'analyst'),
        state: 'Running',
        receipt_ref: 'receipt:agent-create:probe',
        correlation_id: 'probe-agent-create',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/archive$/.test(pathName)) {
      capabilityId = 'archive_agent';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        archived: true,
        reason: String(body.reason || 'headless_probe'),
        receipt_ref: 'receipt:agent-archive:probe',
        correlation_id: 'probe-agent-archive',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/revive$/.test(pathName)) {
      capabilityId = 'revive_agent';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        state: 'Running',
        receipt_ref: 'receipt:agent-revive:probe',
        correlation_id: 'probe-agent-revive',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/clone$/.test(pathName)) {
      capabilityId = 'clone_agent';
      payload = {
        ok: true,
        agent_id: 'agent-clone-probe',
        name: String(body.new_name || 'Clone Probe Agent'),
        receipt_ref: 'receipt:agent-clone:probe',
        correlation_id: 'probe-agent-clone',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/history\/clear$/.test(pathName)) {
      capabilityId = 'clear_agent_history';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        type: 'agent_history_cleared',
        receipt_ref: 'receipt:agent-history-clear:probe',
        correlation_id: 'probe-agent-history-clear',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/archived\/delete$/.test(pathName)) {
      capabilityId = 'delete_archived_agent';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        removed_history_entries: 1,
        removed_archived: true,
        receipt_ref: 'receipt:delete-archived-agent:probe',
        correlation_id: 'probe-delete-archived-agent',
      };
    } else if (route === 'POST /api/shell-socket/agents/archived/delete-all') {
      capabilityId = 'delete_all_archived_agents';
      payload = {
        ok: true,
        removed_history_entries: 2,
        deleted_archived_agents: 2,
        receipt_ref: 'receipt:delete-all-archived-agents:probe',
        correlation_id: 'probe-delete-all-archived-agents',
      };
    } else if (route === 'POST /api/shell-socket/agents/archive-all') {
      capabilityId = 'archive_all_agents';
      payload = {
        ok: true,
        attempted: 2,
        archived_count: 2,
        include_permanent: false,
        receipt_ref: 'receipt:archive-all-agents:probe',
        correlation_id: 'probe-archive-all-agents',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/stop$/.test(pathName)) {
      capabilityId = 'stop_agent';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        state: 'stopping',
        receipt_ref: 'receipt:agent-stop:probe',
        correlation_id: 'probe-agent-stop',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/sessions$/.test(pathName)) {
      capabilityId = 'create_session';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        session_id: 'session-created-probe',
        active_session_id: 'session-created-probe',
        label: 'Probe Session',
        receipt_ref: 'receipt:create-session:probe',
        correlation_id: 'probe-create-session',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/sessions\/[^/]+\/switch$/.test(pathName)) {
      capabilityId = 'switch_session';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        session_id: decodeURIComponent(pathName.split('/')[6] || 'session-probe'),
        active_session_id: decodeURIComponent(pathName.split('/')[6] || 'session-probe'),
        receipt_ref: 'receipt:switch-session:probe',
        correlation_id: 'probe-switch-session',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/suggestions$/.test(pathName)) {
      capabilityId = 'request_agent_suggestions';
      payload = {
        ok: true,
        agent_id: decodeURIComponent(pathName.split('/')[4] || 'agent-probe'),
        suggestions: [{ id: 'suggestion-probe', label: 'Probe suggestion' }],
        receipt_ref: 'receipt:suggestions:probe',
        correlation_id: 'probe-suggestions',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/artifacts\/file\/read$/.test(pathName)) {
      capabilityId = 'read_agent_file_artifact';
      payload = {
        ok: true,
        file: {
          ok: true,
          path: 'probe.txt',
          content: 'probe file content',
          bytes: 18,
          truncated: false,
        },
        receipt_ref: 'receipt:file-artifact:probe',
        correlation_id: 'probe-file-artifact',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/artifacts\/folder\/export$/.test(pathName)) {
      capabilityId = 'export_agent_folder_artifact';
      payload = {
        ok: true,
        folder: {
          ok: true,
          path: 'probe-folder',
          entries: 1,
          tree: [{ path: 'probe-folder/probe.txt', kind: 'file' }],
          truncated: false,
        },
        archive: {
          ok: true,
          file_name: 'probe-folder.zip',
          detail_ref: 'artifact:folder:probe',
        },
        receipt_ref: 'receipt:folder-artifact:probe',
        correlation_id: 'probe-folder-artifact',
      };
    } else if (route === 'POST /api/shell-socket/workflows') {
      capabilityId = 'create_workflow';
      payload = {
        ok: true,
        id: 'workflow-probe',
        workflow_id: 'workflow-probe',
        name: String(body.name || 'Probe Workflow'),
        status: 'created',
        detail_refs: { workflow: 'detail:workflow:probe' },
        receipt_ref: 'receipt:workflow-create:probe',
        correlation_id: 'probe-workflow-create',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/workflows\/[^/]+\/update$/.test(pathName)) {
      capabilityId = 'update_workflow';
      payload = {
        ok: true,
        workflow_id: decodeURIComponent(pathName.split('/')[4] || 'workflow-probe'),
        name: String(body.name || 'Probe Workflow Updated'),
        status: 'updated',
        detail_refs: { workflow: 'detail:workflow:probe' },
        receipt_ref: 'receipt:workflow-update:probe',
        correlation_id: 'probe-workflow-update',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/workflows\/[^/]+\/delete$/.test(pathName)) {
      capabilityId = 'delete_workflow';
      payload = {
        ok: true,
        workflow_id: decodeURIComponent(pathName.split('/')[4] || 'workflow-probe'),
        deleted: true,
        detail_refs: { workflow: 'detail:workflow:probe' },
        receipt_ref: 'receipt:workflow-delete:probe',
        correlation_id: 'probe-workflow-delete',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/workflows\/[^/]+\/run$/.test(pathName)) {
      capabilityId = 'run_workflow';
      payload = {
        ok: true,
        workflow_id: decodeURIComponent(pathName.split('/')[4] || 'workflow-probe'),
        run_id: 'run-workflow-probe',
        status: 'completed',
        output_preview: 'Probe workflow completed.',
        detail_refs: { workflow_run: 'detail:workflow-run:probe' },
        receipt_ref: 'receipt:workflow-run:probe',
        correlation_id: 'probe-workflow-run',
      };
    } else if (route === 'POST /api/shell-socket/scheduler/jobs') {
      capabilityId = 'create_cron_job';
      payload = {
        ok: true,
        job_id: 'job-probe',
        name: String(body.name || 'Probe Schedule'),
        enabled: body.enabled !== false,
        next_run: '2026-05-17T00:00:00Z',
        receipt_ref: 'receipt:create-cron-job:probe',
        correlation_id: 'probe-create-cron-job',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/scheduler\/jobs\/[^/]+\/enable$/.test(pathName)) {
      capabilityId = 'set_cron_job_enabled';
      payload = {
        ok: true,
        job_id: decodeURIComponent(pathName.split('/')[5] || 'job-probe'),
        enabled: body.enabled !== false,
        next_run: '2026-05-17T00:00:00Z',
        receipt_ref: 'receipt:set-cron-job-enabled:probe',
        correlation_id: 'probe-set-cron-job-enabled',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/scheduler\/jobs\/[^/]+\/delete$/.test(pathName)) {
      capabilityId = 'delete_cron_job';
      payload = {
        ok: true,
        job_id: decodeURIComponent(pathName.split('/')[5] || 'job-probe'),
        deleted: true,
        receipt_ref: 'receipt:delete-cron-job:probe',
        correlation_id: 'probe-delete-cron-job',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/scheduler\/jobs\/[^/]+\/run$/.test(pathName)) {
      capabilityId = 'run_schedule';
      payload = {
        ok: true,
        job_id: decodeURIComponent(pathName.split('/')[5] || 'job-probe'),
        status: 'completed',
        ran_at: '2026-05-17T00:00:00Z',
        receipt_ref: 'receipt:run-schedule:probe',
        correlation_id: 'probe-run-schedule',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/scheduler\/triggers\/[^/]+\/enable$/.test(pathName)) {
      capabilityId = 'set_trigger_enabled';
      payload = {
        ok: true,
        trigger_id: decodeURIComponent(pathName.split('/')[5] || 'trigger-probe'),
        enabled: body.enabled !== false,
        receipt_ref: 'receipt:set-trigger-enabled:probe',
        correlation_id: 'probe-set-trigger-enabled',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/scheduler\/triggers\/[^/]+\/delete$/.test(pathName)) {
      capabilityId = 'delete_trigger';
      payload = {
        ok: true,
        trigger_id: decodeURIComponent(pathName.split('/')[5] || 'trigger-probe'),
        deleted: true,
        receipt_ref: 'receipt:delete-trigger:probe',
        correlation_id: 'probe-delete-trigger',
      };
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/git-tree$/.test(pathName)) {
      capabilityId = 'set_git_tree';
      payload = makeIngressAck('git-tree');
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/fresh-session$/.test(pathName)) {
      capabilityId = 'fresh_session';
      payload = makeIngressAck('fresh-session');
    } else if (method === 'POST' && /^\/api\/shell-socket\/agents\/[^/]+\/compact-session$/.test(pathName)) {
      capabilityId = 'compact_session';
      payload = makeIngressAck('compact-session');
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
  const messageResult = await client.submitMessageResult<any>(agentId, { message: 'Probe input' });
  const event = await client.subscribeEvents<any>(sessionId, { cursor: 'event-cursor-0' });
  const search = await client.search<any>({ q: 'probe', scope: sessionId, limit: 10 });
  const issueAck = await client.submitIssue<any>({ kind: 'internal-eval', detail_refs: [detailRef] });
  const approvalAck = await client.submitApprovalDecision<any>('approval-probe', { decision: 'approve' });
  const models = await client.listModels<any>({ limit: 10 });
  const discovery = await client.discoverModels<any>({ input: '__auto__' });
  const download = await client.downloadModel<any>({ provider: 'probe', model: 'probe/auto' });
  const customModelAck = await client.upsertCustomModel<any>({ provider: 'probe', model: 'probe/custom' });
  const customModelDeleteAck = await client.deleteCustomModel<any>({ model_ref: 'probe/custom' });
  const configSet = await client.setConfig<any>({ path: 'display.theme', value: 'auto' });
  const providerKey = await client.saveProviderKey<any>('probe', { key: 'probe-key' });
  const providerKeyRemove = await client.removeProviderKey<any>('probe');
  const providerTest = await client.testProvider<any>('probe');
  const providerUrl = await client.setProviderUrl<any>('probe', { base_url: 'http://127.0.0.1:11434' });
  const providerOAuthStart = await client.startProviderOAuth<any>('github-copilot');
  const providerOAuthPoll = await client.pollProviderOAuth<any>('github-copilot', { poll_id: providerOAuthStart.poll_id });
  const modelAck = await client.setModel<any>(agentId, { model_ref: 'model:auto' });
  const agentConfigAck = await client.updateAgentConfig<any>(agentId, { name: 'Probe Agent' });
  const agentModeAck = await client.updateAgentMode<any>(agentId, { mode: 'normal' });
  const agentToolsAck = await client.updateAgentTools<any>(agentId, { tool_allowlist: [], tool_blocklist: [] });
  const agentCreateAck = await client.createAgent<any>({ role: 'analyst' });
  const agentArchiveAck = await client.archiveAgent<any>(agentId, { reason: 'headless_probe' });
  const agentReviveAck = await client.reviveAgent<any>(agentId, { role: 'analyst' });
  const agentCloneAck = await client.cloneAgent<any>(agentId, { new_name: 'Clone Probe Agent' });
  const agentHistoryClearAck = await client.clearAgentHistory<any>(agentId);
  const archivedAgentDeleteAck = await client.deleteArchivedAgent<any>(agentId, { contract_id: 'contract-probe' });
  const allArchivedAgentsDeleteAck = await client.deleteAllArchivedAgents<any>();
  const archiveAllAgentsAck = await client.archiveAllAgents<any>({ reason: 'headless_probe' });
  const agentStopAck = await client.stopAgent<any>(agentId, { reason: 'headless_probe' });
  const createSessionAck = await client.createSession<any>(agentId, { label: 'Probe Session' });
  const switchSessionAck = await client.switchSession<any>(agentId, sessionId);
  const suggestionsAck = await client.requestAgentSuggestions<any>(agentId, { user_hint: 'probe' });
  const fileArtifact = await client.readAgentFileArtifact<any>(agentId, { path: 'probe.txt' });
  const folderArtifact = await client.exportAgentFolderArtifact<any>(agentId, { path: 'probe-folder' });
  const workflowCreateAck = await client.createWorkflow<any>({ name: 'Probe Workflow', steps: [{ name: 'step-1' }] });
  const workflowUpdateAck = await client.updateWorkflow<any>('workflow-probe', { name: 'Probe Workflow Updated' });
  const workflowRunAck = await client.runWorkflow<any>('workflow-probe', { input: 'probe' });
  const workflowDeleteAck = await client.deleteWorkflow<any>('workflow-probe');
  const cronCreateAck = await client.createCronJob<any>({ name: 'Probe Schedule', enabled: true });
  const cronEnableAck = await client.setCronJobEnabled<any>('job-probe', { enabled: true });
  const scheduleRunAck = await client.runSchedule<any>('job-probe');
  const cronDeleteAck = await client.deleteCronJob<any>('job-probe');
  const triggerEnableAck = await client.setTriggerEnabled<any>('trigger-probe', { enabled: true });
  const triggerDeleteAck = await client.deleteTrigger<any>('trigger-probe');
  const gitAck = await client.setGitTree<any>(agentId, { tree_ref: 'git-tree:current' });
  const freshSessionAck = await client.freshSession<any>(agentId, { reason: 'headless_probe' });
  const compactSessionAck = await client.compactSession<any>(agentId, { reason: 'headless_probe' });
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
      message_result_receipt: messageResult.receipt_ref,
      event_id: event.event_id,
      search_query_id: search.query_id,
      issue_receipt: issueAck.receipt_ref,
      approval_receipt: approvalAck.receipt_ref,
      model_catalog_receipt: models.receipt_ref,
      model_discovery_receipt: discovery.receipt_ref,
      model_download_receipt: download.receipt_ref,
      custom_model_receipt: customModelAck.receipt_ref,
      custom_model_delete_receipt: customModelDeleteAck.receipt_ref,
      config_set_receipt: configSet.receipt_ref,
      provider_key_receipt: providerKey.receipt_ref,
      provider_key_remove_receipt: providerKeyRemove.receipt_ref,
      provider_test_receipt: providerTest.receipt_ref,
      provider_url_receipt: providerUrl.receipt_ref,
      provider_oauth_start_receipt: providerOAuthStart.receipt_ref,
      provider_oauth_poll_receipt: providerOAuthPoll.receipt_ref,
      model_receipt: modelAck.receipt_ref,
      agent_config_receipt: agentConfigAck.receipt_ref,
      agent_mode_receipt: agentModeAck.receipt_ref,
      agent_tools_receipt: agentToolsAck.receipt_ref,
      agent_create_receipt: agentCreateAck.receipt_ref,
      agent_archive_receipt: agentArchiveAck.receipt_ref,
      agent_revive_receipt: agentReviveAck.receipt_ref,
      agent_clone_receipt: agentCloneAck.receipt_ref,
      agent_history_clear_receipt: agentHistoryClearAck.receipt_ref,
      archived_agent_delete_receipt: archivedAgentDeleteAck.receipt_ref,
      all_archived_agents_delete_receipt: allArchivedAgentsDeleteAck.receipt_ref,
      archive_all_agents_receipt: archiveAllAgentsAck.receipt_ref,
      agent_stop_receipt: agentStopAck.receipt_ref,
      create_session_receipt: createSessionAck.receipt_ref,
      switch_session_receipt: switchSessionAck.receipt_ref,
      suggestions_receipt: suggestionsAck.receipt_ref,
      file_artifact_receipt: fileArtifact.receipt_ref,
      folder_artifact_receipt: folderArtifact.receipt_ref,
      workflow_create_receipt: workflowCreateAck.receipt_ref,
      workflow_update_receipt: workflowUpdateAck.receipt_ref,
      workflow_run_receipt: workflowRunAck.receipt_ref,
      workflow_delete_receipt: workflowDeleteAck.receipt_ref,
      cron_create_receipt: cronCreateAck.receipt_ref,
      cron_enable_receipt: cronEnableAck.receipt_ref,
      schedule_run_receipt: scheduleRunAck.receipt_ref,
      cron_delete_receipt: cronDeleteAck.receipt_ref,
      trigger_enable_receipt: triggerEnableAck.receipt_ref,
      trigger_delete_receipt: triggerDeleteAck.receipt_ref,
      git_receipt: gitAck.receipt_ref,
      fresh_session_receipt: freshSessionAck.receipt_ref,
      compact_session_receipt: compactSessionAck.receipt_ref,
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
