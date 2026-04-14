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

export type RuntimeDebtFile = {
  path: string;
  lines: number;
};

export type RuntimePolicyDebt = {
  open_items: number;
  blocked_items: number;
  policy_green_but_debt_remaining: boolean;
  size_exception_count: number;
  oversized_files: number;
  native_pages: number;
  legacy_pages: number;
  classic_asset_files: number;
  classic_href_references: number;
  embedded_fallback_references: number;
  top_classic_files: RuntimeDebtFile[];
};

export type RuntimeOrchestrationSurface = {
  capability_probes: boolean;
  alternative_plans: boolean;
  verifier_request: boolean;
  verifier_registry_mapping: boolean;
  nested_core_projection: boolean;
  receipt_correlation: boolean;
  plan_variants: string[];
  plan_statuses: string[];
  step_statuses: string[];
  correlation_fields: string[];
  adapter_fallback_pass: boolean | null;
  adapter_fallback_threshold: number | null;
  hidden_state_pass: boolean | null;
  hidden_state_violations: number | null;
  receipt_stream_source: string;
  recent_receipts: RuntimeOrchestrationReceipt[];
};

export type RuntimeOrchestrationReceipt = {
  source: string;
  type: string;
  created_at: string;
  receipt_hash: string;
  status: string;
  tool_name: string;
  task_id: string;
  trace_id: string;
  evidence_count: number;
  claim_count: number;
  core_receipt_count: number;
  core_outcome_count: number;
  lineage_ready: boolean;
};

export type RuntimePageData = {
  overview: RuntimeOverview;
  providers: DashboardProviderRow[];
  web: RuntimeWebStatus;
  debt: RuntimePolicyDebt;
  orchestration: RuntimeOrchestrationSurface;
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

function normalizePolicyDebt(payload: JsonRecord): RuntimePolicyDebt {
  const summary = asRecord(payload.summary);
  const topClassicFiles = Array.isArray(payload.top_classic_files) ? payload.top_classic_files : [];
  return {
    open_items: Number(summary.open_items || 0) || 0,
    blocked_items: Number(summary.blocked_items || 0) || 0,
    policy_green_but_debt_remaining: summary.policy_green_but_debt_remaining === true,
    size_exception_count: Number(summary.size_exception_count || 0) || 0,
    oversized_files: Number(summary.oversized_files || 0) || 0,
    native_pages: Number(summary.native_pages || 0) || 0,
    legacy_pages: Number(summary.legacy_pages || 0) || 0,
    classic_asset_files: Number(summary.classic_asset_files || 0) || 0,
    classic_href_references: Number(summary.classic_href_references || 0) || 0,
    embedded_fallback_references: Number(summary.embedded_fallback_references || 0) || 0,
    top_classic_files: topClassicFiles.slice(0, 5).map((row) => {
      const file = asRecord(row);
      return {
        path: String(file.path || '-').trim() || '-',
        lines: Number(file.lines || 0) || 0,
      };
    }),
  };
}

function normalizeOrchestrationSurface(payload: JsonRecord): RuntimeOrchestrationSurface {
  const summary = asRecord(payload.summary);
  const guardrails = asRecord(payload.guardrails);
  return {
    capability_probes: summary.capability_probes === true,
    alternative_plans: summary.alternative_plans === true,
    verifier_request: summary.verifier_request === true,
    verifier_registry_mapping: summary.verifier_registry_mapping === true,
    nested_core_projection: summary.nested_core_projection === true,
    receipt_correlation: summary.receipt_correlation === true,
    plan_variants: Array.isArray(payload.plan_variants)
      ? payload.plan_variants.map((row) => String(row || '').trim()).filter(Boolean)
      : [],
    plan_statuses: Array.isArray(payload.plan_statuses)
      ? payload.plan_statuses.map((row) => String(row || '').trim()).filter(Boolean)
      : [],
    step_statuses: Array.isArray(payload.step_statuses)
      ? payload.step_statuses.map((row) => String(row || '').trim()).filter(Boolean)
      : [],
    correlation_fields: Array.isArray(payload.correlation_fields)
      ? payload.correlation_fields.map((row) => String(row || '').trim()).filter(Boolean)
      : [],
    adapter_fallback_pass:
      guardrails.adapter_fallback_pass === true ? true : guardrails.adapter_fallback_pass === false ? false : null,
    adapter_fallback_threshold:
      Number.isFinite(Number(guardrails.adapter_fallback_threshold)) ? Number(guardrails.adapter_fallback_threshold) : null,
    hidden_state_pass:
      guardrails.hidden_state_pass === true ? true : guardrails.hidden_state_pass === false ? false : null,
    hidden_state_violations:
      guardrails.hidden_state_violations == null ? null : Number(guardrails.hidden_state_violations || 0) || 0,
    receipt_stream_source: '',
    recent_receipts: [],
  };
}

function normalizeOrchestrationReceipts(payload: JsonRecord): {
  source: string;
  receipts: RuntimeOrchestrationReceipt[];
} {
  const receipts = Array.isArray(payload.receipts) ? payload.receipts : [];
  return {
    source: String(payload.source || '').trim(),
    receipts: receipts.slice(0, 6).map((row) => {
      const receipt = asRecord(row);
      return {
        source: String(receipt.source || '').trim() || '-',
        type: String(receipt.type || '').trim() || '-',
        created_at: String(receipt.created_at || '').trim(),
        receipt_hash: String(receipt.receipt_hash || '').trim(),
        status: String(receipt.status || 'unknown').trim() || 'unknown',
        tool_name: String(receipt.tool_name || '').trim() || '-',
        task_id: String(receipt.task_id || '').trim() || '-',
        trace_id: String(receipt.trace_id || '').trim() || '-',
        evidence_count: Number(receipt.evidence_count || 0) || 0,
        claim_count: Number(receipt.claim_count || 0) || 0,
        core_receipt_count: Number(receipt.core_receipt_count || 0) || 0,
        core_outcome_count: Number(receipt.core_outcome_count || 0) || 0,
        lineage_ready: receipt.lineage_ready === true,
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
  const [status, version, providers, agents, webStatus, webReceipts, policyDebt, orchestrationSurface, orchestrationReceipts] = await Promise.all([
    requestJson<JsonRecord>('/api/status'),
    requestJson<JsonRecord>('/api/version'),
    requestJson<JsonRecord>('/api/providers'),
    readJson<unknown[]>('/api/agents', []),
    readJson<JsonRecord>('/api/web/status', {}),
    readJson<JsonRecord>('/api/web/receipts?limit=5', { receipts: [] }),
    readJson<JsonRecord>('/api/runtime/policy-debt', {}),
    readJson<JsonRecord>('/api/runtime/orchestration-surface', {}),
    readJson<JsonRecord>('/api/runtime/orchestration-receipts?limit=6', { receipts: [] }),
  ]);
  const orchestration = normalizeOrchestrationSurface(asRecord(orchestrationSurface));
  const receiptStream = normalizeOrchestrationReceipts(asRecord(orchestrationReceipts));
  return {
    overview: normalizeOverview(asRecord(status), asRecord(version), Array.isArray(agents) ? agents : []),
    providers: normalizeProviders(asRecord(providers)),
    web: normalizeWebStatus(asRecord(webStatus), asRecord(webReceipts)),
    debt: normalizePolicyDebt(asRecord(policyDebt)),
    orchestration: {
      ...orchestration,
      receipt_stream_source: receiptStream.source,
      recent_receipts: receiptStream.receipts,
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
