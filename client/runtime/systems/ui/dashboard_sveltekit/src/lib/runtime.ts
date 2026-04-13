import type { DashboardProviderRow } from '$lib/settings';

export type RuntimeStatus = {
  ok?: boolean;
  connected?: boolean;
  daemon?: string;
  error?: string;
  default_model?: string;
  uptime_seconds?: number;
  api_listen?: string;
  listen?: string;
};

export type RuntimeVersion = {
  version?: string;
  platform?: string;
  arch?: string;
};

export type ProviderRow = {
  id?: string;
  display_name?: string;
  auth_status?: string;
  health?: string;
  reachable?: boolean;
  is_local?: boolean;
};

export type ChannelRow = {
  id?: string;
  name?: string;
  has_token?: boolean;
};

export type AuditEntry = {
  ts?: string;
  action?: string;
  actor?: string;
  agent_id?: string;
};

export type DashboardOverviewSnapshot = {
  status: RuntimeStatus;
  version: RuntimeVersion;
  providers: ProviderRow[];
  channels: ChannelRow[];
  recentAudit: AuditEntry[];
  skillCount: number;
  agentCount: number;
  usageSummary: {
    total_tokens: number;
    total_tools: number;
    total_cost: number;
  };
};

export type RuntimeOverview = {
  version: string;
  platform: string;
  arch: string;
  uptime_seconds: number;
  agent_count: number;
  default_provider: string;
  default_model: string;
  api_listen: string;
  home_dir: string;
  log_level: string;
  network_enabled: boolean;
};

export type RuntimeWebReceipt = {
  requested_url: string;
  method: string;
  status: string;
  blocked: boolean;
  created_at: string;
};

export type RuntimeWebStatus = {
  enabled: boolean;
  rate_limit: string;
  receipts_total: number;
  recent_denied: number;
  last_url: string;
  recent_receipts: RuntimeWebReceipt[];
};

export type RuntimePageData = {
  overview: RuntimeOverview;
  providers: DashboardProviderRow[];
  web: RuntimeWebStatus;
};

type JsonRecord = Record<string, unknown>;

function asRecord(value: unknown): JsonRecord {
  return value && typeof value === 'object' ? (value as JsonRecord) : {};
}

function readAuthToken(): string {
  if (typeof window === 'undefined' || !window.localStorage) return '';
  try {
    return String(window.localStorage.getItem('infring-api-key') || '').trim();
  } catch {
    return '';
  }
}

function requestHeaders(withBody: boolean): Record<string, string> {
  const headers: Record<string, string> = {};
  const token = readAuthToken();
  if (withBody) headers['Content-Type'] = 'application/json';
  if (token) headers.Authorization = `Bearer ${token}`;
  return headers;
}

async function requestJson<T>(url: string): Promise<T> {
  const response = await fetch(url, {
    method: 'GET',
    cache: 'no-store',
    headers: requestHeaders(false),
  });
  if (!response.ok) {
    let message = `GET ${url} failed`;
    try {
      const payload = asRecord(await response.json());
      message = String(payload.error || payload.message || message);
    } catch {
      message = (await response.text().catch(() => message)) || message;
    }
    throw new Error(message);
  }
  return (await response.json()) as T;
}

async function readJson<T>(url: string, fallback: T): Promise<T> {
  try {
    return await requestJson<T>(url);
  } catch {
    return fallback;
  }
}

function normalizeProviders(payload: JsonRecord): DashboardProviderRow[] {
  const rows = Array.isArray(payload.providers) ? payload.providers : [];
  return rows
    .map((row) => asRecord(row))
    .filter((row) => row.auth_status === 'Configured' || row.reachable === true || row.is_local === true)
    .map((row) => ({
      id: String(row.id || '').trim(),
      display_name: String(row.display_name || row.id || '').trim(),
      auth_status: String(row.auth_status || '').trim(),
      api_key_env: String(row.api_key_env || '').trim(),
      base_url: String(row.base_url || '').trim(),
      is_local: row.is_local === true,
    }));
}

function normalizeOverview(status: JsonRecord, version: JsonRecord, agents: unknown[]): RuntimeOverview {
  return {
    version: String(version.version || '-').trim() || '-',
    platform: String(version.platform || '-').trim() || '-',
    arch: String(version.arch || '-').trim() || '-',
    uptime_seconds: Number(status.uptime_seconds || 0) || 0,
    agent_count: Array.isArray(agents) ? agents.length : 0,
    default_provider: String(status.default_provider || '-').trim() || '-',
    default_model: String(status.default_model || '-').trim() || '-',
    api_listen: String(status.api_listen || status.listen || '-').trim() || '-',
    home_dir: String(status.home_dir || '-').trim() || '-',
    log_level: String(status.log_level || '-').trim() || '-',
    network_enabled: status.network_enabled === true,
  };
}

function normalizeWebStatus(statusPayload: JsonRecord, receiptsPayload: JsonRecord): RuntimeWebStatus {
  const policy = asRecord(statusPayload.policy);
  const webConduit = asRecord(policy.web_conduit);
  const receipts = Array.isArray(receiptsPayload.receipts) ? receiptsPayload.receipts : [];
  return {
    enabled: statusPayload.enabled === true,
    rate_limit: webConduit.rate_limit_per_minute ? `${webConduit.rate_limit_per_minute}/min` : '-',
    receipts_total: Number(statusPayload.receipts_total || 0) || 0,
    recent_denied: Number(statusPayload.recent_denied || 0) || 0,
    last_url: String(asRecord(statusPayload.last_receipt).requested_url || '-').trim() || '-',
    recent_receipts: receipts.slice(0, 5).map((row) => {
      const receipt = asRecord(row);
      return {
        requested_url: String(receipt.requested_url || '-').trim() || '-',
        method: String(receipt.method || 'GET').trim() || 'GET',
        status: String(receipt.status || receipt.outcome || 'unknown').trim() || 'unknown',
        blocked: receipt.blocked === true,
        created_at: String(receipt.created_at || receipt.ts || '').trim(),
      };
    }),
  };
}

export async function readRuntimeStatus(): Promise<RuntimeStatus> {
  return readJson<RuntimeStatus>('/api/status', {});
}

export async function readOverviewSnapshot(): Promise<DashboardOverviewSnapshot> {
  const [status, version, providersPayload, usagePayload, auditPayload, channelsPayload, skillsPayload, agentsPayload] =
    await Promise.all([
      readJson<RuntimeStatus>('/api/status', {}),
      readJson<RuntimeVersion>('/api/version', {}),
      readJson<{ providers?: ProviderRow[] }>('/api/providers', {}),
      readJson<{ agents?: Array<{ total_tokens?: number; tool_calls?: number; cost_usd?: number }> }>('/api/usage', {}),
      readJson<{ entries?: AuditEntry[] }>('/api/audit/recent?n=6', {}),
      readJson<{ channels?: ChannelRow[] }>('/api/channels', {}),
      readJson<{ skills?: unknown[] }>('/api/skills', {}),
      readJson<unknown[]>('/api/agents', []),
    ]);

  const usageAgents = Array.isArray(usagePayload.agents) ? usagePayload.agents : [];

  return {
    status,
    version,
    providers: Array.isArray(providersPayload.providers) ? providersPayload.providers : [],
    channels: Array.isArray(channelsPayload.channels) ? channelsPayload.channels : [],
    recentAudit: Array.isArray(auditPayload.entries) ? auditPayload.entries : [],
    skillCount: Array.isArray(skillsPayload.skills) ? skillsPayload.skills.length : 0,
    agentCount: Array.isArray(agentsPayload) ? agentsPayload.length : 0,
    usageSummary: {
      total_tokens: usageAgents.reduce((sum, row) => sum + Number(row.total_tokens || 0), 0),
      total_tools: usageAgents.reduce((sum, row) => sum + Number(row.tool_calls || 0), 0),
      total_cost: usageAgents.reduce((sum, row) => sum + Number(row.cost_usd || 0), 0),
    },
  };
}

export async function readRuntimePageData(): Promise<RuntimePageData> {
  const [status, version, providers, agents, webStatus, webReceipts] = await Promise.all([
    requestJson<JsonRecord>('/api/status'),
    requestJson<JsonRecord>('/api/version'),
    requestJson<JsonRecord>('/api/providers'),
    readJson<unknown[]>('/api/agents', []),
    readJson<JsonRecord>('/api/web/status', {}),
    readJson<JsonRecord>('/api/web/receipts?limit=5', { receipts: [] }),
  ]);
  return {
    overview: normalizeOverview(asRecord(status), asRecord(version), Array.isArray(agents) ? agents : []),
    providers: normalizeProviders(asRecord(providers)),
    web: normalizeWebStatus(asRecord(webStatus), asRecord(webReceipts)),
  };
}

export function formatRelativeTime(timestamp: string | null | undefined): string {
  if (!timestamp) return 'No recent activity';
  const target = new Date(timestamp).getTime();
  if (!Number.isFinite(target)) return 'No recent activity';
  const deltaSeconds = Math.max(0, Math.floor((Date.now() - target) / 1000));
  if (deltaSeconds < 10) return 'just now';
  if (deltaSeconds < 60) return `${deltaSeconds}s ago`;
  if (deltaSeconds < 3600) return `${Math.floor(deltaSeconds / 60)}m ago`;
  if (deltaSeconds < 86400) return `${Math.floor(deltaSeconds / 3600)}h ago`;
  return `${Math.floor(deltaSeconds / 86400)}d ago`;
}
