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

async function readJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const response = await fetch(url, { cache: 'no-store' });
    if (!response.ok) return fallback;
    const payload = (await response.json()) as T;
    return payload || fallback;
  } catch {
    return fallback;
  }
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
