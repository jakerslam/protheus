#!/usr/bin/env node
/* eslint-disable no-console */

export type ShellSocketCapabilityId =
  | 'get_runtime_status'
  | 'list_agents'
  | 'list_sessions'
  | 'get_message_window'
  | 'get_message_detail'
  | 'submit_input'
  | 'subscribe_events'
  | 'search'
  | 'submit_issue'
  | 'submit_approval_decision'
  | 'list_models'
  | 'discover_models'
  | 'set_model'
  | 'set_git_tree'
  | 'fresh_session'
  | 'compact_session'
  | 'submit_terminal_command';

export type ShellSocketRouteDefinition = {
  capabilityId: ShellSocketCapabilityId;
  method: 'GET' | 'POST';
  path: string;
  pathParams?: string[];
  queryParams?: string[];
};

export type ShellSocketClientOptions = {
  baseUrl?: string;
  fetchImpl?: (input: string, init?: Record<string, unknown>) => Promise<any>;
  defaultHeaders?: Record<string, string>;
};

export type ShellSocketRequestOptions = {
  query?: Record<string, unknown>;
  body?: unknown;
  headers?: Record<string, string>;
};

export const SHELL_SOCKET_ROUTES: ReadonlyArray<ShellSocketRouteDefinition> = Object.freeze([
  { capabilityId: 'get_runtime_status', method: 'GET', path: '/api/shell-socket/runtime-status' },
  { capabilityId: 'list_agents', method: 'GET', path: '/api/shell-socket/agents', queryParams: ['cursor', 'limit'] },
  { capabilityId: 'list_sessions', method: 'GET', path: '/api/shell-socket/agents/{agent_id}/sessions', pathParams: ['agent_id'], queryParams: ['cursor', 'limit'] },
  { capabilityId: 'get_message_window', method: 'GET', path: '/api/shell-socket/sessions/{session_id}/messages', pathParams: ['session_id'], queryParams: ['cursor', 'limit'] },
  { capabilityId: 'get_message_detail', method: 'GET', path: '/api/shell-socket/details/{detail_ref}', pathParams: ['detail_ref'], queryParams: ['view', 'limit'] },
  { capabilityId: 'submit_input', method: 'POST', path: '/api/shell-socket/input' },
  { capabilityId: 'subscribe_events', method: 'GET', path: '/api/shell-socket/sessions/{session_id}/events', pathParams: ['session_id'], queryParams: ['cursor'] },
  { capabilityId: 'search', method: 'GET', path: '/api/shell-socket/search', queryParams: ['q', 'scope', 'cursor', 'limit'] },
  { capabilityId: 'submit_issue', method: 'POST', path: '/api/shell-socket/issues' },
  { capabilityId: 'submit_approval_decision', method: 'POST', path: '/api/shell-socket/approvals/{approval_id}/decision', pathParams: ['approval_id'] },
  { capabilityId: 'list_models', method: 'GET', path: '/api/shell-socket/models', queryParams: ['cursor', 'limit'] },
  { capabilityId: 'discover_models', method: 'POST', path: '/api/shell-socket/models/discover' },
  { capabilityId: 'set_model', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/model', pathParams: ['agent_id'] },
  { capabilityId: 'set_git_tree', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/git-tree', pathParams: ['agent_id'] },
  { capabilityId: 'fresh_session', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/fresh-session', pathParams: ['agent_id'] },
  { capabilityId: 'compact_session', method: 'POST', path: '/api/shell-socket/agents/{agent_id}/compact-session', pathParams: ['agent_id'] },
  { capabilityId: 'submit_terminal_command', method: 'POST', path: '/api/shell-socket/terminal/commands' },
]);

const ROUTES_BY_CAPABILITY = new Map(SHELL_SOCKET_ROUTES.map((route) => [route.capabilityId, route]));

function cleanSegment(value: unknown): string {
  return encodeURIComponent(String(value == null ? '' : value).trim());
}

function normalizeBaseUrl(value: unknown): string {
  const raw = String(value == null ? '' : value).trim();
  if (!raw) return '';
  return raw.replace(/\/+$/, '');
}

function appendQuery(path: string, query: Record<string, unknown> = {}, allowed: string[] = []): string {
  const params = new URLSearchParams();
  for (const key of allowed) {
    const value = query[key];
    if (value == null || value === '') continue;
    params.set(key, String(value));
  }
  const encoded = params.toString();
  return encoded ? `${path}?${encoded}` : path;
}

function fillPath(route: ShellSocketRouteDefinition, query: Record<string, unknown> = {}): string {
  let out = route.path;
  for (const key of route.pathParams || []) {
    const value = query[key];
    if (value == null || value === '') throw new Error(`missing_shell_socket_path_param:${route.capabilityId}:${key}`);
    out = out.replace(`{${key}}`, cleanSegment(value));
  }
  return appendQuery(out, query, route.queryParams || []);
}

function resolveFetch(fetchImpl?: ShellSocketClientOptions['fetchImpl']): ShellSocketClientOptions['fetchImpl'] {
  if (fetchImpl) return fetchImpl;
  const globalFetch = (globalThis as any).fetch;
  if (typeof globalFetch === 'function') return globalFetch.bind(globalThis);
  throw new Error('shell_socket_fetch_unavailable');
}

export class ShellSocketGatewayClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: NonNullable<ShellSocketClientOptions['fetchImpl']>;
  private readonly defaultHeaders: Record<string, string>;

  constructor(options: ShellSocketClientOptions = {}) {
    this.baseUrl = normalizeBaseUrl(options.baseUrl);
    this.fetchImpl = resolveFetch(options.fetchImpl);
    this.defaultHeaders = Object.freeze({ ...(options.defaultHeaders || {}) });
  }

  routeFor(capabilityId: ShellSocketCapabilityId): ShellSocketRouteDefinition {
    const route = ROUTES_BY_CAPABILITY.get(capabilityId);
    if (!route) throw new Error(`unknown_shell_socket_capability:${capabilityId}`);
    return route;
  }

  urlFor(capabilityId: ShellSocketCapabilityId, query: Record<string, unknown> = {}): string {
    const route = this.routeFor(capabilityId);
    return `${this.baseUrl}${fillPath(route, query)}`;
  }

  async request<T = unknown>(
    capabilityId: ShellSocketCapabilityId,
    options: ShellSocketRequestOptions = {},
  ): Promise<T> {
    const route = this.routeFor(capabilityId);
    const headers = {
      accept: 'application/json',
      ...(route.method === 'POST' ? { 'content-type': 'application/json' } : {}),
      ...this.defaultHeaders,
      ...(options.headers || {}),
    };
    const init: Record<string, unknown> = { method: route.method, headers };
    if (route.method === 'POST') init.body = JSON.stringify(options.body || {});
    const response = await this.fetchImpl(this.urlFor(capabilityId, options.query || {}), init);
    const text = typeof response?.text === 'function' ? await response.text() : '';
    const payload = text ? JSON.parse(text) : {};
    if (!response || response.ok === false) {
      const status = Number(response?.status || 0);
      const error = new Error(`shell_socket_gateway_request_failed:${capabilityId}:${status || 'unknown'}`);
      (error as any).payload = payload;
      throw error;
    }
    return payload as T;
  }

  getRuntimeStatus<T = unknown>(): Promise<T> {
    return this.request<T>('get_runtime_status');
  }

  listAgents<T = unknown>(query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('list_agents', { query });
  }

  listSessions<T = unknown>(agentId: string, query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('list_sessions', { query: { ...query, agent_id: agentId } });
  }

  getMessageWindow<T = unknown>(sessionId: string, query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('get_message_window', { query: { ...query, session_id: sessionId } });
  }

  getMessageDetail<T = unknown>(detailRef: string, query: { view?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('get_message_detail', { query: { ...query, detail_ref: detailRef } });
  }

  submitInput<T = unknown>(input: unknown): Promise<T> {
    return this.request<T>('submit_input', { body: input });
  }

  subscribeEvents<T = unknown>(sessionId: string, query: { cursor?: string } = {}): Promise<T> {
    return this.request<T>('subscribe_events', { query: { ...query, session_id: sessionId } });
  }

  search<T = unknown>(query: { q: string; scope?: string; cursor?: string; limit?: number }): Promise<T> {
    return this.request<T>('search', { query });
  }

  submitIssue<T = unknown>(issue: unknown): Promise<T> {
    return this.request<T>('submit_issue', { body: issue });
  }

  submitApprovalDecision<T = unknown>(approvalId: string, decision: unknown): Promise<T> {
    return this.request<T>('submit_approval_decision', { query: { approval_id: approvalId }, body: decision });
  }

  listModels<T = unknown>(query: { cursor?: string; limit?: number } = {}): Promise<T> {
    return this.request<T>('list_models', { query });
  }

  discoverModels<T = unknown>(request: unknown): Promise<T> {
    return this.request<T>('discover_models', { body: request });
  }

  setModel<T = unknown>(agentId: string, modelSelection: unknown): Promise<T> {
    return this.request<T>('set_model', { query: { agent_id: agentId }, body: modelSelection });
  }

  setGitTree<T = unknown>(agentId: string, treeSelection: unknown): Promise<T> {
    return this.request<T>('set_git_tree', { query: { agent_id: agentId }, body: treeSelection });
  }

  freshSession<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('fresh_session', { query: { agent_id: agentId }, body: request });
  }

  compactSession<T = unknown>(agentId: string, request: unknown = {}): Promise<T> {
    return this.request<T>('compact_session', { query: { agent_id: agentId }, body: request });
  }

  submitTerminalCommand<T = unknown>(command: unknown): Promise<T> {
    return this.request<T>('submit_terminal_command', { body: command });
  }
}

export function createShellSocketGatewayClient(options: ShellSocketClientOptions = {}): ShellSocketGatewayClient {
  return new ShellSocketGatewayClient(options);
}

export function shellSocketClientSelfTest(): Record<string, unknown> {
  const routeIds = SHELL_SOCKET_ROUTES.map((route) => route.capabilityId);
  const uniqueRouteIds = new Set(routeIds);
  const client = new ShellSocketGatewayClient({
    fetchImpl: async () => ({ ok: true, status: 200, text: async () => '{}' }),
  });
  return {
    ok: routeIds.length === uniqueRouteIds.size && routeIds.length === 17,
    type: 'shell_socket_gateway_client_self_test',
    route_count: routeIds.length,
    unique_route_count: uniqueRouteIds.size,
    sample_urls: {
      status: client.urlFor('get_runtime_status'),
      sessions: client.urlFor('list_sessions', { agent_id: 'agent-a', cursor: 'c1', limit: 10 }),
      detail: client.urlFor('get_message_detail', { detail_ref: 'detail-a', view: 'summary', limit: 1 }),
    },
  };
}

if (typeof process !== 'undefined' && Array.isArray(process.argv) && process.argv.some((arg) => arg === '--self-test=1' || arg === '--self-test')) {
  const result = shellSocketClientSelfTest();
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  process.exitCode = result.ok ? 0 : 1;
}
