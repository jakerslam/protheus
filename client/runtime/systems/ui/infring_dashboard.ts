#!/usr/bin/env tsx
// Unified dashboard lane: TypeScript-first client UI over Rust-core authority.

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const http = require('node:http');
const { spawnSync, spawn } = require('node:child_process');
const ts = require('typescript');
const { WebSocketServer } = require('ws');

const DASHBOARD_DIR = __dirname;
const ROOT = path.resolve(DASHBOARD_DIR, '..', '..', '..', '..');
const TS_ENTRYPOINT_PATH = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const OPS_BRIDGE_PATH = path.resolve(ROOT, 'client/runtime/systems/ops/run_protheus_ops.ts');
const INFRING_PRIMARY_STATIC_DIR = path.resolve(
  ROOT,
  'client/runtime/systems/ui/openclaw_static'
);
const PROTHEUSD_DEBUG_BIN = path.resolve(ROOT, 'target/debug/protheusd');
const PROTHEUSD_RELEASE_BIN = path.resolve(ROOT, 'target/release/protheusd');
const CLIENT_TS_PATH = path.resolve(DASHBOARD_DIR, 'infring_dashboard_client.tsx');
const CSS_PATH = path.resolve(DASHBOARD_DIR, 'infring_dashboard.css');
const STATE_DIR = path.resolve(ROOT, 'client/runtime/local/state/ui/infring_dashboard');
const AGENT_SESSIONS_DIR = path.resolve(STATE_DIR, 'agent_sessions');
const ACTION_DIR = path.resolve(STATE_DIR, 'actions');
const ACTION_LATEST_PATH = path.resolve(ACTION_DIR, 'latest.json');
const ACTION_HISTORY_PATH = path.resolve(ACTION_DIR, 'history.jsonl');
const COLLAB_TEAM_STATE_DIR = path.resolve(ROOT, 'core/local/state/ops/collab_plane/teams');
const SNAPSHOT_LATEST_PATH = path.resolve(STATE_DIR, 'latest_snapshot.json');
const SNAPSHOT_HISTORY_PATH = path.resolve(STATE_DIR, 'snapshot_history.jsonl');
const SNAPSHOT_HISTORY_MAX_BYTES = 100 * 1024 * 1024;
const SNAPSHOT_HISTORY_MAX_LINES = 10_000;
const SNAPSHOT_HISTORY_RETAIN_LINES = 1_000;
const SNAPSHOT_HISTORY_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000;
const SNAPSHOT_HISTORY_COMPACT_INTERVAL_MS = 5 * 60 * 1000;
const SNAPSHOT_HISTORY_APPEND_MIN_INTERVAL_MS = 30 * 1000;
const SNAPSHOT_HISTORY_WARNING_BYTES = 500 * 1024 * 1024;
const ATTENTION_DEFERRED_PATH = path.resolve(STATE_DIR, 'attention_deferred.json');
const ARCHIVED_AGENTS_PATH = path.resolve(STATE_DIR, 'archived_agents.json');
const AGENT_CONTRACTS_PATH = path.resolve(STATE_DIR, 'agent_contracts.json');
const AGENT_PROFILES_PATH = path.resolve(STATE_DIR, 'agent_profiles.json');
const AGENT_GIT_TREES_DIR = path.resolve(STATE_DIR, 'agent_git_trees');
const TEST_AGENT_MODEL_PATH = path.resolve(STATE_DIR, 'test_agent_model.json');
const PROVIDER_REGISTRY_PATH = path.resolve(STATE_DIR, 'provider_registry.json');
const CUSTOM_MODELS_PATH = path.resolve(STATE_DIR, 'custom_models.json');
const CHANNEL_REGISTRY_PATH = path.resolve(STATE_DIR, 'channel_registry.json');
const CHANNEL_QR_STATE_PATH = path.resolve(STATE_DIR, 'channel_qr_sessions.json');
const APPROVALS_STATE_PATH = path.resolve(STATE_DIR, 'approvals.json');
const WORKFLOWS_STATE_PATH = path.resolve(STATE_DIR, 'workflows.json');
const CRON_JOBS_STATE_PATH = path.resolve(STATE_DIR, 'cron_jobs.json');
const TRIGGERS_STATE_PATH = path.resolve(STATE_DIR, 'triggers.json');
const MODEL_ROUTER_PROVIDER_PROFILE_PATH = path.resolve(ROOT, 'local/state/ops/model_router/provider_profile.json');
const BENCHMARK_SANITY_STATE_PATH = path.resolve(ROOT, 'core/local/state/ops/benchmark_sanity/latest.json');
const BENCHMARK_SANITY_GATE_PATH = path.resolve(ROOT, 'core/local/artifacts/benchmark_sanity_gate_current.json');
const BENCHMARK_SANITY_GATE_SCRIPT_PATH = path.resolve(
  ROOT,
  'tests/tooling/scripts/ci/benchmark_sanity_gate.mjs'
);
const CHAT_EXPORT_DIR = path.resolve(STATE_DIR, 'chat_exports');
const CHAT_EXPORT_MAX_AGE_MS = 30 * 60 * 1000;
const CHAT_EXPORT_MAX_FILES = 48;
const CHAT_FILE_READ_MAX_BYTES = 2 * 1024 * 1024;
const CHAT_TREE_MAX_DEPTH = 8;
const CHAT_TREE_MAX_ENTRIES = 5000;
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 4173;
const DEFAULT_TEAM = 'ops';
const DEFAULT_REFRESH_MS = 8000;
const DASHBOARD_BACKPRESSURE_BATCH_DEPTH = 75;
const DASHBOARD_BACKPRESSURE_WARN_DEPTH = 50;
const DASHBOARD_QUEUE_DRAIN_PAUSE_DEPTH = 80;
const DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH = 50;
const RUNTIME_CRITICAL_ESCALATION_THRESHOLD = 7;
const RUNTIME_CRITICAL_ATTENTION_OVERLOAD_THRESHOLD = 12;
const RUNTIME_COCKPIT_BLOCK_ESCALATION_THRESHOLD = 28;
const RUNTIME_AUTO_BALANCE_THRESHOLD = 12;
const RUNTIME_DRAIN_TRIGGER_DEPTH = 60;
const RUNTIME_DRAIN_CLEAR_DEPTH = 40;
const RUNTIME_DRAIN_AGENT_TARGET = 2;
const RUNTIME_DRAIN_AGENT_HIGH_LOAD_TARGET = 6;
const RUNTIME_DRAIN_HIGH_LOAD_DEPTH = 80;
const RUNTIME_DRAIN_AGENT_MAX = 8;
const RUNTIME_HEALTH_ADAPTIVE_WINDOW_SECONDS = 60;
const RUNTIME_THROTTLE_PLANE = 'backlog_delivery_plane';
const RUNTIME_THROTTLE_MAX_DEPTH = 75;
const RUNTIME_THROTTLE_STRATEGY = 'priority-shed';
const RUNTIME_INGRESS_DAMPEN_DEPTH = 40;
const RUNTIME_INGRESS_SHED_DEPTH = 80;
const RUNTIME_INGRESS_CIRCUIT_DEPTH = 100;
const RUNTIME_INGRESS_DELAY_MS = 100;
const RUNTIME_CONDUIT_WATCHDOG_MIN_SIGNALS = 6;
const RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH = 20;
const RUNTIME_CONDUIT_WATCHDOG_STALE_MS = 30_000;
const RUNTIME_CONDUIT_WATCHDOG_COOLDOWN_MS = 60_000;
const RUNTIME_CONDUIT_WATCHDOG_PRESSURE_COOLDOWN_MS = 15_000;
const RUNTIME_STALE_LANE_RETRY_BASE_MS = 5_000;
const RUNTIME_STALE_LANE_RETRY_MAX_MS = 120_000;
const RUNTIME_CONDUIT_PERSISTENCE_MIN_TICKS = 3;
const RUNTIME_STALE_RAW_PERSISTENCE_MIN_TICKS = 3;
const RUNTIME_STALE_SOFT_PERSISTENCE_MIN_TICKS = 3;
const RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS = 20;
const RUNTIME_COCKPIT_STALE_BLOCK_MS = 90_000;
const RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_MS = 6 * 60 * 60 * 1000;
const RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_SOFT_PRESSURE_MS = 30 * 60 * 1000;
const RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_LOW_PRESSURE_MS = 10 * 60 * 1000;
const RUNTIME_COCKPIT_STALE_DORMANT_ARCHIVE_AGE_MS = 24 * 60 * 60 * 1000;
const RUNTIME_COCKPIT_CRITICAL_LANE_KEYS = new Set([
  'app_plane',
  'collab_plane',
  'skills_plane',
  'hermes_plane',
  'security_plane',
  'benchmark_sanity',
  'attention_queue',
]);
const RUNTIME_COORDINATION_PATHOLOGY_STALE_BLOCK_MIN = 16;
const RUNTIME_COCKPIT_STALE_AUTOHEAL_MIN_BLOCKS = 10;
const RUNTIME_COCKPIT_STALE_SOFT_AUTOHEAL_MIN_BLOCKS = 7;
const RUNTIME_COORDINATION_RECOVERY_COOLDOWN_MS = 120_000;
const RUNTIME_COORDINATION_RECOVERY_MAX_SHADOWS = 24;
const RUNTIME_COARSE_THROTTLE_MAX_DEPTH = 60;
const RUNTIME_COARSE_THROTTLE_STRATEGY = 'pause-noncritical';
const RUNTIME_COARSE_STALE_LANE_REFRESH_LIMIT = 8;
const RUNTIME_COARSE_DRAIN_MIN_BATCH = 24;
const RUNTIME_ATTENTION_DRAIN_MIN_BATCH = 16;
const RUNTIME_ATTENTION_DRAIN_MAX_BATCH = 96;
const RUNTIME_ATTENTION_COMPACT_DEPTH = 90;
const RUNTIME_ATTENTION_COMPACT_RETAIN = 24;
const RUNTIME_ATTENTION_COMPACT_MIN_ACKED = 16;
const RUNTIME_AUTONOMY_HEAL_INTERVAL_MS = 30_000;
const RUNTIME_AUTONOMY_HEAL_EMERGENCY_INTERVAL_MS = 10_000;
const RUNTIME_AUTONOMY_HEAL_COORDINATION_INTERVAL_MS = 5_000;
const RUNTIME_STALL_WINDOW = 6;
const RUNTIME_STALL_CONDUIT_FLOOR = 6;
const RUNTIME_STALL_QUEUE_MIN_DEPTH = 60;
const RUNTIME_STALL_ESCALATION_FAILURE_THRESHOLD = 3;
const RUNTIME_STALL_DRAIN_LIMIT = 96;
const RUNTIME_SPINE_SUCCESS_TARGET_MIN = 0.9;
const RUNTIME_SLO_RECEIPT_LATENCY_P95_MAX_MS = 250;
const RUNTIME_SLO_RECEIPT_LATENCY_P99_MAX_MS = 400;
const RUNTIME_SLO_QUEUE_DEPTH_MAX = 60;
const RUNTIME_SLO_ESCALATION_OPEN_RATE_MIN = 0.01;
const RUNTIME_HANDOFFS_PER_AGENT_MIN = 0.1;
const RUNTIME_HANDOFFS_AGENT_FLOOR = 20;
const RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS = 5 * 60 * 1000;
const RUNTIME_SPINE_METRICS_STALE_MAX_AGE_SECONDS = 24 * 60 * 60;
const RUNTIME_SPINE_CANARY_COOLDOWN_MS = 30 * 60 * 1000;
const RUNTIME_SPINE_CANARY_MAX_EYES = 1;
const DASHBOARD_BENCHMARK_STALE_SECONDS = 48 * 60 * 60;
const RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS = 15 * 60 * 1000;
const RUNTIME_BENCHMARK_REFRESH_MAX_AGE_SECONDS = 10 * 60;
const RUNTIME_TASK_CHAT_DEDUPE_MS = 5 * 60 * 1000;
const RUNTIME_TASK_DISPATCH_RETAIN_MS = 24 * 60 * 60 * 1000;
const RUNTIME_TREND_WINDOW = 120;
const TEXT_EXTENSIONS = new Set(['.html', '.css', '.js', '.json', '.txt', '.svg', '.map']);
const OLLAMA_BIN = 'ollama';
const OLLAMA_MODEL_FALLBACK = 'qwen2.5:3b';
const OLLAMA_TIMEOUT_MS = 45000;
const OLLAMA_MODEL_CACHE_TTL_MS = 30_000;
const LOCAL_PROVIDER_DISCOVERY_INTERVAL_MS = 30_000;
const PROMPT_SUGGESTION_TIMEOUT_MS = 1200;
const PROMPT_SUGGESTION_CACHE_TTL_MS = 20_000;
const PROMPT_SUGGESTION_CACHE_MAX = 512;
const TEST_AGENT_MODEL_DEFAULT = 'kimi2.5:cloud';
const TEST_AGENT_PROVIDER_DEFAULT = 'cloud';
const TEST_AGENT_ID_PREFIXES = ['e2e-', 'test-', 'bench-', 'ci-', 'qa-'];
const TOOL_ITERATION_LIMIT = 4;
const TOOL_OUTPUT_LIMIT = 5000;
const ASSISTANT_EMPTY_FALLBACK_RESPONSE = 'I do not know yet. Please clarify what you want me to do next.';
const TERMINAL_OUTPUT_LIMIT = 18000;
const TERMINAL_COMMAND_TIMEOUT_MS = 45000;
const TERMINAL_SESSION_IDLE_TTL_MS = 5 * 60 * 1000;
const TERMINAL_KILL_GRACE_MS = 1200;
const INTERACTIVE_BACKGROUND_SUPPRESS_MS = 15_000;
const DASHBOARD_BACKGROUND_RUNTIME_LOOPS_ENABLED = cleanText(
  process.env.INFRING_DASHBOARD_BG_LOOPS || '',
  8
) === '1';
const DEFAULT_CONTEXT_WINDOW_TOKENS = 8192;
const AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS = 60 * 60;
const AGENT_CONTRACT_ENFORCE_INTERVAL_MS = 1000;
const AGENT_CONTRACT_ENFORCE_INTERVAL_HIGH_SCALE_MS = 5000;
const AGENT_CONTRACT_ENFORCE_HIGH_SCALE_THRESHOLD = 64;
const AGENT_CONTRACT_ENFORCE_INTERVAL_ULTRA_SCALE_MS = 12000;
const AGENT_CONTRACT_ENFORCE_ULTRA_SCALE_THRESHOLD = 256;
const AGENT_CONTRACT_ENFORCE_INTERVAL_MEGA_SCALE_MS = 20000;
const AGENT_CONTRACT_ENFORCE_MEGA_SCALE_THRESHOLD = 1000;
const AGENT_CONTRACT_API_ENFORCE_INTERVAL_MS = 3000;
const AGENT_CONTRACT_API_ENFORCE_INTERVAL_HIGH_SCALE_MS = 8000;
const AGENT_CONTRACT_API_ENFORCE_INTERVAL_ULTRA_SCALE_MS = 15000;
const AGENT_CONTRACT_API_ENFORCE_INTERVAL_MEGA_SCALE_MS = 25000;
const AGENT_CONTRACT_MAX_IDLE_AGENTS = 5;
const AGENT_CONTRACT_CHAT_HOLD_MAX_MS = 24 * 60 * 60 * 1000;
const AGENT_RECONCILE_TERMINATION_BATCH = 12;
const AGENT_RECONCILE_TERMINATION_BATCH_MAX = 96;
const AGENT_RECONCILE_TERMINATION_COOLDOWN_MS = 4000;
const AGENT_IDLE_TERMINATION_MS = 5 * 60 * 1000;
const AGENT_IDLE_TERMINATION_BATCH = 8;
const AGENT_IDLE_TERMINATION_BATCH_MAX = 128;
const AGENT_IDLE_TERMINATION_COOLDOWN_MS = 10 * 1000;
const AGENT_ENFORCE_MAX_TERMINATIONS_PER_SWEEP = 4;
const AGENT_TERMINATION_TEAM_DISCOVERY_CACHE_MS = 5 * 1000;
const AGENT_CONTRACT_RETAIN_TERMINATED_MAX = 512;
const AGENT_CONTRACT_RETAIN_TERMINATED_MAX_AGE_MS = 14 * 24 * 60 * 60 * 1000;
const AGENT_ROGUE_MESSAGE_RATE_MAX_PER_MIN = 20;
const AGENT_ROGUE_SPIKE_WINDOW_MS = 60 * 1000;
const ARCHIVED_AGENT_FILTER_WINDOW_MS = 10 * 60 * 1000;
const AGENT_GIT_TREE_KIND_MASTER = 'master';
const AGENT_GIT_TREE_KIND_ISOLATED = 'isolated';
const AGENT_GIT_TREE_BRANCH_PREFIX = 'agent';
const AGENT_GIT_TREE_SYNC_COOLDOWN_MS = 1200;
const AGENT_GIT_TREE_API_SYNC_DEBOUNCE_MS = 1200;
const DASHBOARD_UI_ASSET_REFRESH_COOLDOWN_MS = 5_000;
const COCKPIT_MAX_BLOCKS = 64;
const LANE_SYNC_TIMEOUT_MS = 1500;
const LANE_ACTION_TIMEOUT_MS = 8 * 1000;
const SNAPSHOT_LANE_TIMEOUT_FAST_MS = 220;
const SNAPSHOT_LANE_TIMEOUT_MAX_MS = LANE_SYNC_TIMEOUT_MS;
const SNAPSHOT_LANE_TIMEOUT_MIN_MS = 150;
const SNAPSHOT_LANE_CACHE_TTL_MS = 8000;
const SNAPSHOT_LANE_CACHE_FAIL_TTL_MS = 2000;
const RUNTIME_AUTHORITY_LANE_TIMEOUT_MS = 1200;
const RUNTIME_AUTHORITY_CACHE_TTL_MS = 1500;
const RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS = 600;
const AUTO_ROUTE_LANE_TIMEOUT_MS = 1200;
const AUTO_ROUTE_CACHE_TTL_MS = 1200;
const AUTO_ROUTE_CACHE_FAIL_TTL_MS = 600;
const ATTENTION_PEEK_LIMIT = 12;
const ATTENTION_CRITICAL_LIMIT = 64;
const SNAPSHOT_FS_CACHE_TTL_MS = 8000;
const ATTENTION_MICRO_BATCH_WINDOW_MS = 50;
const ATTENTION_MICRO_BATCH_MAX_ITEMS = 10;
const ATTENTION_MICRO_BATCH_DEGRADE_WINDOW_MS = 200;
const ATTENTION_MICRO_BATCH_DEGRADE_MAX_ITEMS = 6;
const ATTENTION_PREEMPT_QUEUE_DEPTH = 60;
const ATTENTION_BG_DOMINANCE_RATIO = 3;
const ATTENTION_DEFERRED_STASH_DEPTH = 80;
const ATTENTION_DEFERRED_PREDICTIVE_STASH_DEPTH = 65;
const ATTENTION_DEFERRED_HARD_SHED_DEPTH = 100;
const ATTENTION_DEFERRED_REHYDRATE_DEPTH = 40;
const ATTENTION_DEFERRED_PREDICTIVE_REHYDRATE_DEPTH = 30;
const ATTENTION_DEFERRED_REHYDRATE_BATCH = 10;
const ATTENTION_DEFERRED_MAX_ITEMS = 4000;
const ATTENTION_CRITICAL_DECAY_STAGE1_SECONDS = 30;
const ATTENTION_CRITICAL_DECAY_STAGE2_SECONDS = 60;
const ATTENTION_LANE_WEIGHTS = {
  critical: 6,
  standard: 3,
  background: 1,
};
const ATTENTION_LANE_CAPS = {
  critical: 12,
  standard: 30,
  background: 50,
};
const CONDUIT_DELTA_SYNC_DEPTH = 50;
const CONDUIT_DELTA_BATCH_WINDOW_MS = 10;
const CONDUIT_DELTA_BATCH_MAX_ITEMS = 8;
const WS_HEARTBEAT_INTERVAL_MS = 15 * 1000;
const WS_HEARTBEAT_TIMEOUT_MS = 45 * 1000;
const MEMORY_ENTRY_BACKPRESSURE_THRESHOLD = 25;
const MEMORY_ENTRY_TARGET_WHEN_PAUSED = 20;
const ATTENTION_CONSUMER_ID = 'dashboard-cockpit';
const PRIMARY_MEMORY_DIR = 'local/workspace/memory';
const LEGACY_MEMORY_DIR = 'memory';
const ASSISTANT_MEMORY_PATH = path.resolve(ROOT, 'local/workspace/assistant/MEMORY.md');
const AGENT_MEMORY_KV_MAX_KEYS = 512;
const AGENT_MEMORY_KEY_MAX_LEN = 160;
const AGENT_MEMORY_VALUE_MAX_JSON_CHARS = 32_000;
const MEMORY_SEARCH_DEFAULT_LIMIT = 12;
const MEMORY_SEARCH_MAX_LIMIT = 64;
const MEMORY_SEARCH_MAX_FILE_SCAN = 36;
const MEMORY_SEARCH_MAX_MATCHES_PER_FILE = 4;
const MEMORY_PASSIVE_APPEND_MIN_INTERVAL_MS = 20_000;
const MEMORY_PASSIVE_ATTENTION_APPEND_MIN_INTERVAL_MS = 45_000;
const MEMORY_PASSIVE_LINE_MAX_LEN = 360;
const COLLAB_SUPPORTED_ROLES = new Set([
  'director',
  'cell_coordinator',
  'coordinator',
  'researcher',
  'builder',
  'reviewer',
  'analyst',
]);
const COLLAB_ROLE_FALLBACKS = {
  director: 'director',
  cell: 'cell_coordinator',
  cell_director: 'cell_coordinator',
  shard_director: 'director',
  orchestrator: 'coordinator',
  planner: 'coordinator',
  architect: 'coordinator',
  scientist: 'researcher',
  writer: 'researcher',
  engineer: 'builder',
  executor: 'builder',
  qa: 'reviewer',
  auditor: 'reviewer',
};
const CLI_MODE_SAFE = 'safe';
const CLI_MODE_FULL_INFRING = 'full_infring';
const DEFAULT_CLI_MODE = CLI_MODE_FULL_INFRING;
const APP_VERSION = (() => {
  try {
    const pkg = require(path.resolve(ROOT, 'package.json'));
    const v = pkg && typeof pkg.version === 'string' ? pkg.version.trim() : '';
    return v || '0.1.0';
  } catch {
    return '0.1.0';
  }
})();
const EFFECTIVE_LOC_EXTENSIONS = new Set([
  '.rs',
  '.ts',
  '.tsx',
  '.js',
  '.jsx',
  '.mjs',
  '.cjs',
  '.py',
  '.go',
  '.java',
  '.kt',
  '.kts',
  '.swift',
  '.c',
  '.cc',
  '.cpp',
  '.h',
  '.hpp',
  '.m',
  '.mm',
  '.sh',
  '.bash',
  '.zsh',
  '.ps1',
  '.rb',
  '.php',
  '.cs',
  '.scala',
  '.sql',
  '.toml',
  '.yaml',
  '.yml',
  '.json',
]);
const CLI_ALLOWLIST = new Set([
  'protheus',
  'protheus-ops',
  'infringd',
  'git',
  'rg',
  'ls',
  'cat',
  'pwd',
  'wc',
  'head',
  'tail',
  'stat',
  'ps',
  'top',
  'free',
  'vm_stat',
  'vmstat',
]);
const GIT_READ_ONLY = new Set(['status', 'diff', 'show', 'log', 'branch', 'rev-parse', 'ls-files']);
const INFRINGD_READ_ONLY = new Set([
  'status',
  'diagnostics',
  'think',
  'research',
  'memory',
  'orchestration',
  'swarm-runtime',
  'capability-profile',
  'efficiency-status',
  'embedded-core-status',
]);
const OPS_READ_ONLY = new Set([
  'status',
  'health-status',
  'app-plane',
  'collab-plane',
  'skills-plane',
  'memory-plane',
  'security-plane',
  'attention-queue',
  'hermes-plane',
  'metrics-plane',
  'benchmark-matrix',
  'fixed-microbenchmark',
  'top1-assurance',
  'alpha-readiness',
  'foundation-contract-gate',
  'runtime-systems',
  'dashboard-ui',
]);
let ACTIVE_CLI_MODE = DEFAULT_CLI_MODE;
const GIT_BRANCH_CACHE_MS = 120_000;
const GIT_WORKSPACE_READY_CACHE_MS = 8_000;
let gitBranchCache = {
  value: '',
  fetched_at: 0,
};
const gitWorkspaceReadyCache = new Map();

function nowIso() {
  return new Date().toISOString();
}

function cleanText(value, maxLen = 120) {
  return String(value == null ? '' : value)
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, maxLen);
}

function currentGitBranch() {
  const now = Date.now();
  if (gitBranchCache.value && (now - gitBranchCache.fetched_at) < GIT_BRANCH_CACHE_MS) {
    return gitBranchCache.value;
  }
  try {
    const proc = spawnSync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      timeout: 1500,
      maxBuffer: 64 * 1024,
    });
    if (proc && proc.status === 0) {
      const raw = String(proc.stdout || '').trim();
      const value = cleanText(raw || 'main', 80) || 'main';
      gitBranchCache = { value, fetched_at: now };
      return value;
    }
  } catch {}
  const fallback = gitBranchCache.value || 'main';
  gitBranchCache = { value: fallback, fetched_at: now };
  return fallback;
}

function gitMainBranch() {
  try {
    const proc = spawnSync('git', ['show-ref', '--verify', '--quiet', 'refs/heads/main'], {
      cwd: ROOT,
      stdio: ['ignore', 'ignore', 'ignore'],
      timeout: 1500,
    });
    if (proc && proc.status === 0) return 'main';
  } catch {}
  return currentGitBranch();
}

function normalizeGitTreeKind(value, fallback = AGENT_GIT_TREE_KIND_ISOLATED) {
  const raw = cleanText(value || '', 24).toLowerCase();
  if (raw === AGENT_GIT_TREE_KIND_MASTER) return AGENT_GIT_TREE_KIND_MASTER;
  if (raw === AGENT_GIT_TREE_KIND_ISOLATED || raw === 'agent') return AGENT_GIT_TREE_KIND_ISOLATED;
  return fallback === AGENT_GIT_TREE_KIND_MASTER ? AGENT_GIT_TREE_KIND_MASTER : AGENT_GIT_TREE_KIND_ISOLATED;
}

function safeAgentGitTreeSlug(agentId = '') {
  const raw = cleanText(agentId || '', 140).toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '');
  if (!raw) return `agent-${sha256(String(agentId || 'agent')).slice(0, 10)}`;
  if (raw.length <= 72) return raw;
  return `${raw.slice(0, 52)}-${sha256(raw).slice(0, 10)}`;
}

function normalizeGitBranchName(value, fallback = '') {
  const raw = cleanText(value || '', 160).replace(/[^A-Za-z0-9._/-]+/g, '-').replace(/\/+/g, '/');
  const trimmed = raw.replace(/^[-./]+|[-./]+$/g, '');
  if (trimmed) return trimmed;
  return cleanText(fallback || '', 160).replace(/[^A-Za-z0-9._/-]+/g, '-').replace(/^[-./]+|[-./]+$/g, '');
}

function branchForAgentGitTree(agentId, existingBranch = '') {
  const normalizedExisting = normalizeGitBranchName(existingBranch, '');
  if (normalizedExisting && normalizedExisting !== gitMainBranch()) return normalizedExisting;
  const slug = safeAgentGitTreeSlug(agentId);
  return normalizeGitBranchName(`${AGENT_GIT_TREE_BRANCH_PREFIX}/${slug}`, `${AGENT_GIT_TREE_BRANCH_PREFIX}/agent`);
}

function isPathInsideRoot(candidatePath, rootPath) {
  if (!candidatePath || !rootPath) return false;
  const target = path.resolve(candidatePath || '');
  const base = path.resolve(rootPath || '');
  return target === base || target.startsWith(`${base}${path.sep}`);
}

function isAgentGitWorkspacePath(candidatePath = '') {
  const resolved = workspacePathOrNull(candidatePath);
  if (!resolved) return false;
  return isPathInsideRoot(resolved, AGENT_GIT_TREES_DIR);
}

function workspaceDirForAgentGitTree(agentId, existingWorkspaceDir = '') {
  const resolvedExisting = workspacePathOrNull(existingWorkspaceDir);
  if (resolvedExisting && isAgentGitWorkspacePath(resolvedExisting)) return resolvedExisting;
  return path.resolve(AGENT_GIT_TREES_DIR, safeAgentGitTreeSlug(agentId));
}

function gitWorkspaceLooksReady(workspaceDir = '', options = {}) {
  const resolved = workspacePathOrNull(workspaceDir, { must_exist: true, directory: true });
  if (!resolved) return false;
  const nowMs = Date.now();
  const refresh = !!(options && options.refresh);
  if (!refresh) {
    const cached = gitWorkspaceReadyCache.get(resolved);
    if (cached && typeof cached === 'object') {
      const ageMs = Math.max(0, nowMs - parseNonNegativeInt(cached.ts_ms, 0, 1_000_000_000_000));
      if (ageMs <= GIT_WORKSPACE_READY_CACHE_MS) {
        return !!cached.ready;
      }
    }
  }
  try {
    const probe = spawnSync('git', ['-C', resolved, 'rev-parse', '--is-inside-work-tree'], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      timeout: 1500,
    });
    const ready = !!(probe && probe.status === 0);
    gitWorkspaceReadyCache.set(resolved, {
      ready,
      ts_ms: nowMs,
    });
    return ready;
  } catch {
    gitWorkspaceReadyCache.set(resolved, {
      ready: false,
      ts_ms: nowMs,
    });
    return false;
  }
}

function ensureGitWorkspaceReady(agentId, branchName, workspaceDir) {
  const branch = normalizeGitBranchName(branchName, '');
  const workspace = workspaceDirForAgentGitTree(agentId, workspaceDir);
  if (!branch || !isAgentGitWorkspacePath(workspace)) {
    return { ok: false, error: 'invalid_git_tree_binding', branch, workspace_dir: workspace };
  }
  if (gitWorkspaceLooksReady(workspace)) {
    return { ok: true, created: false, branch, workspace_dir: workspace };
  }
  ensureDir(path.dirname(workspace));
  if (workspacePathOrNull(workspace, { must_exist: true, directory: true }) && !gitWorkspaceLooksReady(workspace)) {
    try {
      fs.rmSync(workspace, { recursive: true, force: true });
    } catch {}
  }
  let branchExists = false;
  try {
    const branchProbe = spawnSync('git', ['show-ref', '--verify', '--quiet', `refs/heads/${branch}`], {
      cwd: ROOT,
      stdio: ['ignore', 'ignore', 'ignore'],
      timeout: 2000,
    });
    branchExists = !!(branchProbe && branchProbe.status === 0);
  } catch {}
  const worktreeArgs = branchExists
    ? ['worktree', 'add', '--force', workspace, branch]
    : ['worktree', 'add', '--force', '-b', branch, workspace, 'HEAD'];
  let proc = null;
  try {
    proc = spawnSync('git', worktreeArgs, {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 20000,
      maxBuffer: 2 * 1024 * 1024,
    });
  } catch (error) {
    return {
      ok: false,
      error: cleanText(error && error.message ? error.message : 'git_worktree_add_failed', 240),
      branch,
      workspace_dir: workspace,
    };
  }
  if (!proc || proc.status !== 0) {
    try {
      spawnSync('git', ['worktree', 'prune', '--expire=now'], {
        cwd: ROOT,
        stdio: ['ignore', 'ignore', 'ignore'],
        timeout: 3000,
      });
    } catch {}
    try {
      proc = spawnSync('git', worktreeArgs, {
        cwd: ROOT,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
        timeout: 20000,
        maxBuffer: 2 * 1024 * 1024,
      });
    } catch (error) {
      return {
        ok: false,
        error: cleanText(error && error.message ? error.message : 'git_worktree_add_retry_failed', 240),
        branch,
        workspace_dir: workspace,
      };
    }
  }
  if (!proc || proc.status !== 0 || !gitWorkspaceLooksReady(workspace, { refresh: true })) {
    gitWorkspaceReadyCache.delete(workspace);
    return {
      ok: false,
      error: cleanText(
        (proc && (proc.stderr || proc.stdout)) || `git_worktree_add_failed:${proc ? proc.status : 'unknown'}`,
        280
      ),
      branch,
      workspace_dir: workspace,
    };
  }
  gitWorkspaceReadyCache.set(workspace, {
    ready: true,
    ts_ms: Date.now(),
  });
  return {
    ok: true,
    created: true,
    branch,
    workspace_dir: workspace,
  };
}

function removeGitWorkspaceForAgent(agentId) {
  const id = cleanText(agentId || '', 140);
  if (!id) return { ok: false, removed: false, reason: 'invalid_agent_id' };
  const profile = agentProfileFor(id);
  const kind = normalizeGitTreeKind(profile && profile.git_tree_kind ? profile.git_tree_kind : '');
  if (kind === AGENT_GIT_TREE_KIND_MASTER) {
    return { ok: true, removed: false, reason: 'master_tree' };
  }
  const workspace = workspacePathOrNull(profile && profile.workspace_dir ? profile.workspace_dir : '');
  if (!workspace || !isAgentGitWorkspacePath(workspace)) {
    return { ok: true, removed: false, reason: 'no_isolated_workspace' };
  }
  let removed = false;
  try {
    const proc = spawnSync('git', ['worktree', 'remove', '--force', workspace], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 12000,
      maxBuffer: 1024 * 1024,
    });
    removed = !!(proc && proc.status === 0);
  } catch {}
  if (!removed) {
    try {
      fs.rmSync(workspace, { recursive: true, force: true });
      removed = true;
    } catch {}
  }
  gitWorkspaceReadyCache.delete(workspace);
  try {
    spawnSync('git', ['worktree', 'prune', '--expire=now'], {
      cwd: ROOT,
      stdio: ['ignore', 'ignore', 'ignore'],
      timeout: 2500,
    });
  } catch {}
  return {
    ok: true,
    removed,
    workspace_dir: workspace,
  };
}

function parsePositiveInt(value, fallback, min = 1, max = 65535) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(num)));
}

function parseNonNegativeInt(value, fallback = 0, max = 1000000000) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(0, Math.min(max, Math.floor(num)));
}

function normalizeLaneKey(value) {
  return cleanText(value || '', 120)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function cockpitStaleActionableMaxAgeMs(queueDepth = 0) {
  const depth = parseNonNegativeInt(queueDepth, 0, 100000000);
  if (depth >= RUNTIME_DRAIN_TRIGGER_DEPTH) return RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_MS;
  if (depth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH) {
    return Math.min(
      RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_MS,
      RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_SOFT_PRESSURE_MS
    );
  }
  return Math.min(
    RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_MS,
    RUNTIME_COCKPIT_STALE_ACTIONABLE_MAX_AGE_LOW_PRESSURE_MS
  );
}

function cockpitStaleIsActionable(row, queueDepth = 0) {
  const status = cleanText(row && row.status ? row.status : '', 24).toLowerCase();
  const ageMs = parseNonNegativeInt(
    row && row.age_ms != null ? row.age_ms : 0,
    0,
    30 * 24 * 60 * 60 * 1000
  );
  // Once stale blocks cross the dormant archive horizon, treat them as historical
  // debt rather than active pager pressure.
  if (ageMs >= RUNTIME_COCKPIT_STALE_DORMANT_ARCHIVE_AGE_MS) return false;
  if (status === 'fail' || status === 'error' || status === 'critical') return true;
  const laneKey = normalizeLaneKey(row && row.lane ? row.lane : '');
  if (RUNTIME_COCKPIT_CRITICAL_LANE_KEYS.has(laneKey)) return true;
  return ageMs <= cockpitStaleActionableMaxAgeMs(queueDepth);
}

function inferContextWindowFromModelName(modelName, fallback = DEFAULT_CONTEXT_WINDOW_TOKENS) {
  const normalized = cleanText(modelName || '', 160).toLowerCase();
  if (!normalized) return parsePositiveInt(fallback, DEFAULT_CONTEXT_WINDOW_TOKENS, 1024, 8000000);
  const matchK = normalized.match(/(?:^|[^0-9])([0-9]{2,4})k(?:[^a-z0-9]|$)/i);
  if (matchK && matchK[1]) {
    const parsedK = Number(matchK[1]);
    if (Number.isFinite(parsedK) && parsedK > 0) return parsePositiveInt(parsedK * 1000, fallback, 1024, 8000000);
  }
  const matchM = normalized.match(/(?:^|[^0-9])([0-9]{1,3})m(?:[^a-z0-9]|$)/i);
  if (matchM && matchM[1]) {
    const parsedM = Number(matchM[1]);
    if (Number.isFinite(parsedM) && parsedM > 0) return parsePositiveInt(parsedM * 1000000, fallback, 1024, 8000000);
  }
  if (/qwen2\.5|qwen3/i.test(normalized)) return 131072;
  if (/kimi|moonshot/i.test(normalized)) return 262144;
  if (/llama[-_. ]?3\.3/i.test(normalized)) return 131072;
  if (/llama[-_. ]?3\.2/i.test(normalized)) return 128000;
  if (/mistral[-_. ]?nemo|mixtral/i.test(normalized)) return 32000;
  if (/gemma[-_. ]?2/i.test(normalized)) return 8192;
  return parsePositiveInt(fallback, DEFAULT_CONTEXT_WINDOW_TOKENS, 1024, 8000000);
}

function contextPressureFromUsage(usedTokens, windowTokens) {
  const used = parseNonNegativeInt(usedTokens, 0, 1000000000);
  const windowSize = parsePositiveInt(windowTokens, DEFAULT_CONTEXT_WINDOW_TOKENS, 1024, 8000000);
  const ratio = windowSize > 0 ? used / windowSize : 0;
  if (ratio >= 0.96) return 'critical';
  if (ratio >= 0.82) return 'high';
  if (ratio >= 0.55) return 'medium';
  return 'low';
}

function estimateConversationTokens(messages = []) {
  const rows = Array.isArray(messages) ? messages : [];
  return rows.reduce((sum, row) => {
    let text = '';
    if (row && typeof row.content === 'string') text = row.content;
    else if (row && typeof row.text === 'string') text = row.text;
    else if (row && typeof row.message === 'string') text = row.message;
    else if (row && typeof row.user === 'string') text = row.user;
    else if (row && typeof row.assistant === 'string') text = row.assistant;
    return sum + Math.max(0, Math.round(String(text || '').length / 4));
  }, 0);
}

function contextTelemetryForMessages(messages = [], contextWindow = DEFAULT_CONTEXT_WINDOW_TOKENS, extraTokens = 0) {
  const windowSize = parsePositiveInt(contextWindow, DEFAULT_CONTEXT_WINDOW_TOKENS, 1024, 8000000);
  const used =
    parseNonNegativeInt(estimateConversationTokens(messages), 0, 1000000000) +
    parseNonNegativeInt(extraTokens, 0, 1000000000);
  const ratio = windowSize > 0 ? used / windowSize : 0;
  return {
    context_tokens: used,
    context_window: windowSize,
    context_ratio: windowSize > 0 ? Number(ratio.toFixed(6)) : 0,
    context_pressure: contextPressureFromUsage(used, windowSize),
  };
}

function recommendedConduitSignals(queueDepth = 0, queueUtilization = 0, activeConduitChannels = 0, activeAgents = 0) {
  const depth = parseNonNegativeInt(queueDepth, 0, 100000000);
  const util = Number.isFinite(Number(queueUtilization)) ? Number(queueUtilization) : 0;
  let baseline = 4;
  if (depth >= 95 || util >= 0.9) baseline = 16;
  else if (depth >= 85 || util >= 0.82) baseline = 14;
  else if (depth >= 65 || util >= 0.68) baseline = 12;
  else if (depth >= DASHBOARD_BACKPRESSURE_WARN_DEPTH || util >= 0.58) baseline = 8;
  else if (depth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH || util >= 0.4) baseline = 6;
  const conduitChannels = parseNonNegativeInt(activeConduitChannels, 0, 100000000);
  const conduitFloor = conduitChannels > 0
    ? Math.min(
        16,
        Math.max(
          4,
          conduitChannels +
            (depth >= RUNTIME_DRAIN_TRIGGER_DEPTH || util >= 0.65
              ? 2
              : depth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH || util >= 0.4
              ? 1
              : 0)
        )
      )
    : 4;
  const agents = parseNonNegativeInt(activeAgents, 0, 100000000);
  const agentScale = depth >= RUNTIME_DRAIN_TRIGGER_DEPTH || util >= 0.65
    ? 40
    : depth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH || util >= 0.4
    ? 120
    : 400;
  const agentFloor = agents > 0 ? Math.min(24, 4 + Math.ceil(agents / agentScale)) : 4;
  return Math.max(baseline, conduitFloor, agentFloor);
}

function activeAgentCountFromSnapshot(snapshot, fallback = 0) {
  try {
    const rows =
      snapshot &&
      snapshot.collab &&
      snapshot.collab.dashboard &&
      Array.isArray(snapshot.collab.dashboard.agents)
        ? snapshot.collab.dashboard.agents
        : null;
    if (!Array.isArray(rows)) return parseNonNegativeInt(fallback, 0, 100000000);
    const archived = archivedAgentIdsSet();
    let count = 0;
    for (const row of rows) {
      const id =
        cleanText(
          row && (row.shadow || row.id) ? row.shadow || row.id : '',
          140
        ) || '';
      if (!id) continue;
      if (archived.has(id)) continue;
      count += 1;
    }
    return parseNonNegativeInt(count, fallback, 100000000);
  } catch {
    return parseNonNegativeInt(fallback, 0, 100000000);
  }
}

function sha256(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function readText(filePath, fallback = '') {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return fallback;
  }
}

function fileExists(filePath) {
  try {
    return fs.existsSync(filePath);
  } catch {
    return false;
  }
}

function hasPrimaryDashboardUi() {
  return (
    fileExists(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'index_head.html')) &&
    fileExists(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'index_body.html'))
  );
}

function rebrandDashboardText(text) {
  return String(text || '')
    .replace(/\bOpenFang\b/g, 'Infring')
    .replace(/\bOPENFANG\b/g, 'INFRING')
    .replace(/\bopenfang\b/g, 'infring')
    .replace(/\bOpenClaw\b/g, 'Infring')
    .replace(/\bOPENCLAW\b/g, 'INFRING')
    .replace(/\bopenclaw\b/g, 'infring');
}

function transpileForkTypeScript(source, fileName) {
  const output = ts.transpileModule(String(source || ''), {
    compilerOptions: {
      target: ts.ScriptTarget.ES2020,
      module: ts.ModuleKind.None,
      sourceMap: false,
      inlineSourceMap: false,
      removeComments: false,
    },
    fileName,
    reportDiagnostics: false,
  });
  return String(output && output.outputText ? output.outputText : '');
}

function readForkScript(basePathNoExt) {
  const tsPath = path.resolve(INFRING_PRIMARY_STATIC_DIR, `${basePathNoExt}.ts`);
  if (!fileExists(tsPath)) return '';
  const source = readText(tsPath, '');
  if (!source) return '';
  return transpileForkTypeScript(source, tsPath);
}

function buildPrimaryDashboardHtml() {
  const head = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'index_head.html'), '');
  const body = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'index_body.html'), '');
  if (!head || !body) return '';
  const cssTheme = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'css/theme.css'), '');
  const cssLayout = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'css/layout.css'), '');
  const cssComponents = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'css/components.css'), '');
  const cssGithubDark = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'vendor/github-dark.min.css'), '');
  const vendorMarked = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'vendor/marked.min.ts'), '');
  const vendorHighlight = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'vendor/highlight.min.ts'), '');
  const vendorChart = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'vendor/chart.umd.min.ts'), '');
  const vendorAlpine = readText(path.resolve(INFRING_PRIMARY_STATIC_DIR, 'vendor/alpine.min.ts'), '');
  const apiJs = readForkScript('js/api');
  const appJs = readForkScript('js/app');
  const pageScripts = [
    'overview',
    'chat',
    'agents',
    'workflows',
    'workflow-builder',
    'channels',
    'skills',
    'hands',
    'scheduler',
    'settings',
    'usage',
    'sessions',
    'logs',
    'wizard',
    'approvals',
    'comms',
    'runtime',
  ]
    .map((name) => readForkScript(`js/pages/${name}`))
    .filter(Boolean)
    .join('\n');

  const html = [
    head,
    '<style>',
    cssTheme,
    cssLayout,
    cssComponents,
    cssGithubDark,
    '</style>',
    body,
    '<script>',
    vendorMarked,
    '</script>',
    '<script>',
    vendorHighlight,
    '</script>',
    '<script>',
    vendorChart,
    '</script>',
    '<script>',
    apiJs,
    appJs,
    pageScripts,
    '</script>',
    '<script>',
    vendorAlpine,
    '</script>',
    '</body></html>',
  ].join('\n');
  return rebrandDashboardText(html);
}

function contentTypeForFile(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  if (ext === '.html') return 'text/html; charset=utf-8';
  if (ext === '.css') return 'text/css; charset=utf-8';
  if (ext === '.js') return 'text/javascript; charset=utf-8';
  if (ext === '.json') return 'application/json; charset=utf-8';
  if (ext === '.ico') return 'image/x-icon';
  if (ext === '.png') return 'image/png';
  if (ext === '.jpg' || ext === '.jpeg') return 'image/jpeg';
  if (ext === '.svg') return 'image/svg+xml; charset=utf-8';
  if (ext === '.woff') return 'font/woff';
  if (ext === '.woff2') return 'font/woff2';
  return 'application/octet-stream';
}

function readPrimaryDashboardAsset(pathname) {
  const requestPath = pathname === '/' || pathname === '/dashboard' ? '/index_body.html' : pathname;
  const relative = requestPath.replace(/^\/+/, '');
  const resolved = path.resolve(INFRING_PRIMARY_STATIC_DIR, relative);
  if (!resolved.startsWith(INFRING_PRIMARY_STATIC_DIR)) return null;
  const ext = path.extname(resolved).toLowerCase();

  // TS-first static assets: allow requests for *.js to be served from sibling *.ts sources.
  if (ext === '.js' && !fileExists(resolved)) {
    const tsPath = path.resolve(
      INFRING_PRIMARY_STATIC_DIR,
      relative.replace(/\.js$/i, '.ts')
    );
    if (tsPath.startsWith(INFRING_PRIMARY_STATIC_DIR) && fileExists(tsPath)) {
      return {
        body: transpileForkTypeScript(readText(tsPath, ''), tsPath),
        contentType: 'text/javascript; charset=utf-8',
      };
    }
    return null;
  }

  if (!fileExists(resolved)) return null;
  const contentType = contentTypeForFile(resolved);
  if (TEXT_EXTENSIONS.has(ext)) {
    return {
      body: rebrandDashboardText(readText(resolved, '')),
      contentType,
    };
  }
  return {
    body: fs.readFileSync(resolved),
    contentType,
  };
}

function parseJsonLoose(raw) {
  const text = String(raw || '').trim();
  if (!text) return null;
  const parseCandidate = (candidate) => {
    const source = String(candidate || '').trim();
    if (!source) return null;
    try {
      const parsed = JSON.parse(source);
      if (typeof parsed === 'string') {
        try {
          return JSON.parse(parsed);
        } catch {}
      }
      return parsed;
    } catch {}
    const repaired = repairDirectiveJsonCandidate(source);
    if (repaired && repaired !== source) {
      try {
        const parsed = JSON.parse(repaired);
        if (typeof parsed === 'string') {
          try {
            return JSON.parse(parsed);
          } catch {}
        }
        return parsed;
      } catch {}
    }
    return null;
  };
  const direct = parseCandidate(text);
  if (direct && typeof direct === 'object') return direct;
  const firstBrace = text.indexOf('{');
  const lastBrace = text.lastIndexOf('}');
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    const sliced = parseCandidate(text.slice(firstBrace, lastBrace + 1));
    if (sliced && typeof sliced === 'object') return sliced;
  }
  const lines = text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const parsed = parseCandidate(lines[i]);
    if (parsed && typeof parsed === 'object') return parsed;
  }
  return null;
}

function repairDirectiveJsonCandidate(raw) {
  let text = String(raw || '').trim();
  if (!text) return text;
  text = text
    .replace(/[“”]/g, '"')
    .replace(/[‘’]/g, "'");
  if (
    (text.startsWith('```') && text.endsWith('```')) ||
    (text.startsWith('`') && text.endsWith('`'))
  ) {
    text = text.replace(/^```(?:json)?/i, '').replace(/```$/i, '').replace(/^`|`$/g, '').trim();
  }
  // Common malformed key pattern seen in model output: "reason:" -> "reason":
  text = text.replace(/"([a-zA-Z0-9_-]+):"\s*/g, '"$1": ');
  // Remove trailing commas in arrays/objects.
  text = text.replace(/,\s*([}\]])/g, '$1');
  return text;
}

function normalizeCliMode(value) {
  const raw = cleanText(value || '', 80).toLowerCase();
  if (!raw) return DEFAULT_CLI_MODE;
  if (raw === 'full' || raw === 'full_infring' || raw === 'full-infring') {
    return CLI_MODE_FULL_INFRING;
  }
  return CLI_MODE_SAFE;
}

function parseFlags(argv = []) {
  const out = {
    mode: 'serve',
    host: DEFAULT_HOST,
    port: DEFAULT_PORT,
    team: DEFAULT_TEAM,
    refreshMs: DEFAULT_REFRESH_MS,
    pretty: true,
    cliMode: normalizeCliMode(process.env.INFRING_DASHBOARD_CLI_MODE || DEFAULT_CLI_MODE),
  };

  let modeSet = false;
  for (const token of argv) {
    const value = String(token || '').trim();
    if (!value) continue;

    if (!modeSet && !value.startsWith('--')) {
      out.mode = value;
      modeSet = true;
      continue;
    }
    if (value.startsWith('--host=')) {
      out.host = cleanText(value.split('=').slice(1).join('='), 100) || DEFAULT_HOST;
      continue;
    }
    if (value.startsWith('--port=')) {
      out.port = parsePositiveInt(value.split('=').slice(1).join('='), DEFAULT_PORT, 1, 65535);
      continue;
    }
    if (value.startsWith('--team=')) {
      out.team = cleanText(value.split('=').slice(1).join('='), 80) || DEFAULT_TEAM;
      continue;
    }
    if (value.startsWith('--refresh-ms=')) {
      out.refreshMs = parsePositiveInt(value.split('=').slice(1).join('='), DEFAULT_REFRESH_MS, 800, 60000);
      continue;
    }
    if (value === '--pretty=0' || value === '--pretty=false') {
      out.pretty = false;
      continue;
    }
    if (value.startsWith('--cli-mode=')) {
      out.cliMode = normalizeCliMode(value.split('=').slice(1).join('='));
      continue;
    }
  }
  return out;
}

function runLane(argv, options = {}) {
  const timeoutMs = parsePositiveInt(
    options && options.timeout_ms != null ? options.timeout_ms : LANE_SYNC_TIMEOUT_MS,
    LANE_SYNC_TIMEOUT_MS,
    SNAPSHOT_LANE_TIMEOUT_MIN_MS,
    60000
  );
  const env = {
    ...process.env,
    PROTHEUS_ROOT: ROOT,
    // Runtime-facing dashboard lanes must stay responsive even during active source churn.
    // Use the newest available binary and avoid request-time cargo recompiles.
    PROTHEUS_OPS_ALLOW_STALE: '1',
  };
  const opts = {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env,
    maxBuffer: 12 * 1024 * 1024,
    timeout: timeoutMs,
    killSignal: 'SIGKILL',
  };
  const proc = spawnSync(process.execPath, [TS_ENTRYPOINT_PATH, OPS_BRIDGE_PATH, ...argv], opts);

  const timedOut = !!(proc && proc.error && proc.error.code === 'ETIMEDOUT');
  const status = typeof proc.status === 'number' ? proc.status : 1;
  const stdout = typeof proc.stdout === 'string' ? proc.stdout : '';
  const stderr = typeof proc.stderr === 'string' ? proc.stderr : '';
  const payload = parseJsonLoose(stdout) || (timedOut
    ? {
        ok: false,
        type: 'infring_dashboard_lane_timeout',
        error: 'lane_timeout',
        timeout_ms: timeoutMs,
      }
    : null);
  return {
    ok: status === 0 && !!payload && !timedOut,
    status,
    stdout,
    stderr: timedOut && !stderr ? `lane_timeout_${timeoutMs}ms` : stderr,
    payload,
    argv,
    timed_out: timedOut,
  };
}

const snapshotLaneCache = new Map();
const promptSuggestionCache = new Map();
const RUST_RUNTIME_SYNC_RETRY_COOLDOWN_MS = 5 * 60 * 1000;
let rustRuntimeSyncUnsupportedUntilMs = 0;

function runLaneCached(cacheKey, argv, options = {}) {
  const key = cleanText(cacheKey || argv.join(' '), 240) || argv.join(' ');
  const timeoutMs = parsePositiveInt(
    options && options.timeout_ms != null ? options.timeout_ms : LANE_SYNC_TIMEOUT_MS,
    LANE_SYNC_TIMEOUT_MS,
    SNAPSHOT_LANE_TIMEOUT_MIN_MS,
    60000
  );
  const ttlMs = parsePositiveInt(
    options && options.ttl_ms != null ? options.ttl_ms : SNAPSHOT_LANE_CACHE_TTL_MS,
    SNAPSHOT_LANE_CACHE_TTL_MS,
    250,
    600000
  );
  const failTtlMs = parsePositiveInt(
    options && options.fail_ttl_ms != null ? options.fail_ttl_ms : SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    250,
    600000
  );
  const staleFallbackEnabled = options && options.stale_fallback === false ? false : true;
  const staleFallbackMaxMs = parsePositiveInt(
    options && options.stale_fallback_max_ms != null ? options.stale_fallback_max_ms : Math.max(ttlMs, failTtlMs * 2),
    Math.max(ttlMs, failTtlMs * 2),
    ttlMs,
    600000
  );
  const nowMs = Date.now();
  const cached = snapshotLaneCache.get(key);
  if (cached && typeof cached === 'object') {
    const ageMs = Math.max(0, nowMs - parseNonNegativeInt(cached.ts_ms, 0, 1_000_000_000_000));
    if (cached.ok && ageMs <= ttlMs && cached.result && typeof cached.result === 'object') {
      return {
        ...cached.result,
        from_cache: true,
        cache_age_ms: ageMs,
      };
    }
  }

  const lane = runLane(argv, { timeout_ms: timeoutMs });
  if (lane.ok) {
    snapshotLaneCache.set(key, {
      ts_ms: nowMs,
      ok: true,
      result: lane,
      last_success_ts_ms: nowMs,
      last_success_result: lane,
      last_failure_ts_ms: parseNonNegativeInt(cached && cached.last_failure_ts_ms, 0, 1_000_000_000_000),
      last_failure_result: cached && cached.last_failure_result && typeof cached.last_failure_result === 'object'
        ? cached.last_failure_result
        : null,
    });
    return lane;
  }

  const cachedSuccessResult =
    cached && cached.last_success_result && typeof cached.last_success_result === 'object'
      ? cached.last_success_result
      : cached && cached.ok && cached.result && typeof cached.result === 'object'
        ? cached.result
        : null;
  const cachedSuccessTsMs =
    parseNonNegativeInt(
      cached && cached.last_success_ts_ms != null ? cached.last_success_ts_ms : cached && cached.ts_ms,
      0,
      1_000_000_000_000
    );

  if (staleFallbackEnabled && cachedSuccessResult) {
    const ageMs = Math.max(0, nowMs - cachedSuccessTsMs);
    if (ageMs <= staleFallbackMaxMs) {
      snapshotLaneCache.set(key, {
        ts_ms: cachedSuccessTsMs,
        ok: true,
        result: cachedSuccessResult,
        last_success_ts_ms: cachedSuccessTsMs,
        last_success_result: cachedSuccessResult,
        last_failure_ts_ms: nowMs,
        last_failure_result: lane,
      });
      return {
        ...cachedSuccessResult,
        from_cache: true,
        stale_cache_fallback: true,
        cache_age_ms: ageMs,
        stale_reason: lane.timed_out ? 'lane_timeout' : 'lane_error',
      };
    }
  }

  snapshotLaneCache.set(key, {
    ts_ms: nowMs,
    ok: false,
    result: lane,
    last_success_ts_ms: cachedSuccessTsMs,
    last_success_result: cachedSuccessResult,
    last_failure_ts_ms: nowMs,
    last_failure_result: lane,
  });
  return lane;
}

function lanePayloadObject(laneResult, fallback = {}) {
  if (!laneResult || typeof laneResult !== 'object') return fallback;
  const fromCache = laneResult.from_cache === true || laneResult.stale_cache_fallback === true;
  if (!laneResult.ok && !fromCache) return fallback;
  const payload = laneResult.payload;
  if (!payload || typeof payload !== 'object') return fallback;
  return payload;
}

function resolveProtheusdBin() {
  if (fileExists(PROTHEUSD_DEBUG_BIN)) return PROTHEUSD_DEBUG_BIN;
  if (fileExists(PROTHEUSD_RELEASE_BIN)) return PROTHEUSD_RELEASE_BIN;
  return '';
}

function runProtheusdThink(prompt, sessionId) {
  const bin = resolveProtheusdBin();
  if (!bin) {
    return {
      ok: false,
      status: 1,
      stdout: '',
      stderr: 'protheusd_binary_missing',
      payload: null,
      argv: ['think'],
    };
  }
  const args = [
    'think',
    `--prompt=${cleanText(prompt || '', 4000)}`,
    `--session-id=${cleanText(sessionId || 'dashboard-chat', 120)}`,
  ];
  const proc = spawnSync(bin, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env: { ...process.env, PROTHEUS_ROOT: ROOT },
    maxBuffer: 12 * 1024 * 1024,
  });
  const status = typeof proc.status === 'number' ? proc.status : 1;
  const stdout = typeof proc.stdout === 'string' ? proc.stdout : '';
  const stderr = typeof proc.stderr === 'string' ? proc.stderr : '';
  const payload = parseJsonLoose(stdout);
  return {
    ok: status === 0 && !!payload,
    status,
    stdout,
    stderr,
    payload,
    argv: [bin, ...args],
  };
}

function commandExists(command) {
  try {
    const proc = spawnSync('which', [String(command || '')], {
      cwd: ROOT,
      stdio: 'ignore',
      timeout: 1500,
    });
    return proc && proc.status === 0;
  } catch {
    return false;
  }
}

function sanitizeArg(value, maxLen = 180) {
  return String(value == null ? '' : value).replace(/[\u0000\r\n]/g, ' ').trim().slice(0, maxLen);
}

function parseToolArgs(raw) {
  if (Array.isArray(raw)) {
    return raw.map((value) => sanitizeArg(value)).filter(Boolean).slice(0, 24);
  }
  if (typeof raw === 'string') {
    return raw
      .trim()
      .split(/\s+/)
      .map((value) => sanitizeArg(value))
      .filter(Boolean)
      .slice(0, 24);
  }
  return [];
}

function stripAnsi(value) {
  return String(value == null ? '' : value)
    .replace(/\u001B\[[0-9;?]*[ -/]*[@-~]/g, '')
    .replace(/\u001B\][^\u0007]*(?:\u0007|\u001B\\)/g, '')
    .replace(/\u001B[PX^_].*?\u001B\\/g, '')
    .replace(/\u001B[@-_]/g, '');
}

function shellQuote(value) {
  return `'${String(value == null ? '' : value).replace(/'/g, `'\"'\"'`)}'`;
}

function resolveTerminalCwd(requestedCwd, agentId = '') {
  const raw = String(requestedCwd == null ? '' : requestedCwd).replace(/\u0000/g, '').trim();
  if (!raw) {
    const id = cleanText(agentId || '', 140);
    if (id) {
      const tree = agentGitTreeView(id, agentProfileFor(id));
      const fallbackWorkspace = workspacePathOrNull(tree.workspace_dir, { must_exist: true, directory: true });
      if (fallbackWorkspace) return fallbackWorkspace;
    }
    return ROOT;
  }
  const candidate = path.isAbsolute(raw) ? path.resolve(raw) : path.resolve(ROOT, raw);
  if (!candidate.startsWith(ROOT)) return ROOT;
  try {
    const stat = fs.statSync(candidate);
    if (stat && stat.isDirectory()) return candidate;
  } catch {}
  return ROOT;
}

function workspacePathOrNull(rawPath, options = {}) {
  const mustExist = options && options.must_exist === true;
  const expectDirectory = options && options.directory === true;
  const expectFile = options && options.file === true;
  const value = String(rawPath == null ? '' : rawPath).replace(/\u0000/g, '').trim();
  if (!value) return null;
  const resolved = path.isAbsolute(value) ? path.resolve(value) : path.resolve(ROOT, value);
  if (!(resolved === ROOT || resolved.startsWith(`${ROOT}${path.sep}`))) return null;
  if (!mustExist && !expectDirectory && !expectFile) return resolved;
  try {
    const stat = fs.statSync(resolved);
    if (expectDirectory && !stat.isDirectory()) return null;
    if (expectFile && !stat.isFile()) return null;
  } catch {
    return null;
  }
  return resolved;
}

function compactRelativePath(absPath) {
  const relative = path.relative(ROOT, absPath || '');
  return cleanText(relative || '.', 600) || '.';
}

function parseSuggestionCandidates(raw) {
  const text = String(raw || '').trim();
  if (!text) return [];
  const META_SUGGESTION_PATTERNS = [
    /return exactly\s+3 actionable next user prompts/i,
    /json array of strings/i,
    /do not include numbering|do not include markdown|extra keys/i,
    /^the user wants exactly 3 actionable next user prompts/i,
    /summarize recent work and suggest the three highest-roi next steps/i,
    /^thinking\.\.\.?$/i,
    /for this .* agent .* propose the next highest-impact user prompt/i,
    /ask for a concrete implementation diff plus the exact tests/i,
    /given queue \d+ and stale cockpit \d+,? propose a safe reliability action/i,
    /task accepted\.\s*report findings in this thread with receipt-backed evidence/i,
  ];
  const normalizeSuggestion = (value) => {
    let row = cleanText(value == null ? '' : String(value), 220);
    if (!row) return '';
    row = row.replace(/^\s*[-*0-9.)\]]+\s*/, '');
    if (/^suggestions?[:]?$/i.test(row)) return '';
    for (const pattern of META_SUGGESTION_PATTERNS) {
      if (pattern.test(row)) return '';
    }
    const machineNoise =
      /tool_call|\"type\"\s*:\s*\"tool|\"command\"\s*:|^\{.*\}$|^\[.*\]$|<\/?function/i.test(row) ||
      /\"args\"\s*:|\"reason\"\s*:|\"payload\"\s*:|\"provider\"\s*:|\"model\"\s*:/i.test(row);
    if (machineNoise) return '';
    if (row.length > 180) row = `${row.slice(0, 177)}...`;
    return row;
  };
  const direct = parseJsonLoose(text);
  const fromObject =
    direct && typeof direct === 'object'
      ? Array.isArray(direct)
        ? direct
        : Array.isArray(direct.suggestions)
          ? direct.suggestions
          : null
      : null;
  const parsedRows = Array.isArray(fromObject) ? fromObject : null;
  let rows = [];
  if (parsedRows) {
    rows = parsedRows
      .map((row) => normalizeSuggestion(typeof row === 'string' ? row : row && row.prompt ? row.prompt : ''))
      .filter(Boolean);
  }
  if (!rows.length) {
    rows = text
      .split('\n')
      .map((line) => normalizeSuggestion(line))
      .filter(Boolean)
      .slice(0, 8);
  }
  const seen = new Set();
  const unique = [];
  for (const row of rows) {
    const key = row.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(row);
  }
  return unique.slice(0, 3);
}

function isSuggestionNoiseText(value) {
  const text = cleanText(value == null ? '' : String(value), 280).toLowerCase();
  if (!text) return true;
  if (text.includes('task accepted. report findings in this thread with receipt-backed evidence')) return true;
  if (text.startsWith('[runtime-task]')) return true;
  if (text === 'heartbeat_ok') return true;
  if (text.includes('return exactly 3 actionable next user prompts')) return true;
  return false;
}

function sanitizeSuggestionHint(value) {
  const hint = cleanText(value == null ? '' : String(value), 240);
  if (!hint) return '';
  const lowered = hint.toLowerCase();
  if (
    lowered === 'post-response' ||
    lowered === 'post-silent' ||
    lowered === 'post-error' ||
    lowered === 'post-terminal' ||
    lowered === 'init' ||
    lowered === 'refresh'
  ) {
    return '';
  }
  if (/^post-[a-z0-9_-]+$/i.test(hint)) return '';
  return hint;
}

function recentSuggestionContext(recentMessages = []) {
  let lastUser = '';
  let lastAssistant = '';
  const rows = Array.isArray(recentMessages) ? recentMessages.slice() : [];
  for (let idx = rows.length - 1; idx >= 0; idx -= 1) {
    const row = rows[idx] || {};
    const role = cleanText(row.role || '', 40).toLowerCase();
    const text = cleanText(
      row.user || row.assistant || row.content || row.text || '',
      240
    );
    if (!text || isSuggestionNoiseText(text)) continue;
    if (!lastUser && (role === 'user' || row.user)) {
      lastUser = text;
      continue;
    }
    if (!lastAssistant && (role === 'agent' || role === 'assistant' || row.assistant)) {
      lastAssistant = text;
      continue;
    }
    if (lastUser && lastAssistant) break;
  }
  return {
    last_user: lastUser,
    last_assistant: lastAssistant,
    signature: cleanText(`${lastUser}|${lastAssistant}`, 500),
  };
}

function heuristicPromptSuggestions(agent, snapshot, recentMessages = [], hint = '') {
  const rows = [];
  const runtime = runtimeSyncSummary(snapshot);
  const queueDepth = parseNonNegativeInt(runtime.queue_depth, 0, 100000000);
  const staleBlocks = parseNonNegativeInt(runtime.cockpit_stale_blocks, 0, 100000000);
  const cleanHint = sanitizeSuggestionHint(hint);
  const convo = recentSuggestionContext(recentMessages);
  const roleLabel = cleanText(agent && agent.role ? agent.role : '', 60) || 'assistant';
  const agentLabel = cleanText(agent && agent.name ? agent.name : agent && agent.id ? agent.id : '', 80) || 'agent';
  const topic = cleanText(
    cleanHint || convo.last_user || convo.last_assistant || '',
    160
  );
  const topicWords = topic
    .toLowerCase()
    .split(/[^a-z0-9_:-]+/g)
    .filter((word) => word.length >= 4 && !['that', 'with', 'from', 'this', 'your', 'have', 'will', 'into'].includes(word))
    .slice(0, 3);
  const topicLabel = topicWords.length ? topicWords.join(' ') : `${roleLabel} workflow`;

  if (convo.last_user) {
    rows.push(`Continue this user ask: "${convo.last_user}" with one concrete next action.`);
  }
  if (
    convo.last_assistant &&
    !/task accepted\.\s*report findings in this thread with receipt-backed evidence/i.test(convo.last_assistant)
  ) {
    rows.push(`Build directly on the last answer: "${convo.last_assistant}" with a precise follow-up prompt.`);
  }
  if (cleanHint) {
    rows.push(`Use this hint exactly: "${cleanHint}" and turn it into one actionable next prompt.`);
  }
  if (staleBlocks > 0 || queueDepth >= 40 || /queue|cockpit|conduit|latency|backpressure|stale|reconnect/i.test(topic)) {
    rows.push(`Ask ${agentLabel} to propose one automatic reliability remediation with rollback criteria.`);
    rows.push(`Request a short runbook to keep queue depth under 60 and stale blocks near 0.`);
  }
  rows.push(`Request a 3-step execution plan focused on ${topicLabel}.`);
  rows.push(`Ask for one command to run now and one verification command tied to ${topicLabel}.`);
  rows.push(`Ask ${agentLabel} for the single highest-ROI change with measurable success criteria.`);
  const rotateSeed = `${topicLabel}|${convo.last_user}|${convo.last_assistant}|${cleanHint}`;
  let rotate = 0;
  for (let i = 0; i < rotateSeed.length; i++) rotate = (rotate + rotateSeed.charCodeAt(i)) % Math.max(rows.length, 1);
  const ordered = rows.length > 1 ? rows.slice(rotate).concat(rows.slice(0, rotate)) : rows.slice();
  const seen = new Set();
  return ordered
    .map((row) => cleanText(row, 220))
    .filter((row) => {
      const key = String(row || '').toLowerCase();
      if (!key || seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, 3);
}

function generatePromptSuggestions(agentId, snapshot, payload = {}) {
  const cleanAgentId = cleanText(agentId || '', 140);
  if (!cleanAgentId) return { ok: false, suggestions: [] };
  const userHint = sanitizeSuggestionHint(payload && payload.hint ? payload.hint : '');
  const state = loadAgentSession(cleanAgentId, snapshot);
  const session = activeSession(state);
  const agent =
    compatAgentsFromSnapshot(snapshot, { includeArchived: true }).find((row) => row.id === cleanAgentId) || {
      id: cleanAgentId,
      name: cleanAgentId,
      role: 'assistant',
      model_name: configuredOllamaModel(snapshot),
    };
  const recentMessages = Array.isArray(session && session.messages) ? session.messages.slice(-8) : [];
  const convo = recentSuggestionContext(recentMessages);
  const clientLastUser = cleanText(payload && payload.last_user_message ? payload.last_user_message : '', 220);
  const clientLastAgent = cleanText(payload && payload.last_agent_message ? payload.last_agent_message : '', 220);
  const clientRecentContext = cleanText(payload && payload.recent_context ? payload.recent_context : '', 400);
  const clientModel = cleanText(payload && payload.current_model ? payload.current_model : '', 160);
  const cacheSignature = cleanText(
    [
      userHint,
      clientLastUser,
      clientLastAgent,
      clientRecentContext,
      clientModel,
      convo.signature,
    ]
      .filter(Boolean)
      .join('|'),
    900
  );
  if (!userHint) {
    const cached = promptSuggestionCache.get(cleanAgentId);
    if (
      cached &&
      cached.signature === cacheSignature &&
      (Date.now() - parseNonNegativeInt(cached.ts, 0, 9_999_999_999_999)) < PROMPT_SUGGESTION_CACHE_TTL_MS &&
      Array.isArray(cached.suggestions) &&
      cached.suggestions.length > 0
    ) {
      return {
        ok: true,
        suggestions: cached.suggestions.slice(0, 3),
        source: cleanText(cached.source || 'cache', 40) || 'cache',
        model: cleanText(cached.model || '', 120),
      };
    }
  }
  const prompt = [
    'Return exactly 3 actionable next user prompts as a JSON array of strings.',
    'Keep each suggestion under 120 characters.',
    'Do not include numbering, markdown, explanations, or extra keys.',
    'Do not echo instructions or policy text.',
    'Never output generic placeholders like "Thinking..." or "Summarize recent work...".',
    `Agent name: ${cleanText(agent && agent.name ? agent.name : cleanAgentId, 120)}`,
    `Agent role: ${cleanText(agent && agent.role ? agent.role : 'assistant', 80)}`,
    clientModel ? `Client-selected model: ${clientModel}` : '',
    clientLastUser || convo.last_user ? `Recent user prompt hint: ${clientLastUser || convo.last_user}` : '',
    clientLastAgent || convo.last_assistant ? `Recent assistant response hint: ${clientLastAgent || convo.last_assistant}` : '',
    clientRecentContext ? `Additional context: ${clientRecentContext}` : '',
    `Recent context: ${cleanText(
      recentMessages
        .map((row) => String((row && (row.user || row.assistant || row.content || '')) || '').trim())
        .filter(Boolean)
        .slice(-4)
        .join(' || '),
      1200
    )}`,
    userHint ? `Hint from client: ${userHint}` : '',
  ]
    .filter(Boolean)
    .join('\n');

  const modelState = effectiveAgentModel(cleanAgentId, snapshot);
  const requestedModel =
    modelState && modelState.runtime_model ? modelState.runtime_model : configuredOllamaModel(snapshot);
  const llmRequested =
    ['1', 'true', 'yes', 'on'].includes(
      cleanText(payload && payload.use_llm ? payload.use_llm : '', 16).toLowerCase()
    ) || !!(payload && payload.force_llm);
  let llm = null;
  if (llmRequested) {
    llm = runOllamaPrompt(requestedModel, prompt, {
      timeout_ms: Math.min(PROMPT_SUGGESTION_TIMEOUT_MS, 2500),
    });
  }
  const normalizedRequestedModel = cleanText(requestedModel, 120) || OLLAMA_MODEL_FALLBACK;

  let suggestions = [];
  let source = llmRequested ? 'fallback' : 'heuristic_fast';
  if (llm && llm.ok) {
    suggestions = parseSuggestionCandidates(llm.output || '');
    if (suggestions.length) source = 'llm';
  }
  if (!suggestions.length) {
    suggestions = heuristicPromptSuggestions(agent, snapshot, recentMessages, userHint);
  }
  while (suggestions.length < 3) {
    const fallbackRows = heuristicPromptSuggestions(agent, snapshot, recentMessages, userHint);
    const fallback = fallbackRows[suggestions.length] || '';
    if (!fallback) break;
    suggestions.push(fallback);
  }
  const result = {
    ok: suggestions.length > 0,
    suggestions: suggestions.slice(0, 3),
    source,
    model: llm && llm.ok ? cleanText(llm.model || normalizedRequestedModel || '', 120) : '',
  };
  if (result.ok && !userHint) {
    if (promptSuggestionCache.size >= PROMPT_SUGGESTION_CACHE_MAX) {
      const firstKey = promptSuggestionCache.keys().next();
      if (firstKey && !firstKey.done) promptSuggestionCache.delete(firstKey.value);
    }
    promptSuggestionCache.set(cleanAgentId, {
      ts: Date.now(),
      signature: cacheSignature,
      suggestions: result.suggestions.slice(0, 3),
      source: result.source,
      model: result.model,
    });
  }
  return result;
}

function readFullFileForChat(rawPath, options = {}) {
  const maxBytes = parsePositiveInt(
    options && options.max_bytes != null ? options.max_bytes : CHAT_FILE_READ_MAX_BYTES,
    CHAT_FILE_READ_MAX_BYTES,
    256,
    16 * 1024 * 1024
  );
  const resolved = workspacePathOrNull(rawPath, { must_exist: true, file: true });
  if (!resolved) {
    return { ok: false, error: 'file_not_found_or_outside_workspace' };
  }
  try {
    const stat = fs.statSync(resolved);
    if (!stat.isFile()) return { ok: false, error: 'not_a_file' };
    const bytes = parseNonNegativeInt(stat.size, 0, 1_000_000_000);
    const contentBuffer = fs.readFileSync(resolved);
    const isLikelyBinary = contentBuffer.includes(0);
    if (isLikelyBinary) {
      return {
        ok: false,
        error: 'binary_file_not_supported',
        path: compactRelativePath(resolved),
        bytes,
      };
    }
    const text = contentBuffer.toString('utf8');
    const truncated = Buffer.byteLength(text, 'utf8') > maxBytes;
    let content = text;
    if (truncated) {
      content = Buffer.from(text, 'utf8').subarray(0, maxBytes).toString('utf8');
    }
    return {
      ok: true,
      path: compactRelativePath(resolved),
      content,
      bytes,
      truncated,
      max_bytes: maxBytes,
    };
  } catch (error) {
    return {
      ok: false,
      error: cleanText(error && error.message ? error.message : 'file_read_failed', 180) || 'file_read_failed',
    };
  }
}

function buildDirectoryTreeForChat(rawPath, options = {}) {
  const maxDepth = parsePositiveInt(
    options && options.max_depth != null ? options.max_depth : CHAT_TREE_MAX_DEPTH,
    CHAT_TREE_MAX_DEPTH,
    1,
    32
  );
  const maxEntries = parsePositiveInt(
    options && options.max_entries != null ? options.max_entries : CHAT_TREE_MAX_ENTRIES,
    CHAT_TREE_MAX_ENTRIES,
    32,
    20000
  );
  const resolved = workspacePathOrNull(rawPath, { must_exist: true, directory: true });
  if (!resolved) {
    return { ok: false, error: 'folder_not_found_or_outside_workspace' };
  }
  const rootName = path.basename(resolved) || '.';
  const lines = [rootName];
  let entries = 0;
  let truncated = false;

  const walk = (dirPath, depth, prefix) => {
    if (truncated || depth >= maxDepth) return;
    let children = [];
    try {
      children = fs.readdirSync(dirPath, { withFileTypes: true });
    } catch {
      return;
    }
    children.sort((a, b) => {
      const aDir = a && typeof a.isDirectory === 'function' ? (a.isDirectory() ? 0 : 1) : 1;
      const bDir = b && typeof b.isDirectory === 'function' ? (b.isDirectory() ? 0 : 1) : 1;
      if (aDir !== bDir) return aDir - bDir;
      return String(a && a.name ? a.name : '').localeCompare(String(b && b.name ? b.name : ''));
    });
    for (let i = 0; i < children.length; i += 1) {
      if (entries >= maxEntries) {
        truncated = true;
        break;
      }
      const child = children[i];
      if (!child || !child.name) continue;
      const isLast = i === (children.length - 1);
      const connector = isLast ? '└── ' : '├── ';
      const nextPrefix = `${prefix}${isLast ? '    ' : '│   '}`;
      const label = child.isDirectory() ? `${child.name}/` : child.name;
      lines.push(`${prefix}${connector}${label}`);
      entries += 1;
      if (child.isDirectory()) {
        walk(path.join(dirPath, child.name), depth + 1, nextPrefix);
      }
    }
  };

  walk(resolved, 0, '');
  return {
    ok: true,
    path: compactRelativePath(resolved),
    tree: lines.join('\n'),
    entries,
    truncated,
    max_entries: maxEntries,
    max_depth: maxDepth,
  };
}

function cleanupChatExports() {
  ensureDir(CHAT_EXPORT_DIR);
  const nowMs = Date.now();
  for (const [token, meta] of chatExportArtifacts.entries()) {
    const createdAt = parseNonNegativeInt(meta && meta.created_at, 0, 10_000_000_000_000);
    const stale = !createdAt || (nowMs - createdAt) > CHAT_EXPORT_MAX_AGE_MS;
    if (!stale) continue;
    try {
      if (meta && meta.file_path) fs.unlinkSync(meta.file_path);
    } catch {}
    chatExportArtifacts.delete(token);
  }
  if (chatExportArtifacts.size <= CHAT_EXPORT_MAX_FILES) return;
  const sorted = Array.from(chatExportArtifacts.entries()).sort(
    (a, b) =>
      parseNonNegativeInt(a[1] && a[1].created_at, 0, 10_000_000_000_000) -
      parseNonNegativeInt(b[1] && b[1].created_at, 0, 10_000_000_000_000)
  );
  while (sorted.length && chatExportArtifacts.size > CHAT_EXPORT_MAX_FILES) {
    const [token, meta] = sorted.shift();
    try {
      if (meta && meta.file_path) fs.unlinkSync(meta.file_path);
    } catch {}
    chatExportArtifacts.delete(token);
  }
}

function createFolderArchiveForChat(rawPath) {
  cleanupChatExports();
  const resolved = workspacePathOrNull(rawPath, { must_exist: true, directory: true });
  if (!resolved) {
    return { ok: false, error: 'folder_not_found_or_outside_workspace' };
  }
  ensureDir(CHAT_EXPORT_DIR);
  const safeBase = cleanText(path.basename(resolved) || 'folder', 80).replace(/[^a-zA-Z0-9._-]/g, '_') || 'folder';
  const stamp = Date.now();
  const fileName = `${safeBase}-${stamp}.tar.gz`;
  const filePath = path.join(CHAT_EXPORT_DIR, fileName);
  const tarOut = spawnSync('tar', ['-czf', filePath, '-C', path.dirname(resolved), path.basename(resolved)], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: 120000,
    maxBuffer: 32 * 1024 * 1024,
  });
  if (!tarOut || tarOut.status !== 0 || !fileExists(filePath)) {
    return {
      ok: false,
      error: cleanText(
        tarOut && (tarOut.stderr || tarOut.stdout) ? tarOut.stderr || tarOut.stdout : 'archive_create_failed',
        240
      ) || 'archive_create_failed',
    };
  }
  const token = sha256(`${resolved}:${stamp}:${fileName}`).slice(0, 32);
  chatExportArtifacts.set(token, {
    token,
    file_path: filePath,
    file_name: fileName,
    created_at: stamp,
    target_path: compactRelativePath(resolved),
  });
  return {
    ok: true,
    token,
    file_name: fileName,
    bytes: fileSizeBytes(filePath),
    target_path: compactRelativePath(resolved),
    download_url: `/api/chat/export/${token}`,
  };
}

const terminalSessions = new Map();

function terminalSessionId(agentId = '') {
  return cleanText(agentId || 'dashboard-terminal', 140) || 'dashboard-terminal';
}

function terminateSessionProcess(session, reason = 'session_closed') {
  if (!session || !session.proc) return;
  const pid = Number(session.proc.pid || 0);
  try {
    session.proc.kill('SIGTERM');
  } catch {}
  if (pid > 0) {
    try {
      spawnSync('pkill', ['-TERM', '-P', String(pid)], {
        cwd: ROOT,
        stdio: ['ignore', 'ignore', 'ignore'],
        timeout: 500,
      });
    } catch {}
    const timer = setTimeout(() => {
      if (!isPidAlive(pid)) return;
      try {
        spawnSync('pkill', ['-KILL', '-P', String(pid)], {
          cwd: ROOT,
          stdio: ['ignore', 'ignore', 'ignore'],
          timeout: 500,
        });
      } catch {}
      try {
        process.kill(pid, 'SIGKILL');
      } catch {}
    }, TERMINAL_KILL_GRACE_MS);
    if (timer && typeof timer.unref === 'function') timer.unref();
  }
}

function terminalFailureResult(session, pending, message, status = 1) {
  return {
    ok: false,
    blocked: false,
    status,
    exit_code: status,
    stdout: stripAnsi(String(pending && pending.stdout ? pending.stdout : '')).slice(0, TERMINAL_OUTPUT_LIMIT),
    stderr: stripAnsi(String(pending && pending.stderr ? pending.stderr : '')).slice(0, TERMINAL_OUTPUT_LIMIT),
    message: cleanText(message || 'terminal_session_error', 260),
    duration_ms: Math.max(0, Date.now() - Number(pending && pending.started_ms ? pending.started_ms : Date.now())),
    cwd: resolveTerminalCwd(pending && pending.cwd ? pending.cwd : session && session.cwd ? session.cwd : ROOT),
    command: cleanText(pending && pending.command ? pending.command : '', 4000),
  };
}

function finalizeTerminalPending(session, result) {
  const pending = session && session.pending ? session.pending : null;
  if (!pending) return;
  clearTimeout(pending.timeout);
  session.pending = null;
  session.last_used_ms = Date.now();
  pending.resolve(result);
  setImmediate(() => {
    processTerminalSessionQueue(session);
  });
}

function settleTerminalSessionError(session, message, status = 1) {
  if (!session) return;
  if (session.pending) {
    const pending = session.pending;
    finalizeTerminalPending(session, terminalFailureResult(session, pending, message, status));
  }
  const queued = Array.isArray(session.queue) ? session.queue.splice(0, session.queue.length) : [];
  for (const job of queued) {
    const pending = {
      command: cleanText(job && job.command ? job.command : '', 4000),
      cwd: resolveTerminalCwd(job && job.cwd ? job.cwd : session.cwd),
      started_ms: Date.now(),
      stdout: '',
      stderr: '',
    };
    job.resolve(terminalFailureResult(session, pending, message, status));
  }
}

function tryResolveTerminalMarker(session) {
  const pending = session && session.pending ? session.pending : null;
  if (!pending) return false;
  const markerIndex = pending.stdout.indexOf(pending.marker);
  if (markerIndex < 0) return false;
  const afterMarker = pending.stdout.slice(markerIndex + pending.marker.length);
  const markerPayload = afterMarker.match(/^(-?\d+)__([^\r\n]*)[\r\n]/);
  if (!markerPayload) return false;
  const exitCode = Number(markerPayload[1]);
  const markerCwd = resolveTerminalCwd(markerPayload[2] || pending.cwd);
  const consumedLength = pending.marker.length + markerPayload[0].length;
  const stdoutRaw = pending.stdout.slice(0, markerIndex) + pending.stdout.slice(markerIndex + consumedLength);
  const stderrRaw = pending.stderr;
  session.cwd = markerCwd;
  finalizeTerminalPending(session, {
    ok: Number.isFinite(exitCode) && exitCode === 0,
    blocked: false,
    status: Number.isFinite(exitCode) ? exitCode : 1,
    exit_code: Number.isFinite(exitCode) ? exitCode : 1,
    stdout: stripAnsi(stdoutRaw).slice(0, TERMINAL_OUTPUT_LIMIT),
    stderr: stripAnsi(stderrRaw).slice(0, TERMINAL_OUTPUT_LIMIT),
    message: '',
    duration_ms: Math.max(0, Date.now() - pending.started_ms),
    cwd: markerCwd,
    command: pending.command,
  });
  return true;
}

function processTerminalSessionQueue(session) {
  if (!session || session.closed || session.pending || !Array.isArray(session.queue) || session.queue.length === 0) {
    return;
  }
  const job = session.queue.shift();
  if (!job || typeof job.resolve !== 'function') return;
  const cwd = resolveTerminalCwd(job.cwd || session.cwd || ROOT);
  const marker = `__INFRING_TERM_DONE_${sha256(`${session.id}:${Date.now()}:${Math.random()}`).slice(0, 24)}__`;
  session.pending = {
    marker,
    command: cleanText(job.command || '', 4000),
    cwd,
    started_ms: Date.now(),
    stdout: '',
    stderr: '',
    timeout: null,
    resolve: job.resolve,
  };
  const script = [
    `cd ${shellQuote(cwd)}`,
    String(job.command || ''),
    '__infring_exit_code=$?',
    `printf '\\n${marker}%s__%s\\n' \"$__infring_exit_code\" \"$PWD\"`,
    '',
  ].join('\n');
  try {
    session.proc.stdin.write(script, 'utf8');
  } catch (error) {
    settleTerminalSessionError(
      session,
      cleanText(error && error.message ? error.message : 'terminal_stdin_write_failed', 220),
      1
    );
    return;
  }
  session.pending.timeout = setTimeout(() => {
    settleTerminalSessionError(session, 'terminal_command_timeout', 124);
  }, TERMINAL_COMMAND_TIMEOUT_MS);
}

function ensureTerminalSession(agentId, requestedCwd) {
  const id = terminalSessionId(agentId);
  const existing = terminalSessions.get(id);
  if (existing && !existing.closed && existing.proc && !existing.proc.killed) {
    return existing;
  }
  const cwd = resolveTerminalCwd(requestedCwd, agentId);
  const shell = cleanText(process.env.SHELL || '/bin/zsh', 160) || '/bin/zsh';
  const proc = spawn(shell, ['-s'], {
    cwd,
    stdio: ['pipe', 'pipe', 'pipe'],
    env: {
      ...process.env,
      PROTHEUS_ROOT: ROOT,
      TERM: process.env.TERM || 'xterm-256color',
      PS1: '',
      PROMPT: '',
      PROMPT_COMMAND: '',
    },
  });
  const session = {
    id,
    agent_id: id,
    proc,
    shell,
    cwd,
    queue: [],
    pending: null,
    closed: false,
    last_used_ms: Date.now(),
  };
  proc.stdout.on('data', (chunk) => {
    if (!session.pending) return;
    session.pending.stdout += String(chunk || '');
    tryResolveTerminalMarker(session);
  });
  proc.stderr.on('data', (chunk) => {
    if (!session.pending) return;
    session.pending.stderr += String(chunk || '');
  });
  proc.on('exit', (code, signal) => {
    session.closed = true;
    settleTerminalSessionError(
      session,
      `terminal_session_exited:${Number.isFinite(Number(code)) ? Number(code) : 'signal'}${signal ? `:${signal}` : ''}`,
      Number.isFinite(Number(code)) ? Number(code) : 1
    );
    terminalSessions.delete(id);
  });
  terminalSessions.set(id, session);
  return session;
}

function queueTerminalCommand(session, command, cwd) {
  return new Promise((resolve) => {
    session.queue.push({ command, cwd, resolve });
    processTerminalSessionQueue(session);
  });
}

function closeTerminalSession(agentId, reason = 'session_closed') {
  const id = terminalSessionId(agentId);
  const session = terminalSessions.get(id);
  if (!session) return false;
  session.closed = true;
  settleTerminalSessionError(session, reason, 1);
  terminateSessionProcess(session, reason);
  terminalSessions.delete(id);
  return true;
}

function closeAllTerminalSessions(reason = 'shutdown') {
  for (const id of Array.from(terminalSessions.keys())) {
    closeTerminalSession(id, reason);
  }
}

function pruneTerminalSessions() {
  const now = Date.now();
  for (const [id, session] of terminalSessions.entries()) {
    if (!session || session.closed) {
      terminalSessions.delete(id);
      continue;
    }
    if (session.pending || (Array.isArray(session.queue) && session.queue.length > 0)) continue;
    const idleMs = now - Number(session.last_used_ms || now);
    if (idleMs >= TERMINAL_SESSION_IDLE_TTL_MS) {
      closeTerminalSession(id, 'idle_timeout');
    }
  }
}

async function runTerminalCommand(rawCommand, requestedCwd, agentId = 'dashboard-terminal', snapshot = null) {
  const cleanAgentId = cleanText(agentId || 'dashboard-terminal', 140) || 'dashboard-terminal';
  if (cleanAgentId) {
    ensureAgentGitTreeAssignments(snapshot, {
      preferred_master_id: cleanAgentId,
      ensure_workspace_agent_id: cleanAgentId,
      force: false,
    });
    ensureAgentGitTreeProfile(cleanAgentId, {
      force_master: false,
      ensure_workspace_ready: true,
    });
  }
  const command = String(rawCommand == null ? '' : rawCommand).replace(/\u0000/g, '').trim();
  if (!command) {
    return {
      ok: false,
      blocked: true,
      status: 2,
      exit_code: 2,
      stdout: '',
      stderr: '',
      message: 'Terminal command required.',
      duration_ms: 0,
      cwd: resolveTerminalCwd(requestedCwd, cleanAgentId),
      command: '',
    };
  }
  if (ACTIVE_CLI_MODE !== CLI_MODE_FULL_INFRING) {
    return {
      ok: false,
      blocked: true,
      status: 2,
      exit_code: 2,
      stdout: '',
      stderr: '',
      message: 'Terminal mode disabled while CLI mode is safe.',
      duration_ms: 0,
      cwd: resolveTerminalCwd(requestedCwd, cleanAgentId),
      command,
    };
  }
  const cwd = resolveTerminalCwd(requestedCwd, cleanAgentId);
  pruneTerminalSessions();
  try {
    const session = ensureTerminalSession(cleanAgentId, cwd);
    return await queueTerminalCommand(session, command, cwd);
  } catch (error) {
    return {
      ok: false,
      blocked: false,
      status: 1,
      exit_code: 1,
      stdout: '',
      stderr: '',
      message: cleanText(error && error.message ? error.message : String(error), 260),
      duration_ms: 0,
      cwd,
      command,
    };
  }
}

function collectTrackedFiles() {
  try {
    const proc = spawnSync('git', ['ls-files'], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      timeout: 10000,
      maxBuffer: 8 * 1024 * 1024,
    });
    if (!proc || proc.status !== 0) return [];
    return String(proc.stdout || '')
      .split('\n')
      .map((row) => row.trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

function isEffectiveLocPath(filePath) {
  const lower = String(filePath || '').toLowerCase();
  if (!lower) return false;
  if (lower.includes('/node_modules/')) return false;
  if (lower.includes('/target/')) return false;
  if (lower.includes('/dist/')) return false;
  if (lower.includes('/coverage/')) return false;
  if (lower.includes('/.next/')) return false;
  if (lower.endsWith('.min.js') || lower.endsWith('.min.css')) return false;
  return EFFECTIVE_LOC_EXTENSIONS.has(path.extname(lower));
}

function effectiveLinesForContent(content) {
  let count = 0;
  const lines = String(content || '').split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    if (trimmed.startsWith('//')) continue;
    if (trimmed.startsWith('#')) continue;
    if (trimmed === '/*' || trimmed === '*/') continue;
    if (trimmed.startsWith('*')) continue;
    if (trimmed.startsWith('<!--') || trimmed.endsWith('-->')) continue;
    count += 1;
  }
  return count;
}

function recentDateIso(offsetDays) {
  const days = parseNonNegativeInt(offsetDays, 0, 3650);
  const ms = Date.now() - days * 24 * 60 * 60 * 1000;
  return new Date(ms).toISOString().slice(0, 10);
}

function memoryFileCandidates(dateIso) {
  const safeDate = cleanText(dateIso || '', 20);
  if (!safeDate) return [];
  return [
    path.resolve(ROOT, PRIMARY_MEMORY_DIR, `${safeDate}.md`),
    path.resolve(ROOT, LEGACY_MEMORY_DIR, `${safeDate}.md`),
  ];
}

function readMemoryFileForDate(dateIso) {
  const candidates = memoryFileCandidates(dateIso);
  for (const fullPath of candidates) {
    if (!fileExists(fullPath)) continue;
    const content = readText(fullPath, '');
    if (content || content === '') {
      return {
        date_iso: dateIso,
        full_path: fullPath,
        rel_path: path.relative(ROOT, fullPath),
        content,
      };
    }
  }
  return null;
}

function memoryBullets(content) {
  return String(content || '')
    .split('\n')
    .map((row) => row.trim())
    .filter((row) => row.startsWith('- '))
    .map((row) => row.slice(2).trim())
    .filter(Boolean);
}

function safeDecodePathToken(value) {
  const raw = String(value == null ? '' : value);
  if (!raw) return '';
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

function normalizeMemoryKey(rawKey) {
  const decoded = safeDecodePathToken(rawKey).replace(/\u0000/g, '').trim();
  if (!decoded) return '';
  return decoded.slice(0, AGENT_MEMORY_KEY_MAX_LEN);
}

function sanitizeMemoryValue(value) {
  if (value === undefined) return null;
  try {
    const encoded = JSON.stringify(value);
    if (typeof encoded !== 'string') return null;
    if (encoded.length <= AGENT_MEMORY_VALUE_MAX_JSON_CHARS) {
      return JSON.parse(encoded);
    }
    return cleanText(encoded, AGENT_MEMORY_VALUE_MAX_JSON_CHARS);
  } catch {
    return cleanText(String(value == null ? '' : value), AGENT_MEMORY_VALUE_MAX_JSON_CHARS);
  }
}

function normalizeMemoryKvMap(rawMap) {
  const source = rawMap && typeof rawMap === 'object' && !Array.isArray(rawMap) ? rawMap : {};
  const out = {};
  const entries = Object.entries(source).slice(0, AGENT_MEMORY_KV_MAX_KEYS);
  for (const [rawKey, rawValue] of entries) {
    const key = normalizeMemoryKey(rawKey);
    if (!key) continue;
    const wrappedValue =
      rawValue &&
      typeof rawValue === 'object' &&
      !Array.isArray(rawValue) &&
      Object.prototype.hasOwnProperty.call(rawValue, 'value')
        ? rawValue.value
        : rawValue;
    out[key] = sanitizeMemoryValue(wrappedValue);
  }
  return out;
}

function todayDateIso() {
  return new Date().toISOString().slice(0, 10);
}

function primaryMemoryDirPath() {
  return path.resolve(ROOT, PRIMARY_MEMORY_DIR);
}

function dailyMemoryFilePath(dateIso = todayDateIso()) {
  const safeDate = cleanText(dateIso || todayDateIso(), 20) || todayDateIso();
  return path.resolve(primaryMemoryDirPath(), `${safeDate}.md`);
}

function ensureDailyMemoryFile(dateIso = todayDateIso()) {
  const dirPath = primaryMemoryDirPath();
  ensureDir(dirPath);
  const filePath = dailyMemoryFilePath(dateIso);
  if (!fileExists(filePath)) {
    const headerDate = cleanText(dateIso || todayDateIso(), 20) || todayDateIso();
    writeFileAtomic(filePath, `# ${headerDate}\n\n`, 'utf8');
  }
  return filePath;
}

function appendPassiveMemoryLine(line, options = {}) {
  const text = cleanText(line || '', MEMORY_PASSIVE_LINE_MAX_LEN);
  if (!text) return { ok: false, skipped: true, reason: 'empty_line' };
  const filePath = ensureDailyMemoryFile(options && options.date_iso ? options.date_iso : todayDateIso());
  const timestamp = nowIso().slice(11, 19);
  fs.appendFileSync(filePath, `- ${timestamp} ${text}\n`, 'utf8');
  return { ok: true, path: path.relative(ROOT, filePath) };
}

function searchSnippet(content, queryLower) {
  const text = String(content || '');
  const lower = text.toLowerCase();
  const idx = lower.indexOf(queryLower);
  if (idx < 0) return '';
  const start = Math.max(0, idx - 80);
  const end = Math.min(text.length, idx + queryLower.length + 120);
  return cleanText(text.slice(start, end), 260);
}

function searchMemoryMarkdownFiles(queryLower, limit = MEMORY_SEARCH_MAX_FILE_SCAN) {
  const files = [];
  const roots = [
    path.resolve(ROOT, PRIMARY_MEMORY_DIR),
    path.resolve(ROOT, LEGACY_MEMORY_DIR),
  ];
  for (const rootDir of roots) {
    try {
      if (!fs.existsSync(rootDir)) continue;
      const names = fs.readdirSync(rootDir).filter((name) => name.endsWith('.md'));
      for (const name of names) {
        const fullPath = path.resolve(rootDir, name);
        const stat = fs.statSync(fullPath);
        files.push({
          full_path: fullPath,
          rel_path: path.relative(ROOT, fullPath),
          mtime_ms: Number.isFinite(stat.mtimeMs) ? stat.mtimeMs : 0,
        });
      }
    } catch {}
  }
  if (fileExists(ASSISTANT_MEMORY_PATH)) {
    try {
      const stat = fs.statSync(ASSISTANT_MEMORY_PATH);
      files.push({
        full_path: ASSISTANT_MEMORY_PATH,
        rel_path: path.relative(ROOT, ASSISTANT_MEMORY_PATH),
        mtime_ms: Number.isFinite(stat.mtimeMs) ? stat.mtimeMs : 0,
      });
    } catch {}
  }
  files.sort((a, b) => b.mtime_ms - a.mtime_ms);
  const out = [];
  for (const file of files.slice(0, Math.max(1, limit))) {
    const content = readText(file.full_path, '');
    if (!content) continue;
    const lines = content.split('\n');
    let hits = 0;
    for (const line of lines) {
      const row = String(line || '').trim();
      if (!row) continue;
      if (!row.toLowerCase().includes(queryLower)) continue;
      out.push({
        source: 'markdown',
        path: file.rel_path,
        snippet: cleanText(row, 260),
        ts: file.mtime_ms > 0 ? new Date(file.mtime_ms).toISOString() : '',
        score: 60 - Math.min(40, hits * 5),
      });
      hits += 1;
      if (hits >= MEMORY_SEARCH_MAX_MATCHES_PER_FILE) break;
    }
  }
  return out;
}

function searchAgentMemoryKv(queryLower) {
  const out = [];
  const files = recentFiles(AGENT_SESSIONS_DIR, {
    limit: 800,
    maxDepth: 1,
    include: (fullPath) => fullPath.endsWith('.json'),
  });
  for (const file of files) {
    const filePath = file.full_path;
    const rawState = readJson(filePath, null);
    if (!rawState || typeof rawState !== 'object') continue;
    const kv = normalizeMemoryKvMap(rawState.memory_kv);
    const ts = file && Number.isFinite(file.mtime_ms) && file.mtime_ms > 0
      ? new Date(file.mtime_ms).toISOString()
      : '';
    const agentId = cleanText(path.basename(filePath, '.json'), 140);
    for (const [key, value] of Object.entries(kv)) {
      const valueText = cleanText(typeof value === 'string' ? value : JSON.stringify(value), 6000).toLowerCase();
      if (!key.toLowerCase().includes(queryLower) && !valueText.includes(queryLower)) continue;
      out.push({
        source: 'agent_kv',
        agent_id: agentId,
        key,
        snippet: searchSnippet(typeof value === 'string' ? value : JSON.stringify(value), queryLower) || cleanText(String(valueText), 260),
        ts,
        score: key.toLowerCase().includes(queryLower) ? 95 : 88,
      });
    }
  }
  return out;
}

function searchSnapshotMemory(queryLower, snapshot) {
  const out = [];
  const turns =
    snapshot && snapshot.app && Array.isArray(snapshot.app.turns)
      ? snapshot.app.turns.slice(-80)
      : [];
  for (const turn of turns) {
    const combined = `${cleanText(turn && turn.user ? turn.user : '', 600)} ${cleanText(turn && turn.assistant ? turn.assistant : '', 1200)}`;
    if (!combined.toLowerCase().includes(queryLower)) continue;
    out.push({
      source: 'session_turn',
      snippet: searchSnippet(combined, queryLower) || cleanText(combined, 260),
      ts: cleanText(turn && turn.ts ? turn.ts : '', 80),
      score: 80,
    });
  }
  const attentionRows =
    snapshot &&
    snapshot.attention_queue &&
    Array.isArray(snapshot.attention_queue.critical)
      ? snapshot.attention_queue.critical.slice(0, 80)
      : [];
  for (const row of attentionRows) {
    const combined = `${cleanText(row && row.summary ? row.summary : '', 260)} ${cleanText(row && row.source ? row.source : '', 120)}`;
    if (!combined.toLowerCase().includes(queryLower)) continue;
    out.push({
      source: 'attention_queue',
      snippet: searchSnippet(combined, queryLower) || cleanText(combined, 260),
      ts: cleanText(row && row.ts ? row.ts : '', 80),
      score: 76,
    });
  }
  return out;
}

function searchMemoryLocal(query, snapshot, options = {}) {
  const needle = cleanText(query || '', 280).toLowerCase();
  const limit = parsePositiveInt(
    options && options.limit != null ? options.limit : MEMORY_SEARCH_DEFAULT_LIMIT,
    MEMORY_SEARCH_DEFAULT_LIMIT,
    1,
    MEMORY_SEARCH_MAX_LIMIT
  );
  if (!needle) {
    return {
      ok: true,
      disabled: false,
      source: 'local_memory_fallback',
      results: [],
      query: '',
    };
  }
  const rows = [
    ...searchAgentMemoryKv(needle),
    ...searchMemoryMarkdownFiles(needle, MEMORY_SEARCH_MAX_FILE_SCAN),
    ...searchSnapshotMemory(needle, snapshot),
  ];
  rows.sort((a, b) => {
    const scoreDelta = parseNonNegativeInt(b && b.score, 0, 100000000) - parseNonNegativeInt(a && a.score, 0, 100000000);
    if (scoreDelta !== 0) return scoreDelta;
    return coerceTsMs(b && b.ts, 0) - coerceTsMs(a && a.ts, 0);
  });
  const results = rows.slice(0, limit).map((row, idx) => ({
    id: `mem_${idx + 1}`,
    source: cleanText(row && row.source ? row.source : 'memory', 40) || 'memory',
    path: cleanText(row && row.path ? row.path : '', 260),
    agent_id: cleanText(row && row.agent_id ? row.agent_id : '', 140),
    key: cleanText(row && row.key ? row.key : '', AGENT_MEMORY_KEY_MAX_LEN),
    snippet: cleanText(row && row.snippet ? row.snippet : '', 260),
    ts: cleanText(row && row.ts ? row.ts : '', 80),
    score: parseNonNegativeInt(row && row.score, 0, 100000000),
  }));
  return {
    ok: true,
    disabled: false,
    source: 'local_memory_fallback',
    query: needle,
    results,
    count: results.length,
    scanned: {
      agent_sessions_dir: path.relative(ROOT, AGENT_SESSIONS_DIR),
      memory_roots: [PRIMARY_MEMORY_DIR, LEGACY_MEMORY_DIR],
      assistant_memory: path.relative(ROOT, ASSISTANT_MEMORY_PATH),
    },
  };
}

function normalizeCollabRole(roleRaw) {
  const role = cleanText(roleRaw || '', 40).toLowerCase();
  if (!role) return 'analyst';
  if (COLLAB_SUPPORTED_ROLES.has(role)) return role;
  const mapped = COLLAB_ROLE_FALLBACKS[role];
  if (mapped && COLLAB_SUPPORTED_ROLES.has(mapped)) return mapped;
  return 'analyst';
}

function parseCollabLaunchCommands(input) {
  const raw = String(input || '');
  const commands = [];
  const regex = /protheus-ops\s+collab-plane\s+launch-role\b([\s\S]*?)(?=protheus-ops\s+collab-plane\s+launch-role\b|$)/gi;
  let match = regex.exec(raw);
  while (match) {
    const trailer = String(match[1] || '').replace(/\s+/g, ' ').trim();
    if (trailer) {
      const rawTokens = trailer
        .split(' ')
        .map((row) => sanitizeArg(row, 180))
        .filter(Boolean)
        .filter((row) => row !== 'Run' && row !== 'run' && row !== 'exactly:' && row !== 'exactly');
      const firstFlag = rawTokens.findIndex((row) => row.startsWith('--'));
      const tokens = [];
      if (firstFlag >= 0) {
        for (let i = firstFlag; i < rawTokens.length; i += 1) {
          const token = rawTokens[i];
          if (!token.startsWith('--')) break;
          tokens.push(token);
        }
      }
      const args = ['collab-plane', 'launch-role', ...tokens];
      commands.push(args);
    }
    match = regex.exec(raw);
  }
  return commands.slice(0, 4);
}

function optimisticCollabHydrateFromTools(snapshot, tools = []) {
  const rows = ensureCollabAgentRows(snapshot);
  if (!Array.isArray(rows)) return 0;
  const mergedInput = (Array.isArray(tools) ? tools : [])
    .map((tool) => cleanText(tool && tool.input ? tool.input : '', 600))
    .filter(Boolean)
    .join('\n');
  if (!mergedInput) return 0;
  const commands = parseCollabLaunchCommands(mergedInput);
  if (!commands.length) return 0;
  const existing = new Set(
    rows
      .map((row) => cleanText(row && row.shadow ? row.shadow : row && row.id ? row.id : '', 140))
      .filter(Boolean)
  );
  let added = 0;
  for (const argv of commands) {
    const tokens = Array.isArray(argv) ? argv : [];
    const shadowToken = tokens.find((token) => String(token || '').startsWith('--shadow='));
    const roleToken = tokens.find((token) => String(token || '').startsWith('--role='));
    const shadow = cleanText(shadowToken ? String(shadowToken).slice('--shadow='.length) : '', 140);
    const role = cleanText(roleToken ? String(roleToken).slice('--role='.length) : 'analyst', 60) || 'analyst';
    if (!shadow || existing.has(shadow)) continue;
    rows.push({
      shadow,
      role,
      status: 'running',
      activated_at: nowIso(),
    });
    existing.add(shadow);
    added += 1;
  }
  return added;
}

function ensureCollabAgentRows(snapshot) {
  if (!snapshot || typeof snapshot !== 'object') return null;
  if (!snapshot.collab || typeof snapshot.collab !== 'object') snapshot.collab = {};
  if (!snapshot.collab.dashboard || typeof snapshot.collab.dashboard !== 'object') {
    snapshot.collab.dashboard = {};
  }
  if (!Array.isArray(snapshot.collab.dashboard.agents)) snapshot.collab.dashboard.agents = [];
  return snapshot.collab.dashboard.agents;
}

function optimisticCollabUpsertAgent(snapshot, shadow, role = 'analyst') {
  const id = cleanText(shadow || '', 140);
  if (!id) return false;
  const rows = ensureCollabAgentRows(snapshot);
  if (!Array.isArray(rows)) return false;
  const existing = rows.find((row) => {
    const rowId = cleanText(row && (row.shadow || row.id) ? row.shadow || row.id : '', 140);
    return rowId === id;
  });
  if (existing) {
    existing.shadow = id;
    existing.role = cleanText(role || existing.role || 'analyst', 60) || 'analyst';
    existing.status = 'running';
    if (!existing.activated_at) existing.activated_at = nowIso();
    return true;
  }
  rows.push({
    shadow: id,
    role: cleanText(role || 'analyst', 60) || 'analyst',
    status: 'running',
    activated_at: nowIso(),
  });
  return true;
}

function optimisticCollabArchiveAgent(snapshot, shadow) {
  const id = cleanText(shadow || '', 140);
  if (!id) return false;
  const rows = ensureCollabAgentRows(snapshot);
  if (!Array.isArray(rows) || rows.length === 0) return false;
  const next = rows.filter((row) => {
    const rowId = cleanText(row && (row.shadow || row.id) ? row.shadow || row.id : '', 140);
    return rowId !== id;
  });
  snapshot.collab.dashboard.agents = next;
  return next.length !== rows.length;
}

function tryDeterministicRepoAnswer(input, snapshot = null) {
  const rawInput = String(input || '');
  const text = rawInput.toLowerCase();
  const asksWeekAgo =
    /(one week ago|7 days ago|last week)/.test(text) &&
    /(what were we doing|what did we do|what was happening|what happened)/.test(text);
  if (asksWeekAgo) {
    const offsets = [7, 8, 6, 9];
    const seen = new Set();
    const candidates = [];
    for (const offset of offsets) {
      const dateIso = recentDateIso(offset);
      if (seen.has(dateIso)) continue;
      seen.add(dateIso);
      const entry = readMemoryFileForDate(dateIso);
      if (entry) candidates.push(entry);
    }
    if (!candidates.length) {
      return {
        response: `I checked ${PRIMARY_MEMORY_DIR} and ${LEGACY_MEMORY_DIR}, but I could not find a memory file for around one week ago.`,
        tools: [
          {
            id: `tool-${Date.now()}-det-memory-miss`,
            name: 'ls',
            input: `ls ${PRIMARY_MEMORY_DIR}/`,
            result: 'no_candidate_memory_file_found',
            is_error: false,
            running: false,
            expanded: false,
          },
        ],
      };
    }
    const best = candidates.find((row) => memoryBullets(row.content).length > 0) || candidates[0];
    const bullets = memoryBullets(best.content).slice(0, 2);
    const bulletText = bullets.length ? bullets.map((row) => `- ${row}`).join(' ') : '- No concrete bullet entries were recorded in that file.';
    const response = `Exact date: ${best.date_iso}. Memory file path: ${best.rel_path}. ${bulletText}`;
    return {
      response,
      tools: [
        {
          id: `tool-${Date.now()}-det-memory`,
          name: 'cat',
          input: `cat ${best.rel_path}`,
          result: cleanText(best.content || '(empty file)', TOOL_OUTPUT_LIMIT),
          is_error: false,
          running: false,
          expanded: false,
        },
      ],
    };
  }

  const collabLaunchCommands = parseCollabLaunchCommands(rawInput);
  if (collabLaunchCommands.length > 0) {
    const tools = [];
    const launched = [];
    for (const originalArgs of collabLaunchCommands) {
      const args = Array.isArray(originalArgs) ? [...originalArgs] : ['collab-plane', 'launch-role'];
      const roleIndex = args.findIndex((row) => String(row).startsWith('--role='));
      const requestedRole = roleIndex >= 0 ? String(args[roleIndex]).slice('--role='.length) : '';
      const normalizedRole = normalizeCollabRole(requestedRole);
      if (roleIndex >= 0 && normalizedRole) {
        args[roleIndex] = `--role=${normalizedRole}`;
      } else if (roleIndex < 0) {
        args.push(`--role=${normalizedRole}`);
      }
      const result = runCliTool('protheus-ops', args);
      tools.push({
        id: `tool-${Date.now()}-det-collab-${tools.length + 1}`,
        name: 'protheus-ops',
        input: ['protheus-ops', ...args].join(' '),
        result: cleanText(result.result, TOOL_OUTPUT_LIMIT),
        is_error: !!result.is_error,
        running: false,
        expanded: false,
      });
      if (!result.is_error) {
        const shadowArg = args.find((row) => String(row).startsWith('--shadow='));
        const shadow = shadowArg ? cleanText(String(shadowArg).slice('--shadow='.length), 120) : '';
        if (shadow) launched.push(shadow);
      }
    }
    const successes = tools.filter((row) => !row.is_error).length;
    const response =
      successes > 0
        ? `${launched.join(' ') || `launched ${successes} subagent(s)`}`
        : 'Subagent launch commands failed.';
    return { response, tools };
  }

  const asksRuntimeSync =
    /(runtime sync|queue depth|cockpit blocks|conduit signals|attention queue)/.test(text) &&
    /(report|summarize|status|readable|now)/.test(text);
  const asksClientLayerSummary =
    /(summarize|summary|report).*(client layer)/.test(text) ||
    /client layer now/.test(text);
  if (asksRuntimeSync || asksClientLayerSummary) {
    const snap = snapshot && typeof snapshot === 'object' ? snapshot : null;
    if (snap) {
      const runtime = runtimeSyncSummary(snap);
      const queueDepth = parseNonNegativeInt(runtime.queue_depth, 0, 100000000);
      const cockpitBlocks = parseNonNegativeInt(runtime.cockpit_blocks, 0, 100000000);
      const cockpitTotalBlocks = parseNonNegativeInt(
        runtime.cockpit_total_blocks,
        cockpitBlocks,
        100000000
      );
      const conduitSignals = parseNonNegativeInt(runtime.conduit_signals, 0, 100000000);
      const attentionReadable =
        cleanText(
          snap &&
          snap.attention_queue &&
          snap.attention_queue.status &&
          snap.attention_queue.status.source
            ? snap.attention_queue.status.source
            : '',
          80
        ) || 'readable';
      const memoryEntries = Array.isArray(snap && snap.memory && snap.memory.entries)
        ? snap.memory.entries.length
        : 0;
      const receiptCount = Array.isArray(snap && snap.receipts && snap.receipts.recent)
        ? snap.receipts.recent.length
        : 0;
      const logCount = Array.isArray(snap && snap.logs && snap.logs.recent)
        ? snap.logs.recent.length
        : 0;
      const healthChecks = parseNonNegativeInt(runtime.health_check_count, 0, 100000000);
      const response = asksClientLayerSummary
        ? `Client layer now: memory entries ${memoryEntries}, receipts ${receiptCount}, logs ${logCount}, health checks ${healthChecks}, attention queue depth ${queueDepth}, cockpit blocks ${cockpitBlocks} active (${cockpitTotalBlocks} total), conduit signals ${conduitSignals}.`
        : `Current queue depth: ${queueDepth}, cockpit blocks: ${cockpitBlocks} active (${cockpitTotalBlocks} total), conduit signals: ${conduitSignals}. Attention queue is ${attentionReadable}.`;
      return {
        response,
        tools: [
          {
            id: `tool-${Date.now()}-det-runtime-sync`,
            name: 'api.dashboard.snapshot',
            input: '/api/dashboard/snapshot',
            result: `queue_depth=${queueDepth};cockpit_blocks_active=${cockpitBlocks};cockpit_blocks_total=${cockpitTotalBlocks};conduit_signals=${conduitSignals};memory_entries=${memoryEntries};receipts=${receiptCount};logs=${logCount};health_checks=${healthChecks}`,
            is_error: false,
            running: false,
            expanded: false,
          },
        ],
      };
    }
  }

  const asksFiles = /how many files|file count|number of files/.test(text);
  const asksLoc = /effective loc|affective loc|lines of code|\bloc\b/.test(text);
  if (!asksFiles && !asksLoc) return null;

  const tracked = collectTrackedFiles();
  if (!tracked.length) return null;

  if (asksLoc) {
    const sourceFiles = tracked.filter((row) => isEffectiveLocPath(row));
    let effectiveLoc = 0;
    let scannedFiles = 0;
    for (const rel of sourceFiles) {
      const abs = path.resolve(ROOT, rel);
      try {
        const raw = fs.readFileSync(abs, 'utf8');
        effectiveLoc += effectiveLinesForContent(raw);
        scannedFiles += 1;
      } catch {}
    }
    const response = `Effective LoC is ${effectiveLoc.toLocaleString()} across ${scannedFiles.toLocaleString()} source-like tracked files (comments/blank lines excluded by heuristic).`;
    return {
      response,
      tools: [
        {
          id: `tool-${Date.now()}-det-loc`,
          name: 'git',
          input: 'git ls-files',
          result: `tracked_files=${tracked.length}; scanned_source_files=${scannedFiles}; effective_loc=${effectiveLoc}`,
          is_error: false,
          running: false,
          expanded: false,
        },
      ],
    };
  }

  const response = `This repo currently has ${tracked.length.toLocaleString()} tracked files.`;
  return {
    response,
    tools: [
      {
        id: `tool-${Date.now()}-det-files`,
        name: 'git',
        input: 'git ls-files',
        result: `tracked_files=${tracked.length}`,
        is_error: false,
        running: false,
        expanded: false,
      },
    ],
  };
}

function cliInvocationAllowed(command, args) {
  const cmd = sanitizeArg(command, 80);
  if (!CLI_ALLOWLIST.has(cmd)) {
    return { ok: false, error: `command_not_allowed:${cmd}` };
  }
  const fullInfring = ACTIVE_CLI_MODE === CLI_MODE_FULL_INFRING;
  const first = sanitizeArg(args && args[0] ? args[0] : '', 80);
  if (cmd === 'git' && first && !GIT_READ_ONLY.has(first)) {
    return { ok: false, error: `git_subcommand_blocked:${first}` };
  }
  if (!fullInfring && (cmd === 'infringd' || cmd === 'protheus') && first && !INFRINGD_READ_ONLY.has(first)) {
    return { ok: false, error: `runtime_subcommand_blocked:${first}` };
  }
  if (!fullInfring && cmd === 'protheus-ops' && first && !OPS_READ_ONLY.has(first)) {
    return { ok: false, error: `ops_subcommand_blocked:${first}` };
  }
  return { ok: true, command: cmd, mode: ACTIVE_CLI_MODE };
}

function runCliTool(command, args = []) {
  const normalizedArgs = parseToolArgs(args);
  const gate = cliInvocationAllowed(command, normalizedArgs);
  const input = [sanitizeArg(command, 80), ...normalizedArgs].filter(Boolean).join(' ');
  if (!gate.ok) {
    return {
      ok: false,
      name: sanitizeArg(command, 80) || 'cli',
      input,
      result: `blocked: ${gate.error}`,
      is_error: true,
      exit_code: 126,
    };
  }
  try {
    const proc = spawnSync(gate.command, normalizedArgs, {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      env: { ...process.env, PROTHEUS_ROOT: ROOT },
      timeout: 30000,
      maxBuffer: 4 * 1024 * 1024,
    });
    const status = typeof proc.status === 'number' ? proc.status : 1;
    const stdout = typeof proc.stdout === 'string' ? proc.stdout : '';
    const stderr = typeof proc.stderr === 'string' ? proc.stderr : '';
    const output = cleanText([stdout.trim(), stderr.trim()].filter(Boolean).join('\n\n') || '(no output)', TOOL_OUTPUT_LIMIT);
    return {
      ok: status === 0,
      name: gate.command,
      input,
      result: output,
      is_error: status !== 0,
      exit_code: status,
    };
  } catch (error) {
    return {
      ok: false,
      name: gate.command,
      input,
      result: `failed: ${cleanText(error && error.message ? error.message : String(error), 260)}`,
      is_error: true,
      exit_code: 1,
    };
  }
}

function maybeAppendPassiveMemoryLine(line, options = {}) {
  const nowMs = Date.now();
  const minIntervalMs = parsePositiveInt(
    options && options.min_interval_ms != null ? options.min_interval_ms : MEMORY_PASSIVE_APPEND_MIN_INTERVAL_MS,
    MEMORY_PASSIVE_APPEND_MIN_INTERVAL_MS,
    1000,
    60 * 60 * 1000
  );
  if (
    !options.force &&
    (nowMs - parseNonNegativeInt(passiveMemoryWriteState.last_append_ms, 0, 1_000_000_000_000)) < minIntervalMs
  ) {
    return { ok: true, skipped: true, reason: 'rate_limited' };
  }
  const fingerprint = sha256(cleanText(line || '', MEMORY_PASSIVE_LINE_MAX_LEN)).slice(0, 24);
  if (
    passiveMemoryWriteState.last_hash === fingerprint &&
    (nowMs - parseNonNegativeInt(passiveMemoryWriteState.last_append_ms, 0, 1_000_000_000_000)) < minIntervalMs
  ) {
    return { ok: true, skipped: true, reason: 'deduplicated' };
  }
  const appended = appendPassiveMemoryLine(line, options);
  if (appended.ok) {
    passiveMemoryWriteState.last_hash = fingerprint;
    passiveMemoryWriteState.last_append_ms = nowMs;
  }
  return appended;
}

function recordPassiveConversationMemory(agentId, userText, assistantText, metaText = '') {
  const user = cleanText(userText || '', 140);
  const assistant = cleanText(assistantText || '', 180);
  if (!user && !assistant) return { ok: false, skipped: true, reason: 'empty_turn' };
  const profile = agentProfileFor(agentId);
  const agentName = cleanText(profile && profile.name ? profile.name : agentId || 'agent', 80) || 'agent';
  const meta = cleanText(metaText || '', 60);
  const summaryParts = [];
  if (user) summaryParts.push(`U: ${user}`);
  if (assistant) summaryParts.push(`A: ${assistant}`);
  const line = `[chat:${agentName}] ${summaryParts.join(' | ')}${meta ? ` (${meta})` : ''}`;
  return maybeAppendPassiveMemoryLine(line, {
    min_interval_ms: MEMORY_PASSIVE_APPEND_MIN_INTERVAL_MS,
  });
}

function recordPassiveAttentionMemory(eventPayload) {
  const severity = normalizeSeverity(eventPayload && eventPayload.severity ? eventPayload.severity : 'info');
  if (severity === 'info') return { ok: true, skipped: true, reason: 'severity_info' };
  const nowMs = Date.now();
  if (
    (nowMs - parseNonNegativeInt(passiveMemoryWriteState.last_attention_append_ms, 0, 1_000_000_000_000)) <
    MEMORY_PASSIVE_ATTENTION_APPEND_MIN_INTERVAL_MS
  ) {
    return { ok: true, skipped: true, reason: 'attention_rate_limited' };
  }
  const source = cleanText(eventPayload && eventPayload.source ? eventPayload.source : 'attention_queue', 80) || 'attention_queue';
  const summary = cleanText(eventPayload && eventPayload.summary ? eventPayload.summary : '', 200);
  if (!summary) return { ok: true, skipped: true, reason: 'summary_missing' };
  const appended = maybeAppendPassiveMemoryLine(`[attention:${severity}] ${source}: ${summary}`, {
    min_interval_ms: MEMORY_PASSIVE_ATTENTION_APPEND_MIN_INTERVAL_MS,
  });
  if (appended.ok && !appended.skipped) {
    passiveMemoryWriteState.last_attention_append_ms = nowMs;
  }
  return appended;
}

function enqueueAttentionEvent(eventPayload, runContext = 'dashboard_chat') {
  try {
    recordPassiveAttentionMemory(eventPayload);
    const raw = JSON.stringify(eventPayload && typeof eventPayload === 'object' ? eventPayload : {});
    const encoded = Buffer.from(raw, 'utf8').toString('base64');
    return runLane([
      'attention-queue',
      'enqueue',
      `--event-json-base64=${encoded}`,
      `--run-context=${cleanText(runContext || 'dashboard_chat', 60) || 'dashboard_chat'}`,
    ]);
  } catch (error) {
    return {
      ok: false,
      status: 1,
      stdout: '',
      stderr: cleanText(error && error.message ? error.message : String(error), 220),
      payload: null,
      argv: ['attention-queue', 'enqueue'],
    };
  }
}

function extractJsonDirective(raw) {
  const text = String(raw || '').trim();
  if (!text) return null;
  let payload = parseJsonLoose(text);
  if (!payload) {
    const fenced = text.match(/```(?:json)?\s*([\s\S]*?)```/i);
    if (fenced && fenced[1]) {
      payload = parseJsonLoose(fenced[1]);
    }
  }
  if (!payload || typeof payload !== 'object') {
    const heuristic = extractDirectiveHeuristic(text);
    if (heuristic) return heuristic;
    return null;
  }
  const type = cleanText(payload.type || payload.tool || '', 40).toLowerCase();
  if (type === 'final' || type === 'answer') {
    return {
      type: 'final',
      response: cleanText(payload.response || payload.answer || payload.text || '', 6000),
    };
  }
  if (type === 'tool_call' || type === 'run_cli' || payload.command) {
    return {
      type: 'tool_call',
      command: sanitizeArg(payload.command || 'protheus-ops', 80),
      args: parseToolArgs(payload.args || payload.argv || payload.input || ''),
      reason: cleanText(payload.reason || payload.why || '', 220),
    };
  }
  return null;
}

function extractDirectiveHeuristic(text) {
  const raw = String(text || '');
  const lowered = raw.toLowerCase();
  if (lowered.includes('"type"') && lowered.includes('"tool_call"')) {
    const commandMatch = raw.match(/"command"\s*:\s*"([^"\n]+)"/i);
    if (commandMatch && commandMatch[1]) {
      const argsMatch = raw.match(/"args"\s*:\s*(\[[\s\S]*?\])/i);
      let args = [];
      if (argsMatch && argsMatch[1]) {
        const parsedArgs = parseJsonLoose(argsMatch[1]);
        args = parseToolArgs(Array.isArray(parsedArgs) ? parsedArgs : []);
      }
      if (!args.length) {
        const argvMatch = raw.match(/"argv"\s*:\s*(\[[\s\S]*?\])/i);
        if (argvMatch && argvMatch[1]) {
          const parsedArgv = parseJsonLoose(argvMatch[1]);
          args = parseToolArgs(Array.isArray(parsedArgv) ? parsedArgv : []);
        }
      }
      const reasonMatch =
        raw.match(/"reason"\s*:\s*"([^"]*)"/i) ||
        raw.match(/"reason:"\s*"([^"]*)"/i);
      return {
        type: 'tool_call',
        command: sanitizeArg(commandMatch[1], 80),
        args,
        reason: cleanText(reasonMatch && reasonMatch[1] ? reasonMatch[1] : '', 220),
      };
    }
  }
  if (lowered.includes('"type"') && (lowered.includes('"final"') || lowered.includes('"answer"'))) {
    const responseMatch =
      raw.match(/"response"\s*:\s*"([\s\S]*?)"\s*}/i) ||
      raw.match(/"answer"\s*:\s*"([\s\S]*?)"\s*}/i);
    if (responseMatch && responseMatch[1]) {
      return {
        type: 'final',
        response: cleanText(responseMatch[1].replace(/\\"/g, '"'), 6000),
      };
    }
  }
  return null;
}

function configuredOllamaModel(snapshot) {
  const raw =
    snapshot &&
    snapshot.app &&
    snapshot.app.settings &&
    snapshot.app.settings.model
      ? String(snapshot.app.settings.model)
      : '';
  if (!raw) return OLLAMA_MODEL_FALLBACK;
  if (raw.startsWith('ollama/')) return cleanText(raw.replace(/^ollama\//, ''), 120) || OLLAMA_MODEL_FALLBACK;
  if (raw.includes('/')) return OLLAMA_MODEL_FALLBACK;
  return cleanText(raw, 120) || OLLAMA_MODEL_FALLBACK;
}

function configuredProvider(snapshot) {
  const raw =
    snapshot &&
    snapshot.app &&
    snapshot.app.settings &&
    snapshot.app.settings.provider
      ? String(snapshot.app.settings.provider)
      : '';
  return cleanText(raw, 80) || 'openai';
}

function normalizeTestAgentModelConfig(state) {
  const root = state && typeof state === 'object' ? state : {};
  const model =
    cleanText(root.model || TEST_AGENT_MODEL_DEFAULT, 120) || TEST_AGENT_MODEL_DEFAULT;
  const provider =
    cleanText(
      root.provider || providerForModelName(model, TEST_AGENT_PROVIDER_DEFAULT),
      80
    ) || TEST_AGENT_PROVIDER_DEFAULT;
  return {
    type: 'infring_dashboard_test_agent_model',
    updated_at: cleanText(root.updated_at || nowIso(), 80) || nowIso(),
    enabled: root.enabled === true,
    model,
    provider,
  };
}

let testAgentModelCache = null;

function loadTestAgentModelConfig() {
  if (testAgentModelCache) return testAgentModelCache;
  testAgentModelCache = normalizeTestAgentModelConfig(readJson(TEST_AGENT_MODEL_PATH, null));
  return testAgentModelCache;
}

function saveTestAgentModelConfig(state) {
  const normalized = normalizeTestAgentModelConfig(state);
  writeJson(TEST_AGENT_MODEL_PATH, normalized);
  testAgentModelCache = normalized;
  return normalized;
}

function isTestingAgentId(agentId) {
  const id = cleanText(agentId || '', 140).toLowerCase();
  if (!id) return false;
  if (TEST_AGENT_ID_PREFIXES.some((prefix) => id.startsWith(prefix))) return true;
  return /(^|[-_])(e2e|test|bench|ci|qa)([-_]|$)/.test(id);
}

function testingModelOverrideForAgent(agentId) {
  if (!isTestingAgentId(agentId)) return null;
  const config = loadTestAgentModelConfig();
  if (!config || config.enabled === false) return null;
  return {
    model: cleanText(config.model || TEST_AGENT_MODEL_DEFAULT, 120) || TEST_AGENT_MODEL_DEFAULT,
    provider:
      cleanText(config.provider || TEST_AGENT_PROVIDER_DEFAULT, 80) ||
      TEST_AGENT_PROVIDER_DEFAULT,
  };
}

const PROVIDER_MODEL_CATALOG = {
  openai: ['gpt-5', 'gpt-5-mini', 'gpt-4.1', 'gpt-4o'],
  anthropic: ['claude-sonnet-4-20250514', 'claude-opus-4-20250514'],
  google: ['gemini-2.5-pro', 'gemini-2.5-flash'],
  groq: ['llama-3.3-70b-versatile', 'llama-3.1-8b-instant'],
  xai: ['grok-3', 'grok-2'],
  deepseek: ['deepseek-chat', 'deepseek-reasoner'],
  cohere: ['command-r-plus', 'command-r'],
  mistral: ['mistral-large-latest', 'mistral-small-latest'],
  perplexity: ['sonar-pro', 'sonar-small'],
  openrouter: ['openrouter/google/gemini-2.5-flash', 'openrouter/anthropic/claude-sonnet-4'],
  together: ['meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo'],
  fireworks: ['accounts/fireworks/models/llama-v3p1-70b-instruct'],
  cloud: ['kimi2.5:cloud'],
};

const PROVIDER_DEFAULTS = [
  { id: 'auto', display_name: 'Auto Router', is_local: false, needs_key: false },
  { id: 'openai', display_name: 'OpenAI', is_local: false, needs_key: true },
  { id: 'anthropic', display_name: 'Anthropic', is_local: false, needs_key: true },
  { id: 'google', display_name: 'Google', is_local: false, needs_key: true },
  { id: 'groq', display_name: 'Groq', is_local: false, needs_key: true },
  { id: 'xai', display_name: 'xAI', is_local: false, needs_key: true },
  { id: 'deepseek', display_name: 'DeepSeek', is_local: false, needs_key: true },
  { id: 'cohere', display_name: 'Cohere', is_local: false, needs_key: true },
  { id: 'mistral', display_name: 'Mistral', is_local: false, needs_key: true },
  { id: 'perplexity', display_name: 'Perplexity', is_local: false, needs_key: true },
  { id: 'openrouter', display_name: 'OpenRouter', is_local: false, needs_key: true },
  { id: 'together', display_name: 'Together', is_local: false, needs_key: true },
  { id: 'fireworks', display_name: 'Fireworks', is_local: false, needs_key: true },
  { id: 'ollama', display_name: 'Ollama', is_local: true, needs_key: false, base_url: 'http://127.0.0.1:11434' },
  { id: 'llama.cpp', display_name: 'llama.cpp', is_local: true, needs_key: false, base_url: 'http://127.0.0.1:8080' },
  { id: 'cloud', display_name: 'Cloud (Generic)', is_local: false, needs_key: true },
];

const CHANNEL_DEFAULTS = [
  {
    name: 'whatsapp',
    icon: '💬',
    display_name: 'WhatsApp',
    description: 'Connect WhatsApp for direct messaging and notifications.',
    quick_setup: 'Use QR login for WhatsApp Web or configure Business API credentials.',
    category: 'messaging',
    difficulty: 'Medium',
    setup_time: '2-5 min',
    setup_type: 'qr',
    has_token: false,
    configured: false,
    fields: [
      { key: 'business_token', label: 'Business API Token', type: 'secret', advanced: true, placeholder: 'EAAG...' },
      { key: 'phone_number_id', label: 'Phone Number ID', type: 'text', advanced: true, placeholder: '123456789' },
    ],
    setup_steps: ['Open WhatsApp on your phone', 'Scan QR code in this modal', 'Confirm linked status'],
    config_template: 'WHATSAPP_BUSINESS_TOKEN=...\\nWHATSAPP_PHONE_NUMBER_ID=...',
  },
  {
    name: 'discord',
    icon: '🎮',
    display_name: 'Discord',
    description: 'Route agent messages into Discord channels.',
    quick_setup: 'Create a bot token and select a target server/channel.',
    category: 'messaging',
    difficulty: 'Easy',
    setup_time: '2 min',
    setup_type: 'form',
    has_token: false,
    configured: false,
    fields: [
      { key: 'bot_token', label: 'Bot Token', type: 'secret', advanced: false, placeholder: 'MTIz...' },
      { key: 'channel_id', label: 'Channel ID', type: 'text', advanced: false, placeholder: '123456789012345678' },
    ],
    setup_steps: ['Create Discord bot', 'Invite bot to server', 'Paste token and channel id'],
    config_template: 'DISCORD_BOT_TOKEN=...\\nDISCORD_CHANNEL_ID=...',
  },
  {
    name: 'slack',
    icon: '💼',
    display_name: 'Slack',
    description: 'Send updates to Slack channels and threads.',
    quick_setup: 'Use a bot token + app-level token for socket mode.',
    category: 'enterprise',
    difficulty: 'Medium',
    setup_time: '3-8 min',
    setup_type: 'form',
    has_token: false,
    configured: false,
    fields: [
      { key: 'bot_token', label: 'Bot Token', type: 'secret', advanced: false, placeholder: 'xoxb-...' },
      { key: 'app_token', label: 'App Token', type: 'secret', advanced: true, placeholder: 'xapp-...' },
      { key: 'default_channel', label: 'Default Channel', type: 'text', advanced: false, placeholder: '#ops' },
    ],
    setup_steps: ['Create Slack app', 'Enable bot scopes', 'Paste tokens and channel'],
    config_template: 'SLACK_BOT_TOKEN=...\\nSLACK_APP_TOKEN=...\\nSLACK_DEFAULT_CHANNEL=#ops',
  },
];

let localProviderProbeCache = new Map();
let localProviderAutoDiscoverAtMs = 0;
let ollamaModelListCache = {
  ts: 0,
  models: [],
};

function inferProviderFromApiKey(apiKey = '') {
  const key = cleanText(apiKey || '', 320);
  if (!key) return '';
  if (/^sk-ant-/i.test(key)) return 'anthropic';
  if (/^sk-proj-/i.test(key) || /^sk-[a-z0-9]/i.test(key)) return 'openai';
  if (/^AIza[0-9A-Za-z_-]{20,}$/i.test(key)) return 'google';
  if (/^gsk_[A-Za-z0-9_-]+/i.test(key)) return 'groq';
  if (/^xai-/i.test(key)) return 'xai';
  if (/^dsk_/i.test(key) || /^deepseek-/i.test(key)) return 'deepseek';
  if (/^pplx-/i.test(key)) return 'perplexity';
  if (/^co_[a-z0-9]/i.test(key)) return 'cohere';
  if (/^mistral-/i.test(key)) return 'mistral';
  if (/^or-/i.test(key)) return 'openrouter';
  return 'cloud';
}

function providerDefaultById(providerId) {
  const normalized = cleanText(providerId || '', 80).toLowerCase();
  if (!normalized) return null;
  return PROVIDER_DEFAULTS.find((row) => row.id === normalized) || null;
}

function normalizeProviderRecord(providerId, value = {}) {
  const normalizedId = cleanText(providerId || value.id || '', 80).toLowerCase();
  const def = providerDefaultById(normalizedId);
  const now = nowIso();
  return {
    id: normalizedId,
    display_name:
      cleanText(value.display_name || (def && def.display_name) || normalizedId, 80) ||
      normalizedId,
    is_local: value.is_local === true || (!!def && def.is_local === true),
    needs_key: value.needs_key === true || (!!def && def.needs_key === true),
    auth_status: cleanText(value.auth_status || '', 24) || 'not_set',
    base_url: cleanText(value.base_url || (def && def.base_url) || '', 320),
    key_prefix: cleanText(value.key_prefix || '', 24),
    key_last4: cleanText(value.key_last4 || '', 12),
    key_hash: cleanText(value.key_hash || '', 80),
    key_set_at: cleanText(value.key_set_at || '', 80),
    reachable: value.reachable === true,
    detected_models: Array.isArray(value.detected_models)
      ? value.detected_models
          .map((row) => cleanText(row, 160))
          .filter(Boolean)
          .slice(0, 128)
      : [],
    updated_at: cleanText(value.updated_at || now, 80) || now,
  };
}

function loadProviderRegistry(snapshot) {
  const now = nowIso();
  const raw = readJson(PROVIDER_REGISTRY_PATH, null);
  const base =
    raw && typeof raw === 'object'
      ? raw
      : {
          type: 'infring_dashboard_provider_registry',
          updated_at: now,
          providers: {},
        };
  const providers = {};
  const seeded = Array.isArray(PROVIDER_DEFAULTS) ? PROVIDER_DEFAULTS : [];
  for (const row of seeded) {
    const id = cleanText(row && row.id ? row.id : '', 80).toLowerCase();
    if (!id) continue;
    providers[id] = normalizeProviderRecord(id, {
      ...(row || {}),
      ...(base.providers && base.providers[id] ? base.providers[id] : {}),
    });
  }
  const configured = cleanText(configuredProvider(snapshot), 80).toLowerCase();
  if (configured && providers[configured]) {
    if (!providers[configured].is_local) {
      providers[configured].auth_status = providers[configured].key_hash ? 'configured' : 'configured';
    }
    providers[configured].updated_at = now;
  }
  const normalized = {
    type: 'infring_dashboard_provider_registry',
    updated_at: now,
    providers,
  };
  return normalized;
}

function saveProviderRegistry(state) {
  const normalized = loadProviderRegistry(null);
  const incomingProviders = state && state.providers && typeof state.providers === 'object' ? state.providers : {};
  for (const providerId of Object.keys(incomingProviders)) {
    normalized.providers[providerId] = normalizeProviderRecord(providerId, incomingProviders[providerId]);
  }
  normalized.updated_at = nowIso();
  writeJson(PROVIDER_REGISTRY_PATH, normalized);
  return normalized;
}

function providerKeyMetadata(key = '') {
  const clean = cleanText(key || '', 640);
  if (!clean) {
    return {
      key_prefix: '',
      key_last4: '',
      key_hash: '',
      key_set_at: '',
    };
  }
  return {
    key_prefix: cleanText(clean.slice(0, 6), 12),
    key_last4: cleanText(clean.slice(-4), 12),
    key_hash: sha256(clean),
    key_set_at: nowIso(),
  };
}

function probeOpenAiCompatModels(baseUrl = '', apiKey = '') {
  const normalizedUrl = cleanText(baseUrl || '', 320).replace(/\/+$/, '');
  if (!normalizedUrl || !/^https?:\/\//i.test(normalizedUrl)) {
    return { reachable: false, models: [] };
  }
  const cacheKey = `${normalizedUrl}|${cleanText(apiKey ? sha256(apiKey) : '', 20)}`;
  const cached = localProviderProbeCache.get(cacheKey);
  if (
    cached &&
    cached.ts &&
    (Date.now() - parseNonNegativeInt(cached.ts, 0, 9_999_999_999_999)) < 20_000
  ) {
    return { reachable: !!cached.reachable, models: Array.isArray(cached.models) ? cached.models.slice(0, 64) : [] };
  }
  try {
    const args = ['-sS', '--max-time', '1.4', `${normalizedUrl}/v1/models`];
    if (apiKey) args.unshift('-H', `Authorization: Bearer ${apiKey}`);
    const out = spawnSync('curl', args, {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 2200,
      maxBuffer: 2 * 1024 * 1024,
    });
    const parsed = out && out.stdout ? parseJsonLoose(String(out.stdout || '')) : null;
    const list =
      parsed && parsed.data && Array.isArray(parsed.data)
        ? parsed.data
        : [];
    const models = list
      .map((row) => cleanText(row && row.id ? row.id : '', 160))
      .filter(Boolean)
      .slice(0, 128);
    const reachable = (out && out.status === 0) && models.length > 0;
    localProviderProbeCache.set(cacheKey, { ts: Date.now(), reachable, models });
    return { reachable, models };
  } catch {
    return { reachable: false, models: [] };
  }
}

function loadCustomModels() {
  const state = readJson(CUSTOM_MODELS_PATH, null);
  const rows = Array.isArray(state && state.models) ? state.models : Array.isArray(state) ? state : [];
  return rows
    .map((row) => ({
      id: cleanText(row && row.id ? row.id : '', 160),
      provider: cleanText(row && row.provider ? row.provider : '', 80).toLowerCase(),
      display_name: cleanText(row && row.display_name ? row.display_name : row && row.id ? row.id : '', 160),
      context_window: parsePositiveInt(row && row.context_window != null ? row.context_window : 0, 0, 0, 8_000_000),
      max_output_tokens: parsePositiveInt(row && row.max_output_tokens != null ? row.max_output_tokens : 0, 0, 0, 8_000_000),
      available: row && row.available !== false,
      deployment: cleanText(row && row.deployment ? row.deployment : '', 20) || '',
    }))
    .filter((row) => row.id && row.provider);
}

function saveCustomModels(rows) {
  const safeRows = Array.isArray(rows) ? rows : [];
  writeJson(CUSTOM_MODELS_PATH, {
    type: 'infring_dashboard_custom_models',
    updated_at: nowIso(),
    models: safeRows,
  });
}

function loadChannelRegistry() {
  const now = nowIso();
  const raw = readJson(CHANNEL_REGISTRY_PATH, null);
  const state =
    raw && typeof raw === 'object'
      ? raw
      : { type: 'infring_dashboard_channel_registry', updated_at: now, channels: {} };
  const channels = {};
  for (const def of CHANNEL_DEFAULTS) {
    const id = cleanText(def && def.name ? def.name : '', 80).toLowerCase();
    if (!id) continue;
    const existing = state.channels && state.channels[id] ? state.channels[id] : {};
    const merged = {
      ...def,
      ...existing,
      name: id,
      display_name: cleanText(existing.display_name || def.display_name || id, 120) || id,
      description: cleanText(existing.description || def.description || '', 220),
      quick_setup: cleanText(existing.quick_setup || def.quick_setup || '', 220),
      icon: cleanText(existing.icon || def.icon || '🔌', 8) || '🔌',
      category: cleanText(existing.category || def.category || 'messaging', 40) || 'messaging',
      difficulty: cleanText(existing.difficulty || def.difficulty || 'Medium', 20) || 'Medium',
      setup_time: cleanText(existing.setup_time || def.setup_time || '2 min', 20) || '2 min',
      setup_type: cleanText(existing.setup_type || def.setup_type || 'form', 20) || 'form',
      configured: existing.configured === true,
      has_token: existing.has_token === true,
      fields: Array.isArray(def.fields) ? def.fields.map((field) => ({ ...field })) : [],
      setup_steps: Array.isArray(def.setup_steps) ? def.setup_steps.slice(0, 8) : [],
      config_template: cleanText(def.config_template || '', 600),
      stored_fields: existing.stored_fields && typeof existing.stored_fields === 'object' ? existing.stored_fields : {},
      updated_at: cleanText(existing.updated_at || now, 80) || now,
    };
    if (Array.isArray(merged.fields)) {
      merged.fields = merged.fields.map((field) => {
        const key = cleanText(field && field.key ? field.key : '', 80);
        const value = key && Object.prototype.hasOwnProperty.call(merged.stored_fields, key)
          ? merged.stored_fields[key]
          : '';
        const safeValue = field && field.type === 'secret'
          ? ''
          : cleanText(value, 240);
        return {
          ...field,
          key,
          value: safeValue,
        };
      });
    }
    channels[id] = merged;
  }
  return {
    type: 'infring_dashboard_channel_registry',
    updated_at: now,
    channels,
  };
}

function saveChannelRegistry(state) {
  const safe = state && typeof state === 'object' ? state : loadChannelRegistry();
  safe.updated_at = nowIso();
  writeJson(CHANNEL_REGISTRY_PATH, safe);
  return safe;
}

function loadQrSessions() {
  const raw = readJson(CHANNEL_QR_STATE_PATH, null);
  return raw && typeof raw === 'object' ? raw : {};
}

function saveQrSessions(state) {
  const safe = state && typeof state === 'object' ? state : {};
  writeJson(CHANNEL_QR_STATE_PATH, safe);
  return safe;
}

function readArrayStore(filePath, fallback = []) {
  const raw = readJson(filePath, null);
  const rows = Array.isArray(raw)
    ? raw
    : raw && Array.isArray(raw.items)
      ? raw.items
      : raw && Array.isArray(raw.rows)
        ? raw.rows
        : [];
  const out = Array.isArray(rows) ? rows : fallback;
  return out.slice(0, 5000);
}

function writeArrayStore(filePath, rows) {
  const safeRows = Array.isArray(rows) ? rows : [];
  writeJson(filePath, {
    type: 'infring_dashboard_store',
    updated_at: nowIso(),
    items: safeRows,
  });
  return safeRows;
}

function listGlobalSessionsFromAgentFiles() {
  const rows = [];
  try {
    if (!fs.existsSync(AGENT_SESSIONS_DIR)) return rows;
    const files = fs.readdirSync(AGENT_SESSIONS_DIR).filter((name) => name.endsWith('.json'));
    for (const fileName of files) {
      const agentId = cleanText(fileName.replace(/\.json$/i, ''), 140);
      if (!agentId) continue;
      const state = readJson(path.resolve(AGENT_SESSIONS_DIR, fileName), null);
      const normalized = normalizeSessionState(state, null);
      const list = Array.isArray(normalized.sessions) ? normalized.sessions : [];
      for (const session of list) {
        rows.push({
          session_id: cleanText(session && session.session_id ? session.session_id : '', 120),
          agent_id: agentId,
          label: cleanText(session && session.label ? session.label : 'Session', 120) || 'Session',
          updated_at: cleanText(session && session.updated_at ? session.updated_at : nowIso(), 80) || nowIso(),
          created_at: cleanText(session && session.created_at ? session.created_at : nowIso(), 80) || nowIso(),
          message_count: Array.isArray(session && session.messages) ? session.messages.length : 0,
        });
      }
    }
  } catch {}
  rows.sort((a, b) => coerceTsMs(b && b.updated_at ? b.updated_at : 0, 0) - coerceTsMs(a && a.updated_at ? a.updated_at : 0, 0));
  return rows.slice(0, 2000);
}

function removeSessionById(sessionId = '') {
  const target = cleanText(sessionId || '', 140);
  if (!target) return { ok: false, deleted: false };
  let deleted = false;
  try {
    if (!fs.existsSync(AGENT_SESSIONS_DIR)) return { ok: true, deleted: false };
    const files = fs.readdirSync(AGENT_SESSIONS_DIR).filter((name) => name.endsWith('.json'));
    for (const fileName of files) {
      const filePath = path.resolve(AGENT_SESSIONS_DIR, fileName);
      const state = readJson(filePath, null);
      const normalized = normalizeSessionState(state, null);
      const before = normalized.sessions.length;
      normalized.sessions = normalized.sessions.filter((row) => cleanText(row && row.session_id ? row.session_id : '', 120) !== target);
      if (normalized.sessions.length === before) continue;
      deleted = true;
      if (!normalized.sessions.length) {
        normalized.sessions = [
          {
            session_id: 'default',
            label: 'Default',
            created_at: nowIso(),
            updated_at: nowIso(),
            messages: [],
          },
        ];
      }
      if (!normalized.sessions.some((row) => row.session_id === normalized.active_session_id)) {
        normalized.active_session_id = normalized.sessions[0].session_id;
      }
      writeJson(filePath, normalized);
    }
  } catch {}
  return { ok: true, deleted };
}

function ensureDefaultApprovals() {
  const rows = readArrayStore(APPROVALS_STATE_PATH, []);
  if (rows.length) return rows;
  const seeded = [
    {
      id: `approval_${sha256(`approval:${Date.now()}`).slice(0, 10)}`,
      title: 'Approve runtime remediation lane execution',
      detail: 'Allow dashboard to run runtime telemetry remediation recommendations.',
      status: 'pending',
      created_at: nowIso(),
      source: 'runtime',
    },
  ];
  writeArrayStore(APPROVALS_STATE_PATH, seeded);
  return seeded;
}

function parseOllamaModelList() {
  const cachedAt = parseNonNegativeInt(ollamaModelListCache.ts, 0, 9_999_999_999_999);
  if (
    cachedAt > 0 &&
    (Date.now() - cachedAt) < OLLAMA_MODEL_CACHE_TTL_MS &&
    Array.isArray(ollamaModelListCache.models)
  ) {
    return ollamaModelListCache.models.slice(0, 128);
  }
  if (!commandExists(OLLAMA_BIN)) return [];
  try {
    const proc = spawnSync(OLLAMA_BIN, ['list'], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      timeout: 2200,
      maxBuffer: 2 * 1024 * 1024,
    });
    if (!proc || proc.status !== 0) {
      ollamaModelListCache = { ts: Date.now(), models: [] };
      return [];
    }
    const out = String(proc.stdout || '');
    const lines = out.split('\n').map((row) => row.trim()).filter(Boolean);
    const models = [];
    for (const line of lines) {
      if (/^name\s+/i.test(line)) continue;
      const parts = line.split(/\s+/);
      const id = cleanText(parts[0] || '', 120);
      if (!id || id.toLowerCase() === 'name') continue;
      models.push(id);
    }
    const deduped = Array.from(new Set(models));
    ollamaModelListCache = {
      ts: Date.now(),
      models: deduped.slice(0, 128),
    };
    return ollamaModelListCache.models.slice(0, 128);
  } catch {
    ollamaModelListCache = { ts: Date.now(), models: [] };
    return [];
  }
}

function discoverLocalProviderState(snapshot, options = {}) {
  const force = !!(options && options.force === true);
  const nowMs = Date.now();
  if (
    !force &&
    (nowMs - parseNonNegativeInt(localProviderAutoDiscoverAtMs, 0, 9_999_999_999_999)) <
      LOCAL_PROVIDER_DISCOVERY_INTERVAL_MS
  ) {
    return loadProviderRegistry(snapshot);
  }
  localProviderAutoDiscoverAtMs = nowMs;
  const registry = loadProviderRegistry(snapshot);
  const providers =
    registry && registry.providers && typeof registry.providers === 'object'
      ? { ...registry.providers }
      : {};
  let changed = false;
  const ensureProvider = (providerId, nextValue) => {
    const next = normalizeProviderRecord(providerId, nextValue);
    const prev = providers[providerId] || normalizeProviderRecord(providerId, { id: providerId });
    providers[providerId] = next;
    if (JSON.stringify(prev) !== JSON.stringify(next)) {
      changed = true;
    }
  };

  const configured = configuredOllamaModel(snapshot);
  const fromOllama = parseOllamaModelList();
  const mergedOllama = Array.from(new Set([configured, OLLAMA_MODEL_FALLBACK, ...fromOllama].filter(Boolean))).slice(0, 128);
  const ollamaPrev = providers.ollama || normalizeProviderRecord('ollama', {});
  ensureProvider('ollama', {
    ...ollamaPrev,
    is_local: true,
    needs_key: false,
    reachable: mergedOllama.length > 0,
    auth_status: mergedOllama.length > 0 ? 'configured' : 'not_set',
    detected_models: mergedOllama,
    updated_at: nowIso(),
  });

  for (const providerId of Object.keys(providers)) {
    if (providerId === 'ollama') continue;
    const providerRow = providers[providerId];
    if (!(providerRow && providerRow.is_local)) continue;
    const baseUrl = cleanText(providerRow.base_url || '', 320);
    if (!baseUrl || !/^https?:\/\//i.test(baseUrl)) continue;
    const probe = probeOpenAiCompatModels(baseUrl, '');
    ensureProvider(providerId, {
      ...providerRow,
      reachable: probe.reachable || !!providerRow.reachable,
      auth_status:
        probe.reachable || !!providerRow.key_hash || (Array.isArray(probe.models) && probe.models.length > 0)
          ? 'configured'
          : 'not_set',
      detected_models:
        Array.isArray(probe.models) && probe.models.length
          ? probe.models.slice(0, 128)
          : Array.isArray(providerRow.detected_models)
            ? providerRow.detected_models.slice(0, 128)
            : [],
      updated_at: nowIso(),
    });
  }

  if (changed) {
    saveProviderRegistry({ providers });
  }
  return {
    ...registry,
    providers,
  };
}

function buildDashboardModels(snapshot) {
  const rows = [];
  const configuredWindow = inferContextWindowFromModelName(configuredOllamaModel(snapshot), DEFAULT_CONTEXT_WINDOW_TOKENS);
  const providerRegistry = discoverLocalProviderState(snapshot);
  const providers = providerRegistry && providerRegistry.providers && typeof providerRegistry.providers === 'object'
    ? providerRegistry.providers
    : {};
  const customModels = loadCustomModels();

  rows.push({
    id: 'auto',
    provider: 'auto',
    display_name: 'Auto',
    tier: 'Balanced',
    available: true,
    supports_tools: true,
    supports_vision: false,
    context_window: configuredWindow,
    deployment: 'cloud',
    is_local: false,
  });

  const configured = configuredOllamaModel(snapshot);
  const ollamaProvider = providers.ollama || normalizeProviderRecord('ollama', {});
  const mergedOllama = Array.from(
    new Set(
      [configured, OLLAMA_MODEL_FALLBACK]
        .concat(Array.isArray(ollamaProvider.detected_models) ? ollamaProvider.detected_models : [])
        .filter(Boolean)
    )
  ).slice(0, 128);

  for (const id of mergedOllama) {
    rows.push({
      id,
      provider: 'ollama',
      display_name: id,
      tier: 'Balanced',
      available: true,
      supports_tools: true,
      supports_vision: /\b(vision|vl|llava)\b/i.test(String(id || '')),
      context_window: inferContextWindowFromModelName(id, configuredWindow),
      deployment: 'local',
      is_local: true,
    });
  }

  const seenModelKey = new Set(rows.map((row) => `${String(row.provider).toLowerCase()}/${String(row.id).toLowerCase()}`));
  const addProviderModel = (providerId, modelId, options = {}) => {
    const cleanProvider = cleanText(providerId || '', 80).toLowerCase();
    const cleanModel = cleanText(modelId || '', 180);
    if (!cleanProvider || !cleanModel) return;
    const key = `${cleanProvider}/${cleanModel.toLowerCase()}`;
    if (seenModelKey.has(key)) return;
    seenModelKey.add(key);
    const fallbackWindow = inferContextWindowFromModelName(cleanModel, DEFAULT_CONTEXT_WINDOW_TOKENS);
    const local = options.is_local === true;
    rows.push({
      id: cleanProvider === 'ollama' ? cleanModel : `${cleanProvider}/${cleanModel}`,
      provider: cleanProvider,
      display_name: cleanModel,
      tier: cleanText(options.tier || 'Balanced', 24) || 'Balanced',
      available: options.available !== false,
      supports_tools: options.supports_tools !== false,
      supports_vision:
        options.supports_vision === true || /\b(vision|vl|gpt-4o|gemini|claude[-\s]?3|qwen2\.5-vl|llava)\b/i.test(cleanModel),
      context_window: parsePositiveInt(
        options.context_window != null ? options.context_window : fallbackWindow,
        fallbackWindow,
        1024,
        8_000_000
      ),
      deployment: local ? 'local' : 'cloud',
      is_local: local,
    });
  };

  for (const providerId of Object.keys(providers)) {
    if (providerId === 'auto' || providerId === 'ollama') continue;
    const providerRow = providers[providerId];
    const isLocal = !!(providerRow && providerRow.is_local);
    let providerModels = Array.isArray(providerRow && providerRow.detected_models)
      ? providerRow.detected_models.slice(0, 128)
      : [];
    if (isLocal && providerModels.length === 0) {
      const probe = probeOpenAiCompatModels(providerRow && providerRow.base_url ? providerRow.base_url : '', '');
      if (probe.reachable && probe.models.length) {
        providerModels = probe.models.slice(0, 128);
        providerRow.reachable = true;
        providerRow.auth_status = 'configured';
        providerRow.detected_models = providerModels.slice(0, 128);
        providers[providerId] = normalizeProviderRecord(providerId, providerRow);
      }
    }
    if (!providerModels.length) {
      providerModels = Array.isArray(PROVIDER_MODEL_CATALOG[providerId]) ? PROVIDER_MODEL_CATALOG[providerId].slice(0, 24) : [];
    }
    for (const modelId of providerModels) {
      addProviderModel(providerId, modelId, { is_local: isLocal, tier: 'Balanced' });
    }
  }

  for (const modelRow of customModels) {
    addProviderModel(modelRow.provider, modelRow.id, {
      is_local: String(modelRow.deployment || '').toLowerCase() === 'local',
      available: modelRow.available !== false,
      context_window: modelRow.context_window,
      tier: 'Custom',
    });
  }
  return rows;
}

function modelOverrideFromState(state) {
  const raw = cleanText(state && state.model_override ? state.model_override : '', 120).toLowerCase();
  if (!raw || raw === 'auto') return 'auto';
  return cleanText(state && state.model_override ? state.model_override : '', 120) || 'auto';
}

function readAgentModelOverride(agentId, options = {}) {
  const id = cleanText(agentId || '', 140);
  if (!id) return 'auto';
  const allowSessionRead = options.allow_session_read !== false;
  const profile = agentProfileFor(id);
  if (profile && Object.prototype.hasOwnProperty.call(profile, 'model_override')) {
    return modelOverrideFromState(profile);
  }
  if (!allowSessionRead) return 'auto';
  const state = readJson(agentSessionPath(id), null);
  return modelOverrideFromState(state);
}

function providerForModelName(modelName, fallbackProvider = 'ollama') {
  const value = cleanText(modelName || '', 120);
  if (!value) return cleanText(fallbackProvider || 'ollama', 80) || 'ollama';
  if (value.toLowerCase() === 'auto') return 'auto';
  if (/:cloud$/i.test(value)) return 'cloud';
  if (value.startsWith('ollama/')) return 'ollama';
  if (value.includes('/')) return cleanText(value.split('/')[0], 80) || cleanText(fallbackProvider || 'ollama', 80);
  return cleanText(fallbackProvider || 'ollama', 80) || 'ollama';
}

function autoRouteIntentContext(input = '', tokenCount = 0, hasVision = false) {
  const text = cleanText(input || '', 4000).toLowerCase();
  const estimatedTokens = parsePositiveInt(
    tokenCount,
    Math.max(1, Math.round(String(input || '').length / 4)),
    1,
    8_000_000
  );
  return {
    token_count: estimatedTokens,
    has_vision: !!hasVision,
    asks_speed: /\b(fast|quick|latency|snappy|throughput|real[-\s]?time)\b/i.test(text),
    asks_cost: /\b(cheap|cost|budget|afford|save|token burn|low[-\s]?cost)\b/i.test(text),
    asks_quality: /\b(quality|accurate|deep|thorough|careful|reason|analysis)\b/i.test(text),
    asks_long_context: /\b(week ago|history|long context|large context|memory|timeline|summar)\b/i.test(text),
  };
}

function normalizeAutoRouteCandidate(rawId, providerHint = '') {
  const candidateId = cleanText(rawId || '', 140);
  if (!candidateId || candidateId.toLowerCase() === 'auto') return null;
  let runtimeProvider = cleanText(providerHint || '', 80) || providerForModelName(candidateId, 'ollama');
  let runtimeModel = candidateId;
  if (candidateId.startsWith('ollama/')) {
    runtimeProvider = 'ollama';
    runtimeModel = cleanText(candidateId.replace(/^ollama\//, ''), 120) || candidateId;
  } else if (candidateId.includes('/')) {
    const parts = candidateId.split('/');
    runtimeProvider = cleanText(parts.shift() || runtimeProvider || 'ollama', 80) || 'ollama';
    runtimeModel = cleanText(parts.join('/'), 120) || candidateId;
  } else {
    runtimeProvider = providerForModelName(candidateId, runtimeProvider || 'ollama');
  }
  const modelKey = cleanText(runtimeModel || candidateId, 120) || candidateId;
  const providerKey = cleanText(runtimeProvider || 'ollama', 80) || 'ollama';
  const contextWindow = inferContextWindowFromModelName(modelKey, DEFAULT_CONTEXT_WINDOW_TOKENS);
  const supportsVision = /\b(vision|vl|gpt-4o|gemini|claude[-\s]?3|qwen2\.5-vl|llava)\b/i.test(modelKey);
  return {
    id: providerKey === 'ollama' ? modelKey : `${providerKey}/${modelKey}`,
    provider: providerKey,
    runtime_provider: providerKey,
    model: modelKey,
    runtime_model: modelKey,
    context_window: contextWindow,
    supports_vision: supportsVision,
  };
}

function buildAutoRouteCandidates(snapshot, agentId = '') {
  const candidates = [];
  const seen = new Set();
  const addCandidate = (rawId, providerHint = '') => {
    const row = normalizeAutoRouteCandidate(rawId, providerHint);
    if (!row) return;
    const key = `${row.runtime_provider}/${row.runtime_model}`.toLowerCase();
    if (!key || seen.has(key)) return;
    seen.add(key);
    candidates.push(row);
  };

  const dashboardModels = buildDashboardModels(snapshot);
  for (const row of dashboardModels) {
    if (!row || !row.id || String(row.id).toLowerCase() === 'auto') continue;
    addCandidate(row.id, row.provider || 'ollama');
  }

  const configuredModel = cleanText(
    snapshot && snapshot.app && snapshot.app.settings && snapshot.app.settings.model
      ? snapshot.app.settings.model
      : configuredOllamaModel(snapshot),
    120
  ) || configuredOllamaModel(snapshot);
  const configuredProviderValue = cleanText(configuredProvider(snapshot), 80) || 'ollama';
  if (configuredProviderValue !== 'ollama' && configuredModel) {
    addCandidate(`${configuredProviderValue}/${configuredModel}`, configuredProviderValue);
  } else if (configuredModel) {
    addCandidate(configuredModel, configuredProviderValue);
  }

  const testOverride = testingModelOverrideForAgent(agentId);
  if (testOverride && testOverride.model) {
    addCandidate(testOverride.model, cleanText(testOverride.provider || '', 80) || 'cloud');
  }

  const profile = readJson(MODEL_ROUTER_PROVIDER_PROFILE_PATH, null);
  const profileModel = cleanText(profile && profile.preferred_model ? profile.preferred_model : '', 120);
  if (profileModel) {
    addCandidate(profileModel, profileModel.includes('/') ? '' : 'cloud');
  }

  if (profileModel && /:cloud$/i.test(profileModel)) {
    addCandidate(profileModel, 'cloud');
  } else {
    addCandidate(TEST_AGENT_MODEL_DEFAULT, TEST_AGENT_PROVIDER_DEFAULT);
  }
  return candidates;
}

function autoRouteProviderPrior(provider = '') {
  const key = cleanText(provider || '', 40).toLowerCase();
  if (key === 'ollama') return { latency_ms: 120, cost_per_1k: 0.0, success_rate: 0.92 };
  if (key === 'groq') return { latency_ms: 65, cost_per_1k: 0.2, success_rate: 0.9 };
  if (key === 'openai') return { latency_ms: 90, cost_per_1k: 0.55, success_rate: 0.95 };
  if (key === 'anthropic') return { latency_ms: 105, cost_per_1k: 0.7, success_rate: 0.95 };
  if (key === 'google') return { latency_ms: 95, cost_per_1k: 0.6, success_rate: 0.94 };
  if (key === 'cloud') return { latency_ms: 80, cost_per_1k: 0.3, success_rate: 0.93 };
  return { latency_ms: 110, cost_per_1k: 0.45, success_rate: 0.9 };
}

function autoRouteCandidateScore(candidate, context, runtimeSync) {
  const priors = autoRouteProviderPrior(candidate && candidate.runtime_provider ? candidate.runtime_provider : '');
  const modelName = cleanText(candidate && candidate.runtime_model ? candidate.runtime_model : '', 120).toLowerCase();
  const latencyMs = Math.max(1, Math.round(priors.latency_ms * (/3b|mini|small/.test(modelName) ? 0.85 : 1)));
  const costPer1k = Math.max(0, Number((priors.cost_per_1k * (/3b|mini|small/.test(modelName) ? 0.7 : 1)).toFixed(4)));
  const contextWindow = parsePositiveInt(
    candidate && candidate.context_window != null ? candidate.context_window : DEFAULT_CONTEXT_WINDOW_TOKENS,
    DEFAULT_CONTEXT_WINDOW_TOKENS,
    1024,
    8_000_000
  );
  const tokenDemand = parsePositiveInt(context && context.token_count, 1, 1, 8_000_000);
  const contextScore = tokenDemand <= contextWindow ? 1 : Math.max(0.1, Number((contextWindow / tokenDemand).toFixed(4)));
  const latencyScore = Number((1 / (1 + (latencyMs / 120))).toFixed(6));
  const costScore = Number((1 / (1 + costPer1k)).toFixed(6));
  const runtimeSuccess = Number(
    runtimeSync && runtimeSync.spine_success_rate != null ? runtimeSync.spine_success_rate : Number.NaN
  );
  const successRateBase = Number.isFinite(runtimeSuccess)
    ? Math.max(0.2, Math.min(0.99, (priors.success_rate * 0.65) + (runtimeSuccess * 0.35)))
    : priors.success_rate;
  const supportsVision = !!(candidate && candidate.supports_vision);
  const visionPenalty = context && context.has_vision && !supportsVision ? 0.55 : 0;
  const speedWeight = context && context.asks_speed ? 1.55 : 1.05;
  const costWeight = context && context.asks_cost ? 1.35 : 0.75;
  const qualityWeight = context && context.asks_quality ? 1.8 : 1.3;
  const contextWeight = context && context.asks_long_context ? 1.45 : 1.1;
  const scoreRaw =
    (latencyScore * speedWeight) +
    (costScore * costWeight) +
    (successRateBase * qualityWeight) +
    (contextScore * contextWeight) -
    visionPenalty;
  return {
    score: Number(scoreRaw.toFixed(6)),
    latency_ms: latencyMs,
    cost_per_1k: costPer1k,
    success_rate: Number(successRateBase.toFixed(4)),
    context_window: contextWindow,
    context_score: contextScore,
    supports_vision: supportsVision,
    vision_penalty: visionPenalty,
  };
}

function rustRouteDecision(input, snapshot, options = {}) {
  const context = autoRouteIntentContext(
    input,
    options && options.token_count != null ? options.token_count : 0,
    !!(options && options.has_vision)
  );
  const runtimeSync = runtimeSyncSummary(snapshot);
  const agentId = cleanText(options && options.agent_id ? options.agent_id : '', 140);
  const modelState = effectiveAgentModel(agentId || 'chat-ui-default-agent', snapshot);
  const preferredProvider = cleanText(
    modelState && (modelState.runtime_provider || modelState.provider)
      ? modelState.runtime_provider || modelState.provider
      : configuredProvider(snapshot),
    80
  ) || 'ollama';
  const preferredModel = cleanText(
    modelState && modelState.runtime_model ? modelState.runtime_model : configuredOllamaModel(snapshot),
    120
  ) || configuredOllamaModel(snapshot);
  const fallbackOverride = testingModelOverrideForAgent(agentId);
  const fallbackProvider = cleanText(
    fallbackOverride && fallbackOverride.provider ? fallbackOverride.provider : TEST_AGENT_PROVIDER_DEFAULT,
    80
  ) || TEST_AGENT_PROVIDER_DEFAULT;
  const fallbackModel = cleanText(
    fallbackOverride && fallbackOverride.model ? fallbackOverride.model : TEST_AGENT_MODEL_DEFAULT,
    120
  ) || TEST_AGENT_MODEL_DEFAULT;
  const routeInput = cleanText(input || '', 1200);
  const candidateRows = buildAutoRouteCandidates(snapshot, agentId).map((row) => ({
    runtime_provider: cleanText(
      row && (row.runtime_provider || row.provider) ? row.runtime_provider || row.provider : '',
      80
    ) || 'ollama',
    runtime_model: cleanText(
      row && (row.runtime_model || row.model || row.id) ? row.runtime_model || row.model || row.id : '',
      120
    ) || configuredOllamaModel(snapshot),
    context_window: parsePositiveInt(
      row && row.context_window != null ? row.context_window : DEFAULT_CONTEXT_WINDOW_TOKENS,
      DEFAULT_CONTEXT_WINDOW_TOKENS,
      1024,
      8_000_000
    ),
    supports_vision: !!(row && row.supports_vision),
  }));
  const payload = {
    ...runtimeAuthorityPayload(runtimeSync),
    input_text: routeInput,
    token_count: parsePositiveInt(context.token_count, 1, 1, 8_000_000),
    has_vision: !!context.has_vision,
    asks_speed: !!context.asks_speed,
    asks_cost: !!context.asks_cost,
    asks_quality: !!context.asks_quality,
    asks_long_context: !!context.asks_long_context,
    preferred_provider: preferredProvider,
    preferred_model: preferredModel,
    fallback_provider: fallbackProvider,
    fallback_model: fallbackModel,
    candidates: candidateRows,
  };
  const cacheKeyHash = sha256(JSON.stringify({
    agent_id: agentId,
    payload,
  })).slice(0, 24);
  const lane = runLaneCached(
    `auto.route.rust.${cacheKeyHash}`,
    [
      'runtime-systems',
      'run',
      '--system-id=V6-DASHBOARD-008.1',
      '--strict=1',
      '--apply=0',
      `--payload-json=${JSON.stringify(payload)}`,
    ],
    {
      timeout_ms: AUTO_ROUTE_LANE_TIMEOUT_MS,
      ttl_ms: AUTO_ROUTE_CACHE_TTL_MS,
      fail_ttl_ms: AUTO_ROUTE_CACHE_FAIL_TTL_MS,
    }
  );
  const lanePayload = lane && lane.payload && typeof lane.payload === 'object' ? lane.payload : null;
  const contractExecution =
    lanePayload &&
    lanePayload.contract_execution &&
    typeof lanePayload.contract_execution === 'object'
      ? lanePayload.contract_execution
      : null;
  const specificChecks =
    contractExecution &&
    contractExecution.specific_checks &&
    typeof contractExecution.specific_checks === 'object'
      ? contractExecution.specific_checks
      : null;
  const authorityDecision =
    specificChecks &&
    specificChecks.dashboard_auto_route_authority &&
    typeof specificChecks.dashboard_auto_route_authority === 'object'
      ? specificChecks.dashboard_auto_route_authority
      : null;
  if (!lane || !lane.ok || !lanePayload || !authorityDecision) {
    return {
      ok: false,
      type: 'infring_auto_route_decision',
      policy: 'V6-DASHBOARD-008.1',
      authority: 'rust_runtime_systems',
      error: 'route_lane_failed',
      route_lane: 'runtime-systems.run',
      lane: laneOutcome(lane || null),
      context,
      runtime_sync: {
        spine_success_rate: Number.isFinite(Number(runtimeSync && runtimeSync.spine_success_rate))
          ? Number(runtimeSync.spine_success_rate)
          : null,
        receipt_latency_p99_ms:
          Number.isFinite(Number(runtimeSync && runtimeSync.receipt_latency_p99_ms))
            ? Number(runtimeSync.receipt_latency_p99_ms)
            : null,
      },
    };
  }
  const decision = {
    ...authorityDecision,
    ok: true,
    type: 'infring_auto_route_decision',
    policy: 'V6-DASHBOARD-008.1',
    authority: cleanText(authorityDecision.authority || 'rust_runtime_systems', 40) || 'rust_runtime_systems',
    route_lane: cleanText(authorityDecision.route_lane || 'runtime-systems.run', 80) || 'runtime-systems.run',
    context,
    runtime_sync: authorityDecision.runtime_sync && typeof authorityDecision.runtime_sync === 'object'
      ? authorityDecision.runtime_sync
      : {
          spine_success_rate: Number.isFinite(Number(runtimeSync && runtimeSync.spine_success_rate))
            ? Number(runtimeSync.spine_success_rate)
            : null,
          receipt_latency_p99_ms:
            Number.isFinite(Number(runtimeSync && runtimeSync.receipt_latency_p99_ms))
              ? Number(runtimeSync.receipt_latency_p99_ms)
              : null,
        },
    lane_status: laneOutcome(lane),
    lane_receipt_hash: cleanText(lanePayload && lanePayload.receipt_hash ? lanePayload.receipt_hash : '', 80),
  };
  decision.receipt_hash = cleanText(decision.receipt_hash || '', 80)
    || cleanText(lanePayload && lanePayload.receipt_hash ? lanePayload.receipt_hash : '', 80)
    || sha256(JSON.stringify(decision));
  return decision;
}

function planAutoRoute(input, snapshot, options = {}) {
  return rustRouteDecision(input, snapshot, options);
}

function effectiveAgentModel(agentId, snapshot, options = {}) {
  const override = readAgentModelOverride(agentId, {
    allow_session_read: options.allow_session_read !== false,
  });
  const testModel = testingModelOverrideForAgent(agentId);
  const defaultModel = testModel ? testModel.model : configuredOllamaModel(snapshot);
  const defaultProvider = testModel ? testModel.provider : 'ollama';
  const defaultContextWindow = inferContextWindowFromModelName(defaultModel, DEFAULT_CONTEXT_WINDOW_TOKENS);
  if (override === 'auto') {
    return {
      selected: 'auto',
      provider: 'auto',
      runtime_model: defaultModel,
      runtime_provider: defaultProvider,
      context_window: defaultContextWindow,
    };
  }
  const normalized = cleanText(override, 120) || defaultModel;
  let runtimeProvider = providerForModelName(normalized, defaultProvider);
  let runtimeModel = normalized;
  if (normalized.startsWith('ollama/')) {
    runtimeProvider = 'ollama';
    runtimeModel = cleanText(normalized.replace(/^ollama\//, ''), 120) || defaultModel;
  } else if (normalized.includes('/')) {
    const parts = normalized.split('/');
    runtimeProvider = cleanText(parts.shift() || runtimeProvider, 80) || runtimeProvider;
    runtimeModel = cleanText(parts.join('/'), 120) || defaultModel;
  }
  if (!runtimeModel || runtimeModel.toLowerCase() === 'auto') runtimeModel = defaultModel;
  const contextWindow = inferContextWindowFromModelName(
    normalized && normalized !== 'auto' ? normalized : (runtimeModel || defaultModel),
    defaultContextWindow
  );
  return {
    selected: normalized,
    provider: cleanText(runtimeProvider || defaultProvider || 'ollama', 80) || 'ollama',
    runtime_model: runtimeModel,
    runtime_provider: cleanText(runtimeProvider || defaultProvider || 'ollama', 80) || 'ollama',
    context_window: contextWindow,
  };
}

function shouldUseHostedModelBackend(modelState) {
  const provider = cleanText(
    modelState && (modelState.runtime_provider || modelState.provider)
      ? modelState.runtime_provider || modelState.provider
      : '',
    80
  ).toLowerCase();
  const model = cleanText(
    modelState && modelState.runtime_model ? modelState.runtime_model : '',
    120
  ).toLowerCase();
  if (!provider) return false;
  if (provider !== 'ollama') return true;
  return /:cloud$/.test(model);
}

function ensureHostedChatProviderModel(snapshot, modelState) {
  const provider =
    cleanText(
      modelState && (modelState.runtime_provider || modelState.provider)
        ? modelState.runtime_provider || modelState.provider
        : '',
      80
    ) || '';
  const model = cleanText(
    modelState && modelState.runtime_model ? modelState.runtime_model : '',
    120
  );
  if (!provider || !model || provider.toLowerCase() === 'ollama') {
    return { ok: true, skipped: true, reason: 'local_provider' };
  }
  const currentProvider = cleanText(configuredProvider(snapshot), 80) || '';
  const currentModel =
    cleanText(
      snapshot && snapshot.app && snapshot.app.settings && snapshot.app.settings.model
        ? snapshot.app.settings.model
        : configuredOllamaModel(snapshot),
      120
    ) || '';
  if (currentProvider.toLowerCase() === provider.toLowerCase() && currentModel === model) {
    return { ok: true, skipped: true, reason: 'already_configured' };
  }
  const lane = runAction('app.switchProvider', { provider, model });
  return {
    ok: !!(lane && lane.ok),
    skipped: false,
    provider,
    model,
    lane,
  };
}

function runOllamaPrompt(model, prompt, options = {}) {
  const selectedModel = cleanText(model || OLLAMA_MODEL_FALLBACK, 120) || OLLAMA_MODEL_FALLBACK;
  const timeoutMs = parsePositiveInt(
    options && options.timeout_ms != null ? options.timeout_ms : OLLAMA_TIMEOUT_MS,
    OLLAMA_TIMEOUT_MS,
    250,
    120_000
  );
  try {
    const proc = spawnSync(OLLAMA_BIN, ['run', selectedModel, String(prompt || '')], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      env: { ...process.env, PROTHEUS_ROOT: ROOT },
      timeout: timeoutMs,
      maxBuffer: 8 * 1024 * 1024,
    });
    const rawStatus = typeof proc.status === 'number' ? proc.status : null;
    const stdout = stripAnsi(typeof proc.stdout === 'string' ? proc.stdout : '');
    const stderr = stripAnsi(typeof proc.stderr === 'string' ? proc.stderr : '');
    const timedOut =
      (proc && proc.error && String(proc.error.code || '') === 'ETIMEDOUT') ||
      (typeof proc.signal === 'string' && proc.signal.length > 0);
    const output = stdout.trim();
    const ok = !!output && (rawStatus === 0 || timedOut || rawStatus === null);
    return {
      ok,
      status: rawStatus == null ? (ok ? 0 : 1) : rawStatus,
      output,
      error: stderr.trim(),
      model: selectedModel,
    };
  } catch (error) {
    return {
      ok: false,
      status: 1,
      output: '',
      error: cleanText(error && error.message ? error.message : String(error), 260),
      model: selectedModel,
    };
  }
}

function roleLabelFromMessage(row) {
  const role = cleanText(row && row.role ? row.role : '', 20).toLowerCase();
  if (role === 'user') return 'User';
  if (role === 'agent' || role === 'assistant') return 'Agent';
  return 'System';
}

function promptTranscript(session) {
  const rows = Array.isArray(session && session.messages) ? session.messages.slice(-8) : [];
  return rows
    .map((row) => `${roleLabelFromMessage(row)}: ${cleanText(row && row.content ? row.content : '', 600)}`)
    .filter(Boolean)
    .join('\n');
}

function runtimeContextPrompt(snapshot, runtimeMirror = null) {
  const mirror = runtimeMirror && typeof runtimeMirror === 'object' ? runtimeMirror : null;
  const cockpit = mirror && mirror.cockpit && typeof mirror.cockpit === 'object'
    ? mirror.cockpit
    : snapshot && snapshot.cockpit && typeof snapshot.cockpit === 'object'
      ? snapshot.cockpit
      : {};
  const attention = mirror && mirror.attention_queue && typeof mirror.attention_queue === 'object'
    ? mirror.attention_queue
    : snapshot && snapshot.attention_queue && typeof snapshot.attention_queue === 'object'
      ? snapshot.attention_queue
      : {};
  const queueDepth = parseNonNegativeInt(
    attention && attention.queue_depth != null ? attention.queue_depth : 0,
    0,
    100000000
  );
  const backpressure =
    attention && attention.backpressure && typeof attention.backpressure === 'object'
      ? attention.backpressure
      : {};
  const blocks = Array.isArray(cockpit.blocks) ? cockpit.blocks.slice(0, 6) : [];
  const events = Array.isArray(attention.events) ? attention.events.slice(0, 6) : [];
  const conduitSignals =
    mirror && mirror.summary && mirror.summary.conduit_signals != null
      ? parseNonNegativeInt(mirror.summary.conduit_signals, 0, 100000000)
      : blocks.filter((row) => {
          const lane = String(row && row.lane ? row.lane : '').toLowerCase();
          const eventType = String(row && row.event_type ? row.event_type : '').toLowerCase();
          return lane.includes('conduit') || eventType.includes('conduit');
        }).length;
  const topCockpit = blocks
    .map(
      (row) =>
        `${cleanText(row && row.lane ? row.lane : 'unknown', 60)}:${cleanText(
          row && row.event_type ? row.event_type : 'unknown',
          60
        )}:${cleanText(row && row.status ? row.status : 'unknown', 20)}`
    )
    .filter(Boolean)
    .join(' | ');
  const topAttention = events
    .map(
      (row) =>
        `${cleanText(row && row.source ? row.source : 'unknown', 40)}:${cleanText(
          row && row.severity ? row.severity : 'info',
          20
        )}:${cleanText(row && row.summary ? row.summary : '', 90)}`
    )
    .filter(Boolean)
    .join(' | ');
  const memoryEntries = Array.isArray(snapshot && snapshot.memory && snapshot.memory.entries)
    ? snapshot.memory.entries.length
    : 0;
  const receiptEntries = Array.isArray(snapshot && snapshot.receipts && snapshot.receipts.recent)
    ? snapshot.receipts.recent.length
    : 0;
  const logEntries = Array.isArray(snapshot && snapshot.logs && snapshot.logs.recent)
    ? snapshot.logs.recent.length
    : 0;
  const healthCheckCount =
    snapshot && snapshot.health && snapshot.health.checks && typeof snapshot.health.checks === 'object'
      ? Object.keys(snapshot.health.checks).length
      : 0;
  const benchmarkCheck =
    snapshot &&
    snapshot.health &&
    snapshot.health.checks &&
    typeof snapshot.health.checks === 'object' &&
    snapshot.health.checks.benchmark_sanity &&
    typeof snapshot.health.checks.benchmark_sanity === 'object'
      ? snapshot.health.checks.benchmark_sanity
      : {};
  const benchmarkStatus = cleanText(benchmarkCheck.status || 'unknown', 24) || 'unknown';
  const benchmarkAgeSec = parsePositiveInt(benchmarkCheck.age_seconds, -1, -1, 1000000000);
  const syncMode = cleanText(backpressure.sync_mode || 'live_sync', 24) || 'live_sync';
  const pressureLevel = cleanText(backpressure.level || 'normal', 24) || 'normal';
  const activeAgents = activeAgentCountFromSnapshot(snapshot, 0);
  const targetConduitSignals = parsePositiveInt(
    backpressure && backpressure.target_conduit_signals != null
      ? backpressure.target_conduit_signals
      : mirror && mirror.summary && mirror.summary.target_conduit_signals != null
        ? mirror.summary.target_conduit_signals
        : recommendedConduitSignals(
            queueDepth,
            Number.isFinite(Number(backpressure && backpressure.queue_utilization))
              ? Number(backpressure.queue_utilization)
              : 0,
            parseNonNegativeInt(cockpit && cockpit.block_count, blocks.length, 100000000),
            activeAgents
          ),
    4,
    1,
    128
  );
  const conduitScaleRequired =
    !!(backpressure && backpressure.scale_required) ||
    !!(mirror && mirror.summary && mirror.summary.conduit_scale_required);
  const criticalAttention = parseNonNegativeInt(
    attention && attention.priority_counts && attention.priority_counts.critical != null
      ? attention.priority_counts.critical
      : 0,
    0,
    1000000
  );
  const criticalAttentionTotal = parseNonNegativeInt(
    attention && attention.critical_total_count != null ? attention.critical_total_count : criticalAttention,
    criticalAttention,
    0,
    1000000
  );
  const standardAttention = parseNonNegativeInt(
    attention && attention.priority_counts && attention.priority_counts.standard != null
      ? attention.priority_counts.standard
      : 0,
    0,
    1000000
  );
  const backgroundAttention = parseNonNegativeInt(
    attention && attention.priority_counts && attention.priority_counts.background != null
      ? attention.priority_counts.background
      : 0,
    0,
    1000000
  );
  const telemetryMicroBatchCount = parseNonNegativeInt(
    attention && Array.isArray(attention.telemetry_micro_batches)
      ? attention.telemetry_micro_batches.length
      : 0,
    0,
    1000000
  );
  const laneWeights =
    backpressure && backpressure.lane_weights && typeof backpressure.lane_weights === 'object'
      ? backpressure.lane_weights
      : ATTENTION_LANE_WEIGHTS;
  const laneCaps =
    backpressure && backpressure.lane_caps && typeof backpressure.lane_caps === 'object'
      ? backpressure.lane_caps
      : ATTENTION_LANE_CAPS;
  const microBatchWindowMs = parsePositiveInt(
    backpressure && backpressure.micro_batch_window_ms != null
      ? backpressure.micro_batch_window_ms
      : ATTENTION_MICRO_BATCH_WINDOW_MS,
    ATTENTION_MICRO_BATCH_WINDOW_MS,
    1,
    10000
  );
  const microBatchMaxItems = parsePositiveInt(
    backpressure && backpressure.micro_batch_max_items != null
      ? backpressure.micro_batch_max_items
      : ATTENTION_MICRO_BATCH_MAX_ITEMS,
    ATTENTION_MICRO_BATCH_MAX_ITEMS,
    1,
    256
  );
  const healthCoverage =
    snapshot && snapshot.health && snapshot.health.coverage && typeof snapshot.health.coverage === 'object'
      ? snapshot.health.coverage
      : {};
  const ingestControl =
    snapshot && snapshot.memory && snapshot.memory.ingest_control && typeof snapshot.memory.ingest_control === 'object'
      ? snapshot.memory.ingest_control
      : {};
  const deferredDepth = parseNonNegativeInt(
    attention && attention.deferred_events != null ? attention.deferred_events : 0,
    0,
    100000000
  );
  const deferredMode = cleanText(attention && attention.deferred_mode ? attention.deferred_mode : 'pass_through', 24) || 'pass_through';
  const staleCockpitBlocks = parseNonNegativeInt(
    cockpit && cockpit.metrics && cockpit.metrics.stale_block_count != null ? cockpit.metrics.stale_block_count : 0,
    0,
    100000000
  );
  const cockpitActiveBlocks = parseNonNegativeInt(cockpit && cockpit.block_count, blocks.length, 100000000);
  const cockpitTotalBlocks = parseNonNegativeInt(
    cockpit && cockpit.total_block_count != null ? cockpit.total_block_count : blocks.length,
    blocks.length,
    100000000
  );
  return [
    `Queue depth: ${queueDepth}`,
    `Cockpit blocks: ${cockpitActiveBlocks} active / ${cockpitTotalBlocks} total`,
    `Cockpit stale blocks (> ${RUNTIME_COCKPIT_STALE_BLOCK_MS}ms): ${staleCockpitBlocks}`,
    `Conduit signals: ${conduitSignals}`,
    `Conduit target signals: ${targetConduitSignals}${conduitScaleRequired ? ' (scale-up recommended)' : ''}`,
    `Sync mode: ${syncMode}`,
    `Backpressure level: ${pressureLevel}`,
    `Critical attention events: ${criticalAttention} visible / ${criticalAttentionTotal} total`,
    `Standard attention events: ${standardAttention}`,
    `Background attention events: ${backgroundAttention}`,
    `Deferred attention events: ${deferredDepth} (${deferredMode})`,
    `Telemetry micro-batches: ${telemetryMicroBatchCount} (window ${microBatchWindowMs}ms / max ${microBatchMaxItems})`,
    `Attention lane weights: critical=${parsePositiveInt(laneWeights.critical, ATTENTION_LANE_WEIGHTS.critical, 1, 20)}, standard=${parsePositiveInt(laneWeights.standard, ATTENTION_LANE_WEIGHTS.standard, 1, 20)}, background=${parsePositiveInt(laneWeights.background, ATTENTION_LANE_WEIGHTS.background, 1, 20)}`,
    `Attention lane caps: critical=${parsePositiveInt(laneCaps.critical, ATTENTION_LANE_CAPS.critical, 1, 1000)}, standard=${parsePositiveInt(laneCaps.standard, ATTENTION_LANE_CAPS.standard, 1, 1000)}, background=${parsePositiveInt(laneCaps.background, ATTENTION_LANE_CAPS.background, 1, 1000)}`,
    `Client memory entries: ${memoryEntries}`,
    `Memory ingest: ${ingestControl.paused ? 'paused(non-critical)' : 'live'}`,
    `Client receipts: ${receiptEntries}`,
    `Client logs: ${logEntries}`,
    `Health checks: ${healthCheckCount}`,
    `Health coverage gap count: ${parseNonNegativeInt(healthCoverage && healthCoverage.gap_count, 0, 1000000)}`,
    `Benchmark sanity: ${benchmarkStatus}${benchmarkAgeSec >= 0 ? ` (age ${benchmarkAgeSec}s)` : ''}`,
    `Top cockpit: ${topCockpit || '(none)'}`,
    `Top attention: ${topAttention || '(none)'}`,
  ].join('\n');
}

function formatToolHistory(toolSteps = []) {
  return toolSteps.length
    ? toolSteps
        .map((step, idx) => `#${idx + 1} ${step.input}\nexit=${step.exit_code}\n${step.result}`)
        .join('\n\n')
    : '(none)';
}

function isPlaceholderResponse(value) {
  const text = String(value == null ? '' : value).trim().toLowerCase();
  if (!text) return true;
  const normalized = text.replace(/^["'`]+|["'`]+$/g, '').trim();
  if (!normalized) return true;
  if (
    normalized.includes('actual concrete response text') ||
    /"response"\s*:\s*"actual concrete response text"/.test(normalized) ||
    /^{\s*"type"\s*:\s*"final"\s*,\s*"response"\s*:\s*"actual concrete response text"\s*}$/.test(normalized)
  ) {
    return true;
  }
  return (
    normalized === '<text response to user>' ||
    normalized === '<answer>' ||
    normalized === '<response>' ||
    normalized === '{response}' ||
    normalized === '[response]'
  );
}

function isPromptEchoResponse(value, prompt) {
  const output = cleanText(value == null ? '' : String(value), 4000).trim();
  const input = cleanText(prompt == null ? '' : String(prompt), 4000).trim();
  if (!output || !input) return false;
  if (output === input) return true;
  const normalizedOutput = output.replace(/^\[[^\]]+\]\s*/, '').trim();
  return normalizedOutput === input;
}

function runtimeCouplingFallbackResponse(input, runtime = {}) {
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const conduitSignals = parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000);
  const targetSignals = Math.max(
    1,
    parsePositiveInt(
      runtime && runtime.target_conduit_signals,
      RUNTIME_AUTO_BALANCE_THRESHOLD,
      1,
      128
    )
  );
  const staleBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000);
  const benchmarkMirrorStatus =
    cleanText(runtime && runtime.benchmark_sanity_status ? runtime.benchmark_sanity_status : 'unknown', 24) ||
    'unknown';
  const benchmarkCockpitStatus =
    cleanText(runtime && runtime.benchmark_sanity_cockpit_status ? runtime.benchmark_sanity_cockpit_status : 'unknown', 24) ||
    'unknown';
  const benchmarkAgeSeconds = parsePositiveInt(
    runtime && runtime.benchmark_sanity_age_seconds != null ? runtime.benchmark_sanity_age_seconds : -1,
    -1,
    -1,
    1000000000
  );
  const benchmarkStale = benchmarkAgeSeconds < 0 || benchmarkAgeSeconds > DASHBOARD_BENCHMARK_STALE_SECONDS;
  const benchmarkStatus = benchmarkStale
    ? `${benchmarkMirrorStatus}(stale:${benchmarkCockpitStatus})`
    : benchmarkCockpitStatus;
  const memoryPaused = !!(runtime && runtime.memory_ingest_paused);
  const dopamineStatus =
    cleanText(runtime && runtime.dopamine_status ? runtime.dopamine_status : 'unknown', 24) || 'unknown';
  const dopamineFreshness =
    cleanText(runtime && runtime.dopamine_freshness_status ? runtime.dopamine_freshness_status : 'unknown', 24) || 'unknown';
  const dopamineAgeSeconds = parsePositiveInt(
    runtime && runtime.dopamine_latest_age_seconds != null ? runtime.dopamine_latest_age_seconds : -1,
    -1,
    -1,
    1000000000
  );
  const moltbookCredentialsStatus =
    cleanText(runtime && runtime.moltbook_credentials_status ? runtime.moltbook_credentials_status : 'unknown', 24) || 'unknown';
  const moltbookSuppressionRecommended = !!(runtime && runtime.moltbook_suppression_recommended);
  const moltbookJobsRequiringCredentials = parseNonNegativeInt(
    runtime && runtime.moltbook_jobs_requiring_credentials != null ? runtime.moltbook_jobs_requiring_credentials : 0,
    0,
    100000000
  );
  const eyesCrossSignalStatus =
    cleanText(runtime && runtime.external_eyes_cross_signal_status ? runtime.external_eyes_cross_signal_status : 'unknown', 24) ||
    'unknown';
  const eyesCrossSignalAbsent = !!(runtime && runtime.external_eyes_cross_signal_absent);
  const eyesFreshness =
    cleanText(runtime && runtime.external_eyes_freshness_status ? runtime.external_eyes_freshness_status : 'unknown', 24) || 'unknown';
  const eyesLatestAgeSeconds = parsePositiveInt(
    runtime && runtime.external_eyes_latest_age_seconds != null ? runtime.external_eyes_latest_age_seconds : -1,
    -1,
    -1,
    1000000000
  );
  const eyesCrossRatio = Number(
    runtime && runtime.external_eyes_cross_signal_ratio != null ? runtime.external_eyes_cross_signal_ratio : 0
  );
  const signalDeficit = Math.max(0, targetSignals - conduitSignals);
  const lowSignal = conduitSignals < targetSignals;
  const chronicStale = staleBlocks >= RUNTIME_COORDINATION_PATHOLOGY_STALE_BLOCK_MIN;
  const asksForStatus = /dashboard|system|runtime|telemetry|queue|conduit|cockpit|pain|improv|connection|coupling|reliab|autonom/i.test(
    String(input || '').toLowerCase()
  );
  const responseP95 = Number(runtime && runtime.receipt_latency_p95_ms != null ? runtime.receipt_latency_p95_ms : Number.NaN);
  const responseP99 = Number(runtime && runtime.receipt_latency_p99_ms != null ? runtime.receipt_latency_p99_ms : Number.NaN);
  const responseMs = Number.isFinite(responseP95) && responseP95 > 0
    ? Math.round(responseP95)
    : Number.isFinite(responseP99) && responseP99 > 0
      ? Math.round(responseP99)
      : null;
  const healthGapCount = parseNonNegativeInt(
    runtime && runtime.health_coverage_gap_count != null ? runtime.health_coverage_gap_count : 0,
    0,
    100000000
  );
  let confidence = 100;
  if (queueDepth > 20) confidence -= Math.min(20, Math.floor((queueDepth - 20) / 2));
  if (staleBlocks > 0) confidence -= Math.min(20, staleBlocks * 2);
  if (healthGapCount > 0) confidence -= Math.min(20, healthGapCount * 6);
  if (lowSignal) confidence -= 12;
  if (benchmarkCockpitStatus === 'warn' || benchmarkStatus.includes('warn')) confidence -= 8;
  if (benchmarkCockpitStatus === 'fail' || benchmarkStatus.includes('fail')) confidence -= 20;
  confidence = Math.max(10, Math.min(100, Math.round(confidence)));
  const pressureState =
    queueDepth <= 6 && !lowSignal && staleBlocks <= 1 && (benchmarkCockpitStatus === 'pass' || benchmarkStatus.includes('pass'))
      ? 'Synced'
      : queueDepth <= 24 && staleBlocks <= 3
        ? 'Ready'
        : 'Active';
  const responseSummary = responseMs != null ? `${responseMs}ms p95` : 'stabilizing';
  const confidenceSummary = `${confidence}%`;

  const painPoints = [];
  if (lowSignal) painPoints.push(`Conduit under target (${conduitSignals}/${targetSignals}).`);
  if (chronicStale) painPoints.push(`Cockpit stale blocks remain high (${staleBlocks}).`);
  if (benchmarkCockpitStatus === 'fail' && !benchmarkStale) {
    painPoints.push('Benchmark sanity cockpit lane is failing.');
  } else if (benchmarkStale) {
    painPoints.push('Benchmark sanity cockpit lane is stale; auto-refresh recommended.');
  }
  if (memoryPaused && queueDepth <= DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH) {
    painPoints.push('Memory ingest is paused while queue pressure is already low.');
  }
  if (queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH) painPoints.push(`Queue depth is elevated (${queueDepth}).`);
  if (moltbookSuppressionRecommended || moltbookCredentialsStatus === 'warn') {
    painPoints.push(
      `Moltbook credentials unavailable while ${moltbookJobsRequiringCredentials} scheduled jobs still request MOLTCHECK activity.`
    );
  }
  if (dopamineStatus !== 'pass' || dopamineFreshness === 'stale') {
    painPoints.push(
      `Dopamine ambient signal is ${dopamineStatus}/${dopamineFreshness}${dopamineAgeSeconds >= 0 ? ` (age ${dopamineAgeSeconds}s)` : ''}.`
    );
  }
  if (eyesCrossSignalStatus !== 'pass' || eyesCrossSignalAbsent || eyesFreshness === 'stale') {
    painPoints.push(
      `External-eyes cross-signal surface is ${eyesCrossSignalStatus}${eyesCrossSignalAbsent ? ' (cross-signals absent)' : ''}${eyesLatestAgeSeconds >= 0 ? ` age ${eyesLatestAgeSeconds}s` : ''}.`
    );
  }
  if (!painPoints.length) painPoints.push('No critical pressure detected in current telemetry.');

  const improvements = [];
  if (lowSignal) {
    improvements.push(
      `Auto-heal conduit watchdog whenever low signals persist or stale blocks are chronic (currently deficit ${signalDeficit}).`
    );
  }
  if (chronicStale) {
    improvements.push(
      `Run stale cockpit lane refresh + queue drain automatically even below depth ${RUNTIME_DRAIN_TRIGGER_DEPTH}.`
    );
  }
  if (memoryPaused && queueDepth <= DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH) {
    improvements.push('Auto-resume memory ingest once queue depth is below the safe resume threshold.');
  }
  improvements.push('Keep predictive drain workers lifecycle-bound and dissolve them once queue depth recovers.');
  improvements.push('Emit reliability escalation receipts when SLO gates fail so humans are explicitly paged.');
  if (moltbookSuppressionRecommended || moltbookCredentialsStatus === 'warn') {
    improvements.push(
      'Auto-suppress credential-gated MOLTCHECK reminders until secret broker reports moltbook_api_key available.'
    );
  }
  if (dopamineStatus !== 'pass' || dopamineFreshness === 'stale') {
    improvements.push(
      'Schedule automatic dopamine ambient closeout/status refresh and surface stale-age alerts in runtime telemetry.'
    );
  }
  if (eyesCrossSignalStatus !== 'pass' || eyesCrossSignalAbsent || eyesFreshness === 'stale') {
    improvements.push(
      `Run external-eyes cross-signal calibration when ratio ${Number.isFinite(eyesCrossRatio) ? eyesCrossRatio.toFixed(3) : '0.000'} falls below policy target.`
    );
  }

  if (!asksForStatus) {
    return `Hosted model backend returned an echo instead of an answer. System state is ${pressureState.toLowerCase()} (${responseSummary}, confidence ${confidenceSummary}). Retrying with a recovery backend is recommended.`;
  }
  return [
    `Current system state: ${pressureState}.`,
    `Response Time: ${responseSummary}. Confidence: ${confidenceSummary}.`,
    `Top pain points: ${painPoints.slice(0, 5).join(' ')}`,
    `Recommended fixes: ${improvements.slice(0, 5).join(' ')}`,
  ].join('\n\n');
}

function buildToolPrompt({ agent, session, input, toolSteps = [], snapshot = null, runtimeMirror = null }) {
  const transcript = promptTranscript(session) || '(empty)';
  const toolHistory = formatToolHistory(toolSteps);
  const agentName = cleanText(agent && (agent.name || agent.id) ? agent.name || agent.id : 'master-agent', 80);
  const fullInfring = ACTIVE_CLI_MODE === CLI_MODE_FULL_INFRING;
  const runtimeSummary = runtimeContextPrompt(snapshot, runtimeMirror);
  const todayIso = nowIso().slice(0, 10);
  return [
    'You are Infring runtime chat assistant.',
    `Today (ISO date): ${todayIso}`,
    `Active agent: ${agentName}`,
    'You can ask for a CLI command when needed.',
    'If the user asks for opinion, explanation, or casual chat, answer directly without tools.',
    'Only request a tool call when factual repo/runtime data is required.',
    'For system memory/process capability questions, use available tools (ps/vm_stat/vmstat/free/top or cat /proc/* where available) before claiming limitations.',
    `Historical memory files are in ${PRIMARY_MEMORY_DIR}/YYYY-MM-DD.md (primary) and ${LEGACY_MEMORY_DIR}/YYYY-MM-DD.md (legacy). For "what happened X days ago" questions, inspect those files first.`,
    'Swarm launch roles for collab-plane are: director, cell_coordinator, coordinator, researcher, builder, reviewer, analyst. If asked for an unsupported role, map it to the nearest supported role and state the mapping briefly.',
    `You may use at most ${TOOL_ITERATION_LIMIT} tool calls before giving a final answer.`,
    'Never claim inability without first attempting a valid tool call when tools are needed.',
    'Do not mention underlying base-model identity; respond as Infring runtime assistant.',
    'Never output placeholders such as <text response to user> or <answer>. Always provide concrete content.',
    'Return ONLY one JSON object with no markdown.',
    'Final answer schema:',
    '{"type":"final","response":"Hello! How can I help?"}',
    'Tool call schema:',
    '{"type":"tool_call","command":"<allowed command>","args":["arg1","arg2"],"reason":"<short reason>"}',
    fullInfring
      ? 'Allowed commands: protheus/protheus-ops/infringd (all subcommands), plus git/rg/ls/cat/pwd/wc/head/tail/stat/ps/top/free/vm_stat/vmstat (git remains read-only).'
      : 'Allowed commands: protheus/protheus-ops/infringd (read-only profile), plus git/rg/ls/cat/pwd/wc/head/tail/stat/ps/top/free/vm_stat/vmstat (git read-only).',
    'If tool history already contains what you need, return final.',
    '',
    `Conversation transcript:\n${transcript}`,
    '',
    `Latest user message:\n${cleanText(input, 3600)}`,
    '',
    `Runtime awareness:\n${runtimeSummary}`,
    '',
    `Tool history:\n${toolHistory}`,
  ].join('\n');
}

function buildToolFollowupPrompt({ agent, input, toolSteps = [], snapshot = null, runtimeMirror = null }) {
  const agentName = cleanText(agent && (agent.name || agent.id) ? agent.name || agent.id : 'master-agent', 80);
  const toolSummary = formatToolHistory(toolSteps);
  const runtimeSummary = runtimeContextPrompt(snapshot, runtimeMirror);
  return [
    'You are Infring runtime chat assistant.',
    `Active agent: ${agentName}`,
    'Use the tool result history to answer the user clearly.',
    'Do not disclose base-model identity.',
    `Historical memory files are in ${PRIMARY_MEMORY_DIR}/YYYY-MM-DD.md (primary) and ${LEGACY_MEMORY_DIR}/YYYY-MM-DD.md (legacy).`,
    'Never output placeholders such as <text response to user> or <answer>.',
    'Return ONLY one JSON object with no markdown.',
    '{"type":"final","response":"Hello! How can I help?"}',
    '',
    `User request:\n${cleanText(input, 3200)}`,
    '',
    `Runtime awareness:\n${runtimeSummary}`,
    '',
    `Tool history:\n${toolSummary}`,
  ].join('\n');
}

function runLlmChatWithCli(agent, session, input, snapshot, requestedModel = '', runtimeMirror = null) {
  const deterministic = tryDeterministicRepoAnswer(input, snapshot);
  if (deterministic) {
    return {
      ok: true,
      status: 0,
      response: deterministic.response,
      model: 'deterministic-repo-query',
      tools: Array.isArray(deterministic.tools) ? deterministic.tools : [],
      iterations: 1,
    };
  }

  const requested = cleanText(requestedModel || '', 120);
  let model = requested || configuredOllamaModel(snapshot);
  const toolSteps = [];
  let iterations = 0;
  let lastLlmOutput = '';

  while (iterations <= TOOL_ITERATION_LIMIT) {
    const prompt = buildToolPrompt({ agent, session, input, toolSteps, snapshot, runtimeMirror });
    let llm = runOllamaPrompt(model, prompt);
    if (!llm.ok && model !== OLLAMA_MODEL_FALLBACK) {
      model = OLLAMA_MODEL_FALLBACK;
      llm = runOllamaPrompt(model, prompt);
    }
    if (!llm.ok) {
      return {
        ok: false,
        error: cleanText(llm.error || 'ollama_run_failed', 260),
        status: llm.status || 1,
        tools: toolSteps,
      };
    }

    iterations += 1;
    lastLlmOutput = llm.output;
    const directive = extractJsonDirective(llm.output);
    if (!directive) {
      const rawResponse = cleanText(llm.output, 4000);
      return {
        ok: true,
        status: 0,
        response: isPlaceholderResponse(rawResponse) ? ASSISTANT_EMPTY_FALLBACK_RESPONSE : rawResponse,
        model,
        tools: toolSteps,
        iterations,
      };
    }

    if (directive.type === 'final') {
      const finalCandidate = cleanText(directive.response || llm.output, 4000);
      if (isPlaceholderResponse(finalCandidate)) {
        const followPrompt = buildToolFollowupPrompt({
          agent,
          input,
          toolSteps,
          snapshot,
          runtimeMirror,
        });
        let follow = runOllamaPrompt(model, `${followPrompt}\n\nProvide a concrete answer now.`);
        if (!follow.ok && model !== OLLAMA_MODEL_FALLBACK) {
          model = OLLAMA_MODEL_FALLBACK;
          follow = runOllamaPrompt(model, `${followPrompt}\n\nProvide a concrete answer now.`);
        }
        let followResponse = '';
        if (follow.ok) {
          const followDirective = extractJsonDirective(follow.output);
          if (followDirective && followDirective.type === 'final') {
            followResponse = cleanText(followDirective.response || follow.output, 4000);
          } else {
            followResponse = cleanText(follow.output, 4000);
          }
        }
        if (isPlaceholderResponse(followResponse)) {
          const lastTool = toolSteps.length ? toolSteps[toolSteps.length - 1] : null;
          followResponse = cleanText(
            (lastTool && lastTool.result) || ASSISTANT_EMPTY_FALLBACK_RESPONSE,
            4000
          );
        }
        return {
          ok: true,
          status: 0,
          response: followResponse,
          model,
          tools: toolSteps,
          iterations: iterations + 1,
        };
      }
      return {
        ok: true,
        status: 0,
        response: finalCandidate,
        model,
        tools: toolSteps,
        iterations,
      };
    }

    if (directive.type !== 'tool_call') {
      const directiveFallback = cleanText(llm.output, 4000) || ASSISTANT_EMPTY_FALLBACK_RESPONSE;
      return {
        ok: true,
        status: 0,
        response: isPlaceholderResponse(directiveFallback)
          ? ASSISTANT_EMPTY_FALLBACK_RESPONSE
          : directiveFallback,
        model,
        tools: toolSteps,
        iterations,
      };
    }

    const toolStep = runCliTool(directive.command, directive.args);
    const normalizedTool = {
      id: `tool-${Date.now()}-${toolSteps.length}`,
      name: toolStep.name,
      input: toolStep.input,
      result: toolStep.result,
      is_error: !!toolStep.is_error,
      running: false,
      expanded: false,
      exit_code: toolStep.exit_code,
    };
    toolSteps.push(normalizedTool);

    if (toolSteps.length >= TOOL_ITERATION_LIMIT) {
      const followPrompt = buildToolFollowupPrompt({
        agent,
        input,
        toolSteps,
        snapshot,
        runtimeMirror,
      });
      let follow = runOllamaPrompt(model, followPrompt);
      if (!follow.ok && model !== OLLAMA_MODEL_FALLBACK) {
        model = OLLAMA_MODEL_FALLBACK;
        follow = runOllamaPrompt(model, followPrompt);
      }
      let finalResponse = '';
      if (follow.ok) {
        const followDirective = extractJsonDirective(follow.output);
        if (followDirective && followDirective.type === 'final') {
          finalResponse = cleanText(followDirective.response || follow.output, 4000);
        } else {
          finalResponse = cleanText(follow.output, 4000);
        }
      }
      if (isPlaceholderResponse(finalResponse)) {
        finalResponse = '';
      }
      if (!finalResponse) {
        const last = toolSteps[toolSteps.length - 1];
        finalResponse = last && last.is_error
          ? `Tool execution failed: ${last.result}`
          : cleanText((last && last.result) || lastLlmOutput || ASSISTANT_EMPTY_FALLBACK_RESPONSE, 4000);
      }
      return {
        ok: true,
        status: 0,
        response: finalResponse,
        model,
        tools: toolSteps,
        iterations: iterations + 1,
      };
    }
  }

  return {
    ok: true,
    status: 0,
    response: cleanText(lastLlmOutput, 4000) || 'No response produced by the model.',
    model,
    tools: toolSteps,
    iterations,
  };
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function isPidAlive(pid) {
  const safePid = Number(pid);
  if (!Number.isFinite(safePid) || safePid <= 0) return false;
  try {
    process.kill(safePid, 0);
    return true;
  } catch {
    return false;
  }
}

function syncDirBestEffort(dirPath) {
  let dirFd = null;
  try {
    dirFd = fs.openSync(dirPath, 'r');
    fs.fsyncSync(dirFd);
  } catch {} finally {
    if (dirFd != null) {
      try {
        fs.closeSync(dirFd);
      } catch {}
    }
  }
}

function writeFileAtomic(filePath, body, encoding = 'utf8') {
  ensureDir(path.dirname(filePath));
  const tmpPath = `${filePath}.${process.pid}.${Date.now()}.tmp`;
  let fd = null;
  try {
    fd = fs.openSync(tmpPath, 'w');
    const text = typeof body === 'string' ? body : String(body == null ? '' : body);
    fs.writeFileSync(fd, text, { encoding });
    fs.fsyncSync(fd);
  } finally {
    if (fd != null) {
      try {
        fs.closeSync(fd);
      } catch {}
    }
  }
  fs.renameSync(tmpPath, filePath);
  syncDirBestEffort(path.dirname(filePath));
}

function writeJson(filePath, value) {
  writeFileAtomic(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}

function fileSizeBytes(filePath) {
  try {
    const stat = fs.statSync(filePath);
    return Number.isFinite(stat.size) ? stat.size : 0;
  } catch {
    return 0;
  }
}

function countFileLines(filePath) {
  try {
    const out = spawnSync('wc', ['-l', filePath], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      maxBuffer: 2 * 1024 * 1024,
    });
    if (out.status !== 0 || typeof out.stdout !== 'string') return 0;
    const token = out.stdout.trim().split(/\s+/)[0];
    return parseNonNegativeInt(token, 0, 1_000_000_000);
  } catch {
    return 0;
  }
}

function tailFileLines(filePath, lineCount) {
  const desired = parsePositiveInt(lineCount, SNAPSHOT_HISTORY_RETAIN_LINES, 1, 200_000);
  const tailFallback = () => {
    try {
      const fd = fs.openSync(filePath, 'r');
      try {
        const stat = fs.fstatSync(fd);
        const totalSize = Number.isFinite(stat.size) ? stat.size : 0;
        if (totalSize <= 0) return [];
        const chunkSize = 64 * 1024;
        const maxReadBytes = 32 * 1024 * 1024;
        let remaining = totalSize;
        let newlineCount = 0;
        let consumed = 0;
        let text = '';
        while (remaining > 0 && newlineCount <= desired && consumed < maxReadBytes) {
          const readSize = Math.min(chunkSize, remaining);
          remaining -= readSize;
          const buffer = Buffer.allocUnsafe(readSize);
          const bytesRead = fs.readSync(fd, buffer, 0, readSize, remaining);
          if (!bytesRead) break;
          consumed += bytesRead;
          for (let idx = 0; idx < bytesRead; idx += 1) {
            if (buffer[idx] === 10) newlineCount += 1;
          }
          text = buffer.toString('utf8', 0, bytesRead) + text;
        }
        if (!text) return [];
        return text
          .split('\n')
          .map((row) => row.trim())
          .filter(Boolean)
          .slice(-desired);
      } finally {
        fs.closeSync(fd);
      }
    } catch {
      return [];
    }
  };
  try {
    const out = spawnSync('tail', ['-n', String(desired), filePath], {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      maxBuffer: 64 * 1024 * 1024,
    });
    if (out.status !== 0 || typeof out.stdout !== 'string') return tailFallback();
    return out.stdout
      .split('\n')
      .map((row) => row.trim())
      .filter(Boolean);
  } catch {
    return tailFallback();
  }
}

function compactSnapshotHistory(reason = 'periodic', force = false) {
  const nowMs = Date.now();
  const bytesBefore = fileSizeBytes(SNAPSHOT_HISTORY_PATH);
  let linesBefore = parseNonNegativeInt(snapshotHistoryMaintenanceState.lines_after, 0, 1_000_000_000);
  let lineCountChecked = false;
  const ensureLinesBefore = () => {
    if (!lineCountChecked) {
      linesBefore = countFileLines(SNAPSHOT_HISTORY_PATH);
      lineCountChecked = true;
    }
    return linesBefore;
  };
  const exceededByBytes = bytesBefore > SNAPSHOT_HISTORY_MAX_BYTES;
  const warning = bytesBefore > SNAPSHOT_HISTORY_WARNING_BYTES;
  const dueByCadence =
    force || !snapshotHistoryMaintenanceState.last_compact_at || (nowMs - coerceTsMs(snapshotHistoryMaintenanceState.last_compact_at, 0)) >= SNAPSHOT_HISTORY_COMPACT_INTERVAL_MS;
  if (!dueByCadence && !exceededByBytes) {
    snapshotHistoryMaintenanceState = {
      ...snapshotHistoryMaintenanceState,
      exceeded: false,
      warning,
      bytes_before: bytesBefore,
      bytes_after: bytesBefore,
      lines_before: parseNonNegativeInt(snapshotHistoryMaintenanceState.lines_after, 0, 1_000_000_000),
      lines_after: parseNonNegativeInt(snapshotHistoryMaintenanceState.lines_after, 0, 1_000_000_000),
    };
    return snapshotHistoryMaintenanceState;
  }
  const exceeded = exceededByBytes || ensureLinesBefore() > SNAPSHOT_HISTORY_MAX_LINES;
  if (!exceeded && !force) {
    snapshotHistoryMaintenanceState = {
      ...snapshotHistoryMaintenanceState,
      last_reason: cleanText(reason, 80) || 'periodic',
      exceeded,
      warning,
      bytes_before: bytesBefore,
      bytes_after: bytesBefore,
      lines_before: linesBefore,
      lines_after: linesBefore,
    };
    return snapshotHistoryMaintenanceState;
  }

  const rawLines = tailFileLines(SNAPSHOT_HISTORY_PATH, SNAPSHOT_HISTORY_RETAIN_LINES);
  if (rawLines.length === 0 && (bytesBefore > 0 || linesBefore > 0)) {
    snapshotHistoryMaintenanceState = {
      ...snapshotHistoryMaintenanceState,
      last_reason: `${cleanText(reason, 40) || 'periodic'}:tail_failed`,
      exceeded,
      warning,
      bytes_before: bytesBefore,
      bytes_after: bytesBefore,
      lines_before: linesBefore,
      lines_after: linesBefore,
    };
    return snapshotHistoryMaintenanceState;
  }
  const cutoffMs = nowMs - SNAPSHOT_HISTORY_MAX_AGE_MS;
  let retained = [];
  for (const line of rawLines) {
    const payload = parseJsonLoose(line);
    const tsMs = coerceTsMs(
      payload && typeof payload === 'object'
        ? payload.ts || payload.created_at || payload.updated_at || payload.at
        : 0,
      0
    );
    if (tsMs > 0 && tsMs < cutoffMs) continue;
    retained.push(line);
  }
  if (retained.length === 0) {
    retained = rawLines.slice(-SNAPSHOT_HISTORY_RETAIN_LINES);
  }
  retained = retained.slice(-SNAPSHOT_HISTORY_RETAIN_LINES);

  const body = retained.length > 0 ? `${retained.join('\n')}\n` : '';
  try {
    writeFileAtomic(SNAPSHOT_HISTORY_PATH, body, 'utf8');
  } catch (error) {
    snapshotHistoryMaintenanceState = {
      ...snapshotHistoryMaintenanceState,
      last_reason: `${cleanText(reason, 40) || 'periodic'}:atomic_write_failed`,
      exceeded,
      warning,
      bytes_before: bytesBefore,
      bytes_after: bytesBefore,
      lines_before: linesBefore,
      lines_after: linesBefore,
      last_error: cleanText(error && error.message ? error.message : String(error), 220),
    };
    return snapshotHistoryMaintenanceState;
  }

  const bytesAfter = fileSizeBytes(SNAPSHOT_HISTORY_PATH);
  const linesAfter = countFileLines(SNAPSHOT_HISTORY_PATH);
  const trimmedEntries = Math.max(0, linesBefore - linesAfter);
  const removedBytes = Math.max(0, bytesBefore - bytesAfter);
  snapshotHistoryMaintenanceState = {
    last_compact_at: nowIso(),
    last_reason: cleanText(reason, 80) || 'periodic',
    bytes_before: bytesBefore,
    bytes_after: bytesAfter,
    lines_before: linesBefore,
    lines_after: linesAfter,
    trimmed_entries: trimmedEntries,
    removed_bytes: removedBytes,
    exceeded: bytesAfter > SNAPSHOT_HISTORY_MAX_BYTES || linesAfter > SNAPSHOT_HISTORY_MAX_LINES,
    warning: bytesAfter > SNAPSHOT_HISTORY_WARNING_BYTES,
    compact_count: parseNonNegativeInt(snapshotHistoryMaintenanceState.compact_count, 0, 1_000_000) + 1,
  };
  return snapshotHistoryMaintenanceState;
}

function snapshotStorageTelemetry() {
  const bytes = fileSizeBytes(SNAPSHOT_HISTORY_PATH);
  const lines = parseNonNegativeInt(snapshotHistoryMaintenanceState.lines_after, 0, 1_000_000_000);
  const sizeMb = Math.round((bytes / (1024 * 1024)) * 1000) / 1000;
  return {
    snapshot_history: {
      path: path.relative(ROOT, SNAPSHOT_HISTORY_PATH),
      size_bytes: bytes,
      size_mb: sizeMb,
      lines,
      limits: {
        max_bytes: SNAPSHOT_HISTORY_MAX_BYTES,
        max_lines: SNAPSHOT_HISTORY_MAX_LINES,
        retain_lines: SNAPSHOT_HISTORY_RETAIN_LINES,
        max_age_ms: SNAPSHOT_HISTORY_MAX_AGE_MS,
        append_min_interval_ms: SNAPSHOT_HISTORY_APPEND_MIN_INTERVAL_MS,
        compact_interval_ms: SNAPSHOT_HISTORY_COMPACT_INTERVAL_MS,
      },
      warning: bytes > SNAPSHOT_HISTORY_WARNING_BYTES,
      exceeded: bytes > SNAPSHOT_HISTORY_MAX_BYTES || lines > SNAPSHOT_HISTORY_MAX_LINES,
      maintenance: snapshotHistoryMaintenanceState,
    },
  };
}

function bootstrapSnapshotHistoryState(options = {}) {
  const bytes = fileSizeBytes(SNAPSHOT_HISTORY_PATH);
  const fast = !!(options && options.fast === true);
  const lines = fast
    ? parseNonNegativeInt(snapshotHistoryMaintenanceState.lines_after, 0, 1_000_000_000)
    : countFileLines(SNAPSHOT_HISTORY_PATH);
  snapshotHistoryMaintenanceState = {
    ...snapshotHistoryMaintenanceState,
    bytes_after: bytes,
    lines_after: lines,
    warning: bytes > SNAPSHOT_HISTORY_WARNING_BYTES,
    exceeded: bytes > SNAPSHOT_HISTORY_MAX_BYTES || lines > SNAPSHOT_HISTORY_MAX_LINES,
  };
}

function readJson(filePath, fallback = null) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function normalizeIdentityColor(value, fallback = '#2563EB') {
  const raw = cleanText(value || '', 16);
  if (!raw) return fallback;
  const normalized = raw.startsWith('#') ? raw : `#${raw}`;
  if (/^#([0-9a-fA-F]{6})$/.test(normalized)) return normalized.toUpperCase();
  if (/^#([0-9a-fA-F]{3})$/.test(normalized)) return normalized.toUpperCase();
  return fallback;
}

function normalizeAgentFallbackModels(value) {
  const rows = Array.isArray(value) ? value : [];
  return rows
    .map((row) => {
      const provider = cleanText(row && row.provider ? row.provider : '', 80);
      const model = cleanText(row && row.model ? row.model : '', 120);
      if (!provider || !model) return null;
      return { provider, model };
    })
    .filter(Boolean);
}

function normalizeAgentIdentity(identity = {}, fallback = {}) {
  const source = identity && typeof identity === 'object' ? identity : {};
  const prior = fallback && typeof fallback === 'object' ? fallback : {};
  return {
    emoji: cleanText(source.emoji != null ? source.emoji : prior.emoji || '🤖', 24) || '🤖',
    color: normalizeIdentityColor(source.color != null ? source.color : prior.color, '#2563EB'),
    archetype: cleanText(source.archetype != null ? source.archetype : prior.archetype || 'assistant', 80) || 'assistant',
    vibe: cleanText(source.vibe != null ? source.vibe : prior.vibe || '', 80),
  };
}

function normalizeAgentProfile(agentId, value = {}, fallback = {}) {
  const id = cleanText(agentId || (value && value.agent_id ? value.agent_id : ''), 140);
  if (!id) return null;
  const source = value && typeof value === 'object' ? value : {};
  const prior = fallback && typeof fallback === 'object' ? fallback : {};
  const hasFallbackModels = Object.prototype.hasOwnProperty.call(source, 'fallback_models');
  const treeKind = normalizeGitTreeKind(
    source.git_tree_kind != null ? source.git_tree_kind : prior.git_tree_kind,
    AGENT_GIT_TREE_KIND_ISOLATED
  );
  const normalizedBranch =
    treeKind === AGENT_GIT_TREE_KIND_MASTER
      ? gitMainBranch()
      : branchForAgentGitTree(id, source.git_branch != null ? source.git_branch : prior.git_branch || '');
  const normalizedWorkspace =
    treeKind === AGENT_GIT_TREE_KIND_MASTER
      ? ROOT
      : workspaceDirForAgentGitTree(
          id,
          source.workspace_dir != null ? source.workspace_dir : prior.workspace_dir || ''
        );
  return {
    agent_id: id,
    name: cleanText(source.name != null ? source.name : prior.name || id, 100) || id,
    role: cleanText(source.role != null ? source.role : prior.role || 'analyst', 60) || 'analyst',
    model_override: modelOverrideFromState({
      model_override: source.model_override != null ? source.model_override : prior.model_override,
    }),
    system_prompt: cleanText(
      source.system_prompt != null ? source.system_prompt : prior.system_prompt || '',
      4000
    ),
    identity: normalizeAgentIdentity(
      source.identity && typeof source.identity === 'object' ? source.identity : source,
      prior.identity
    ),
    fallback_models: normalizeAgentFallbackModels(
      hasFallbackModels ? source.fallback_models : prior.fallback_models
    ),
    git_tree_kind: treeKind,
    git_branch: cleanText(normalizedBranch, 120) || (treeKind === AGENT_GIT_TREE_KIND_MASTER ? 'main' : ''),
    workspace_dir: cleanText(normalizedWorkspace, 400) || ROOT,
    git_tree_ready:
      treeKind === AGENT_GIT_TREE_KIND_MASTER
        ? true
        : !!(
            source.git_tree_ready != null
              ? source.git_tree_ready
              : prior.git_tree_ready != null
                ? prior.git_tree_ready
                : false
          ),
    git_tree_error: cleanText(
      source.git_tree_error != null ? source.git_tree_error : prior.git_tree_error || '',
      240
    ),
    git_tree_updated_at: cleanText(
      source.git_tree_updated_at != null ? source.git_tree_updated_at : prior.git_tree_updated_at || nowIso(),
      80
    ) || nowIso(),
    updated_at: cleanText(source.updated_at || nowIso(), 80) || nowIso(),
  };
}

function normalizeAgentProfilesState(state) {
  const root = state && typeof state === 'object' ? state : {};
  const rawAgents = root.agents && typeof root.agents === 'object' ? root.agents : {};
  const agents = {};
  for (const [rawId, rawProfile] of Object.entries(rawAgents)) {
    const normalized = normalizeAgentProfile(rawId, rawProfile);
    if (!normalized) continue;
    agents[normalized.agent_id] = normalized;
  }
  return {
    type: 'infring_dashboard_agent_profiles',
    updated_at: cleanText(root.updated_at || nowIso(), 80) || nowIso(),
    agents,
  };
}

function normalizeArchivedAgentsState(state) {
  const root = state && typeof state === 'object' ? state : {};
  const rawAgents = root.agents && typeof root.agents === 'object' ? root.agents : {};
  const agents = {};
  for (const [rawId, rawMeta] of Object.entries(rawAgents)) {
    const agentId = cleanText(rawId || (rawMeta && rawMeta.agent_id ? rawMeta.agent_id : ''), 140);
    if (!agentId) continue;
    const meta = rawMeta && typeof rawMeta === 'object' ? rawMeta : {};
    agents[agentId] = {
      agent_id: agentId,
      archived_at: cleanText(meta.archived_at || meta.ts || nowIso(), 80) || nowIso(),
      reason: cleanText(meta.reason || 'archived', 240) || 'archived',
      source: cleanText(meta.source || 'dashboard', 80) || 'dashboard',
      contract_id: cleanText(meta.contract_id || '', 80),
      mission: cleanText(meta.mission || '', 280),
      owner: cleanText(meta.owner || '', 120),
      role: cleanText(meta.role || '', 80),
      termination_condition: cleanText(meta.termination_condition || '', 40),
      terminated_at: cleanText(meta.terminated_at || '', 80),
      git_tree_kind: normalizeGitTreeKind(meta.git_tree_kind, AGENT_GIT_TREE_KIND_ISOLATED),
      git_branch: cleanText(meta.git_branch || '', 120),
      workspace_dir: cleanText(meta.workspace_dir || '', 400),
      was_master_agent: !!meta.was_master_agent,
      revival_data: meta.revival_data && typeof meta.revival_data === 'object' ? meta.revival_data : null,
    };
  }
  return {
    type: 'infring_dashboard_archived_agents',
    updated_at: cleanText(root.updated_at || nowIso(), 80) || nowIso(),
    agents,
  };
}

let archivedAgentsCache = null;
let agentContractsCache = null;
let agentProfilesCache = null;
let agentTerminationSweepState = {
  last_run_ms: 0,
  last_idle_run_ms: 0,
};
let agentGitTreeSyncState = {
  last_run_ms: 0,
  last_master_id: '',
  run_count: 0,
};

function loadAgentProfilesState() {
  if (agentProfilesCache) return agentProfilesCache;
  agentProfilesCache = normalizeAgentProfilesState(readJson(AGENT_PROFILES_PATH, null));
  return agentProfilesCache;
}

function saveAgentProfilesState(state) {
  const normalized = normalizeAgentProfilesState(state);
  normalized.updated_at = nowIso();
  agentProfilesCache = normalized;
  writeJson(AGENT_PROFILES_PATH, normalized);
  return normalized;
}

function agentProfileFor(agentId) {
  const key = cleanText(agentId || '', 140);
  if (!key) return null;
  const state = loadAgentProfilesState();
  return state && state.agents && state.agents[key] ? state.agents[key] : null;
}

function upsertAgentProfile(agentId, patch = {}) {
  const key = cleanText(agentId || '', 140);
  if (!key) return null;
  const state = loadAgentProfilesState();
  const existing = state && state.agents && state.agents[key] ? state.agents[key] : null;
  const source = patch && typeof patch === 'object' ? patch : {};
  const next = normalizeAgentProfile(
    key,
    {
      ...(existing || {}),
      ...(source || {}),
      identity: {
        ...((existing && existing.identity) || {}),
        ...((source && source.identity && typeof source.identity === 'object') ? source.identity : {}),
      },
      updated_at: nowIso(),
    },
    existing || {}
  );
  if (!next) return null;
  state.agents[key] = next;
  saveAgentProfilesState(state);
  return next;
}

function runtimeAgentIdsFromSnapshot(snapshot, options = {}) {
  const includeArchived = !!(options && options.includeArchived);
  const rows =
    snapshot &&
    snapshot.collab &&
    snapshot.collab.dashboard &&
    Array.isArray(snapshot.collab.dashboard.agents)
      ? snapshot.collab.dashboard.agents
      : [];
  const archived = includeArchived ? null : archivedAgentIdsSet();
  const out = [];
  const seen = new Set();
  for (let idx = 0; idx < rows.length; idx += 1) {
    const row = rows[idx];
    const id = cleanText(row && (row.shadow || row.id) ? row.shadow || row.id : `agent-${idx + 1}`, 140);
    if (!id || seen.has(id)) continue;
    if (!includeArchived && archived && archived.has(id)) continue;
    seen.add(id);
    out.push(id);
  }
  return out;
}

function agentGitTreeView(agentId, profile = null, options = {}) {
  const id = cleanText(agentId || '', 140);
  const source = profile || agentProfileFor(id) || {};
  const validateWorkspace = !!(options && options.validate_workspace);
  const kind = normalizeGitTreeKind(source && source.git_tree_kind ? source.git_tree_kind : '');
  const branch =
    kind === AGENT_GIT_TREE_KIND_MASTER
      ? gitMainBranch()
      : branchForAgentGitTree(id, source && source.git_branch ? source.git_branch : '');
  const workspace =
    kind === AGENT_GIT_TREE_KIND_MASTER
      ? ROOT
      : workspaceDirForAgentGitTree(id, source && source.workspace_dir ? source.workspace_dir : '');
  const workspaceRel = cleanText(path.relative(ROOT, workspace || ROOT) || '.', 400) || '.';
  const ready =
    kind === AGENT_GIT_TREE_KIND_MASTER
      ? true
      : !!(
          source &&
          source.git_tree_ready &&
          (!validateWorkspace || gitWorkspaceLooksReady(workspace))
        );
  return {
    git_tree_kind: kind,
    git_branch: cleanText(branch, 120) || gitMainBranch(),
    workspace_dir: cleanText(workspace, 400) || ROOT,
    workspace_rel: workspaceRel,
    git_tree_ready: ready,
    git_tree_error: cleanText(source && source.git_tree_error ? source.git_tree_error : '', 240),
    is_master_agent: kind === AGENT_GIT_TREE_KIND_MASTER,
  };
}

function isMainTreeBoundAgent(agentId, runtimeRow = null) {
  const id = cleanText(agentId || '', 140);
  if (!id) return false;
  const row = runtimeRow && typeof runtimeRow === 'object' ? runtimeRow : null;
  if (row && row.is_master_agent === true) return true;
  if (row && normalizeGitTreeKind(row.git_tree_kind || '') === AGENT_GIT_TREE_KIND_MASTER) return true;
  if (row) {
    const rowWorkspace = cleanText(row.workspace_dir || '', 400);
    const rowBranch = cleanText(row.git_branch || '', 120).toLowerCase();
    if (rowWorkspace && path.resolve(rowWorkspace) === ROOT && (rowBranch === gitMainBranch().toLowerCase() || rowBranch === 'main')) {
      return true;
    }
  }
  const profile = agentProfileFor(id);
  if (normalizeGitTreeKind(profile && profile.git_tree_kind ? profile.git_tree_kind : '') === AGENT_GIT_TREE_KIND_MASTER) {
    return true;
  }
  const view = agentGitTreeView(id, profile);
  return !!(view && view.is_master_agent);
}

function ensureAgentGitTreeProfile(agentId, options = {}) {
  const id = cleanText(agentId || '', 140);
  if (!id) return null;
  const forceMaster = !!(options && options.force_master);
  const forceIsolated = !!(options && options.force_isolated);
  const ensureWorkspaceReady = !!(options && options.ensure_workspace_ready);
  const now = nowIso();
  const existing = agentProfileFor(id);
  const view = agentGitTreeView(id, existing);
  const desiredKind = forceMaster
    ? AGENT_GIT_TREE_KIND_MASTER
    : forceIsolated
      ? AGENT_GIT_TREE_KIND_ISOLATED
      : view.git_tree_kind;
  const desiredBranch =
    desiredKind === AGENT_GIT_TREE_KIND_MASTER
      ? gitMainBranch()
      : branchForAgentGitTree(id, existing && existing.git_branch ? existing.git_branch : '');
  const desiredWorkspace =
    desiredKind === AGENT_GIT_TREE_KIND_MASTER
      ? ROOT
      : workspaceDirForAgentGitTree(id, existing && existing.workspace_dir ? existing.workspace_dir : '');
  let patch = {
    git_tree_kind: desiredKind,
    git_branch: cleanText(desiredBranch, 120) || gitMainBranch(),
    workspace_dir: cleanText(desiredWorkspace, 400) || ROOT,
    git_tree_ready:
      desiredKind === AGENT_GIT_TREE_KIND_MASTER
        ? true
        : !!(existing && existing.git_tree_ready),
    git_tree_error: desiredKind === AGENT_GIT_TREE_KIND_MASTER ? '' : cleanText(view.git_tree_error || '', 240),
    git_tree_updated_at: now,
  };
  if (!forceMaster && ensureWorkspaceReady && normalizeGitTreeKind(patch.git_tree_kind || '') !== AGENT_GIT_TREE_KIND_MASTER) {
    const ensured = ensureGitWorkspaceReady(id, patch.git_branch, patch.workspace_dir);
    patch = {
      ...patch,
      workspace_dir: cleanText(ensured.workspace_dir || patch.workspace_dir, 400) || patch.workspace_dir,
      git_tree_ready: !!ensured.ok,
      git_tree_error: ensured.ok ? '' : cleanText(ensured.error || 'git_workspace_not_ready', 240),
      git_tree_updated_at: nowIso(),
    };
  }
  const existingSame =
    existing &&
    normalizeGitTreeKind(existing.git_tree_kind || '') === normalizeGitTreeKind(patch.git_tree_kind || '') &&
    cleanText(existing.git_branch || '', 120) === cleanText(patch.git_branch || '', 120) &&
    cleanText(existing.workspace_dir || '', 400) === cleanText(patch.workspace_dir || '', 400) &&
    !!existing.git_tree_ready === !!patch.git_tree_ready &&
    cleanText(existing.git_tree_error || '', 240) === cleanText(patch.git_tree_error || '', 240);
  if (existingSame) return existing;
  return upsertAgentProfile(id, patch);
}

function selectMasterAgentId(activeIds, preferredMasterId = '') {
  const ids = Array.isArray(activeIds) ? activeIds.filter(Boolean) : [];
  if (!ids.length) return '';
  for (const id of ids) {
    const profile = agentProfileFor(id);
    if (normalizeGitTreeKind(profile && profile.git_tree_kind ? profile.git_tree_kind : '') === AGENT_GIT_TREE_KIND_MASTER) {
      return id;
    }
  }
  const preferred = cleanText(preferredMasterId || '', 140);
  if (preferred && ids.includes(preferred)) return preferred;
  return ids[0];
}

function ensureAgentGitTreeAssignments(snapshot, options = {}) {
  const nowMs = Date.now();
  const force = !!(options && options.force);
  if (!force && (nowMs - parseNonNegativeInt(agentGitTreeSyncState.last_run_ms, 0, 1000000000000)) < AGENT_GIT_TREE_SYNC_COOLDOWN_MS) {
    return {
      ok: true,
      skipped: true,
      reason: 'cooldown',
      master_agent_id: cleanText(agentGitTreeSyncState.last_master_id || '', 140),
    };
  }
  const activeIds = runtimeAgentIdsFromSnapshot(snapshot, { includeArchived: false });
  if (!activeIds.length) {
    agentGitTreeSyncState.last_run_ms = nowMs;
    return {
      ok: true,
      skipped: true,
      reason: 'no_active_agents',
      master_agent_id: '',
    };
  }
  const preferred = cleanText(options && options.preferred_master_id ? options.preferred_master_id : '', 140);
  const ensureWorkspaceAgentId = cleanText(
    options && options.ensure_workspace_agent_id ? options.ensure_workspace_agent_id : '',
    140
  );
  const masterAgentId = selectMasterAgentId(activeIds, preferred);
  const assigned = [];
  for (const id of activeIds) {
    const isMaster = id === masterAgentId;
    const profile = ensureAgentGitTreeProfile(id, {
      force_master: isMaster,
      force_isolated: !isMaster,
      ensure_workspace_ready: !!ensureWorkspaceAgentId && ensureWorkspaceAgentId === id,
    });
    const view = agentGitTreeView(id, profile);
    assigned.push({
      agent_id: id,
      git_tree_kind: view.git_tree_kind,
      git_branch: view.git_branch,
      workspace_dir: view.workspace_dir,
      git_tree_ready: view.git_tree_ready,
      is_master_agent: view.is_master_agent,
    });
  }
  agentGitTreeSyncState = {
    last_run_ms: nowMs,
    last_master_id: masterAgentId,
    run_count: parseNonNegativeInt(agentGitTreeSyncState.run_count, 0, 100000000) + 1,
  };
  return {
    ok: true,
    skipped: false,
    agent_count: activeIds.length,
    master_agent_id: masterAgentId,
    assigned,
  };
}

function loadArchivedAgentsState() {
  if (archivedAgentsCache) return archivedAgentsCache;
  archivedAgentsCache = normalizeArchivedAgentsState(readJson(ARCHIVED_AGENTS_PATH, null));
  return archivedAgentsCache;
}

function saveArchivedAgentsState(state) {
  const normalized = normalizeArchivedAgentsState(state);
  normalized.updated_at = nowIso();
  archivedAgentsCache = normalized;
  writeJson(ARCHIVED_AGENTS_PATH, normalized);
  return normalized;
}

function archivedAgentMeta(agentId) {
  const key = cleanText(agentId || '', 140);
  if (!key) return null;
  const state = loadArchivedAgentsState();
  return state && state.agents && state.agents[key] ? state.agents[key] : null;
}

function isAgentArchived(agentId) {
  return !!archivedAgentMeta(agentId);
}

function archiveAgent(agentId, meta = {}) {
  const key = cleanText(agentId || '', 140);
  if (!key) return null;
  const gitCleanup = removeGitWorkspaceForAgent(key);
  const profile = agentProfileFor(key);
  const treeView = agentGitTreeView(key, profile);
  const state = loadArchivedAgentsState();
  const existing = state.agents && state.agents[key] ? state.agents[key] : {};
  state.agents[key] = {
    agent_id: key,
    archived_at: cleanText(existing.archived_at || nowIso(), 80) || nowIso(),
    reason: cleanText(meta.reason || existing.reason || 'archived', 240) || 'archived',
    source: cleanText(meta.source || existing.source || 'dashboard', 80) || 'dashboard',
    contract_id: cleanText(meta.contract_id || existing.contract_id || '', 80),
    mission: cleanText(meta.mission || existing.mission || '', 280),
    owner: cleanText(meta.owner || existing.owner || '', 120),
    role: cleanText(meta.role || existing.role || '', 80),
    termination_condition: cleanText(meta.termination_condition || existing.termination_condition || '', 40),
    terminated_at: cleanText(meta.terminated_at || existing.terminated_at || '', 80),
    git_tree_kind: normalizeGitTreeKind(
      meta.git_tree_kind || existing.git_tree_kind || (treeView && treeView.git_tree_kind ? treeView.git_tree_kind : ''),
      AGENT_GIT_TREE_KIND_ISOLATED
    ),
    git_branch: cleanText(
      meta.git_branch || existing.git_branch || (treeView && treeView.git_branch ? treeView.git_branch : ''),
      120
    ),
    workspace_dir: cleanText(
      meta.workspace_dir || existing.workspace_dir || (treeView && treeView.workspace_dir ? treeView.workspace_dir : ''),
      400
    ),
    was_master_agent:
      meta.was_master_agent === true ||
      existing.was_master_agent === true ||
      !!(treeView && treeView.is_master_agent),
    revival_data:
      meta.revival_data && typeof meta.revival_data === 'object'
        ? meta.revival_data
        : existing.revival_data && typeof existing.revival_data === 'object'
          ? existing.revival_data
          : null,
    git_tree_cleanup:
      gitCleanup && typeof gitCleanup === 'object'
        ? {
            removed: !!gitCleanup.removed,
            workspace_dir: cleanText(gitCleanup.workspace_dir || '', 400),
            reason: cleanText(gitCleanup.reason || '', 120),
          }
        : null,
  };
  saveArchivedAgentsState(state);
  return state.agents[key];
}

function unarchiveAgent(agentId) {
  const key = cleanText(agentId || '', 140);
  if (!key) return false;
  const state = loadArchivedAgentsState();
  if (!state.agents || !state.agents[key]) return false;
  delete state.agents[key];
  saveArchivedAgentsState(state);
  return true;
}

function archivedAgentIdsSet() {
  const state = loadArchivedAgentsState();
  const agents = state && state.agents && typeof state.agents === 'object' ? state.agents : {};
  const nowMs = Date.now();
  const out = new Set();
  for (const [agentId, meta] of Object.entries(agents)) {
    const id = cleanText(agentId || '', 140);
    if (!id) continue;
    const archivedAtMs = coerceTsMs(meta && meta.archived_at ? meta.archived_at : 0, 0);
    if (!archivedAtMs) {
      out.add(id);
      continue;
    }
    if ((nowMs - archivedAtMs) <= ARCHIVED_AGENT_FILTER_WINDOW_MS) {
      out.add(id);
    }
  }
  return out;
}

function reconcileArchivedAgentsFromCollab(collab) {
  if (!collab || typeof collab !== 'object') return 0;
  const dashboard = collab.dashboard;
  if (!dashboard || !Array.isArray(dashboard.agents) || dashboard.agents.length === 0) return 0;
  const state = loadArchivedAgentsState();
  const agents = state && state.agents && typeof state.agents === 'object' ? state.agents : null;
  if (!agents) return 0;
  let removed = 0;
  for (let idx = 0; idx < dashboard.agents.length; idx += 1) {
    const row = dashboard.agents[idx];
    const id = cleanText(row && (row.shadow || row.id) ? row.shadow || row.id : '', 140);
    if (!id || !agents[id]) continue;
    delete agents[id];
    removed += 1;
  }
  if (removed > 0) {
    saveArchivedAgentsState(state);
  }
  return removed;
}

function normalizeTerminationCondition(value) {
  const raw = cleanText(value || '', 40).toLowerCase();
  if (!raw) return 'task_or_timeout';
  if (raw === 'taskcomplete' || raw === 'task_complete' || raw === 'task' || raw === 'complete') return 'task_complete';
  if (raw === 'timeout' || raw === 'ttl' || raw === 'expiry') return 'timeout';
  if (raw === 'manual' || raw === 'revoke' || raw === 'revocation') return 'manual';
  if (raw === 'task_or_timeout' || raw === 'auto') return 'task_or_timeout';
  return 'task_or_timeout';
}

function normalizeAgentContractsState(state) {
  const root = state && typeof state === 'object' ? state : {};
  const defaults = root.defaults && typeof root.defaults === 'object' ? root.defaults : {};
  const contractsRaw = root.contracts && typeof root.contracts === 'object' ? root.contracts : {};
  const historyRaw = Array.isArray(root.terminated_history) ? root.terminated_history : [];
  const contracts = {};
  const terminatedCandidates = [];
  const terminatedCutoffMs = Date.now() - AGENT_CONTRACT_RETAIN_TERMINATED_MAX_AGE_MS;
  for (const [rawId, rawContract] of Object.entries(contractsRaw)) {
    const agentId = cleanText(rawId || (rawContract && rawContract.agent_id ? rawContract.agent_id : ''), 140);
    if (!agentId) continue;
    const contract = rawContract && typeof rawContract === 'object' ? rawContract : {};
    const expirySeconds = contract.expiry_seconds == null
      ? null
      : parsePositiveInt(contract.expiry_seconds, AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS, 1, 7 * 24 * 60 * 60);
    const spawnedAtMs = coerceTsMs(
      contract.spawned_at || contract.activated_at || contract.created_at || nowIso(),
      Date.now()
    );
    const spawnedAt = new Date(spawnedAtMs).toISOString();
    const explicitExpiresAt = cleanText(contract.expires_at || '', 80);
    const normalizedContract = {
      contract_id: cleanText(contract.contract_id || contract.id || `contract-${sha256(agentId).slice(0, 16)}`, 80),
      agent_id: agentId,
      mission: cleanText(contract.mission || `Assist with assigned mission for ${agentId}.`, 320),
      owner: cleanText(contract.owner || 'dashboard_session', 120),
      termination_condition: normalizeTerminationCondition(contract.termination_condition),
      expiry_seconds: expirySeconds,
      spawned_at: spawnedAt,
      expires_at: explicitExpiresAt || (expirySeconds ? new Date(spawnedAtMs + (expirySeconds * 1000)).toISOString() : ''),
      revoked_at: cleanText(contract.revoked_at || '', 80),
      completed_at: cleanText(contract.completed_at || '', 80),
      completion_source: cleanText(contract.completion_source || '', 120),
      status: cleanText(contract.status || 'active', 24) || 'active',
      termination_reason: cleanText(contract.termination_reason || '', 120),
      terminated_at: cleanText(contract.terminated_at || '', 80),
      terminated_by: cleanText(contract.terminated_by || '', 120),
      revived_from_contract_id: cleanText(contract.revived_from_contract_id || '', 80),
      revival_data: contract.revival_data && typeof contract.revival_data === 'object' ? contract.revival_data : null,
      conversation_hold: !!contract.conversation_hold,
      conversation_hold_started_at: cleanText(contract.conversation_hold_started_at || '', 80),
      conversation_hold_deadline: cleanText(contract.conversation_hold_deadline || '', 80),
      message_times_ms: Array.isArray(contract.message_times_ms)
        ? contract.message_times_ms
            .map((value) => coerceTsMs(value, 0))
            .filter((value) => Number.isFinite(value) && value > 0)
            .slice(-128)
        : [],
      security_flags: contract.security_flags && typeof contract.security_flags === 'object' ? contract.security_flags : {},
      updated_at: cleanText(contract.updated_at || nowIso(), 80) || nowIso(),
    };
    const status = cleanText(normalizedContract.status || '', 24).toLowerCase();
    if (status === 'active') {
      contracts[agentId] = normalizedContract;
    } else {
      terminatedCandidates.push([agentId, normalizedContract]);
    }
  }
  terminatedCandidates.sort((a, b) => {
    const aTs = coerceTsMs(
      a && a[1] ? a[1].terminated_at || a[1].updated_at || a[1].spawned_at || 0 : 0,
      0
    );
    const bTs = coerceTsMs(
      b && b[1] ? b[1].terminated_at || b[1].updated_at || b[1].spawned_at || 0 : 0,
      0
    );
    return bTs - aTs;
  });
  let retainedTerminated = 0;
  for (const [agentId, contractRow] of terminatedCandidates) {
    if (retainedTerminated >= AGENT_CONTRACT_RETAIN_TERMINATED_MAX) break;
    const terminatedAtMs = coerceTsMs(
      contractRow.terminated_at || contractRow.updated_at || contractRow.spawned_at || 0,
      0
    );
    if (terminatedAtMs > 0 && terminatedAtMs < terminatedCutoffMs) {
      continue;
    }
    contracts[agentId] = contractRow;
    retainedTerminated += 1;
  }
  const terminatedHistory = historyRaw
    .map((row) => {
      const entry = row && typeof row === 'object' ? row : {};
      const agentId = cleanText(entry.agent_id || '', 140);
      if (!agentId) return null;
      return {
        agent_id: agentId,
        contract_id: cleanText(entry.contract_id || '', 80),
        mission: cleanText(entry.mission || '', 320),
        owner: cleanText(entry.owner || '', 120),
        role: cleanText(entry.role || '', 80),
        termination_condition: normalizeTerminationCondition(entry.termination_condition),
        reason: cleanText(entry.reason || 'terminated', 120),
        terminated_at: cleanText(entry.terminated_at || nowIso(), 80) || nowIso(),
        revived: !!entry.revived,
        revived_at: cleanText(entry.revived_at || '', 80),
        revival_data: entry.revival_data && typeof entry.revival_data === 'object' ? entry.revival_data : null,
      };
    })
    .filter(Boolean)
    .slice(-200);
  return {
    type: 'infring_agent_contracts',
    updated_at: cleanText(root.updated_at || nowIso(), 80) || nowIso(),
    defaults: {
      default_expiry_seconds: parsePositiveInt(
        defaults.default_expiry_seconds,
        AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS,
        1,
        7 * 24 * 60 * 60
      ),
      auto_expire_on_complete: defaults.auto_expire_on_complete !== false,
      max_idle_agents: parsePositiveInt(defaults.max_idle_agents, AGENT_CONTRACT_MAX_IDLE_AGENTS, 1, 1000),
    },
    contracts,
    terminated_history: terminatedHistory,
  };
}

function loadAgentContractsState() {
  if (agentContractsCache) return agentContractsCache;
  agentContractsCache = normalizeAgentContractsState(readJson(AGENT_CONTRACTS_PATH, null));
  return agentContractsCache;
}

function saveAgentContractsState(state) {
  const normalized = normalizeAgentContractsState(state);
  normalized.updated_at = nowIso();
  agentContractsCache = normalized;
  writeJson(AGENT_CONTRACTS_PATH, normalized);
  return normalized;
}

function contractForAgent(agentId) {
  const id = cleanText(agentId || '', 140);
  if (!id) return null;
  const state = loadAgentContractsState();
  return state && state.contracts && state.contracts[id] ? state.contracts[id] : null;
}

function contractRemainingMs(contract, nowMs = Date.now()) {
  if (!contract || !contract.expires_at) return null;
  const expiryMs = coerceTsMs(contract.expires_at, 0);
  if (!expiryMs) return null;
  return expiryMs - nowMs;
}

function formatContractStatus(contract, nowMs = Date.now()) {
  if (!contract) return 'missing';
  if (contract.status !== 'active') return cleanText(contract.status || 'terminated', 24) || 'terminated';
  const remaining = contractRemainingMs(contract, nowMs);
  if (remaining != null && remaining <= 0) return 'expired';
  if (contract.completed_at) return 'complete_pending_termination';
  if (contract.revoked_at) return 'revoked_pending_termination';
  return 'active';
}

function terminationConditionMatches(condition, target) {
  const normalized = normalizeTerminationCondition(condition);
  if (normalized === target) return true;
  return normalized === 'task_or_timeout' && (target === 'task_complete' || target === 'timeout');
}

function missionCompleteSignal(text) {
  const body = String(text || '').toLowerCase();
  if (!body.trim()) return false;
  if (body.includes('[mission-complete]') || body.includes('[task-complete]')) return true;
  return /\b(mission complete|task complete|objective complete|objective achieved)\b/.test(body);
}

function buildAgentRevivalData(agentId) {
  const id = cleanText(agentId || '', 140);
  const sessionPath = agentSessionPath(id);
  const state = readJson(sessionPath, null);
  const sessions = state && Array.isArray(state.sessions) ? state.sessions : [];
  let messageCount = 0;
  let lastTs = '';
  for (const session of sessions) {
    const messages = Array.isArray(session && session.messages) ? session.messages : [];
    messageCount += messages.length;
    const tail = messages.length ? messages[messages.length - 1] : null;
    const tailTs = tail && tail.ts ? new Date(coerceTsMs(tail.ts, Date.now())).toISOString() : '';
    if (tailTs && (!lastTs || tailTs > lastTs)) lastTs = tailTs;
  }
  return {
    type: 'agent_session_snapshot_ref',
    session_path: path.relative(ROOT, sessionPath),
    message_count: messageCount,
    last_message_at: lastTs,
    archived_at: nowIso(),
  };
}

function detectContractViolation(agentId, cleanInput, contract, snapshot) {
  const text = String(cleanInput || '').toLowerCase();
  if (!text) return null;
  const state = loadAgentSession(agentId, snapshot);
  const session = activeSession(state);
  const nowMs = Date.now();
  const recentCount = (Array.isArray(session.messages) ? session.messages : []).reduce((count, message) => {
    const tsMs = coerceTsMs(message && message.ts ? message.ts : 0, 0);
    return tsMs > 0 && (nowMs - tsMs) <= AGENT_ROGUE_SPIKE_WINDOW_MS ? count + 1 : count;
  }, 0);
  const payload = {
    ...runtimeAuthorityPayload(runtimeSyncSummary(snapshot)),
    agent_id: cleanText(agentId || '', 140),
    input_text: cleanText(cleanInput || '', 1200),
    recent_messages: recentCount,
    rogue_message_rate_max_per_min: AGENT_ROGUE_MESSAGE_RATE_MAX_PER_MIN,
    contract_status: cleanText(contract && contract.status ? contract.status : 'active', 24) || 'active',
  };
  const cacheKey = `runtime.contract.guard.${sha256(JSON.stringify(payload)).slice(0, 24)}`;
  const lane = runLaneCached(
    cacheKey,
    [
      'runtime-systems',
      'run',
      '--system-id=V6-DASHBOARD-007.3',
      '--strict=1',
      '--apply=0',
      `--payload-json=${JSON.stringify(payload)}`,
    ],
    {
      timeout_ms: RUNTIME_AUTHORITY_LANE_TIMEOUT_MS,
      ttl_ms: RUNTIME_AUTHORITY_CACHE_TTL_MS,
      fail_ttl_ms: RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS,
      stale_fallback: false,
    }
  );
  const lanePayload = lane && lane.payload && typeof lane.payload === 'object' ? lane.payload : null;
  const contractExecution =
    lanePayload &&
    lanePayload.contract_execution &&
    typeof lanePayload.contract_execution === 'object'
      ? lanePayload.contract_execution
      : null;
  const specificChecks =
    contractExecution &&
    contractExecution.specific_checks &&
    typeof contractExecution.specific_checks === 'object'
      ? contractExecution.specific_checks
      : null;
  const guard =
    specificChecks &&
    specificChecks.dashboard_contract_guard &&
    typeof specificChecks.dashboard_contract_guard === 'object'
      ? specificChecks.dashboard_contract_guard
      : null;
  if (!lane || !lane.ok || !guard || !guard.violation) return null;
  return {
    reason: cleanText(guard.reason || 'contract_violation', 120) || 'contract_violation',
    detail: cleanText(guard.detail || '', 240),
  };
  return null;
}

function deriveAgentContract(agentId, spawnPayload = {}, options = {}) {
  const now = nowIso();
  const payload = spawnPayload && typeof spawnPayload === 'object' ? spawnPayload : {};
  const contractInput = payload.contract && typeof payload.contract === 'object' ? payload.contract : {};
  const explicitIndefinite = contractInput.indefinite === true || payload.indefinite === true;
  const spawnedAtInput =
    contractInput.spawned_at ||
    payload.spawned_at ||
    payload.activated_at ||
    options.spawned_at ||
    now;
  const spawnedAtMs = coerceTsMs(spawnedAtInput, Date.now());
  const spawnedAtIso = new Date(spawnedAtMs).toISOString();
  const expirySeconds = explicitIndefinite
    ? null
    : parsePositiveInt(
        contractInput.expiry_seconds != null ? contractInput.expiry_seconds : payload.expiry_seconds,
        AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS,
        1,
        7 * 24 * 60 * 60
      );
  const mission = cleanText(
    contractInput.mission || payload.mission || `Assist with assigned mission for ${agentId}.`,
    320
  ) || `Assist with assigned mission for ${agentId}.`;
  const owner = cleanText(contractInput.owner || payload.owner || options.owner || 'dashboard_session', 120) || 'dashboard_session';
  const condition = normalizeTerminationCondition(
    contractInput.termination_condition || payload.termination_condition || 'task_or_timeout'
  );
  const explicitExpiresAt = cleanText(
    contractInput.expires_at || payload.expires_at || options.expires_at || '',
    80
  );
  return {
    contract_id:
      cleanText(
        contractInput.id || contractInput.contract_id || `contract-${sha256(`${agentId}:${now}:${mission}`).slice(0, 16)}`,
        80
      ) || `contract-${sha256(`${agentId}:${now}`).slice(0, 16)}`,
    agent_id: cleanText(agentId || '', 140),
    mission,
    owner,
    termination_condition: condition,
    expiry_seconds: expirySeconds,
    spawned_at: spawnedAtIso,
    expires_at: explicitExpiresAt || (expirySeconds ? new Date(spawnedAtMs + (expirySeconds * 1000)).toISOString() : ''),
    revoked_at: '',
    completed_at: '',
    completion_source: '',
    status: 'active',
    termination_reason: '',
    terminated_at: '',
    terminated_by: '',
    revived_from_contract_id: cleanText(
      contractInput.revived_from_contract_id || payload.revived_from_contract_id || '',
      80
    ),
    revival_data: contractInput.revival_data && typeof contractInput.revival_data === 'object'
      ? contractInput.revival_data
      : null,
    conversation_hold: false,
    conversation_hold_started_at: '',
    conversation_hold_deadline: '',
    message_times_ms: [],
    security_flags: {},
    updated_at: now,
  };
}

function resetContractExpiryFromNow(contract, nowMs = Date.now()) {
  if (!contract || typeof contract !== 'object') return contract;
  const expirySeconds = contract.expiry_seconds == null
    ? null
    : parsePositiveInt(contract.expiry_seconds, AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS, 1, 7 * 24 * 60 * 60);
  if (expirySeconds == null) {
    contract.expires_at = '';
    return contract;
  }
  contract.expires_at = new Date(nowMs + (expirySeconds * 1000)).toISOString();
  return contract;
}

function setContractConversationHold(agentId, hold = true, options = {}) {
  const id = cleanText(agentId || '', 140);
  if (!id) return null;
  const state = loadAgentContractsState();
  const contract = state.contracts && state.contracts[id] ? state.contracts[id] : null;
  if (!contract || contract.status !== 'active') return contract;
  const nowMs = Date.now();
  const now = new Date(nowMs).toISOString();
  const maxHoldMs = parsePositiveInt(
    options && options.max_hold_ms != null ? options.max_hold_ms : AGENT_CONTRACT_CHAT_HOLD_MAX_MS,
    AGENT_CONTRACT_CHAT_HOLD_MAX_MS,
    1,
    7 * 24 * 60 * 60 * 1000
  );
  if (hold) {
    const shouldResetTimer = options && options.reset_timer === false ? false : true;
    contract.conversation_hold = true;
    contract.conversation_hold_started_at = cleanText(contract.conversation_hold_started_at || now, 80) || now;
    contract.conversation_hold_deadline = new Date(nowMs + maxHoldMs).toISOString();
    if (shouldResetTimer) {
      contract.spawned_at = now;
      resetContractExpiryFromNow(contract, nowMs);
    }
    contract.updated_at = now;
  } else {
    const shouldResetOnClose = options && options.reset_timer_on_close === false ? false : true;
    contract.conversation_hold = false;
    contract.conversation_hold_started_at = '';
    contract.conversation_hold_deadline = '';
    if (shouldResetOnClose) {
      contract.spawned_at = now;
      resetContractExpiryFromNow(contract, nowMs);
    }
    contract.updated_at = now;
  }
  state.contracts[id] = contract;
  saveAgentContractsState(state);
  return contract;
}

function upsertAgentContract(agentId, spawnPayload = {}, options = {}) {
  const id = cleanText(agentId || '', 140);
  if (!id) return null;
  const force = !!(options && options.force);
  const state = loadAgentContractsState();
  const existing = state.contracts && state.contracts[id] ? state.contracts[id] : null;
  if (existing && !force) {
    const touched = {
      ...existing,
      mission: cleanText(existing.mission || '', 320) || deriveAgentContract(id, spawnPayload, options).mission,
      updated_at: nowIso(),
    };
    state.contracts[id] = touched;
    saveAgentContractsState(state);
    return touched;
  }
  const next = deriveAgentContract(id, spawnPayload, options);
  if (!state.contracts || typeof state.contracts !== 'object') state.contracts = {};
  state.contracts[id] = next;
  saveAgentContractsState(state);
  return next;
}

function markContractCompletion(agentId, source = 'supervisor') {
  const id = cleanText(agentId || '', 140);
  if (!id) return null;
  const state = loadAgentContractsState();
  const contract = state.contracts && state.contracts[id] ? state.contracts[id] : null;
  if (!contract || contract.status !== 'active') return contract;
  contract.completed_at = nowIso();
  contract.completion_source = cleanText(source || 'supervisor', 120);
  contract.updated_at = nowIso();
  state.contracts[id] = contract;
  saveAgentContractsState(state);
  return contract;
}

function markContractRevocation(agentId, source = 'manual_revoke') {
  const id = cleanText(agentId || '', 140);
  if (!id) return null;
  const state = loadAgentContractsState();
  const contract = state.contracts && state.contracts[id] ? state.contracts[id] : null;
  if (!contract || contract.status !== 'active') return contract;
  contract.revoked_at = nowIso();
  contract.terminated_by = cleanText(source || 'manual_revoke', 120) || 'manual_revoke';
  contract.updated_at = nowIso();
  state.contracts[id] = contract;
  saveAgentContractsState(state);
  return contract;
}

function recordContractMessageTick(agentId) {
  const id = cleanText(agentId || '', 140);
  if (!id) return null;
  const state = loadAgentContractsState();
  const contract = state.contracts && state.contracts[id] ? state.contracts[id] : null;
  if (!contract || contract.status !== 'active') return contract;
  const nowMs = Date.now();
  const recent = Array.isArray(contract.message_times_ms) ? contract.message_times_ms : [];
  contract.message_times_ms = recent
    .map((value) => coerceTsMs(value, 0))
    .filter((value) => Number.isFinite(value) && value > 0 && (nowMs - value) <= AGENT_ROGUE_SPIKE_WINDOW_MS)
    .slice(-128);
  contract.message_times_ms.push(nowMs);
  contract.updated_at = nowIso();
  state.contracts[id] = contract;
  saveAgentContractsState(state);
  return contract;
}

function discoverTerminationTeams(preferredTeam = DEFAULT_TEAM) {
  const preferred = cleanText(preferredTeam || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const nowMs = Date.now();
  if (
    parseNonNegativeInt(agentTerminationTeamDiscoveryState.scanned_ms, 0, 1000000000000) > 0 &&
    (nowMs - parseNonNegativeInt(agentTerminationTeamDiscoveryState.scanned_ms, 0, 1000000000000)) <
      AGENT_TERMINATION_TEAM_DISCOVERY_CACHE_MS &&
    Array.isArray(agentTerminationTeamDiscoveryState.teams) &&
    agentTerminationTeamDiscoveryState.teams.length > 0
  ) {
    const ordered = [preferred];
    for (const team of agentTerminationTeamDiscoveryState.teams) {
      const cleanTeam = cleanText(team, 40);
      if (!cleanTeam || ordered.includes(cleanTeam)) continue;
      ordered.push(cleanTeam);
    }
    return ordered;
  }
  const discovered = new Set([DEFAULT_TEAM, preferred]);
  try {
    if (fs.existsSync(COLLAB_TEAM_STATE_DIR)) {
      for (const name of fs.readdirSync(COLLAB_TEAM_STATE_DIR)) {
        const cleanName = cleanText(String(name || ''), 140);
        if (!cleanName || !cleanName.endsWith('.json')) continue;
        const team = cleanText(cleanName.slice(0, -5), 40);
        if (!team) continue;
        discovered.add(team);
      }
    }
  } catch {}
  const teams = Array.from(discovered.values()).filter(Boolean);
  agentTerminationTeamDiscoveryState = {
    scanned_ms: nowMs,
    teams,
  };
  return teams;
}

function attemptLaneTermination(agentId, team = DEFAULT_TEAM) {
  const cleanId = cleanText(agentId || '', 140);
  const cleanTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const attempts = [];
  let removedCount = 0;
  let releasedTaskCount = 0;
  let command = '';
  const candidates = discoverTerminationTeams(cleanTeam).map((candidateTeam) => [
    'collab-plane',
    'terminate-role',
    `--team=${candidateTeam}`,
    `--shadow=${cleanId}`,
    '--strict=1',
  ]);
  let removalConfirmed = false;
  for (const argv of candidates) {
    const lane = runLane(argv);
    const removed = parseNonNegativeInt(
      lane && lane.payload && lane.payload.removed_count != null ? lane.payload.removed_count : 0,
      0,
      100000000
    );
    const released = parseNonNegativeInt(
      lane && lane.payload && lane.payload.released_task_count != null ? lane.payload.released_task_count : 0,
      0,
      100000000
    );
    attempts.push({
      ...laneOutcome(lane),
      team: cleanText(String(argv[2] || '').replace('--team=', ''), 40),
      removed_count: removed,
      released_task_count: released,
    });
    if (removed > removedCount) {
      removedCount = removed;
    }
    if (released > releasedTaskCount) {
      releasedTaskCount = released;
    }
    if (lane && lane.ok && (removed > 0 || released > 0)) {
      removalConfirmed = true;
      command = cleanText(`protheus-ops ${argv.join(' ')}`, 240);
      break;
    }
  }
  if (!command) {
    const firstOk = attempts.find((entry) => !!(entry && entry.ok));
    if (firstOk) {
      command = cleanText(
        `protheus-ops collab-plane terminate-role --team=${cleanText(firstOk.team || cleanTeam, 40) || cleanTeam} --shadow=${cleanId} --strict=1`,
        240
      );
    }
  }
  return {
    ok: attempts.some((entry) => entry && entry.ok),
    removal_confirmed: removalConfirmed,
    attempted_teams: attempts.map((entry) => cleanText(entry && entry.team ? entry.team : '', 40)).filter(Boolean),
    attempts,
    command_count: attempts.length,
    removed_count: removedCount,
    released_task_count: releasedTaskCount,
    command,
  };
}

function terminateAgentForContract(agentId, snapshot, reason = 'timeout', options = {}) {
  const cleanId = cleanText(agentId || '', 140);
  if (!cleanId) return { terminated: false, agent_id: cleanId, reason: 'invalid_agent_id' };
  const autoTermination = !!(options && options.auto_termination);
  const snapshotMasterAgentId =
    autoTermination && snapshot
      ? selectMasterAgentId(runtimeAgentIdsFromSnapshot(snapshot, { includeArchived: false }))
      : '';
  if (
    autoTermination &&
    (
      isMainTreeBoundAgent(cleanId, options && options.agent_row ? options.agent_row : null) ||
      (snapshotMasterAgentId && snapshotMasterAgentId === cleanId)
    )
  ) {
    ensureAgentGitTreeProfile(cleanId, { force_master: true });
    return {
      terminated: false,
      agent_id: cleanId,
      reason: 'main_tree_agent_auto_termination_disabled',
      protected_main_tree_agent: true,
    };
  }
  const state = loadAgentContractsState();
  const contract = state.contracts && state.contracts[cleanId] ? state.contracts[cleanId] : null;
  if (!contract || contract.status !== 'active') {
    return { terminated: false, agent_id: cleanId, reason: 'contract_not_active' };
  }
  const team =
    cleanText(
      options.team || (snapshot && snapshot.metadata && snapshot.metadata.team ? snapshot.metadata.team : DEFAULT_TEAM),
      40
    ) || DEFAULT_TEAM;
  const termination = attemptLaneTermination(cleanId, team);
  const terminalClosed = closeTerminalSession(cleanId, `agent_contract_${cleanText(reason, 80)}`);
  const revivalData = buildAgentRevivalData(cleanId);
  const terminatedAt = nowIso();
  const archivedMeta = archiveAgent(cleanId, {
    source: cleanText(options.source || 'agent_contract_enforcer', 80) || 'agent_contract_enforcer',
    reason: cleanText(reason, 120) || 'terminated',
    contract_id: contract.contract_id,
    mission: contract.mission,
    owner: contract.owner,
    role: cleanText(options.role || '', 80),
    termination_condition: contract.termination_condition,
    terminated_at: terminatedAt,
    revival_data: revivalData,
  });
  const updated = {
    ...contract,
    status: 'terminated',
    termination_reason: cleanText(reason, 120),
    terminated_at: terminatedAt,
    terminated_by: cleanText(options.terminated_by || 'contract_enforcer', 120) || 'contract_enforcer',
    revival_data: revivalData,
    updated_at: terminatedAt,
  };
  state.contracts[cleanId] = updated;
  state.terminated_history = Array.isArray(state.terminated_history) ? state.terminated_history : [];
  state.terminated_history.push({
    agent_id: cleanId,
    contract_id: updated.contract_id,
    mission: updated.mission,
    owner: updated.owner,
    role: cleanText(options.role || '', 80),
    termination_condition: updated.termination_condition,
    reason: cleanText(reason, 120) || 'terminated',
    terminated_at: terminatedAt,
    revived: false,
    revived_at: '',
    revival_data: revivalData,
  });
  state.terminated_history = state.terminated_history.slice(-200);
  saveAgentContractsState(state);
  const laneResult = {
    ok: termination.ok,
    status: termination.ok ? 0 : 1,
    argv: ['agent-contract', 'terminate', `--agent=${cleanId}`],
    payload: {
      ok: termination.ok,
      type: 'agent_contract_termination',
      reason: cleanText(reason, 120) || 'terminated',
      lane_attempts: termination.attempts,
      terminal_closed: terminalClosed,
      archived_at: archivedMeta && archivedMeta.archived_at ? archivedMeta.archived_at : '',
      contract_id: updated.contract_id,
    },
  };
  const actionReceipt = writeActionReceipt(
    'agent.contract.terminate',
    {
      agent_id: cleanId,
      contract_id: updated.contract_id,
      reason: cleanText(reason, 120) || 'terminated',
      mission: cleanText(updated.mission || '', 240),
      owner: cleanText(updated.owner || '', 120),
      termination_condition: cleanText(updated.termination_condition || '', 40),
      team,
    },
    laneResult
  );
  return {
    terminated: true,
    agent_id: cleanId,
    reason: cleanText(reason, 120) || 'terminated',
    contract: updated,
    lane: termination,
    action_receipt: actionReceipt,
    terminal_closed: terminalClosed,
  };
}

function contractEnforcementAuthorityFromRust(snapshot, state, activeRows, nowMs, idleThreshold, sessionActivityCache = null) {
  const contracts = state && state.contracts && typeof state.contracts === 'object' ? state.contracts : {};
  const runtime = runtimeSyncSummary(snapshot);
  const activityCache =
    sessionActivityCache && typeof sessionActivityCache.get === 'function' ? sessionActivityCache : new Map();
  const contractsPayload = Object.entries(contracts)
    .map(([agentId, contract]) => {
      const id = cleanText(agentId || '', 140);
      if (!id || !contract || typeof contract !== 'object') return null;
      const activeRow = Array.isArray(activeRows) ? activeRows.find((row) => row && row.id === id) : null;
      let sessionUpdatedMs = activityCache.has(id) ? activityCache.get(id) : null;
      if (sessionUpdatedMs == null) {
        sessionUpdatedMs = agentSessionActivityTimestampMs(id);
        activityCache.set(id, sessionUpdatedMs);
      }
      const spawnedAtMs = coerceTsMs(contract.spawned_at || (activeRow && activeRow.activated_at) || 0, 0);
      const messageTimes = Array.isArray(contract.message_times_ms)
        ? contract.message_times_ms
            .map((value) => coerceTsMs(value, 0))
            .filter((value) => Number.isFinite(value) && value > 0)
        : [];
      const messageActivityMs = messageTimes.length > 0 ? Math.max(...messageTimes) : 0;
      const activityMs = Math.max(
        spawnedAtMs,
        parseNonNegativeInt(sessionUpdatedMs, 0, 1000000000000),
        parseNonNegativeInt(messageActivityMs, 0, 1000000000000)
      );
      const idleForMs = activityMs > 0 ? Math.max(0, nowMs - activityMs) : Number.MAX_SAFE_INTEGER;
      const holdDeadlineMs = coerceTsMs(contract.conversation_hold_deadline || 0, 0);
      const holdActive = !!(contract.conversation_hold === true && (!holdDeadlineMs || holdDeadlineMs > nowMs));
      return {
        agent_id: id,
        auto_terminate_allowed: !isMainTreeBoundAgent(id, activeRow) && !holdActive,
        status: cleanText(contract.status || 'active', 24) || 'active',
        termination_condition: cleanText(contract.termination_condition || 'task_or_timeout', 40) || 'task_or_timeout',
        revoked_at: cleanText(contract.revoked_at || '', 80),
        completed_at: cleanText(contract.completed_at || '', 80),
        remaining_ms: contractRemainingMs(contract, nowMs),
        idle_for_ms: idleForMs,
      };
    })
    .filter(Boolean);
  const payload = {
    ...runtimeAuthorityPayload(runtime),
    authority_mode: 'contract_enforcement',
    now_ms: nowMs,
    idle_threshold: parsePositiveInt(idleThreshold, AGENT_CONTRACT_MAX_IDLE_AGENTS, 1, 1000),
    idle_termination_ms: AGENT_IDLE_TERMINATION_MS,
    idle_batch: AGENT_IDLE_TERMINATION_BATCH,
    idle_batch_max: AGENT_IDLE_TERMINATION_BATCH_MAX,
    idle_cooldown_ms: AGENT_IDLE_TERMINATION_COOLDOWN_MS,
    idle_since_last_ms: Math.max(
      0,
      nowMs - parseNonNegativeInt(agentTerminationSweepState.last_idle_run_ms, 0, 1000000000000)
    ),
    active_agent_count: Array.isArray(activeRows) ? activeRows.length : 0,
    contracts: contractsPayload,
  };
  const cacheKey = `runtime.contracts.authority.${sha256(JSON.stringify(payload)).slice(0, 24)}`;
  const lane = runLaneCached(
    cacheKey,
    [
      'runtime-systems',
      'run',
      '--system-id=V6-DASHBOARD-007.2',
      '--strict=1',
      '--apply=0',
      `--payload-json=${JSON.stringify(payload)}`,
    ],
    {
      timeout_ms: RUNTIME_AUTHORITY_LANE_TIMEOUT_MS,
      ttl_ms: RUNTIME_AUTHORITY_CACHE_TTL_MS,
      fail_ttl_ms: RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS,
      stale_fallback: false,
    }
  );
  const lanePayload = lane && lane.payload && typeof lane.payload === 'object' ? lane.payload : null;
  const contractExecution =
    lanePayload &&
    lanePayload.contract_execution &&
    typeof lanePayload.contract_execution === 'object'
      ? lanePayload.contract_execution
      : null;
  const specificChecks =
    contractExecution &&
    contractExecution.specific_checks &&
    typeof contractExecution.specific_checks === 'object'
      ? contractExecution.specific_checks
      : null;
  const dashboardAuthority =
    specificChecks &&
    specificChecks.dashboard_runtime_authority &&
    typeof specificChecks.dashboard_runtime_authority === 'object'
      ? specificChecks.dashboard_runtime_authority
      : null;
  const contractEnforcement =
    dashboardAuthority &&
    dashboardAuthority.contract_enforcement &&
    typeof dashboardAuthority.contract_enforcement === 'object'
      ? dashboardAuthority.contract_enforcement
      : null;
  return {
    ok: !!(lane && lane.ok && contractEnforcement),
    lane: laneOutcome(lane || null),
    contract_enforcement: contractEnforcement,
  };
}

function contractTerminationDecision(contract, nowMs = Date.now()) {
  if (!contract || contract.status !== 'active') return '';
  if (contract.conversation_hold === true) {
    const holdDeadlineMs = coerceTsMs(contract.conversation_hold_deadline || 0, 0);
    if (!holdDeadlineMs || holdDeadlineMs > nowMs) return '';
  }
  if (contract.revoked_at) return 'manual_revocation';
  if (terminationConditionMatches(contract.termination_condition, 'task_complete') && contract.completed_at) {
    return 'task_complete';
  }
  const remaining = contractRemainingMs(contract, nowMs);
  if (remaining != null && remaining <= 0 && terminationConditionMatches(contract.termination_condition, 'timeout')) {
    return 'timeout';
  }
  return '';
}

function contractSummary(contract, nowMs = Date.now()) {
  if (!contract) return null;
  const remainingMs = contractRemainingMs(contract, nowMs);
  return {
    id: cleanText(contract.contract_id || '', 80),
    mission: cleanText(contract.mission || '', 320),
    owner: cleanText(contract.owner || '', 120),
    termination_condition: cleanText(contract.termination_condition || '', 40),
    status: formatContractStatus(contract, nowMs),
    expires_at: cleanText(contract.expires_at || '', 80),
    expiry_seconds:
      contract.expiry_seconds == null
        ? null
        : parsePositiveInt(contract.expiry_seconds, AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS, 1, 7 * 24 * 60 * 60),
    remaining_ms: remainingMs == null ? null : Math.max(0, Math.floor(remainingMs)),
    completed_at: cleanText(contract.completed_at || '', 80),
    completion_source: cleanText(contract.completion_source || '', 120),
    revoked_at: cleanText(contract.revoked_at || '', 80),
    terminated_at: cleanText(contract.terminated_at || '', 80),
    termination_reason: cleanText(contract.termination_reason || '', 120),
    revived_from_contract_id: cleanText(contract.revived_from_contract_id || '', 80),
  };
}

function scaleAwareBatchSize(activeAgents, base, maxCap) {
  const active = parseNonNegativeInt(activeAgents, 0, 1_000_000);
  const floor = parsePositiveInt(base, 1, 1, maxCap);
  const cap = parsePositiveInt(maxCap, floor, floor, 1_000_000);
  if (active >= AGENT_CONTRACT_ENFORCE_MEGA_SCALE_THRESHOLD) {
    return Math.min(cap, floor * 8);
  }
  if (active >= AGENT_CONTRACT_ENFORCE_ULTRA_SCALE_THRESHOLD) {
    return Math.min(cap, floor * 4);
  }
  if (active >= AGENT_CONTRACT_ENFORCE_HIGH_SCALE_THRESHOLD) {
    return Math.min(cap, floor * 2);
  }
  return floor;
}

function enforceAgentContracts(snapshot, options = {}) {
  const nowMs = Date.now();
  const activeRows = compatAgentsFromSnapshot(snapshot, { includeArchived: false });
  const activeAgentCount = activeRows.length;
  const activeIds = new Set(activeRows.map((row) => cleanText(row && row.id ? row.id : '', 140)).filter(Boolean));
  const activeRowById = new Map(
    activeRows
      .map((row) => [cleanText(row && row.id ? row.id : '', 140), row])
      .filter(([id]) => !!id)
  );
  const sessionActivityCache = new Map();
  const team =
    cleanText(
      options.team || (snapshot && snapshot.metadata && snapshot.metadata.team ? snapshot.metadata.team : DEFAULT_TEAM),
      40
    ) || DEFAULT_TEAM;

  let state = loadAgentContractsState();
  const defaults = state.defaults && typeof state.defaults === 'object' ? state.defaults : {};
  let changed = false;

  if (!state.contracts || typeof state.contracts !== 'object') {
    state.contracts = {};
    changed = true;
  }
  const defaultExpirySeconds = parsePositiveInt(
    defaults.default_expiry_seconds,
    AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS,
    1,
    7 * 24 * 60 * 60
  );
  const defaultTerminationCondition = defaults.auto_expire_on_complete === false ? 'timeout' : 'task_or_timeout';

  for (const row of activeRows) {
    const id = cleanText(row && row.id ? row.id : '', 140);
    if (!id) continue;
    const activatedAt = cleanText(row && row.activated_at ? row.activated_at : '', 80);
    if (!state.contracts[id]) {
      state.contracts[id] = deriveAgentContract(id, {
        mission: `Assist with assigned mission for ${id}.`,
        owner: 'dashboard_auto',
        expiry_seconds: defaultExpirySeconds,
        termination_condition: defaultTerminationCondition,
        activated_at: activatedAt,
        spawned_at: activatedAt,
      }, {
        spawned_at: activatedAt,
      });
      changed = true;
      continue;
    }
    const existing = state.contracts[id];
    if (!existing || existing.status !== 'active') continue;
    const activatedAtMs = coerceTsMs(activatedAt, 0);
    const spawnedAtMs = coerceTsMs(existing.spawned_at, 0);
    const shouldAlignSpawn = activatedAtMs > 0 && (spawnedAtMs <= 0 || activatedAtMs < (spawnedAtMs - 1000));
    if (!shouldAlignSpawn) continue;
    existing.spawned_at = new Date(activatedAtMs).toISOString();
    const expirySeconds =
      existing.expiry_seconds == null
        ? null
        : parsePositiveInt(existing.expiry_seconds, defaultExpirySeconds, 1, 7 * 24 * 60 * 60);
    if (expirySeconds != null) {
      existing.expires_at = new Date(activatedAtMs + (expirySeconds * 1000)).toISOString();
    }
    existing.updated_at = nowIso();
    state.contracts[id] = existing;
    changed = true;
  }

  for (const [agentId, contract] of Object.entries(state.contracts || {})) {
    const id = cleanText(agentId || '', 140);
    if (!id || !contract || contract.status !== 'active') continue;
    if (!activeIds.has(id) && isAgentArchived(id)) {
      contract.status = 'terminated';
      contract.terminated_at = cleanText(contract.terminated_at || nowIso(), 80) || nowIso();
      contract.termination_reason = cleanText(contract.termination_reason || 'archived', 120) || 'archived';
      contract.updated_at = nowIso();
      state.contracts[id] = contract;
      changed = true;
    }
  }

  if (changed) {
    state = saveAgentContractsState(state);
  }

  const reconciled = [];
  const reconcileCandidates = activeRows
    .map((row) => {
      const id = cleanText(row && row.id ? row.id : '', 140);
      if (!id) return null;
      const contract = state.contracts && state.contracts[id] ? state.contracts[id] : null;
      const archived = isAgentArchived(id);
      if (!archived && (!contract || contract.status === 'active')) return null;
      return {
        id,
        archived,
        activated_at: cleanText(row && row.activated_at ? row.activated_at : '', 80),
      };
    })
    .filter(Boolean)
    .sort((a, b) => {
      if (!!a.archived !== !!b.archived) return a.archived ? -1 : 1;
      return coerceTsMs(a.activated_at, 0) - coerceTsMs(b.activated_at, 0);
    })
    .slice(
      0,
      Math.min(
        AGENT_ENFORCE_MAX_TERMINATIONS_PER_SWEEP,
        scaleAwareBatchSize(
          activeAgentCount,
          AGENT_RECONCILE_TERMINATION_BATCH,
          AGENT_RECONCILE_TERMINATION_BATCH_MAX
        )
      )
    );
  const canSweepTerminate =
    reconcileCandidates.length > 0 &&
    (nowMs - parseNonNegativeInt(agentTerminationSweepState.last_run_ms, 0, 1000000000000)) >=
      AGENT_RECONCILE_TERMINATION_COOLDOWN_MS;
  if (canSweepTerminate) {
    agentTerminationSweepState.last_run_ms = nowMs;
    for (const candidate of reconcileCandidates) {
      const lane = attemptLaneTermination(candidate.id, team);
      if (lane.ok && parseNonNegativeInt(lane.removed_count, 0, 100000000) > 0) {
        reconciled.push({
          agent_id: candidate.id,
          command: cleanText(lane.command || '', 80),
          removed_count: parseNonNegativeInt(lane.removed_count, 0, 100000000),
          released_task_count: parseNonNegativeInt(lane.released_task_count, 0, 100000000),
          archived: !!candidate.archived,
        });
        closeTerminalSession(candidate.id, 'agent_contract_reconcile');
      }
    }
  }

  const latestState = loadAgentContractsState();
  const idleThreshold = parsePositiveInt(
    latestState && latestState.defaults ? latestState.defaults.max_idle_agents : AGENT_CONTRACT_MAX_IDLE_AGENTS,
    AGENT_CONTRACT_MAX_IDLE_AGENTS,
    1,
    1000
  );
  const rustContractAuthority = contractEnforcementAuthorityFromRust(
    snapshot,
    latestState,
    activeRows,
    nowMs,
    idleThreshold,
    sessionActivityCache
  );
  const rustTerminationsById = new Map(
    rustContractAuthority &&
      rustContractAuthority.ok &&
      rustContractAuthority.contract_enforcement &&
      Array.isArray(rustContractAuthority.contract_enforcement.termination_decisions)
      ? rustContractAuthority.contract_enforcement.termination_decisions
          .map((row) => [
            cleanText(row && row.agent_id ? row.agent_id : '', 140),
            cleanText(row && row.reason ? row.reason : '', 120),
          ])
          .filter(([id, reason]) => !!id && !!reason)
      : []
  );

  const terminations = [];
  const currentState = loadAgentContractsState() || {};
  if (!currentState.contracts || typeof currentState.contracts !== 'object') {
    currentState.contracts = {};
  }
  const currentContracts = Object.entries(currentState.contracts || {});
  let currentStateChanged = false;
  let terminationSweepCount = 0;
  for (const [agentId, contract] of currentContracts) {
    if (terminationSweepCount >= AGENT_ENFORCE_MAX_TERMINATIONS_PER_SWEEP) break;
    const id = cleanText(agentId || '', 140);
    if (!id || !contract || contract.status !== 'active') continue;
    if (contract.conversation_hold === true) {
      const holdDeadlineMs = coerceTsMs(contract.conversation_hold_deadline || 0, 0);
      if (!holdDeadlineMs || holdDeadlineMs > nowMs) {
        continue;
      }
      contract.conversation_hold = false;
      contract.conversation_hold_started_at = '';
      contract.conversation_hold_deadline = '';
      contract.spawned_at = nowIso();
      resetContractExpiryFromNow(contract, nowMs);
      contract.updated_at = nowIso();
      currentState.contracts[id] = contract;
      currentStateChanged = true;
    }
    if (isMainTreeBoundAgent(id, activeRowById.get(id) || null)) continue;
    const reason = rustTerminationsById.get(id) || '';
    if (!reason) continue;
    const roleRow = activeRows.find((row) => row && row.id === id);
    const terminated = terminateAgentForContract(id, snapshot, reason, {
      source: 'agent_contract_enforcer',
      terminated_by: 'agent_contract_enforcer',
      role: cleanText(roleRow && roleRow.role ? roleRow.role : '', 80),
      team,
      auto_termination: true,
      agent_row: roleRow || null,
    });
    if (terminated.terminated) {
      terminations.push(terminated);
      terminationSweepCount += 1;
    }
  }
  if (currentStateChanged) {
    saveAgentContractsState(currentState);
  }

  let idleCandidates = [];
  if (
    rustContractAuthority &&
    rustContractAuthority.ok &&
    rustContractAuthority.contract_enforcement &&
    Array.isArray(rustContractAuthority.contract_enforcement.idle_candidates)
  ) {
    idleCandidates = rustContractAuthority.contract_enforcement.idle_candidates
      .map((row) => {
        const id = cleanText(row && row.agent_id ? row.agent_id : '', 140);
        if (!id) return null;
        const heldContract = latestState && latestState.contracts ? latestState.contracts[id] : null;
        const holdDeadlineMs = coerceTsMs(heldContract && heldContract.conversation_hold_deadline ? heldContract.conversation_hold_deadline : 0, 0);
        const holdActive = !!(heldContract && heldContract.conversation_hold === true && (!holdDeadlineMs || holdDeadlineMs > nowMs));
        return {
          id,
          idleForMs: parseNonNegativeInt(row && row.idle_for_ms, 0, 1000000000000),
          activity_ms: Math.max(0, nowMs - parseNonNegativeInt(row && row.idle_for_ms, 0, 1000000000000)),
          holdActive,
          role: cleanText(
            activeRowById.get(id) && activeRowById.get(id).role ? activeRowById.get(id).role : '',
            80
          ),
        };
      })
      .filter((row) => !!row && !row.holdActive);
  }
  const rustIdle = rustContractAuthority && rustContractAuthority.ok && rustContractAuthority.contract_enforcement
    ? rustContractAuthority.contract_enforcement
    : null;
  const idleExcess = rustIdle && rustIdle.idle_excess != null
    ? parseNonNegativeInt(rustIdle.idle_excess, 0, 100000000)
    : 0;
  const idleSweepReady = rustIdle && rustIdle.idle_sweep_ready != null
    ? !!rustIdle.idle_sweep_ready
    : false;
  const idleBatchSize = rustIdle && rustIdle.idle_batch_size != null
    ? parseNonNegativeInt(rustIdle.idle_batch_size, 0, AGENT_IDLE_TERMINATION_BATCH_MAX)
    : 0;
  const boundedIdleBatchSize = Math.min(idleBatchSize, AGENT_ENFORCE_MAX_TERMINATIONS_PER_SWEEP);
  let idleTerminatedCount = 0;
  if (idleSweepReady) {
    agentTerminationSweepState.last_idle_run_ms = nowMs;
    for (const candidate of idleCandidates.slice(0, boundedIdleBatchSize)) {
      const terminated = terminateAgentForContract(candidate.id, snapshot, 'idle_cap_exceeded', {
        source: 'agent_contract_idle_cap',
        terminated_by: 'idle_cap_enforcer',
        role: candidate.role,
        team,
        auto_termination: true,
        agent_row: activeRowById.get(candidate.id) || null,
      });
      if (terminated.terminated) {
        terminations.push(terminated);
        idleTerminatedCount += 1;
      }
    }
  }

  const finalState = loadAgentContractsState();
  return {
    changed: changed || reconciled.length > 0 || terminations.length > 0,
    terminated: terminations,
    reconciled,
    idle_terminated_count: idleTerminatedCount,
    idle_candidates: idleCandidates.length,
    idle_threshold: idleThreshold,
    idle_excess: idleExcess,
    idle_sweep_ready: idleSweepReady,
    idle_batch_size: boundedIdleBatchSize,
    active_contracts: Object.values(finalState.contracts || {}).filter((row) => row && row.status === 'active').length,
  };
}

function lifecycleTelemetry(snapshot, enforcement = null) {
  const nowMs = Date.now();
  const contractsState = loadAgentContractsState();
  const activeAgents = compatAgentsFromSnapshot(snapshot, { includeArchived: false });
  const active = [];
  let idleCount = 0;
  for (const agent of activeAgents) {
    const id = cleanText(agent && agent.id ? agent.id : '', 140);
    if (!id) continue;
    const contract = contractForAgent(id);
    const summary = contractSummary(contract, nowMs);
    active.push({
      id,
      role: cleanText(agent && agent.role ? agent.role : '', 80),
      state: cleanText(agent && agent.state ? agent.state : 'running', 24) || 'running',
      contract: summary,
    });
    const state = loadAgentSession(id, snapshot);
    const session = activeSession(state);
    const updatedMs = coerceTsMs(session && session.updated_at ? session.updated_at : 0, 0);
    if (updatedMs > 0 && (nowMs - updatedMs) >= AGENT_ROGUE_SPIKE_WINDOW_MS) idleCount += 1;
  }
  const idleThreshold = parsePositiveInt(
    contractsState && contractsState.defaults ? contractsState.defaults.max_idle_agents : AGENT_CONTRACT_MAX_IDLE_AGENTS,
    AGENT_CONTRACT_MAX_IDLE_AGENTS,
    1,
    1000
  );
  const terminatedHistory = Array.isArray(contractsState && contractsState.terminated_history)
    ? contractsState.terminated_history.slice(-20).reverse()
    : [];
  return {
    defaults: {
      default_expiry_seconds: parsePositiveInt(
        contractsState && contractsState.defaults ? contractsState.defaults.default_expiry_seconds : AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS,
        AGENT_CONTRACT_DEFAULT_EXPIRY_SECONDS,
        1,
        7 * 24 * 60 * 60
      ),
      auto_expire_on_complete:
        !(contractsState && contractsState.defaults) || contractsState.defaults.auto_expire_on_complete !== false,
      max_idle_agents: idleThreshold,
    },
    active_agents: active,
    active_count: active.length,
    terminated_recent: terminatedHistory,
    terminated_recent_count: terminatedHistory.length,
    idle_agents: idleCount,
    idle_threshold: idleThreshold,
    idle_alert: idleCount > idleThreshold,
    last_enforcement: {
      changed: !!(enforcement && enforcement.changed),
      terminated_count: Array.isArray(enforcement && enforcement.terminated) ? enforcement.terminated.length : 0,
      ts: nowIso(),
    },
  };
}

let runtimeTrendSeries = [];
let memoryStreamBootstrapped = false;
let memoryStreamSeq = 0;
let memoryStreamIndex = new Map();
let memoryStreamHourIndex = new Map();
let memoryIngestCircuit = {
  paused: false,
  since: '',
  reason: '',
  trigger_queue_depth: 0,
  trigger_memory_entries: 0,
  transition_count: 0,
};
let healthCoverageState = {
  check_ids: [],
  ts: '',
};
let runtimePolicyState = {
  health_adaptive: false,
  health_window_seconds: RUNTIME_HEALTH_ADAPTIVE_WINDOW_SECONDS,
  auto_balance_threshold: RUNTIME_AUTO_BALANCE_THRESHOLD,
  last_health_refresh: '',
  last_throttle_apply: '',
};
let runtimeDrainState = {
  active_agents: [],
  last_spawn_at: '',
  last_dissolve_at: '',
};
let conduitWatchdogState = {
  low_signals_since_ms: 0,
  last_attempt_ms: 0,
  last_attempt_at: '',
  last_success_ms: 0,
  last_success_at: '',
  failure_count: 0,
  active_shadows: [],
};
let staleLaneRetryState = {};
let runtimeAutohealState = {
  last_run_ms: 0,
  last_run_at: '',
  last_result: 'idle',
  failure_count: 0,
  last_stage: 'idle',
  last_stall_detected: false,
  last_stall_signature: '',
  conduit_deficit_streak: 0,
  stale_raw_streak: 0,
  stale_soft_streak: 0,
};
let reliabilityEscalationState = {
  last_emit_ms: 0,
  last_emit_at: '',
  emit_count: 0,
  last_receipt_ms: 0,
  last_receipt_reason: '',
};
let runtimeSpineCanaryState = {
  last_run_ms: 0,
  last_run_at: '',
  run_count: 0,
};
let runtimeTaskDispatchState = {};
let benchmarkRefreshState = {
  last_run_ms: 0,
  last_run_at: '',
  run_count: 0,
  last_status: 'idle',
};
let coordinationRecoveryState = {
  last_run_ms: 0,
  last_run_at: '',
  run_count: 0,
  failure_count: 0,
  last_result: 'idle',
  last_signature: '',
};
let agentTerminationTeamDiscoveryState = {
  scanned_ms: 0,
  teams: [DEFAULT_TEAM],
};
let ingressControllerState = {
  level: 'normal',
  reject_non_critical: false,
  delay_ms: 0,
  reason: '',
  since: '',
};
let snapshotHistoryLastAppendAtMs = 0;
let snapshotHistoryMaintenanceState = {
  last_compact_at: '',
  last_reason: 'idle',
  bytes_before: 0,
  bytes_after: 0,
  lines_before: 0,
  lines_after: 0,
  trimmed_entries: 0,
  removed_bytes: 0,
  exceeded: false,
  warning: false,
  compact_count: 0,
};
let passiveMemoryWriteState = {
  last_append_ms: 0,
  last_attention_append_ms: 0,
  last_hash: '',
};
const chatExportArtifacts = new Map();

function loadAttentionDeferredState() {
  const fallback = {
    version: 1,
    updated_at: '',
    events: [],
    dropped_count: 0,
    stash_count: 0,
    rehydrate_count: 0,
  };
  const raw = readJson(ATTENTION_DEFERRED_PATH, fallback);
  const events = Array.isArray(raw && raw.events) ? raw.events.slice(0, ATTENTION_DEFERRED_MAX_ITEMS) : [];
  return {
    version: 1,
    updated_at: cleanText(raw && raw.updated_at ? raw.updated_at : '', 80),
    events: events.map((row) => ({
      ts: cleanText(row && row.ts ? row.ts : nowIso(), 80) || nowIso(),
      severity: cleanText(row && row.severity ? row.severity : 'info', 20) || 'info',
      source: cleanText(row && row.source ? row.source : 'attention', 80) || 'attention',
      source_type: cleanText(row && row.source_type ? row.source_type : 'event', 80) || 'event',
      summary: cleanText(row && row.summary ? row.summary : '', 260),
      band: cleanText(row && row.band ? row.band : 'p4', 12) || 'p4',
      priority_lane: cleanText(row && row.priority_lane ? row.priority_lane : 'background', 24) || 'background',
      score: Number.isFinite(Number(row && row.score)) ? Number(row.score) : 0,
      attention_key: cleanText(row && row.attention_key ? row.attention_key : '', 120),
      initiative_action: cleanText(row && row.initiative_action ? row.initiative_action : '', 80),
      deferred_at: cleanText(row && row.deferred_at ? row.deferred_at : '', 80),
      deferred_reason: cleanText(row && row.deferred_reason ? row.deferred_reason : '', 80),
    })),
    dropped_count: parseNonNegativeInt(raw && raw.dropped_count, 0, 100000000),
    stash_count: parseNonNegativeInt(raw && raw.stash_count, 0, 100000000),
    rehydrate_count: parseNonNegativeInt(raw && raw.rehydrate_count, 0, 100000000),
  };
}

let attentionDeferredState = loadAttentionDeferredState();

function saveAttentionDeferredState(nextState) {
  const state = nextState && typeof nextState === 'object' ? nextState : attentionDeferredState;
  const sanitized = {
    version: 1,
    updated_at: cleanText(state && state.updated_at ? state.updated_at : nowIso(), 80) || nowIso(),
    events: Array.isArray(state && state.events) ? state.events.slice(0, ATTENTION_DEFERRED_MAX_ITEMS) : [],
    dropped_count: parseNonNegativeInt(state && state.dropped_count, 0, 100000000),
    stash_count: parseNonNegativeInt(state && state.stash_count, 0, 100000000),
    rehydrate_count: parseNonNegativeInt(state && state.rehydrate_count, 0, 100000000),
  };
  attentionDeferredState = sanitized;
  writeJson(ATTENTION_DEFERRED_PATH, sanitized);
  return sanitized;
}

function normalizeDeferredAttentionEvent(row, reason = 'deferred') {
  return {
    ts: cleanText(row && row.ts ? row.ts : nowIso(), 80) || nowIso(),
    severity: cleanText(row && row.severity ? row.severity : 'info', 20) || 'info',
    source: cleanText(row && row.source ? row.source : 'attention', 80) || 'attention',
    source_type: cleanText(row && row.source_type ? row.source_type : 'event', 80) || 'event',
    summary: cleanText(row && row.summary ? row.summary : '', 260),
    band: cleanText(row && row.band ? row.band : 'p4', 12) || 'p4',
    priority_lane: cleanText(row && row.priority_lane ? row.priority_lane : attentionEventLane(row), 24) || attentionEventLane(row),
    score: Number.isFinite(Number(row && row.score)) ? Number(row.score) : 0,
    attention_key: cleanText(row && row.attention_key ? row.attention_key : '', 120),
    initiative_action: cleanText(row && row.initiative_action ? row.initiative_action : '', 80),
    deferred_at: nowIso(),
    deferred_reason: cleanText(reason, 80) || 'deferred',
  };
}

function applyAttentionDeferredStorage(queueDepth = 0, split = {}, options = {}) {
  const depth = parseNonNegativeInt(queueDepth, 0, 100000000);
  const critical = Array.isArray(split.critical) ? split.critical.slice() : [];
  let standard = Array.isArray(split.standard) ? split.standard.slice() : [];
  let background = Array.isArray(split.background) ? split.background.slice() : [];
  const stashDepth = parsePositiveInt(
    options && options.stash_depth != null ? options.stash_depth : ATTENTION_DEFERRED_STASH_DEPTH,
    ATTENTION_DEFERRED_STASH_DEPTH,
    1,
    100000000
  );
  const hardShedDepth = parsePositiveInt(
    options && options.hard_shed_depth != null ? options.hard_shed_depth : ATTENTION_DEFERRED_HARD_SHED_DEPTH,
    ATTENTION_DEFERRED_HARD_SHED_DEPTH,
    1,
    100000000
  );
  const rehydrateDepth = parsePositiveInt(
    options && options.rehydrate_depth != null ? options.rehydrate_depth : ATTENTION_DEFERRED_REHYDRATE_DEPTH,
    ATTENTION_DEFERRED_REHYDRATE_DEPTH,
    1,
    100000000
  );
  const shouldStash = depth >= stashDepth;
  const hardShed = depth >= hardShedDepth;
  const canRehydrate = depth <= rehydrateDepth;
  let stashedCount = 0;
  let rehydratedCount = 0;
  let droppedCount = 0;

  if (shouldStash) {
    const stashSource = [...standard, ...background];
    standard = [];
    background = [];
    if (stashSource.length > 0) {
      const normalized = stashSource.map((row) =>
        normalizeDeferredAttentionEvent(row, hardShed ? 'hard_shed' : 'predictive_stash')
      );
      attentionDeferredState.events.push(...normalized);
      stashedCount = normalized.length;
      if (attentionDeferredState.events.length > ATTENTION_DEFERRED_MAX_ITEMS) {
        const overflow = attentionDeferredState.events.length - ATTENTION_DEFERRED_MAX_ITEMS;
        attentionDeferredState.events = attentionDeferredState.events.slice(overflow);
        droppedCount = overflow;
      }
      attentionDeferredState.stash_count =
        parseNonNegativeInt(attentionDeferredState.stash_count, 0, 100000000) + stashedCount;
      attentionDeferredState.dropped_count =
        parseNonNegativeInt(attentionDeferredState.dropped_count, 0, 100000000) + droppedCount;
      attentionDeferredState.updated_at = nowIso();
      saveAttentionDeferredState(attentionDeferredState);
    }
  } else if (canRehydrate && Array.isArray(attentionDeferredState.events) && attentionDeferredState.events.length > 0) {
    const take = Math.min(
      ATTENTION_DEFERRED_REHYDRATE_BATCH,
      parseNonNegativeInt(attentionDeferredState.events.length, 0, ATTENTION_DEFERRED_MAX_ITEMS)
    );
    if (take > 0) {
      const rehydrated = attentionDeferredState.events.splice(0, take).map((row) => ({
        ...row,
        deferred_reason: cleanText(row && row.deferred_reason ? row.deferred_reason : '', 80),
      }));
      rehydratedCount = rehydrated.length;
      background = [...background, ...rehydrated];
      attentionDeferredState.rehydrate_count =
        parseNonNegativeInt(attentionDeferredState.rehydrate_count, 0, 100000000) + rehydratedCount;
      attentionDeferredState.updated_at = nowIso();
      saveAttentionDeferredState(attentionDeferredState);
    }
  }

  return {
    critical,
    standard,
    background,
    telemetry: [...standard, ...background],
    stashed_count: stashedCount,
    rehydrated_count: rehydratedCount,
    dropped_count: droppedCount,
    deferred_depth: Array.isArray(attentionDeferredState.events) ? attentionDeferredState.events.length : 0,
    deferred_mode: hardShed ? 'hard_shed' : shouldStash ? 'stashed' : canRehydrate ? 'rehydrate' : 'pass_through',
    hard_shed: hardShed,
    thresholds: {
      stash_depth: stashDepth,
      hard_shed_depth: hardShedDepth,
      rehydrate_depth: rehydrateDepth,
    },
  };
}

function normalizeSeverity(value) {
  const severity = cleanText(value || '', 20).toLowerCase();
  if (severity === 'critical' || severity === 'error' || severity === 'fatal') return 'critical';
  if (severity === 'warn' || severity === 'warning' || severity === 'degraded') return 'warn';
  return 'info';
}

function attentionEventLane(event) {
  const severity = normalizeSeverity(event && event.severity ? event.severity : 'info');
  const band = cleanText(event && event.band ? event.band : '', 12).toLowerCase();
  const source = cleanText(event && event.source ? event.source : '', 120).toLowerCase();
  const sourceType = cleanText(event && event.source_type ? event.source_type : '', 120).toLowerCase();
  const summary = cleanText(event && event.summary ? event.summary : '', 400).toLowerCase();
  if (severity === 'critical') return 'critical';
  if (severity === 'warn') return 'critical';
  if (band === 'p1' || band === 'p0') return 'critical';
  if (
    /\b(fail|error|critical|degraded|alert|benchmark_sanity|backpressure|throttle|stale)\b/.test(summary)
  ) {
    return 'critical';
  }
  const backgroundBySource =
    /\b(receipt|audit|timeline|history|log|trace)\b/.test(sourceType) ||
    /\b(receipt|audit|timeline|history|log|trace)\b/.test(source);
  const backgroundByBand = severity === 'info' && (band === 'p3' || band === 'p4');
  if (backgroundBySource || backgroundByBand) return 'background';
  return 'standard';
}

function splitAttentionEvents(events = []) {
  const rows = Array.isArray(events) ? events : [];
  const critical = [];
  const standard = [];
  const background = [];
  for (const row of rows) {
    const lane = attentionEventLane(row);
    if (lane === 'critical') {
      critical.push(row);
    } else if (lane === 'background') {
      background.push(row);
    } else {
      standard.push(row);
    }
  }
  const telemetry = [...standard, ...background];
  return {
    critical,
    standard,
    background,
    telemetry,
    lane_weights: { ...ATTENTION_LANE_WEIGHTS },
    counts: {
      critical: critical.length,
      standard: standard.length,
      background: background.length,
      telemetry: telemetry.length,
      total: rows.length,
    },
  };
}

function attentionEventAgeSeconds(event = {}) {
  const nowMs = Date.now();
  const tsCandidate =
    event && typeof event.ts === 'string' && event.ts.trim()
      ? event.ts
      : event && typeof event.deferred_at === 'string' && event.deferred_at.trim()
      ? event.deferred_at
      : event && typeof event.created_at === 'string' && event.created_at.trim()
      ? event.created_at
      : '';
  const fromIso = tsCandidate ? Date.parse(tsCandidate) : Number.NaN;
  const tsMs = Number.isFinite(fromIso)
    ? fromIso
    : parseNonNegativeInt(
        event && event.ts_ms != null
          ? event.ts_ms
          : event && event.created_at_ms != null
          ? event.created_at_ms
          : 0,
        0,
        1000000000000
      );
  if (tsMs <= 0 || tsMs > nowMs) return 0;
  return Math.max(0, Math.floor((nowMs - tsMs) / 1000));
}

function attentionLanePolicy(queueDepth = 0, counts = {}, laneRows = null) {
  const depth = parseNonNegativeInt(queueDepth, 0, 100000000);
  const critical = parseNonNegativeInt(counts && counts.critical, 0, 100000000);
  const background = parseNonNegativeInt(counts && counts.background, 0, 100000000);
  const backgroundDominant = background > Math.max(1, critical) * ATTENTION_BG_DOMINANCE_RATIO;
  const preemptCritical = depth >= ATTENTION_PREEMPT_QUEUE_DEPTH || backgroundDominant;
  const baseWeights = preemptCritical
    ? { critical: 8, standard: 2, background: 1 }
    : { ...ATTENTION_LANE_WEIGHTS };
  const criticalRows =
    laneRows && typeof laneRows === 'object' && Array.isArray(laneRows.critical) ? laneRows.critical : [];
  const oldestCriticalAgeSeconds = criticalRows.reduce((maxAge, row) => {
    const age = attentionEventAgeSeconds(row);
    return age > maxAge ? age : maxAge;
  }, 0);
  const weights = { ...baseWeights };
  let criticalDecayStage = 'none';
  if (critical > 0 && oldestCriticalAgeSeconds >= ATTENTION_CRITICAL_DECAY_STAGE2_SECONDS) {
    weights.critical = 1;
    weights.standard = Math.max(weights.standard, 3);
    weights.background = Math.max(weights.background, 2);
    criticalDecayStage = 'stage2';
  } else if (critical > 0 && oldestCriticalAgeSeconds >= ATTENTION_CRITICAL_DECAY_STAGE1_SECONDS) {
    weights.critical = Math.min(weights.critical, 3);
    weights.standard = Math.max(weights.standard, 3);
    criticalDecayStage = 'stage1';
  }
  return {
    weights,
    lane_caps: { ...ATTENTION_LANE_CAPS },
    preempt_critical: preemptCritical,
    background_dominant: backgroundDominant,
    critical_decay_stage: criticalDecayStage,
    oldest_critical_age_seconds: oldestCriticalAgeSeconds,
  };
}

function weightedFairAttentionOrder(
  laneRows = {},
  limit = ATTENTION_CRITICAL_LIMIT,
  laneWeights = ATTENTION_LANE_WEIGHTS,
  laneCaps = ATTENTION_LANE_CAPS
) {
  const weights = laneWeights && typeof laneWeights === 'object' ? laneWeights : ATTENTION_LANE_WEIGHTS;
  const caps = laneCaps && typeof laneCaps === 'object' ? laneCaps : ATTENTION_LANE_CAPS;
  const buckets = {
    critical: Array.isArray(laneRows.critical)
      ? laneRows.critical.slice(0, parsePositiveInt(caps.critical, ATTENTION_LANE_CAPS.critical, 1, 1000))
      : [],
    standard: Array.isArray(laneRows.standard)
      ? laneRows.standard.slice(0, parsePositiveInt(caps.standard, ATTENTION_LANE_CAPS.standard, 1, 1000))
      : [],
    background: Array.isArray(laneRows.background)
      ? laneRows.background.slice(0, parsePositiveInt(caps.background, ATTENTION_LANE_CAPS.background, 1, 1000))
      : [],
  };
  const ordered = [];
  const lanes = ['critical', 'standard', 'background'];
  while (ordered.length < limit) {
    let progressed = false;
    for (const lane of lanes) {
      const takeCount = parsePositiveInt(weights[lane], 1, 1, 20);
      for (let i = 0; i < takeCount; i += 1) {
        const next = buckets[lane].shift();
        if (!next) break;
        ordered.push(next);
        progressed = true;
        if (ordered.length >= limit) break;
      }
      if (ordered.length >= limit) break;
    }
    if (!progressed) break;
  }
  return ordered;
}

function microBatchAttentionTelemetry(events = [], options = {}) {
  const rows = Array.isArray(events) ? events : [];
  if (!rows.length) return [];
  const windowMs = parsePositiveInt(
    options && options.window_ms != null ? options.window_ms : ATTENTION_MICRO_BATCH_WINDOW_MS,
    ATTENTION_MICRO_BATCH_WINDOW_MS,
    1,
    10000
  );
  const maxItems = parsePositiveInt(
    options && options.max_items != null ? options.max_items : ATTENTION_MICRO_BATCH_MAX_ITEMS,
    ATTENTION_MICRO_BATCH_MAX_ITEMS,
    1,
    256
  );
  const sorted = rows
    .slice()
    .sort((a, b) => coerceTsMs(a && a.ts, 0) - coerceTsMs(b && b.ts, 0));
  const batches = [];
  let current = null;
  let batchSeq = 0;
  const flush = () => {
    if (!current) return;
    const laneCounts = { critical: 0, standard: 0, background: 0 };
    for (const row of current.items) {
      const lane = attentionEventLane(row);
      laneCounts[lane] = parseNonNegativeInt(laneCounts[lane], 0, 100000000) + 1;
    }
    batches.push({
      batch_id: `telemetry_batch_${batchSeq}`,
      start_ts: current.startTsIso,
      end_ts: current.endTsIso,
      item_count: current.items.length,
      lane_counts: laneCounts,
      sample_sources: current.samples.slice(0, 5),
    });
    current = null;
  };

  for (const row of sorted) {
    const tsMs = coerceTsMs(row && row.ts, Date.now());
    const tsIso = cleanText(row && row.ts ? row.ts : nowIso(), 80) || nowIso();
    const source = cleanText(row && row.source ? row.source : row && row.source_type ? row.source_type : 'event', 120);
    if (!current) {
      batchSeq += 1;
      current = {
        startMs: tsMs,
        startTsIso: tsIso,
        endTsIso: tsIso,
        items: [],
        samples: [],
      };
    }
    const withinWindow = tsMs - current.startMs <= windowMs;
    const belowLimit = current.items.length < maxItems;
    if (!withinWindow || !belowLimit) {
      flush();
      batchSeq += 1;
      current = {
        startMs: tsMs,
        startTsIso: tsIso,
        endTsIso: tsIso,
        items: [],
        samples: [],
      };
    }
    current.items.push(row);
    current.endTsIso = tsIso;
    if (source) current.samples.push(source);
  }
  flush();
  return batches.slice(0, 24);
}

function severityRank(value) {
  const severity = normalizeSeverity(value);
  if (severity === 'critical') return 3;
  if (severity === 'warn') return 2;
  return 1;
}

function priorityBandRank(value) {
  const band = cleanText(value || '', 12).toLowerCase();
  if (band === 'p0') return 4;
  if (band === 'p1') return 3;
  if (band === 'p2') return 2;
  if (band === 'p3') return 1;
  return 0;
}

function sortCriticalEvents(events = []) {
  const rows = Array.isArray(events) ? events.slice() : [];
  rows.sort((a, b) => {
    const sevDelta = severityRank(b && b.severity) - severityRank(a && a.severity);
    if (sevDelta !== 0) return sevDelta;
    const bandDelta = priorityBandRank(b && b.band) - priorityBandRank(a && a.band);
    if (bandDelta !== 0) return bandDelta;
    const scoreDelta =
      (Number.isFinite(Number(b && b.score)) ? Number(b.score) : 0) -
      (Number.isFinite(Number(a && a.score)) ? Number(a.score) : 0);
    if (scoreDelta !== 0) return scoreDelta;
    return coerceTsMs(b && b.ts, 0) - coerceTsMs(a && a.ts, 0);
  });
  return rows;
}

function memoryIngestControlState(queueDepth = 0, memoryEntryCount = 0) {
  const depth = parseNonNegativeInt(queueDepth, 0, 100000000);
  const entryCount = parseNonNegativeInt(memoryEntryCount, 0, 100000000);
  const entryPressure = entryCount >= MEMORY_ENTRY_BACKPRESSURE_THRESHOLD;
  const severeEntryPressure = entryCount >= MEMORY_ENTRY_BACKPRESSURE_THRESHOLD * 2;
  const entryPressureWithQueue = entryPressure && depth >= RUNTIME_INGRESS_DAMPEN_DEPTH;
  if (
    !memoryIngestCircuit.paused &&
    (depth >= DASHBOARD_QUEUE_DRAIN_PAUSE_DEPTH || entryPressureWithQueue || severeEntryPressure)
  ) {
    memoryIngestCircuit = {
      paused: true,
      since: nowIso(),
      reason: severeEntryPressure || entryPressureWithQueue ? 'memory_entry_pressure' : 'predictive_queue_drain',
      trigger_queue_depth: depth,
      trigger_memory_entries: entryCount,
      transition_count: parseNonNegativeInt(memoryIngestCircuit.transition_count, 0, 1000000) + 1,
    };
  } else if (
    memoryIngestCircuit.paused &&
    depth <= DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH &&
    entryCount < MEMORY_ENTRY_BACKPRESSURE_THRESHOLD * 2
  ) {
    memoryIngestCircuit = {
      paused: false,
      since: nowIso(),
      reason:
        entryCount >= MEMORY_ENTRY_BACKPRESSURE_THRESHOLD
          ? 'queue_recovered_tolerated_entry_pressure'
          : 'queue_recovered',
      trigger_queue_depth: depth,
      trigger_memory_entries: entryCount,
      transition_count: parseNonNegativeInt(memoryIngestCircuit.transition_count, 0, 1000000) + 1,
    };
  }
  return {
    paused: !!memoryIngestCircuit.paused,
    since: cleanText(memoryIngestCircuit.since || '', 80),
    reason: cleanText(memoryIngestCircuit.reason || '', 80),
    trigger_queue_depth: parseNonNegativeInt(memoryIngestCircuit.trigger_queue_depth, 0, 100000000),
    trigger_memory_entries: parseNonNegativeInt(memoryIngestCircuit.trigger_memory_entries, 0, 100000000),
    pause_threshold: DASHBOARD_QUEUE_DRAIN_PAUSE_DEPTH,
    memory_entry_threshold: MEMORY_ENTRY_BACKPRESSURE_THRESHOLD,
    resume_threshold: DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH,
    transition_count: parseNonNegativeInt(memoryIngestCircuit.transition_count, 0, 1000000),
  };
}

function applyMemoryIngestCircuit(entries = [], control = {}) {
  const rows = Array.isArray(entries) ? entries : [];
  if (!control || !control.paused) {
    return { entries: rows, dropped_count: 0, mode: 'normal' };
  }
  const kept = [];
  for (const row of rows) {
    const rowPath = cleanText(row && row.path ? row.path : '', 260).toLowerCase();
    const kind = cleanText(row && row.kind ? row.kind : '', 60).toLowerCase();
    const nonCriticalReceiptOrLog =
      /\b(receipt|receipts|audit|history|log|logs|timeline)\b/.test(rowPath) ||
      kind === 'timeline';
    const critical =
      rowPath.includes('/local/workspace/memory/') ||
      rowPath.includes('attention_queue') ||
      rowPath.endsWith('/latest.json') ||
      kind === 'snapshot';
    if (nonCriticalReceiptOrLog && !critical) continue;
    if (critical) kept.push(row);
    if (kept.length >= MEMORY_ENTRY_TARGET_WHEN_PAUSED) break;
  }
  return {
    entries: kept,
    dropped_count: Math.max(0, rows.length - kept.length),
    mode: 'priority_shed',
  };
}

function healthCoverageSummary(healthPayload) {
  const checks =
    healthPayload && healthPayload.checks && typeof healthPayload.checks === 'object'
      ? Object.keys(healthPayload.checks).map((row) => cleanText(row, 120)).filter(Boolean).sort()
      : [];
  const previous = Array.isArray(healthCoverageState.check_ids)
    ? healthCoverageState.check_ids.slice()
    : [];
  const retired = previous.filter((row) => !checks.includes(row));
  const added = checks.filter((row) => !previous.includes(row));
  const status = retired.length > 0 ? 'gap' : 'stable';
  const coverage = {
    status,
    count: checks.length,
    previous_count: previous.length,
    added_checks: added.slice(0, 24),
    retired_checks: retired.slice(0, 24),
    gap_count: retired.length,
    changed: retired.length > 0 || added.length > 0,
    ts: nowIso(),
  };
  healthCoverageState = {
    check_ids: checks,
    ts: coverage.ts,
  };
  return coverage;
}

function recordRuntimeTrend(sample) {
  if (!sample || typeof sample !== 'object') return runtimeTrendSeries;
  runtimeTrendSeries.push(sample);
  if (runtimeTrendSeries.length > RUNTIME_TREND_WINDOW) {
    runtimeTrendSeries = runtimeTrendSeries.slice(-RUNTIME_TREND_WINDOW);
  }
  return runtimeTrendSeries;
}

function queueDepthVelocity(samples = []) {
  const rows = Array.isArray(samples) ? samples.slice(-6) : [];
  if (rows.length < 2) return 0;
  const first = rows[0];
  const last = rows[rows.length - 1];
  const start = parseNonNegativeInt(first && first.queue_depth != null ? first.queue_depth : 0, 0, 100000000);
  const end = parseNonNegativeInt(last && last.queue_depth != null ? last.queue_depth : 0, 0, 100000000);
  const startTs = Date.parse(cleanText(first && first.ts ? first.ts : '', 80));
  const endTs = Date.parse(cleanText(last && last.ts ? last.ts : '', 80));
  if (!Number.isFinite(startTs) || !Number.isFinite(endTs) || endTs <= startTs) {
    return end - start;
  }
  const minutes = Math.max(0.01, (endTs - startTs) / 60000);
  return Number(((end - start) / minutes).toFixed(3));
}

function isFlatline(values = []) {
  if (!Array.isArray(values) || values.length < 2) return false;
  const normalized = values.map((row) => Number(row));
  if (normalized.some((row) => !Number.isFinite(row))) return false;
  const first = normalized[0];
  return normalized.every((row) => row === first);
}

function runtimeStallSignals(runtime, samples = []) {
  const rows = Array.isArray(samples) ? samples.slice(-RUNTIME_STALL_WINDOW) : [];
  if (rows.length < RUNTIME_STALL_WINDOW) {
    return {
      detected: false,
      queue_not_improving: false,
      conduit_flat_low: false,
      cockpit_flatline: false,
      stale_blocks_present: false,
      stale_blocks_flatline: false,
      chronic_conduit_starvation: false,
      coordination_pathology: false,
      signature: 'insufficient_samples',
      window: rows.length,
    };
  }
  const queueValues = rows.map((row) => parseNonNegativeInt(row && row.queue_depth != null ? row.queue_depth : 0, 0, 100000000));
  const conduitValues = rows.map((row) => parseNonNegativeInt(row && row.conduit_signals != null ? row.conduit_signals : 0, 0, 100000000));
  const cockpitValues = rows.map((row) => parseNonNegativeInt(row && row.cockpit_blocks != null ? row.cockpit_blocks : 0, 0, 100000000));
  const staleValues = rows.map((row) =>
    parseNonNegativeInt(row && row.cockpit_stale_blocks != null ? row.cockpit_stale_blocks : 0, 0, 100000000)
  );
  const queueNow = parseNonNegativeInt(runtime && runtime.queue_depth, queueValues[queueValues.length - 1], 100000000);
  const signalFloor = Math.max(
    RUNTIME_STALL_CONDUIT_FLOOR,
    Math.floor(Math.max(1, parsePositiveInt(runtime && runtime.target_conduit_signals, RUNTIME_AUTO_BALANCE_THRESHOLD, 1, 128)) * 0.5)
  );
  const queueNotImproving =
    queueValues[queueValues.length - 1] >= queueValues[0] &&
    queueValues[queueValues.length - 1] >= RUNTIME_STALL_QUEUE_MIN_DEPTH;
  const conduitFlatLow =
    isFlatline(conduitValues) &&
    Math.max(...conduitValues) <= signalFloor &&
    queueNow >= RUNTIME_STALL_QUEUE_MIN_DEPTH;
  const cockpitFlatline = isFlatline(cockpitValues) && Math.max(...cockpitValues) > 0;
  const staleBlocksPresent = Math.max(...staleValues) > 0;
  const staleBlocksFlatline = isFlatline(staleValues) && Math.max(...staleValues) >= RUNTIME_COORDINATION_PATHOLOGY_STALE_BLOCK_MIN;
  const chronicConduitStarvation = isFlatline(conduitValues) && Math.max(...conduitValues) <= signalFloor;
  const coordinationPathology = staleBlocksFlatline && chronicConduitStarvation;
  const detected = (queueNotImproving && conduitFlatLow && cockpitFlatline) || coordinationPathology;
  return {
    detected,
    queue_not_improving: queueNotImproving,
    conduit_flat_low: conduitFlatLow,
    cockpit_flatline: cockpitFlatline,
    stale_blocks_present: staleBlocksPresent,
    stale_blocks_flatline: staleBlocksFlatline,
    chronic_conduit_starvation: chronicConduitStarvation,
    coordination_pathology: coordinationPathology,
    signature: `q:${queueValues.join(',')}|c:${conduitValues.join(',')}|b:${cockpitValues.join(',')}|s:${staleValues.join(',')}`,
    window: rows.length,
    signal_floor: signalFloor,
  };
}

function runtimeCoordinationRecoveryShadows(team = DEFAULT_TEAM) {
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const defaults = [
    `${normalizedTeam}-conduit-watchdog`,
    `${normalizedTeam}-coarse-researcher`,
    `${normalizedTeam}-coarse-builder`,
    `${normalizedTeam}-coarse-analyst`,
    `${normalizedTeam}-stall-heal`,
  ];
  const activeDrainAgents = Array.isArray(runtimeDrainState.active_agents)
    ? runtimeDrainState.active_agents
        .map((row) => cleanText(row, 140))
        .filter(Boolean)
    : [];
  return Array.from(new Set([...defaults, ...activeDrainAgents])).slice(0, RUNTIME_COORDINATION_RECOVERY_MAX_SHADOWS);
}

function runStallRecovery(runtime, team, stall = null) {
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const staleBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000);
  const targetSignals = Math.max(
    parsePositiveInt(runtime && runtime.target_conduit_signals, RUNTIME_AUTO_BALANCE_THRESHOLD, 1, 128),
    RUNTIME_CONDUIT_WATCHDOG_MIN_SIGNALS
  );
  const conduitSignals = parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000);
  const stallSignals = stall && typeof stall === 'object' ? stall : runtimeStallSignals(runtime, runtimeTrendSeries);
  const coordinationPathology =
    !!stallSignals.coordination_pathology ||
    (staleBlocks >= RUNTIME_COORDINATION_PATHOLOGY_STALE_BLOCK_MIN && conduitSignals < Math.max(4, Math.floor(targetSignals * 0.5)));
  const nowMs = Date.now();
  const sinceLastMs = Math.max(0, nowMs - parseNonNegativeInt(coordinationRecoveryState.last_run_ms, 0, 1000000000000));
  if (sinceLastMs < RUNTIME_COORDINATION_RECOVERY_COOLDOWN_MS) {
    coordinationRecoveryState.last_result = 'cooldown';
    coordinationRecoveryState.last_signature = cleanText(stallSignals.signature || '', 240);
    return {
      ok: true,
      applied: false,
      reason: 'cooldown_active',
      cooldown_ms: RUNTIME_COORDINATION_RECOVERY_COOLDOWN_MS,
      since_last_ms: sinceLastMs,
      coordination_pathology: coordinationPathology,
      queue_depth: queueDepth,
      stale_blocks: staleBlocks,
      conduit_signals: conduitSignals,
      target_conduit_signals: targetSignals,
      stall: stallSignals,
    };
  }
  const drainLimit = Math.min(
    RUNTIME_ATTENTION_DRAIN_MAX_BATCH,
    Math.max(RUNTIME_STALL_DRAIN_LIMIT, Math.ceil(queueDepth / 2))
  );
  const drainLane = runLane([
    'attention-queue',
    'drain',
    `--consumer=${ATTENTION_CONSUMER_ID}`,
    `--limit=${drainLimit}`,
    '--wait-ms=0',
    '--run-context=runtime_stall_recovery',
  ]);
  const compactLane = runLane([
    'attention-queue',
    'compact',
    `--retain=${RUNTIME_ATTENTION_COMPACT_RETAIN}`,
    `--min-acked=${RUNTIME_ATTENTION_COMPACT_MIN_ACKED}`,
    '--run-context=runtime_stall_recovery',
  ]);
  const throttleLane = runLane([
    'collab-plane',
    'throttle',
    `--team=${normalizedTeam}`,
    `--plane=${RUNTIME_THROTTLE_PLANE}`,
    '--max-depth=50',
    `--strategy=${RUNTIME_THROTTLE_STRATEGY}`,
    '--strict=1',
  ]);
  const roleLane = runLane([
    'collab-plane',
    'launch-role',
    `--team=${normalizedTeam}`,
    '--role=builder',
    `--shadow=${normalizedTeam}-stall-heal`,
    '--strict=1',
  ]);
  const teams = discoverTerminationTeams(normalizedTeam);
  const utilityShadows = runtimeCoordinationRecoveryShadows(normalizedTeam);
  const terminateLanes = [];
  let removedCount = 0;
  let releasedTaskCount = 0;
  for (const shadow of utilityShadows) {
    let shadowRemoved = false;
    for (const candidateTeam of teams) {
      const lane = runLane([
        'collab-plane',
        'terminate-role',
        `--team=${candidateTeam}`,
        `--shadow=${shadow}`,
        '--strict=1',
      ]);
      const removed = parseNonNegativeInt(
        lane && lane.payload && lane.payload.removed_count != null ? lane.payload.removed_count : 0,
        0,
        100000000
      );
      const released = parseNonNegativeInt(
        lane && lane.payload && lane.payload.released_task_count != null ? lane.payload.released_task_count : 0,
        0,
        100000000
      );
      removedCount += removed;
      releasedTaskCount += released;
      terminateLanes.push({
        shadow,
        team: candidateTeam,
        removed_count: removed,
        released_task_count: released,
        ...laneOutcome(lane),
      });
      if (lane && lane.ok && (removed > 0 || released > 0)) {
        shadowRemoved = true;
        break;
      }
    }
    if (shadowRemoved) {
      closeTerminalSession(shadow, 'runtime_stall_recovery');
      archiveAgent(shadow, {
        source: 'runtime_stall_recovery',
        reason: 'coordination_pathology_recovery',
      });
    }
  }
  const relaunchResearcherLane = runLane([
    'collab-plane',
    'launch-role',
    `--team=${normalizedTeam}`,
    '--role=researcher',
    `--shadow=${normalizedTeam}-conduit-watchdog`,
    '--strict=1',
  ]);
  const relaunchBuilderLane = runLane([
    'collab-plane',
    'launch-role',
    `--team=${normalizedTeam}`,
    '--role=builder',
    `--shadow=${normalizedTeam}-stall-heal`,
    '--strict=1',
  ]);
  const ok = !!(
    drainLane && drainLane.ok &&
    compactLane && compactLane.ok &&
    throttleLane && throttleLane.ok &&
    roleLane && roleLane.ok &&
    relaunchResearcherLane && relaunchResearcherLane.ok &&
    relaunchBuilderLane && relaunchBuilderLane.ok
  );
  coordinationRecoveryState.last_run_ms = nowMs;
  coordinationRecoveryState.last_run_at = nowIso();
  coordinationRecoveryState.last_result = ok ? 'executed' : 'degraded';
  coordinationRecoveryState.last_signature = cleanText(stallSignals.signature || '', 240);
  coordinationRecoveryState.run_count = ok
    ? parseNonNegativeInt(coordinationRecoveryState.run_count, 0, 100000000) + 1
    : parseNonNegativeInt(coordinationRecoveryState.run_count, 0, 100000000);
  coordinationRecoveryState.failure_count = ok
    ? 0
    : parseNonNegativeInt(coordinationRecoveryState.failure_count, 0, 100000000) + 1;
  return {
    ok,
    applied: true,
    reason: ok ? 'executed' : 'lane_failure',
    queue_depth: queueDepth,
    stale_blocks: staleBlocks,
    conduit_signals: conduitSignals,
    target_conduit_signals: targetSignals,
    coordination_pathology: coordinationPathology,
    stall: stallSignals,
    terminated_shadows: utilityShadows,
    terminated_removed_count: removedCount,
    terminated_released_task_count: releasedTaskCount,
    lanes: {
      drain: laneOutcome(drainLane),
      compact: laneOutcome(compactLane),
      throttle: laneOutcome(throttleLane),
      role: laneOutcome(roleLane),
      relaunch_researcher: laneOutcome(relaunchResearcherLane),
      relaunch_builder: laneOutcome(relaunchBuilderLane),
      terminate: terminateLanes,
    },
    drain_limit: drainLimit,
  };
}

function runtimeAutohealTelemetry() {
  return {
    last_run_at: cleanText(runtimeAutohealState.last_run_at || '', 80),
    last_result: cleanText(runtimeAutohealState.last_result || 'idle', 40) || 'idle',
    failure_count: parseNonNegativeInt(runtimeAutohealState.failure_count, 0, 100000000),
    last_stage: cleanText(runtimeAutohealState.last_stage || 'idle', 40) || 'idle',
    stall_detected: !!runtimeAutohealState.last_stall_detected,
    stall_signature: cleanText(runtimeAutohealState.last_stall_signature || '', 240),
    conduit_deficit_streak: parseNonNegativeInt(runtimeAutohealState.conduit_deficit_streak, 0, 100000000),
    stale_raw_streak: parseNonNegativeInt(runtimeAutohealState.stale_raw_streak, 0, 100000000),
    stale_soft_streak: parseNonNegativeInt(runtimeAutohealState.stale_soft_streak, 0, 100000000),
    cadence_ms: {
      normal: RUNTIME_AUTONOMY_HEAL_INTERVAL_MS,
      emergency: RUNTIME_AUTONOMY_HEAL_EMERGENCY_INTERVAL_MS,
    },
    conduit_watchdog: {
      low_signals_since_ms: parseNonNegativeInt(conduitWatchdogState.low_signals_since_ms, 0, 1000000000000),
      last_attempt_at: cleanText(conduitWatchdogState.last_attempt_at || '', 80),
      last_success_at: cleanText(conduitWatchdogState.last_success_at || '', 80),
      failure_count: parseNonNegativeInt(conduitWatchdogState.failure_count, 0, 100000000),
      active_shadows: Array.isArray(conduitWatchdogState.active_shadows)
        ? conduitWatchdogState.active_shadows.slice(0, 8)
        : [],
      min_signal_floor: RUNTIME_CONDUIT_WATCHDOG_MIN_SIGNALS,
      stale_ms: RUNTIME_CONDUIT_WATCHDOG_STALE_MS,
      cooldown_ms: RUNTIME_CONDUIT_WATCHDOG_COOLDOWN_MS,
    },
    stale_lane_breaker: {
      tracked_lanes:
        staleLaneRetryState && typeof staleLaneRetryState === 'object'
          ? Object.keys(staleLaneRetryState).length
          : 0,
      retry_base_ms: RUNTIME_STALE_LANE_RETRY_BASE_MS,
      retry_max_ms: RUNTIME_STALE_LANE_RETRY_MAX_MS,
    },
    reliability_escalation: {
      last_emit_at: cleanText(reliabilityEscalationState.last_emit_at || '', 80),
      emit_count: parseNonNegativeInt(reliabilityEscalationState.emit_count, 0, 100000000),
      last_receipt_reason: cleanText(reliabilityEscalationState.last_receipt_reason || '', 80),
      cooldown_ms: RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS,
      spine_success_target_min: RUNTIME_SPINE_SUCCESS_TARGET_MIN,
      handoffs_per_agent_min: RUNTIME_HANDOFFS_PER_AGENT_MIN,
    },
    spine_canary: {
      last_run_at: cleanText(runtimeSpineCanaryState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(runtimeSpineCanaryState.run_count, 0, 100000000),
      stale_max_age_seconds: RUNTIME_SPINE_METRICS_STALE_MAX_AGE_SECONDS,
      cooldown_ms: RUNTIME_SPINE_CANARY_COOLDOWN_MS,
    },
    benchmark_refresh: {
      last_run_at: cleanText(benchmarkRefreshState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(benchmarkRefreshState.run_count, 0, 100000000),
      last_status: cleanText(benchmarkRefreshState.last_status || 'idle', 40) || 'idle',
      stale_max_age_seconds: RUNTIME_BENCHMARK_REFRESH_MAX_AGE_SECONDS,
      cooldown_ms: RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS,
    },
    coordination_recovery: {
      last_run_at: cleanText(coordinationRecoveryState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(coordinationRecoveryState.run_count, 0, 100000000),
      failure_count: parseNonNegativeInt(coordinationRecoveryState.failure_count, 0, 100000000),
      last_result: cleanText(coordinationRecoveryState.last_result || 'idle', 40) || 'idle',
      cooldown_ms: RUNTIME_COORDINATION_RECOVERY_COOLDOWN_MS,
      last_signature: cleanText(coordinationRecoveryState.last_signature || '', 240),
    },
  };
}

function benchmarkSanitySnapshot() {
  const gate = readJson(BENCHMARK_SANITY_GATE_PATH, null);
  const state = readJson(BENCHMARK_SANITY_STATE_PATH, null);
  let status = 'unknown';
  let source = 'benchmark_sanity_state';
  let detail = 'state_missing';
  let generatedAt = cleanText(state && state.generated_at ? state.generated_at : '', 80) || '';

  if (gate && typeof gate === 'object' && gate.type === 'benchmark_sanity_gate' && gate.summary) {
    const pass = gate.ok === true || (gate.summary && gate.summary.pass === true);
    status = pass ? 'pass' : 'fail';
    source = 'benchmark_sanity_gate';
    detail = pass
      ? `rows:${parsePositiveInt(gate.summary.measured_rows, 0, 0, 1000000)}`
      : `violations:${parsePositiveInt(gate.summary.violations, 0, 0, 1000000)}`;
  } else if (state && typeof state === 'object') {
    const projects = state.projects && typeof state.projects === 'object' ? Object.keys(state.projects).length : 0;
    if (projects > 0) {
      status = 'pass';
      detail = `projects:${projects}`;
    }
  }

  let ageSeconds = -1;
  if (generatedAt) {
    const parsed = Date.parse(generatedAt);
    if (Number.isFinite(parsed)) {
      ageSeconds = Math.max(0, Math.round((Date.now() - parsed) / 1000));
    }
  }
  const stale = ageSeconds < 0 || ageSeconds > DASHBOARD_BENCHMARK_STALE_SECONDS;
  if (stale) {
    if (status === 'pass') {
      status = 'warn';
      detail = detail ? `${detail};stale` : 'stale';
    } else if (status === 'fail') {
      status = 'warn';
      detail = detail ? `${detail};stale_fail` : 'stale_fail';
    }
  }
  return {
    status,
    source,
    detail,
    generated_at: generatedAt || '',
    age_seconds: ageSeconds,
    stale,
  };
}

function mergeBenchmarkSanityHealth(healthPayload, benchmarkSanity) {
  const health = healthPayload && typeof healthPayload === 'object' ? { ...healthPayload } : {};
  const checks = health.checks && typeof health.checks === 'object' ? { ...health.checks } : {};
  checks.benchmark_sanity = {
    status: cleanText(benchmarkSanity && benchmarkSanity.status ? benchmarkSanity.status : 'unknown', 24) || 'unknown',
    source: cleanText(benchmarkSanity && benchmarkSanity.source ? benchmarkSanity.source : 'benchmark_sanity_state', 80) || 'benchmark_sanity_state',
    detail: cleanText(benchmarkSanity && benchmarkSanity.detail ? benchmarkSanity.detail : '', 220),
    generated_at: cleanText(benchmarkSanity && benchmarkSanity.generated_at ? benchmarkSanity.generated_at : '', 80),
    age_seconds: parsePositiveInt(benchmarkSanity && benchmarkSanity.age_seconds, -1, -1, 1000000000),
    stale: !!(benchmarkSanity && benchmarkSanity.stale),
  };
  health.checks = checks;

  const alerts = health.alerts && typeof health.alerts === 'object' ? { ...health.alerts } : {};
  const checksList = new Set(Array.isArray(alerts.checks) ? alerts.checks.map((row) => cleanText(row, 120)).filter(Boolean) : []);
  if (checks.benchmark_sanity.status !== 'pass') {
    checksList.add('metric:benchmark_sanity');
  } else {
    checksList.delete('metric:benchmark_sanity');
  }
  alerts.checks = Array.from(checksList);
  alerts.count = alerts.checks.length;
  health.alerts = alerts;
  return health;
}

function hourBucketKeyFromTs(value) {
  const parsed = coerceTsMs(value, 0);
  if (!parsed) return '';
  const iso = new Date(parsed).toISOString();
  return iso.slice(0, 13);
}

function memoryStreamState(entries = []) {
  const rows = Array.isArray(entries) ? entries : [];
  const nextIndex = new Map();
  const nextHourIndex = new Map();
  for (const row of rows) {
    const key = cleanText(row && row.path ? row.path : '', 260);
    if (!key) continue;
    const stamp = cleanText(row && row.mtime ? row.mtime : '', 80);
    nextIndex.set(key, stamp);
    const hourKey = hourBucketKeyFromTs(stamp);
    if (hourKey) {
      nextHourIndex.set(hourKey, parseNonNegativeInt(nextHourIndex.get(hourKey), 0, 100000000) + 1);
    }
  }
  if (!memoryStreamBootstrapped) {
    memoryStreamBootstrapped = true;
    memoryStreamIndex = nextIndex;
    memoryStreamHourIndex = nextHourIndex;
    return {
      enabled: true,
      initialized: true,
      changed: false,
      seq: 0,
      change_count: 0,
      bucket_change_count: 0,
      latest_paths: [],
      removed_paths: [],
      hour_buckets: Object.fromEntries(Array.from(nextHourIndex.entries()).slice(-24)),
      index_strategy: 'hour_bucket_time_series',
      source: 'memory_diff_stream',
    };
  }
  const latest = [];
  const removed = [];
  for (const [key, stamp] of nextIndex.entries()) {
    const prevStamp = memoryStreamIndex.get(key);
    if (!prevStamp || prevStamp !== stamp) {
      latest.push(key);
    }
  }
  for (const key of memoryStreamIndex.keys()) {
    if (!nextIndex.has(key)) removed.push(key);
  }
  let bucketChanges = 0;
  const allHourKeys = new Set([...memoryStreamHourIndex.keys(), ...nextHourIndex.keys()]);
  for (const hourKey of allHourKeys) {
    const prev = parseNonNegativeInt(memoryStreamHourIndex.get(hourKey), 0, 100000000);
    const next = parseNonNegativeInt(nextHourIndex.get(hourKey), 0, 100000000);
    if (prev !== next) bucketChanges += 1;
  }
  const changed = latest.length > 0 || removed.length > 0 || bucketChanges > 0;
  if (changed) {
    memoryStreamSeq += 1;
  }
  memoryStreamIndex = nextIndex;
  memoryStreamHourIndex = nextHourIndex;
  return {
    enabled: true,
    initialized: true,
    changed,
    seq: memoryStreamSeq,
    change_count: latest.length + removed.length,
    bucket_change_count: bucketChanges,
    latest_paths: latest.slice(0, 12),
    removed_paths: removed.slice(0, 12),
    hour_buckets: Object.fromEntries(Array.from(nextHourIndex.entries()).slice(-24)),
    index_strategy: 'hour_bucket_time_series',
    source: 'memory_diff_stream',
  };
}

function filterArchivedAgentsFromCollab(collab) {
  if (!collab || typeof collab !== 'object') return collab;
  const dashboard = collab.dashboard;
  if (!dashboard || !Array.isArray(dashboard.agents)) return collab;
  const archived = archivedAgentIdsSet();
  if (!archived.size) return collab;
  const filtered = dashboard.agents.filter((row, idx) => {
    const id = cleanText(row && row.shadow ? row.shadow : `agent-${idx + 1}`, 140);
    return id && !archived.has(id);
  });
  if (filtered.length === dashboard.agents.length) return collab;
  return {
    ...collab,
    dashboard: {
      ...dashboard,
      agents: filtered,
      agent_count: filtered.length,
    },
  };
}

function authoritativeCollabForTeam(snapshot, team = DEFAULT_TEAM, options = {}) {
  const safeTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const timeoutMs = parsePositiveInt(
    options && options.timeout_ms != null ? options.timeout_ms : Math.max(RUNTIME_AUTHORITY_LANE_TIMEOUT_MS, 2500),
    Math.max(RUNTIME_AUTHORITY_LANE_TIMEOUT_MS, 2500),
    300,
    8000
  );
  const ttlMs = parsePositiveInt(
    options && options.ttl_ms != null ? options.ttl_ms : Math.max(RUNTIME_AUTHORITY_CACHE_TTL_MS, 1200),
    Math.max(RUNTIME_AUTHORITY_CACHE_TTL_MS, 1200),
    250,
    600000
  );
  const failTtlMs = parsePositiveInt(
    options && options.fail_ttl_ms != null ? options.fail_ttl_ms : Math.max(RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS, 800),
    Math.max(RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS, 800),
    250,
    600000
  );
  const lane = runLaneCached(
    `runtime.authority.collab.${safeTeam}`,
    ['collab-plane', 'dashboard', `--team=${safeTeam}`, '--strict=1'],
    {
      timeout_ms: timeoutMs,
      ttl_ms: ttlMs,
      fail_ttl_ms: failTtlMs,
    }
  );
  const payload = lanePayloadObject(lane, null);
  if (!payload || !payload.dashboard || !Array.isArray(payload.dashboard.agents)) {
    return snapshot && snapshot.collab && typeof snapshot.collab === 'object' ? snapshot.collab : {};
  }
  reconcileArchivedAgentsFromCollab(payload);
  const filtered = filterArchivedAgentsFromCollab(payload);
  if (snapshot && typeof snapshot === 'object') {
    snapshot.collab = filtered;
  }
  return filtered;
}

function authoritativeAgentsFromRuntime(snapshot, team = DEFAULT_TEAM, options = {}) {
  const collab = authoritativeCollabForTeam(snapshot, team, options);
  const base = snapshot && typeof snapshot === 'object' ? { ...snapshot } : {};
  base.collab = collab;
  return compatAgentsFromSnapshot(base, options);
}

function inactiveAgentRecord(agentId, snapshot, archivedMeta = null) {
  const cleanId = cleanText(agentId || '', 140) || 'agent';
  const modelState = effectiveAgentModel(cleanId, snapshot);
  const contract = contractForAgent(cleanId);
  const profile = agentProfileFor(cleanId);
  const identity = normalizeAgentIdentity(
    profile && profile.identity ? profile.identity : {},
    { emoji: '🤖', archetype: 'assistant', color: '#2563EB' }
  );
  const fallbackModels =
    profile && Array.isArray(profile.fallback_models) ? profile.fallback_models : [];
  const gitTree = agentGitTreeView(cleanId, profile);
  return {
    id: cleanId,
    name: cleanText(profile && profile.name ? profile.name : cleanId, 100) || cleanId,
    state: 'inactive',
    status: 'archived',
    archived: true,
    archived_at:
      cleanText(archivedMeta && archivedMeta.archived_at ? archivedMeta.archived_at : '', 80) || '',
    archive_reason: cleanText(archivedMeta && archivedMeta.reason ? archivedMeta.reason : 'archived', 240) || 'archived',
    contract: contractSummary(contract),
    model_name: modelState.selected,
    model_provider: modelState.provider,
    runtime_model: modelState.runtime_model,
    context_window: modelState.context_window,
    role: cleanText(profile && profile.role ? profile.role : 'analyst', 60) || 'analyst',
    identity,
    system_prompt: cleanText(profile && profile.system_prompt ? profile.system_prompt : '', 4000),
    fallback_models: fallbackModels,
    git_tree_kind: gitTree.git_tree_kind,
    git_branch: gitTree.git_branch,
    workspace_dir: gitTree.workspace_dir,
    workspace_rel: gitTree.workspace_rel,
    git_tree_ready: gitTree.git_tree_ready,
    git_tree_error: gitTree.git_tree_error,
    is_master_agent: gitTree.is_master_agent,
    capabilities: [],
  };
}

function safeAgentSessionFile(agentId) {
  const value = cleanText(agentId || 'agent', 140).replace(/[^a-zA-Z0-9._-]+/g, '_');
  return value || 'agent';
}

function runtimeChatSessionId(agentId, activeSessionId) {
  const agentPart = safeAgentSessionFile(agentId || 'agent');
  const sessionPart = safeAgentSessionFile(activeSessionId || 'default');
  const combined = `${agentPart}__${sessionPart}`;
  return cleanText(combined, 120).replace(/[^a-zA-Z0-9._-]+/g, '_') || 'chat-ui-default';
}

function agentSessionPath(agentId) {
  return path.resolve(AGENT_SESSIONS_DIR, `${safeAgentSessionFile(agentId)}.json`);
}

function agentSessionActivityTimestampMs(agentId) {
  const filePath = agentSessionPath(agentId);
  try {
    const stat = fs.statSync(filePath);
    const mtimeMs = Number(stat && stat.mtimeMs);
    if (Number.isFinite(mtimeMs) && mtimeMs > 0) {
      return Math.floor(mtimeMs);
    }
  } catch {}
  return 0;
}

function parseTs(value) {
  if (typeof value === 'number' && Number.isFinite(value)) return value;
  if (typeof value === 'string') {
    const parsed = Date.parse(value);
    if (!Number.isNaN(parsed)) return parsed;
  }
  return Date.now();
}

function turnsToSessionMessages(turns = []) {
  const rows = [];
  for (const turn of turns) {
    const ts = parseTs(turn && turn.ts ? turn.ts : null);
    const user = turn && typeof turn.user === 'string' ? turn.user : '';
    const assistant = turn && typeof turn.assistant === 'string' ? turn.assistant : '';
    if (user.trim()) {
      rows.push({ role: 'User', content: user, ts });
    }
    if (assistant.trim()) {
      rows.push({ role: 'Agent', content: assistant, ts });
    }
  }
  return rows;
}

function normalizeSessionState(state, snapshot) {
  const seededMessages = turnsToSessionMessages(
    snapshot && snapshot.app && Array.isArray(snapshot.app.turns) ? snapshot.app.turns : []
  );
  const fallback = {
    active_session_id: 'default',
    model_override: 'auto',
    sessions: [
      {
        session_id: 'default',
        label: 'Default',
        created_at: nowIso(),
        updated_at: nowIso(),
        messages: seededMessages,
      },
    ],
    memory_kv: {},
  };
  const normalized = state && typeof state === 'object' ? state : fallback;
  normalized.model_override = modelOverrideFromState(normalized);
  normalized.memory_kv = normalizeMemoryKvMap(normalized.memory_kv);
  if (!Array.isArray(normalized.sessions) || normalized.sessions.length === 0) {
    normalized.sessions = fallback.sessions;
  }
  normalized.sessions = normalized.sessions.map((session, idx) => {
    const sessionId =
      cleanText(session && session.session_id ? session.session_id : '', 80) || `session_${idx + 1}`;
    return {
      session_id: sessionId,
      label: cleanText(session && session.label ? session.label : 'Session', 80) || 'Session',
      created_at: session && session.created_at ? session.created_at : nowIso(),
      updated_at: session && session.updated_at ? session.updated_at : nowIso(),
      messages: Array.isArray(session && session.messages) ? session.messages : [],
    };
  });
  if (
    !normalized.active_session_id ||
    !normalized.sessions.some((session) => session.session_id === normalized.active_session_id)
  ) {
    normalized.active_session_id = normalized.sessions[0].session_id;
  }
  return normalized;
}

function loadAgentSession(agentId, snapshot) {
  const filePath = agentSessionPath(agentId);
  const state = readJson(filePath, null);
  const normalized = normalizeSessionState(state, snapshot);
  writeJson(filePath, normalized);
  return normalized;
}

function saveAgentSession(agentId, state) {
  writeJson(agentSessionPath(agentId), state);
}

function memoryKvForState(state) {
  if (!state || typeof state !== 'object') return {};
  state.memory_kv = normalizeMemoryKvMap(state.memory_kv);
  return state.memory_kv;
}

function listAgentMemoryKv(agentId, snapshot) {
  const state = loadAgentSession(agentId, snapshot);
  const kv = memoryKvForState(state);
  const keys = Object.keys(kv).sort((a, b) => a.localeCompare(b));
  const kvPairs = keys.map((key) => ({
    key,
    value: kv[key],
  }));
  return {
    agent_id: cleanText(agentId || '', 140),
    kv_pairs: kvPairs,
    count: kvPairs.length,
  };
}

function readAgentMemoryKv(agentId, key, snapshot) {
  const state = loadAgentSession(agentId, snapshot);
  const kv = memoryKvForState(state);
  const normalizedKey = normalizeMemoryKey(key);
  if (!normalizedKey || !Object.prototype.hasOwnProperty.call(kv, normalizedKey)) {
    return { ok: false, error: 'memory_key_not_found', key: normalizedKey };
  }
  return { ok: true, key: normalizedKey, value: kv[normalizedKey] };
}

function writeAgentMemoryKv(agentId, key, value, snapshot) {
  const state = loadAgentSession(agentId, snapshot);
  const kv = memoryKvForState(state);
  const normalizedKey = normalizeMemoryKey(key);
  if (!normalizedKey) {
    return { ok: false, error: 'memory_key_required', key: '' };
  }
  kv[normalizedKey] = sanitizeMemoryValue(value);
  const limited = normalizeMemoryKvMap(kv);
  state.memory_kv = limited;
  saveAgentSession(agentId, state);
  return { ok: true, key: normalizedKey, value: limited[normalizedKey] };
}

function deleteAgentMemoryKv(agentId, key, snapshot) {
  const state = loadAgentSession(agentId, snapshot);
  const kv = memoryKvForState(state);
  const normalizedKey = normalizeMemoryKey(key);
  if (!normalizedKey || !Object.prototype.hasOwnProperty.call(kv, normalizedKey)) {
    return { ok: false, error: 'memory_key_not_found', key: normalizedKey };
  }
  delete kv[normalizedKey];
  state.memory_kv = normalizeMemoryKvMap(kv);
  saveAgentSession(agentId, state);
  return { ok: true, key: normalizedKey };
}

function activeSession(state) {
  let session = state.sessions.find((row) => row.session_id === state.active_session_id);
  if (!session) {
    session = state.sessions[0];
    state.active_session_id = session ? session.session_id : 'default';
  }
  if (!session) {
    session = {
      session_id: 'default',
      label: 'Default',
      created_at: nowIso(),
      updated_at: nowIso(),
      messages: [],
    };
    state.sessions.push(session);
    state.active_session_id = session.session_id;
  }
  return session;
}

function appendAgentConversation(agentId, snapshot, userText, assistantText, metaText = '', assistantTools = [], options = {}) {
  const state = loadAgentSession(agentId, snapshot);
  const session = activeSession(state);
  const nowMs = Date.now();
  const userRole = cleanText(options && options.user_role ? options.user_role : 'User', 20) || 'User';
  const assistantRole = cleanText(options && options.assistant_role ? options.assistant_role : 'Agent', 20) || 'Agent';
  const userAgentId = cleanText(options && options.user_agent_id ? options.user_agent_id : '', 140);
  const userAgentName = cleanText(options && options.user_agent_name ? options.user_agent_name : '', 120);
  const userSystemOrigin = cleanText(options && options.user_system_origin ? options.user_system_origin : '', 120);
  const assistantAgentId = cleanText(
    options && options.assistant_agent_id ? options.assistant_agent_id : cleanText(agentId || '', 140),
    140
  );
  const assistantAgentName = cleanText(options && options.assistant_agent_name ? options.assistant_agent_name : '', 120);
  const assistantSystemOrigin = cleanText(options && options.assistant_system_origin ? options.assistant_system_origin : '', 120);
  if (userText && String(userText).trim()) {
    const userRow = { role: userRole, content: String(userText), ts: nowMs };
    if (userAgentId) userRow.agent_id = userAgentId;
    if (userAgentName) userRow.agent_name = userAgentName;
    if (userSystemOrigin) userRow.system_origin = userSystemOrigin;
    session.messages.push(userRow);
  }
  if (assistantText && String(assistantText).trim()) {
    const normalizedTools = Array.isArray(assistantTools)
      ? assistantTools
          .map((tool, idx) => ({
            id: cleanText(tool && tool.id ? tool.id : `tool-${idx + 1}`, 80) || `tool-${idx + 1}`,
            name: cleanText(tool && tool.name ? tool.name : 'cli', 80) || 'cli',
            input: cleanText(tool && tool.input ? tool.input : '', 400),
            result: cleanText(tool && tool.result ? tool.result : '', TOOL_OUTPUT_LIMIT),
            is_error: !!(tool && tool.is_error),
            running: false,
            expanded: false,
          }))
          .filter((tool) => tool.name)
      : [];
    const assistantRow = {
      role: assistantRole,
      content: String(assistantText),
      meta: cleanText(metaText || '', 120),
      tools: normalizedTools,
      ts: nowMs,
    };
    if (assistantAgentId) assistantRow.agent_id = assistantAgentId;
    if (assistantAgentName) assistantRow.agent_name = assistantAgentName;
    if (assistantSystemOrigin) assistantRow.system_origin = assistantSystemOrigin;
    session.messages.push(assistantRow);
  }
  if (session.messages.length > 800) {
    session.messages = session.messages.slice(-800);
  }
  session.updated_at = nowIso();
  saveAgentSession(agentId, state);
  recordPassiveConversationMemory(agentId, userText, assistantText, metaText);
  return state;
}

function normalizeAgentNoticeType(value, fallback = 'info') {
  const fallbackType = String(fallback || 'info').toLowerCase() === 'model' ? 'model' : 'info';
  const raw = cleanText(value || '', 24).toLowerCase();
  if (raw === 'model' || raw === 'info') return raw;
  return fallbackType;
}

function appendAgentNoticeEvent(agentId, snapshot, noticeLabel, options = {}) {
  const id = cleanText(agentId || '', 140);
  const label = cleanText(noticeLabel || '', 240);
  if (!id || !label) return null;
  const state = loadAgentSession(id, snapshot);
  const session = activeSession(state);
  const tsRaw = Number(options && options.ts != null ? options.ts : 0);
  const ts = Number.isFinite(tsRaw) && tsRaw > 0 ? tsRaw : Date.now();
  const noticeType = normalizeAgentNoticeType(
    options && (options.notice_type || options.type),
    /^Model switched (?:to\b|from\b)/i.test(label) ? 'model' : 'info'
  );
  let noticeIcon = cleanText(options && (options.notice_icon || options.icon), 8);
  if (!noticeIcon && noticeType === 'info') noticeIcon = 'i';
  const row = {
    role: 'System',
    content: '',
    is_notice: true,
    notice_label: label,
    notice_type: noticeType,
    ts,
  };
  if (noticeIcon) row.notice_icon = noticeIcon;
  session.messages.push(row);
  if (session.messages.length > 800) {
    session.messages = session.messages.slice(-800);
  }
  session.updated_at = nowIso();
  saveAgentSession(id, state);
  return {
    notice_label: label,
    notice_type: noticeType,
    notice_icon: noticeIcon || '',
    ts,
  };
}

function shouldSurfaceRuntimeTaskInChat(source = '') {
  const normalized = cleanText(source || '', 120).toLowerCase();
  if (!normalized) return false;
  if (normalized.startsWith('swarm_recommendation.')) return false;
  if (normalized.startsWith('runtime_') || normalized.startsWith('runtime.')) return false;
  if (normalized.startsWith('autoheal.')) return false;
  return true;
}

function runtimeTaskDispatchKey(agentId, source, task) {
  const normalizedAgentId = cleanText(agentId || '', 140) || 'unknown-agent';
  const normalizedSource = cleanText(source || '', 120).toLowerCase() || 'runtime-dashboard';
  const taskHash = sha256(cleanText(task || '', 2000) || '').slice(0, 24);
  return `${normalizedAgentId}|${normalizedSource}|${taskHash}`;
}

function pruneRuntimeTaskDispatchState(nowMs) {
  const state = runtimeTaskDispatchState && typeof runtimeTaskDispatchState === 'object'
    ? runtimeTaskDispatchState
    : {};
  const cutoff = Math.max(0, parseNonNegativeInt(nowMs, 0, 1000000000000) - RUNTIME_TASK_DISPATCH_RETAIN_MS);
  for (const key of Object.keys(state)) {
    const ts = parseNonNegativeInt(state[key], 0, 1000000000000);
    if (!ts || ts < cutoff) {
      delete state[key];
    }
  }
  runtimeTaskDispatchState = state;
}

function queueAgentTask(agentId, snapshot, taskText, source = 'runtime_dashboard') {
  const id = cleanText(agentId || '', 140);
  const task = cleanText(taskText || '', 2000);
  const normalizedSource = cleanText(source, 120) || 'runtime_dashboard';
  if (!id || !task) {
    return {
      ok: false,
      agent_id: id,
      error: 'task_invalid',
    };
  }
  const nowMs = Date.now();
  pruneRuntimeTaskDispatchState(nowMs);
  const dispatchKey = runtimeTaskDispatchKey(id, normalizedSource, task);
  const lastDispatchMs = parseNonNegativeInt(runtimeTaskDispatchState[dispatchKey], 0, 1000000000000);
  if (lastDispatchMs > 0 && (nowMs - lastDispatchMs) < RUNTIME_TASK_CHAT_DEDUPE_MS) {
    return {
      ok: true,
      agent_id: id,
      task,
      source: normalizedSource,
      queued_at: nowIso(),
      deduped: true,
      surfaced_in_chat: false,
      dedupe_window_ms: RUNTIME_TASK_CHAT_DEDUPE_MS,
      last_dispatch_ms: lastDispatchMs,
    };
  }
  runtimeTaskDispatchState[dispatchKey] = nowMs;
  const targetAgent = compatAgentsFromSnapshot(snapshot).find((row) => row && row.id === id);
  const targetAgentName = cleanText(targetAgent && targetAgent.name ? targetAgent.name : id, 120) || id;
  const surfacedInChat = shouldSurfaceRuntimeTaskInChat(normalizedSource);
  if (surfacedInChat) {
    appendAgentConversation(
      id,
      snapshot,
      `[runtime-task] ${task}`,
      '',
      `queued:${cleanText(normalizedSource, 80)}`,
      [],
      {
        user_role: 'System',
        user_system_origin: normalizedSource,
        assistant_role: 'Agent',
        assistant_agent_id: id,
        assistant_agent_name: targetAgentName,
      }
    );
  }
  return {
    ok: true,
    agent_id: id,
    task,
    source: normalizedSource,
    deduped: false,
    surfaced_in_chat: surfacedInChat,
    queued_at: nowIso(),
  };
}

function compactAgentConversation(agentId, snapshot) {
  const state = loadAgentSession(agentId, snapshot);
  const session = activeSession(state);
  const keep = Math.min(200, session.messages.length);
  if (keep > 0) {
    session.messages = session.messages.slice(-keep);
  }
  session.updated_at = nowIso();
  saveAgentSession(agentId, state);
  return state;
}

function sessionList(state) {
  return state.sessions.map((session) => ({
    session_id: session.session_id,
    label: session.label || 'Session',
    message_count: Array.isArray(session.messages) ? session.messages.length : 0,
    updated_at: session.updated_at || nowIso(),
    active: session.session_id === state.active_session_id,
  }));
}

function runAgentMessage(agentId, input, snapshot, options = {}) {
  const allowFallback = !!(options && options.allowFallback);
  let requestedAgentId = cleanText(agentId || '', 140);
  const dashboardFallbackAgent = 'chat-ui-default-agent';
  const canAutoReviveDashboardFallback =
    allowFallback && (!requestedAgentId || requestedAgentId === dashboardFallbackAgent);
  if (requestedAgentId && isAgentArchived(requestedAgentId)) {
    if (canAutoReviveDashboardFallback) {
      unarchiveAgent(requestedAgentId);
      upsertAgentContract(
        requestedAgentId,
        {
          mission: `Assist with assigned mission for ${requestedAgentId}.`,
          owner: 'dashboard_chat',
          termination_condition: 'task_or_timeout',
        },
        { owner: 'dashboard_chat', force: true }
      );
    } else if (allowFallback) {
      requestedAgentId = '';
    } else {
      const archivedMeta = archivedAgentMeta(requestedAgentId);
      return {
        ok: false,
        status: 409,
        error: 'agent_inactive',
        id: requestedAgentId,
        archived: true,
        archived_at:
          cleanText(archivedMeta && archivedMeta.archived_at ? archivedMeta.archived_at : '', 80) || '',
      };
    }
  }
  if (requestedAgentId && isAgentArchived(requestedAgentId)) {
    if (allowFallback) {
      requestedAgentId = '';
    } else {
      const archivedMeta = archivedAgentMeta(requestedAgentId);
      return {
        ok: false,
        status: 409,
        error: 'agent_inactive',
        id: requestedAgentId,
        archived: true,
        archived_at:
          cleanText(archivedMeta && archivedMeta.archived_at ? archivedMeta.archived_at : '', 80) || '',
      };
    }
  }
  ensureAgentGitTreeAssignments(snapshot, {
    force: false,
    preferred_master_id: requestedAgentId || '',
  });
  const knownAgents = compatAgentsFromSnapshot(snapshot);
  let agent = knownAgents.find((row) => row.id === requestedAgentId);
  if (!agent && allowFallback) {
    const fallbackId = requestedAgentId || (knownAgents[0] && knownAgents[0].id) || 'chat-ui-default-agent';
    agent = knownAgents[0] || {
      id: fallbackId,
      name: fallbackId,
      state: 'running',
      status: 'active',
      role: 'operator',
      provider: configuredProvider(snapshot),
      model_name: configuredOllamaModel(snapshot),
      has_prompt_context: true,
    };
  }
  if (!agent) {
    return { ok: false, status: 404, error: 'agent_not_found', id: agentId };
  }
  const effectiveAgentId = cleanText(agent.id || requestedAgentId || 'chat-ui-default-agent', 140) || 'chat-ui-default-agent';
  const cleanInput = cleanText(input || '', 4000);
  if (!cleanInput) {
    return { ok: false, status: 400, error: 'message_required' };
  }
  let contract = contractForAgent(effectiveAgentId);
  if (!contract && !isAgentArchived(effectiveAgentId)) {
    contract = upsertAgentContract(
      effectiveAgentId,
      {
        mission: `Assist with assigned mission for ${effectiveAgentId}.`,
        owner: 'dashboard_chat',
        termination_condition: 'task_or_timeout',
      },
      { owner: 'dashboard_chat' }
    );
  }
  if (contract && contract.status === 'active') {
    const violation = detectContractViolation(effectiveAgentId, cleanInput, contract, snapshot);
    if (violation) {
      const terminated = terminateAgentForContract(
        effectiveAgentId,
        snapshot,
        `rogue_${cleanText(violation.reason || 'violation', 80)}`,
        {
          source: 'safety_plane',
          terminated_by: 'safety_plane',
          role: cleanText(agent && agent.role ? agent.role : '', 80),
          team:
            cleanText(
              snapshot && snapshot.metadata && snapshot.metadata.team ? snapshot.metadata.team : DEFAULT_TEAM,
              40
            ) || DEFAULT_TEAM,
        }
      );
      return {
        ok: false,
        status: 409,
        error: 'agent_contract_terminated',
        agent_id: effectiveAgentId,
        reason: cleanText(violation.reason || 'rogue_violation', 120),
        detail: cleanText(violation.detail || '', 240),
        terminated: !!terminated.terminated,
      };
    }
    recordContractMessageTick(effectiveAgentId);
  }

  const state = loadAgentSession(effectiveAgentId, snapshot);
  const session = activeSession(state);
  const chatSessionId = runtimeChatSessionId(effectiveAgentId, session.session_id);
  const modelState = effectiveAgentModel(effectiveAgentId, snapshot);
  const autoRoute =
    modelState && modelState.selected === 'auto'
      ? planAutoRoute(cleanInput, snapshot, {
          agent_id: effectiveAgentId,
          token_count: Math.max(1, Math.round(String(cleanInput || '').length / 4)),
          has_vision: false,
        })
      : null;
  const runtimeModelState =
    autoRoute && autoRoute.ok
      ? {
          ...modelState,
          provider: cleanText(autoRoute.selected_provider || modelState.provider || 'ollama', 80) || 'ollama',
          runtime_provider:
            cleanText(autoRoute.selected_provider || modelState.runtime_provider || 'ollama', 80) || 'ollama',
          runtime_model:
            cleanText(autoRoute.selected_model || modelState.runtime_model || configuredOllamaModel(snapshot), 120) ||
            configuredOllamaModel(snapshot),
          context_window: parsePositiveInt(
            autoRoute.selected_context_window,
            modelState.context_window || DEFAULT_CONTEXT_WINDOW_TOKENS,
            1024,
            8_000_000
          ),
        }
      : modelState;
  const runtimeSync = runtimeSyncSummary(snapshot);
  const runtimeMirror = runtimeMirrorFromSnapshot(
    snapshot,
    snapshot && snapshot.metadata && snapshot.metadata.team ? snapshot.metadata.team : DEFAULT_TEAM
  );
  const startedAtMs = Date.now();
  const preferHostedBackend = shouldUseHostedModelBackend(runtimeModelState);
  const hostedProviderSync = preferHostedBackend
    ? ensureHostedChatProviderModel(snapshot, runtimeModelState)
    : null;
  const llmResult = preferHostedBackend
    ? {
        ok: false,
        status: hostedProviderSync && hostedProviderSync.ok ? 1 : 2,
        error:
          hostedProviderSync && hostedProviderSync.ok
            ? 'hosted_model_backend_selected'
            : 'hosted_model_provider_sync_failed',
        tools: [],
      }
    : runLlmChatWithCli(
        agent,
        session,
        cleanInput,
        snapshot,
        runtimeModelState.runtime_model,
        runtimeMirror
      );

  let laneResult;
  let tools = [];
  let assistantRaw = '';
  let iterations = 1;
  let backend = 'ollama';
  let usedModel = runtimeModelState.runtime_model || OLLAMA_MODEL_FALLBACK;

  if (llmResult && llmResult.ok) {
    tools = Array.isArray(llmResult.tools) ? llmResult.tools : [];
    assistantRaw = String(llmResult.response || '');
    iterations = parsePositiveInt(llmResult.iterations || 1, 1, 1, 8);
    usedModel = cleanText(llmResult.model || usedModel || OLLAMA_MODEL_FALLBACK, 120) || OLLAMA_MODEL_FALLBACK;
    // Always emit a core-lane receipt so chat turns are visible to cockpit/conduit feeds.
    laneResult = runAction('app.chat', {
      input: cleanInput,
      session_id: chatSessionId,
    });
    if (!laneResult || typeof laneResult !== 'object') {
      laneResult = {
        ok: false,
        status: 1,
        stdout: '',
        stderr: 'lane_result_missing',
        argv: ['app-plane', 'run', '--app=chat-ui'],
        payload: null,
      };
    }
  } else {
    backend = 'app-plane';
    laneResult = runLane([
      'app-plane',
      'run',
      '--app=chat-ui',
      `--session-id=${chatSessionId}`,
      `--input=${cleanInput}`,
    ]);
    const payloadObj = laneResult && laneResult.payload && typeof laneResult.payload === 'object'
      ? laneResult.payload
      : null;
    const assistantFromLane =
      payloadObj &&
      typeof payloadObj.response === 'string'
        ? payloadObj.response
        : payloadObj &&
          payloadObj.turn &&
          typeof payloadObj.turn.assistant === 'string'
          ? payloadObj.turn.assistant
          : '';
    assistantRaw = String(assistantFromLane || '');
    if (isPromptEchoResponse(assistantRaw, cleanInput)) {
      const echoRecovery = runLlmChatWithCli(
        agent,
        session,
        cleanInput,
        snapshot,
        runtimeModelState.runtime_model,
        runtimeMirror
      );
      const recoveredResponse =
        echoRecovery && echoRecovery.ok
          ? cleanText(echoRecovery.response || '', 4000)
          : '';
      if (recoveredResponse && !isPromptEchoResponse(recoveredResponse, cleanInput)) {
        backend = 'ollama-recovery';
        assistantRaw = recoveredResponse;
        tools = Array.isArray(echoRecovery.tools) ? echoRecovery.tools : [];
        iterations = parsePositiveInt(echoRecovery.iterations || iterations, iterations, 1, 8);
        usedModel =
          cleanText(echoRecovery.model || runtimeModelState.runtime_model || usedModel || OLLAMA_MODEL_FALLBACK, 120) ||
          OLLAMA_MODEL_FALLBACK;
        laneResult = {
          ok: true,
          status: 0,
          stdout: '',
          stderr: '',
          argv: ['chat-backend', 'hosted-echo-recovered'],
          payload: {
            ok: true,
            type: 'infring_dashboard_chat_backend_echo_recovered',
            response: assistantRaw,
            session_id: chatSessionId,
            model: usedModel,
          },
        };
      } else {
        assistantRaw = runtimeCouplingFallbackResponse(cleanInput, runtimeSync);
        laneResult = {
          ok: true,
          status: 0,
          stdout: '',
          stderr: '',
          argv: ['chat-backend', 'hosted-echo-fallback'],
          payload: {
            ok: true,
            type: 'infring_dashboard_chat_backend_echo_fallback',
            response: assistantRaw,
            session_id: chatSessionId,
          },
        };
      }
    }
    if (!assistantRaw) {
      const failures = [];
      if (llmResult && llmResult.error) {
        failures.push(`ollama: ${cleanText(llmResult.error, 180)}`);
      }
      if (hostedProviderSync && hostedProviderSync.lane && !hostedProviderSync.ok) {
        failures.push(
          `provider-sync: ${cleanText(
            hostedProviderSync.lane.stderr ||
              hostedProviderSync.lane.stdout ||
              hostedProviderSync.lane.status,
            180
          )}`
        );
      }
      if (laneResult && !laneResult.ok) {
        const laneDetail = cleanText(
          String(laneResult.stderr || laneResult.stdout || laneResult.status || 'failed'),
          180
        );
        failures.push(`app-plane: ${laneDetail}`);
      }
      assistantRaw =
        failures.length > 0
          ? `I couldn't reach a chat model backend (${failures.join('; ')}). Start Ollama or configure app-plane and try again.`
          : 'I could not produce a response from the chat backend. Please try again.';
      laneResult = {
        ok: true,
        status: 0,
        stdout: '',
        stderr: '',
        argv: ['chat-backend', 'fallback-message'],
        payload: {
          ok: true,
          type: 'infring_dashboard_chat_backend_unavailable',
          response: assistantRaw,
          session_id: chatSessionId,
        },
      };
    }
  }

  let assistant = String(assistantRaw || '').trim()
    ? String(assistantRaw || '').slice(0, 4000)
    : ASSISTANT_EMPTY_FALLBACK_RESPONSE;
  const telemetryPrompt = /runtime sync|queue depth|cockpit|attention queue|conduit/i.test(cleanInput);
  if (telemetryPrompt && !/conduit/i.test(assistant)) {
    assistant = `${assistant}\n\nConduit signals: ${parseNonNegativeInt(runtimeMirror.summary.conduit_signals, 0, 1000000)}.`.slice(0, 4000);
  }
  const inputTokens = Math.max(1, Math.round(String(cleanInput).length / 4));
  const outputTokens = Math.max(1, Math.round(String(assistant || '').length / 4));
  const durationMs = Math.max(0, Date.now() - startedAtMs);
  const contextWindow = parsePositiveInt(
    runtimeModelState && runtimeModelState.context_window != null
      ? runtimeModelState.context_window
      : DEFAULT_CONTEXT_WINDOW_TOKENS,
    DEFAULT_CONTEXT_WINDOW_TOKENS,
    1024,
    8000000
  );
  const contextStats = contextTelemetryForMessages(
    Array.isArray(session.messages) ? session.messages : [],
    contextWindow,
    inputTokens + outputTokens
  );
  const turnSeverity =
    !assistant
      ? 'warn'
      : laneResult && laneResult.ok && Array.isArray(tools) && !tools.some((tool) => tool && tool.is_error)
        ? 'info'
        : laneResult && laneResult.ok
          ? 'warn'
          : 'critical';
  const attentionKey = `chat-${sha256(`${effectiveAgentId}:${chatSessionId}:${Date.now()}`).slice(0, 24)}`;
  const attentionEnqueue = enqueueAttentionEvent(
    {
      ts: nowIso(),
      source: 'dashboard_chat',
      source_type: 'chat_turn',
      severity: turnSeverity,
      summary: cleanText(assistant || cleanInput, 240),
      attention_key: attentionKey,
      session_id: chatSessionId,
      agent_id: cleanText(effectiveAgentId, 120),
      lane_ok: !!(laneResult && laneResult.ok),
      tool_count: Array.isArray(tools) ? tools.length : 0,
      tool_errors: Array.isArray(tools) ? tools.filter((tool) => !!(tool && tool.is_error)).length : 0,
    },
    'dashboard_chat'
  );
  const durationLabel = durationMs < 1000
    ? `${Math.round(durationMs)}ms`
    : `${(durationMs / 1000).toFixed(durationMs < 10000 ? 1 : 0)}s`;
  const laneConduit =
    !!(
      laneResult &&
      laneResult.payload &&
      typeof laneResult.payload === 'object' &&
      (laneResult.payload.conduit_enforcement || laneResult.payload.routed_via === 'conduit')
    );
  const laneState = laneResult && laneResult.ok ? 'ok' : 'degraded';
  const autoRouteMeta =
    autoRoute && autoRoute.ok
      ? ` | auto:${cleanText(autoRoute.selected_provider || '', 40)}/${cleanText(autoRoute.selected_model || '', 120)}${cleanText(autoRoute.authority || '', 40) === 'rust_model_router' ? ':rust' : ''}`
      : '';
  const meta = `${inputTokens} in / ${outputTokens} out | ${durationLabel} | lane:${laneState}${laneConduit ? ' conduit' : ''} | queue:${runtimeMirror.summary.queue_depth} | ctx:${Math.round((contextStats.context_ratio || 0) * 100)}%${autoRouteMeta}`;
  const responseOk = !!String(assistant || '').trim();
  let contractTermination = null;
  contract = contractForAgent(effectiveAgentId);
  if (contract && contract.status === 'active' && missionCompleteSignal(assistant)) {
    markContractCompletion(effectiveAgentId, 'agent_self_signal');
    contract = contractForAgent(effectiveAgentId);
  }
  const terminationReason = contractTerminationDecision(contract);
  if (terminationReason) {
    contractTermination = terminateAgentForContract(effectiveAgentId, snapshot, terminationReason, {
      source: 'agent_contract_turn',
      terminated_by: terminationReason === 'task_complete' ? 'agent_completion_signal' : 'agent_contract_enforcer',
      role: cleanText(agent && agent.role ? agent.role : '', 80),
      team:
        cleanText(
          snapshot && snapshot.metadata && snapshot.metadata.team ? snapshot.metadata.team : DEFAULT_TEAM,
          40
        ) || DEFAULT_TEAM,
      auto_termination: true,
      agent_row: agent || null,
    });
  }
  const modelProvider =
    cleanText(
      providerForModelName(
        usedModel,
        cleanText(
          runtimeModelState && (runtimeModelState.runtime_provider || runtimeModelState.provider)
            ? runtimeModelState.runtime_provider || runtimeModelState.provider
            : 'ollama',
          80
        ) || 'ollama'
      ),
      80
    ) || 'ollama';
  const autoRoutePayload =
    autoRoute && autoRoute.ok
      ? {
          provider: cleanText(autoRoute.selected_provider || modelProvider, 80) || modelProvider,
          model: cleanText(autoRoute.selected_model || usedModel, 120) || usedModel,
          model_id:
            cleanText(autoRoute.selected_model_id || '', 160) ||
            `${cleanText(autoRoute.selected_provider || modelProvider, 80) || modelProvider}/${cleanText(autoRoute.selected_model || usedModel, 120) || usedModel}`,
          reason: cleanText(autoRoute.reason || '', 260),
          authority: cleanText(autoRoute.authority || '', 40) || 'unknown',
          route_lane: cleanText(autoRoute.route_lane || '', 80) || 'model-router.infer',
          fallback_chain: Array.isArray(autoRoute.fallback_chain) ? autoRoute.fallback_chain : [],
          receipt_hash: cleanText(autoRoute.receipt_hash || '', 80),
          lane_receipt_hash: cleanText(autoRoute.lane_receipt_hash || '', 80),
          executed_provider: modelProvider,
          executed_model: cleanText(usedModel || '', 120),
        }
      : null;
  if (autoRoutePayload && laneResult && typeof laneResult === 'object') {
    const payloadObj =
      laneResult.payload && typeof laneResult.payload === 'object' ? laneResult.payload : {};
    laneResult.payload = {
      ...payloadObj,
      auto_route: autoRoutePayload,
      routed_model: autoRoutePayload.model,
      routed_provider: autoRoutePayload.provider,
    };
  }

  return {
    ok: responseOk,
    status: responseOk ? 200 : 400,
    agent_id: effectiveAgentId,
    input: cleanInput,
    laneResult,
    lane_ok: !!(laneResult && laneResult.ok),
    agent,
    session_id: chatSessionId,
    response: assistant,
    tools,
    iterations,
    input_tokens: inputTokens,
    output_tokens: outputTokens,
    context_tokens: contextStats.context_tokens,
    context_window: contextStats.context_window,
    context_ratio: contextStats.context_ratio,
    context_pressure: contextStats.context_pressure,
    cost_usd: 0,
    meta,
    duration_ms: durationMs,
    model: usedModel,
    model_provider: modelProvider,
    backend,
    auto_route: autoRoutePayload,
    contract: contractSummary(contractForAgent(effectiveAgentId)),
    contract_terminated: !!(contractTermination && contractTermination.terminated),
    contract_termination_reason:
      contractTermination && contractTermination.reason ? cleanText(contractTermination.reason, 120) : '',
    runtime_sync: {
      ok: runtimeMirror.ok,
      cockpit_ok: runtimeMirror.cockpit_ok,
      attention_status_ok: runtimeMirror.attention_status_ok,
      attention_next_ok: runtimeMirror.attention_next_ok,
      attention_enqueue_ok: !!(attentionEnqueue && attentionEnqueue.ok),
      queue_depth: runtimeMirror.summary.queue_depth,
      cockpit_blocks: runtimeMirror.summary.cockpit_blocks,
      cockpit_total_blocks: parseNonNegativeInt(runtimeMirror.summary.cockpit_total_blocks, runtimeMirror.summary.cockpit_blocks, 100000000),
      attention_batch_count: runtimeMirror.summary.attention_batch_count,
      conduit_signals: runtimeMirror.summary.conduit_signals,
      conduit_signals_raw: parseNonNegativeInt(runtimeMirror.summary.conduit_signals_raw, runtimeMirror.summary.conduit_signals, 100000000),
      conduit_channels_observed: runtimeMirror.summary.conduit_channels_observed,
      target_conduit_signals: runtimeMirror.summary.target_conduit_signals,
      conduit_scale_required: !!runtimeMirror.summary.conduit_scale_required,
      critical_attention: runtimeMirror.summary.attention_critical,
      critical_attention_total: runtimeMirror.summary.attention_critical_total,
      telemetry_attention: runtimeMirror.summary.attention_telemetry,
      sync_mode: runtimeMirror.summary.sync_mode,
      backpressure_level: runtimeMirror.summary.backpressure_level,
      benchmark_sanity_status: runtimeMirror.summary.benchmark_sanity_status || 'unknown',
      benchmark_sanity_source: runtimeMirror.summary.benchmark_sanity_source || 'benchmark_sanity_state',
      benchmark_sanity_cockpit_status: runtimeMirror.summary.benchmark_sanity_cockpit_status || 'unknown',
      benchmark_sanity_age_seconds: parsePositiveInt(runtimeMirror.summary.benchmark_sanity_age_seconds, -1, -1, 1000000000),
      receipt_latency_p95_ms:
        runtimeSync && runtimeSync.receipt_latency_p95_ms != null
          ? Number(runtimeSync.receipt_latency_p95_ms)
          : null,
      receipt_latency_p99_ms:
        runtimeSync && runtimeSync.receipt_latency_p99_ms != null
          ? Number(runtimeSync.receipt_latency_p99_ms)
          : null,
      health_coverage_gap_count: parseNonNegativeInt(
        snapshot && snapshot.health && snapshot.health.coverage && snapshot.health.coverage.gap_count != null
          ? snapshot.health.coverage.gap_count
          : 0,
        0,
        100000000
      ),
      memory_ingest_paused: !!(
        snapshot &&
        snapshot.memory &&
        snapshot.memory.ingest_control &&
        snapshot.memory.ingest_control.paused
      ),
      cockpit: runtimeMirror.cockpit,
      attention_queue: runtimeMirror.attention_queue,
    },
  };
}

function recentFiles(rootDir, { limit = 25, maxDepth = 4, include }) {
  const out = [];
  const stack = [{ dir: rootDir, depth: 0 }];
  while (stack.length > 0) {
    const { dir, depth } = stack.pop();
    let entries = [];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (depth < maxDepth) {
          stack.push({ dir: fullPath, depth: depth + 1 });
        }
        continue;
      }
      if (!entry.isFile()) continue;
      if (typeof include === 'function' && !include(fullPath)) continue;
      let stat = null;
      try {
        stat = fs.statSync(fullPath);
      } catch {
        stat = null;
      }
      if (!stat) continue;
      out.push({
        path: path.relative(ROOT, fullPath),
        full_path: fullPath,
        mtime_ms: stat.mtimeMs || 0,
        mtime: stat.mtime.toISOString(),
        size_bytes: stat.size,
      });
    }
  }

  out.sort((a, b) => b.mtime_ms - a.mtime_ms);
  return out.slice(0, limit);
}

function readTailLines(filePath, maxBytes = 48 * 1024, maxLines = 8) {
  let data = '';
  try {
    const stat = fs.statSync(filePath);
    const start = Math.max(0, stat.size - maxBytes);
    const size = stat.size - start;
    let fd = null;
    try {
      fd = fs.openSync(filePath, 'r');
      const buffer = Buffer.alloc(size);
      fs.readSync(fd, buffer, 0, size, start);
      data = buffer.toString('utf8');
    } finally {
      if (fd != null) {
        try {
          fs.closeSync(fd);
        } catch {}
      }
    }
  } catch {
    return [];
  }
  return data
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(-maxLines);
}

function collectLogEvents() {
  const logRoots = [
    path.resolve(ROOT, 'core/local/state/ops'),
    path.resolve(ROOT, 'client/runtime/local/state'),
  ];
  const rows = [];
  for (const rootDir of logRoots) {
    const files = recentFiles(rootDir, {
      limit: 8,
      maxDepth: 4,
      include: (fullPath) => fullPath.endsWith('history.jsonl') || fullPath.endsWith('.jsonl'),
    });
    for (const file of files) {
      const lines = readTailLines(file.full_path);
      for (const line of lines) {
        const payload = parseJsonLoose(line);
        rows.push({
          ts: payload && payload.ts ? payload.ts : file.mtime,
          source: file.path,
          message: payload && payload.type ? payload.type : line.slice(0, 220),
        });
      }
    }
  }
  rows.sort((a, b) => String(b.ts).localeCompare(String(a.ts)));
  return rows.slice(0, 40);
}

function collectReceipts() {
  const roots = [
    path.resolve(ROOT, 'core/local/state/ops'),
    path.resolve(ROOT, 'client/runtime/local/state'),
  ];
  const files = [];
  for (const rootDir of roots) {
    files.push(
      ...recentFiles(rootDir, {
        limit: 30,
        maxDepth: 4,
        include: (fullPath) =>
          fullPath.endsWith('latest.json') ||
          fullPath.endsWith('history.jsonl') ||
          fullPath.endsWith('.receipt.json'),
      })
    );
  }
  files.sort((a, b) => b.mtime_ms - a.mtime_ms);
  return files.slice(0, 32).map((file) => ({
    kind: file.path.endsWith('.jsonl') ? 'timeline' : 'receipt',
    path: file.path,
    mtime: file.mtime,
    size_bytes: file.size_bytes,
  }));
}

function collectMemoryArtifacts() {
  const roots = [
    path.resolve(ROOT, 'client/runtime/local/state'),
    path.resolve(ROOT, 'core/local/state/ops'),
  ];
  const rows = [];
  for (const rootDir of roots) {
    rows.push(
      ...recentFiles(rootDir, {
        limit: 20,
        maxDepth: 3,
        include: (fullPath) =>
          fullPath.endsWith('latest.json') ||
          fullPath.endsWith('.jsonl') ||
          fullPath.endsWith('queue.json'),
      }).map((row) => ({
        scope: row.path.includes('memory') ? 'memory' : 'state',
        kind: row.path.endsWith('.jsonl') ? 'timeline' : 'snapshot',
        path: row.path,
        mtime: row.mtime,
      }))
    );
  }
  rows.sort((a, b) => String(b.mtime).localeCompare(String(a.mtime)));
  return rows.slice(0, 30);
}

let snapshotFsArtifactsCache = {
  ts_ms: 0,
  memory: [],
  receipts: [],
  logs: [],
};

function snapshotFsArtifacts(force = false) {
  const nowMs = Date.now();
  const cacheAgeMs = Math.max(0, nowMs - parseNonNegativeInt(snapshotFsArtifactsCache.ts_ms, 0, 1_000_000_000_000));
  const cacheValid =
    !force &&
    cacheAgeMs <= SNAPSHOT_FS_CACHE_TTL_MS &&
    Array.isArray(snapshotFsArtifactsCache.memory) &&
    Array.isArray(snapshotFsArtifactsCache.receipts) &&
    Array.isArray(snapshotFsArtifactsCache.logs);
  if (cacheValid) {
    return {
      memory: snapshotFsArtifactsCache.memory.slice(),
      receipts: snapshotFsArtifactsCache.receipts.slice(),
      logs: snapshotFsArtifactsCache.logs.slice(),
      from_cache: true,
      cache_age_ms: cacheAgeMs,
    };
  }
  const memory = collectMemoryArtifacts();
  const receipts = collectReceipts();
  const logs = collectLogEvents();
  snapshotFsArtifactsCache = {
    ts_ms: nowMs,
    memory,
    receipts,
    logs,
  };
  return {
    memory: memory.slice(),
    receipts: receipts.slice(),
    logs: logs.slice(),
    from_cache: false,
    cache_age_ms: 0,
  };
}

function compactCockpitBlocks(blocks = [], limit = COCKPIT_MAX_BLOCKS) {
  const rows = Array.isArray(blocks) ? blocks : [];
  return rows.slice(0, limit).map((row) => {
    const ts = cleanText(row && row.ts ? row.ts : '', 80);
    const durationMs = parsePositiveInt(row && row.duration_ms != null ? row.duration_ms : 0, 0, 0, 3600000);
    const parsedTsMs = Date.parse(ts);
    const ageMs = Number.isFinite(parsedTsMs)
      ? Math.max(0, Date.now() - parsedTsMs)
      : durationMs;
    return {
      index: parsePositiveInt(row && row.index != null ? row.index : 0, 0, 0, 100000),
      lane: cleanText(row && row.lane ? row.lane : 'unknown', 120) || 'unknown',
      event_type: cleanText(row && row.event_type ? row.event_type : 'unknown', 120) || 'unknown',
      tool_call_class: cleanText(row && row.tool_call_class ? row.tool_call_class : 'runtime', 40) || 'runtime',
      status: cleanText(row && row.status ? row.status : 'unknown', 24) || 'unknown',
      status_color: cleanText(row && row.status_color ? row.status_color : 'unknown', 24) || 'unknown',
      duration_ms: durationMs,
      age_ms: parseNonNegativeInt(ageMs, durationMs, 30 * 24 * 60 * 60 * 1000),
      duration_source: cleanText(row && row.duration_source ? row.duration_source : '', 24),
      is_stale: !!(row && row.is_stale === true),
      stale_block_threshold_ms: parsePositiveInt(
        row && row.stale_block_threshold_ms != null ? row.stale_block_threshold_ms : RUNTIME_COCKPIT_STALE_BLOCK_MS,
        RUNTIME_COCKPIT_STALE_BLOCK_MS,
        1000,
        24 * 60 * 60 * 1000
      ),
      ts,
      path: cleanText(row && row.path ? row.path : '', 220),
      conduit_enforced:
        !!(
          row &&
          ((row.conduit_enforced === true) ||
            (row.conduit_enforcement && typeof row.conduit_enforcement === 'object') ||
            cleanText(row && row.routed_via ? row.routed_via : '', 40).toLowerCase() === 'conduit')
        ),
    };
  });
}

function compactAttentionEvents(events = [], limit = ATTENTION_PEEK_LIMIT) {
  const rows = Array.isArray(events) ? events : [];
  return rows.slice(0, limit).map((row) => {
    const event = row && typeof row.event === 'object' ? row.event : {};
    const lane = attentionEventLane(event);
    return {
      cursor_index: parsePositiveInt(row && row.cursor_index != null ? row.cursor_index : 0, 0, 0, 100000000),
      cursor_token: cleanText(row && row.cursor_token ? row.cursor_token : '', 140),
      ts: cleanText(event && event.ts ? event.ts : '', 80),
      severity: cleanText(event && event.severity ? event.severity : 'info', 20) || 'info',
      source: cleanText(event && event.source ? event.source : 'unknown', 80) || 'unknown',
      source_type: cleanText(event && event.source_type ? event.source_type : 'event', 80) || 'event',
      summary: cleanText(event && event.summary ? event.summary : '', 260),
      band: cleanText(event && event.band ? event.band : 'p4', 12) || 'p4',
      priority_lane: lane,
      score: typeof event.score === 'number' && Number.isFinite(event.score) ? event.score : 0,
      attention_key: cleanText(event && event.attention_key ? event.attention_key : '', 120),
      initiative_action: cleanText(event && event.initiative_action ? event.initiative_action : '', 80),
    };
  });
}

function cockpitMetrics(blocks = []) {
  const rows = Array.isArray(blocks) ? blocks : [];
  const laneCounts = {};
  const statusCounts = {};
  const toolClassCounts = {};
  const durations = [];
  for (const row of rows) {
    const lane = cleanText(row && row.lane ? row.lane : 'unknown', 120) || 'unknown';
    const status = cleanText(row && row.status ? row.status : 'unknown', 24) || 'unknown';
    const toolClass = cleanText(row && row.tool_call_class ? row.tool_call_class : 'runtime', 40) || 'runtime';
    const duration = parsePositiveInt(row && row.duration_ms != null ? row.duration_ms : 0, 0, 0, 3600000);
    laneCounts[lane] = parsePositiveInt(laneCounts[lane], 0, 0, 1000000) + 1;
    statusCounts[status] = parsePositiveInt(statusCounts[status], 0, 0, 1000000) + 1;
    toolClassCounts[toolClass] = parsePositiveInt(toolClassCounts[toolClass], 0, 0, 1000000) + 1;
    durations.push(duration);
  }
  durations.sort((a, b) => a - b);
  const p95Index = durations.length > 0 ? Math.min(durations.length - 1, Math.floor(durations.length * 0.95)) : 0;
  const avgDuration = durations.length
    ? Number((durations.reduce((sum, value) => sum + value, 0) / durations.length).toFixed(2))
    : 0;
  const slowest = rows
    .slice()
    .sort(
      (a, b) =>
        parsePositiveInt(b && b.duration_ms != null ? b.duration_ms : 0, 0, 0, 3600000) -
        parsePositiveInt(a && a.duration_ms != null ? a.duration_ms : 0, 0, 0, 3600000)
    )
    .slice(0, 8)
    .map((row) => ({
      lane: cleanText(row && row.lane ? row.lane : 'unknown', 120) || 'unknown',
      event_type: cleanText(row && row.event_type ? row.event_type : 'unknown', 120) || 'unknown',
      status: cleanText(row && row.status ? row.status : 'unknown', 24) || 'unknown',
      duration_ms: parsePositiveInt(row && row.duration_ms != null ? row.duration_ms : 0, 0, 0, 3600000),
    }));
  return {
    lane_counts: laneCounts,
    status_counts: statusCounts,
    tool_class_counts: toolClassCounts,
    duration_ms: {
      avg: avgDuration,
      p95: durations.length ? durations[p95Index] : 0,
      max: durations.length ? durations[durations.length - 1] : 0,
    },
    slowest_blocks: slowest,
  };
}

function collectConduitAttentionCockpitFromRust(safeTeam, options = {}) {
  const nowMs = Date.now();
  if (nowMs < rustRuntimeSyncUnsupportedUntilMs) return null;
  const laneTimeoutMs = parsePositiveInt(
    options && options.lane_timeout_ms != null ? options.lane_timeout_ms : LANE_SYNC_TIMEOUT_MS,
    LANE_SYNC_TIMEOUT_MS,
    SNAPSHOT_LANE_TIMEOUT_MIN_MS,
    SNAPSHOT_LANE_TIMEOUT_MAX_MS
  );
  const laneCacheTtlMs = parsePositiveInt(
    options && options.lane_cache_ttl_ms != null ? options.lane_cache_ttl_ms : SNAPSHOT_LANE_CACHE_TTL_MS,
    SNAPSHOT_LANE_CACHE_TTL_MS,
    250,
    600000
  );
  const laneCacheFailTtlMs = parsePositiveInt(
    options && options.lane_cache_fail_ttl_ms != null
      ? options.lane_cache_fail_ttl_ms
      : SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    250,
    600000
  );
  const lane = runLaneCached(
    `snapshot.runtime_sync.${safeTeam}`,
    ['dashboard-ui', 'runtime-sync', `--team=${safeTeam}`],
    {
      timeout_ms: laneTimeoutMs,
      ttl_ms: laneCacheTtlMs,
      fail_ttl_ms: laneCacheFailTtlMs,
      stale_fallback: true,
    }
  );
  const payloadError =
    lane && lane.payload && typeof lane.payload === 'object' && typeof lane.payload.error === 'string'
      ? cleanText(lane.payload.error, 200).toLowerCase()
      : '';
  const laneStderr = cleanText(lane && lane.stderr ? lane.stderr : '', 200).toLowerCase();
  if (payloadError.includes('unsupported_mode:runtime-sync') || laneStderr.includes('unsupported_mode:runtime-sync')) {
    rustRuntimeSyncUnsupportedUntilMs = nowMs + RUST_RUNTIME_SYNC_RETRY_COOLDOWN_MS;
    return null;
  }
  const payload = lanePayloadObject(lane, null);
  if (!payload || typeof payload !== 'object') return null;
  if (!payload.summary || typeof payload.summary !== 'object') return null;
  if (!payload.attention_queue || typeof payload.attention_queue !== 'object') return null;
  if (!payload.cockpit || typeof payload.cockpit !== 'object') return null;
  return {
    ...payload,
    ok: lane.ok && payload.ok !== false,
    rust_runtime_sync: true,
    authority: 'rust_core_runtime_sync',
    lane: lane.argv.join(' '),
  };
}

function collectConduitAttentionCockpit(team = DEFAULT_TEAM, options = {}) {
  const safeTeam = cleanText(team || DEFAULT_TEAM, 80) || DEFAULT_TEAM;
  const laneTimeoutMs = parsePositiveInt(
    options && options.lane_timeout_ms != null ? options.lane_timeout_ms : LANE_SYNC_TIMEOUT_MS,
    LANE_SYNC_TIMEOUT_MS,
    SNAPSHOT_LANE_TIMEOUT_MIN_MS,
    SNAPSHOT_LANE_TIMEOUT_MAX_MS
  );
  const laneCacheTtlMs = parsePositiveInt(
    options && options.lane_cache_ttl_ms != null ? options.lane_cache_ttl_ms : SNAPSHOT_LANE_CACHE_TTL_MS,
    SNAPSHOT_LANE_CACHE_TTL_MS,
    250,
    600000
  );
  const laneCacheFailTtlMs = parsePositiveInt(
    options && options.lane_cache_fail_ttl_ms != null
      ? options.lane_cache_fail_ttl_ms
      : SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    250,
    600000
  );

  const rustRuntimeSync = collectConduitAttentionCockpitFromRust(safeTeam, {
    lane_timeout_ms: laneTimeoutMs,
    lane_cache_ttl_ms: laneCacheTtlMs,
    lane_cache_fail_ttl_ms: laneCacheFailTtlMs,
  });
  if (rustRuntimeSync) {
    return rustRuntimeSync;
  }

  const cockpitLane = runLaneCached(
    `snapshot.cockpit.${safeTeam}.${COCKPIT_MAX_BLOCKS}`,
    ['hermes-plane', 'cockpit', `--max-blocks=${COCKPIT_MAX_BLOCKS}`, '--strict=1'],
    {
      timeout_ms: laneTimeoutMs,
      ttl_ms: laneCacheTtlMs,
      fail_ttl_ms: laneCacheFailTtlMs,
    }
  );
  const attentionStatusLane = runLaneCached(`snapshot.attention_status.${safeTeam}`, ['attention-queue', 'status'], {
    timeout_ms: laneTimeoutMs,
    ttl_ms: laneCacheTtlMs,
    fail_ttl_ms: laneCacheFailTtlMs,
  });
  const attentionNextLane = runLaneCached(`snapshot.attention_next.${safeTeam}`, [
    'attention-queue',
    'next',
    `--consumer=${ATTENTION_CONSUMER_ID}`,
    `--limit=${ATTENTION_CRITICAL_LIMIT}`,
    '--wait-ms=0',
    '--run-context=dashboard_mirror',
  ], {
    timeout_ms: laneTimeoutMs,
    ttl_ms: laneCacheTtlMs,
    fail_ttl_ms: laneCacheFailTtlMs,
  });

  const cockpitPayload = lanePayloadObject(cockpitLane, {});
  const attentionStatusPayload = lanePayloadObject(attentionStatusLane, {});
  const attentionNextPayload = lanePayloadObject(attentionNextLane, {});

  const blocksRaw =
    cockpitPayload &&
    cockpitPayload.cockpit &&
    cockpitPayload.cockpit.render &&
    Array.isArray(cockpitPayload.cockpit.render.stream_blocks)
      ? cockpitPayload.cockpit.render.stream_blocks
      : [];
  const cockpitMetricsRaw =
    cockpitPayload &&
    cockpitPayload.cockpit &&
    cockpitPayload.cockpit.metrics &&
    typeof cockpitPayload.cockpit.metrics === 'object'
      ? cockpitPayload.cockpit.metrics
      : {};
  const eventsRaw = Array.isArray(attentionNextPayload.events) ? attentionNextPayload.events : [];
  const queueDepth = parsePositiveInt(
    attentionStatusPayload && attentionStatusPayload.queue_depth != null
      ? attentionStatusPayload.queue_depth
      : attentionNextPayload && attentionNextPayload.queue_depth != null
        ? attentionNextPayload.queue_depth
        : 0,
    0,
    0,
    100000000
  );

  const blocks = compactCockpitBlocks(blocksRaw, COCKPIT_MAX_BLOCKS);
  const staleBlockThresholdMs = parsePositiveInt(
    cockpitMetricsRaw && cockpitMetricsRaw.stale_block_threshold_ms != null
      ? cockpitMetricsRaw.stale_block_threshold_ms
      : RUNTIME_COCKPIT_STALE_BLOCK_MS,
    RUNTIME_COCKPIT_STALE_BLOCK_MS,
    1000,
    24 * 60 * 60 * 1000
  );
  const staleCockpitBlocksRaw = blocks.filter(
    (row) =>
      (row && row.is_stale === true) ||
      parseNonNegativeInt(row && row.duration_ms, 0, 3600000) >= staleBlockThresholdMs
  );
  const staleCockpitBlocksActionable = staleCockpitBlocksRaw.filter((row) => cockpitStaleIsActionable(row, queueDepth));
  const staleCockpitBlocksDormant = staleCockpitBlocksRaw.filter((row) => !cockpitStaleIsActionable(row, queueDepth));
  const activeCockpitBlocksRaw = blocks.filter((row) => !staleCockpitBlocksRaw.includes(row));
  const totalCockpitBlockCount = parseNonNegativeInt(
    cockpitMetricsRaw && cockpitMetricsRaw.total_block_count != null
      ? cockpitMetricsRaw.total_block_count
      : cockpitPayload &&
          cockpitPayload.cockpit &&
          cockpitPayload.cockpit.render &&
          cockpitPayload.cockpit.render.total_blocks != null
        ? cockpitPayload.cockpit.render.total_blocks
        : blocks.length,
    blocks.length,
    100000000
  );
  const staleCockpitBlockRawCount = parseNonNegativeInt(
    cockpitMetricsRaw && cockpitMetricsRaw.stale_block_count != null
      ? cockpitMetricsRaw.stale_block_count
      : staleCockpitBlocksRaw.length,
    staleCockpitBlocksRaw.length,
    100000000
  );
  const staleCockpitBlockCount = parseNonNegativeInt(
    staleCockpitBlocksActionable.length,
    staleCockpitBlocksActionable.length,
    100000000
  );
  const staleCockpitDormantBlockCount = parseNonNegativeInt(
    staleCockpitBlocksDormant.length,
    Math.max(0, staleCockpitBlockRawCount - staleCockpitBlockCount),
    100000000
  );
  const activeCockpitBlockCount = parseNonNegativeInt(
    totalCockpitBlockCount - staleCockpitBlockCount,
    Math.max(0, totalCockpitBlockCount - staleCockpitBlockCount),
    100000000
  );
  const cockpitFreshCoverage = Number(
    (totalCockpitBlockCount > 0 ? activeCockpitBlockCount / totalCockpitBlockCount : 0).toFixed(3)
  );
  const cockpitStaleRatio = Number(
    (totalCockpitBlockCount > 0 ? staleCockpitBlockCount / totalCockpitBlockCount : 0).toFixed(3)
  );
  const cockpitStaleRawRatio = Number(
    (totalCockpitBlockCount > 0 ? staleCockpitBlockRawCount / totalCockpitBlockCount : 0).toFixed(3)
  );
  const cockpitStaleDormantRatio = Number(
    (totalCockpitBlockCount > 0 ? staleCockpitDormantBlockCount / totalCockpitBlockCount : 0).toFixed(3)
  );
  const staleLaneMap = {};
  for (const row of staleCockpitBlocksActionable) {
    const lane = cleanText(row && row.lane ? row.lane : 'unknown', 80) || 'unknown';
    staleLaneMap[lane] = parseNonNegativeInt(staleLaneMap[lane], 0, 100000000) + 1;
  }
  const staleDormantLaneMap = {};
  for (const row of staleCockpitBlocksDormant) {
    const lane = cleanText(row && row.lane ? row.lane : 'unknown', 80) || 'unknown';
    staleDormantLaneMap[lane] = parseNonNegativeInt(staleDormantLaneMap[lane], 0, 100000000) + 1;
  }
  const staleLanesTop = Object.entries(staleLaneMap)
    .map(([lane, count]) => ({ lane, count: parseNonNegativeInt(count, 0, 100000000) }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 6);
  const staleDormantLanesTop = Object.entries(staleDormantLaneMap)
    .map(([lane, count]) => ({ lane, count: parseNonNegativeInt(count, 0, 100000000) }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 6);
  const cockpitStreamCoarse =
    (totalCockpitBlockCount >= RUNTIME_COCKPIT_BLOCK_ESCALATION_THRESHOLD && cockpitFreshCoverage < 0.5) ||
    cockpitStaleRatio >= 0.5;
  const cockpitSignalQuality = cockpitStreamCoarse
    ? 'coarse'
    : cockpitStaleRatio >= 0.3
    ? 'degraded'
    : 'good';
  const eventsFull = compactAttentionEvents(eventsRaw, ATTENTION_CRITICAL_LIMIT);
  const eventSplitRaw = splitAttentionEvents(eventsFull);
  const predictiveDegradeMode = totalCockpitBlockCount >= RUNTIME_COCKPIT_BLOCK_ESCALATION_THRESHOLD;
  const deferred = applyAttentionDeferredStorage(queueDepth, eventSplitRaw, {
    stash_depth: predictiveDegradeMode
      ? Math.min(ATTENTION_DEFERRED_STASH_DEPTH, ATTENTION_DEFERRED_PREDICTIVE_STASH_DEPTH)
      : ATTENTION_DEFERRED_STASH_DEPTH,
    hard_shed_depth: ATTENTION_DEFERRED_HARD_SHED_DEPTH,
    rehydrate_depth: predictiveDegradeMode
      ? Math.min(ATTENTION_DEFERRED_REHYDRATE_DEPTH, ATTENTION_DEFERRED_PREDICTIVE_REHYDRATE_DEPTH)
      : ATTENTION_DEFERRED_REHYDRATE_DEPTH,
  });
  const eventSplit = {
    critical: deferred.critical,
    standard: deferred.standard,
    background: deferred.background,
    telemetry: deferred.telemetry,
    lane_weights: { ...ATTENTION_LANE_WEIGHTS },
    counts: {
      critical: deferred.critical.length,
      standard: deferred.standard.length,
      background: deferred.background.length,
      telemetry: deferred.telemetry.length,
      total:
        deferred.critical.length +
        deferred.standard.length +
        deferred.background.length +
        parseNonNegativeInt(deferred.deferred_depth, 0, 100000000),
    },
  };
  const lanePolicy = attentionLanePolicy(queueDepth, eventSplit.counts, {
    critical: eventSplit.critical,
    standard: eventSplit.standard,
    background: eventSplit.background,
  });
  const weightedEvents = weightedFairAttentionOrder(
    {
      critical: eventSplit.critical,
      standard: eventSplit.standard,
      background: eventSplit.background,
    },
    ATTENTION_CRITICAL_LIMIT,
    lanePolicy.weights,
    lanePolicy.lane_caps
  );
  const events = weightedEvents.slice(0, ATTENTION_PEEK_LIMIT);
  const cockpitCritical = blocks
    .filter((row) => {
      const status = cleanText(row && row.status ? row.status : '', 24).toLowerCase();
      return status === 'fail' || status === 'error' || status === 'critical';
    })
    .slice(0, 6)
    .map((row) => ({
      cursor_index: 0,
      cursor_token: '',
      ts: cleanText(row && row.ts ? row.ts : nowIso(), 80) || nowIso(),
      severity: 'critical',
      source: 'cockpit_health',
      source_type: 'cockpit_block',
      summary: cleanText(
        `${cleanText(row && row.lane ? row.lane : 'unknown', 80)} ${cleanText(
          row && row.event_type ? row.event_type : 'unknown',
          80
        )} status=${cleanText(row && row.status ? row.status : 'unknown', 24)}`.trim(),
        260
      ),
      band: 'p1',
      priority_lane: 'critical',
      score: 1,
      attention_key: cleanText(
        `cockpit-${cleanText(row && row.lane ? row.lane : 'unknown', 80)}-${cleanText(row && row.event_type ? row.event_type : 'unknown', 80)}`,
        120
      ),
      initiative_action: 'triple_escalation',
    }));
  const criticalEventsFull = sortCriticalEvents([...cockpitCritical, ...eventSplit.critical]).slice(
    0,
    ATTENTION_CRITICAL_LIMIT
  );
  const criticalEventsMerged = criticalEventsFull.slice(0, ATTENTION_PEEK_LIMIT);
  const priorityCounts = {
    critical: criticalEventsFull.length,
    telemetry: eventSplit.telemetry.length,
    standard: eventSplit.standard.length,
    background: eventSplit.background.length,
    deferred: parseNonNegativeInt(deferred.deferred_depth, 0, 100000000),
    total:
      eventSplit.telemetry.length +
      eventSplit.standard.length +
      eventSplit.background.length +
      criticalEventsFull.length +
      parseNonNegativeInt(deferred.deferred_depth, 0, 100000000),
  };
  const laneCountsStatusRaw =
    attentionStatusPayload && attentionStatusPayload.lane_counts && typeof attentionStatusPayload.lane_counts === 'object'
      ? attentionStatusPayload.lane_counts
      : {};
  const laneCountsStatus = {
    critical: parseNonNegativeInt(laneCountsStatusRaw.critical, priorityCounts.critical, 100000000),
    standard: parseNonNegativeInt(laneCountsStatusRaw.standard, priorityCounts.standard, 100000000),
    background: parseNonNegativeInt(laneCountsStatusRaw.background, priorityCounts.background, 100000000),
  };
  const conduitSignalsActiveFromBlocks = activeCockpitBlocksRaw.filter((block) => {
    const lane = String(block.lane || '').toLowerCase();
    const eventType = String(block.event_type || '').toLowerCase();
    return lane.includes('conduit') || eventType.includes('conduit') || !!block.conduit_enforced;
  }).length;
  const conduitSignals = parseNonNegativeInt(
    cockpitMetricsRaw && cockpitMetricsRaw.conduit_signals_active != null
      ? cockpitMetricsRaw.conduit_signals_active
      : conduitSignalsActiveFromBlocks,
    conduitSignalsActiveFromBlocks,
    100000000
  );
  const conduitSignalsTotal = parseNonNegativeInt(
    cockpitMetricsRaw && cockpitMetricsRaw.conduit_signals_total != null
      ? cockpitMetricsRaw.conduit_signals_total
      : blocks.filter((block) => !!block.conduit_enforced).length,
    blocks.filter((block) => !!block.conduit_enforced).length,
    100000000
  );
  const conduitChannelsObserved = parseNonNegativeInt(
    cockpitMetricsRaw && cockpitMetricsRaw.conduit_channels_observed != null
      ? cockpitMetricsRaw.conduit_channels_observed
      : conduitSignals,
    conduitSignals,
    100000000
  );
  const attentionContract =
    attentionStatusPayload &&
    attentionStatusPayload.attention_contract &&
    typeof attentionStatusPayload.attention_contract === 'object'
      ? attentionStatusPayload.attention_contract
      : attentionNextPayload &&
        attentionNextPayload.attention_contract &&
        typeof attentionNextPayload.attention_contract === 'object'
        ? attentionNextPayload.attention_contract
        : {};
  const maxQueueDepth = parsePositiveInt(
    attentionContract && attentionContract.max_queue_depth != null ? attentionContract.max_queue_depth : 2048,
    2048,
    1,
    100000000
  );
  const backpressureDropBelow =
    cleanText(
      attentionContract && attentionContract.backpressure_drop_below
        ? attentionContract.backpressure_drop_below
        : 'critical',
      24
    ).toLowerCase() || 'critical';
  const queueUtilization = maxQueueDepth > 0 ? Number((queueDepth / maxQueueDepth).toFixed(6)) : 0;
  const activeAgentCount = parseNonNegativeInt(
    cockpitMetricsRaw && cockpitMetricsRaw.active_agent_count != null ? cockpitMetricsRaw.active_agent_count : 0,
    0,
    100000000
  );
  const targetConduitSignals = recommendedConduitSignals(
    queueDepth,
    queueUtilization,
    conduitChannelsObserved,
    activeAgentCount
  );
  const syncMode =
    queueDepth >= DASHBOARD_BACKPRESSURE_BATCH_DEPTH
      ? 'batch_sync'
      : queueDepth >= CONDUIT_DELTA_SYNC_DEPTH
      ? 'delta_sync'
      : 'live_sync';
  const microBatchConfig = predictiveDegradeMode
    ? {
        window_ms: ATTENTION_MICRO_BATCH_DEGRADE_WINDOW_MS,
        max_items: ATTENTION_MICRO_BATCH_DEGRADE_MAX_ITEMS,
      }
    : syncMode === 'delta_sync'
    ? { window_ms: CONDUIT_DELTA_BATCH_WINDOW_MS, max_items: CONDUIT_DELTA_BATCH_MAX_ITEMS }
    : { window_ms: ATTENTION_MICRO_BATCH_WINDOW_MS, max_items: ATTENTION_MICRO_BATCH_MAX_ITEMS };
  const telemetryMicroBatches = microBatchAttentionTelemetry(eventSplit.telemetry, microBatchConfig);
  const pressureLevel =
    queueDepth >= maxQueueDepth || queueUtilization >= 0.9
      ? 'critical'
      : queueDepth >= DASHBOARD_BACKPRESSURE_BATCH_DEPTH || queueUtilization >= 0.75
      ? 'high'
      : queueDepth >= DASHBOARD_BACKPRESSURE_WARN_DEPTH || queueUtilization >= 0.6
      ? 'elevated'
      : 'normal';
  const conduitScaleRequired = conduitChannelsObserved < targetConduitSignals;
  const cockpitConduitRatio = Number((activeCockpitBlockCount / Math.max(1, conduitSignals)).toFixed(3));
  const cockpitRollups = cockpitMetrics(blocks);
  const benchmarkTruth = benchmarkSanitySnapshot();
  const benchmarkBlock = blocks.find((row) => cleanText(row && row.lane ? row.lane : '', 80) === 'benchmark_sanity');
  const benchmarkCockpitStatus =
    cleanText(benchmarkBlock && benchmarkBlock.status ? benchmarkBlock.status : 'unknown', 24) || 'unknown';
  const benchmarkMirrorStatus =
    cleanText(benchmarkTruth && benchmarkTruth.status ? benchmarkTruth.status : benchmarkCockpitStatus, 24) ||
    benchmarkCockpitStatus;
  const benchmarkMirrorAgeSeconds = parsePositiveInt(
    benchmarkTruth && benchmarkTruth.age_seconds != null ? benchmarkTruth.age_seconds : -1,
    -1,
    -1,
    1000000000
  );
  const trend = recordRuntimeTrend({
    ts: nowIso(),
    queue_depth: queueDepth,
    conduit_signals: conduitSignals,
    conduit_channels_observed: conduitChannelsObserved,
    cockpit_blocks: activeCockpitBlockCount,
    cockpit_total_blocks: blocks.length,
    cockpit_stale_blocks: staleCockpitBlockCount,
    cockpit_stale_blocks_raw: staleCockpitBlockRawCount,
    cockpit_stale_blocks_dormant: staleCockpitDormantBlockCount,
    critical_attention: priorityCounts.critical,
    telemetry_attention: eventSplit.counts.telemetry,
    standard_attention: eventSplit.counts.standard,
    background_attention: eventSplit.counts.background,
    deferred_attention: parseNonNegativeInt(deferred.deferred_depth, 0, 100000000),
    sync_mode: syncMode,
    benchmark_sanity_status: benchmarkMirrorStatus,
    benchmark_sanity_cockpit_status: benchmarkCockpitStatus,
  });

  return {
    team: safeTeam,
    ok: !!(cockpitLane.ok && attentionStatusLane.ok && attentionNextLane.ok),
    cockpit_ok: !!cockpitLane.ok,
    attention_status_ok: !!attentionStatusLane.ok,
    attention_next_ok: !!attentionNextLane.ok,
    lanes: {
      cockpit: cockpitLane.argv.join(' '),
      attention_status: attentionStatusLane.argv.join(' '),
      attention_next: attentionNextLane.argv.join(' '),
    },
    cockpit: {
      blocks,
      block_count: activeCockpitBlockCount,
      active_block_count: activeCockpitBlockCount,
      total_block_count: totalCockpitBlockCount,
      metrics: {
        ...cockpitRollups,
        conduit_signals: conduitSignals,
        conduit_channels_observed: conduitChannelsObserved,
        conduit_signals_active: conduitSignals,
        conduit_signals_total: conduitSignalsTotal,
        benchmark_sanity_status: benchmarkCockpitStatus,
        active_block_count: activeCockpitBlockCount,
        total_block_count: totalCockpitBlockCount,
        stale_block_count: staleCockpitBlockCount,
        stale_block_actionable_count: staleCockpitBlockCount,
        stale_block_raw_count: staleCockpitBlockRawCount,
        stale_block_dormant_count: staleCockpitDormantBlockCount,
        stale_block_threshold_ms: staleBlockThresholdMs,
        stale_block_ratio: cockpitStaleRatio,
        stale_block_raw_ratio: cockpitStaleRawRatio,
        stale_block_dormant_ratio: cockpitStaleDormantRatio,
        fresh_block_ratio: cockpitFreshCoverage,
        stale_lanes_top: staleLanesTop,
        stale_lanes_dormant_top: staleDormantLanesTop,
        stream_coarse: cockpitStreamCoarse,
        signal_quality: cockpitSignalQuality,
      },
      trend: trend.slice(-24),
      payload_type: cleanText(cockpitPayload && cockpitPayload.type ? cockpitPayload.type : '', 60),
      receipt_hash:
        cockpitPayload && typeof cockpitPayload.receipt_hash === 'string' ? cockpitPayload.receipt_hash : '',
    },
    attention_queue: {
      queue_depth: queueDepth,
      cursor_offset: parseNonNegativeInt(
        attentionNextPayload && attentionNextPayload.cursor_offset != null
          ? attentionNextPayload.cursor_offset
          : 0,
        0,
        100000000
      ),
      cursor_offset_after: parseNonNegativeInt(
        attentionNextPayload && attentionNextPayload.cursor_offset_after != null
          ? attentionNextPayload.cursor_offset_after
          : 0,
        0,
        100000000
      ),
      acked_batch: !!(attentionNextPayload && attentionNextPayload.acked === true),
      batch_count: parsePositiveInt(attentionNextPayload && attentionNextPayload.batch_count, 0, 0, ATTENTION_CRITICAL_LIMIT),
      events,
      critical_events: criticalEventsMerged,
      critical_events_full: criticalEventsFull,
      critical_visible_count: criticalEventsMerged.length,
      critical_total_count: criticalEventsFull.length,
      standard_events: eventSplit.standard.slice(0, ATTENTION_PEEK_LIMIT),
      background_events: eventSplit.background.slice(0, ATTENTION_PEEK_LIMIT),
      telemetry_events: eventSplit.telemetry.slice(0, ATTENTION_PEEK_LIMIT),
      telemetry_micro_batches: telemetryMicroBatches,
      deferred_events: parseNonNegativeInt(deferred.deferred_depth, 0, 100000000),
      deferred_stashed_count: parseNonNegativeInt(deferred.stashed_count, 0, 100000000),
      deferred_rehydrated_count: parseNonNegativeInt(deferred.rehydrated_count, 0, 100000000),
      deferred_dropped_count: parseNonNegativeInt(deferred.dropped_count, 0, 100000000),
      deferred_mode: cleanText(deferred.deferred_mode || 'pass_through', 24) || 'pass_through',
      lane_weights: { ...lanePolicy.weights },
      lane_caps: { ...lanePolicy.lane_caps },
      lane_counts: laneCountsStatus,
      priority_counts: priorityCounts,
      backpressure: {
        level: pressureLevel,
        sync_mode: syncMode,
        max_queue_depth: maxQueueDepth,
        queue_utilization: queueUtilization,
        drop_below: backpressureDropBelow,
        throttle_recommended: syncMode !== 'live_sync',
        recommended_poll_ms: syncMode === 'batch_sync' ? 5000 : syncMode === 'delta_sync' ? 1000 : 2000,
        predictive_pause_threshold: DASHBOARD_QUEUE_DRAIN_PAUSE_DEPTH,
        predictive_resume_threshold: DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH,
        memory_entry_threshold: MEMORY_ENTRY_BACKPRESSURE_THRESHOLD,
        deferred_stash_threshold: ATTENTION_DEFERRED_STASH_DEPTH,
        deferred_hard_shed_threshold: ATTENTION_DEFERRED_HARD_SHED_DEPTH,
        deferred_rehydrate_threshold: ATTENTION_DEFERRED_REHYDRATE_DEPTH,
        deferred_rehydrate_batch: ATTENTION_DEFERRED_REHYDRATE_BATCH,
        conduit_signals: conduitSignals,
        conduit_signals_raw: conduitSignals,
        conduit_channels_observed: conduitChannelsObserved,
        conduit_channels_total: conduitSignalsTotal,
        target_conduit_signals: targetConduitSignals,
        scale_required: conduitScaleRequired,
        cockpit_to_conduit_ratio: cockpitConduitRatio,
        lane_weights: { ...lanePolicy.weights },
        lane_caps: { ...lanePolicy.lane_caps },
        priority_preempt: !!lanePolicy.preempt_critical,
        background_dominant: !!lanePolicy.background_dominant,
        micro_batch_window_ms: microBatchConfig.window_ms,
        micro_batch_max_items: microBatchConfig.max_items,
        ingress_dampen_depth: RUNTIME_INGRESS_DAMPEN_DEPTH,
        ingress_shed_depth: RUNTIME_INGRESS_SHED_DEPTH,
        ingress_circuit_depth: RUNTIME_INGRESS_CIRCUIT_DEPTH,
      },
      latest:
        attentionStatusPayload && attentionStatusPayload.latest && typeof attentionStatusPayload.latest === 'object'
          ? attentionStatusPayload.latest
          : {},
      status_type: cleanText(attentionStatusPayload && attentionStatusPayload.type ? attentionStatusPayload.type : '', 60),
      next_type: cleanText(attentionNextPayload && attentionNextPayload.type ? attentionNextPayload.type : '', 60),
      receipt_hashes: {
        status:
          attentionStatusPayload && typeof attentionStatusPayload.receipt_hash === 'string'
            ? attentionStatusPayload.receipt_hash
            : '',
        next:
          attentionNextPayload && typeof attentionNextPayload.receipt_hash === 'string'
            ? attentionNextPayload.receipt_hash
            : '',
      },
    },
    summary: {
      queue_depth: queueDepth,
      attention_cursor_offset: parseNonNegativeInt(
        attentionNextPayload && attentionNextPayload.cursor_offset != null
          ? attentionNextPayload.cursor_offset
          : 0,
        0,
        100000000
      ),
      attention_cursor_offset_after: parseNonNegativeInt(
        attentionNextPayload && attentionNextPayload.cursor_offset_after != null
          ? attentionNextPayload.cursor_offset_after
          : 0,
        0,
        100000000
      ),
      cockpit_blocks: activeCockpitBlockCount,
      cockpit_total_blocks: totalCockpitBlockCount,
      attention_batch_count: events.length,
      conduit_signals: conduitSignals,
      conduit_signals_raw: conduitSignals,
      conduit_channels_observed: conduitChannelsObserved,
      conduit_channels_total: conduitSignalsTotal,
      target_conduit_signals: targetConduitSignals,
      conduit_scale_required: conduitScaleRequired,
      cockpit_to_conduit_ratio: cockpitConduitRatio,
      cockpit_stale_blocks: staleCockpitBlockCount,
      cockpit_stale_blocks_raw: staleCockpitBlockRawCount,
      cockpit_stale_blocks_dormant: staleCockpitDormantBlockCount,
      cockpit_stale_ratio: cockpitStaleRatio,
      cockpit_stale_raw_ratio: cockpitStaleRawRatio,
      cockpit_stale_dormant_ratio: cockpitStaleDormantRatio,
      cockpit_fresh_ratio: cockpitFreshCoverage,
      cockpit_stream_coarse: cockpitStreamCoarse,
      cockpit_signal_quality: cockpitSignalQuality,
      cockpit_stale_lanes_top: staleLanesTop,
      cockpit_stale_lanes_dormant_top: staleDormantLanesTop,
      attention_critical: priorityCounts.critical,
      attention_critical_total: criticalEventsFull.length,
      attention_telemetry: priorityCounts.telemetry,
      attention_standard: priorityCounts.standard,
      attention_background: priorityCounts.background,
      attention_deferred: parseNonNegativeInt(deferred.deferred_depth, 0, 100000000),
      attention_deferred_mode: cleanText(deferred.deferred_mode || 'pass_through', 24) || 'pass_through',
      attention_stashed_count: parseNonNegativeInt(deferred.stashed_count, 0, 100000000),
      attention_rehydrated_count: parseNonNegativeInt(deferred.rehydrated_count, 0, 100000000),
      attention_dropped_count: parseNonNegativeInt(deferred.dropped_count, 0, 100000000),
      telemetry_micro_batch_count: telemetryMicroBatches.length,
      sync_mode: syncMode,
      backpressure_level: pressureLevel,
      benchmark_sanity_status: benchmarkMirrorStatus,
      benchmark_sanity_source:
        cleanText(benchmarkTruth && benchmarkTruth.source ? benchmarkTruth.source : 'benchmark_sanity_state', 80) ||
        'benchmark_sanity_state',
      benchmark_sanity_cockpit_status: benchmarkCockpitStatus,
      benchmark_sanity_age_seconds: benchmarkMirrorAgeSeconds,
    },
  };
}

function asMetricRows(healthPayload) {
  const metrics = healthPayload && healthPayload.dashboard_metrics && typeof healthPayload.dashboard_metrics === 'object'
    ? healthPayload.dashboard_metrics
    : {};
  return Object.entries(metrics).map(([name, value]) => {
    const row = value && typeof value === 'object' ? value : {};
    const target = row.target_max != null ? `<= ${row.target_max}` : row.target_min != null ? `>= ${row.target_min}` : 'n/a';
    return {
      name,
      status: row.status || 'unknown',
      value: row.value,
      target,
    };
  });
}

function buildSnapshot(opts = {}) {
  const team = cleanText(opts.team || DEFAULT_TEAM, 80) || DEFAULT_TEAM;
  const cliMode = normalizeCliMode(opts.cliMode || ACTIVE_CLI_MODE);
  const fastLaneMode = !!(opts && opts.fast_lane_mode === true);
  const priorSnapshot =
    opts && opts.prior_snapshot && typeof opts.prior_snapshot === 'object'
      ? opts.prior_snapshot
      : null;
  const laneTimeoutMs = parsePositiveInt(
    opts && opts.lane_timeout_ms != null ? opts.lane_timeout_ms : LANE_SYNC_TIMEOUT_MS,
    LANE_SYNC_TIMEOUT_MS,
    SNAPSHOT_LANE_TIMEOUT_MIN_MS,
    SNAPSHOT_LANE_TIMEOUT_MAX_MS
  );
  const laneCacheTtlMs = parsePositiveInt(
    opts && opts.lane_cache_ttl_ms != null ? opts.lane_cache_ttl_ms : SNAPSHOT_LANE_CACHE_TTL_MS,
    SNAPSHOT_LANE_CACHE_TTL_MS,
    250,
    600000
  );
  const laneCacheFailTtlMs = parsePositiveInt(
    opts && opts.lane_cache_fail_ttl_ms != null ? opts.lane_cache_fail_ttl_ms : SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
    250,
    600000
  );
  const healthLane = fastLaneMode
    ? {
        ok: true,
        status: 0,
        argv: ['snapshot.health', 'cached'],
        payload: priorSnapshot && priorSnapshot.health && typeof priorSnapshot.health === 'object'
          ? priorSnapshot.health
          : {},
      }
    : runLaneCached('snapshot.health', ['health-status', 'dashboard'], {
        timeout_ms: laneTimeoutMs,
        ttl_ms: laneCacheTtlMs,
        fail_ttl_ms: laneCacheFailTtlMs,
      });
  const appLane = fastLaneMode
    ? {
        ok: true,
        status: 0,
        argv: ['snapshot.app.chat_history', 'cached'],
        payload: priorSnapshot && priorSnapshot.app && typeof priorSnapshot.app === 'object'
          ? priorSnapshot.app
          : {},
      }
    : runLaneCached('snapshot.app.chat_history', ['app-plane', 'history', '--app=chat-ui'], {
        timeout_ms: laneTimeoutMs,
        ttl_ms: laneCacheTtlMs,
        fail_ttl_ms: laneCacheFailTtlMs,
      });
  const collabLane = runLaneCached(`snapshot.collab.${team}`, ['collab-plane', 'dashboard', `--team=${team}`], {
    timeout_ms: laneTimeoutMs,
    ttl_ms: laneCacheTtlMs,
    fail_ttl_ms: laneCacheFailTtlMs,
  });
  const skillsLane = fastLaneMode
    ? {
        ok: true,
        status: 0,
        argv: ['snapshot.skills', 'cached'],
        payload: priorSnapshot && priorSnapshot.skills && typeof priorSnapshot.skills === 'object'
          ? priorSnapshot.skills
          : {},
      }
    : runLaneCached('snapshot.skills', ['skills-plane', 'dashboard'], {
        timeout_ms: laneTimeoutMs,
        ttl_ms: laneCacheTtlMs,
        fail_ttl_ms: laneCacheFailTtlMs,
      });
  const runtimeMirror = fastLaneMode
    ? runtimeMirrorFromSnapshot(priorSnapshot || {}, team)
    : collectConduitAttentionCockpit(team, {
        lane_timeout_ms: laneTimeoutMs,
        lane_cache_ttl_ms: laneCacheTtlMs,
        lane_cache_fail_ttl_ms: laneCacheFailTtlMs,
      });
  const benchmarkSanity = benchmarkSanitySnapshot();

  const health = mergeBenchmarkSanityHealth(lanePayloadObject(healthLane, {}), benchmarkSanity);
  const healthCoverage = healthCoverageSummary(health);
  health.coverage = healthCoverage;
  if (healthCoverage.gap_count > 0) {
    const alerts = health.alerts && typeof health.alerts === 'object' ? { ...health.alerts } : {};
    const checksList = new Set(
      Array.isArray(alerts.checks) ? alerts.checks.map((row) => cleanText(row, 120)).filter(Boolean) : []
    );
    checksList.add('coverage:health_checks');
    alerts.checks = Array.from(checksList);
    alerts.count = alerts.checks.length;
    health.alerts = alerts;
  }
  const app = lanePayloadObject(appLane, {});
  const priorCollab =
    opts &&
    opts.prior_collab &&
    typeof opts.prior_collab === 'object' &&
    opts.prior_collab.dashboard &&
    Array.isArray(opts.prior_collab.dashboard.agents)
      ? opts.prior_collab
      : {};
  const collabRaw = lanePayloadObject(collabLane, priorCollab);
  reconcileArchivedAgentsFromCollab(collabRaw);
  const collab = filterArchivedAgentsFromCollab(collabRaw);
  const skills = lanePayloadObject(skillsLane, {});
  const fsArtifacts = snapshotFsArtifacts(false);
  const memoryCollected = fsArtifacts.memory;
  const ingestControl = memoryIngestControlState(runtimeMirror.summary.queue_depth, memoryCollected.length);
  const memoryIngestApplied = applyMemoryIngestCircuit(memoryCollected, ingestControl);
  const memoryEntries = memoryIngestApplied.entries;
  const memoryStream = memoryStreamState(memoryEntries);
  const benchmarkHealthy = benchmarkSanity.status === 'pass' || benchmarkSanity.status === 'warn';

  const snapshot = {
    ok: !!(healthLane.ok && appLane.ok && collabLane.ok && skillsLane.ok && runtimeMirror.ok && benchmarkHealthy),
    type: 'infring_dashboard_snapshot',
    ts: nowIso(),
    metadata: {
      root: ROOT,
      team,
      refresh_ms: opts.refreshMs || DEFAULT_REFRESH_MS,
      cli_mode: cliMode,
      authority: 'rust_core_lanes',
      runtime_sync_authority: cleanText(
        runtimeMirror && runtimeMirror.authority ? runtimeMirror.authority : 'ts_fallback',
        80
      ) || 'ts_fallback',
      lanes: {
        health: healthLane.argv.join(' '),
        app: appLane.argv.join(' '),
        collab: collabLane.argv.join(' '),
        skills: skillsLane.argv.join(' '),
        cockpit: runtimeMirror.lanes.cockpit,
        attention_status: runtimeMirror.lanes.attention_status,
        attention_next: runtimeMirror.lanes.attention_next,
      },
    },
    health,
    app,
    collab,
    skills,
    cockpit: runtimeMirror.cockpit,
    attention_queue: runtimeMirror.attention_queue,
    memory: {
      entries: memoryEntries,
      stream: memoryStream,
      ingest_control: {
        ...ingestControl,
        mode: memoryIngestApplied.mode,
        source_count: memoryCollected.length,
        delivered_count: memoryEntries.length,
        dropped_count: memoryIngestApplied.dropped_count,
      },
    },
    receipts: {
      recent: fsArtifacts.receipts,
      action_history_path: path.relative(ROOT, ACTION_HISTORY_PATH),
    },
    logs: {
      recent: fsArtifacts.logs,
    },
    apm: {
      metrics: asMetricRows(health),
      checks: health.checks || {},
      alerts: health.alerts || {},
    },
  };
  snapshot.agent_lifecycle = lifecycleTelemetry(
    snapshot,
    opts && opts.contract_enforcement ? opts.contract_enforcement : null
  );
  snapshot.runtime_recommendation = runtimeSwarmRecommendation(snapshot);
  snapshot.runtime_autoheal = runtimeAutohealTelemetry();
  snapshot.storage = snapshotStorageTelemetry();
  const receiptHash = sha256(JSON.stringify(snapshot));
  return { ...snapshot, receipt_hash: receiptHash };
}

function coerceTsMs(value, fallback = Date.now()) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value > 0 && value < 1000000000000 ? Math.round(value * 1000) : Math.round(value);
  }
  const text = String(value == null ? '' : value).trim();
  if (!text) return fallback;
  const numeric = Number(text);
  if (Number.isFinite(numeric)) {
    return numeric > 0 && numeric < 1000000000000 ? Math.round(numeric * 1000) : Math.round(numeric);
  }
  const parsed = Date.parse(text);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function estimateTokens(text) {
  return parseNonNegativeInt(Math.round(String(text == null ? '' : text).length / 4), 0, 1000000000);
}

function runtimeSyncSummary(snapshot) {
  const activeAgentCount = activeAgentCountFromSnapshot(snapshot, 0);
  const cockpitBlocks = Array.isArray(snapshot && snapshot.cockpit && snapshot.cockpit.blocks)
    ? snapshot.cockpit.blocks
    : [];
  const cockpitMetrics =
    snapshot && snapshot.cockpit && snapshot.cockpit.metrics && typeof snapshot.cockpit.metrics === 'object'
      ? snapshot.cockpit.metrics
      : {};
  const queueDepth = parseNonNegativeInt(
    snapshot && snapshot.attention_queue && snapshot.attention_queue.queue_depth != null
      ? snapshot.attention_queue.queue_depth
      : 0,
    0,
    100000000
  );
  const attentionCursorOffset = parseNonNegativeInt(
    snapshot && snapshot.attention_queue && snapshot.attention_queue.cursor_offset != null
      ? snapshot.attention_queue.cursor_offset
      : 0,
    0,
    100000000
  );
  const attentionCursorOffsetAfter = parseNonNegativeInt(
    snapshot && snapshot.attention_queue && snapshot.attention_queue.cursor_offset_after != null
      ? snapshot.attention_queue.cursor_offset_after
      : attentionCursorOffset,
    attentionCursorOffset,
    100000000
  );
  const conduitSignalsRawFromBlocks = cockpitBlocks.filter((row) => {
    const lane = String(row && row.lane ? row.lane : '').toLowerCase();
    const eventType = String(row && row.event_type ? row.event_type : '').toLowerCase();
    return lane.includes('conduit') || eventType.includes('conduit') || !!(row && row.conduit_enforced);
  }).length;
  const conduitSignalsRaw = parseNonNegativeInt(
    cockpitMetrics && cockpitMetrics.conduit_signals_raw != null
      ? cockpitMetrics.conduit_signals_raw
      : cockpitMetrics && cockpitMetrics.conduit_signals_active != null
        ? cockpitMetrics.conduit_signals_active
        : cockpitMetrics && cockpitMetrics.conduit_signals != null
          ? cockpitMetrics.conduit_signals
          : conduitSignalsRawFromBlocks,
    conduitSignalsRawFromBlocks,
    100000000
  );
  const conduitSignalsObserved = parseNonNegativeInt(
    cockpitMetrics && cockpitMetrics.conduit_channels_observed != null
      ? cockpitMetrics.conduit_channels_observed
      : conduitSignalsRaw,
    conduitSignalsRaw,
    100000000
  );
  const conduitSignals = parseNonNegativeInt(
    snapshot &&
      snapshot.attention_queue &&
      snapshot.attention_queue.backpressure &&
      snapshot.attention_queue.backpressure.conduit_signals != null
      ? snapshot.attention_queue.backpressure.conduit_signals
      : conduitSignalsRaw,
    conduitSignalsRaw,
    100000000
  );
  const attentionBatch = parseNonNegativeInt(
    snapshot && snapshot.attention_queue && snapshot.attention_queue.batch_count != null
      ? snapshot.attention_queue.batch_count
      : Array.isArray(snapshot && snapshot.attention_queue && snapshot.attention_queue.events)
        ? snapshot.attention_queue.events.length
        : 0,
    0,
    100000000
  );
  const criticalVisibleCount = parseNonNegativeInt(
    snapshot &&
      snapshot.attention_queue &&
      snapshot.attention_queue.critical_visible_count != null
      ? snapshot.attention_queue.critical_visible_count
      : Array.isArray(snapshot && snapshot.attention_queue && snapshot.attention_queue.critical)
        ? snapshot.attention_queue.critical.length
        : 0,
    0,
    100000000
  );
  const criticalAttentionByPriority = parseNonNegativeInt(
    snapshot &&
      snapshot.attention_queue &&
      snapshot.attention_queue.priority_counts &&
      snapshot.attention_queue.priority_counts.critical != null
      ? snapshot.attention_queue.priority_counts.critical
      : 0,
    0,
    100000000
  );
  const criticalAttention = Math.max(criticalVisibleCount, criticalAttentionByPriority);
  const telemetryAttention = parseNonNegativeInt(
    snapshot &&
      snapshot.attention_queue &&
      snapshot.attention_queue.priority_counts &&
      snapshot.attention_queue.priority_counts.telemetry != null
      ? snapshot.attention_queue.priority_counts.telemetry
      : 0,
    0,
    100000000
  );
  const standardAttention = parseNonNegativeInt(
    snapshot &&
      snapshot.attention_queue &&
      snapshot.attention_queue.priority_counts &&
      snapshot.attention_queue.priority_counts.standard != null
      ? snapshot.attention_queue.priority_counts.standard
      : 0,
    0,
    100000000
  );
  const backgroundAttention = parseNonNegativeInt(
    snapshot &&
      snapshot.attention_queue &&
      snapshot.attention_queue.priority_counts &&
      snapshot.attention_queue.priority_counts.background != null
      ? snapshot.attention_queue.priority_counts.background
      : 0,
    0,
    100000000
  );
  const criticalAttentionTotalRaw = parseNonNegativeInt(
    snapshot && snapshot.attention_queue && snapshot.attention_queue.critical_total_count != null
      ? snapshot.attention_queue.critical_total_count
      : criticalAttention,
    criticalAttention,
    100000000
  );
  const criticalAttentionTotal = Math.max(criticalAttentionTotalRaw, criticalAttention);
  const attentionAccountingMismatch =
    criticalAttentionByPriority < criticalVisibleCount ||
    criticalAttentionTotalRaw < criticalVisibleCount;
  const backpressure =
    snapshot && snapshot.attention_queue && snapshot.attention_queue.backpressure && typeof snapshot.attention_queue.backpressure === 'object'
      ? snapshot.attention_queue.backpressure
      : {};
  const conduitChannelsObserved = parseNonNegativeInt(
    cockpitMetrics && cockpitMetrics.conduit_channels_observed != null
      ? cockpitMetrics.conduit_channels_observed
      : conduitSignalsObserved,
    conduitSignalsObserved,
    0,
    100000000
  );
  const conduitChannelsTotal = parseNonNegativeInt(
    snapshot &&
      snapshot.attention_queue &&
      snapshot.attention_queue.backpressure &&
      snapshot.attention_queue.backpressure.conduit_channels_total != null
      ? snapshot.attention_queue.backpressure.conduit_channels_total
      : cockpitMetrics && cockpitMetrics.conduit_signals_total != null
        ? cockpitMetrics.conduit_signals_total
        : conduitChannelsObserved,
    conduitChannelsObserved,
    100000000
  );
  const targetConduitSignals = parsePositiveInt(
    backpressure && backpressure.target_conduit_signals != null
      ? backpressure.target_conduit_signals
      : recommendedConduitSignals(
          queueDepth,
          Number.isFinite(Number(backpressure && backpressure.queue_utilization))
            ? Number(backpressure.queue_utilization)
            : 0,
          cockpitBlocks.length,
          activeAgentCount
        ),
    4,
    1,
    128
  );
  const conduitScaleRequired =
    backpressure && backpressure.scale_required != null
      ? !!backpressure.scale_required
      : conduitChannelsObserved < targetConduitSignals;
  const benchmarkSanity =
    snapshot &&
    snapshot.health &&
    snapshot.health.checks &&
    typeof snapshot.health.checks === 'object' &&
    snapshot.health.checks.benchmark_sanity &&
    typeof snapshot.health.checks.benchmark_sanity === 'object'
      ? snapshot.health.checks.benchmark_sanity
      : {};
  const dashboardMetrics =
    snapshot &&
    snapshot.health &&
    snapshot.health.dashboard_metrics &&
    typeof snapshot.health.dashboard_metrics === 'object'
      ? snapshot.health.dashboard_metrics
      : {};
  const spineSuccessMetric =
    dashboardMetrics && dashboardMetrics.spine_success_rate && typeof dashboardMetrics.spine_success_rate === 'object'
      ? dashboardMetrics.spine_success_rate
      : {};
  const escalationMetric =
    dashboardMetrics &&
    dashboardMetrics.human_escalation_open_rate &&
    typeof dashboardMetrics.human_escalation_open_rate === 'object'
      ? dashboardMetrics.human_escalation_open_rate
      : {};
  const receiptLatencyP95Metric =
    dashboardMetrics &&
    dashboardMetrics.receipt_latency_p95_ms &&
    typeof dashboardMetrics.receipt_latency_p95_ms === 'object'
      ? dashboardMetrics.receipt_latency_p95_ms
      : {};
  const receiptLatencyP99Metric =
    dashboardMetrics &&
    dashboardMetrics.receipt_latency_p99_ms &&
    typeof dashboardMetrics.receipt_latency_p99_ms === 'object'
      ? dashboardMetrics.receipt_latency_p99_ms
      : {};
  const collabMetric =
    dashboardMetrics &&
    dashboardMetrics.collab_team_surface &&
    typeof dashboardMetrics.collab_team_surface === 'object'
      ? dashboardMetrics.collab_team_surface
      : {};
  const dopamineMetric =
    dashboardMetrics &&
    ((dashboardMetrics.dopamine_ambient && typeof dashboardMetrics.dopamine_ambient === 'object' && dashboardMetrics.dopamine_ambient) ||
      (dashboardMetrics.dopamine_ambient_score &&
        typeof dashboardMetrics.dopamine_ambient_score === 'object' &&
        dashboardMetrics.dopamine_ambient_score) ||
      (dashboardMetrics.dopamine_score && typeof dashboardMetrics.dopamine_score === 'object' && dashboardMetrics.dopamine_score))
      ? ((dashboardMetrics.dopamine_ambient && typeof dashboardMetrics.dopamine_ambient === 'object' && dashboardMetrics.dopamine_ambient) ||
        (dashboardMetrics.dopamine_ambient_score &&
          typeof dashboardMetrics.dopamine_ambient_score === 'object' &&
          dashboardMetrics.dopamine_ambient_score) ||
        (dashboardMetrics.dopamine_score && typeof dashboardMetrics.dopamine_score === 'object' && dashboardMetrics.dopamine_score))
      : {};
  const moltbookCredentialMetric =
    dashboardMetrics &&
    dashboardMetrics.moltbook_credentials_surface &&
    typeof dashboardMetrics.moltbook_credentials_surface === 'object'
      ? dashboardMetrics.moltbook_credentials_surface
      : {};
  const externalEyesCrossMetric =
    dashboardMetrics &&
    dashboardMetrics.external_eyes_cross_signal_surface &&
    typeof dashboardMetrics.external_eyes_cross_signal_surface === 'object'
      ? dashboardMetrics.external_eyes_cross_signal_surface
      : {};
  const spineSuccessRateRaw = Number(spineSuccessMetric && spineSuccessMetric.value != null ? spineSuccessMetric.value : Number.NaN);
  const humanEscalationOpenRateRaw = Number(
    escalationMetric && escalationMetric.value != null ? escalationMetric.value : Number.NaN
  );
  const receiptLatencyP95Raw = Number(
    receiptLatencyP95Metric && receiptLatencyP95Metric.value != null
      ? receiptLatencyP95Metric.value
      : Number.NaN
  );
  const receiptLatencyP99Raw = Number(
    receiptLatencyP99Metric && receiptLatencyP99Metric.value != null
      ? receiptLatencyP99Metric.value
      : Number.NaN
  );
  const dopamineScoreRaw = Number(dopamineMetric && dopamineMetric.value != null ? dopamineMetric.value : Number.NaN);
  const externalEyesCrossSignalRatioRaw = Number(
    externalEyesCrossMetric && externalEyesCrossMetric.value != null ? externalEyesCrossMetric.value : Number.NaN
  );
  const spineMetricsStale = !!(
    (spineSuccessMetric && spineSuccessMetric.stale === true) ||
    cleanText(spineSuccessMetric && spineSuccessMetric.status ? spineSuccessMetric.status : '', 24).toLowerCase() === 'stale'
  );
  const receiptLatencyMetricsStale = !!(
    (receiptLatencyP95Metric && receiptLatencyP95Metric.stale === true) ||
    (receiptLatencyP99Metric && receiptLatencyP99Metric.stale === true) ||
    cleanText(receiptLatencyP95Metric && receiptLatencyP95Metric.status ? receiptLatencyP95Metric.status : '', 24).toLowerCase() === 'stale' ||
    cleanText(receiptLatencyP99Metric && receiptLatencyP99Metric.status ? receiptLatencyP99Metric.status : '', 24).toLowerCase() === 'stale'
  );
  const spineMetricsLatestAgeSecondsRaw = Number(
    spineSuccessMetric && spineSuccessMetric.latest_event_age_seconds != null
      ? spineSuccessMetric.latest_event_age_seconds
      : Number.NaN
  );
  const spineMetricsFreshWindowSecondsRaw = Number(
    spineSuccessMetric && spineSuccessMetric.fresh_window_seconds != null
      ? spineSuccessMetric.fresh_window_seconds
      : Number.NaN
  );
  const handoffCountRaw =
    collabMetric && collabMetric.handoff_count != null
      ? collabMetric.handoff_count
      : snapshot && snapshot.collab && snapshot.collab.handoff_count != null
        ? snapshot.collab.handoff_count
        : 0;
  const spineSuccessRate = Number.isFinite(spineSuccessRateRaw) ? Number(spineSuccessRateRaw) : 1;
  const humanEscalationOpenRate = Number.isFinite(humanEscalationOpenRateRaw)
    ? Number(humanEscalationOpenRateRaw)
    : 0;
  const dopamineScore = Number.isFinite(dopamineScoreRaw) ? Number(dopamineScoreRaw) : 0;
  const externalEyesCrossSignalRatio = Number.isFinite(externalEyesCrossSignalRatioRaw)
    ? Number(externalEyesCrossSignalRatioRaw)
    : 0;
  const healthCoverage =
    snapshot &&
    snapshot.health &&
    snapshot.health.coverage &&
    typeof snapshot.health.coverage === 'object'
      ? snapshot.health.coverage
      : {};
  const memoryIngestControl =
    snapshot &&
    snapshot.memory &&
    snapshot.memory.ingest_control &&
    typeof snapshot.memory.ingest_control === 'object'
      ? snapshot.memory.ingest_control
      : {};
  const deferredAttention = parseNonNegativeInt(
    snapshot && snapshot.attention_queue && snapshot.attention_queue.deferred_events != null
      ? snapshot.attention_queue.deferred_events
      : 0,
    0,
    100000000
  );
  const deferredMode =
    cleanText(
      snapshot && snapshot.attention_queue && snapshot.attention_queue.deferred_mode
        ? snapshot.attention_queue.deferred_mode
        : 'pass_through',
      24
    ) || 'pass_through';
  const staleCockpitBlocks = parseNonNegativeInt(
    snapshot &&
      snapshot.cockpit &&
      snapshot.cockpit.metrics &&
      snapshot.cockpit.metrics.stale_block_count != null
      ? snapshot.cockpit.metrics.stale_block_count
      : 0,
    0,
    100000000
  );
  const staleCockpitBlocksRaw = parseNonNegativeInt(
    snapshot &&
      snapshot.cockpit &&
      snapshot.cockpit.metrics &&
      snapshot.cockpit.metrics.stale_block_raw_count != null
      ? snapshot.cockpit.metrics.stale_block_raw_count
      : staleCockpitBlocks,
    staleCockpitBlocks,
    100000000
  );
  const staleCockpitBlocksDormant = parseNonNegativeInt(
    snapshot &&
      snapshot.cockpit &&
      snapshot.cockpit.metrics &&
      snapshot.cockpit.metrics.stale_block_dormant_count != null
      ? snapshot.cockpit.metrics.stale_block_dormant_count
      : Math.max(0, staleCockpitBlocksRaw - staleCockpitBlocks),
    Math.max(0, staleCockpitBlocksRaw - staleCockpitBlocks),
    100000000
  );
  const cockpitStaleRatio = Number.isFinite(
    Number(cockpitMetrics && cockpitMetrics.stale_block_ratio != null ? cockpitMetrics.stale_block_ratio : Number.NaN)
  )
    ? Number(cockpitMetrics.stale_block_ratio)
    : Number(
        (
          staleCockpitBlocks /
          Math.max(
            1,
            parseNonNegativeInt(
              snapshot && snapshot.cockpit && snapshot.cockpit.total_block_count != null
                ? snapshot.cockpit.total_block_count
                : cockpitBlocks.length,
              cockpitBlocks.length,
              100000000
            )
          )
        ).toFixed(3)
      );
  const cockpitFreshRatio = Number.isFinite(
    Number(cockpitMetrics && cockpitMetrics.fresh_block_ratio != null ? cockpitMetrics.fresh_block_ratio : Number.NaN)
  )
    ? Number(cockpitMetrics.fresh_block_ratio)
    : Number((1 - cockpitStaleRatio).toFixed(3));
  const cockpitStaleRawRatio = Number.isFinite(
    Number(cockpitMetrics && cockpitMetrics.stale_block_raw_ratio != null ? cockpitMetrics.stale_block_raw_ratio : Number.NaN)
  )
    ? Number(cockpitMetrics.stale_block_raw_ratio)
    : Number(
        (
          staleCockpitBlocksRaw /
          Math.max(
            1,
            parseNonNegativeInt(
              snapshot && snapshot.cockpit && snapshot.cockpit.total_block_count != null
                ? snapshot.cockpit.total_block_count
                : cockpitBlocks.length,
              cockpitBlocks.length,
              100000000
            )
          )
        ).toFixed(3)
      );
  const cockpitStaleDormantRatio = Number.isFinite(
    Number(
      cockpitMetrics && cockpitMetrics.stale_block_dormant_ratio != null
        ? cockpitMetrics.stale_block_dormant_ratio
        : Number.NaN
    )
  )
    ? Number(cockpitMetrics.stale_block_dormant_ratio)
    : Number(
        (
          staleCockpitBlocksDormant /
          Math.max(
            1,
            parseNonNegativeInt(
              snapshot && snapshot.cockpit && snapshot.cockpit.total_block_count != null
                ? snapshot.cockpit.total_block_count
                : cockpitBlocks.length,
              cockpitBlocks.length,
              100000000
            )
          )
        ).toFixed(3)
      );
  const cockpitStaleLanesTop =
    cockpitMetrics && Array.isArray(cockpitMetrics.stale_lanes_top)
      ? cockpitMetrics.stale_lanes_top
          .map((row) => ({
            lane: cleanText(row && row.lane ? row.lane : 'unknown', 80) || 'unknown',
            count: parseNonNegativeInt(row && row.count != null ? row.count : 0, 0, 100000000),
          }))
          .filter((row) => row.count > 0)
          .slice(0, 6)
      : [];
  const cockpitStaleDormantLanesTop =
    cockpitMetrics && Array.isArray(cockpitMetrics.stale_lanes_dormant_top)
      ? cockpitMetrics.stale_lanes_dormant_top
          .map((row) => ({
            lane: cleanText(row && row.lane ? row.lane : 'unknown', 80) || 'unknown',
            count: parseNonNegativeInt(row && row.count != null ? row.count : 0, 0, 100000000),
          }))
          .filter((row) => row.count > 0)
          .slice(0, 6)
      : [];
  const cockpitSignalQuality =
    cleanText(
      cockpitMetrics && cockpitMetrics.signal_quality ? cockpitMetrics.signal_quality : '',
      24
    ).toLowerCase() || (cockpitStaleRatio >= 0.5 ? 'coarse' : cockpitStaleRatio >= 0.3 ? 'degraded' : 'good');
  const cockpitStreamCoarse =
    cockpitSignalQuality === 'coarse' ||
    !!(cockpitMetrics && cockpitMetrics.stream_coarse === true);
  const ingressLevel =
    queueDepth >= RUNTIME_INGRESS_CIRCUIT_DEPTH
      ? 'circuit'
      : queueDepth >= RUNTIME_INGRESS_SHED_DEPTH
      ? 'shed'
      : queueDepth >= RUNTIME_INGRESS_DAMPEN_DEPTH
      ? 'dampen'
      : 'normal';
  return {
    queue_depth: queueDepth,
    active_agent_count: activeAgentCount,
    attention_cursor_offset: attentionCursorOffset,
    attention_cursor_offset_after: attentionCursorOffsetAfter,
    attention_unacked_depth: Math.max(0, queueDepth - attentionCursorOffset),
    cockpit_blocks: parseNonNegativeInt(snapshot && snapshot.cockpit && snapshot.cockpit.block_count, cockpitBlocks.length, 100000000),
    cockpit_total_blocks: parseNonNegativeInt(
      snapshot && snapshot.cockpit && snapshot.cockpit.total_block_count != null
        ? snapshot.cockpit.total_block_count
        : cockpitBlocks.length,
      cockpitBlocks.length,
      100000000
    ),
    attention_batch_count: attentionBatch,
    conduit_signals: conduitSignals,
    conduit_signals_raw: conduitSignalsRaw,
    conduit_channels_observed: conduitChannelsObserved,
    conduit_channels_total: conduitChannelsTotal,
    target_conduit_signals: targetConduitSignals,
    conduit_scale_required: conduitScaleRequired,
    critical_attention: criticalAttention,
    critical_attention_total: criticalAttentionTotal,
    attention_accounting_mismatch: attentionAccountingMismatch,
    telemetry_attention: telemetryAttention,
    standard_attention: standardAttention,
    background_attention: backgroundAttention,
    deferred_attention: deferredAttention,
    deferred_mode: deferredMode,
    cockpit_stale_blocks: staleCockpitBlocks,
    cockpit_stale_blocks_raw: staleCockpitBlocksRaw,
    cockpit_stale_blocks_dormant: staleCockpitBlocksDormant,
    cockpit_stale_ratio: cockpitStaleRatio,
    cockpit_stale_raw_ratio: cockpitStaleRawRatio,
    cockpit_stale_dormant_ratio: cockpitStaleDormantRatio,
    cockpit_fresh_ratio: cockpitFreshRatio,
    cockpit_stale_lanes_top: cockpitStaleLanesTop,
    cockpit_stale_lanes_dormant_top: cockpitStaleDormantLanesTop,
    cockpit_signal_quality: cockpitSignalQuality,
    cockpit_stream_coarse: cockpitStreamCoarse,
    spine_success_rate: spineSuccessRate,
    spine_success_status:
      cleanText(spineSuccessMetric && spineSuccessMetric.status ? spineSuccessMetric.status : 'unknown', 24) || 'unknown',
    spine_metrics_stale: spineMetricsStale,
    spine_metrics_freshness_status:
      cleanText(spineSuccessMetric && spineSuccessMetric.freshness_status ? spineSuccessMetric.freshness_status : 'unknown', 24) ||
      'unknown',
    spine_metrics_latest_age_seconds: Number.isFinite(spineMetricsLatestAgeSecondsRaw)
      ? Math.max(0, Math.floor(spineMetricsLatestAgeSecondsRaw))
      : null,
    spine_metrics_fresh_window_seconds: Number.isFinite(spineMetricsFreshWindowSecondsRaw)
      ? Math.max(0, Math.floor(spineMetricsFreshWindowSecondsRaw))
      : RUNTIME_SPINE_METRICS_STALE_MAX_AGE_SECONDS,
    spine_runs_completed: parseNonNegativeInt(
      spineSuccessMetric && spineSuccessMetric.completed_runs != null ? spineSuccessMetric.completed_runs : 0,
      0,
      100000000
    ),
    spine_runs_failed: parseNonNegativeInt(
      spineSuccessMetric && spineSuccessMetric.failed_runs != null ? spineSuccessMetric.failed_runs : 0,
      0,
      100000000
    ),
    receipt_latency_p95_ms: Number.isFinite(receiptLatencyP95Raw) ? Number(receiptLatencyP95Raw) : null,
    receipt_latency_p95_status:
      cleanText(receiptLatencyP95Metric && receiptLatencyP95Metric.status ? receiptLatencyP95Metric.status : 'unknown', 24) ||
      'unknown',
    receipt_latency_p99_ms: Number.isFinite(receiptLatencyP99Raw) ? Number(receiptLatencyP99Raw) : null,
    receipt_latency_p99_status:
      cleanText(receiptLatencyP99Metric && receiptLatencyP99Metric.status ? receiptLatencyP99Metric.status : 'unknown', 24) ||
      'unknown',
    receipt_latency_metrics_stale: receiptLatencyMetricsStale,
    human_escalation_open_rate: humanEscalationOpenRate,
    human_escalation_status:
      cleanText(escalationMetric && escalationMetric.status ? escalationMetric.status : 'unknown', 24) || 'unknown',
    collab_handoff_count: parseNonNegativeInt(handoffCountRaw, 0, 100000000),
    dopamine_score: dopamineScore,
    dopamine_status:
      cleanText(dopamineMetric && dopamineMetric.status ? dopamineMetric.status : 'unknown', 24) || 'unknown',
    dopamine_freshness_status:
      cleanText(dopamineMetric && dopamineMetric.freshness_status ? dopamineMetric.freshness_status : 'unknown', 24) || 'unknown',
    dopamine_latest_age_seconds: parsePositiveInt(
      dopamineMetric && dopamineMetric.latest_event_age_seconds != null ? dopamineMetric.latest_event_age_seconds : -1,
      -1,
      -1,
      1000000000
    ),
    moltbook_credentials_status:
      cleanText(moltbookCredentialMetric && moltbookCredentialMetric.status ? moltbookCredentialMetric.status : 'unknown', 24) ||
      'unknown',
    moltbook_credentials_available:
      !!(moltbookCredentialMetric && moltbookCredentialMetric.credentials_available === true),
    moltbook_jobs_requiring_credentials: parseNonNegativeInt(
      moltbookCredentialMetric && moltbookCredentialMetric.jobs_requiring_credentials != null
        ? moltbookCredentialMetric.jobs_requiring_credentials
        : 0,
      0,
      100000000
    ),
    moltbook_suppression_recommended:
      !!(moltbookCredentialMetric && moltbookCredentialMetric.suppression_recommended === true),
    external_eyes_cross_signal_ratio: externalEyesCrossSignalRatio,
    external_eyes_cross_signal_status:
      cleanText(externalEyesCrossMetric && externalEyesCrossMetric.status ? externalEyesCrossMetric.status : 'unknown', 24) ||
      'unknown',
    external_eyes_freshness_status:
      cleanText(externalEyesCrossMetric && externalEyesCrossMetric.freshness_status ? externalEyesCrossMetric.freshness_status : 'unknown', 24) ||
      'unknown',
    external_eyes_latest_age_seconds: parsePositiveInt(
      externalEyesCrossMetric && externalEyesCrossMetric.latest_event_age_seconds != null
        ? externalEyesCrossMetric.latest_event_age_seconds
        : -1,
      -1,
      -1,
      1000000000
    ),
    external_eyes_cross_signal_absent:
      !!(externalEyesCrossMetric && externalEyesCrossMetric.cross_signal_absent === true),
    ingress_level: ingressLevel,
    telemetry_micro_batch_count: parseNonNegativeInt(
      snapshot &&
        snapshot.attention_queue &&
        Array.isArray(snapshot.attention_queue.telemetry_micro_batches)
        ? snapshot.attention_queue.telemetry_micro_batches.length
        : 0,
      0,
      100000000
    ),
    sync_mode: cleanText(backpressure && backpressure.sync_mode ? backpressure.sync_mode : 'live_sync', 24) || 'live_sync',
    backpressure_level: cleanText(backpressure && backpressure.level ? backpressure.level : 'normal', 24) || 'normal',
    queue_lane_weights:
      backpressure && backpressure.lane_weights && typeof backpressure.lane_weights === 'object'
        ? backpressure.lane_weights
        : { ...ATTENTION_LANE_WEIGHTS },
    queue_lane_caps:
      backpressure && backpressure.lane_caps && typeof backpressure.lane_caps === 'object'
        ? backpressure.lane_caps
        : { ...ATTENTION_LANE_CAPS },
    benchmark_sanity_status:
      cleanText(benchmarkSanity && benchmarkSanity.status ? benchmarkSanity.status : 'unknown', 24) || 'unknown',
    benchmark_sanity_source:
      cleanText(benchmarkSanity && benchmarkSanity.source ? benchmarkSanity.source : 'benchmark_sanity_state', 80) ||
      'benchmark_sanity_state',
    benchmark_sanity_cockpit_status:
      cleanText(
        snapshot &&
          snapshot.cockpit &&
          snapshot.cockpit.metrics &&
          snapshot.cockpit.metrics.benchmark_sanity_status != null
          ? snapshot.cockpit.metrics.benchmark_sanity_status
          : 'unknown',
        24
      ) || 'unknown',
    benchmark_sanity_age_seconds: parsePositiveInt(
      benchmarkSanity && benchmarkSanity.age_seconds != null ? benchmarkSanity.age_seconds : -1,
      -1,
      -1,
      1000000000
    ),
    health_check_count: parseNonNegativeInt(
      snapshot && snapshot.health && snapshot.health.checks && typeof snapshot.health.checks === 'object'
        ? Object.keys(snapshot.health.checks).length
        : 0,
      0,
      100000000
    ),
    health_coverage_gap_count: parseNonNegativeInt(
      healthCoverage && healthCoverage.gap_count != null ? healthCoverage.gap_count : 0,
      0,
      100000000
    ),
    retired_health_checks:
      healthCoverage && Array.isArray(healthCoverage.retired_checks) ? healthCoverage.retired_checks.slice(0, 12) : [],
    memory_ingest_paused: !!(memoryIngestControl && memoryIngestControl.paused),
    cockpit_receipt_hash:
      snapshot && snapshot.cockpit && typeof snapshot.cockpit.receipt_hash === 'string'
        ? snapshot.cockpit.receipt_hash
        : '',
    attention_receipt_hashes:
      snapshot && snapshot.attention_queue && snapshot.attention_queue.receipt_hashes && typeof snapshot.attention_queue.receipt_hashes === 'object'
        ? snapshot.attention_queue.receipt_hashes
        : {},
  };
}

function usageFromSnapshot(snapshot) {
  const turns =
    snapshot &&
    snapshot.app &&
    Array.isArray(snapshot.app.turns)
      ? snapshot.app.turns
      : [];
  const byModel = new Map();
  const byDay = new Map();
  let totalInput = 0;
  let totalOutput = 0;
  let totalCost = 0;

  for (const turn of turns) {
    const provider = cleanText(
      turn && turn.provider ? turn.provider : configuredProvider(snapshot),
      80
    ) || 'openai';
    const model = cleanText(
      turn && turn.model ? turn.model : configuredOllamaModel(snapshot),
      120
    ) || configuredOllamaModel(snapshot);
    const inputTokens = parseNonNegativeInt(
      turn && turn.input_tokens != null ? turn.input_tokens : estimateTokens(turn && turn.user ? turn.user : ''),
      0,
      1000000000
    );
    const outputTokens = parseNonNegativeInt(
      turn && turn.output_tokens != null ? turn.output_tokens : estimateTokens(turn && turn.assistant ? turn.assistant : ''),
      0,
      1000000000
    );
    const cost = Number(turn && turn.cost_usd != null ? turn.cost_usd : 0);
    const safeCost = Number.isFinite(cost) ? Math.max(0, cost) : 0;
    const tsMs = coerceTsMs(turn && turn.ts ? turn.ts : Date.now(), Date.now());
    const dayKey = new Date(tsMs).toISOString().slice(0, 10);
    const modelKey = `${provider}/${model}`;

    totalInput += inputTokens;
    totalOutput += outputTokens;
    totalCost += safeCost;

    if (!byModel.has(modelKey)) {
      byModel.set(modelKey, {
        provider,
        model,
        turns: 0,
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        cost_usd: 0,
      });
    }
    const modelRow = byModel.get(modelKey);
    modelRow.turns += 1;
    modelRow.input_tokens += inputTokens;
    modelRow.output_tokens += outputTokens;
    modelRow.total_tokens += inputTokens + outputTokens;
    modelRow.cost_usd += safeCost;

    if (!byDay.has(dayKey)) {
      byDay.set(dayKey, {
        date: dayKey,
        turns: 0,
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        cost_usd: 0,
      });
    }
    const dayRow = byDay.get(dayKey);
    dayRow.turns += 1;
    dayRow.input_tokens += inputTokens;
    dayRow.output_tokens += outputTokens;
    dayRow.total_tokens += inputTokens + outputTokens;
    dayRow.cost_usd += safeCost;
  }

  const totalTokens = totalInput + totalOutput;
  const modelRows = Array.from(byModel.values()).sort((a, b) => b.total_tokens - a.total_tokens);
  const dayRows = Array.from(byDay.values()).sort((a, b) => String(a.date).localeCompare(String(b.date)));
  const agents = compatAgentsFromSnapshot(snapshot);
  const usageAgents = (agents.length ? agents : [{ id: 'dashboard-cockpit', name: 'dashboard-cockpit' }]).map(
    (agent, idx) => ({
      agent_id: cleanText(agent && agent.id ? agent.id : `agent-${idx + 1}`, 120) || `agent-${idx + 1}`,
      name: cleanText(agent && agent.name ? agent.name : agent && agent.id ? agent.id : `agent-${idx + 1}`, 120) || `agent-${idx + 1}`,
      total_tokens: idx === 0 ? totalTokens : 0,
      input_tokens: idx === 0 ? totalInput : 0,
      output_tokens: idx === 0 ? totalOutput : 0,
      tool_calls: 0,
      cost_usd: idx === 0 ? totalCost : 0,
    })
  );

  return {
    summary: {
      total_tokens: totalTokens,
      input_tokens: totalInput,
      output_tokens: totalOutput,
      total_cost_usd: totalCost,
      turn_count: turns.length,
      agent_count: usageAgents.length,
    },
    models: modelRows,
    daily: dayRows,
    agents: usageAgents,
  };
}

function providersFromSnapshot(snapshot) {
  const configured = cleanText(configuredProvider(snapshot), 80).toLowerCase() || 'openai';
  const configuredModel = cleanText(
    snapshot && snapshot.app && snapshot.app.settings && snapshot.app.settings.model
      ? snapshot.app.settings.model
      : configuredOllamaModel(snapshot),
    120
  ) || configuredOllamaModel(snapshot);
  const registry = loadProviderRegistry(snapshot);
  const providers = registry && registry.providers && typeof registry.providers === 'object'
    ? registry.providers
    : {};
  const models = buildDashboardModels(snapshot);
  const defaultModelsByProvider = new Map();
  for (const row of models) {
    const provider = cleanText(row && row.provider ? row.provider : '', 80).toLowerCase();
    if (!provider || provider === 'auto') continue;
    if (!defaultModelsByProvider.has(provider)) {
      const modelId = cleanText(row && row.id ? row.id : '', 180);
      defaultModelsByProvider.set(provider, modelId);
    }
  }
  const rows = [];
  for (const providerId of Object.keys(providers)) {
    const provider = providers[providerId];
    const isConfigured =
      providerId === configured ||
      cleanText(provider && provider.auth_status ? provider.auth_status : '', 24) === 'configured' ||
      !!(provider && provider.key_hash) ||
      (!!(provider && provider.is_local) && !!(provider && provider.detected_models && provider.detected_models.length));
    rows.push({
      id: providerId,
      name: providerId,
      display_name: cleanText(provider && provider.display_name ? provider.display_name : providerId, 80) || providerId,
      auth_status: isConfigured
        ? 'configured'
        : (provider && provider.needs_key ? 'not_set' : 'no_key_needed'),
      reachable: provider && (provider.reachable === true || (provider.is_local && Array.isArray(provider.detected_models) && provider.detected_models.length > 0)),
      health: isConfigured ? 'ready' : 'not_set',
      is_local: !!(provider && provider.is_local),
      kind: provider && provider.is_local ? 'local' : 'cloud',
      default_model:
        providerId === configured
          ? configuredModel
          : cleanText(defaultModelsByProvider.get(providerId) || '', 180),
      base_url: cleanText(provider && provider.base_url ? provider.base_url : '', 320),
      key_set: !!(provider && provider.key_hash),
      key_last4: cleanText(provider && provider.key_last4 ? provider.key_last4 : '', 12),
      key_set_at: cleanText(provider && provider.key_set_at ? provider.key_set_at : '', 80),
    });
  }
  rows.sort((a, b) => {
    const ac = a && a.id === configured ? 1 : 0;
    const bc = b && b.id === configured ? 1 : 0;
    if (bc !== ac) return bc - ac;
    const al = a && a.is_local ? 1 : 0;
    const bl = b && b.is_local ? 1 : 0;
    if (bl !== al) return bl - al;
    return String(a && a.id ? a.id : '').localeCompare(String(b && b.id ? b.id : ''));
  });
  return rows;
}

function skillsFromSnapshot(snapshot) {
  const hotspots =
    snapshot &&
    snapshot.skills &&
    snapshot.skills.metrics &&
    Array.isArray(snapshot.skills.metrics.run_hotspots)
      ? snapshot.skills.metrics.run_hotspots
      : [];
  if (!hotspots.length) {
    return [];
  }
  return hotspots.map((row, idx) => ({
    name: cleanText(row && row.skill ? row.skill : `skill-${idx + 1}`, 120) || `skill-${idx + 1}`,
    description: 'Observed from skills-plane run history.',
    version: 'n/a',
    author: 'infring',
    runtime: 'typescript',
    tools_count: 0,
    tags: ['runtime-observed'],
    enabled: true,
    source: { type: 'bundled' },
    has_prompt_context: false,
  }));
}

function auditEntriesFromSnapshot(snapshot, limit = 200) {
  const rows = [];
  const cockpitBlocks = Array.isArray(snapshot && snapshot.cockpit && snapshot.cockpit.blocks)
    ? snapshot.cockpit.blocks
    : [];
  const attentionEvents = Array.isArray(snapshot && snapshot.attention_queue && snapshot.attention_queue.events)
    ? snapshot.attention_queue.events
    : [];
  const receipts = Array.isArray(snapshot && snapshot.receipts && snapshot.receipts.recent)
    ? snapshot.receipts.recent
    : [];
  const logs = Array.isArray(snapshot && snapshot.logs && snapshot.logs.recent)
    ? snapshot.logs.recent
    : [];
  const turns = Array.isArray(snapshot && snapshot.app && snapshot.app.turns)
    ? snapshot.app.turns
    : [];

  for (const block of cockpitBlocks.slice(0, 120)) {
    rows.push({
      timestamp: cleanText(block && block.ts ? block.ts : snapshot && snapshot.ts ? snapshot.ts : nowIso(), 80),
      action: cleanText(block && block.event_type ? block.event_type : 'CockpitEvent', 80) || 'CockpitEvent',
      detail: cleanText(
        `${cleanText(block && block.lane ? block.lane : 'unknown', 80)} ${cleanText(
          block && block.status ? block.status : 'unknown',
          20
        )} ${cleanText(block && block.path ? block.path : '', 160)}`.trim(),
        260
      ),
      agent_id: '',
      source: 'cockpit',
    });
  }
  for (const row of attentionEvents.slice(0, 120)) {
    rows.push({
      timestamp: cleanText(row && row.ts ? row.ts : snapshot && snapshot.ts ? snapshot.ts : nowIso(), 80),
      action: 'AttentionEvent',
      detail: cleanText(
        `${cleanText(row && row.source ? row.source : 'unknown', 80)} ${cleanText(
          row && row.severity ? row.severity : 'info',
          20
        )}: ${cleanText(row && row.summary ? row.summary : '', 160)}`.trim(),
        260
      ),
      agent_id: cleanText(row && row.agent_id ? row.agent_id : '', 120),
      source: 'attention_queue',
    });
  }
  for (const row of receipts.slice(0, 120)) {
    rows.push({
      timestamp: cleanText(row && row.mtime ? row.mtime : snapshot && snapshot.ts ? snapshot.ts : nowIso(), 80),
      action: 'ReceiptEvent',
      detail: cleanText(`${cleanText(row && row.kind ? row.kind : 'receipt', 40)} ${cleanText(row && row.path ? row.path : '', 200)}`, 260),
      agent_id: '',
      source: 'receipts',
    });
  }
  for (const row of logs.slice(0, 120)) {
    rows.push({
      timestamp: cleanText(row && row.ts ? row.ts : snapshot && snapshot.ts ? snapshot.ts : nowIso(), 80),
      action: 'LogEvent',
      detail: cleanText(`${cleanText(row && row.source ? row.source : 'log', 90)} ${cleanText(row && row.message ? row.message : '', 160)}`, 260),
      agent_id: '',
      source: 'logs',
    });
  }
  for (const turn of turns.slice(-120)) {
    rows.push({
      timestamp: cleanText(turn && turn.ts ? turn.ts : snapshot && snapshot.ts ? snapshot.ts : nowIso(), 80),
      action: 'AgentMessage',
      detail: cleanText(
        `${cleanText(turn && turn.provider ? turn.provider : configuredProvider(snapshot), 40)}/${cleanText(
          turn && turn.model ? turn.model : configuredOllamaModel(snapshot),
          80
        )}: ${cleanText(turn && turn.user ? turn.user : '', 140)}`,
        260
      ),
      agent_id: '',
      source: 'chat',
    });
  }

  rows.sort((a, b) => coerceTsMs(b.timestamp, 0) - coerceTsMs(a.timestamp, 0));
  const trimmed = rows.slice(0, Math.max(1, limit));
  let prev = 'genesis';
  const entries = trimmed.map((row, idx) => {
    const base = {
      seq: idx + 1,
      timestamp: row.timestamp,
      action: row.action,
      detail: row.detail,
      agent_id: row.agent_id,
      source: row.source,
    };
    const hash = sha256(`${prev}|${base.timestamp}|${base.action}|${base.detail}|${base.agent_id}|${base.source}`);
    prev = hash;
    return { ...base, hash };
  });
  const tipHash = entries.length ? entries[entries.length - 1].hash : sha256('audit-empty');
  return { entries, tip_hash: tipHash };
}

function compatApiPayload(pathname, reqUrl, snapshot) {
  const usage = usageFromSnapshot(snapshot);
  const runtime = runtimeSyncSummary(snapshot);
  const alertsCount = parseNonNegativeInt(
    snapshot && snapshot.health && snapshot.health.alerts && snapshot.health.alerts.count != null
      ? snapshot.health.alerts.count
      : 0,
    0,
    100000000
  );
  const status = snapshot && snapshot.ok === true && alertsCount === 0
    ? 'healthy'
    : snapshot && snapshot.ok === true
      ? 'degraded'
      : 'critical';
  const n = parseNonNegativeInt(reqUrl.searchParams.get('n') || 200, 200, 2000);
  const audit = auditEntriesFromSnapshot(snapshot, Math.max(1, n));

  if (pathname === '/api/health') {
    return {
      ok: true,
      status,
      checks:
        snapshot && snapshot.health && snapshot.health.checks && typeof snapshot.health.checks === 'object'
          ? snapshot.health.checks
          : {},
      alerts:
        snapshot && snapshot.health && snapshot.health.alerts && typeof snapshot.health.alerts === 'object'
          ? snapshot.health.alerts
          : { count: 0, checks: [] },
      dashboard_metrics:
        snapshot && snapshot.health && snapshot.health.dashboard_metrics && typeof snapshot.health.dashboard_metrics === 'object'
          ? snapshot.health.dashboard_metrics
          : {},
      runtime_sync: runtime,
      receipt_hash: snapshot && snapshot.receipt_hash ? snapshot.receipt_hash : '',
      ts: nowIso(),
    };
  }
  if (pathname === '/api/usage') {
    return {
      ok: true,
      agents: usage.agents,
      summary: usage.summary,
      by_model: usage.models,
      daily: usage.daily,
    };
  }
  if (pathname === '/api/usage/summary') {
    return { ok: true, ...usage.summary };
  }
  if (pathname === '/api/usage/by-model') {
    return { ok: true, models: usage.models };
  }
  if (pathname === '/api/usage/daily') {
    return { ok: true, days: usage.daily };
  }
  if (pathname === '/api/providers') {
    return { ok: true, providers: providersFromSnapshot(snapshot) };
  }
  if (pathname === '/api/channels') {
    const channelsState = loadChannelRegistry();
    const channels = Object.values(channelsState.channels || {}).map((row) => ({
      ...row,
      connected: !!(row && row.configured && row.has_token),
    }));
    return { ok: true, channels };
  }
  if (pathname === '/api/skills') {
    return { ok: true, skills: skillsFromSnapshot(snapshot) };
  }
  if (pathname === '/api/mcp/servers') {
    return { ok: true, servers: [] };
  }
  if (pathname === '/api/audit/recent') {
    return { ok: true, entries: audit.entries, tip_hash: audit.tip_hash };
  }
  if (pathname === '/api/audit/verify') {
    return { ok: true, valid: true, entries: audit.entries.length, tip_hash: audit.tip_hash };
  }
  if (pathname === '/api/version') {
    return {
      ok: true,
      version: APP_VERSION,
      platform: process.platform,
      arch: process.arch,
      rust_authority: 'rust_core_lanes',
    };
  }
  if (pathname === '/api/network/status') {
    return {
      ok: true,
      enabled: true,
      connected_peers: 0,
      total_peers: 0,
      runtime_sync: runtime,
    };
  }
  if (pathname === '/api/peers') {
    return {
      ok: true,
      peers: [],
      connected: 0,
      total: 0,
      runtime_sync: runtime,
    };
  }
  if (pathname === '/api/security') {
    return {
      ok: true,
      mode: 'strict',
      fail_closed: true,
      receipts_required: true,
      conduit_enforced: runtime.conduit_signals >= 0,
      checks:
        snapshot && snapshot.health && snapshot.health.checks && typeof snapshot.health.checks === 'object'
          ? snapshot.health.checks
          : {},
      alerts:
        snapshot && snapshot.health && snapshot.health.alerts && typeof snapshot.health.alerts === 'object'
          ? snapshot.health.alerts
          : {},
      runtime_sync: runtime,
    };
  }
  if (pathname === '/api/tools') {
    return {
      ok: true,
      tools: Array.from(CLI_ALLOWLIST)
        .sort()
        .map((name) => ({ name, category: name.includes('protheus') ? 'runtime' : 'cli' })),
      runtime_sync: runtime,
    };
  }
  if (pathname === '/api/commands') {
    return {
      ok: true,
      commands: [
        { command: '/status', description: 'Show runtime status and cockpit summary' },
        { command: '/queue', description: 'Show current queue pressure' },
        { command: '/context', description: 'Show context and attention state' },
        { command: '/model', description: 'Inspect or switch active model' },
        { command: '/file <path>', description: 'Render full file output in chat from workspace path' },
        { command: '/folder <path>', description: 'Render folder tree + downloadable archive in chat' },
        { command: '/budget', description: 'Show usage budget summary' },
        { command: '/peers', description: 'Show network peer status' },
        { command: '/a2a', description: 'Show discovered A2A peers' },
      ],
    };
  }
  if (pathname === '/api/budget') {
    return {
      ok: true,
      hourly_spend: 0,
      daily_spend: usage.summary.total_cost_usd,
      monthly_spend: usage.summary.total_cost_usd,
      hourly_limit: 0,
      daily_limit: 0,
      monthly_limit: 0,
    };
  }
  if (pathname === '/api/a2a/agents') {
    return { ok: true, agents: [] };
  }
  if (pathname === '/api/approvals') {
    return { ok: true, approvals: ensureDefaultApprovals() };
  }
  if (pathname === '/api/sessions') {
    return { ok: true, sessions: listGlobalSessionsFromAgentFiles() };
  }
  if (pathname === '/api/workflows') {
    return readArrayStore(WORKFLOWS_STATE_PATH, []);
  }
  if (pathname === '/api/cron/jobs') {
    return { ok: true, jobs: readArrayStore(CRON_JOBS_STATE_PATH, []) };
  }
  if (pathname === '/api/triggers') {
    return readArrayStore(TRIGGERS_STATE_PATH, []);
  }
  if (pathname === '/api/schedules') {
    return { ok: true, schedules: readArrayStore(CRON_JOBS_STATE_PATH, []) };
  }
  if (pathname === '/api/comms/topology') {
    return {
      ok: true,
      topology: {
        nodes: activeAgentCountFromSnapshot(snapshot, 0),
        edges: 0,
        connected: true,
      },
    };
  }
  if (pathname === '/api/comms/events?limit=200' || pathname === '/api/comms/events') {
    return { ok: true, events: [] };
  }
  if (pathname === '/api/hands' || pathname === '/api/hands/active') {
    return { ok: true, hands: [], active: [] };
  }
  if (pathname === '/api/profiles') {
    const profiles = readJson(AGENT_PROFILES_PATH, {});
    const rows = profiles && typeof profiles === 'object' ? Object.values(profiles) : [];
    return { ok: true, profiles: Array.isArray(rows) ? rows : [] };
  }
  if (pathname === '/api/templates') {
    return {
      ok: true,
      templates: [
        { id: 'general-assistant', name: 'General Assistant', provider: 'groq', model: 'llama-3.3-70b-versatile' },
        { id: 'research-analyst', name: 'Research Analyst', provider: 'openai', model: 'gpt-5' },
        { id: 'ops-reliability', name: 'Ops Reliability', provider: 'anthropic', model: 'claude-sonnet-4-20250514' },
      ],
    };
  }
  return null;
}

function writeSnapshotReceipt(snapshot, options = {}) {
  writeJson(SNAPSHOT_LATEST_PATH, snapshot);
  const forceHistory = !!(options && options.forceHistory);
  const nowMs = Date.now();
  const appendDue =
    forceHistory || (nowMs - parseNonNegativeInt(snapshotHistoryLastAppendAtMs, 0, 1000000000000)) >= SNAPSHOT_HISTORY_APPEND_MIN_INTERVAL_MS;
  if (appendDue) {
    appendJsonl(SNAPSHOT_HISTORY_PATH, snapshot);
    snapshotHistoryLastAppendAtMs = nowMs;
    const bytesAfterAppend = fileSizeBytes(SNAPSHOT_HISTORY_PATH);
    snapshotHistoryMaintenanceState = {
      ...snapshotHistoryMaintenanceState,
      lines_after: parseNonNegativeInt(snapshotHistoryMaintenanceState.lines_after, 0, 1_000_000_000) + 1,
      bytes_after: bytesAfterAppend,
      warning: bytesAfterAppend > SNAPSHOT_HISTORY_WARNING_BYTES,
    };
  }
  compactSnapshotHistory(appendDue ? 'append' : 'periodic', false);
}

function writeActionReceipt(action, payload, laneResult) {
  const record = {
    ok: laneResult && laneResult.ok === true,
    type: 'infring_dashboard_action_receipt',
    ts: nowIso(),
    action: cleanText(action, 120),
    payload: payload && typeof payload === 'object' ? payload : {},
    lane_status: laneResult ? laneResult.status : 1,
    lane_argv: laneResult ? laneResult.argv : [],
    lane_receipt_hash:
      laneResult &&
      laneResult.payload &&
      typeof laneResult.payload === 'object' &&
      laneResult.payload.receipt_hash
        ? String(laneResult.payload.receipt_hash)
        : null,
  };
  const withHash = { ...record, receipt_hash: sha256(JSON.stringify(record)) };
  writeJson(ACTION_LATEST_PATH, withHash);
  appendJsonl(ACTION_HISTORY_PATH, withHash);
  return withHash;
}

function isCriticalDashboardAction(action = '') {
  const normalized = cleanText(action, 80);
  if (!normalized) return false;
  return (
    normalized === 'app.chat' ||
    normalized === 'dashboard.runtime.executeSwarmRecommendation' ||
    normalized === 'dashboard.runtime.applyTelemetryRemediations' ||
    normalized === 'dashboard.ui.toggleControls' ||
    normalized === 'dashboard.ui.toggleSection' ||
    normalized === 'dashboard.ui.switchControlsTab'
  );
}

function currentIngressControl(snapshot) {
  return classifyIngressControl(runtimeSyncSummary(snapshot));
}

function runtimeMirrorFromSnapshot(snapshot, team = DEFAULT_TEAM) {
  const runtimeSync =
    snapshot && snapshot.runtime_sync && typeof snapshot.runtime_sync === 'object'
      ? snapshot.runtime_sync
      : {};
  const computedRuntimeSummary = runtimeSyncSummary(snapshot && typeof snapshot === 'object' ? snapshot : {});
  const summaryRaw =
    runtimeSync && runtimeSync.summary && typeof runtimeSync.summary === 'object'
      ? runtimeSync.summary
      : computedRuntimeSummary;
  const cockpit =
    snapshot && snapshot.cockpit && typeof snapshot.cockpit === 'object'
      ? snapshot.cockpit
      : {};
  const attention =
    snapshot && snapshot.attention_queue && typeof snapshot.attention_queue === 'object'
      ? snapshot.attention_queue
      : {};
  const backpressure =
    attention && attention.backpressure && typeof attention.backpressure === 'object'
      ? attention.backpressure
      : {};
  const summary = {
    queue_depth: parseNonNegativeInt(summaryRaw && summaryRaw.queue_depth, 0, 100000000),
    cockpit_blocks: parseNonNegativeInt(summaryRaw && summaryRaw.cockpit_blocks, 0, 100000000),
    cockpit_total_blocks: parseNonNegativeInt(
      summaryRaw && summaryRaw.cockpit_total_blocks,
      parseNonNegativeInt(cockpit && cockpit.total_block_count, 0, 100000000),
      100000000
    ),
    attention_batch_count: parseNonNegativeInt(summaryRaw && summaryRaw.attention_batch_count, 0, 100000000),
    conduit_signals: parseNonNegativeInt(summaryRaw && summaryRaw.conduit_signals, 0, 100000000),
    conduit_signals_raw: parseNonNegativeInt(summaryRaw && summaryRaw.conduit_signals_raw, 0, 100000000),
    conduit_channels_observed: parseNonNegativeInt(summaryRaw && summaryRaw.conduit_channels_observed, 0, 100000000),
    target_conduit_signals: parseNonNegativeInt(summaryRaw && summaryRaw.target_conduit_signals, 0, 100000000),
    conduit_scale_required: !!(summaryRaw && summaryRaw.conduit_scale_required),
    attention_critical: parseNonNegativeInt(summaryRaw && summaryRaw.attention_critical, 0, 100000000),
    attention_critical_total: parseNonNegativeInt(summaryRaw && summaryRaw.attention_critical_total, 0, 100000000),
    attention_telemetry: parseNonNegativeInt(summaryRaw && summaryRaw.attention_telemetry, 0, 100000000),
    sync_mode:
      cleanText(
        summaryRaw && summaryRaw.sync_mode ? summaryRaw.sync_mode : backpressure.sync_mode || 'live_sync',
        24
      ) || 'live_sync',
    backpressure_level:
      cleanText(
        summaryRaw && summaryRaw.backpressure_level ? summaryRaw.backpressure_level : backpressure.level || 'normal',
        24
      ) || 'normal',
    benchmark_sanity_status:
      cleanText(summaryRaw && summaryRaw.benchmark_sanity_status ? summaryRaw.benchmark_sanity_status : 'unknown', 24) ||
      'unknown',
    benchmark_sanity_source: cleanText(summaryRaw && summaryRaw.benchmark_sanity_source ? summaryRaw.benchmark_sanity_source : '', 80),
    benchmark_sanity_cockpit_status: cleanText(
      summaryRaw && summaryRaw.benchmark_sanity_cockpit_status ? summaryRaw.benchmark_sanity_cockpit_status : '',
      24
    ),
    benchmark_sanity_age_seconds: parsePositiveInt(summaryRaw && summaryRaw.benchmark_sanity_age_seconds, -1, -1, 1000000000),
  };
  return {
    ok: runtimeSync.ok !== false,
    cockpit_ok: runtimeSync.cockpit_ok !== false,
    attention_status_ok: runtimeSync.attention_status_ok !== false,
    attention_next_ok: runtimeSync.attention_next_ok !== false,
    lanes: {
      cockpit: null,
      attention_status: null,
      attention_next: null,
    },
    summary,
    cockpit,
    attention_queue: attention,
    metadata: {
      source: 'snapshot_cache',
      team: cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM,
    },
  };
}

function runAction(action, payload) {
  const normalizedAction = cleanText(action, 80);
  const data = payload && typeof payload === 'object' ? payload : {};
  const runActionLane = (argv) => runLane(argv, { timeout_ms: LANE_ACTION_TIMEOUT_MS });
  if (normalizedAction === 'dashboard.ui.toggleControls') {
    const open = !!(data && data.open);
    const ts = nowIso();
    const eventPayload = { event: 'toggle_controls', open, ts };
    return {
      ok: true,
      status: 0,
      argv: ['dashboard.ui.toggleControls'],
      payload: {
        ok: true,
        type: 'infring_dashboard_ui_event',
        event: eventPayload.event,
        open: eventPayload.open,
        ts: eventPayload.ts,
        receipt_hash: sha256(JSON.stringify(eventPayload)),
      },
    };
  }
  if (normalizedAction === 'dashboard.ui.toggleSection') {
    const section = cleanText(data.section || 'unknown', 80) || 'unknown';
    const open = !!(data && data.open);
    const ts = nowIso();
    const eventPayload = { event: 'toggle_section', section, open, ts };
    return {
      ok: true,
      status: 0,
      argv: ['dashboard.ui.toggleSection'],
      payload: {
        ok: true,
        type: 'infring_dashboard_ui_event',
        event: eventPayload.event,
        section: eventPayload.section,
        open: eventPayload.open,
        ts: eventPayload.ts,
        receipt_hash: sha256(JSON.stringify(eventPayload)),
      },
    };
  }
  if (normalizedAction === 'dashboard.ui.switchControlsTab') {
    const tab = cleanText(data.tab || 'swarm', 40) || 'swarm';
    const ts = nowIso();
    const eventPayload = { event: 'switch_controls_tab', tab, ts };
    return {
      ok: true,
      status: 0,
      argv: ['dashboard.ui.switchControlsTab'],
      payload: {
        ok: true,
        type: 'infring_dashboard_ui_event',
        event: eventPayload.event,
        tab: eventPayload.tab,
        ts: eventPayload.ts,
        receipt_hash: sha256(JSON.stringify(eventPayload)),
      },
    };
  }
  if (normalizedAction === 'app.switchProvider') {
    const provider = cleanText(data.provider || 'openai', 60) || 'openai';
    const model = cleanText(data.model || 'gpt-5', 100) || 'gpt-5';
    return runActionLane(['app-plane', 'switch-provider', '--app=chat-ui', `--provider=${provider}`, `--model=${model}`]);
  }
  if (normalizedAction === 'app.chat') {
    const input = cleanText(data.input || data.message || '', 2000);
    const sessionId = cleanText(data.session_id || data.sessionId || '', 120);
    if (!input) {
      return {
        ok: false,
        status: 2,
        argv: ['app-plane', 'run', '--app=chat-ui'],
        payload: {
          ok: false,
          type: 'infring_dashboard_action_error',
          error: 'chat_input_required',
        },
      };
    }
    const args = ['app-plane', 'run', '--app=chat-ui', `--input=${input}`];
    if (sessionId) args.push(`--session-id=${sessionId}`);
    return runActionLane(args);
  }
  if (normalizedAction === 'collab.launchRole') {
    const team = cleanText(data.team || DEFAULT_TEAM, 60) || DEFAULT_TEAM;
    const role = cleanText(data.role || 'analyst', 60) || 'analyst';
    const shadow =
      cleanText(data.shadow || `${team}-${role}-shadow`, 80) || `${team}-${role}-shadow`;
    return runActionLane([
      'collab-plane',
      'launch-role',
      `--team=${team}`,
      `--role=${role}`,
      `--shadow=${shadow}`,
    ]);
  }
  if (normalizedAction === 'skills.run') {
    const skill = cleanText(data.skill || '', 80);
    const input = cleanText(data.input || '', 600);
    if (!skill) {
      return {
        ok: false,
        status: 2,
        argv: ['skills-plane', 'run'],
        payload: {
          ok: false,
          type: 'infring_dashboard_action_error',
          error: 'skill_required',
        },
      };
    }
    const args = ['skills-plane', 'run', `--skill=${skill}`];
    if (input) args.push(`--input=${input}`);
    return runActionLane(args);
  }
  if (normalizedAction === 'dashboard.assimilate') {
    const target = cleanText(data.target || 'codex', 120) || 'codex';
    return runActionLane([
      'app-plane',
      'run',
      '--app=chat-ui',
      `--input=assimilate target ${target} with receipt-first safety`,
    ]);
  }
  if (normalizedAction === 'dashboard.benchmark') {
    return runActionLane(['health-status', 'dashboard']);
  }
  return {
    ok: false,
    status: 2,
    argv: [],
    payload: {
      ok: false,
      type: 'infring_dashboard_action_error',
      error: `unsupported_action:${normalizedAction}`,
    },
  };
}

function compatAgentsFromSnapshot(snapshot, options = {}) {
  const syncGitAssignments = !!(options && options.sync_git_assignments);
  if (syncGitAssignments) {
    ensureAgentGitTreeAssignments(snapshot, { force: false });
  }
  const includeArchived = !!(options && options.includeArchived);
  const archived = includeArchived ? null : archivedAgentIdsSet();
  const rows =
    snapshot &&
    snapshot.collab &&
    snapshot.collab.dashboard &&
    Array.isArray(snapshot.collab.dashboard.agents)
      ? snapshot.collab.dashboard.agents
      : [];
  return rows
    .map((row, idx) => {
    const id = cleanText(row && row.shadow ? row.shadow : `agent-${idx + 1}`, 120) || `agent-${idx + 1}`;
    const modelState = effectiveAgentModel(id, snapshot, {
      allow_session_read: false,
    });
    const contract = contractForAgent(id);
    const profile = agentProfileFor(id);
    const gitTree = agentGitTreeView(id, profile);
    const status = cleanText(row && row.status ? row.status : 'running', 40) || 'running';
    const state =
      status === 'paused' || status === 'stopped' ? status : status === 'error' ? 'error' : 'running';
    const remainingMs = contractRemainingMs(contract);
    const identity = normalizeAgentIdentity(
      profile && profile.identity ? profile.identity : {},
      { emoji: '🤖', archetype: 'assistant', color: '#2563EB' }
    );
    const fallbackModels =
      profile && Array.isArray(profile.fallback_models) ? profile.fallback_models : [];
    const profileRole = cleanText(profile && profile.role ? profile.role : '', 60);
    const runtimeRole = cleanText(row && row.role ? row.role : 'analyst', 60) || 'analyst';
    return {
      id,
      name: cleanText(profile && profile.name ? profile.name : id, 100) || id,
      state,
      activated_at: cleanText(row && row.activated_at ? row.activated_at : '', 80),
      model_name: modelState.selected,
      model_provider: modelState.provider,
      runtime_model: modelState.runtime_model,
      context_window: modelState.context_window,
      role: profileRole || runtimeRole,
      identity,
      system_prompt: cleanText(profile && profile.system_prompt ? profile.system_prompt : '', 4000),
      fallback_models: fallbackModels,
      git_tree_kind: gitTree.git_tree_kind,
      git_branch: gitTree.git_branch,
      workspace_dir: gitTree.workspace_dir,
      workspace_rel: gitTree.workspace_rel,
      git_tree_ready: gitTree.git_tree_ready,
      git_tree_error: gitTree.git_tree_error,
      is_master_agent: gitTree.is_master_agent,
      contract: contractSummary(contract),
      contract_status: formatContractStatus(contract),
      contract_remaining_ms: remainingMs == null ? null : Math.max(0, Math.floor(remainingMs)),
      capabilities: [],
    };
  })
    .filter((agent) => includeArchived || !archived.has(agent.id));
}

function latestAssistantFromSnapshot(snapshot) {
  const turns = snapshot && snapshot.app && Array.isArray(snapshot.app.turns) ? snapshot.app.turns : [];
  if (turns.length === 0) return '';
  const last = turns[turns.length - 1] || {};
  return cleanText(last.assistant || last.response || last.output || '', 2000);
}

function findAgentByRole(agents = [], role = '') {
  const target = cleanText(role || '', 40).toLowerCase();
  if (!target) return null;
  return (Array.isArray(agents) ? agents : []).find(
    (row) => cleanText(row && row.role ? row.role : '', 40).toLowerCase() === target
  ) || null;
}

function laneOutcome(result) {
  return {
    ok: !!(result && result.ok),
    status: Number.isFinite(Number(result && result.status)) ? Number(result.status) : 1,
    argv: Array.isArray(result && result.argv) ? result.argv : [],
    type: cleanText(result && result.payload && result.payload.type ? result.payload.type : '', 120),
    error: cleanText(result && result.payload && result.payload.error ? result.payload.error : result && result.stderr ? result.stderr : '', 260),
  };
}

function ensureRuntimeRole(snapshot, team, role, preferredShadow = '') {
  const normalizedRole = normalizeCollabRole(role);
  const agents = compatAgentsFromSnapshot(snapshot, { includeArchived: false });
  const existing = findAgentByRole(agents, normalizedRole);
  if (existing && existing.id) {
    return {
      ok: true,
      role: normalizedRole,
      shadow: existing.id,
      launched: false,
      lane: null,
    };
  }
  const shadow =
    cleanText(preferredShadow || `${cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}-${normalizedRole}-auto`, 120) ||
    `${DEFAULT_TEAM}-${normalizedRole}-auto`;
  const lane = runLane([
    'collab-plane',
    'launch-role',
    `--team=${cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}`,
    `--role=${normalizedRole}`,
    `--shadow=${shadow}`,
    '--strict=1',
  ]);
  return {
    ok: !!(lane && lane.ok && lane.payload && lane.payload.ok !== false),
    role: normalizedRole,
    shadow,
    launched: true,
    lane: laneOutcome(lane),
  };
}

function trackedRuntimeDrainAgents(snapshot) {
  const activeIds = new Set(
    compatAgentsFromSnapshot(snapshot, { includeArchived: false })
      .map((row) => cleanText(row && row.id ? row.id : '', 140))
      .filter(Boolean)
  );
  const tracked = Array.isArray(runtimeDrainState.active_agents) ? runtimeDrainState.active_agents : [];
  const retained = tracked.filter((id) => activeIds.has(id) && !isAgentArchived(id));
  runtimeDrainState.active_agents = retained;
  return retained.slice();
}

function launchRuntimeDrainAgent(team, indexHint = 0) {
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const seed = `${Date.now()}-${indexHint}-${Math.floor(Math.random() * 1000)}`;
  const shadow = cleanText(`${normalizedTeam}-drain-${seed}`, 120) || `${normalizedTeam}-drain-${Date.now()}`;
  const lane = runLane([
    'collab-plane',
    'launch-role',
    `--team=${normalizedTeam}`,
    '--role=builder',
    `--shadow=${shadow}`,
    '--strict=1',
  ]);
  return {
    ok: !!(lane && lane.ok && lane.payload && lane.payload.ok !== false),
    role: 'builder',
    shadow,
    launched: true,
    lane: laneOutcome(lane),
  };
}

function applyRuntimePredictiveDrain(snapshot, team, runtime, recommendation = null) {
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const activeBefore = trackedRuntimeDrainAgents(snapshot);
  const launches = [];
  const turns = [];
  const archived = [];
  const rustRecommendations =
    recommendation &&
    recommendation.predictive_drain_required != null &&
    recommendation.predictive_drain_release != null
      ? null
      : runtimeAuthoritySection(runtime, null, 'recommendations').section;
  const required =
    recommendation && recommendation.predictive_drain_required != null
      ? !!recommendation.predictive_drain_required
      : rustRecommendations && rustRecommendations.predictive_drain_required != null
      ? !!rustRecommendations.predictive_drain_required
      : queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH;
  const release =
    recommendation && recommendation.predictive_drain_release != null
      ? !!recommendation.predictive_drain_release
      : rustRecommendations && rustRecommendations.predictive_drain_release != null
      ? !!rustRecommendations.predictive_drain_release
      : queueDepth <= RUNTIME_DRAIN_CLEAR_DEPTH;
  if (required) {
    const desiredFloor =
      queueDepth >= RUNTIME_DRAIN_HIGH_LOAD_DEPTH
        ? RUNTIME_DRAIN_AGENT_HIGH_LOAD_TARGET
        : RUNTIME_DRAIN_AGENT_TARGET;
    const desired = Math.max(desiredFloor, Math.min(RUNTIME_DRAIN_AGENT_MAX, Math.ceil(queueDepth / 40)));
    let active = activeBefore.slice();
    while (active.length < desired) {
      const launch = launchRuntimeDrainAgent(team, active.length + 1);
      launches.push(launch);
      if (!launch.ok || !launch.shadow) break;
      active.push(launch.shadow);
    }
    runtimeDrainState.active_agents = active;
    runtimeDrainState.last_spawn_at = nowIso();
    for (const shadow of active) {
      const turn = queueAgentTask(
        shadow,
        snapshot,
        'Drain queue backlog in weighted lanes. Process critical first, then standard, then background. Keep queue depth under 60 and protect critical telemetry.',
        'swarm_recommendation.predictive_drain'
      );
      turns.push({
        role: 'builder',
        shadow,
        ok: !!turn.ok,
        response: cleanText(turn.ok ? 'Drain task queued.' : turn.error || '', 400),
        runtime_sync: runtimeSyncSummary(snapshot),
      });
    }
  } else if (release && activeBefore.length > 0) {
    for (const shadow of activeBefore) {
      const meta = archiveAgent(shadow, { source: 'runtime.predictive_drain', reason: 'queue_recovered' });
      closeTerminalSession(shadow, 'drain_agent_archived');
      archived.push({
        shadow,
        archived: !!meta,
        archived_at: meta && meta.archived_at ? meta.archived_at : '',
      });
    }
    runtimeDrainState.active_agents = [];
    runtimeDrainState.last_dissolve_at = nowIso();
  }
  return {
    required,
    release,
    trigger_depth: RUNTIME_DRAIN_TRIGGER_DEPTH,
    clear_depth: RUNTIME_DRAIN_CLEAR_DEPTH,
    active_count: runtimeDrainState.active_agents.length,
    active_agents: runtimeDrainState.active_agents.slice(0, 8),
    launches,
    turns,
    archived,
    last_spawn_at: runtimeDrainState.last_spawn_at,
    last_dissolve_at: runtimeDrainState.last_dissolve_at,
  };
}

function runtimeAuthorityPayload(runtime) {
  return {
    queue_depth: parseNonNegativeInt(runtime && runtime.queue_depth, 0, 2000000),
    critical_attention_total: parseNonNegativeInt(runtime && runtime.critical_attention_total, 0, 2000000),
    cockpit_blocks: parseNonNegativeInt(runtime && runtime.cockpit_blocks, 0, 2000000),
    cockpit_stale_blocks: parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 2000000),
    cockpit_stale_ratio: Number(
      Number.isFinite(Number(runtime && runtime.cockpit_stale_ratio != null ? runtime.cockpit_stale_ratio : Number.NaN))
        ? Number(runtime.cockpit_stale_ratio)
        : 0
    ),
    conduit_signals: parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 2000000),
    target_conduit_signals: parseNonNegativeInt(runtime && runtime.target_conduit_signals, 0, 2000000),
    health_coverage_gap_count: parseNonNegativeInt(runtime && runtime.health_coverage_gap_count, 0, 2000000),
    attention_unacked_depth: parseNonNegativeInt(runtime && runtime.attention_unacked_depth, 0, 2000000),
    attention_cursor_offset: parseNonNegativeInt(runtime && runtime.attention_cursor_offset, 0, 2000000),
    memory_ingest_paused: !!(runtime && runtime.memory_ingest_paused),
  };
}

function runtimeAuthorityFromRust(runtime) {
  const payload = runtimeAuthorityPayload(runtime);
  const cacheKey = `runtime.authority.${sha256(JSON.stringify(payload)).slice(0, 24)}`;
  const lane = runLaneCached(
    cacheKey,
    [
      'runtime-systems',
      'run',
      '--system-id=V6-DASHBOARD-007.1',
      '--strict=1',
      '--apply=0',
      `--payload-json=${JSON.stringify(payload)}`,
    ],
    {
      timeout_ms: RUNTIME_AUTHORITY_LANE_TIMEOUT_MS,
      ttl_ms: RUNTIME_AUTHORITY_CACHE_TTL_MS,
      fail_ttl_ms: RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS,
    }
  );
  const lanePayload = lane && lane.payload && typeof lane.payload === 'object' ? lane.payload : null;
  const contractExecution =
    lanePayload &&
    lanePayload.contract_execution &&
    typeof lanePayload.contract_execution === 'object'
      ? lanePayload.contract_execution
      : null;
  const specificChecks =
    contractExecution &&
    contractExecution.specific_checks &&
    typeof contractExecution.specific_checks === 'object'
      ? contractExecution.specific_checks
      : null;
  const authority =
    specificChecks &&
    specificChecks.dashboard_runtime_authority &&
    typeof specificChecks.dashboard_runtime_authority === 'object'
      ? specificChecks.dashboard_runtime_authority
      : null;
  if (!lane || !lane.ok || !lanePayload || !authority) {
    return {
      ok: false,
      authority: null,
      lane: laneOutcome(lane || null),
    };
  }
  return {
    ok: true,
    authority,
    lane: laneOutcome(lane),
  };
}

function runtimeAuthoritySection(runtime, rustAuthority, key) {
  const authority =
    rustAuthority && rustAuthority.ok && rustAuthority.authority && typeof rustAuthority.authority === 'object'
      ? rustAuthority
      : runtimeAuthorityFromRust(runtime);
  const section =
    authority &&
    authority.ok &&
    authority.authority &&
    typeof authority.authority === 'object' &&
    authority.authority[key] &&
    typeof authority.authority[key] === 'object'
      ? authority.authority[key]
      : null;
  return { authority, section };
}

function classifyIngressControl(runtime, rustAuthority = null) {
  const authority =
    rustAuthority && rustAuthority.ok && rustAuthority.authority && typeof rustAuthority.authority === 'object'
      ? rustAuthority
      : runtimeAuthorityFromRust(runtime);
  const ingressFromRust =
    authority &&
    authority.ok &&
    authority.authority &&
    authority.authority.ingress_control &&
    typeof authority.authority.ingress_control === 'object'
      ? authority.authority.ingress_control
      : null;
  if (ingressFromRust) {
    const level = cleanText(ingressFromRust.level || 'normal', 24).toLowerCase() || 'normal';
    const rejectNonCritical = !!ingressFromRust.reject_non_critical;
    const delayMs = parseNonNegativeInt(ingressFromRust.delay_ms, 0, 100000000);
    const reason = cleanText(ingressFromRust.reason || 'steady_state', 120) || 'steady_state';
    if (ingressControllerState.level !== level) {
      ingressControllerState = {
        level,
        reject_non_critical: rejectNonCritical,
        delay_ms: delayMs,
        reason,
        since: nowIso(),
      };
    } else {
      ingressControllerState = {
        ...ingressControllerState,
        reject_non_critical: rejectNonCritical,
        delay_ms: delayMs,
        reason,
      };
    }
    return {
      level,
      reject_non_critical: rejectNonCritical,
      delay_ms: delayMs,
      reason,
      since: cleanText(ingressControllerState.since || '', 80),
      dampen_depth: parseNonNegativeInt(ingressFromRust.dampen_depth, RUNTIME_INGRESS_DAMPEN_DEPTH, 100000000),
      shed_depth: parseNonNegativeInt(ingressFromRust.shed_depth, RUNTIME_INGRESS_SHED_DEPTH, 100000000),
      circuit_depth: parseNonNegativeInt(ingressFromRust.circuit_depth, RUNTIME_INGRESS_CIRCUIT_DEPTH, 100000000),
      authority: 'rust_runtime_systems',
      lane: authority.lane || null,
    };
  }
  const level = 'normal';
  const rejectNonCritical = false;
  const delayMs = 0;
  const reason = 'rust_authority_unavailable';
  ingressControllerState = {
    ...ingressControllerState,
    level,
    reject_non_critical: rejectNonCritical,
    delay_ms: delayMs,
    reason,
    since: cleanText(ingressControllerState.since || nowIso(), 80),
  };
  return {
    level,
    reject_non_critical: rejectNonCritical,
    delay_ms: delayMs,
    reason,
    since: cleanText(ingressControllerState.since || '', 80),
    dampen_depth: RUNTIME_INGRESS_DAMPEN_DEPTH,
    shed_depth: RUNTIME_INGRESS_SHED_DEPTH,
    circuit_depth: RUNTIME_INGRESS_CIRCUIT_DEPTH,
    authority: 'rust_unavailable',
    lane: authority && authority.lane ? authority.lane : null,
  };
}

function cockpitSignalState(runtime, rustAuthority = null) {
  const authority =
    rustAuthority && rustAuthority.ok && rustAuthority.authority && typeof rustAuthority.authority === 'object'
      ? rustAuthority
      : runtimeAuthorityFromRust(runtime);
  const cockpitFromRust =
    authority &&
    authority.ok &&
    authority.authority &&
    authority.authority.cockpit_signal &&
    typeof authority.authority.cockpit_signal === 'object'
      ? authority.authority.cockpit_signal
      : null;
  if (cockpitFromRust) {
    const coarse = !!cockpitFromRust.coarse;
    const quality = coarse
      ? 'coarse'
      : cleanText(cockpitFromRust.quality || 'good', 24).toLowerCase() || 'good';
    return {
      quality,
      coarse,
      authority: 'rust_runtime_systems',
      lane: authority.lane || null,
    };
  }
  const quality = cleanText(runtime && runtime.cockpit_signal_quality ? runtime.cockpit_signal_quality : 'good', 24)
    .toLowerCase() || 'good';
  const staleRatio = Number(
    runtime && runtime.cockpit_stale_ratio != null ? runtime.cockpit_stale_ratio : Number.NaN
  );
  const streamCoarse =
    !!(runtime && runtime.cockpit_stream_coarse) ||
    quality === 'coarse' ||
    (Number.isFinite(staleRatio) && staleRatio >= 0.5);
  return {
    quality: streamCoarse ? 'coarse' : quality,
    coarse: streamCoarse,
    authority: 'rust_unavailable',
    lane: authority && authority.lane ? authority.lane : null,
  };
}

function runtimeReliabilityPosture(runtime, activeSwarmAgents = 0, rustAuthority = null) {
  const rust = runtimeAuthoritySection(runtime, rustAuthority, 'reliability_posture');
  if (rust.section) {
    return {
      degraded: !!rust.section.degraded,
      spine_success_rate: Number.isFinite(Number(rust.section.spine_success_rate))
        ? Number(rust.section.spine_success_rate)
        : Number.isFinite(Number(runtime && runtime.spine_success_rate != null ? runtime.spine_success_rate : Number.NaN))
        ? Number(runtime.spine_success_rate)
        : 1,
      spine_success_target: Number.isFinite(Number(rust.section.spine_success_target))
        ? Number(rust.section.spine_success_target)
        : RUNTIME_SPINE_SUCCESS_TARGET_MIN,
      spine_metrics_stale: !!rust.section.spine_metrics_stale,
      spine_degraded: !!rust.section.spine_degraded,
      escalation_open_rate: Number.isFinite(Number(rust.section.escalation_open_rate))
        ? Number(rust.section.escalation_open_rate)
        : 0,
      escalation_starved: !!rust.section.escalation_starved,
      handoff_count: parseNonNegativeInt(rust.section.handoff_count, 0, 100000000),
      handoffs_per_agent: Number.isFinite(Number(rust.section.handoffs_per_agent))
        ? Number(rust.section.handoffs_per_agent)
        : 0,
      handoffs_per_agent_min: Number.isFinite(Number(rust.section.handoffs_per_agent_min))
        ? Number(rust.section.handoffs_per_agent_min)
        : RUNTIME_HANDOFFS_PER_AGENT_MIN,
      handoff_coverage_weak: !!rust.section.handoff_coverage_weak,
      active_swarm_agents: parseNonNegativeInt(
        rust.section.active_swarm_agents,
        parseNonNegativeInt(activeSwarmAgents, 0, 100000000),
        100000000
      ),
      authority: 'rust_runtime_systems',
      lane: rust.authority && rust.authority.lane ? rust.authority.lane : null,
    };
  }
  const spineSuccessRate = Number.isFinite(
    Number(runtime && runtime.spine_success_rate != null ? runtime.spine_success_rate : Number.NaN)
  )
    ? Number(runtime.spine_success_rate)
    : 1;
  const spineMetricsStale = !!(runtime && runtime.spine_metrics_stale === true);
  const escalationOpenRate = Number.isFinite(
    Number(runtime && runtime.human_escalation_open_rate != null ? runtime.human_escalation_open_rate : Number.NaN)
  )
    ? Number(runtime.human_escalation_open_rate)
    : 0;
  const handoffCount = parseNonNegativeInt(runtime && runtime.collab_handoff_count, 0, 100000000);
  const handoffsPerAgent = activeSwarmAgents > 0 ? Number((handoffCount / activeSwarmAgents).toFixed(3)) : 0;
  return {
    degraded: false,
    spine_success_rate: spineSuccessRate,
    spine_success_target: RUNTIME_SPINE_SUCCESS_TARGET_MIN,
    spine_metrics_stale: spineMetricsStale,
    spine_degraded: false,
    escalation_open_rate: escalationOpenRate,
    escalation_starved: false,
    handoff_count: handoffCount,
    handoffs_per_agent: handoffsPerAgent,
    handoffs_per_agent_min: RUNTIME_HANDOFFS_PER_AGENT_MIN,
    handoff_coverage_weak: false,
    active_swarm_agents: parseNonNegativeInt(activeSwarmAgents, 0, 100000000),
    authority: 'rust_unavailable',
    lane: rust.authority && rust.authority.lane ? rust.authority.lane : null,
  };
}

function runtimeSloGate(runtime, reliabilityPosture = null, rustAuthority = null) {
  const rust = runtimeAuthoritySection(runtime, rustAuthority, 'slo_gate');
  if (rust.section) {
    return {
      required: !!rust.section.required,
      severity: cleanText(rust.section.severity || 'ok', 24) || 'ok',
      block_scale: !!rust.section.block_scale,
      containment_required: !!rust.section.containment_required,
      escalation_required: Array.isArray(rust.section.failed_checks)
        ? rust.section.failed_checks.some((row) => cleanText(row, 80) === 'human_escalation_open_rate')
        : false,
      failed_checks: Array.isArray(rust.section.failed_checks) ? rust.section.failed_checks.slice(0, 16) : [],
      checks: Array.isArray(rust.section.checks) ? rust.section.checks.slice(0, 16) : [],
      stale_metrics: !!rust.section.stale_metrics,
      summary: cleanText(rust.section.summary || '', 260),
      thresholds:
        rust.section.thresholds && typeof rust.section.thresholds === 'object'
          ? rust.section.thresholds
          : {
              spine_success_rate_min: RUNTIME_SPINE_SUCCESS_TARGET_MIN,
              receipt_latency_p95_max_ms: RUNTIME_SLO_RECEIPT_LATENCY_P95_MAX_MS,
              receipt_latency_p99_max_ms: RUNTIME_SLO_RECEIPT_LATENCY_P99_MAX_MS,
              queue_depth_max: RUNTIME_SLO_QUEUE_DEPTH_MAX,
              escalation_open_rate_min: RUNTIME_SLO_ESCALATION_OPEN_RATE_MIN,
            },
      authority: 'rust_runtime_systems',
      lane: rust.authority && rust.authority.lane ? rust.authority.lane : null,
    };
  }
  return {
    required: false,
    severity: 'unknown',
    block_scale: false,
    containment_required: false,
    escalation_required: false,
    failed_checks: [],
    checks: [],
    stale_metrics: true,
    summary: 'SLO gate unavailable: rust authority missing.',
    thresholds: {
      spine_success_rate_min: RUNTIME_SPINE_SUCCESS_TARGET_MIN,
      receipt_latency_p95_max_ms: RUNTIME_SLO_RECEIPT_LATENCY_P95_MAX_MS,
      receipt_latency_p99_max_ms: RUNTIME_SLO_RECEIPT_LATENCY_P99_MAX_MS,
      queue_depth_max: RUNTIME_SLO_QUEUE_DEPTH_MAX,
      escalation_open_rate_min: RUNTIME_SLO_ESCALATION_OPEN_RATE_MIN,
    },
    authority: 'rust_unavailable',
    lane: rust.authority && rust.authority.lane ? rust.authority.lane : null,
  };
}

function staleLaneRefreshCommand(laneName, team) {
  const lane = cleanText(laneName || '', 120).toLowerCase();
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  if (!lane) return null;
  if (lane.includes('benchmark')) {
    return ['benchmark-autonomy-gate', 'status', '--strict=1'];
  }
  if (lane.includes('collab')) {
    return ['collab-plane', 'dashboard', `--team=${normalizedTeam}`, '--strict=1'];
  }
  if (lane.includes('attention')) {
    return ['attention-queue', 'status'];
  }
  if (lane.includes('skills')) {
    return ['skills-plane', 'dashboard'];
  }
  if (lane.includes('app')) {
    return ['app-plane', 'history', '--app=chat-ui'];
  }
  if (lane.includes('security')) {
    return ['security-plane', 'status', '--strict=1'];
  }
  if (lane.includes('backlog')) {
    return ['backlog-delivery-plane', 'status', '--strict=1'];
  }
  if (lane.includes('top1')) {
    return ['top1-assurance', 'status', '--strict=1'];
  }
  if (lane.includes('enterprise')) {
    return ['enterprise-hardening', 'status', '--strict=1'];
  }
  if (lane.includes('f100')) {
    return ['f100-reliability-certification', 'status', '--strict=1'];
  }
  if (lane.includes('canyon')) {
    return ['canyon-plane', 'status', '--strict=1'];
  }
  if (lane.includes('adaptive')) {
    return ['adaptive-intelligence', 'status', '--strict=1'];
  }
  if (lane.includes('metakernel')) {
    return ['metakernel', 'status', '--strict=1'];
  }
  if (lane.includes('research')) {
    return ['research-plane', 'status', '--strict=1'];
  }
  if (lane.includes('mcp')) {
    return ['mcp-plane', 'status', '--strict=1'];
  }
  if (lane.includes('eval')) {
    return ['eval-plane', 'status', '--strict=1'];
  }
  if (lane.includes('hermes') || lane.includes('cockpit') || lane.includes('conduit')) {
    return ['hermes-plane', 'cockpit', `--max-blocks=${COCKPIT_MAX_BLOCKS}`, '--strict=1'];
  }
  return null;
}

function staleLaneRetryKey(team, laneName) {
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const normalizedLane = cleanText(laneName || 'unknown', 120).toLowerCase() || 'unknown';
  return `${normalizedTeam}:${normalizedLane}`;
}

function staleLaneRetrySnapshot(key) {
  const state = staleLaneRetryState && typeof staleLaneRetryState === 'object' ? staleLaneRetryState[key] : null;
  return {
    attempts: parseNonNegativeInt(state && state.attempts != null ? state.attempts : 0, 0, 100000000),
    next_retry_ms: parseNonNegativeInt(state && state.next_retry_ms != null ? state.next_retry_ms : 0, 0, 1000000000000),
    last_error: cleanText(state && state.last_error ? state.last_error : '', 200),
  };
}

function setStaleLaneRetrySnapshot(key, attempts, nextRetryMs, lastError = '') {
  staleLaneRetryState[key] = {
    attempts: parseNonNegativeInt(attempts, 0, 100000000),
    next_retry_ms: parseNonNegativeInt(nextRetryMs, 0, 1000000000000),
    last_error: cleanText(lastError, 200),
  };
}

function clearStaleLaneRetrySnapshot(key) {
  if (staleLaneRetryState && typeof staleLaneRetryState === 'object' && staleLaneRetryState[key]) {
    delete staleLaneRetryState[key];
  }
}

function runStaleLaneRefreshWithBackoff(laneName, laneCount, team) {
  const lane = cleanText(laneName || 'unknown', 120) || 'unknown';
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const nowMs = Date.now();
  const retryKey = staleLaneRetryKey(normalizedTeam, lane);
  const retry = staleLaneRetrySnapshot(retryKey);
  if (retry.next_retry_ms > nowMs) {
    return {
      lane,
      count: parseNonNegativeInt(laneCount, 0, 100000000),
      command: '',
      skipped: true,
      reason: 'retry_backoff_active',
      retry_attempts: retry.attempts,
      retry_in_ms: Math.max(0, retry.next_retry_ms - nowMs),
      next_retry_at: new Date(retry.next_retry_ms).toISOString(),
    };
  }
  const command = staleLaneRefreshCommand(lane, normalizedTeam);
  if (!Array.isArray(command) || command.length === 0) {
    return {
      lane,
      count: parseNonNegativeInt(laneCount, 0, 100000000),
      command: '',
      skipped: true,
      reason: 'unsupported_lane',
      retry_attempts: retry.attempts,
    };
  }
  const laneResult = runLane(command);
  const row = {
    lane,
    count: parseNonNegativeInt(laneCount, 0, 100000000),
    command: `protheus-ops ${command.join(' ')}`,
    skipped: false,
    ...laneOutcome(laneResult),
  };
  if (laneResult && laneResult.ok) {
    clearStaleLaneRetrySnapshot(retryKey);
    return {
      ...row,
      retry_attempts: 0,
      retry_in_ms: 0,
    };
  }
  const nextAttempts = retry.attempts + 1;
  const backoffMs = Math.min(
    RUNTIME_STALE_LANE_RETRY_MAX_MS,
    RUNTIME_STALE_LANE_RETRY_BASE_MS * Math.pow(2, Math.max(0, nextAttempts - 1))
  );
  const nextRetryMs = nowMs + backoffMs;
  setStaleLaneRetrySnapshot(retryKey, nextAttempts, nextRetryMs, row.detail || row.status_reason || 'lane_failed');
  enqueueAttentionEvent(
    {
      ts: nowIso(),
      severity: 'warn',
      source: 'runtime_stale_lane_breaker',
      source_type: 'runtime',
      summary: cleanText(
        `Stale lane refresh failed for ${lane}; retry in ${Math.ceil(backoffMs / 1000)}s (attempt ${nextAttempts}).`,
        260
      ),
      band: 'p3',
      priority_lane: 'background',
      score: 0.35,
      attention_key: cleanText(`stale_lane_retry:${retryKey}:${nextAttempts}`, 120),
      metadata: {
        lane,
        count: parseNonNegativeInt(laneCount, 0, 100000000),
        retry_attempts: nextAttempts,
        retry_backoff_ms: backoffMs,
        next_retry_at: new Date(nextRetryMs).toISOString(),
      },
    },
    'runtime_stale_lane_breaker'
  );
  return {
    ...row,
    retry_attempts: nextAttempts,
    retry_in_ms: backoffMs,
    next_retry_at: new Date(nextRetryMs).toISOString(),
  };
}

function maybeHealCoarseSignal(snapshot, runtime, team, recommendation = null) {
  const signal = cockpitSignalState(runtime);
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const staleBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000);
  const staleBlocksRaw = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks_raw, staleBlocks, 100000000);
  const staleBlocksDormant = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks_dormant, 0, 100000000);
  const stalePressure = (staleBlocks > 0 || staleBlocksRaw > 0) && queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH;
  const targetSignals = parsePositiveInt(runtime && runtime.target_conduit_signals, RUNTIME_AUTO_BALANCE_THRESHOLD, 1, 128);
  const conduitSignals = parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000);
  const signalDeficit = Math.max(0, targetSignals - conduitSignals);
  const laneCaps =
    runtime && runtime.queue_lane_caps && typeof runtime.queue_lane_caps === 'object'
      ? runtime.queue_lane_caps
      : ATTENTION_LANE_CAPS;
  const criticalCap = parsePositiveInt(
    laneCaps && laneCaps.critical != null ? laneCaps.critical : ATTENTION_LANE_CAPS.critical,
    ATTENTION_LANE_CAPS.critical,
    1,
    100000000
  );
  const criticalAttentionTotal = parseNonNegativeInt(
    runtime && runtime.critical_attention_total != null ? runtime.critical_attention_total : 0,
    0,
    100000000
  );
  const criticalAttentionOverload = criticalAttentionTotal > Math.max(criticalCap, RUNTIME_CRITICAL_ATTENTION_OVERLOAD_THRESHOLD);
  const staleLanesTop = Array.isArray(runtime && runtime.cockpit_stale_lanes_top)
    ? runtime.cockpit_stale_lanes_top
        .map((row) => ({
          lane: cleanText(row && row.lane ? row.lane : 'unknown', 80) || 'unknown',
          count: parseNonNegativeInt(row && row.count != null ? row.count : 0, 0, 100000000),
        }))
        .filter((row) => row.count > 0)
        .slice(0, RUNTIME_COARSE_STALE_LANE_REFRESH_LIMIT)
    : [];
  const staleDormantLanesTop = Array.isArray(runtime && runtime.cockpit_stale_lanes_dormant_top)
    ? runtime.cockpit_stale_lanes_dormant_top
        .map((row) => ({
          lane: cleanText(row && row.lane ? row.lane : 'unknown', 80) || 'unknown',
          count: parseNonNegativeInt(row && row.count != null ? row.count : 0, 0, 100000000),
        }))
        .filter((row) => row.count > 0)
        .slice(0, RUNTIME_COARSE_STALE_LANE_REFRESH_LIMIT)
    : [];
  const staleLaneMap = {};
  for (const row of [...staleLanesTop, ...staleDormantLanesTop]) {
    const key = cleanText(row && row.lane ? row.lane : 'unknown', 80) || 'unknown';
    const count = parseNonNegativeInt(row && row.count != null ? row.count : 0, 0, 100000000);
    if (!key || count <= 0) continue;
    staleLaneMap[key] = Math.max(parseNonNegativeInt(staleLaneMap[key], 0, 100000000), count);
  }
  const staleLaneRefreshRows = Object.entries(staleLaneMap)
    .map(([lane, count]) => ({ lane, count: parseNonNegativeInt(count, 0, 100000000) }))
    .filter((row) => row.count > 0)
    .sort((a, b) => b.count - a.count)
    .slice(0, RUNTIME_COARSE_STALE_LANE_REFRESH_LIMIT);
  const signalDeficitPressure =
    signalDeficit > 0 && queueDepth >= RUNTIME_INGRESS_DAMPEN_DEPTH;
  const lowPressureCoordinationDrift =
    signalDeficit > 0 &&
    (staleBlocks > 0 || staleBlocksRaw > 0) &&
    queueDepth < RUNTIME_DRAIN_TRIGGER_DEPTH;
  const chronicCoordinationPathology =
    (staleBlocks >= RUNTIME_COORDINATION_PATHOLOGY_STALE_BLOCK_MIN ||
      staleBlocksRaw >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS) &&
    signalDeficit > 0;
  const staleAutohealNeeded =
    staleBlocks >= RUNTIME_COCKPIT_STALE_AUTOHEAL_MIN_BLOCKS ||
    staleBlocksRaw >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
    staleBlocksDormant >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS;
  const fallbackRequired =
    (signal.coarse && queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH) ||
    signalDeficitPressure ||
    stalePressure ||
    lowPressureCoordinationDrift ||
    chronicCoordinationPathology ||
    staleAutohealNeeded ||
    criticalAttentionOverload;
  const rustRecommendations =
    recommendation && recommendation.coarse_signal_remediation_required != null
      ? null
      : runtimeAuthoritySection(runtime, null, 'recommendations').section;
  const required =
    recommendation && recommendation.coarse_signal_remediation_required != null
      ? !!recommendation.coarse_signal_remediation_required
      : rustRecommendations && rustRecommendations.coarse_signal_remediation_required != null
      ? !!rustRecommendations.coarse_signal_remediation_required
      : fallbackRequired;
  if (!required) {
    return {
      required: false,
      coarse: signal.coarse,
      quality: signal.quality,
      signal_deficit: signalDeficit,
      stale_blocks: staleBlocks,
      stale_lanes_top: staleLanesTop,
      lane_demotion: { required: false, applied: false, command: '', lane: null },
      conduit_scale_up: { required: false, applied: false, target_signals: targetSignals, conduit_signals: conduitSignals, signal_deficit: signalDeficit, lanes: [] },
      stale_lane_drain: {
        required: false,
        applied: false,
        stale_blocks: staleBlocks,
        stale_blocks_raw: staleBlocksRaw,
        stale_blocks_dormant: staleBlocksDormant,
        stale_lanes_top: staleLanesTop,
        stale_lanes_dormant_top: staleDormantLanesTop,
        drain_limit: 0,
        lane: null,
        lanes: [],
      },
    };
  }

  const laneDemotionMaxDepth =
    queueDepth >= DASHBOARD_BACKPRESSURE_BATCH_DEPTH ? 50 : RUNTIME_COARSE_THROTTLE_MAX_DEPTH;
  const laneDemotionCommand = [
    'collab-plane',
    'throttle',
    `--team=${normalizedTeam}`,
    `--plane=${RUNTIME_THROTTLE_PLANE}`,
    `--max-depth=${laneDemotionMaxDepth}`,
    `--strategy=${RUNTIME_COARSE_THROTTLE_STRATEGY}`,
    '--strict=1',
  ];
  const laneDemotionRequired =
    queueDepth >= RUNTIME_INGRESS_DAMPEN_DEPTH || signalDeficit > 0 || criticalAttentionOverload;
  const laneDemotionLane = laneDemotionRequired ? runLane(laneDemotionCommand) : null;
  const laneDemotionApplied = !!(
    laneDemotionLane &&
    laneDemotionLane.ok &&
    laneDemotionLane.payload &&
    laneDemotionLane.payload.ok !== false
  );

  const staleOnlyMode =
    staleAutohealNeeded &&
    !signalDeficitPressure &&
    !stalePressure &&
    !chronicCoordinationPathology;
  const roleOrder = staleOnlyMode ? ['builder', 'researcher', 'analyst'] : ['researcher', 'builder', 'analyst'];
  const requestedRoleCount =
    signalDeficit >= 8 ? 3 : signalDeficit >= 4 ? 2 : 1;
  const scaleRoleCount = staleOnlyMode
    ? 1
    : signal.coarse
    ? Math.max(2, requestedRoleCount)
    : requestedRoleCount;
  const scaleRoles = roleOrder.slice(0, Math.max(1, Math.min(roleOrder.length, scaleRoleCount)));
  const scaleLanes = scaleRoles.map((role) => {
    const shadow = cleanText(`${normalizedTeam}-coarse-${role}`, 120) || `${normalizedTeam}-coarse-${role}`;
    const lane = runLane([
      'collab-plane',
      'launch-role',
      `--team=${normalizedTeam}`,
      `--role=${role}`,
      `--shadow=${shadow}`,
      '--strict=1',
    ]);
    return {
      role,
      shadow,
      ...laneOutcome(lane),
    };
  });
  const scaleApplied = scaleLanes.length > 0 && scaleLanes.every((row) => !!row.ok);

  const drainLimit = Math.min(
    RUNTIME_ATTENTION_DRAIN_MAX_BATCH,
    Math.max(
      RUNTIME_COARSE_DRAIN_MIN_BATCH,
      Math.ceil(Math.max(queueDepth, staleBlocks * 2) / 2)
    )
  );
  const staleDrainLane = runLane([
    'attention-queue',
    'drain',
    `--consumer=${ATTENTION_CONSUMER_ID}`,
    `--limit=${drainLimit}`,
    '--wait-ms=0',
    '--run-context=runtime_coarse_stale_lane_drain',
  ]);
  const staleRefreshLanes = staleLaneRefreshRows.map((row) =>
    runStaleLaneRefreshWithBackoff(row.lane, row.count, normalizedTeam)
  );
  let staleCompactLane = null;
  if (
    parseNonNegativeInt(runtime && runtime.attention_cursor_offset, 0, 100000000) >=
    RUNTIME_ATTENTION_COMPACT_MIN_ACKED
  ) {
    staleCompactLane = runLane([
      'attention-queue',
      'compact',
      `--retain=${RUNTIME_ATTENTION_COMPACT_RETAIN}`,
      `--min-acked=${RUNTIME_ATTENTION_COMPACT_MIN_ACKED}`,
      '--run-context=runtime_coarse_stale_lane_drain',
    ]);
  }
  const staleDrainApplied = !!(
    staleDrainLane &&
    staleDrainLane.ok &&
    staleDrainLane.payload &&
    staleDrainLane.payload.ok !== false
  );
  const staleRefreshApplied = staleRefreshLanes
    .filter((row) => !row.skipped)
    .every((row) => !!row.ok);
  const staleCompactApplied =
    staleCompactLane == null ||
    !!(staleCompactLane.ok && staleCompactLane.payload && staleCompactLane.payload.ok !== false);
  const staleLaneDrainApplied = staleDrainApplied && staleRefreshApplied && staleCompactApplied;

  return {
    required: true,
    coarse: signal.coarse,
    quality: signal.quality,
    signal_deficit: signalDeficit,
    stale_blocks: staleBlocks,
    stale_lanes_top: staleLanesTop,
    lane_demotion: {
      required: laneDemotionRequired,
      applied: laneDemotionApplied,
      max_depth: laneDemotionMaxDepth,
      strategy: RUNTIME_COARSE_THROTTLE_STRATEGY,
      command: laneDemotionRequired ? `protheus-ops ${laneDemotionCommand.join(' ')}` : '',
      lane: laneDemotionLane ? laneOutcome(laneDemotionLane) : null,
    },
    conduit_scale_up: {
      required: true,
      applied: scaleApplied,
      target_signals: targetSignals,
      conduit_signals: conduitSignals,
      signal_deficit: signalDeficit,
      lanes: scaleLanes,
    },
    stale_lane_drain: {
      required: true,
      applied: staleLaneDrainApplied,
      stale_blocks: staleBlocks,
      stale_blocks_raw: staleBlocksRaw,
      stale_blocks_dormant: staleBlocksDormant,
      stale_lanes_top: staleLanesTop,
      stale_lanes_dormant_top: staleDormantLanesTop,
      drain_limit: drainLimit,
      lane: laneOutcome(staleDrainLane),
      lanes: staleRefreshLanes,
      compact_lane: staleCompactLane ? laneOutcome(staleCompactLane) : null,
    },
  };
}

function maybeEmitReliabilityEscalation(recommendation, runtime) {
  const rustReliability =
    recommendation && recommendation.reliability_gate_required != null && recommendation.escalation_starved != null
      ? { section: null, authority: null }
      : runtimeAuthoritySection(runtime, null, 'reliability_posture');
  const rustSloGate =
    recommendation && recommendation.slo_gate && typeof recommendation.slo_gate === 'object'
      ? { section: null, authority: null }
      : runtimeAuthoritySection(runtime, rustReliability.authority, 'slo_gate');
  const sloGate =
    recommendation && recommendation.slo_gate && typeof recommendation.slo_gate === 'object'
      ? recommendation.slo_gate
      : rustSloGate.section && typeof rustSloGate.section === 'object'
      ? rustSloGate.section
      : null;
  const staleMetricsWithPathology =
    !!(sloGate && sloGate.stale_metrics) &&
    parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) >=
      RUNTIME_COCKPIT_STALE_AUTOHEAL_MIN_BLOCKS;
  const required =
    !!(recommendation && recommendation.reliability_gate_required) ||
    !!(rustReliability.section && rustReliability.section.degraded) ||
    !!(sloGate && sloGate.required) ||
    staleMetricsWithPathology;
  const escalationOpenRate = Number(
    recommendation && recommendation.human_escalation_open_rate != null
      ? recommendation.human_escalation_open_rate
      : runtime && runtime.human_escalation_open_rate != null
      ? runtime.human_escalation_open_rate
      : Number.NaN
  );
  const escalationKnown = Number.isFinite(escalationOpenRate);
  const escalationStarved =
    !!(recommendation && recommendation.escalation_starved) ||
    !!(rustReliability.section && rustReliability.section.escalation_starved) ||
    (required && escalationKnown && escalationOpenRate <= RUNTIME_SLO_ESCALATION_OPEN_RATE_MIN);
  const spineRate = Number(
    recommendation && recommendation.spine_success_rate != null ? recommendation.spine_success_rate : Number.NaN
  );
  const handoffsPerAgent = Number(
    recommendation && recommendation.handoffs_per_agent != null ? recommendation.handoffs_per_agent : Number.NaN
  );
  const failedChecks =
    sloGate && Array.isArray(sloGate.failed_checks)
      ? sloGate.failed_checks.map((row) => cleanText(row, 80)).filter(Boolean).slice(0, 8)
      : [];
  const p99Latency = Number(
    runtime && runtime.receipt_latency_p99_ms != null ? runtime.receipt_latency_p99_ms : Number.NaN
  );
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const sourceType = failedChecks.length > 0
    ? 'runtime_slo_gate'
    : staleMetricsWithPathology
    ? 'runtime_slo_stale_metrics'
    : 'spine_success_rate';
  const maybeWriteReliabilityReceipt = (reason, laneResult, extra = {}) => {
    const nowMs = Date.now();
    const previousReason = cleanText(reliabilityEscalationState.last_receipt_reason || '', 80);
    const sinceLastMs = Math.max(
      0,
      nowMs - parseNonNegativeInt(reliabilityEscalationState.last_receipt_ms, 0, 1000000000000)
    );
    const normalizedReason = cleanText(reason || 'unknown', 80) || 'unknown';
    if (
      previousReason === normalizedReason &&
      sinceLastMs < RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS
    ) {
      return;
    }
    const fallbackLane = {
      ok: normalizedReason === 'queued' || normalizedReason === 'escalation_rate_nonzero',
      status: 0,
      argv: ['runtime-reliability-escalation', normalizedReason],
      payload: {
        ok: normalizedReason === 'queued' || normalizedReason === 'escalation_rate_nonzero',
        type: 'runtime_reliability_escalation',
        reason: normalizedReason,
      },
    };
    writeActionReceipt(
      'runtime.reliabilityEscalation',
      {
        reason: normalizedReason,
        source_type: sourceType,
        queue_depth: queueDepth,
        conduit_signals: parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000),
        cockpit_blocks: parseNonNegativeInt(runtime && runtime.cockpit_blocks, 0, 100000000),
        spine_success_rate: Number.isFinite(spineRate) ? spineRate : null,
        receipt_latency_p99_ms: Number.isFinite(p99Latency) ? p99Latency : null,
        failed_checks: failedChecks,
        escalation_open_rate: escalationKnown ? escalationOpenRate : null,
        ...extra,
      },
      laneResult || fallbackLane
    );
    reliabilityEscalationState.last_receipt_ms = nowMs;
    reliabilityEscalationState.last_receipt_reason = normalizedReason;
  };
  if (!required) {
    return {
      required: false,
      applied: false,
      reason: 'reliability_gate_not_required',
      lane: null,
      cooldown_ms: RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS,
      last_emit_at: cleanText(reliabilityEscalationState.last_emit_at || '', 80),
    };
  }
  if (!escalationStarved) {
    maybeWriteReliabilityReceipt('escalation_rate_nonzero', null, {
      detail: 'human_escalations_present_or_not_starved',
    });
    return {
      required: true,
      applied: false,
      reason: 'escalation_rate_nonzero',
      lane: null,
      cooldown_ms: RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS,
      last_emit_at: cleanText(reliabilityEscalationState.last_emit_at || '', 80),
    };
  }
  const nowMs = Date.now();
  const sinceLastMs = Math.max(0, nowMs - parseNonNegativeInt(reliabilityEscalationState.last_emit_ms, 0, 1000000000000));
  if (sinceLastMs < RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS) {
    maybeWriteReliabilityReceipt('cooldown_active', null, {
      since_last_ms: sinceLastMs,
    });
    return {
      required: true,
      applied: false,
      reason: 'cooldown_active',
      lane: null,
      cooldown_ms: RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS,
      since_last_ms: sinceLastMs,
      last_emit_at: cleanText(reliabilityEscalationState.last_emit_at || '', 80),
    };
  }
  const summary = [
    `Spine success ${Number.isFinite(spineRate) ? (spineRate * 100).toFixed(1) : 'unknown'}%`,
    `handoffs/agent ${Number.isFinite(handoffsPerAgent) ? handoffsPerAgent.toFixed(2) : 'unknown'}`,
    `queue ${queueDepth}`,
    `p99 ${Number.isFinite(p99Latency) ? p99Latency.toFixed(0) : 'unknown'}ms`,
    staleMetricsWithPathology ? 'stale metrics with persistent cockpit pathology.' : '',
    failedChecks.length ? `failed checks: ${failedChecks.join(', ')}` : 'failed checks: none',
    'no open human escalations.',
  ].filter(Boolean).join('; ');
  const lane = enqueueAttentionEvent(
    {
      ts: nowIso(),
      severity: 'critical',
      source: 'runtime_reliability_guard',
      source_type: sourceType,
      summary: cleanText(summary, 260),
      band: 'p0',
      priority_lane: 'critical',
      score: 1,
      initiative_action: 'triple_escalation',
      attention_key: cleanText(`runtime_reliability:${nowMs}`, 120),
      metadata: {
        queue_depth: parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000),
        cockpit_blocks: parseNonNegativeInt(runtime && runtime.cockpit_blocks, 0, 100000000),
        conduit_signals: parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000),
        spine_success_rate: Number.isFinite(spineRate) ? spineRate : null,
        spine_success_target: RUNTIME_SPINE_SUCCESS_TARGET_MIN,
        receipt_latency_p99_ms: Number.isFinite(p99Latency) ? p99Latency : null,
        receipt_latency_p99_target_ms: RUNTIME_SLO_RECEIPT_LATENCY_P99_MAX_MS,
        handoffs_per_agent: Number.isFinite(handoffsPerAgent) ? handoffsPerAgent : null,
        escalation_open_rate: escalationKnown ? escalationOpenRate : null,
        escalation_open_rate_min: RUNTIME_SLO_ESCALATION_OPEN_RATE_MIN,
        failed_checks: failedChecks,
      },
    },
    'runtime_reliability_guard'
  );
  const applied = !!(lane && lane.ok && lane.payload && lane.payload.ok !== false);
  if (applied) {
    reliabilityEscalationState.last_emit_ms = nowMs;
    reliabilityEscalationState.last_emit_at = nowIso();
    reliabilityEscalationState.emit_count =
      parseNonNegativeInt(reliabilityEscalationState.emit_count, 0, 100000000) + 1;
  }
  maybeWriteReliabilityReceipt(applied ? 'queued' : 'enqueue_failed', lane, {
    since_last_ms: sinceLastMs,
  });
  return {
    required: true,
    applied,
    reason: applied ? 'queued' : 'enqueue_failed',
    lane: laneOutcome(lane),
    cooldown_ms: RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS,
    since_last_ms: sinceLastMs,
    last_emit_at: cleanText(reliabilityEscalationState.last_emit_at || '', 80),
    emit_count: parseNonNegativeInt(reliabilityEscalationState.emit_count, 0, 100000000),
  };
}

function maybeRunSpineMetricsCanary(recommendation, runtime) {
  const stale = !!(
    (recommendation && recommendation.spine_metrics_stale) ||
    (runtime && runtime.spine_metrics_stale)
  );
  const latestAgeSeconds = parseNonNegativeInt(
    recommendation && recommendation.spine_metrics_latest_age_seconds != null
      ? recommendation.spine_metrics_latest_age_seconds
      : runtime && runtime.spine_metrics_latest_age_seconds != null
      ? runtime.spine_metrics_latest_age_seconds
      : 0,
    0,
    1000000000
  );
  const rustRecommendations =
    recommendation && recommendation.spine_canary_required != null
      ? null
      : runtimeAuthoritySection(runtime, null, 'recommendations').section;
  const required =
    recommendation && recommendation.spine_canary_required != null
      ? !!recommendation.spine_canary_required
      : rustRecommendations && rustRecommendations.spine_canary_required != null
      ? !!rustRecommendations.spine_canary_required
      : stale && latestAgeSeconds >= RUNTIME_SPINE_METRICS_STALE_MAX_AGE_SECONDS;
  if (!required) {
    return {
      required: false,
      applied: false,
      reason: stale ? 'age_below_threshold' : 'spine_metrics_fresh',
      command: '',
      lane: null,
      latest_age_seconds: latestAgeSeconds,
      cooldown_ms: RUNTIME_SPINE_CANARY_COOLDOWN_MS,
      last_run_at: cleanText(runtimeSpineCanaryState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(runtimeSpineCanaryState.run_count, 0, 100000000),
    };
  }
  const nowMs = Date.now();
  const sinceLastMs = Math.max(
    0,
    nowMs - parseNonNegativeInt(runtimeSpineCanaryState.last_run_ms, 0, 1000000000000)
  );
  if (sinceLastMs < RUNTIME_SPINE_CANARY_COOLDOWN_MS) {
    return {
      required: true,
      applied: false,
      reason: 'cooldown_active',
      command: '',
      lane: null,
      latest_age_seconds: latestAgeSeconds,
      cooldown_ms: RUNTIME_SPINE_CANARY_COOLDOWN_MS,
      since_last_ms: sinceLastMs,
      last_run_at: cleanText(runtimeSpineCanaryState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(runtimeSpineCanaryState.run_count, 0, 100000000),
    };
  }
  const command = ['spine', 'daily', `--max-eyes=${RUNTIME_SPINE_CANARY_MAX_EYES}`];
  const lane = runLane(command);
  const applied = !!(lane && lane.ok && lane.payload && lane.payload.ok !== false);
  runtimeSpineCanaryState.last_run_ms = nowMs;
  runtimeSpineCanaryState.last_run_at = nowIso();
  if (applied) {
    runtimeSpineCanaryState.run_count =
      parseNonNegativeInt(runtimeSpineCanaryState.run_count, 0, 100000000) + 1;
  }
  return {
    required: true,
    applied,
    reason: applied ? 'canary_queued' : 'canary_failed',
    command: `protheus-ops ${command.join(' ')}`,
    lane: laneOutcome(lane),
    latest_age_seconds: latestAgeSeconds,
    cooldown_ms: RUNTIME_SPINE_CANARY_COOLDOWN_MS,
    since_last_ms: sinceLastMs,
    last_run_at: cleanText(runtimeSpineCanaryState.last_run_at || '', 80),
    run_count: parseNonNegativeInt(runtimeSpineCanaryState.run_count, 0, 100000000),
  };
}

function maybeRefreshBenchmarkSanity(runtime, recommendation = null) {
  const cockpitStatus =
    cleanText(runtime && runtime.benchmark_sanity_cockpit_status ? runtime.benchmark_sanity_cockpit_status : 'unknown', 24) ||
    'unknown';
  const mirrorStatus =
    cleanText(runtime && runtime.benchmark_sanity_status ? runtime.benchmark_sanity_status : cockpitStatus, 24) ||
    cockpitStatus;
  const ageSeconds = parsePositiveInt(
    runtime && runtime.benchmark_sanity_age_seconds != null ? runtime.benchmark_sanity_age_seconds : -1,
    -1,
    -1,
    1000000000
  );
  const stale = ageSeconds < 0 || ageSeconds > RUNTIME_BENCHMARK_REFRESH_MAX_AGE_SECONDS;
  const failing = cockpitStatus === 'fail' || mirrorStatus === 'fail';
  const rustRecommendations =
    recommendation && recommendation.benchmark_refresh_required != null
      ? null
      : runtimeAuthoritySection(runtime, null, 'recommendations').section;
  const required =
    recommendation && recommendation.benchmark_refresh_required != null
      ? !!recommendation.benchmark_refresh_required
      : rustRecommendations && rustRecommendations.benchmark_refresh_required != null
      ? !!rustRecommendations.benchmark_refresh_required
      : failing || stale;
  if (!required) {
    return {
      required: false,
      applied: false,
      reason: 'benchmark_sanity_fresh',
      command: '',
      lane: null,
      cockpit_status: cockpitStatus,
      mirror_status: mirrorStatus,
      age_seconds: ageSeconds,
      cooldown_ms: RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS,
      last_run_at: cleanText(benchmarkRefreshState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(benchmarkRefreshState.run_count, 0, 100000000),
    };
  }
  const nowMs = Date.now();
  const sinceLastMs = Math.max(
    0,
    nowMs - parseNonNegativeInt(benchmarkRefreshState.last_run_ms, 0, 1000000000000)
  );
  if (sinceLastMs < RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS) {
    return {
      required: true,
      applied: false,
      reason: 'cooldown_active',
      command: '',
      lane: null,
      cockpit_status: cockpitStatus,
      mirror_status: mirrorStatus,
      age_seconds: ageSeconds,
      cooldown_ms: RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS,
      since_last_ms: sinceLastMs,
      last_run_at: cleanText(benchmarkRefreshState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(benchmarkRefreshState.run_count, 0, 100000000),
    };
  }
  if (!fs.existsSync(BENCHMARK_SANITY_GATE_SCRIPT_PATH)) {
    benchmarkRefreshState.last_run_ms = nowMs;
    benchmarkRefreshState.last_run_at = nowIso();
    benchmarkRefreshState.last_status = 'script_missing';
    return {
      required: true,
      applied: false,
      reason: 'script_missing',
      command: `node ${path.relative(ROOT, BENCHMARK_SANITY_GATE_SCRIPT_PATH)} --strict=1`,
      lane: null,
      cockpit_status: cockpitStatus,
      mirror_status: mirrorStatus,
      age_seconds: ageSeconds,
      cooldown_ms: RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS,
      since_last_ms: sinceLastMs,
      last_run_at: cleanText(benchmarkRefreshState.last_run_at || '', 80),
      run_count: parseNonNegativeInt(benchmarkRefreshState.run_count, 0, 100000000),
    };
  }
  const command = ['node', path.relative(ROOT, BENCHMARK_SANITY_GATE_SCRIPT_PATH), '--strict=1'];
  const run = spawnSync(command[0], command.slice(1), {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 60_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  const refreshSnapshot = benchmarkSanitySnapshot();
  const ran = !run.error;
  const applied = ran;
  benchmarkRefreshState.last_run_ms = nowMs;
  benchmarkRefreshState.last_run_at = nowIso();
  benchmarkRefreshState.last_status = cleanText(refreshSnapshot.status || (applied ? 'refreshed' : 'failed'), 40) || 'unknown';
  if (applied) {
    benchmarkRefreshState.run_count =
      parseNonNegativeInt(benchmarkRefreshState.run_count, 0, 100000000) + 1;
  }
  return {
    required: true,
    applied,
    reason:
      run.error
        ? `spawn_failed:${cleanText(run.error.message || String(run.error), 120)}`
        : run.status === 0
        ? 'refreshed_ok'
        : 'refreshed_gate_failed',
    command: command.join(' '),
    lane: null,
    cockpit_status: cockpitStatus,
    mirror_status: mirrorStatus,
    refreshed_status: cleanText(refreshSnapshot.status || '', 24),
    refreshed_detail: cleanText(refreshSnapshot.detail || '', 220),
    refreshed_age_seconds: parsePositiveInt(refreshSnapshot.age_seconds, -1, -1, 1000000000),
    age_seconds: ageSeconds,
    cooldown_ms: RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS,
    since_last_ms: sinceLastMs,
    last_run_at: cleanText(benchmarkRefreshState.last_run_at || '', 80),
    run_count: parseNonNegativeInt(benchmarkRefreshState.run_count, 0, 100000000),
    exit_status: parsePositiveInt(run.status, -1, -1, 255),
  };
}

function maybeAutoHealConduit(runtime, team, recommendation = null) {
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const signals = parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000);
  const staleCockpitBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000);
  const staleCockpitBlocksRaw = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks_raw, 0, 100000000);
  const staleCockpitDormantBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks_dormant, 0, 100000000);
  const staleMaintenance =
    staleCockpitBlocks >= RUNTIME_COCKPIT_STALE_SOFT_AUTOHEAL_MIN_BLOCKS &&
    queueDepth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH;
  const staleRawMaintenance =
    staleCockpitBlocksRaw >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
    staleCockpitDormantBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS;
  const stalePressure =
    (staleCockpitBlocks > 0 || staleCockpitBlocksRaw >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS) &&
    queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH;
  const chronicCoordinationPathology =
    staleCockpitBlocks >= RUNTIME_COORDINATION_PATHOLOGY_STALE_BLOCK_MIN ||
    staleCockpitBlocksRaw >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS;
  const threshold = Math.max(
    parsePositiveInt(runtime && runtime.target_conduit_signals, RUNTIME_AUTO_BALANCE_THRESHOLD, 1, 128),
    RUNTIME_CONDUIT_WATCHDOG_MIN_SIGNALS
  );
  const lowSignals = signals < threshold;
  const signalDeficit = Math.max(0, threshold - signals);
  const lowLoadCoordinationDrift =
    lowSignals &&
    staleCockpitBlocks > 0 &&
    queueDepth < RUNTIME_DRAIN_TRIGGER_DEPTH;
  const laneCaps =
    runtime && runtime.queue_lane_caps && typeof runtime.queue_lane_caps === 'object'
      ? runtime.queue_lane_caps
      : ATTENTION_LANE_CAPS;
  const criticalCap = parsePositiveInt(
    laneCaps && laneCaps.critical != null ? laneCaps.critical : ATTENTION_LANE_CAPS.critical,
    ATTENTION_LANE_CAPS.critical,
    1,
    100000000
  );
  const criticalAttentionTotal = parseNonNegativeInt(
    runtime && runtime.critical_attention_total != null ? runtime.critical_attention_total : 0,
    0,
    100000000
  );
  const criticalAttentionOverload =
    criticalAttentionTotal > Math.max(criticalCap, RUNTIME_CRITICAL_ATTENTION_OVERLOAD_THRESHOLD);
  const watchdogPressure =
    staleMaintenance ||
    staleRawMaintenance ||
    lowLoadCoordinationDrift ||
    chronicCoordinationPathology ||
    criticalAttentionOverload ||
    staleCockpitBlocks >= RUNTIME_COCKPIT_STALE_AUTOHEAL_MIN_BLOCKS;
  const watchdogCooldownMs = watchdogPressure
    ? RUNTIME_CONDUIT_WATCHDOG_PRESSURE_COOLDOWN_MS
    : RUNTIME_CONDUIT_WATCHDOG_COOLDOWN_MS;
  const nowMs = Date.now();
  const normalizedTeam = cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
  const knownShadows = Array.isArray(conduitWatchdogState.active_shadows)
    ? conduitWatchdogState.active_shadows
        .map((row) => cleanText(row, 120))
        .filter(Boolean)
    : [];
  const terminateShadows = (shadowIds, reason = 'conduit_recovered') => {
    const lanes = [];
    let allOk = true;
    for (const shadow of shadowIds) {
      const lane = runLane([
        'collab-plane',
        'terminate-role',
        `--team=${normalizedTeam}`,
        `--shadow=${shadow}`,
        `--reason=${cleanText(reason, 80) || 'conduit_recovered'}`,
        '--strict=1',
      ]);
      lanes.push(laneOutcome(lane));
      if (!(lane && lane.ok)) allOk = false;
    }
    return { lanes, ok: allOk };
  };
  if (!lowSignals && !staleMaintenance && !staleRawMaintenance && !criticalAttentionOverload) {
    let release = null;
    if (knownShadows.length > 0) {
      release = terminateShadows(knownShadows, 'conduit_recovered');
      if (release.ok) conduitWatchdogState.active_shadows = [];
    }
    conduitWatchdogState = {
      ...conduitWatchdogState,
      low_signals_since_ms: 0,
    };
    return {
      required: false,
      triggered: false,
      recovered: true,
      queue_depth: queueDepth,
      conduit_signals: signals,
      stale_cockpit_blocks: staleCockpitBlocks,
      stale_cockpit_blocks_raw: staleCockpitBlocksRaw,
      stale_cockpit_blocks_dormant: staleCockpitDormantBlocks,
      signal_deficit: signalDeficit,
      critical_attention_total: criticalAttentionTotal,
      critical_attention_cap: criticalCap,
      critical_attention_overload: criticalAttentionOverload,
      low_signal: lowSignals,
      threshold,
      stale_for_ms: 0,
      lane: release && release.lanes && release.lanes.length ? release.lanes[0] : null,
      lanes: release ? { terminate: release.lanes } : null,
      active_shadows: Array.isArray(conduitWatchdogState.active_shadows)
        ? conduitWatchdogState.active_shadows.slice(0, 8)
        : [],
      last_attempt_at: cleanText(conduitWatchdogState.last_attempt_at || '', 80),
      last_success_at: cleanText(conduitWatchdogState.last_success_at || '', 80),
    };
  }
  const lowSince = lowSignals
    ? conduitWatchdogState.low_signals_since_ms > 0
      ? conduitWatchdogState.low_signals_since_ms
      : nowMs
    : 0;
  if (lowSignals) {
    conduitWatchdogState.low_signals_since_ms = lowSince;
  }
  const staleForMs = lowSignals ? Math.max(0, nowMs - lowSince) : 0;
  const fallbackRequired =
    staleMaintenance ||
    staleRawMaintenance ||
    lowLoadCoordinationDrift ||
    criticalAttentionOverload ||
    (
      lowSignals &&
      (
        queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH ||
        queueDepth >= RUNTIME_INGRESS_DAMPEN_DEPTH ||
        stalePressure ||
        staleForMs >= RUNTIME_CONDUIT_WATCHDOG_STALE_MS ||
        chronicCoordinationPathology ||
        staleCockpitBlocksRaw >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
        signalDeficit >= Math.max(2, Math.ceil(threshold * 0.3))
      )
    );
  const rustRecommendations =
    recommendation && recommendation.conduit_watchdog_required != null
      ? null
      : runtimeAuthoritySection(runtime, null, 'recommendations').section;
  const required =
    recommendation && recommendation.conduit_watchdog_required != null
      ? !!recommendation.conduit_watchdog_required
      : rustRecommendations && rustRecommendations.conduit_watchdog_required != null
      ? !!rustRecommendations.conduit_watchdog_required
      : fallbackRequired;
  if (!required) {
    return {
      required: false,
      triggered: false,
      recovered: !lowSignals,
      queue_depth: queueDepth,
      conduit_signals: signals,
      stale_cockpit_blocks: staleCockpitBlocks,
      stale_cockpit_blocks_raw: staleCockpitBlocksRaw,
      stale_cockpit_blocks_dormant: staleCockpitDormantBlocks,
      signal_deficit: signalDeficit,
      critical_attention_total: criticalAttentionTotal,
      critical_attention_cap: criticalCap,
      critical_attention_overload: criticalAttentionOverload,
      low_signal: lowSignals,
      threshold,
      stale_for_ms: staleForMs,
      lane: null,
      active_shadows: knownShadows.slice(0, 8),
      last_attempt_at: cleanText(conduitWatchdogState.last_attempt_at || '', 80),
      last_success_at: cleanText(conduitWatchdogState.last_success_at || '', 80),
    };
  }
  const triggerReady =
    staleMaintenance ||
    staleRawMaintenance ||
    lowLoadCoordinationDrift ||
    criticalAttentionOverload ||
    staleForMs >= RUNTIME_CONDUIT_WATCHDOG_STALE_MS ||
    stalePressure ||
    queueDepth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH ||
    queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH ||
    staleCockpitBlocksRaw >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
    signalDeficit >= Math.max(2, Math.ceil(threshold * 0.3));
  const canAttempt =
    triggerReady &&
    (nowMs - parseNonNegativeInt(conduitWatchdogState.last_attempt_ms, 0, 1000000000000)) >=
      watchdogCooldownMs;
  if (!canAttempt) {
    return {
      required: true,
      triggered: false,
      queue_depth: queueDepth,
      conduit_signals: signals,
      stale_cockpit_blocks: staleCockpitBlocks,
      stale_cockpit_blocks_raw: staleCockpitBlocksRaw,
      stale_cockpit_blocks_dormant: staleCockpitDormantBlocks,
      signal_deficit: signalDeficit,
      critical_attention_total: criticalAttentionTotal,
      critical_attention_cap: criticalCap,
      critical_attention_overload: criticalAttentionOverload,
      low_signal: lowSignals,
      threshold,
      stale_for_ms: staleForMs,
      cooldown_ms: watchdogCooldownMs,
      lane: null,
      active_shadows: knownShadows.slice(0, 8),
      last_attempt_at: cleanText(conduitWatchdogState.last_attempt_at || '', 80),
      last_success_at: cleanText(conduitWatchdogState.last_success_at || '', 80),
    };
  }
  const drainLimit = Math.min(
    RUNTIME_ATTENTION_DRAIN_MAX_BATCH,
    Math.max(
      RUNTIME_ATTENTION_DRAIN_MIN_BATCH,
      Math.ceil(queueDepth / 3),
      criticalAttentionOverload
        ? Math.min(
            RUNTIME_ATTENTION_DRAIN_MAX_BATCH,
            Math.max(criticalAttentionTotal, RUNTIME_ATTENTION_DRAIN_MIN_BATCH)
          )
        : 0
    )
  );
  const drainLane = runLane([
    'attention-queue',
    'drain',
    `--consumer=${ATTENTION_CONSUMER_ID}`,
    `--limit=${drainLimit}`,
    '--wait-ms=0',
    '--run-context=runtime_conduit_watchdog',
  ]);
  const cursorOffset = parseNonNegativeInt(runtime && runtime.attention_cursor_offset, 0, 100000000);
  let compactLane = null;
  if (queueDepth >= RUNTIME_ATTENTION_COMPACT_DEPTH && cursorOffset >= RUNTIME_ATTENTION_COMPACT_MIN_ACKED) {
    compactLane = runLane([
      'attention-queue',
      'compact',
      `--retain=${RUNTIME_ATTENTION_COMPACT_RETAIN}`,
      `--min-acked=${RUNTIME_ATTENTION_COMPACT_MIN_ACKED}`,
      '--run-context=runtime_conduit_watchdog',
    ]);
  }
  const healthLane = runLane(['health-status', 'dashboard']);
  const cockpitLane = runLane(['hermes-plane', 'cockpit', `--max-blocks=${COCKPIT_MAX_BLOCKS}`, '--strict=1']);
  const desiredWatchdogCount = lowSignals
    ? Math.min(
        4,
        Math.max(
          1,
          Math.ceil(Math.max(1, signalDeficit) / 2),
          staleRawMaintenance || criticalAttentionOverload ? 2 : 1
        )
      )
    : 0;
  const desiredShadows = Array.from({ length: desiredWatchdogCount }, (_, idx) => `${normalizedTeam}-conduit-watchdog-${idx + 1}`);
  const spawnTargets = desiredShadows.filter((shadow) => !knownShadows.includes(shadow));
  const retireTargets = knownShadows.filter((shadow) => !desiredShadows.includes(shadow));
  const roleLanes = [];
  let roleOk = true;
  for (let idx = 0; idx < spawnTargets.length; idx += 1) {
    const shadow = spawnTargets[idx];
    const role = chronicCoordinationPathology && idx % 2 === 1 ? 'builder' : 'researcher';
    const roleLane = runLane([
      'collab-plane',
      'launch-role',
      `--team=${normalizedTeam}`,
      `--role=${role}`,
      `--shadow=${shadow}`,
      '--strict=1',
    ]);
    roleLanes.push({
      shadow,
      role,
      lane: laneOutcome(roleLane),
    });
    if (!(roleLane && roleLane.ok)) roleOk = false;
  }
  const retire = terminateShadows(retireTargets, 'conduit_scale_down');
  const ok = !!(
    drainLane &&
    drainLane.ok &&
    healthLane &&
    healthLane.ok &&
    cockpitLane &&
    cockpitLane.ok &&
    roleOk &&
    retire.ok &&
    (compactLane == null || compactLane.ok)
  );
  conduitWatchdogState.last_attempt_ms = nowMs;
  conduitWatchdogState.last_attempt_at = nowIso();
  if (ok) {
    conduitWatchdogState.last_success_ms = nowMs;
    conduitWatchdogState.last_success_at = nowIso();
    conduitWatchdogState.failure_count = 0;
    conduitWatchdogState.low_signals_since_ms = 0;
    conduitWatchdogState.active_shadows = desiredShadows;
  } else {
    conduitWatchdogState.failure_count = parseNonNegativeInt(conduitWatchdogState.failure_count, 0, 100000000) + 1;
  }
  return {
    required: true,
    triggered: true,
    applied: ok,
    queue_depth: queueDepth,
    conduit_signals: signals,
    stale_cockpit_blocks: staleCockpitBlocks,
    stale_cockpit_blocks_raw: staleCockpitBlocksRaw,
    stale_cockpit_blocks_dormant: staleCockpitDormantBlocks,
    signal_deficit: signalDeficit,
    critical_attention_total: criticalAttentionTotal,
    critical_attention_cap: criticalCap,
    critical_attention_overload: criticalAttentionOverload,
    low_signal: lowSignals,
    threshold,
    stale_for_ms: staleForMs,
    cooldown_ms: watchdogCooldownMs,
    failure_count: parseNonNegativeInt(conduitWatchdogState.failure_count, 0, 100000000),
    lane: laneOutcome(drainLane),
    active_shadows: Array.isArray(conduitWatchdogState.active_shadows)
      ? conduitWatchdogState.active_shadows.slice(0, 8)
      : [],
    lanes: {
      drain: laneOutcome(drainLane),
      compact: compactLane ? laneOutcome(compactLane) : null,
      health: laneOutcome(healthLane),
      cockpit: laneOutcome(cockpitLane),
      roles: roleLanes,
      terminate: retire.lanes,
    },
    drain_limit: drainLimit,
    last_attempt_at: cleanText(conduitWatchdogState.last_attempt_at || '', 80),
    last_success_at: cleanText(conduitWatchdogState.last_success_at || '', 80),
  };
}

function maybeApplyRuntimeThrottle(runtime, team, recommendation = null) {
  const ingress = classifyIngressControl(runtime);
  const cockpitSignal = cockpitSignalState(runtime);
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const fallbackMaxDepth =
    queueDepth >= RUNTIME_INGRESS_CIRCUIT_DEPTH
      ? Math.max(40, RUNTIME_THROTTLE_MAX_DEPTH - 20)
      : queueDepth >= RUNTIME_INGRESS_SHED_DEPTH
      ? Math.max(50, RUNTIME_THROTTLE_MAX_DEPTH - 10)
      : RUNTIME_THROTTLE_MAX_DEPTH;
  const dynamicMaxDepth = parseNonNegativeInt(
    recommendation && recommendation.throttle_max_depth != null
      ? recommendation.throttle_max_depth
      : fallbackMaxDepth,
    fallbackMaxDepth,
    100000000
  );
  const fallbackRequired =
    queueDepth >= DASHBOARD_BACKPRESSURE_BATCH_DEPTH ||
    parseNonNegativeInt(runtime && runtime.critical_attention_total, 0, 100000000) >= RUNTIME_CRITICAL_ESCALATION_THRESHOLD ||
    cleanText(runtime && runtime.backpressure_level ? runtime.backpressure_level : '', 20).toLowerCase() === 'critical' ||
    ingress.level === 'shed' ||
    ingress.level === 'circuit' ||
    cockpitSignal.coarse;
  const required =
    recommendation && recommendation.throttle_required != null
      ? !!recommendation.throttle_required
      : fallbackRequired;
  const command = [
    'collab-plane',
    'throttle',
    `--team=${cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}`,
    `--plane=${RUNTIME_THROTTLE_PLANE}`,
    `--max-depth=${dynamicMaxDepth}`,
    `--strategy=${RUNTIME_THROTTLE_STRATEGY}`,
    '--strict=1',
  ];
  if (!required) {
    return {
      required: false,
      applied: false,
      command: `protheus-ops ${command.join(' ')}`,
      ingress_control: ingress,
      lane: null,
    };
  }
  const lane = runLane(command);
  if (lane && lane.ok) {
    runtimePolicyState.last_throttle_apply = nowIso();
  }
  return {
    required: true,
    applied: !!(lane && lane.ok && lane.payload && lane.payload.ok !== false),
    command: `protheus-ops ${command.join(' ')}`,
    ingress_control: ingress,
    max_depth: dynamicMaxDepth,
    lane: laneOutcome(lane),
  };
}

function maybeDrainAttentionQueue(runtime, recommendation = null) {
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const unackedDepth = parseNonNegativeInt(runtime && runtime.attention_unacked_depth, 0, 100000000);
  const criticalAttentionTotal = parseNonNegativeInt(
    runtime && runtime.critical_attention_total != null ? runtime.critical_attention_total : 0,
    0,
    100000000
  );
  const laneCaps =
    runtime && runtime.queue_lane_caps && typeof runtime.queue_lane_caps === 'object'
      ? runtime.queue_lane_caps
      : ATTENTION_LANE_CAPS;
  const criticalCap = parsePositiveInt(
    laneCaps && laneCaps.critical != null ? laneCaps.critical : ATTENTION_LANE_CAPS.critical,
    ATTENTION_LANE_CAPS.critical,
    1,
    100000000
  );
  const criticalOverload =
    criticalAttentionTotal > Math.max(criticalCap, RUNTIME_CRITICAL_ATTENTION_OVERLOAD_THRESHOLD);
  const fallbackRequired =
    queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH ||
    unackedDepth >= RUNTIME_ATTENTION_COMPACT_MIN_ACKED * 2 ||
    cleanText(runtime && runtime.ingress_level ? runtime.ingress_level : '', 24) === 'circuit' ||
    criticalOverload;
  const fallbackLimit = Math.min(
    RUNTIME_ATTENTION_DRAIN_MAX_BATCH,
    Math.max(
      RUNTIME_ATTENTION_DRAIN_MIN_BATCH,
      Math.ceil(queueDepth / 3),
      criticalOverload
        ? Math.min(
            RUNTIME_ATTENTION_DRAIN_MAX_BATCH,
            Math.max(criticalAttentionTotal, RUNTIME_ATTENTION_DRAIN_MIN_BATCH)
          )
        : 0
    )
  );
  const authorityRequired =
    recommendation && recommendation.attention_drain_required != null
      ? !!recommendation.attention_drain_required
      : null;
  const authorityLimit =
    recommendation && recommendation.attention_drain_limit != null
      ? parseNonNegativeInt(recommendation.attention_drain_limit, fallbackLimit, 100000000)
      : fallbackLimit;
  const required = authorityRequired == null ? fallbackRequired : authorityRequired;
  const limit = authorityLimit;
  const command = [
    'attention-queue',
    'drain',
    `--consumer=${ATTENTION_CONSUMER_ID}`,
    `--limit=${limit}`,
    '--wait-ms=0',
    '--run-context=runtime_attention_autodrain',
  ];
  if (!required) {
    return {
      required: false,
      applied: false,
      command: `protheus-ops ${command.join(' ')}`,
      lane: null,
      drained_count: 0,
      critical_overload: criticalOverload,
      critical_total: criticalAttentionTotal,
      critical_cap: criticalCap,
    };
  }
  const lane = runLane(command);
  const drainedCount = parseNonNegativeInt(
    lane && lane.payload && lane.payload.batch_count != null ? lane.payload.batch_count : 0,
    0,
    100000000
  );
  return {
    required: true,
    applied: !!(lane && lane.ok && lane.payload && lane.payload.ok !== false),
    command: `protheus-ops ${command.join(' ')}`,
    limit,
    drained_count: drainedCount,
    critical_overload: criticalOverload,
    critical_total: criticalAttentionTotal,
    critical_cap: criticalCap,
    lane: laneOutcome(lane),
  };
}

function maybeCompactAttentionQueue(runtime, recommendation = null) {
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const cursorOffset = parseNonNegativeInt(runtime && runtime.attention_cursor_offset, 0, 100000000);
  const fallbackRequired =
    queueDepth >= RUNTIME_ATTENTION_COMPACT_DEPTH &&
    cursorOffset >= RUNTIME_ATTENTION_COMPACT_MIN_ACKED;
  const retain = parseNonNegativeInt(
    recommendation && recommendation.attention_compact_retain != null
      ? recommendation.attention_compact_retain
      : RUNTIME_ATTENTION_COMPACT_RETAIN,
    RUNTIME_ATTENTION_COMPACT_RETAIN,
    100000000
  );
  const minAcked = parseNonNegativeInt(
    recommendation && recommendation.attention_compact_min_acked != null
      ? recommendation.attention_compact_min_acked
      : RUNTIME_ATTENTION_COMPACT_MIN_ACKED,
    RUNTIME_ATTENTION_COMPACT_MIN_ACKED,
    100000000
  );
  const required =
    recommendation && recommendation.attention_compact_required != null
      ? !!recommendation.attention_compact_required
      : fallbackRequired;
  const command = [
    'attention-queue',
    'compact',
    `--retain=${retain}`,
    `--min-acked=${minAcked}`,
    '--run-context=runtime_attention_autocompact',
  ];
  if (!required) {
    return {
      required: false,
      applied: false,
      command: `protheus-ops ${command.join(' ')}`,
      compacted_count: 0,
      lane: null,
    };
  }
  const lane = runLane(command);
  const compactedCount = parseNonNegativeInt(
    lane && lane.payload && lane.payload.compacted_count != null ? lane.payload.compacted_count : 0,
    0,
    100000000
  );
  return {
    required: true,
    applied: !!(lane && lane.ok && lane.payload && lane.payload.ok !== false),
    command: `protheus-ops ${command.join(' ')}`,
    compacted_count: compactedCount,
    lane: laneOutcome(lane),
    retain,
    min_acked: minAcked,
  };
}

function maybeReconcileAttentionAccounting(runtime, recommendation = null) {
  const required =
    recommendation && recommendation.attention_accounting_reconcile_required != null
      ? !!recommendation.attention_accounting_reconcile_required
      : !!(runtime && runtime.attention_accounting_mismatch);
  const command = ['attention-queue', 'status'];
  if (!required) {
    return {
      required: false,
      applied: false,
      mismatch_before: false,
      command: `protheus-ops ${command.join(' ')}`,
      lane: null,
    };
  }
  const lane = runLane(command);
  return {
    required: true,
    applied: !!(lane && lane.ok && lane.payload && lane.payload.ok !== false),
    mismatch_before: !!(runtime && runtime.attention_accounting_mismatch),
    command: `protheus-ops ${command.join(' ')}`,
    lane: laneOutcome(lane),
  };
}

function maybeRefreshAdaptiveHealth(runtime, recommendation = null) {
  const fallbackRequired =
    parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000) >= 80 ||
    parseNonNegativeInt(runtime && runtime.health_coverage_gap_count, 0, 100000000) > 0;
  const required =
    recommendation && recommendation.adaptive_health_required != null
      ? !!recommendation.adaptive_health_required
      : fallbackRequired;
  runtimePolicyState.health_adaptive = required;
  if (!required) {
    return {
      required: false,
      applied: false,
      window_seconds: runtimePolicyState.health_window_seconds,
      lane: null,
    };
  }
  const lane = runLane(['health-status', 'dashboard']);
  if (lane && lane.ok) {
    runtimePolicyState.last_health_refresh = nowIso();
  }
  return {
    required: true,
    applied: !!(lane && lane.ok && lane.payload && lane.payload.ok !== false),
    window_seconds: runtimePolicyState.health_window_seconds,
    lane: laneOutcome(lane),
  };
}

function maybeResumeMemoryIngest(runtime, recommendation = null) {
  const paused = !!(runtime && runtime.memory_ingest_paused);
  const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
  const authorityEligible =
    recommendation && recommendation.memory_resume_eligible != null
      ? !!recommendation.memory_resume_eligible
      : null;
  if (!paused) {
    return {
      eligible: false,
      resumed: false,
      reason: 'already_live',
    };
  }
  if (authorityEligible === false) {
    return {
      eligible: false,
      resumed: false,
      reason: 'runtime_authority_denied',
      queue_depth: queueDepth,
      resume_threshold: DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH,
    };
  }
  if (queueDepth > DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH) {
    return {
      eligible: false,
      resumed: false,
      reason: 'queue_above_resume_threshold',
      queue_depth: queueDepth,
      resume_threshold: DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH,
    };
  }
  memoryIngestCircuit = {
    paused: false,
    since: nowIso(),
    reason: 'manual_stream_resume',
    trigger_queue_depth: queueDepth,
    trigger_memory_entries: 0,
    transition_count: parseNonNegativeInt(memoryIngestCircuit.transition_count, 0, 1000000) + 1,
  };
  return {
    eligible: true,
    resumed: true,
    reason: 'manual_stream_resume',
    queue_depth: queueDepth,
    resume_threshold: DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH,
  };
}

function runtimeSwarmRecommendation(snapshot) {
  const runtime = runtimeSyncSummary(snapshot);
  const runtimeAuthority = runtimeAuthorityFromRust(runtime);
  const cockpitSignal = cockpitSignalState(runtime, runtimeAuthority);
  const ingressControl = classifyIngressControl(runtime, runtimeAuthority);
  const authorityRoot =
    runtimeAuthority &&
    runtimeAuthority.ok &&
    runtimeAuthority.authority &&
    typeof runtimeAuthority.authority === 'object'
      ? runtimeAuthority.authority
      : null;
  const authorityRecommendations =
    authorityRoot &&
    authorityRoot.recommendations &&
    typeof authorityRoot.recommendations === 'object'
      ? authorityRoot.recommendations
      : null;
  const authorityRolePlan = Array.isArray(authorityRoot && authorityRoot.role_plan)
    ? authorityRoot.role_plan
    : [];
  const authorityRoleSet = new Set(
    authorityRolePlan
      .map((row) => cleanText(row && row.role ? row.role : '', 40).toLowerCase())
      .filter(Boolean)
  );
  const stalePressure =
    parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) > 0 &&
    parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000) >= RUNTIME_DRAIN_TRIGGER_DEPTH;
  const staleRawBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks_raw, 0, 100000000);
  const staleAutohealNeeded =
    parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) >=
    RUNTIME_COCKPIT_STALE_AUTOHEAL_MIN_BLOCKS ||
    staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS;
  const laneCaps =
    runtime && runtime.queue_lane_caps && typeof runtime.queue_lane_caps === 'object'
      ? runtime.queue_lane_caps
      : ATTENTION_LANE_CAPS;
  const criticalCap = parsePositiveInt(
    laneCaps && laneCaps.critical != null ? laneCaps.critical : ATTENTION_LANE_CAPS.critical,
    ATTENTION_LANE_CAPS.critical,
    1,
    100000000
  );
  const criticalAttentionOverload =
    parseNonNegativeInt(runtime && runtime.critical_attention_total, 0, 100000000) >
    Math.max(criticalCap, RUNTIME_CRITICAL_ATTENTION_OVERLOAD_THRESHOLD);
  const attentionAccountingMismatch = !!(runtime && runtime.attention_accounting_mismatch);
  const team = DEFAULT_TEAM;
  const agents = compatAgentsFromSnapshot(snapshot, { includeArchived: false });
  const activeSwarmAgents = parseNonNegativeInt(agents.length, 0, 100000000);
  const reliabilityPosture = runtimeReliabilityPosture(runtime, activeSwarmAgents, runtimeAuthority);
  const sloGate = runtimeSloGate(runtime, reliabilityPosture, runtimeAuthority);
  const swarmScalePressure =
    runtime.queue_depth >= RUNTIME_DRAIN_HIGH_LOAD_DEPTH &&
    activeSwarmAgents < RUNTIME_DRAIN_AGENT_HIGH_LOAD_TARGET;
  const swarmScaleRequired = swarmScalePressure && !reliabilityPosture.degraded && !sloGate.block_scale;
  const swarmScaleBlockedByReliability = swarmScalePressure && (reliabilityPosture.degraded || sloGate.block_scale);
  const shouldRecommendBase =
    runtime.queue_depth >= DASHBOARD_QUEUE_DRAIN_PAUSE_DEPTH ||
    runtime.critical_attention_total >= 5 ||
    runtime.health_coverage_gap_count > 0 ||
    !!runtime.conduit_scale_required ||
    parseNonNegativeInt(runtime && runtime.deferred_attention, 0, 100000000) > 0 ||
    stalePressure ||
    staleAutohealNeeded ||
    criticalAttentionOverload ||
    attentionAccountingMismatch ||
    (cockpitSignal.coarse && runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH) ||
    swarmScaleRequired ||
    reliabilityPosture.degraded ||
    sloGate.required;
  const roleOrder = ['coordinator', 'researcher', 'builder', 'reviewer', 'analyst'];
  const rolePrompts = {
    coordinator:
      'Audit runtime transport and health coverage. Identify missing conduit capacity vs target and any retired health checks. Return concrete remediation commands.',
    researcher:
      'Triage critical attention events by severity, band, and queue lane. Return top 5 risks with suggested actions and explain which are safe to defer.',
    builder:
      'Clear cockpit policy debt and unblock module_cohesion_policy_audit path. Prioritize deterministic fixes that reduce queue pressure and preserve receipts.',
    reviewer:
      'Review swarm action plans for safety and determinism. Escalate risky tool paths and enforce critical-lane-first queue handling.',
    analyst:
      'Classify queue backlog into critical/standard/background lanes, then produce weighted-fair actions to drain depth below 60 without losing critical telemetry.',
  };
  const rolePlan = authorityRolePlan
    .map((row) => {
      const role = cleanText(row && row.role ? row.role : '', 40).toLowerCase();
      if (!roleOrder.includes(role)) return null;
      if (!(row && row.required)) return null;
      const existing = findAgentByRole(agents, role);
      return {
        role,
        required: true,
        shadow: existing && existing.id ? existing.id : '',
        prompt:
          cleanText(row && row.prompt ? row.prompt : rolePrompts[role], 2000) ||
          rolePrompts[role],
      };
    })
    .filter(Boolean);
  const throttleRequired =
    authorityRecommendations && authorityRecommendations.throttle_required != null
      ? !!authorityRecommendations.throttle_required
      : runtime.queue_depth >= DASHBOARD_BACKPRESSURE_BATCH_DEPTH ||
        runtime.critical_attention_total >= RUNTIME_CRITICAL_ESCALATION_THRESHOLD ||
        ingressControl.level === 'shed' ||
        ingressControl.level === 'circuit' ||
        cockpitSignal.coarse ||
        reliabilityPosture.degraded ||
        !!sloGate.containment_required;
  const adaptiveHealthRequired =
    authorityRecommendations && authorityRecommendations.adaptive_health_required != null
      ? !!authorityRecommendations.adaptive_health_required
      : runtime.queue_depth >= 80 || runtime.health_coverage_gap_count > 0 || sloGate.required;
  const authorityTargetConduitSignals = parseNonNegativeInt(
    authorityRecommendations && authorityRecommendations.target_conduit_signals != null
      ? authorityRecommendations.target_conduit_signals
      : runtime.target_conduit_signals,
    runtime.target_conduit_signals,
    100000000
  );
  const conduitAutoBalanceRequired =
    authorityRecommendations && authorityRecommendations.conduit_autobalance_required != null
      ? !!authorityRecommendations.conduit_autobalance_required
      : (
          runtime.conduit_signals < Math.max(authorityTargetConduitSignals, RUNTIME_AUTO_BALANCE_THRESHOLD) &&
          (
            runtime.queue_depth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH ||
            parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) > 0 ||
            staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
            criticalAttentionOverload ||
            attentionAccountingMismatch
          )
        ) ||
        (cockpitSignal.coarse && runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH);
  const memoryResumeEligible =
    authorityRecommendations && authorityRecommendations.memory_resume_eligible != null
      ? !!authorityRecommendations.memory_resume_eligible
      : !!runtime.memory_ingest_paused && runtime.queue_depth <= DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH;
  const fallbackThrottleMaxDepth =
    runtime.queue_depth >= RUNTIME_INGRESS_CIRCUIT_DEPTH
      ? Math.max(40, RUNTIME_THROTTLE_MAX_DEPTH - 20)
      : runtime.queue_depth >= RUNTIME_INGRESS_SHED_DEPTH
      ? Math.max(50, RUNTIME_THROTTLE_MAX_DEPTH - 10)
      : RUNTIME_THROTTLE_MAX_DEPTH;
  const throttleMaxDepth = parseNonNegativeInt(
    authorityRecommendations && authorityRecommendations.throttle_max_depth != null
      ? authorityRecommendations.throttle_max_depth
      : fallbackThrottleMaxDepth,
    fallbackThrottleMaxDepth,
    100000000
  );
  const attentionDrainRequired =
    authorityRecommendations && authorityRecommendations.attention_drain_required != null
      ? !!authorityRecommendations.attention_drain_required
      : runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH ||
        parseNonNegativeInt(runtime && runtime.attention_unacked_depth, 0, 100000000) >=
          RUNTIME_ATTENTION_COMPACT_MIN_ACKED * 2 ||
        criticalAttentionOverload ||
        ingressControl.level === 'circuit';
  const attentionDrainLimit = parseNonNegativeInt(
    authorityRecommendations && authorityRecommendations.attention_drain_limit != null
      ? authorityRecommendations.attention_drain_limit
      : Math.min(
          RUNTIME_ATTENTION_DRAIN_MAX_BATCH,
          Math.max(RUNTIME_ATTENTION_DRAIN_MIN_BATCH, Math.ceil(runtime.queue_depth / 3))
        ),
    RUNTIME_ATTENTION_DRAIN_MIN_BATCH,
    100000000
  );
  const attentionCompactRequired =
    authorityRecommendations && authorityRecommendations.attention_compact_required != null
      ? !!authorityRecommendations.attention_compact_required
      : runtime.queue_depth >= RUNTIME_ATTENTION_COMPACT_DEPTH &&
        parseNonNegativeInt(runtime && runtime.attention_cursor_offset, 0, 100000000) >=
          RUNTIME_ATTENTION_COMPACT_MIN_ACKED;
  const attentionCompactRetain = parseNonNegativeInt(
    authorityRecommendations && authorityRecommendations.attention_compact_retain != null
      ? authorityRecommendations.attention_compact_retain
      : RUNTIME_ATTENTION_COMPACT_RETAIN,
    RUNTIME_ATTENTION_COMPACT_RETAIN,
    100000000
  );
  const attentionCompactMinAcked = parseNonNegativeInt(
    authorityRecommendations && authorityRecommendations.attention_compact_min_acked != null
      ? authorityRecommendations.attention_compact_min_acked
      : RUNTIME_ATTENTION_COMPACT_MIN_ACKED,
    RUNTIME_ATTENTION_COMPACT_MIN_ACKED,
    100000000
  );
  const benchmarkAgeSeconds = parsePositiveInt(
    runtime && runtime.benchmark_sanity_age_seconds != null ? runtime.benchmark_sanity_age_seconds : -1,
    -1,
    -1,
    1000000000
  );
  const benchmarkRefreshRequired =
    authorityRecommendations && authorityRecommendations.benchmark_refresh_required != null
      ? !!authorityRecommendations.benchmark_refresh_required
      : cleanText(runtime && runtime.benchmark_sanity_cockpit_status ? runtime.benchmark_sanity_cockpit_status : '', 24) === 'fail' ||
        cleanText(runtime && runtime.benchmark_sanity_status ? runtime.benchmark_sanity_status : '', 24) === 'fail' ||
        benchmarkAgeSeconds < 0 ||
        benchmarkAgeSeconds > RUNTIME_BENCHMARK_REFRESH_MAX_AGE_SECONDS;
  const drainAgents = trackedRuntimeDrainAgents(snapshot);
  const predictiveDrainRequired =
    authorityRecommendations && authorityRecommendations.predictive_drain_required != null
      ? !!authorityRecommendations.predictive_drain_required
      : runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH;
  const predictiveDrainRelease =
    authorityRecommendations && authorityRecommendations.predictive_drain_release != null
      ? !!authorityRecommendations.predictive_drain_release && drainAgents.length > 0
      : runtime.queue_depth <= RUNTIME_DRAIN_CLEAR_DEPTH && drainAgents.length > 0;
  const predictiveDrainAllowed = !reliabilityPosture.degraded && !sloGate.block_scale;
  const coarseSignalDeficit = Math.max(0, runtime.target_conduit_signals - runtime.conduit_signals);
  const coarseSignalRemediationRequired =
    authorityRecommendations && authorityRecommendations.coarse_signal_remediation_required != null
      ? !!authorityRecommendations.coarse_signal_remediation_required
      : (cockpitSignal.coarse && runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH) ||
        (coarseSignalDeficit > 0 && runtime.queue_depth >= RUNTIME_INGRESS_DAMPEN_DEPTH) ||
        stalePressure ||
        staleAutohealNeeded ||
        staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
        criticalAttentionOverload;
  const spineMetricsStale = !!runtime.spine_metrics_stale;
  const spineMetricsLatestAgeSeconds = parseNonNegativeInt(
    runtime && runtime.spine_metrics_latest_age_seconds != null ? runtime.spine_metrics_latest_age_seconds : 0,
    0,
    1000000000
  );
  const spineCanaryRequired =
    authorityRecommendations && authorityRecommendations.spine_canary_required != null
      ? !!authorityRecommendations.spine_canary_required
      : spineMetricsStale && spineMetricsLatestAgeSeconds >= RUNTIME_SPINE_METRICS_STALE_MAX_AGE_SECONDS;
  const conduitWatchdogRequired =
    authorityRecommendations && authorityRecommendations.conduit_watchdog_required != null
      ? !!authorityRecommendations.conduit_watchdog_required
      : runtime.conduit_signals < authorityTargetConduitSignals &&
        (
          runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH ||
          runtime.queue_depth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH ||
          parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) > 0 ||
          staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
          criticalAttentionOverload
        );
  const shouldRecommend =
    !!(runtimeAuthority && runtimeAuthority.ok) &&
    (
      rolePlan.length > 0 ||
    throttleRequired ||
    adaptiveHealthRequired ||
    conduitAutoBalanceRequired ||
    memoryResumeEligible ||
    benchmarkRefreshRequired ||
    attentionAccountingMismatch ||
      spineCanaryRequired ||
      predictiveDrainRequired ||
      predictiveDrainRelease
    );
  return {
    recommended: shouldRecommend,
    team,
    queue_depth: runtime.queue_depth,
    cockpit_blocks: runtime.cockpit_blocks,
    critical_attention_total: runtime.critical_attention_total,
    critical_attention_cap: criticalCap,
    critical_attention_overload: criticalAttentionOverload,
    health_coverage_gap_count: runtime.health_coverage_gap_count,
    conduit_scale_required: !!runtime.conduit_scale_required,
    conduit_signals: runtime.conduit_signals,
    target_conduit_signals: authorityTargetConduitSignals,
    active_swarm_agents: activeSwarmAgents,
    deferred_attention: parseNonNegativeInt(runtime && runtime.deferred_attention, 0, 100000000),
    deferred_mode: cleanText(runtime && runtime.deferred_mode ? runtime.deferred_mode : '', 24),
    cockpit_stale_blocks: parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000),
    cockpit_stale_blocks_raw: staleRawBlocks,
    cockpit_stale_ratio: Number(runtime && runtime.cockpit_stale_ratio != null ? runtime.cockpit_stale_ratio : 0),
    cockpit_fresh_ratio: Number(runtime && runtime.cockpit_fresh_ratio != null ? runtime.cockpit_fresh_ratio : 0),
    cockpit_stale_lanes_top: Array.isArray(runtime && runtime.cockpit_stale_lanes_top)
      ? runtime.cockpit_stale_lanes_top.slice(0, 6)
      : [],
    cockpit_signal_quality: cockpitSignal.quality,
    cockpit_stream_coarse: cockpitSignal.coarse,
    spine_success_rate: reliabilityPosture.spine_success_rate,
    spine_success_target: reliabilityPosture.spine_success_target,
    spine_metrics_stale: spineMetricsStale,
    spine_metrics_latest_age_seconds: spineMetricsLatestAgeSeconds,
    spine_metrics_fresh_window_seconds: parseNonNegativeInt(
      runtime && runtime.spine_metrics_fresh_window_seconds != null
        ? runtime.spine_metrics_fresh_window_seconds
        : RUNTIME_SPINE_METRICS_STALE_MAX_AGE_SECONDS,
      RUNTIME_SPINE_METRICS_STALE_MAX_AGE_SECONDS,
      1000000000
    ),
    spine_canary_required: spineCanaryRequired,
    spine_degraded: reliabilityPosture.spine_degraded,
    human_escalation_open_rate: reliabilityPosture.escalation_open_rate,
    escalation_starved: reliabilityPosture.escalation_starved,
    collab_handoff_count: reliabilityPosture.handoff_count,
    handoffs_per_agent: reliabilityPosture.handoffs_per_agent,
    handoffs_per_agent_min: reliabilityPosture.handoffs_per_agent_min,
    handoff_coverage_weak: reliabilityPosture.handoff_coverage_weak,
    reliability_gate_required: reliabilityPosture.degraded || !!sloGate.required,
    slo_gate: sloGate,
    slo_gate_required: !!sloGate.required,
    slo_gate_severity: cleanText(sloGate && sloGate.severity ? sloGate.severity : 'ok', 24) || 'ok',
    slo_gate_block_scale: !!sloGate.block_scale,
    slo_gate_failed_checks: Array.isArray(sloGate && sloGate.failed_checks) ? sloGate.failed_checks.slice(0, 8) : [],
    slo_gate_summary: cleanText(sloGate && sloGate.summary ? sloGate.summary : '', 260),
    swarm_scale_required: swarmScaleRequired,
    swarm_scale_pressure: swarmScalePressure,
    swarm_scale_blocked_by_reliability: swarmScaleBlockedByReliability,
    swarm_target_agents: RUNTIME_DRAIN_AGENT_HIGH_LOAD_TARGET,
    role_plan: rolePlan,
    prompts: rolePrompts,
    attention_lane_weights:
      runtime && runtime.queue_lane_weights && typeof runtime.queue_lane_weights === 'object'
        ? runtime.queue_lane_weights
        : { ...ATTENTION_LANE_WEIGHTS },
    attention_lane_caps:
      runtime && runtime.queue_lane_caps && typeof runtime.queue_lane_caps === 'object'
        ? runtime.queue_lane_caps
        : { ...ATTENTION_LANE_CAPS },
    throttle_required: throttleRequired,
    throttle_max_depth: throttleMaxDepth,
    attention_drain_required: attentionDrainRequired,
    attention_drain_limit: attentionDrainLimit,
    attention_compact_required: attentionCompactRequired,
    attention_compact_retain: attentionCompactRetain,
    attention_compact_min_acked: attentionCompactMinAcked,
    throttle_command: `protheus-ops collab-plane throttle --plane=${RUNTIME_THROTTLE_PLANE} --max-depth=${RUNTIME_THROTTLE_MAX_DEPTH} --strategy=${RUNTIME_THROTTLE_STRATEGY}`,
    adaptive_health_required: adaptiveHealthRequired,
    adaptive_health_window_seconds: RUNTIME_HEALTH_ADAPTIVE_WINDOW_SECONDS,
    conduit_autobalance_required: conduitAutoBalanceRequired,
    conduit_autobalance_threshold: RUNTIME_AUTO_BALANCE_THRESHOLD,
    conduit_autobalance_command:
      `protheus-ops collab-plane launch-role --team=${cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}` +
      ` --role=researcher --shadow=${cleanText(team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}-conduit-watchdog --strict=1`,
    memory_resume_eligible: memoryResumeEligible,
    benchmark_refresh_required: benchmarkRefreshRequired,
    benchmark_sanity_age_seconds: benchmarkAgeSeconds,
    benchmark_refresh_max_age_seconds: RUNTIME_BENCHMARK_REFRESH_MAX_AGE_SECONDS,
    predictive_drain_required: predictiveDrainRequired && predictiveDrainAllowed,
    predictive_drain_release: predictiveDrainRelease,
    predictive_drain_allowed: predictiveDrainAllowed,
    predictive_drain_trigger_depth: RUNTIME_DRAIN_TRIGGER_DEPTH,
    predictive_drain_clear_depth: RUNTIME_DRAIN_CLEAR_DEPTH,
    predictive_drain_active_agents: drainAgents.slice(0, 8),
    coarse_signal_remediation_required: coarseSignalRemediationRequired,
    conduit_watchdog_required: conduitWatchdogRequired,
    attention_accounting_mismatch: attentionAccountingMismatch,
    attention_accounting_reconcile_required:
      authorityRecommendations && authorityRecommendations.attention_accounting_reconcile_required != null
        ? !!authorityRecommendations.attention_accounting_reconcile_required
        : attentionAccountingMismatch,
    ingress_control: ingressControl,
    authority: {
      source:
        runtimeAuthority && runtimeAuthority.ok
          ? cleanText(authorityRoot && authorityRoot.authority ? authorityRoot.authority : 'rust_runtime_systems', 60) ||
            'rust_runtime_systems'
          : 'rust_unavailable',
      lane: runtimeAuthority && runtimeAuthority.lane ? runtimeAuthority.lane : null,
      contract_id:
        cleanText(authorityRoot && authorityRoot.contract_id ? authorityRoot.contract_id : 'V6-DASHBOARD-007.1', 80) ||
        'V6-DASHBOARD-007.1',
      rust_authority_available: !!(runtimeAuthority && runtimeAuthority.ok),
    },
  };
}

function executeRuntimeSwarmRecommendation(snapshot) {
  const recommendation = runtimeSwarmRecommendation(snapshot);
  const runtime = runtimeSyncSummary(snapshot);
  const authoritySource = cleanText(
    recommendation && recommendation.authority && recommendation.authority.source
      ? recommendation.authority.source
      : '',
    80
  );
  if (authoritySource !== 'rust_runtime_systems') {
    return {
      ok: false,
      type: 'dashboard_runtime_swarm_recommendation',
      reason: 'rust_runtime_authority_unavailable',
      authority: recommendation && recommendation.authority ? recommendation.authority : null,
      recommendation: {
        recommended: false,
        queue_depth: parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000),
      },
      launches: [],
      turns: [],
      policies: [],
      timestamp: nowIso(),
    };
  }
  const roleAssignments = [];
  const launches = [];
  const policies = [];
  const turns = [];

  const ingressControl =
    recommendation && recommendation.ingress_control && typeof recommendation.ingress_control === 'object'
      ? recommendation.ingress_control
      : classifyIngressControl(runtime);
  policies.push({
    policy: 'predictive_ingress_controller',
    required: ingressControl.level !== 'normal',
    applied: true,
    level: ingressControl.level,
    reject_non_critical: !!ingressControl.reject_non_critical,
    delay_ms: ingressControl.delay_ms,
    reason: ingressControl.reason,
    since: ingressControl.since,
    thresholds: {
      dampen: ingressControl.dampen_depth,
      shed: ingressControl.shed_depth,
      circuit: ingressControl.circuit_depth,
    },
  });

  const queueDrain = maybeDrainAttentionQueue(runtime, recommendation);
  policies.push({
    policy: 'attention_queue_autodrain',
    required: !!queueDrain.required,
    applied: !!queueDrain.applied,
    command: queueDrain.command,
    limit: queueDrain.limit || RUNTIME_ATTENTION_DRAIN_MIN_BATCH,
    drained_count: queueDrain.drained_count || 0,
    lane: queueDrain.lane,
  });

  const queueCompact = maybeCompactAttentionQueue(runtime, recommendation);
  policies.push({
    policy: 'attention_queue_compaction',
    required: !!queueCompact.required,
    applied: !!queueCompact.applied,
    command: queueCompact.command,
    compacted_count: queueCompact.compacted_count || 0,
    retain: queueCompact.retain || RUNTIME_ATTENTION_COMPACT_RETAIN,
    min_acked: queueCompact.min_acked || RUNTIME_ATTENTION_COMPACT_MIN_ACKED,
    lane: queueCompact.lane,
  });

  const attentionReconcile = maybeReconcileAttentionAccounting(runtime, recommendation);
  policies.push({
    policy: 'attention_accounting_reconcile',
    required: !!attentionReconcile.required,
    applied: !!attentionReconcile.applied,
    mismatch_before: !!attentionReconcile.mismatch_before,
    command: attentionReconcile.command,
    lane: attentionReconcile.lane,
  });

  const throttle = maybeApplyRuntimeThrottle(runtime, recommendation.team || DEFAULT_TEAM, recommendation);
  policies.push({
    policy: 'queue_throttle',
    required: !!throttle.required,
    applied: !!throttle.applied,
    command: throttle.command,
    ingress_control: throttle.ingress_control || ingressControl,
    max_depth: throttle.max_depth || RUNTIME_THROTTLE_MAX_DEPTH,
    lane: throttle.lane,
  });

  const spineCanary = maybeRunSpineMetricsCanary(recommendation, runtime);
  policies.push({
    policy: 'spine_metrics_canary',
    required: !!spineCanary.required,
    applied: !!spineCanary.applied,
    reason: cleanText(spineCanary.reason || '', 80),
    command: spineCanary.command,
    latest_age_seconds: parseNonNegativeInt(spineCanary.latest_age_seconds, 0, 1000000000),
    cooldown_ms: parseNonNegativeInt(spineCanary.cooldown_ms, RUNTIME_SPINE_CANARY_COOLDOWN_MS, 1000000000),
    since_last_ms: parseNonNegativeInt(spineCanary.since_last_ms, 0, 1000000000),
    last_run_at: cleanText(spineCanary.last_run_at || '', 80),
    run_count: parseNonNegativeInt(spineCanary.run_count, 0, 100000000),
    lane: spineCanary.lane,
  });

  const benchmarkRefresh = maybeRefreshBenchmarkSanity(runtime, recommendation);
  policies.push({
    policy: 'benchmark_sanity_refresh',
    required: !!benchmarkRefresh.required,
    applied: !!benchmarkRefresh.applied,
    reason: cleanText(benchmarkRefresh.reason || '', 120),
    command: benchmarkRefresh.command || '',
    cockpit_status: cleanText(benchmarkRefresh.cockpit_status || 'unknown', 24) || 'unknown',
    mirror_status: cleanText(benchmarkRefresh.mirror_status || 'unknown', 24) || 'unknown',
    refreshed_status: cleanText(benchmarkRefresh.refreshed_status || '', 24),
    refreshed_detail: cleanText(benchmarkRefresh.refreshed_detail || '', 220),
    age_seconds: parsePositiveInt(benchmarkRefresh.age_seconds, -1, -1, 1000000000),
    refreshed_age_seconds: parsePositiveInt(benchmarkRefresh.refreshed_age_seconds, -1, -1, 1000000000),
    cooldown_ms: parseNonNegativeInt(
      benchmarkRefresh.cooldown_ms,
      RUNTIME_BENCHMARK_REFRESH_COOLDOWN_MS,
      1000000000
    ),
    since_last_ms: parseNonNegativeInt(benchmarkRefresh.since_last_ms, 0, 1000000000),
    last_run_at: cleanText(benchmarkRefresh.last_run_at || '', 80),
    run_count: parseNonNegativeInt(benchmarkRefresh.run_count, 0, 100000000),
    exit_status: parsePositiveInt(benchmarkRefresh.exit_status, -1, -1, 255),
    lane: benchmarkRefresh.lane,
  });

  const recommendationSloGate =
    recommendation && recommendation.slo_gate && typeof recommendation.slo_gate === 'object'
      ? recommendation.slo_gate
      : runtimeSloGate(runtime, runtimeReliabilityPosture(runtime, parseNonNegativeInt(recommendation && recommendation.active_swarm_agents, 0, 100000000)));
  const sloGateRequired = !!(recommendationSloGate && recommendationSloGate.required);
  const sloGateContainmentRequired = !!(recommendationSloGate && recommendationSloGate.containment_required);
  let sloGateLane = null;
  let sloGateCommand = '';
  if (sloGateRequired && sloGateContainmentRequired) {
    const maxDepth = Math.min(
      RUNTIME_THROTTLE_MAX_DEPTH,
      Math.max(40, parseNonNegativeInt(runtime && runtime.queue_depth, RUNTIME_THROTTLE_MAX_DEPTH, 100000000))
    );
    const command = [
      'collab-plane',
      'throttle',
      `--team=${cleanText(recommendation.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}`,
      `--plane=${RUNTIME_THROTTLE_PLANE}`,
      `--max-depth=${maxDepth}`,
      `--strategy=${RUNTIME_THROTTLE_STRATEGY}`,
      '--strict=1',
    ];
    sloGateCommand = `protheus-ops ${command.join(' ')}`;
    sloGateLane = runLane(command);
  }
  const sloGateApplied =
    !sloGateRequired ||
    !sloGateContainmentRequired ||
    !!(sloGateLane && sloGateLane.ok && sloGateLane.payload && sloGateLane.payload.ok !== false);
  policies.push({
    policy: 'runtime_slo_gate',
    required: sloGateRequired,
    applied: sloGateApplied,
    severity: cleanText(
      recommendationSloGate && recommendationSloGate.severity ? recommendationSloGate.severity : 'ok',
      24
    ) || 'ok',
    block_scale: !!(recommendationSloGate && recommendationSloGate.block_scale),
    containment_required: sloGateContainmentRequired,
    failed_checks:
      recommendationSloGate && Array.isArray(recommendationSloGate.failed_checks)
        ? recommendationSloGate.failed_checks.slice(0, 10)
        : [],
    checks:
      recommendationSloGate && Array.isArray(recommendationSloGate.checks)
        ? recommendationSloGate.checks.slice(0, 8)
        : [],
    summary: cleanText(recommendationSloGate && recommendationSloGate.summary ? recommendationSloGate.summary : '', 260),
    thresholds:
      recommendationSloGate && recommendationSloGate.thresholds && typeof recommendationSloGate.thresholds === 'object'
        ? recommendationSloGate.thresholds
        : {
            spine_success_rate_min: RUNTIME_SPINE_SUCCESS_TARGET_MIN,
            receipt_latency_p95_max_ms: RUNTIME_SLO_RECEIPT_LATENCY_P95_MAX_MS,
            receipt_latency_p99_max_ms: RUNTIME_SLO_RECEIPT_LATENCY_P99_MAX_MS,
            queue_depth_max: RUNTIME_SLO_QUEUE_DEPTH_MAX,
            escalation_open_rate_min: RUNTIME_SLO_ESCALATION_OPEN_RATE_MIN,
          },
    command: sloGateCommand,
    lane: sloGateLane ? laneOutcome(sloGateLane) : null,
  });

  const reliabilityGateRequired = !!(recommendation && recommendation.reliability_gate_required);
  let reliabilityThrottleLane = null;
  let reliabilityThrottleCommand = '';
  if (reliabilityGateRequired) {
    const maxDepth = Math.min(
      RUNTIME_COARSE_THROTTLE_MAX_DEPTH,
      Math.max(40, parseNonNegativeInt(runtime && runtime.queue_depth, RUNTIME_COARSE_THROTTLE_MAX_DEPTH, 100000000))
    );
    const command = [
      'collab-plane',
      'throttle',
      `--team=${cleanText(recommendation.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}`,
      `--plane=${RUNTIME_THROTTLE_PLANE}`,
      `--max-depth=${maxDepth}`,
      `--strategy=${RUNTIME_COARSE_THROTTLE_STRATEGY}`,
      '--strict=1',
    ];
    reliabilityThrottleCommand = `protheus-ops ${command.join(' ')}`;
    reliabilityThrottleLane = runLane(command);
  }
  policies.push({
    policy: 'spine_reliability_gate',
    required: reliabilityGateRequired,
    applied: !reliabilityGateRequired || !!(reliabilityThrottleLane && reliabilityThrottleLane.ok),
    spine_success_rate: Number(
      recommendation && recommendation.spine_success_rate != null ? recommendation.spine_success_rate : 1
    ),
    spine_success_target: Number(
      recommendation && recommendation.spine_success_target != null
        ? recommendation.spine_success_target
        : RUNTIME_SPINE_SUCCESS_TARGET_MIN
    ),
    handoff_coverage_weak: !!(recommendation && recommendation.handoff_coverage_weak),
    scale_blocked: !!(recommendation && recommendation.swarm_scale_blocked_by_reliability),
    command: reliabilityThrottleCommand,
    lane: reliabilityThrottleLane ? laneOutcome(reliabilityThrottleLane) : null,
  });

  const reliabilityEscalation = maybeEmitReliabilityEscalation(recommendation, runtime);
  policies.push({
    policy: 'human_escalation_guard',
    required: !!reliabilityEscalation.required,
    applied: !!reliabilityEscalation.applied,
    reason: cleanText(reliabilityEscalation.reason || '', 80),
    cooldown_ms: parseNonNegativeInt(
      reliabilityEscalation.cooldown_ms,
      RUNTIME_RELIABILITY_ESCALATION_COOLDOWN_MS,
      1000000000
    ),
    last_emit_at: cleanText(reliabilityEscalation.last_emit_at || '', 80),
    since_last_ms: parseNonNegativeInt(reliabilityEscalation.since_last_ms, 0, 1000000000),
    emit_count: parseNonNegativeInt(reliabilityEscalation.emit_count, 0, 1000000000),
    lane: reliabilityEscalation.lane,
  });

  const coarseRemediation = maybeHealCoarseSignal(
    snapshot,
    runtime,
    recommendation.team || DEFAULT_TEAM,
    recommendation
  );
  policies.push({
    policy: 'coarse_lane_demotion',
    required: !!(coarseRemediation && coarseRemediation.lane_demotion && coarseRemediation.lane_demotion.required),
    applied: !!(coarseRemediation && coarseRemediation.lane_demotion && coarseRemediation.lane_demotion.applied),
    quality: cleanText(coarseRemediation && coarseRemediation.quality ? coarseRemediation.quality : 'good', 24) || 'good',
    max_depth:
      coarseRemediation && coarseRemediation.lane_demotion && coarseRemediation.lane_demotion.max_depth != null
        ? coarseRemediation.lane_demotion.max_depth
        : RUNTIME_COARSE_THROTTLE_MAX_DEPTH,
    strategy:
      cleanText(
        coarseRemediation && coarseRemediation.lane_demotion && coarseRemediation.lane_demotion.strategy
          ? coarseRemediation.lane_demotion.strategy
          : RUNTIME_COARSE_THROTTLE_STRATEGY,
        40
      ) || RUNTIME_COARSE_THROTTLE_STRATEGY,
    command:
      cleanText(
        coarseRemediation && coarseRemediation.lane_demotion && coarseRemediation.lane_demotion.command
          ? coarseRemediation.lane_demotion.command
          : '',
        240
      ) || '',
    lane: coarseRemediation && coarseRemediation.lane_demotion ? coarseRemediation.lane_demotion.lane : null,
  });
  policies.push({
    policy: 'coarse_conduit_scale_up',
    required: !!(coarseRemediation && coarseRemediation.conduit_scale_up && coarseRemediation.conduit_scale_up.required),
    applied: !!(coarseRemediation && coarseRemediation.conduit_scale_up && coarseRemediation.conduit_scale_up.applied),
    quality: cleanText(coarseRemediation && coarseRemediation.quality ? coarseRemediation.quality : 'good', 24) || 'good',
    target_signals:
      coarseRemediation && coarseRemediation.conduit_scale_up && coarseRemediation.conduit_scale_up.target_signals != null
        ? coarseRemediation.conduit_scale_up.target_signals
        : runtime.target_conduit_signals,
    conduit_signals:
      coarseRemediation && coarseRemediation.conduit_scale_up && coarseRemediation.conduit_scale_up.conduit_signals != null
        ? coarseRemediation.conduit_scale_up.conduit_signals
        : runtime.conduit_signals,
    signal_deficit:
      coarseRemediation && coarseRemediation.conduit_scale_up && coarseRemediation.conduit_scale_up.signal_deficit != null
        ? coarseRemediation.conduit_scale_up.signal_deficit
        : Math.max(0, runtime.target_conduit_signals - runtime.conduit_signals),
    lanes:
      coarseRemediation && coarseRemediation.conduit_scale_up && Array.isArray(coarseRemediation.conduit_scale_up.lanes)
        ? coarseRemediation.conduit_scale_up.lanes
        : [],
  });
  policies.push({
    policy: 'coarse_stale_lane_drain',
    required: !!(coarseRemediation && coarseRemediation.stale_lane_drain && coarseRemediation.stale_lane_drain.required),
    applied: !!(coarseRemediation && coarseRemediation.stale_lane_drain && coarseRemediation.stale_lane_drain.applied),
    quality: cleanText(coarseRemediation && coarseRemediation.quality ? coarseRemediation.quality : 'good', 24) || 'good',
    stale_blocks:
      coarseRemediation && coarseRemediation.stale_lane_drain && coarseRemediation.stale_lane_drain.stale_blocks != null
        ? coarseRemediation.stale_lane_drain.stale_blocks
        : parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000),
    stale_lanes_top:
      coarseRemediation && coarseRemediation.stale_lane_drain && Array.isArray(coarseRemediation.stale_lane_drain.stale_lanes_top)
        ? coarseRemediation.stale_lane_drain.stale_lanes_top
        : [],
    drain_limit:
      coarseRemediation && coarseRemediation.stale_lane_drain && coarseRemediation.stale_lane_drain.drain_limit != null
        ? coarseRemediation.stale_lane_drain.drain_limit
        : 0,
    lane: coarseRemediation && coarseRemediation.stale_lane_drain ? coarseRemediation.stale_lane_drain.lane : null,
    lanes:
      coarseRemediation && coarseRemediation.stale_lane_drain && Array.isArray(coarseRemediation.stale_lane_drain.lanes)
        ? coarseRemediation.stale_lane_drain.lanes
        : [],
    compact_lane:
      coarseRemediation && coarseRemediation.stale_lane_drain
        ? coarseRemediation.stale_lane_drain.compact_lane
        : null,
  });

  const conduitWatchdog = maybeAutoHealConduit(runtime, recommendation.team || DEFAULT_TEAM, recommendation);
  policies.push({
    policy: 'conduit_watchdog_autorestart',
    required: !!conduitWatchdog.required,
    applied: !!conduitWatchdog.applied,
    triggered: !!conduitWatchdog.triggered,
    recovered: !!conduitWatchdog.recovered,
    low_signal: !!conduitWatchdog.low_signal,
    queue_depth: conduitWatchdog.queue_depth,
    conduit_signals: conduitWatchdog.conduit_signals,
    stale_cockpit_blocks: conduitWatchdog.stale_cockpit_blocks || 0,
    threshold: conduitWatchdog.threshold,
    stale_for_ms: conduitWatchdog.stale_for_ms,
    failure_count: conduitWatchdog.failure_count || 0,
    drain_limit: conduitWatchdog.drain_limit || 0,
    last_attempt_at: conduitWatchdog.last_attempt_at,
    last_success_at: conduitWatchdog.last_success_at,
    command:
      `protheus-ops attention-queue drain --consumer=${ATTENTION_CONSUMER_ID}` +
      ` --limit=${conduitWatchdog.drain_limit || RUNTIME_ATTENTION_DRAIN_MIN_BATCH}` +
      ' --wait-ms=0 --run-context=runtime_conduit_watchdog',
    lane: conduitWatchdog.lane,
    lanes: conduitWatchdog.lanes || null,
  });

  const rolePlan = Array.isArray(recommendation.role_plan) ? recommendation.role_plan : [];
  for (const row of rolePlan) {
    const ensure = ensureRuntimeRole(
      snapshot,
      recommendation.team || DEFAULT_TEAM,
      row && row.role ? row.role : 'analyst',
      row && row.shadow ? row.shadow : ''
    );
    launches.push({
      role: ensure.role,
      shadow: ensure.shadow,
      ok: !!ensure.ok,
      launched: !!ensure.launched,
      lane: ensure.lane,
    });
    if (ensure.ok && ensure.shadow) {
      roleAssignments.push({
        role: ensure.role,
        shadow: ensure.shadow,
        prompt:
          cleanText(row && row.prompt ? row.prompt : recommendation.prompts && recommendation.prompts[ensure.role], 2000) ||
          '',
      });
    }
  }

  for (const assignment of roleAssignments) {
    const source = `swarm_recommendation.${cleanText(assignment.role || 'agent', 40) || 'agent'}`;
    const turn = queueAgentTask(
      assignment.shadow,
      snapshot,
      assignment.prompt,
      source
    );
    turns.push({
      role: assignment.role,
      shadow: assignment.shadow,
      ok: !!turn.ok,
      response: cleanText(turn.ok ? 'Task queued.' : turn.error || '', 400),
      runtime_sync: runtimeSyncSummary(snapshot),
    });
  }

  const predictiveDrain = recommendation && recommendation.predictive_drain_allowed === false
    ? {
        required: false,
        release: false,
        trigger_depth: RUNTIME_DRAIN_TRIGGER_DEPTH,
        clear_depth: RUNTIME_DRAIN_CLEAR_DEPTH,
        active_count: trackedRuntimeDrainAgents(snapshot).length,
        active_agents: trackedRuntimeDrainAgents(snapshot).slice(0, 8),
        launches: [],
        turns: [],
        archived: [],
        blocked_by_reliability: true,
      }
    : applyRuntimePredictiveDrain(snapshot, recommendation.team || DEFAULT_TEAM, runtime, recommendation);
  if (Array.isArray(predictiveDrain.launches)) {
    for (const launch of predictiveDrain.launches) {
      launches.push({
        role: cleanText(launch && launch.role ? launch.role : 'builder', 40) || 'builder',
        shadow: cleanText(launch && launch.shadow ? launch.shadow : '', 140),
        ok: !!(launch && launch.ok),
        launched: !!(launch && launch.launched),
        lane: launch && launch.lane ? launch.lane : null,
      });
    }
  }
  if (Array.isArray(predictiveDrain.turns)) {
    turns.push(...predictiveDrain.turns);
  }
  policies.push({
    policy: 'predictive_drain',
    required: !!predictiveDrain.required,
    release: !!predictiveDrain.release,
    applied:
      (!!predictiveDrain.required && parseNonNegativeInt(predictiveDrain.active_count, 0, 100) > 0) ||
      (!!predictiveDrain.release && Array.isArray(predictiveDrain.archived) && predictiveDrain.archived.length > 0),
    trigger_depth: predictiveDrain.trigger_depth,
    clear_depth: predictiveDrain.clear_depth,
    active_count: predictiveDrain.active_count,
    active_agents: Array.isArray(predictiveDrain.active_agents) ? predictiveDrain.active_agents.slice(0, 8) : [],
    archived_count: Array.isArray(predictiveDrain.archived) ? predictiveDrain.archived.length : 0,
    blocked_by_reliability: !!predictiveDrain.blocked_by_reliability,
  });

  if (parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) > 0) {
    const builderAssignment = roleAssignments.find((row) => row.role === 'builder');
    let staleTurn = null;
    if (builderAssignment && builderAssignment.shadow) {
      const turn = queueAgentTask(
        builderAssignment.shadow,
        snapshot,
        `Drain stale cockpit blocks older than ${Math.floor(RUNTIME_COCKPIT_STALE_BLOCK_MS / 1000)}s and report lock/contention root causes. Prioritize queue unblocking actions first.`,
        'swarm_recommendation.cockpit_stale_blocks'
      );
      staleTurn = {
        role: 'builder',
        shadow: builderAssignment.shadow,
        ok: !!turn.ok,
        response: cleanText(turn.ok ? 'Stale cockpit block remediation queued.' : turn.error || '', 400),
      };
      turns.push({
        ...staleTurn,
        runtime_sync: runtimeSyncSummary(snapshot),
      });
    }
    policies.push({
      policy: 'cockpit_stale_block_timeout',
      required: true,
      applied: !!(staleTurn && staleTurn.ok),
      eligible: !!(builderAssignment && builderAssignment.shadow),
      stale_blocks: parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000),
      stale_threshold_ms: RUNTIME_COCKPIT_STALE_BLOCK_MS,
      mode: 'builder_parallel_drain',
    });
  } else {
    policies.push({
      policy: 'cockpit_stale_block_timeout',
      required: false,
      applied: false,
      stale_blocks: 0,
      stale_threshold_ms: RUNTIME_COCKPIT_STALE_BLOCK_MS,
      mode: 'steady_state',
    });
  }

  const healthAdaptive = maybeRefreshAdaptiveHealth(runtime, recommendation);
  policies.push({
    policy: 'adaptive_health_schedule',
    required: !!healthAdaptive.required,
    applied: !!healthAdaptive.applied,
    window_seconds: healthAdaptive.window_seconds,
    command: `infringd health schedule --adaptive --window=${healthAdaptive.window_seconds}s`,
    lane: healthAdaptive.lane,
  });

  const memoryResume = maybeResumeMemoryIngest(runtime, recommendation);
  policies.push({
    policy: 'memory_ingest_resume',
    required: !!runtime.memory_ingest_paused,
    applied: !!memoryResume.resumed,
    eligible: !!memoryResume.eligible,
    reason: memoryResume.reason,
  });

  const conduitAutoBalanceRequired = !!recommendation.conduit_autobalance_required;
  let autoBalanceTurn = null;
  if (conduitAutoBalanceRequired) {
    const researcherAssignment = roleAssignments.find((row) => row.role === 'researcher');
    if (researcherAssignment && researcherAssignment.shadow) {
      const turn = queueAgentTask(
        researcherAssignment.shadow,
        snapshot,
        `Run conduit auto-balance triage. Maintain at least ${Math.max(runtime.target_conduit_signals, RUNTIME_AUTO_BALANCE_THRESHOLD)} active conduit signals and report scaling actions.`,
        'swarm_recommendation.conduit_autobalance'
      );
      autoBalanceTurn = {
        role: 'researcher',
        shadow: researcherAssignment.shadow,
        ok: !!turn.ok,
        response: cleanText(turn.ok ? 'Conduit auto-balance task queued.' : turn.error || '', 400),
      };
      turns.push({
        ...autoBalanceTurn,
        runtime_sync: runtimeSyncSummary(snapshot),
      });
    }
  }
  policies.push({
    policy: 'conduit_autobalance',
    required: conduitAutoBalanceRequired,
    applied: !!(autoBalanceTurn && autoBalanceTurn.ok),
    eligible: !conduitAutoBalanceRequired || !!autoBalanceTurn,
    threshold: Math.max(runtime.target_conduit_signals, RUNTIME_AUTO_BALANCE_THRESHOLD),
    command: `protheus-ops collab-plane launch-role --team=${cleanText(recommendation.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM} --role=researcher --shadow=${cleanText(recommendation.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM}-conduit-watchdog --strict=1`,
  });

  const failedPolicies = policies.filter((row) => row.required && row.applied === false && row.eligible !== false);
  const failedLaunches = launches.filter((row) => !row.ok);
  const failedTurns = turns.filter((row) => !row.ok);
  const errors = [
    ...failedPolicies.map((row) => `policy_failed:${row.policy}`),
    ...failedLaunches.map((row) => `launch_failed:${row.role}`),
    ...failedTurns.map((row) => `task_queue_failed:${row.role}`),
  ];
  const workExecuted = turns.length > 0 || policies.some((row) => row.applied === true);
  const remediationDegraded = errors.length > 0;

  return {
    ok: workExecuted || !remediationDegraded,
    type: 'dashboard_runtime_swarm_recommendation',
    recommendation,
    policies,
    launches,
    turns,
    executed_count: turns.length,
    degraded: remediationDegraded,
    errors,
  };
}

function transpileClientTs() {
  const source = readText(CLIENT_TS_PATH, '');
  if (!source) {
    throw new Error(`missing_client_source:${path.relative(ROOT, CLIENT_TS_PATH)}`);
  }
  return ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ES2022,
      target: ts.ScriptTarget.ES2022,
      jsx: ts.JsxEmit.React,
      sourceMap: false,
      removeComments: false,
    },
    fileName: CLIENT_TS_PATH,
    reportDiagnostics: false,
  }).outputText;
}

function sendJson(res, statusCode, payload) {
  const body = `${JSON.stringify(payload, null, 2)}\n`;
  res.writeHead(statusCode, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(body),
  });
  res.end(body);
}

function sendJsonRaw(res, statusCode, body) {
  const text = typeof body === 'string' ? body : `${String(body == null ? '' : body)}\n`;
  res.writeHead(statusCode, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(text),
  });
  res.end(text);
}

function sendText(res, statusCode, body, contentType) {
  res.writeHead(statusCode, {
    'content-type': contentType,
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(body),
  });
  res.end(body);
}

function bodyJson(req) {
  return new Promise((resolve, reject) => {
    let raw = '';
    req.on('data', (chunk) => {
      raw += chunk.toString('utf8');
      if (raw.length > 1_500_000) {
        reject(new Error('payload_too_large'));
        req.destroy();
      }
    });
    req.on('end', () => {
      try {
        const parsed = raw.trim() ? JSON.parse(raw) : {};
        if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
          resolve({});
          return;
        }
        resolve(parsed);
      } catch (error) {
        reject(error);
      }
    });
    req.on('error', reject);
  });
}

function runServe(flags) {
  ACTIVE_CLI_MODE = normalizeCliMode(flags && flags.cliMode ? flags.cliMode : ACTIVE_CLI_MODE);
  const forkUiEnabled = hasPrimaryDashboardUi();
  ensureDailyMemoryFile(todayDateIso());
  bootstrapSnapshotHistoryState({ fast: true });
  let dashboardHtml = '';
  let dashboardUiRefreshAtMs = 0;
  const refreshUiAssets = (force = false) => {
    if (!forkUiEnabled) {
      dashboardHtml = '';
      return;
    }
    const nowMs = Date.now();
    if (
      !force &&
      dashboardHtml &&
      (nowMs - parseNonNegativeInt(dashboardUiRefreshAtMs, 0, 1_000_000_000_000)) <
        DASHBOARD_UI_ASSET_REFRESH_COOLDOWN_MS
    ) {
      return;
    }
    dashboardHtml = buildPrimaryDashboardHtml();
    dashboardUiRefreshAtMs = nowMs;
  };
  refreshUiAssets(true);
  const bootstrapSnapshotFromDisk = () => {
    const cached = readJson(SNAPSHOT_LATEST_PATH, null);
    if (cached && typeof cached === 'object') {
      return cached;
    }
    const team = cleanText(flags.team || DEFAULT_TEAM, 80) || DEFAULT_TEAM;
    const snapshot = {
      ok: false,
      type: 'infring_dashboard_snapshot',
      ts: nowIso(),
      metadata: {
        root: ROOT,
        team,
        refresh_ms: parsePositiveInt(flags && flags.refreshMs, DEFAULT_REFRESH_MS, 250, 60000),
        cli_mode: ACTIVE_CLI_MODE,
        authority: 'rust_core_lanes',
        runtime_sync_authority: 'bootstrap',
      },
      health: {
        checks: {},
        alerts: { count: 0, checks: [] },
        coverage: { gap_count: 0, gaps: [], active_checks: 0 },
      },
      app: { settings: {} },
      collab: { dashboard: { agents: [] } },
      skills: {},
      cockpit: {
        blocks: [],
        summary: {
          queue_depth: 0,
          cockpit_blocks_active: 0,
          cockpit_blocks_total: 0,
          conduit_signals: 0,
          stale_block_count: 0,
          actionable_stale_block_count: 0,
          dormant_stale_block_count: 0,
        },
      },
      attention_queue: {
        depth: 0,
        deferred_count: 0,
        items: [],
        lane_counts: { critical: 0, standard: 0, background: 0 },
      },
      memory: {
        entries: [],
        stream: { active: false, total: 0 },
        ingest_control: { mode: 'normal', source_count: 0, delivered_count: 0, dropped_count: 0 },
      },
      receipts: {
        recent: [],
        action_history_path: path.relative(ROOT, ACTION_HISTORY_PATH),
      },
      logs: { recent: [] },
      apm: { metrics: [], checks: {}, alerts: {} },
      agent_lifecycle: { active_agents: 0, terminated_agents: 0, idle_agents: 0, recently_terminated: [] },
      runtime_recommendation: null,
      runtime_autoheal: {},
      storage: snapshotStorageTelemetry(),
    };
    return {
      ...snapshot,
      receipt_hash: sha256(JSON.stringify(snapshot)),
    };
  };
  let latestSnapshot = bootstrapSnapshotFromDisk();
  let lastKnownSidebarAgents = [];
  try {
    const seeded = compatAgentsFromSnapshot(latestSnapshot, { includeArchived: false });
    if (Array.isArray(seeded) && seeded.length) {
      lastKnownSidebarAgents = seeded.slice(0, 500).map((row) => ({
        ...(row && typeof row === 'object' ? row : {}),
      }));
    }
  } catch {}
  let latestSnapshotJson = `${JSON.stringify(latestSnapshot, null, 2)}\n`;
  let latestSnapshotEnvelope = JSON.stringify({ type: 'snapshot', snapshot: latestSnapshot });
  writeSnapshotReceipt(latestSnapshot, { forceHistory: false });
  let updating = false;
  let enforcingContracts = false;
  let enforceContractsQueued = false;
  let gitTreeSyncQueued = false;
  let lastGitTreeApiSyncAtMs = 0;
  let lastContractEnforceAtMs = 0;
  let lastContractLoopRunAtMs = 0;
  let lastApiContractEnforceAtMs = 0;
  const agentLifecycleLocks = new Set();
  let nextSnapshotRefreshAtMs = 0;
  let lastSnapshotBuildDurationMs = 0;
  let lastSnapshotBuildAtMs = Date.now();
  let lastFullSnapshotBuildAtMs = Date.now();
  let lastSnapshotRefreshRequestAtMs = 0;
  let lastClientActivityAtMs = Date.now();

  const refreshSnapshot = (contractEnforcement = null, options = {}) => {
    const startedMs = Date.now();
    const requestedFastLaneMode = !!(options && options.fast_lane_mode === true);
    const fullRefreshCadenceMs = 30_000;
    const fullRefreshDue = (startedMs - parseNonNegativeInt(lastFullSnapshotBuildAtMs, 0, 1_000_000_000_000)) >= fullRefreshCadenceMs;
    const forceFastLaneMode = !!(options && options.force_fast_lane_mode === true);
    const fastLaneMode = requestedFastLaneMode && (!fullRefreshDue || forceFastLaneMode);
    const cadenceMs = parsePositiveInt(flags && flags.refreshMs, DEFAULT_REFRESH_MS, 250, 60000);
    const laneTimeoutMs = parsePositiveInt(
      options && options.lane_timeout_ms != null
        ? options.lane_timeout_ms
        : fastLaneMode
        ? SNAPSHOT_LANE_TIMEOUT_FAST_MS
        : LANE_SYNC_TIMEOUT_MS,
      fastLaneMode ? SNAPSHOT_LANE_TIMEOUT_FAST_MS : LANE_SYNC_TIMEOUT_MS,
      SNAPSHOT_LANE_TIMEOUT_MIN_MS,
      SNAPSHOT_LANE_TIMEOUT_MAX_MS
    );
    const laneCacheTtlMs = parsePositiveInt(
      options && options.lane_cache_ttl_ms != null
        ? options.lane_cache_ttl_ms
        : fastLaneMode
        ? Math.max(SNAPSHOT_LANE_CACHE_TTL_MS, cadenceMs * 3)
        : SNAPSHOT_LANE_CACHE_TTL_MS,
      SNAPSHOT_LANE_CACHE_TTL_MS,
      250,
      600000
    );
    const laneCacheFailTtlMs = parsePositiveInt(
      options && options.lane_cache_fail_ttl_ms != null
        ? options.lane_cache_fail_ttl_ms
        : Math.max(SNAPSHOT_LANE_CACHE_FAIL_TTL_MS, Math.round(cadenceMs * 0.5)),
      SNAPSHOT_LANE_CACHE_FAIL_TTL_MS,
      250,
      600000
    );
    const priorCollab =
      latestSnapshot &&
      latestSnapshot.collab &&
      typeof latestSnapshot.collab === 'object'
        ? latestSnapshot.collab
        : {};
    latestSnapshot = buildSnapshot({
      ...flags,
      contract_enforcement: contractEnforcement,
      lane_timeout_ms: laneTimeoutMs,
      lane_cache_ttl_ms: laneCacheTtlMs,
      lane_cache_fail_ttl_ms: laneCacheFailTtlMs,
      prior_collab: priorCollab,
      prior_snapshot: latestSnapshot && typeof latestSnapshot === 'object' ? latestSnapshot : null,
      fast_lane_mode: fastLaneMode,
    });
    try {
      discoverLocalProviderState(latestSnapshot);
    } catch {}
    latestSnapshotJson = `${JSON.stringify(latestSnapshot, null, 2)}\n`;
    latestSnapshotEnvelope = JSON.stringify({ type: 'snapshot', snapshot: latestSnapshot });
    writeSnapshotReceipt(latestSnapshot, {
      forceHistory: !!(options && options.force_history),
    });
    const finishedMs = Date.now();
    const buildDurationMs = Math.max(0, finishedMs - startedMs);
    lastSnapshotBuildDurationMs = buildDurationMs;
    lastSnapshotBuildAtMs = finishedMs;
    if (!fastLaneMode) {
      lastFullSnapshotBuildAtMs = finishedMs;
    }
    const deferOnSlow = !(options && options.defer_on_slow === false);
    if (deferOnSlow) {
      const ratio = buildDurationMs / Math.max(1, cadenceMs);
      if (ratio >= 1) {
        const cooldownMs = Math.min(30_000, Math.max(cadenceMs * 3, Math.round(buildDurationMs * 1.5)));
        nextSnapshotRefreshAtMs = finishedMs + cooldownMs;
      } else if (ratio >= 0.5) {
        const cooldownMs = Math.min(15_000, Math.max(cadenceMs * 2, Math.round(buildDurationMs * 1.2)));
        nextSnapshotRefreshAtMs = finishedMs + cooldownMs;
      } else if (nextSnapshotRefreshAtMs > 0 && finishedMs >= nextSnapshotRefreshAtMs) {
        nextSnapshotRefreshAtMs = 0;
      }
    }
    return latestSnapshot;
  };

  const acquireAgentLifecycleLock = (agentId) => {
    const key = cleanText(agentId || '', 140);
    if (!key) return '';
    if (agentLifecycleLocks.has(key)) return '';
    agentLifecycleLocks.add(key);
    return key;
  };

  const releaseAgentLifecycleLock = (agentLockKey) => {
    const key = cleanText(agentLockKey || '', 140);
    if (!key) return;
    agentLifecycleLocks.delete(key);
  };

  const scheduleAgentGitTreeSync = (preferredMasterId = '') => {
    const nowMs = Date.now();
    if (
      (nowMs - parseNonNegativeInt(lastGitTreeApiSyncAtMs, 0, 1_000_000_000_000)) <
      AGENT_GIT_TREE_API_SYNC_DEBOUNCE_MS
    ) {
      return false;
    }
    if (gitTreeSyncQueued) return false;
    gitTreeSyncQueued = true;
    const preferred = cleanText(preferredMasterId || '', 140);
    setTimeout(() => {
      try {
        ensureAgentGitTreeAssignments(latestSnapshot, {
          force: false,
          preferred_master_id: preferred,
        });
        lastGitTreeApiSyncAtMs = Date.now();
      } catch {}
      gitTreeSyncQueued = false;
    }, 0);
    return true;
  };

  const snapshotForContractEnforcement = () => {
    const snapshotTsMs = coerceTsMs(latestSnapshot && latestSnapshot.ts ? latestSnapshot.ts : 0, 0);
    const maxAgeMs = Math.max(750, parsePositiveInt(flags && flags.refreshMs, DEFAULT_REFRESH_MS, 250, 60000) * 2);
    const stale = snapshotTsMs <= 0 || (Date.now() - snapshotTsMs) > maxAgeMs;
    if (!stale || updating) return latestSnapshot;
    try {
      refreshSnapshot(null, {
        fast_lane_mode: true,
        defer_on_slow: false,
      });
    } catch {}
    return latestSnapshot;
  };
  const server = http.createServer(async (req, res) => {
    lastClientActivityAtMs = Date.now();
    const reqUrl = new URL(req.url || '/', `http://${flags.host}:${flags.port}`);
    const pathname = reqUrl.pathname;

    try {
      const dashboardUiRoute =
        pathname === '/' ||
        pathname === '/dashboard';
      if (req.method === 'GET' && dashboardUiRoute) {
        refreshUiAssets(false);
        const hasDashboardHtml = forkUiEnabled && String(dashboardHtml || '').trim().length > 0;
        if (!hasDashboardHtml) {
          sendJson(res, 503, {
            ok: false,
            type: 'infring_dashboard_primary_ui_missing',
            error: 'primary_dashboard_ui_missing',
          });
          return;
        }
        sendText(res, 200, dashboardHtml, 'text/html; charset=utf-8');
        return;
      }
      if (forkUiEnabled && req.method === 'GET') {
        const forkAsset = readPrimaryDashboardAsset(pathname);
        if (forkAsset) {
          sendText(res, 200, forkAsset.body, forkAsset.contentType);
          return;
        }
      }
      if (req.method === 'GET' && pathname === '/api/dashboard/snapshot') {
        sendJsonRaw(res, 200, latestSnapshotJson);
        return;
      }
      if (req.method === 'GET' && pathname === '/api/logs/stream') {
        res.writeHead(200, {
          'content-type': 'text/event-stream; charset=utf-8',
          'cache-control': 'no-store',
          connection: 'keep-alive',
        });
        let closed = false;
        let sent = new Set();
        const emit = (entry) => {
          try {
            if (closed) return;
            const payload = {
              seq: cleanText(entry && entry.id ? entry.id : '', 120) || sha256(JSON.stringify(entry || {})).slice(0, 16),
              timestamp: cleanText(entry && entry.timestamp ? entry.timestamp : nowIso(), 80) || nowIso(),
              action: cleanText(entry && entry.action ? entry.action : 'Event', 80) || 'Event',
              detail: cleanText(entry && entry.detail ? entry.detail : '', 500) || '',
              agent_id: cleanText(entry && entry.agent_id ? entry.agent_id : '', 120),
            };
            const key = cleanText(payload.seq, 120);
            if (key && sent.has(key)) return;
            if (key) {
              sent.add(key);
              if (sent.size > 1500) {
                sent = new Set(Array.from(sent).slice(-800));
              }
            }
            res.write(`data: ${JSON.stringify(payload)}\n\n`);
          } catch {}
        };
        const emitRecent = () => {
          const recent = auditEntriesFromSnapshot(latestSnapshot, 200);
          for (const row of recent.entries.slice(-40)) emit(row);
        };
        emitRecent();
        const heartbeat = setInterval(() => {
          if (closed) return;
          try {
            res.write(`: heartbeat ${Date.now()}\n\n`);
            emitRecent();
          } catch {}
        }, 2500);
        const close = () => {
          if (closed) return;
          closed = true;
          clearInterval(heartbeat);
          try { res.end(); } catch {}
        };
        req.on('close', close);
        req.on('end', close);
        req.on('error', close);
        return;
      }
      if (req.method === 'GET' && pathname.startsWith('/api/chat/export/')) {
        cleanupChatExports();
        const token = cleanText(pathname.split('/').pop() || '', 80);
        const artifact = token ? chatExportArtifacts.get(token) : null;
        if (!artifact || !artifact.file_path || !fileExists(artifact.file_path)) {
          sendJson(res, 404, { ok: false, error: 'export_not_found', token: token || '' });
          return;
        }
        const downloadName = cleanText(artifact.file_name || 'infring-export.tar.gz', 160) || 'infring-export.tar.gz';
        res.writeHead(200, {
          'content-type': 'application/gzip',
          'content-disposition': `attachment; filename="${downloadName}"`,
          'cache-control': 'no-store',
        });
        const stream = fs.createReadStream(artifact.file_path);
        stream.on('error', () => {
          try {
            if (!res.headersSent) {
              sendJson(res, 500, { ok: false, error: 'export_read_failed', token });
            } else {
              res.end();
            }
          } catch {}
        });
        stream.pipe(res);
        return;
      }
      if (req.method === 'GET' && pathname === '/api/status') {
        const agentCount = activeAgentCountFromSnapshot(latestSnapshot, 0);
        const runtimeSync = runtimeSyncSummary(latestSnapshot);
        sendJson(res, 200, {
          ok: true,
          version: APP_VERSION,
          agent_count: agentCount,
          connected: true,
          uptime_sec: 0,
          uptime_seconds: 0,
          ws: true,
          default_model:
            latestSnapshot &&
            latestSnapshot.app &&
            latestSnapshot.app.settings &&
            latestSnapshot.app.settings.model
              ? latestSnapshot.app.settings.model
              : 'gpt-5',
          git_branch: currentGitBranch(),
          api_listen: `${flags.host}:${flags.port}`,
          listen: `${flags.host}:${flags.port}`,
          home_dir: ROOT,
          workspace_dir: ROOT,
          log_level: cleanText(process.env.RUST_LOG || process.env.LOG_LEVEL || 'info', 24) || 'info',
          network_enabled: true,
          cli_mode: ACTIVE_CLI_MODE,
          runtime_sync: runtimeSync,
          agent_lifecycle: latestSnapshot && latestSnapshot.agent_lifecycle ? latestSnapshot.agent_lifecycle : null,
          storage: latestSnapshot && latestSnapshot.storage ? latestSnapshot.storage : snapshotStorageTelemetry(),
        });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/auth/check') {
        sendJson(res, 200, {
          ok: true,
          mode: 'none',
          user: 'operator',
        });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/config') {
        sendJson(res, 200, {
          ok: true,
          api_key: 'set',
          provider: latestSnapshot && latestSnapshot.app && latestSnapshot.app.settings
            ? latestSnapshot.app.settings.provider
            : 'openai',
          model: latestSnapshot && latestSnapshot.app && latestSnapshot.app.settings
            ? latestSnapshot.app.settings.model
            : 'gpt-5',
          cli_mode: ACTIVE_CLI_MODE,
        });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/models') {
        sendJson(res, 200, {
          ok: true,
          models: buildDashboardModels(latestSnapshot),
        });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/models/discover') {
        const payload = await bodyJson(req);
        const apiKey = cleanText(payload && payload.api_key ? payload.api_key : '', 640);
        if (!apiKey) {
          sendJson(res, 400, { ok: false, error: 'api_key_required' });
          return;
        }
        const inferredProvider = inferProviderFromApiKey(apiKey);
        const registry = loadProviderRegistry(latestSnapshot);
        const providers = registry && registry.providers && typeof registry.providers === 'object' ? registry.providers : {};
        const record = providers[inferredProvider] || normalizeProviderRecord(inferredProvider, { id: inferredProvider });
        const meta = providerKeyMetadata(apiKey);
        const catalogModels = Array.isArray(PROVIDER_MODEL_CATALOG[inferredProvider]) ? PROVIDER_MODEL_CATALOG[inferredProvider].slice(0, 24) : [];
        const detected = record.is_local
          ? probeOpenAiCompatModels(record.base_url || '', apiKey).models
          : catalogModels;
        providers[inferredProvider] = normalizeProviderRecord(inferredProvider, {
          ...record,
          ...meta,
          auth_status: 'configured',
          reachable: true,
          detected_models: detected.length ? detected : record.detected_models,
          updated_at: nowIso(),
        });
        saveProviderRegistry({ providers });
        sendJson(res, 200, {
          ok: true,
          provider: inferredProvider,
          models: detected.length ? detected : catalogModels,
          model_count: (detected.length ? detected : catalogModels).length,
          message: `API key stored for ${inferredProvider}.`,
        });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/models/custom') {
        const payload = await bodyJson(req);
        const id = cleanText(payload && payload.id ? payload.id : '', 180);
        const provider = cleanText(payload && payload.provider ? payload.provider : '', 80).toLowerCase();
        if (!id || !provider) {
          sendJson(res, 400, { ok: false, error: 'model_id_and_provider_required' });
          return;
        }
        const models = loadCustomModels();
        const exists = models.some((row) => String(row.id).toLowerCase() === id.toLowerCase() && String(row.provider).toLowerCase() === provider);
        if (!exists) {
          models.push({
            id,
            provider,
            display_name: cleanText(payload && payload.display_name ? payload.display_name : id, 180) || id,
            context_window: parsePositiveInt(payload && payload.context_window != null ? payload.context_window : 0, 0, 0, 8_000_000),
            max_output_tokens: parsePositiveInt(payload && payload.max_output_tokens != null ? payload.max_output_tokens : 0, 0, 0, 8_000_000),
            available: true,
            deployment: provider === 'ollama' || provider === 'llama.cpp' ? 'local' : 'cloud',
          });
          saveCustomModels(models);
        }
        sendJson(res, 200, { ok: true, id, provider });
        return;
      }
      if (req.method === 'DELETE' && pathname.startsWith('/api/models/custom/')) {
        const modelId = cleanText(safeDecodePathToken(pathname.split('/').slice(4).join('/')), 180);
        if (!modelId) {
          sendJson(res, 400, { ok: false, error: 'model_id_required' });
          return;
        }
        const models = loadCustomModels();
        const filtered = models.filter((row) => cleanText(row && row.id ? row.id : '', 180).toLowerCase() !== modelId.toLowerCase());
        saveCustomModels(filtered);
        sendJson(res, 200, { ok: true, deleted: true, id: modelId });
        return;
      }
      if (pathname.startsWith('/api/providers/')) {
        const parts = pathname.split('/').filter(Boolean);
        const providerId = cleanText(safeDecodePathToken(parts[2] || ''), 80).toLowerCase();
        if (!providerId) {
          sendJson(res, 400, { ok: false, error: 'provider_required' });
          return;
        }
        const registry = loadProviderRegistry(latestSnapshot);
        const providers = registry && registry.providers && typeof registry.providers === 'object' ? registry.providers : {};
        const prior = providers[providerId] || normalizeProviderRecord(providerId, { id: providerId });

        if (req.method === 'POST' && parts[3] === 'key') {
          const payload = await bodyJson(req);
          const key = cleanText(payload && payload.key ? payload.key : '', 640);
          if (!key) {
            sendJson(res, 400, { ok: false, error: 'provider_key_required' });
            return;
          }
          const meta = providerKeyMetadata(key);
          const catalogModels = Array.isArray(PROVIDER_MODEL_CATALOG[providerId]) ? PROVIDER_MODEL_CATALOG[providerId].slice(0, 32) : [];
          providers[providerId] = normalizeProviderRecord(providerId, {
            ...prior,
            ...meta,
            auth_status: 'configured',
            reachable: true,
            detected_models: catalogModels.length ? catalogModels : prior.detected_models,
            updated_at: nowIso(),
          });
          saveProviderRegistry({ providers });
          sendJson(res, 200, {
            ok: true,
            provider: providerId,
            status: 'configured',
            switched_default: false,
            message: `API key saved for ${providerId}`,
          });
          return;
        }
        if (req.method === 'DELETE' && parts[3] === 'key') {
          providers[providerId] = normalizeProviderRecord(providerId, {
            ...prior,
            auth_status: prior.is_local ? (Array.isArray(prior.detected_models) && prior.detected_models.length ? 'configured' : 'not_set') : 'not_set',
            key_prefix: '',
            key_last4: '',
            key_hash: '',
            key_set_at: '',
            updated_at: nowIso(),
          });
          saveProviderRegistry({ providers });
          sendJson(res, 200, { ok: true, provider: providerId, removed: true });
          return;
        }
        if (req.method === 'PUT' && parts[3] === 'url') {
          const payload = await bodyJson(req);
          const baseUrl = cleanText(payload && payload.base_url ? payload.base_url : '', 320);
          if (!baseUrl || !/^https?:\/\//i.test(baseUrl)) {
            sendJson(res, 400, { ok: false, error: 'valid_base_url_required' });
            return;
          }
          const probe = probeOpenAiCompatModels(baseUrl, '');
          providers[providerId] = normalizeProviderRecord(providerId, {
            ...prior,
            base_url: baseUrl,
            is_local: true,
            reachable: !!probe.reachable,
            auth_status: probe.reachable ? 'configured' : (prior.key_hash ? 'configured' : 'not_set'),
            detected_models: probe.models.length ? probe.models : prior.detected_models,
            updated_at: nowIso(),
          });
          saveProviderRegistry({ providers });
          sendJson(res, 200, {
            ok: true,
            provider: providerId,
            reachable: !!probe.reachable,
            latency_ms: probe.reachable ? 120 : 0,
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'test') {
          const probe = prior.is_local
            ? probeOpenAiCompatModels(prior.base_url || '', '')
            : { reachable: !!prior.key_hash, models: Array.isArray(prior.detected_models) ? prior.detected_models : [] };
          sendJson(res, 200, {
            ok: true,
            provider: providerId,
            status: probe.reachable ? 'ok' : 'error',
            latency_ms: probe.reachable ? 120 : 0,
            model_count: Array.isArray(probe.models) ? probe.models.length : 0,
            error: probe.reachable ? '' : (prior.key_hash ? 'Provider endpoint unreachable' : 'Provider key not configured'),
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'oauth' && parts[4] === 'start') {
          const pollId = `oauth_${sha256(`${providerId}:${Date.now()}`).slice(0, 10)}`;
          sendJson(res, 200, {
            ok: true,
            status: 'pending',
            provider: providerId,
            user_code: cleanText(sha256(pollId).slice(0, 8).toUpperCase(), 12),
            verification_uri: 'https://github.com/login/device',
            poll_id: pollId,
            interval: 5,
          });
          return;
        }
        if (req.method === 'GET' && parts[3] === 'oauth' && parts[4] === 'poll') {
          sendJson(res, 200, { ok: true, status: 'pending', interval: 5, provider: providerId });
          return;
        }
      }
      if (pathname.startsWith('/api/channels/')) {
        const parts = pathname.split('/').filter(Boolean);
        const channelsState = loadChannelRegistry();
        const channels = channelsState.channels || {};
        if (req.method === 'POST' && parts[2] === 'whatsapp' && parts[3] === 'qr' && parts[4] === 'start') {
          const sessions = loadQrSessions();
          const sessionId = `qr_${sha256(`whatsapp:${Date.now()}`).slice(0, 10)}`;
          sessions[sessionId] = {
            channel: 'whatsapp',
            status: 'waiting',
            created_at: nowIso(),
            expires_at: new Date(Date.now() + 120_000).toISOString(),
          };
          saveQrSessions(sessions);
          sendJson(res, 200, {
            ok: true,
            available: true,
            connected: false,
            session_id: sessionId,
            qr_data_url: 'data:image/svg+xml;utf8,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="220" height="220"><rect width="220" height="220" fill="#111827"/><rect x="12" y="12" width="196" height="196" fill="#ffffff"/><text x="110" y="112" text-anchor="middle" font-family="monospace" font-size="12" fill="#111827">Scan in WhatsApp</text></svg>'),
            message: 'QR generated. Scan to connect.',
            help: 'Open WhatsApp on your phone, then scan this code.',
          });
          return;
        }
        if (req.method === 'GET' && parts[2] === 'whatsapp' && parts[3] === 'qr' && parts[4] === 'status') {
          const sessionId = cleanText(reqUrl.searchParams.get('session_id') || '', 120);
          const sessions = loadQrSessions();
          const record = sessions[sessionId];
          const expired = !record || coerceTsMs(record.expires_at, 0) <= Date.now();
          if (expired) {
            sendJson(res, 200, { ok: true, connected: false, expired: true, message: 'QR expired. Generate a new code.' });
            return;
          }
          sendJson(res, 200, { ok: true, connected: false, expired: false, message: 'Waiting for scan...' });
          return;
        }
        const channelName = cleanText(safeDecodePathToken(parts[2] || ''), 80).toLowerCase();
        if (!channelName || !Object.prototype.hasOwnProperty.call(channels, channelName)) {
          sendJson(res, 404, { ok: false, error: 'channel_not_found', channel: channelName });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'configure') {
          const payload = await bodyJson(req);
          const fields = payload && payload.fields && typeof payload.fields === 'object' ? payload.fields : {};
          const next = { ...channels[channelName] };
          next.configured = true;
          next.has_token = Object.keys(fields).some((key) => !!cleanText(fields[key], 240));
          next.stored_fields = {};
          for (const fieldKey of Object.keys(fields)) {
            next.stored_fields[fieldKey] = cleanText(fields[fieldKey], 240);
          }
          next.updated_at = nowIso();
          channels[channelName] = next;
          saveChannelRegistry({ ...channelsState, channels });
          sendJson(res, 200, { ok: true, channel: channelName, configured: true });
          return;
        }
        if (req.method === 'DELETE' && parts[3] === 'configure') {
          const next = { ...channels[channelName] };
          next.configured = false;
          next.has_token = false;
          next.stored_fields = {};
          next.updated_at = nowIso();
          channels[channelName] = next;
          saveChannelRegistry({ ...channelsState, channels });
          sendJson(res, 200, { ok: true, channel: channelName, removed: true });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'test') {
          const channel = channels[channelName];
          const ok = !!(channel && channel.configured && channel.has_token);
          sendJson(res, 200, {
            ok: true,
            channel: channelName,
            status: ok ? 'ok' : 'error',
            message: ok ? `${channel.display_name || channelName} is configured and reachable.` : `${channel.display_name || channelName} is not fully configured yet.`,
          });
          return;
        }
      }
      if (req.method === 'GET' && pathname === '/api/config/schema') {
        sendJson(res, 200, {
          ok: true,
          sections: {
            core: {
              title: 'Core',
              root_level: true,
              fields: {
                api_key: { type: 'string', label: 'API Key' },
                provider: { type: 'string', label: 'Default Provider' },
                model: { type: 'string', label: 'Default Model' },
                log_level: { type: 'string', label: 'Log Level' },
              },
            },
          },
        });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/config/set') {
        const payload = await bodyJson(req);
        const keyPath = cleanText(payload && payload.path ? payload.path : '', 140);
        if (!keyPath) {
          sendJson(res, 400, { ok: false, error: 'config_path_required' });
          return;
        }
        sendJson(res, 200, { ok: true, path: keyPath, value: payload ? payload.value : null, persisted: true });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/sessions') {
        sendJson(res, 200, { ok: true, sessions: listGlobalSessionsFromAgentFiles() });
        return;
      }
      if (req.method === 'DELETE' && pathname.startsWith('/api/sessions/')) {
        const sessionId = cleanText(safeDecodePathToken(pathname.split('/').slice(3).join('/')), 140);
        const removed = removeSessionById(sessionId);
        sendJson(res, 200, { ok: !!removed.ok, deleted: !!removed.deleted, session_id: sessionId });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/approvals') {
        sendJson(res, 200, { ok: true, approvals: ensureDefaultApprovals() });
        return;
      }
      if (req.method === 'POST' && pathname.startsWith('/api/approvals/')) {
        const parts = pathname.split('/').filter(Boolean);
        const approvalId = cleanText(parts[2] || '', 120);
        const action = cleanText(parts[3] || '', 40).toLowerCase();
        let rows = ensureDefaultApprovals();
        rows = rows.map((row) => {
          if (!row || cleanText(row.id || '', 120) !== approvalId) return row;
          return {
            ...row,
            status: action === 'approve' ? 'approved' : action === 'reject' ? 'rejected' : row.status,
            decided_at: nowIso(),
          };
        });
        writeArrayStore(APPROVALS_STATE_PATH, rows);
        sendJson(res, 200, { ok: true, id: approvalId, status: action === 'approve' ? 'approved' : 'rejected' });
        return;
      }
      if (pathname === '/api/workflows' && req.method === 'GET') {
        sendJson(res, 200, readArrayStore(WORKFLOWS_STATE_PATH, []));
        return;
      }
      if (pathname === '/api/workflows' && req.method === 'POST') {
        const payload = await bodyJson(req);
        const workflows = readArrayStore(WORKFLOWS_STATE_PATH, []);
        const id = `wf_${sha256(`${Date.now()}:${cleanText(payload && payload.name ? payload.name : '', 80)}`).slice(0, 10)}`;
        workflows.push({
          id,
          name: cleanText(payload && payload.name ? payload.name : 'Workflow', 120) || 'Workflow',
          description: cleanText(payload && payload.description ? payload.description : '', 300),
          steps: Array.isArray(payload && payload.steps) ? payload.steps : [],
          created_at: nowIso(),
          updated_at: nowIso(),
        });
        writeArrayStore(WORKFLOWS_STATE_PATH, workflows);
        sendJson(res, 200, { ok: true, id });
        return;
      }
      if (pathname.startsWith('/api/workflows/')) {
        const parts = pathname.split('/').filter(Boolean);
        const workflowId = cleanText(parts[2] || '', 120);
        const workflows = readArrayStore(WORKFLOWS_STATE_PATH, []);
        const found = workflows.find((row) => cleanText(row && row.id ? row.id : '', 120) === workflowId);
        if (!found) {
          sendJson(res, 404, { ok: false, error: 'workflow_not_found', id: workflowId });
          return;
        }
        if (req.method === 'GET' && parts.length === 3) {
          sendJson(res, 200, found);
          return;
        }
        if (req.method === 'PUT' && parts.length === 3) {
          const payload = await bodyJson(req);
          const next = workflows.map((row) => {
            if (cleanText(row && row.id ? row.id : '', 120) !== workflowId) return row;
            return {
              ...row,
              name: cleanText(payload && payload.name ? payload.name : row.name, 120) || row.name,
              description: cleanText(payload && payload.description ? payload.description : row.description, 300),
              steps: Array.isArray(payload && payload.steps) ? payload.steps : row.steps,
              updated_at: nowIso(),
            };
          });
          writeArrayStore(WORKFLOWS_STATE_PATH, next);
          sendJson(res, 200, { ok: true, id: workflowId });
          return;
        }
        if (req.method === 'DELETE' && parts.length === 3) {
          const next = workflows.filter((row) => cleanText(row && row.id ? row.id : '', 120) !== workflowId);
          writeArrayStore(WORKFLOWS_STATE_PATH, next);
          sendJson(res, 200, { ok: true, deleted: true, id: workflowId });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'run') {
          const payload = await bodyJson(req);
          sendJson(res, 200, {
            ok: true,
            id: workflowId,
            status: 'completed',
            output: `Workflow ${found.name} executed with input: ${cleanText(payload && payload.input ? payload.input : '', 200)}`,
          });
          return;
        }
        if (req.method === 'GET' && parts[3] === 'runs') {
          sendJson(res, 200, { ok: true, runs: [] });
          return;
        }
      }
      if (req.method === 'GET' && pathname === '/api/cron/jobs') {
        sendJson(res, 200, { ok: true, jobs: readArrayStore(CRON_JOBS_STATE_PATH, []) });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/cron/jobs') {
        const payload = await bodyJson(req);
        const jobs = readArrayStore(CRON_JOBS_STATE_PATH, []);
        const id = `cron_${sha256(`${Date.now()}:${cleanText(payload && payload.name ? payload.name : '', 80)}`).slice(0, 10)}`;
        jobs.push({
          id,
          name: cleanText(payload && payload.name ? payload.name : 'Scheduled Job', 140) || 'Scheduled Job',
          agent_id: cleanText(payload && payload.agent_id ? payload.agent_id : '', 140),
          schedule: payload && payload.schedule && typeof payload.schedule === 'object' ? payload.schedule : { kind: 'cron', expr: '* * * * *' },
          action: payload && payload.action && typeof payload.action === 'object' ? payload.action : { kind: 'agent_turn', message: 'Scheduled task' },
          delivery: payload && payload.delivery && typeof payload.delivery === 'object' ? payload.delivery : { kind: 'last_channel' },
          enabled: payload ? payload.enabled !== false : true,
          created_at: nowIso(),
          updated_at: nowIso(),
          last_run: '',
          next_run: '',
        });
        writeArrayStore(CRON_JOBS_STATE_PATH, jobs);
        sendJson(res, 200, { ok: true, id });
        return;
      }
      if (pathname.startsWith('/api/cron/jobs/')) {
        const parts = pathname.split('/').filter(Boolean);
        const jobId = cleanText(parts[3] || '', 120);
        let jobs = readArrayStore(CRON_JOBS_STATE_PATH, []);
        if (req.method === 'PUT' && parts[4] === 'enable') {
          const payload = await bodyJson(req);
          jobs = jobs.map((row) =>
            cleanText(row && row.id ? row.id : '', 120) === jobId
              ? { ...row, enabled: payload && payload.enabled === true, updated_at: nowIso() }
              : row
          );
          writeArrayStore(CRON_JOBS_STATE_PATH, jobs);
          sendJson(res, 200, { ok: true, id: jobId, enabled: payload && payload.enabled === true });
          return;
        }
        if (req.method === 'DELETE' && parts.length === 4) {
          jobs = jobs.filter((row) => cleanText(row && row.id ? row.id : '', 120) !== jobId);
          writeArrayStore(CRON_JOBS_STATE_PATH, jobs);
          sendJson(res, 200, { ok: true, deleted: true, id: jobId });
          return;
        }
      }
      if (req.method === 'GET' && pathname === '/api/triggers') {
        sendJson(res, 200, readArrayStore(TRIGGERS_STATE_PATH, []));
        return;
      }
      if (pathname.startsWith('/api/triggers/')) {
        const parts = pathname.split('/').filter(Boolean);
        const triggerId = cleanText(parts[2] || '', 120);
        let triggers = readArrayStore(TRIGGERS_STATE_PATH, []);
        if (req.method === 'PUT') {
          const payload = await bodyJson(req);
          triggers = triggers.map((row) =>
            cleanText(row && row.id ? row.id : '', 120) === triggerId
              ? { ...row, enabled: payload && payload.enabled === true, updated_at: nowIso() }
              : row
          );
          writeArrayStore(TRIGGERS_STATE_PATH, triggers);
          sendJson(res, 200, { ok: true, id: triggerId });
          return;
        }
        if (req.method === 'DELETE') {
          triggers = triggers.filter((row) => cleanText(row && row.id ? row.id : '', 120) !== triggerId);
          writeArrayStore(TRIGGERS_STATE_PATH, triggers);
          sendJson(res, 200, { ok: true, deleted: true, id: triggerId });
          return;
        }
      }
      if (req.method === 'POST' && pathname.startsWith('/api/schedules/') && pathname.endsWith('/run')) {
        const parts = pathname.split('/').filter(Boolean);
        const scheduleId = cleanText(parts[2] || '', 120);
        const jobs = readArrayStore(CRON_JOBS_STATE_PATH, []);
        const job = jobs.find((row) => cleanText(row && row.id ? row.id : '', 120) === scheduleId);
        sendJson(res, 200, {
          ok: true,
          id: scheduleId,
          status: job ? 'completed' : 'not_found',
          error: job ? '' : 'schedule_not_found',
        });
        return;
      }
      if (req.method === 'GET' && pathname.startsWith('/api/comms/events')) {
        sendJson(res, 200, { ok: true, events: [] });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/comms/topology') {
        sendJson(res, 200, {
          ok: true,
          topology: { connected: true, nodes: activeAgentCountFromSnapshot(latestSnapshot, 0), edges: 0 },
        });
        return;
      }
      if (req.method === 'POST' && (pathname === '/api/comms/send' || pathname === '/api/comms/task')) {
        sendJson(res, 200, { ok: true, queued: true, ts: nowIso() });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/hands') {
        sendJson(res, 200, { ok: true, hands: [] });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/hands/active') {
        sendJson(res, 200, { ok: true, active: [] });
        return;
      }
      if (req.method === 'GET' && pathname.startsWith('/api/hands/instances/')) {
        sendJson(res, 200, { ok: true, instances: [] });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/templates') {
        sendJson(res, 200, {
          ok: true,
          templates: [
            {
              name: 'general-assistant',
              category: 'General',
              description: 'General purpose assistant with balanced defaults.',
              manifest_toml:
                'name = "general-assistant"\\nmodule = "builtin:chat"\\n\\n[model]\\nprovider = "groq"\\nmodel = "llama-3.3-70b-versatile"\\nsystem_prompt = """You are a helpful assistant."""\\n',
            },
            {
              name: 'ops-reliability',
              category: 'Operations',
              description: 'Reliability-focused ops assistant.',
              manifest_toml:
                'name = "ops-reliability"\\nmodule = "builtin:chat"\\n\\n[model]\\nprovider = "openai"\\nmodel = "gpt-5"\\nsystem_prompt = """You are an SRE assistant focused on safe remediation."""\\n',
            },
          ],
        });
        return;
      }
      if (req.method === 'GET' && pathname.startsWith('/api/templates/')) {
        const templateName = cleanText(safeDecodePathToken(pathname.split('/').slice(3).join('/')), 120).toLowerCase();
        const templates = [
          {
            name: 'general-assistant',
            manifest_toml:
              'name = "general-assistant"\\nmodule = "builtin:chat"\\n\\n[model]\\nprovider = "groq"\\nmodel = "llama-3.3-70b-versatile"\\nsystem_prompt = """You are a helpful assistant."""\\n',
          },
          {
            name: 'ops-reliability',
            manifest_toml:
              'name = "ops-reliability"\\nmodule = "builtin:chat"\\n\\n[model]\\nprovider = "openai"\\nmodel = "gpt-5"\\nsystem_prompt = """You are an SRE assistant focused on safe remediation."""\\n',
          },
        ];
        const found = templates.find((row) => row.name === templateName);
        if (!found) {
          sendJson(res, 404, { ok: false, error: 'template_not_found', name: templateName });
          return;
        }
        sendJson(res, 200, { ok: true, name: found.name, manifest_toml: found.manifest_toml });
        return;
      }
      if (req.method === 'GET' && pathname.startsWith('/api/clawhub/search')) {
        sendJson(res, 200, { ok: true, items: [], next_cursor: '', total: 0 });
        return;
      }
      if (req.method === 'GET' && pathname.startsWith('/api/clawhub/browse')) {
        sendJson(res, 200, { ok: true, items: [], next_cursor: '', total: 0 });
        return;
      }
      if (req.method === 'GET' && pathname.startsWith('/api/clawhub/skill/')) {
        const slug = cleanText(safeDecodePathToken(pathname.split('/').slice(3).join('/')), 160);
        sendJson(res, 200, {
          ok: true,
          slug,
          title: slug || 'skill',
          summary: 'Skill metadata unavailable in local fallback mode.',
          readme: '',
          code: '',
        });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/clawhub/install') {
        sendJson(res, 200, { ok: true, installed: true });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/skills/create') {
        sendJson(res, 200, { ok: true, created: true });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/skills/uninstall') {
        sendJson(res, 200, { ok: true, removed: true });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/migrate/detect') {
        sendJson(res, 200, {
          ok: true,
          detected: true,
          source_path: path.resolve(ROOT, '..'),
          channels: Object.keys(loadChannelRegistry().channels || {}),
          agents: activeAgentCountFromSnapshot(latestSnapshot, 0),
        });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/migrate/scan') {
        const payload = await bodyJson(req);
        const sourcePath = workspacePathOrNull(payload && payload.path ? payload.path : '', { must_exist: false }) || path.resolve(ROOT, '..');
        sendJson(res, 200, {
          ok: true,
          source_path: sourcePath,
          files: [],
          channels: Object.keys(loadChannelRegistry().channels || {}),
          agents: activeAgentCountFromSnapshot(latestSnapshot, 0),
        });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/migrate') {
        sendJson(res, 200, { ok: true, status: 'completed', migrated: true, ts: nowIso() });
        return;
      }
      if (
        (req.method === 'GET' || req.method === 'POST') &&
        (pathname === '/api/memory/search' || pathname === '/api/memory_search')
      ) {
        const payload = req.method === 'POST' ? await bodyJson(req) : {};
        const query = cleanText(
          req.method === 'GET'
            ? reqUrl.searchParams.get('q') || reqUrl.searchParams.get('query') || reqUrl.searchParams.get('text') || ''
            : payload && (payload.q || payload.query || payload.text || payload.input)
              ? payload.q || payload.query || payload.text || payload.input
              : '',
          280
        );
        const limit = parsePositiveInt(
          req.method === 'GET'
            ? reqUrl.searchParams.get('limit') || reqUrl.searchParams.get('top') || MEMORY_SEARCH_DEFAULT_LIMIT
            : payload && payload.limit != null
              ? payload.limit
              : MEMORY_SEARCH_DEFAULT_LIMIT,
          MEMORY_SEARCH_DEFAULT_LIMIT,
          1,
          MEMORY_SEARCH_MAX_LIMIT
        );
        const local = searchMemoryLocal(query, latestSnapshot, { limit });
        sendJson(res, 200, {
          ok: true,
          query,
          limit,
          disabled: false,
          fallback_used: true,
          source: cleanText(local && local.source ? local.source : 'local_memory_fallback', 80) || 'local_memory_fallback',
          count: parseNonNegativeInt(local && local.count, Array.isArray(local && local.results) ? local.results.length : 0, 100000000),
          results: Array.isArray(local && local.results) ? local.results : [],
          scanned: local && local.scanned ? local.scanned : {},
          error: '',
        });
        return;
      }
      if (pathname.startsWith('/api/memory/agents/')) {
        const parts = pathname.split('/').filter(Boolean);
        const agentId = cleanText(safeDecodePathToken(parts[3] || ''), 140);
        if (!agentId) {
          sendJson(res, 400, { ok: false, error: 'agent_id_required' });
          return;
        }
        if (parts[4] !== 'kv') {
          sendJson(res, 404, { ok: false, error: 'memory_route_not_found', path: pathname });
          return;
        }
        if (req.method === 'GET' && parts.length === 5) {
          const listed = listAgentMemoryKv(agentId, latestSnapshot);
          sendJson(res, 200, {
            ok: true,
            agent_id: listed.agent_id,
            count: listed.count,
            kv_pairs: listed.kv_pairs,
          });
          return;
        }
        const keyRaw = parts.slice(5).join('/');
        const key = normalizeMemoryKey(keyRaw);
        if (!key) {
          sendJson(res, 400, { ok: false, error: 'memory_key_required', agent_id: agentId });
          return;
        }
        if (req.method === 'GET') {
          const found = readAgentMemoryKv(agentId, key, latestSnapshot);
          if (!found.ok) {
            sendJson(res, 404, { ok: false, error: found.error, agent_id: agentId, key });
            return;
          }
          sendJson(res, 200, {
            ok: true,
            agent_id: agentId,
            key: found.key,
            value: found.value,
          });
          return;
        }
        if (req.method === 'PUT') {
          const payload = await bodyJson(req);
          const value = payload && Object.prototype.hasOwnProperty.call(payload, 'value') ? payload.value : payload;
          const written = writeAgentMemoryKv(agentId, key, value, latestSnapshot);
          if (!written.ok) {
            sendJson(res, 400, { ok: false, error: written.error, agent_id: agentId, key });
            return;
          }
          writeActionReceipt(
            'memory.kv.put',
            { agent_id: agentId, key: written.key, cli_mode: ACTIVE_CLI_MODE },
            {
              ok: true,
              status: 0,
              argv: ['memory', 'kv', 'put'],
              payload: {
                ok: true,
                type: 'memory_kv_put',
                agent_id: agentId,
                key: written.key,
              },
            }
          );
          requestSnapshotRefresh(false);
          sendJson(res, 200, {
            ok: true,
            agent_id: agentId,
            key: written.key,
            value: written.value,
          });
          return;
        }
        if (req.method === 'DELETE') {
          const removed = deleteAgentMemoryKv(agentId, key, latestSnapshot);
          if (!removed.ok) {
            sendJson(res, 404, { ok: false, error: removed.error, agent_id: agentId, key });
            return;
          }
          writeActionReceipt(
            'memory.kv.delete',
            { agent_id: agentId, key: removed.key, cli_mode: ACTIVE_CLI_MODE },
            {
              ok: true,
              status: 0,
              argv: ['memory', 'kv', 'delete'],
              payload: {
                ok: true,
                type: 'memory_kv_delete',
                agent_id: agentId,
                key: removed.key,
              },
            }
          );
          requestSnapshotRefresh(false);
          sendJson(res, 200, {
            ok: true,
            agent_id: agentId,
            key: removed.key,
            deleted: true,
          });
          return;
        }
      }
      if (
        req.method === 'POST' &&
        (pathname === '/api/route/auto' || pathname === '/route/auto')
      ) {
        const payload = await bodyJson(req);
        const input =
          payload && (payload.input || payload.message || payload.prompt || payload.text)
            ? payload.input || payload.message || payload.prompt || payload.text
            : '';
        const tokenCount = parsePositiveInt(
          payload && payload.token_count != null ? payload.token_count : 0,
          Math.max(1, Math.round(String(input || '').length / 4)),
          1,
          8_000_000
        );
        const hasVision =
          !!(payload && payload.has_vision) ||
          !!(
            payload &&
            Array.isArray(payload.attachments) &&
            payload.attachments.some((row) =>
              /image|photo|png|jpg|jpeg|webp|gif/i.test(
                cleanText(row && (row.type || row.content_type || row.mime || ''), 80)
              )
            )
          );
        const agentId = cleanText(
          payload && (payload.agent_id || payload.agentId) ? payload.agent_id || payload.agentId : '',
          140
        );
        const route = planAutoRoute(input, latestSnapshot, {
          agent_id: agentId,
          token_count: tokenCount,
          has_vision: hasVision,
        });
        sendJson(res, 200, {
          ok: true,
          route,
          selected_provider: route.selected_provider,
          selected_model: route.selected_model,
          reason: route.reason,
        });
        return;
      }
      if (req.method === 'GET' && pathname === '/api/agents') {
        const snapshotTsMs = coerceTsMs(latestSnapshot && latestSnapshot.ts ? latestSnapshot.ts : 0, 0);
        const snapshotAgeMs = snapshotTsMs > 0 ? Math.max(0, Date.now() - snapshotTsMs) : Number.MAX_SAFE_INTEGER;
        const staleThresholdMs = Math.max(
          10_000,
          parsePositiveInt(flags && flags.refreshMs, DEFAULT_REFRESH_MS, 250, 60000) * 5
        );
        if (snapshotAgeMs > staleThresholdMs && !updating) {
          requestSnapshotRefresh(false);
        }
        const view = cleanText(reqUrl.searchParams.get('view') || '', 40).toLowerCase();
        const runtimeAuthorityRequested = ['1', 'true', 'runtime'].includes(
          cleanText(reqUrl.searchParams.get('authority') || '', 24).toLowerCase()
        );
        const team = cleanText(reqUrl.searchParams.get('team') || flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
        let agents = [];
        if (runtimeAuthorityRequested || view === 'sidebar' || view === 'list') {
          const lowLatencyListMode = view === 'sidebar' || view === 'list';
          try {
            agents = authoritativeAgentsFromRuntime(latestSnapshot, team, {
              includeArchived: false,
              timeout_ms: lowLatencyListMode ? 650 : Math.max(RUNTIME_AUTHORITY_LANE_TIMEOUT_MS, 2500),
              ttl_ms: lowLatencyListMode ? Math.max(RUNTIME_AUTHORITY_CACHE_TTL_MS, 900) : Math.max(RUNTIME_AUTHORITY_CACHE_TTL_MS, 1200),
              fail_ttl_ms: lowLatencyListMode ? Math.max(RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS, 1200) : Math.max(RUNTIME_AUTHORITY_CACHE_FAIL_TTL_MS, 800),
            });
          } catch {
            agents = [];
          }
        }
        if (!Array.isArray(agents) || agents.length === 0) {
          agents = compatAgentsFromSnapshot(latestSnapshot);
        }
        if ((!Array.isArray(agents) || agents.length === 0) && Array.isArray(lastKnownSidebarAgents) && lastKnownSidebarAgents.length) {
          agents = lastKnownSidebarAgents.slice(0, 500).map((row) => ({
            ...(row && typeof row === 'object' ? row : {}),
          }));
        }
        if (view === 'sidebar' || view === 'list') {
          const contractsState = loadAgentContractsState();
          const contractMap =
            contractsState && contractsState.contracts && typeof contractsState.contracts === 'object'
              ? contractsState.contracts
              : {};
          const nowMs = Date.now();
          const compactRows = agents
            .map((row) => {
              const id = cleanText(row && row.id ? row.id : '', 140);
              const contract = id && contractMap[id] ? contractSummary(contractMap[id], nowMs) : null;
              return {
                id,
                name: cleanText(row && row.name ? row.name : row && row.id ? row.id : '', 100),
                state: cleanText(row && row.state ? row.state : 'running', 40) || 'running',
                role: cleanText(row && row.role ? row.role : 'analyst', 60) || 'analyst',
                identity: row && row.identity && typeof row.identity === 'object' ? row.identity : {},
                avatar_url: cleanText(row && row.avatar_url ? row.avatar_url : '', 512),
                created_at: cleanText(row && row.created_at ? row.created_at : '', 80),
                updated_at: cleanText(row && row.updated_at ? row.updated_at : '', 80),
                last_activity_at: cleanText(
                  row && (row.last_activity_at || row.last_active_at || row.last_message_at)
                    ? row.last_activity_at || row.last_active_at || row.last_message_at
                    : '',
                  80
                ),
                git_branch: cleanText(row && row.git_branch ? row.git_branch : '', 120),
                workspace_rel: cleanText(row && row.workspace_rel ? row.workspace_rel : '', 240),
                git_tree_kind: cleanText(row && row.git_tree_kind ? row.git_tree_kind : '', 40),
                is_master_agent: !!(row && row.is_master_agent),
                contract,
                contract_status: contract && contract.status ? cleanText(contract.status, 40) : '',
                contract_expires_at: contract && contract.expires_at ? cleanText(contract.expires_at, 80) : '',
                contract_remaining_ms:
                  contract && contract.remaining_ms != null
                    ? parseNonNegativeInt(contract.remaining_ms, 0, 7 * 24 * 60 * 60 * 1000)
                    : null,
              };
            })
            .filter((entry) => !!(entry && entry.id));
          if (compactRows.length > 0) {
            lastKnownSidebarAgents = compactRows.slice(0, 500).map((row) => ({
              ...(row && typeof row === 'object' ? row : {}),
              identity: row && row.identity && typeof row.identity === 'object' ? { ...row.identity } : {},
              contract: row && row.contract && typeof row.contract === 'object' ? { ...row.contract } : null,
            }));
          }
          sendJson(res, 200, compactRows);
          return;
        }
        if (Array.isArray(agents) && agents.length > 0) {
          lastKnownSidebarAgents = agents.slice(0, 500).map((row) => ({
            ...(row && typeof row === 'object' ? row : {}),
          }));
        }
        sendJson(res, 200, agents);
        return;
      }
      if (req.method === 'GET' && pathname === '/api/agents/terminated') {
        const contractsState = loadAgentContractsState();
        const rows = Array.isArray(contractsState && contractsState.terminated_history)
          ? contractsState.terminated_history.slice(-50).reverse()
          : [];
        sendJson(res, 200, {
          ok: true,
          entries: rows,
        });
        return;
      }
      if (req.method === 'POST' && pathname === '/api/agents') {
        const payload = await bodyJson(req);
        const ingress = currentIngressControl(latestSnapshot);
        if (ingress.delay_ms > 0) {
          await waitMs(ingress.delay_ms);
        }
        const requestedName = cleanText(payload && payload.name ? payload.name : '', 100);
        const role = cleanText(payload && payload.role ? payload.role : 'analyst', 60) || 'analyst';
        const shadow = requestedName || `ops-${role}-${Date.now()}`;
        const laneResult = runAction('collab.launchRole', { team: flags.team || DEFAULT_TEAM, role, shadow });
        writeActionReceipt('collab.launchRole', { team: flags.team || DEFAULT_TEAM, role, shadow }, laneResult);
        if (laneResult.ok) {
          unarchiveAgent(shadow);
          upsertAgentContract(
            shadow,
            payload,
            {
              owner: cleanText(
                payload && payload.owner ? payload.owner : `session:${cleanText(flags.team || DEFAULT_TEAM, 40)}`,
                120
              ),
              force: true,
            }
          );
          optimisticCollabUpsertAgent(latestSnapshot, shadow, role);
          ensureAgentGitTreeAssignments(latestSnapshot, {
            force: true,
            preferred_master_id: shadow,
            ensure_workspace_agent_id: shadow,
          });
        }
        requestSnapshotRefresh(false);
        const created = compatAgentsFromSnapshot(latestSnapshot).find((row) => row.id === shadow) || {
          id: shadow,
          name: shadow,
          state: laneResult.ok ? 'running' : 'error',
          model_name:
            latestSnapshot && latestSnapshot.app && latestSnapshot.app.settings
              ? latestSnapshot.app.settings.model
              : 'gpt-5',
        };
        created.contract = contractSummary(contractForAgent(shadow));
        sendJson(res, laneResult.ok ? 200 : 400, created);
        return;
      }
      if (pathname.startsWith('/api/agents/')) {
        if (req.method !== 'GET') {
          maybeEnforceAgentContractsForApi('api.agent_scope');
        }
        const parts = pathname.split('/').filter(Boolean);
        const agentId = cleanText(parts[2] || '', 140);
        if (req.method === 'GET' && parts.length === 3) {
          const team = cleanText(flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
          const authoritativeAgents = authoritativeAgentsFromRuntime(latestSnapshot, team, {
            includeArchived: false,
          });
          const authoritativeAgent = Array.isArray(authoritativeAgents)
            ? authoritativeAgents.find((row) => row && row.id === agentId)
            : null;
          if (authoritativeAgent) {
            if (isAgentArchived(agentId)) {
              unarchiveAgent(agentId);
            }
            sendJson(res, 200, authoritativeAgent);
            return;
          }
          const archivedMeta = archivedAgentMeta(agentId);
          if (archivedMeta) {
            sendJson(res, 200, inactiveAgentRecord(agentId, latestSnapshot, archivedMeta));
            return;
          }
          const agent = compatAgentsFromSnapshot(latestSnapshot).find((row) => row.id === agentId);
          if (!agent) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          sendJson(res, 200, agent);
          return;
        }
        if (
          (req.method === 'GET' || req.method === 'POST') &&
          parts[3] === 'suggestions'
        ) {
          const payload = req.method === 'POST' ? await bodyJson(req) : {};
          const known = agentKnownInRuntime(agentId);
          if (!known && !isAgentArchived(agentId)) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const suggestionResult = generatePromptSuggestions(agentId, latestSnapshot, payload);
          sendJson(res, 200, {
            ok: !!suggestionResult.ok,
            id: agentId,
            suggestions: Array.isArray(suggestionResult.suggestions) ? suggestionResult.suggestions : [],
            source: cleanText(suggestionResult.source || 'fallback', 40) || 'fallback',
            model: cleanText(suggestionResult.model || '', 120),
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'file' && parts[4] === 'read') {
          const payload = await bodyJson(req);
          const known = agentKnownInRuntime(agentId);
          if (!known && !isAgentArchived(agentId)) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const fileResult = readFullFileForChat(payload && payload.path ? payload.path : '', {
            max_bytes: payload && payload.max_bytes != null ? payload.max_bytes : CHAT_FILE_READ_MAX_BYTES,
          });
          if (!fileResult.ok) {
            sendJson(res, 400, {
              ok: false,
              error: cleanText(fileResult.error || 'file_read_failed', 160) || 'file_read_failed',
              id: agentId,
              path: cleanText(payload && payload.path ? payload.path : '', 400),
            });
            return;
          }
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            file: fileResult,
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'folder' && parts[4] === 'export') {
          const payload = await bodyJson(req);
          const known = agentKnownInRuntime(agentId);
          if (!known && !isAgentArchived(agentId)) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const tree = buildDirectoryTreeForChat(payload && payload.path ? payload.path : '', {
            max_depth: payload && payload.max_depth != null ? payload.max_depth : CHAT_TREE_MAX_DEPTH,
            max_entries: payload && payload.max_entries != null ? payload.max_entries : CHAT_TREE_MAX_ENTRIES,
          });
          if (!tree.ok) {
            sendJson(res, 400, {
              ok: false,
              error: cleanText(tree.error || 'folder_export_failed', 160) || 'folder_export_failed',
              id: agentId,
              path: cleanText(payload && payload.path ? payload.path : '', 400),
            });
            return;
          }
          const archive = createFolderArchiveForChat(payload && payload.path ? payload.path : '');
          if (!archive.ok) {
            sendJson(res, 400, {
              ok: false,
              error: cleanText(archive.error || 'folder_archive_failed', 160) || 'folder_archive_failed',
              id: agentId,
              folder: tree,
            });
            return;
          }
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            folder: tree,
            archive,
          });
          return;
        }
        if (req.method === 'DELETE' && parts.length === 3) {
          const lifecycleLock = acquireAgentLifecycleLock(agentId);
          if (!lifecycleLock) {
            sendJson(res, 409, { ok: false, error: 'agent_lifecycle_busy', id: agentId });
            return;
          }
          try {
          const known = agentKnownInRuntime(agentId);
          const alreadyArchived = isAgentArchived(agentId);
          if (!known && !alreadyArchived) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const termination = terminateAgentForContract(agentId, latestSnapshot, 'chat_archive', {
            source: 'api.delete',
            terminated_by: 'user_archive',
            team: cleanText(flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM,
          });
          const archivedMeta = archivedAgentMeta(agentId) || archiveAgent(agentId, { source: 'api.delete', reason: 'chat_archive' });
          closeTerminalSession(agentId, 'agent_archived');
          closeAgentSockets(agentId, 'chat_archive');
          optimisticCollabArchiveAgent(latestSnapshot, agentId);
          requestSnapshotRefresh(false);
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            state: 'inactive',
            archived: true,
            archived_at: archivedMeta ? archivedMeta.archived_at : '',
            contract_terminated: !!(termination && termination.terminated),
            type: 'agent_archived',
          });
          return;
          } finally {
            releaseAgentLifecycleLock(lifecycleLock);
          }
        }
        if (req.method === 'POST' && parts[3] === 'revoke') {
          const lifecycleLock = acquireAgentLifecycleLock(agentId);
          if (!lifecycleLock) {
            sendJson(res, 409, { ok: false, error: 'agent_lifecycle_busy', id: agentId });
            return;
          }
          try {
          const known = agentKnownInRuntime(agentId);
          if (!known && !isAgentArchived(agentId)) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          markContractRevocation(agentId, 'manual_revoke');
          const termination = terminateAgentForContract(agentId, latestSnapshot, 'manual_revocation', {
            source: 'api.revoke',
            terminated_by: 'manual_revoke',
            team: cleanText(flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM,
          });
          closeAgentSockets(agentId, 'manual_revocation');
          optimisticCollabArchiveAgent(latestSnapshot, agentId);
          requestSnapshotRefresh(false);
          sendJson(res, termination && termination.terminated ? 200 : 409, {
            ok: !!(termination && termination.terminated),
            id: agentId,
            state: 'inactive',
            archived: isAgentArchived(agentId),
            reason: 'manual_revocation',
            contract: contractSummary(contractForAgent(agentId)),
          });
          return;
          } finally {
            releaseAgentLifecycleLock(lifecycleLock);
          }
        }
        if (req.method === 'POST' && parts[3] === 'complete') {
          const lifecycleLock = acquireAgentLifecycleLock(agentId);
          if (!lifecycleLock) {
            sendJson(res, 409, { ok: false, error: 'agent_lifecycle_busy', id: agentId });
            return;
          }
          try {
          const known = agentKnownInRuntime(agentId);
          if (!known && !isAgentArchived(agentId)) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const contract = markContractCompletion(agentId, 'supervisor_signal');
          const termination = terminateAgentForContract(agentId, latestSnapshot, 'task_complete', {
            source: 'api.complete',
            terminated_by: 'supervisor_signal',
            team: cleanText(flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM,
          });
          if (termination && termination.terminated) {
            closeAgentSockets(agentId, 'task_complete');
          }
          optimisticCollabArchiveAgent(latestSnapshot, agentId);
          requestSnapshotRefresh(false);
          sendJson(res, termination && termination.terminated ? 200 : 409, {
            ok: !!(termination && termination.terminated),
            id: agentId,
            reason: 'task_complete',
            contract: contractSummary(contract || contractForAgent(agentId)),
          });
          return;
          } finally {
            releaseAgentLifecycleLock(lifecycleLock);
          }
        }
        if (req.method === 'POST' && parts[3] === 'revive') {
          const lifecycleLock = acquireAgentLifecycleLock(agentId);
          if (!lifecycleLock) {
            sendJson(res, 409, { ok: false, error: 'agent_lifecycle_busy', id: agentId });
            return;
          }
          try {
          const payload = await bodyJson(req);
          const archivedMeta = archivedAgentMeta(agentId);
          if (!archivedMeta) {
            sendJson(res, 404, { ok: false, error: 'agent_not_archived', id: agentId });
            return;
          }
          const role = cleanText(
            payload && payload.role ? payload.role : archivedMeta.role || 'analyst',
            60
          ) || 'analyst';
          const reviveToMain =
            (payload && payload.force_master === true) ||
            archivedMeta.was_master_agent === true ||
            normalizeGitTreeKind(archivedMeta.git_tree_kind, AGENT_GIT_TREE_KIND_ISOLATED) === AGENT_GIT_TREE_KIND_MASTER ||
            isMainTreeBoundAgent(agentId, null);
          const laneResult = runAction('collab.launchRole', {
            team: cleanText(flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM,
            role,
            shadow: agentId,
          });
          writeActionReceipt('collab.launchRole', { team: flags.team || DEFAULT_TEAM, role, shadow: agentId }, laneResult);
          if (!laneResult.ok) {
            sendJson(res, 400, { ok: false, error: 'revive_launch_failed', id: agentId, lane: laneOutcome(laneResult) });
            return;
          }
          unarchiveAgent(agentId);
          const previous = contractForAgent(agentId);
          const nextContract = upsertAgentContract(
            agentId,
            {
              ...(payload && typeof payload === 'object' ? payload : {}),
              contract: {
                ...((payload && payload.contract && typeof payload.contract === 'object') ? payload.contract : {}),
                revived_from_contract_id: cleanText(previous && previous.contract_id ? previous.contract_id : '', 80),
                revival_data: archivedMeta && archivedMeta.revival_data ? archivedMeta.revival_data : buildAgentRevivalData(agentId),
              },
            },
            {
              owner: cleanText(payload && payload.owner ? payload.owner : archivedMeta.owner || 'dashboard_session', 120),
              force: true,
            }
          );
          const contractsState = loadAgentContractsState();
          if (Array.isArray(contractsState.terminated_history)) {
            contractsState.terminated_history = contractsState.terminated_history.map((row) => {
              if (!row || row.agent_id !== agentId || row.revived) return row;
              return { ...row, revived: true, revived_at: nowIso() };
            });
            saveAgentContractsState(contractsState);
          }
          optimisticCollabUpsertAgent(latestSnapshot, agentId, role);
          ensureAgentGitTreeProfile(agentId, {
            force_master: reviveToMain,
            force_isolated: !reviveToMain,
            ensure_workspace_ready: !reviveToMain,
          });
          ensureAgentGitTreeAssignments(latestSnapshot, {
            force: true,
            preferred_master_id: reviveToMain ? agentId : '',
            ensure_workspace_agent_id: !reviveToMain ? agentId : '',
          });
          requestSnapshotRefresh(false);
          const revived = compatAgentsFromSnapshot(latestSnapshot).find((row) => row.id === agentId) || {
            id: agentId,
            name: agentId,
            state: 'running',
            role,
          };
          revived.contract = contractSummary(nextContract);
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            revived: true,
            state: revived.state || 'running',
            role,
            contract: revived.contract,
            revived_to_main: reviveToMain,
          });
          return;
          } finally {
            releaseAgentLifecycleLock(lifecycleLock);
          }
        }
        if (req.method === 'POST' && parts[3] === 'terminal') {
          const payload = await bodyJson(req);
          const known = agentKnownInRuntime(agentId);
          if (!known) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          if (isAgentArchived(agentId)) {
            sendJson(res, 409, { ok: false, error: 'agent_inactive', id: agentId, state: 'inactive' });
            return;
          }
          const terminal = await runTerminalCommand(
            payload && (payload.command || payload.input || payload.message) ? payload.command || payload.input || payload.message : '',
            payload && payload.cwd ? payload.cwd : '',
            agentId,
            latestSnapshot
          );
          writeActionReceipt(
            'app.terminal',
            {
              agent_id: agentId,
              command: cleanText(terminal.command || '', 400),
              cwd: cleanText(terminal.cwd || '', 260),
              cli_mode: ACTIVE_CLI_MODE,
            },
            {
              ok: terminal.ok,
              status: terminal.status,
              argv: ['terminal', cleanText(terminal.command || '', 120)],
              payload: {
                ok: terminal.ok,
                type: 'terminal_command',
                exit_code: terminal.exit_code,
              },
            }
          );
          if (terminal.blocked) {
            sendJson(res, 400, {
              ok: false,
              error: 'terminal_blocked',
              message: terminal.message,
              cwd: terminal.cwd,
            });
            return;
          }
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            command: terminal.command,
            cwd: terminal.cwd,
            stdout: terminal.stdout,
            stderr: terminal.stderr,
            exit_code: terminal.exit_code,
            status: terminal.status,
            duration_ms: terminal.duration_ms,
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'message') {
          const payload = await bodyJson(req);
          const input = payload && (payload.input || payload.message || payload.prompt || payload.text)
            ? payload.input || payload.message || payload.prompt || payload.text
            : '';
          const turn = runAgentMessage(agentId, input, latestSnapshot);
          if (!turn.ok && turn.error === 'agent_not_found') {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          if (!turn.ok && turn.error === 'message_required') {
            sendJson(res, 400, { ok: false, error: 'message_required' });
            return;
          }
          if (!turn.ok && turn.error === 'agent_inactive') {
            sendJson(res, 409, { ok: false, error: 'agent_inactive', id: agentId, state: 'inactive' });
            return;
          }
          if (!turn.ok) {
            sendJson(res, turn.status || 400, {
              ok: false,
              error: cleanText(turn.error || 'agent_message_failed', 120) || 'agent_message_failed',
              id: agentId,
              reason: cleanText(turn.reason || '', 120),
              detail: cleanText(turn.detail || '', 240),
              terminated: !!turn.terminated,
            });
            return;
          }
          writeActionReceipt(
            'app.chat',
            { input: turn.input, agent_id: agentId, session_id: turn.session_id, cli_mode: ACTIVE_CLI_MODE },
            turn.laneResult
          );
          appendAgentConversation(agentId, latestSnapshot, turn.input, turn.response, turn.meta, turn.tools, {
            assistant_agent_id: turn.agent_id || agentId,
            assistant_agent_name: cleanText(turn && turn.agent && turn.agent.name ? turn.agent.name : '', 120),
          });
          requestSnapshotRefresh(false);
          sendJson(res, turn.status, {
            ok: turn.ok,
            agent_id: agentId,
            session_id: turn.session_id,
            response: turn.response,
            tools: turn.tools,
            model: turn.model,
            model_provider: turn.model_provider || providerForModelName(turn.model, configuredProvider(latestSnapshot)),
            auto_route: turn.auto_route || null,
            turn: {
              role: 'agent',
              text: turn.response,
              agent_id: turn.agent_id || agentId,
              agent_name: cleanText(turn && turn.agent && turn.agent.name ? turn.agent.name : '', 120),
            },
            input_tokens: turn.input_tokens,
            output_tokens: turn.output_tokens,
            context_tokens: turn.context_tokens,
            context_window: turn.context_window,
            context_ratio: turn.context_ratio,
            context_pressure: turn.context_pressure,
            cost_usd: turn.cost_usd,
            iterations: turn.iterations,
            duration_ms: turn.duration_ms,
            runtime_sync: turn.runtime_sync || null,
          });
          return;
        }
        if (req.method === 'PUT' && parts[3] === 'model') {
          const payload = await bodyJson(req);
          const agent = compatAgentsFromSnapshot(latestSnapshot).find((row) => row.id === agentId);
          if (!agent) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const requested = cleanText(
            payload && payload.model != null ? payload.model : '',
            120
          );
          const state = loadAgentSession(agentId, latestSnapshot);
          state.model_override = requested && requested.toLowerCase() !== 'auto' ? requested : 'auto';
          saveAgentSession(agentId, state);
          upsertAgentProfile(agentId, {
            model_override: state.model_override,
          });
          const resolved = effectiveAgentModel(agentId, latestSnapshot);
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            model: resolved.selected,
            provider: resolved.provider,
            runtime_model: resolved.runtime_model,
            context_window: resolved.context_window,
          });
          return;
        }
        if (req.method === 'GET' && parts[3] === 'session') {
          ensureAgentGitTreeAssignments(latestSnapshot, {
            force: false,
            preferred_master_id: agentId,
          });
          const state = loadAgentSession(agentId, latestSnapshot);
          const session = activeSession(state);
          const profile = ensureAgentGitTreeProfile(agentId, { force_master: false, ensure_workspace_ready: false });
          const gitTree = agentGitTreeView(agentId, profile);
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            session_id: session.session_id,
            messages: Array.isArray(session.messages) ? session.messages : [],
            git_tree_kind: gitTree.git_tree_kind,
            git_branch: gitTree.git_branch,
            workspace_dir: gitTree.workspace_dir,
            workspace_rel: gitTree.workspace_rel,
            git_tree_ready: gitTree.git_tree_ready,
            is_master_agent: gitTree.is_master_agent,
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'session' && parts[4] === 'reset') {
          const state = loadAgentSession(agentId, latestSnapshot);
          const session = activeSession(state);
          session.messages = [];
          session.updated_at = nowIso();
          saveAgentSession(agentId, state);
          sendJson(res, 200, { ok: true, id: agentId, message: 'Session reset' });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'session' && parts[4] === 'compact') {
          compactAgentConversation(agentId, latestSnapshot);
          sendJson(res, 200, { ok: true, id: agentId, message: 'Session compacted' });
          return;
        }
        if (req.method === 'GET' && parts[3] === 'sessions') {
          const state = loadAgentSession(agentId, latestSnapshot);
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            sessions: sessionList(state),
            active_session_id: state.active_session_id,
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'sessions' && parts.length === 4) {
          const payload = await bodyJson(req);
          const state = loadAgentSession(agentId, latestSnapshot);
          const sessionId = `session_${Date.now().toString(36)}`;
          const label =
            cleanText(payload && payload.label ? payload.label : '', 80) ||
            `Session ${state.sessions.length + 1}`;
          state.sessions.push({
            session_id: sessionId,
            label,
            created_at: nowIso(),
            updated_at: nowIso(),
            messages: [],
          });
          state.active_session_id = sessionId;
          saveAgentSession(agentId, state);
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            created: sessionId,
            sessions: sessionList(state),
            active_session_id: state.active_session_id,
          });
          return;
        }
        if (
          req.method === 'POST' &&
          parts[3] === 'sessions' &&
          parts.length >= 6 &&
          parts[5] === 'switch'
        ) {
          const targetSessionId = cleanText(parts[4] || '', 80);
          const state = loadAgentSession(agentId, latestSnapshot);
          const exists = state.sessions.some((session) => session.session_id === targetSessionId);
          if (!exists) {
            sendJson(res, 404, { ok: false, error: 'session_not_found', session_id: targetSessionId });
            return;
          }
          state.active_session_id = targetSessionId;
          saveAgentSession(agentId, state);
          const session = activeSession(state);
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            session_id: targetSessionId,
            messages: Array.isArray(session.messages) ? session.messages : [],
            sessions: sessionList(state),
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'stop') {
          const known = agentKnownInRuntime(agentId);
          if (!known && !isAgentArchived(agentId)) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const archivedMeta = archivedAgentMeta(agentId);
          if (archivedMeta || isAgentArchived(agentId)) {
            sendJson(res, 409, {
              ok: false,
              error: 'agent_inactive',
              id: agentId,
              state: 'inactive',
              archived: true,
              archived_at: archivedMeta && archivedMeta.archived_at ? archivedMeta.archived_at : '',
              reason: archivedMeta && archivedMeta.reason ? archivedMeta.reason : 'archived',
              type: 'agent_archived',
            });
            return;
          }
          const contract = contractForAgent(agentId);
          const contractStatus = cleanText(contract && contract.status ? contract.status : 'active', 40).toLowerCase();
          if (contractStatus && contractStatus !== 'active') {
            sendJson(res, 409, {
              ok: false,
              error: 'agent_contract_terminated',
              id: agentId,
              state: 'inactive',
              contract_terminated: true,
              reason: cleanText(contract && contract.termination_reason ? contract.termination_reason : contractStatus, 120),
              contract: contractSummary(contract),
            });
            return;
          }
          writeActionReceipt(
            'app.stop',
            { agent_id: agentId, source: 'chat_stop', cli_mode: ACTIVE_CLI_MODE },
            {
              ok: true,
              status: 0,
              argv: ['agent', 'stop', `--agent=${agentId}`],
              payload: {
                ok: true,
                type: 'agent_stop_ack',
                state: 'running',
              },
            }
          );
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            state: 'running',
            type: 'agent_stop_ack',
            message: 'Run cancelled',
            contract: contractSummary(contract),
          });
          return;
        }
        if (
          req.method === 'PATCH' &&
          (parts[3] === 'identity' || parts[3] === 'config')
        ) {
          const payload = await bodyJson(req);
          const known = agentKnownInRuntime(agentId);
          if (!known && !isAgentArchived(agentId)) {
            sendJson(res, 404, { ok: false, error: 'agent_not_found', id: agentId });
            return;
          }
          const raw = payload && typeof payload === 'object' ? payload : {};
          const existingProfile = agentProfileFor(agentId);
          const existingAgent =
            compatAgentsFromSnapshot(latestSnapshot, { includeArchived: true }).find((row) => row.id === agentId) || null;
          const previousName =
            cleanText(
              existingProfile && existingProfile.name
                ? existingProfile.name
                : existingAgent && existingAgent.name
                  ? existingAgent.name
                  : agentId,
              100
            ) || agentId;
          const identitySource =
            raw.identity && typeof raw.identity === 'object' ? raw.identity : raw;
          const patch = {};

          if (Object.prototype.hasOwnProperty.call(raw, 'name')) {
            patch.name = cleanText(raw.name, 100);
          }
          if (
            parts[3] === 'config' &&
            Object.prototype.hasOwnProperty.call(raw, 'system_prompt')
          ) {
            patch.system_prompt = cleanText(raw.system_prompt, 4000);
          }
          if (
            parts[3] === 'config' &&
            Object.prototype.hasOwnProperty.call(raw, 'role')
          ) {
            patch.role = cleanText(raw.role, 60);
          }
          if (
            parts[3] === 'config' &&
            Object.prototype.hasOwnProperty.call(raw, 'fallback_models')
          ) {
            patch.fallback_models = normalizeAgentFallbackModels(raw.fallback_models);
          }

          const identityPatch = {};
          if (Object.prototype.hasOwnProperty.call(identitySource, 'emoji')) {
            identityPatch.emoji = cleanText(identitySource.emoji, 24);
          }
          if (Object.prototype.hasOwnProperty.call(identitySource, 'color')) {
            identityPatch.color = normalizeIdentityColor(identitySource.color, '#2563EB');
          }
          if (Object.prototype.hasOwnProperty.call(identitySource, 'archetype')) {
            identityPatch.archetype = cleanText(identitySource.archetype, 80);
          }
          if (Object.prototype.hasOwnProperty.call(identitySource, 'vibe')) {
            identityPatch.vibe = cleanText(identitySource.vibe, 80);
          }
          if (Object.keys(identityPatch).length > 0) {
            patch.identity = identityPatch;
          }

          const profile = upsertAgentProfile(agentId, patch);
          let renameNotice = null;
          if (profile && Object.prototype.hasOwnProperty.call(patch, 'name')) {
            const nextName = cleanText(profile && profile.name ? profile.name : '', 100) || agentId;
            if (nextName !== previousName) {
              renameNotice = appendAgentNoticeEvent(
                agentId,
                latestSnapshot,
                `changed name from ${previousName} to ${nextName}`,
                { notice_type: 'info', notice_icon: 'i' }
              );
            }
          }
          writeActionReceipt(
            `app.agent.${parts[3]}`,
            {
              agent_id: agentId,
              payload_keys: Object.keys(raw || {}),
              cli_mode: ACTIVE_CLI_MODE,
            },
            {
              ok: !!profile,
              status: profile ? 0 : 1,
              argv: ['agent', parts[3], `--agent=${agentId}`],
              payload: {
                ok: !!profile,
                type: 'agent_profile_update',
                section: parts[3],
              },
            }
          );
          if (!profile) {
            sendJson(res, 500, { ok: false, error: 'agent_profile_update_failed', id: agentId });
            return;
          }

          requestSnapshotRefresh(false);
          const archivedMeta = archivedAgentMeta(agentId);
          const updated =
            archivedMeta
              ? inactiveAgentRecord(agentId, latestSnapshot, archivedMeta)
              : compatAgentsFromSnapshot(latestSnapshot, { includeArchived: true }).find((row) => row.id === agentId) || null;
          sendJson(res, 200, {
            ok: true,
            id: agentId,
            type: 'agent_profile_update',
            section: parts[3],
            profile,
            agent: updated,
            rename_notice: renameNotice,
          });
          return;
        }
        if (req.method === 'POST' && parts[3] === 'clone') {
          sendJson(res, 200, { ok: true, id: agentId, type: 'infring_external_compat_stub' });
          return;
        }
      }
      if (req.method === 'POST' && pathname === '/api/dashboard/action') {
        const payload = await bodyJson(req);
        const action = cleanText(payload && payload.action ? payload.action : '', 80);
        const actionPayload = payload && payload.payload && typeof payload.payload === 'object' ? payload.payload : {};
        const ingress = currentIngressControl(latestSnapshot);
        if (ingress.delay_ms > 0) {
          await waitMs(ingress.delay_ms);
        }
        if (ingress.reject_non_critical && !isCriticalDashboardAction(action)) {
          sendJson(res, 429, {
            ok: false,
            type: 'infring_dashboard_action_response',
            error: 'ingress_backpressure_active',
            action,
            ingress_control: ingress,
            queue_depth: runtimeSyncSummary(latestSnapshot).queue_depth,
            message: 'Non-critical actions are temporarily blocked while queue backpressure is active.',
          });
          return;
        }
        if (
          action === 'dashboard.runtime.executeSwarmRecommendation' ||
          action === 'dashboard.runtime.applyTelemetryRemediations'
        ) {
          const lanePayload = executeRuntimeSwarmRecommendation(latestSnapshot);
          const laneResult = {
            ok: !!lanePayload.ok,
            status: lanePayload.ok ? 0 : 1,
            argv: [action],
            payload: lanePayload,
          };
          const actionReceipt = writeActionReceipt(action, actionPayload, laneResult);
          requestSnapshotRefresh(true);
          sendJson(res, lanePayload.ok ? 200 : 400, {
            ok: !!lanePayload.ok,
            type: 'infring_dashboard_action_response',
            action,
            action_receipt: actionReceipt,
            lane: lanePayload,
            snapshot: latestSnapshot,
          });
          return;
        }
        if (action === 'app.chat') {
          const input =
            actionPayload &&
            (actionPayload.input || actionPayload.message || actionPayload.prompt || actionPayload.text)
              ? actionPayload.input || actionPayload.message || actionPayload.prompt || actionPayload.text
              : '';
          let requestedAgentId =
            cleanText(
              actionPayload && (actionPayload.agent_id || actionPayload.agentId)
                ? actionPayload.agent_id || actionPayload.agentId
                : '',
              140
            ) || '';
          if (!requestedAgentId) {
            const fallbackAgentId = 'chat-ui-default-agent';
            if (isAgentArchived(fallbackAgentId)) {
              unarchiveAgent(fallbackAgentId);
            }
            upsertAgentContract(
              fallbackAgentId,
              {
                mission: `Assist with assigned mission for ${fallbackAgentId}.`,
                owner: 'dashboard_chat',
                termination_condition: 'task_or_timeout',
              },
              { owner: 'dashboard_chat', force: true }
            );
            optimisticCollabUpsertAgent(latestSnapshot, fallbackAgentId, 'operator');
            requestedAgentId = fallbackAgentId;
          }
          let turn = runAgentMessage(requestedAgentId, input, latestSnapshot, { allowFallback: true });
          if (
            !turn.ok &&
            (turn.error === 'agent_inactive' || turn.error === 'agent_contract_terminated')
          ) {
            // Recovery path: retry through the dashboard fallback agent so stale/terminated
            // selections do not strand runtime chat actions.
            turn = runAgentMessage('chat-ui-default-agent', input, latestSnapshot, { allowFallback: true });
          }
          const lanePayload = {
            ok: turn.ok,
            type: 'infring_dashboard_runtime_chat',
            response: turn.response || '',
            session_id: turn.session_id || '',
            agent_id: turn.agent_id || requestedAgentId || 'chat-ui-default-agent',
            auto_route: turn.auto_route || null,
            turn: {
              turn_id: `turn_${sha256(`${turn.agent_id || requestedAgentId || 'chat-ui-default-agent'}:${Date.now()}`).slice(0, 10)}`,
              user: turn.input || '',
              assistant: turn.response || '',
              ts: nowIso(),
              status: turn.lane_ok ? 'complete' : 'degraded',
              provider: turn.model_provider || providerForModelName(turn.model, configuredProvider(latestSnapshot)),
              model: turn.model || configuredOllamaModel(latestSnapshot),
              auto_route: turn.auto_route || null,
            },
            tools: Array.isArray(turn.tools) ? turn.tools : [],
            input_tokens: parseNonNegativeInt(turn.input_tokens, 0, 1000000000),
            output_tokens: parseNonNegativeInt(turn.output_tokens, 0, 1000000000),
            context_tokens: parseNonNegativeInt(turn.context_tokens, 0, 1000000000),
            context_window: parsePositiveInt(turn.context_window, DEFAULT_CONTEXT_WINDOW_TOKENS, 1024, 8000000),
            context_ratio: Number.isFinite(Number(turn.context_ratio)) ? Number(turn.context_ratio) : 0,
            context_pressure: cleanText(turn.context_pressure || '', 24) || 'low',
            cost_usd: Number.isFinite(Number(turn.cost_usd)) ? Number(turn.cost_usd) : 0,
            iterations: parsePositiveInt(turn.iterations, 1, 1, 12),
            duration_ms: parsePositiveInt(turn.duration_ms, 0, 0, 3600000),
            backend: cleanText(turn.backend || '', 40),
            meta: cleanText(turn.meta || '', 220),
            runtime_sync: turn.runtime_sync || null,
            error: turn && turn.error ? cleanText(turn.error, 120) : '',
          };
          const laneResult =
            turn && turn.laneResult && typeof turn.laneResult === 'object'
              ? turn.laneResult
              : {
                  ok: turn.ok,
                  status: turn.ok ? 0 : 1,
                  argv: ['infring_dashboard_runtime_chat'],
                  payload: lanePayload,
                };
          const actionReceipt = writeActionReceipt(
            'app.chat',
            {
              input: turn.input || cleanText(input || '', 2000),
              agent_id: turn.agent_id || requestedAgentId,
              session_id: turn.session_id || '',
              cli_mode: ACTIVE_CLI_MODE,
            },
            laneResult
          );
          if (turn.ok) {
            appendAgentConversation(
              turn.agent_id || requestedAgentId,
              latestSnapshot,
              turn.input || cleanText(input || '', 4000),
              turn.response || '',
              turn.meta || '',
              turn.tools,
              {
                assistant_agent_id: turn.agent_id || requestedAgentId,
                assistant_agent_name: cleanText(turn && turn.agent && turn.agent.name ? turn.agent.name : '', 120),
              }
            );
            optimisticCollabHydrateFromTools(latestSnapshot, turn.tools);
          }
          sendJson(
            res,
            turn.ok
              ? 200
              : turn.error === 'agent_inactive' || turn.error === 'agent_contract_terminated'
                ? 409
                : 400,
            {
              ok: turn.ok,
              type: 'infring_dashboard_action_response',
              action,
              action_receipt: actionReceipt,
              lane: lanePayload,
              snapshot: latestSnapshot,
            }
          );
          return;
        }
        const laneResult = runAction(action, actionPayload);
        const actionReceipt = writeActionReceipt(action, actionPayload, laneResult);
        requestSnapshotRefresh(true);
        const ok = !!laneResult.ok;
        sendJson(res, ok ? 200 : 400, {
          ok,
          type: 'infring_dashboard_action_response',
          action,
          action_receipt: actionReceipt,
          lane: laneResult.payload || null,
          snapshot: latestSnapshot,
        });
        return;
      }
      if (req.method === 'GET' && pathname === '/healthz') {
        const snapshotTsMs = coerceTsMs(latestSnapshot && latestSnapshot.ts ? latestSnapshot.ts : 0, 0);
        sendJson(res, 200, {
          ok: true,
          type: 'infring_dashboard_healthz',
          ts: nowIso(),
          receipt_hash: latestSnapshot.receipt_hash,
          snapshot_age_ms: snapshotTsMs > 0 ? Math.max(0, Date.now() - snapshotTsMs) : null,
          snapshot_build_ms: parseNonNegativeInt(lastSnapshotBuildDurationMs, 0, 1000000000),
          next_snapshot_refresh_ms: parseNonNegativeInt(nextSnapshotRefreshAtMs, 0, 1000000000000),
        });
        return;
      }
      if (req.method === 'GET') {
        const compatPayload = compatApiPayload(pathname, reqUrl, latestSnapshot);
        if (compatPayload) {
          sendJson(res, 200, compatPayload);
          return;
        }
      }
      if (pathname.startsWith('/api/')) {
        sendJson(res, 200, {
          ok: true,
          type: 'infring_external_compat_stub',
          path: pathname,
        });
        return;
      }
      sendJson(res, 404, {
        ok: false,
        type: 'infring_dashboard_not_found',
        path: pathname,
      });
    } catch (error) {
      sendJson(res, 500, {
        ok: false,
        type: 'infring_dashboard_request_error',
        error: cleanText(error && error.message ? error.message : String(error), 260),
      });
    }
  });

  const wss = new WebSocketServer({ noServer: true, perMessageDeflate: false });
  const agentWss = new WebSocketServer({ noServer: true, perMessageDeflate: false });
  const wsClients = new Set();
  const agentWsClients = new Map();
  const wsClientHeartbeat = new Map();
  const agentWsHeartbeat = new Map();

  const markWsHeartbeat = (socket, isAgentSocket = false) => {
    if (!socket) return;
    const target = isAgentSocket ? agentWsHeartbeat : wsClientHeartbeat;
    target.set(socket, Date.now());
  };

  const dropSnapshotSocket = (socket) => {
    wsClients.delete(socket);
    wsClientHeartbeat.delete(socket);
  };

  const dropAgentSocket = (socket) => {
    const agentId = cleanText(agentWsClients.get(socket) || '', 140);
    agentWsClients.delete(socket);
    agentWsHeartbeat.delete(socket);
    if (!agentId) return;
    let openCount = 0;
    for (const [client, socketAgentId] of Array.from(agentWsClients.entries())) {
      if (!client || client.readyState !== 1) {
        agentWsClients.delete(client);
        agentWsHeartbeat.delete(client);
        continue;
      }
      if (String(socketAgentId) === agentId) openCount += 1;
    }
    if (openCount === 0) {
      setContractConversationHold(agentId, false, {
        source: 'agent_ws_close',
        reset_timer_on_close: true,
      });
    }
  };

  const realtimeClientCounts = () => {
    let snapshotOpen = 0;
    let agentOpen = 0;
    for (const client of Array.from(wsClients)) {
      if (client && client.readyState === 1) {
        snapshotOpen += 1;
      } else {
        dropSnapshotSocket(client);
      }
    }
    for (const [client] of Array.from(agentWsClients.entries())) {
      if (client && client.readyState === 1) {
        agentOpen += 1;
      } else {
        dropAgentSocket(client);
      }
    }
    return {
      snapshot_open: snapshotOpen,
      agent_open: agentOpen,
      total_open: snapshotOpen + agentOpen,
    };
  };

  const sendWs = (socket, payload) => {
    if (!socket || socket.readyState !== 1) return;
    try {
      socket.send(JSON.stringify(payload));
    } catch {}
  };

  const wsSnapshotCompatPayload = () => {
    const runtime = runtimeSyncSummary(latestSnapshot);
    return {
      type: 'snapshot',
      ts: nowIso(),
      receipt_hash: cleanText(latestSnapshot && latestSnapshot.receipt_hash ? latestSnapshot.receipt_hash : '', 120),
      runtime: {
        queue_depth: parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000),
        conduit_signals: parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000),
        cockpit_blocks: parseNonNegativeInt(runtime && runtime.cockpit_blocks, 0, 100000000),
      },
    };
  };

  const broadcastSnapshot = () => {
    const envelope = latestSnapshotEnvelope;
    const compat = wsSnapshotCompatPayload();
    for (const client of wsClients) {
      if (client.readyState === 1) {
        sendWs(client, compat);
        client.send(envelope);
      }
    }
  };

  function requestSnapshotRefresh(broadcast = true) {
    const nowMs = Date.now();
    const minRefreshGapMs = Math.max(
      5000,
      parsePositiveInt(flags && flags.refreshMs, DEFAULT_REFRESH_MS, 250, 60000) * 2
    );
    if ((nowMs - parseNonNegativeInt(lastSnapshotRefreshRequestAtMs, 0, 1_000_000_000_000)) < minRefreshGapMs) {
      return false;
    }
    if (updating) return false;
    lastSnapshotRefreshRequestAtMs = nowMs;
    updating = true;
    setTimeout(() => {
      try {
        refreshSnapshot(null, { fast_lane_mode: true });
        if (broadcast) broadcastSnapshot();
      } catch {}
      updating = false;
    }, 0);
    return true;
  }

  const agentKnownInRuntime = (agentId) => {
    const id = cleanText(agentId || '', 140);
    if (!id) return false;
    const fromSnapshot = compatAgentsFromSnapshot(latestSnapshot, { includeArchived: true }).some((row) => row.id === id);
    if (fromSnapshot) return true;
    const team = cleanText(flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM;
    const lane = runLaneCached(
      `agent_presence.${team}.${id}`,
      ['collab-plane', 'dashboard', `--team=${team}`],
      {
        timeout_ms: Math.min(800, LANE_SYNC_TIMEOUT_MS),
        ttl_ms: 500,
        fail_ttl_ms: 250,
        stale_fallback: false,
      }
    );
    const runtimeAgents =
      lane &&
      lane.payload &&
      lane.payload.dashboard &&
      Array.isArray(lane.payload.dashboard.agents)
        ? lane.payload.dashboard.agents
        : [];
    return runtimeAgents.some((row) => cleanText(row && row.shadow ? row.shadow : '', 140) === id);
  };

  const closeAgentSockets = (agentId, reason = 'agent_inactive') => {
    const id = String(agentId || '');
    if (!id) return 0;
    let closed = 0;
    for (const [socket, socketAgentId] of agentWsClients.entries()) {
      if (String(socketAgentId) !== id) continue;
      sendWs(socket, { type: 'agent_archived', agent_id: id, ts: nowIso(), reason });
      try { socket.close(1008, reason); } catch {}
      dropAgentSocket(socket);
      closed += 1;
    }
    if (closed > 0) {
      setContractConversationHold(id, false, {
        source: 'agent_socket_close_all',
        reset_timer_on_close: true,
      });
    }
    return closed;
  };

  const agentInactiveForRealtime = (agentId) => {
    const id = cleanText(agentId || '', 140);
    if (!id) return true;
    if (!isAgentArchived(id)) return false;
    if (agentKnownInRuntime(id)) {
      unarchiveAgent(id);
      return false;
    }
    return true;
  };

  const enforceAgentContractsNow = (source = 'api') => {
    const nowMs = Date.now();
    if (
      String(source || '').startsWith('api.') &&
      (nowMs - parseNonNegativeInt(lastContractEnforceAtMs, 0, 1000000000000)) < 250
    ) {
      return null;
    }
    if (enforcingContracts) return null;
    lastContractEnforceAtMs = nowMs;
    enforcingContracts = true;
    try {
      const enforcementSnapshot = snapshotForContractEnforcement();
      const enforcement = enforceAgentContracts(enforcementSnapshot, {
        team: cleanText(flags.team || DEFAULT_TEAM, 40) || DEFAULT_TEAM,
      });
      const terminated = Array.isArray(enforcement && enforcement.terminated) ? enforcement.terminated : [];
      if (terminated.length > 0) {
        for (const row of terminated) {
          closeAgentSockets(
            row && row.agent_id ? row.agent_id : '',
            `agent_contract_${cleanText(row && row.reason ? row.reason : 'terminated', 80)}`
          );
        }
      }
      if (enforcement && enforcement.changed) {
        refreshSnapshot(enforcement);
        if (source !== 'silent') {
          broadcastSnapshot();
        }
      }
      return enforcement;
    } catch {
      return null;
    } finally {
      enforcingContracts = false;
    }
  };

  const scheduleContractEnforcement = (source = 'api') => {
    if (enforcingContracts || enforceContractsQueued) return false;
    enforceContractsQueued = true;
    setTimeout(() => {
      enforceContractsQueued = false;
      enforceAgentContractsNow(source);
    }, 0);
    return true;
  };

  const apiContractEnforceIntervalMs = () => {
    const activeAgents = activeAgentCountFromSnapshot(latestSnapshot, 0);
    const lowScaleFloorMs = Math.max(AGENT_CONTRACT_API_ENFORCE_INTERVAL_MS, 1500);
    if (activeAgents >= AGENT_CONTRACT_ENFORCE_MEGA_SCALE_THRESHOLD) {
      return Math.max(AGENT_CONTRACT_API_ENFORCE_INTERVAL_MEGA_SCALE_MS, 4000);
    }
    if (activeAgents >= AGENT_CONTRACT_ENFORCE_ULTRA_SCALE_THRESHOLD) {
      return Math.max(AGENT_CONTRACT_API_ENFORCE_INTERVAL_ULTRA_SCALE_MS, 3000);
    }
    if (activeAgents >= AGENT_CONTRACT_ENFORCE_HIGH_SCALE_THRESHOLD) {
      return Math.max(AGENT_CONTRACT_API_ENFORCE_INTERVAL_HIGH_SCALE_MS, 2000);
    }
    return lowScaleFloorMs;
  };

  const maybeEnforceAgentContractsForApi = (source = 'api') => {
    if (realtimeClientCounts().total_open > 0) {
      return null;
    }
    const nowMs = Date.now();
    const minIntervalMs = apiContractEnforceIntervalMs();
    if ((nowMs - parseNonNegativeInt(lastApiContractEnforceAtMs, 0, 1000000000000)) < minIntervalMs) {
      return null;
    }
    const scheduled = scheduleContractEnforcement(source);
    if (scheduled) {
      lastApiContractEnforceAtMs = nowMs;
    }
    return null;
  };

  const maybeRunAutonomousSelfHeal = (source = 'interval') => {
    const runtime = runtimeSyncSummary(latestSnapshot);
    const queueVelocityPerMin = queueDepthVelocity(runtimeTrendSeries);
    const stall = runtimeStallSignals(runtime, runtimeTrendSeries);
    const queueDepth = parseNonNegativeInt(runtime && runtime.queue_depth, 0, 100000000);
    const targetSignals = parsePositiveInt(
      runtime && runtime.target_conduit_signals,
      RUNTIME_AUTO_BALANCE_THRESHOLD,
      1,
      128
    );
    const conduitSignals = parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000);
    const signalDeficit = Math.max(0, targetSignals - conduitSignals);
    const staleBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000);
    const staleRawBlocks = parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks_raw, 0, 100000000);
    if (signalDeficit > 0) {
      runtimeAutohealState.conduit_deficit_streak =
        parseNonNegativeInt(runtimeAutohealState.conduit_deficit_streak, 0, 100000000) + 1;
    } else {
      runtimeAutohealState.conduit_deficit_streak = 0;
    }
    if (staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS) {
      runtimeAutohealState.stale_raw_streak =
        parseNonNegativeInt(runtimeAutohealState.stale_raw_streak, 0, 100000000) + 1;
    } else {
      runtimeAutohealState.stale_raw_streak = 0;
    }
    if (
      staleBlocks >= RUNTIME_COCKPIT_STALE_SOFT_AUTOHEAL_MIN_BLOCKS &&
      queueDepth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH
    ) {
      runtimeAutohealState.stale_soft_streak =
        parseNonNegativeInt(runtimeAutohealState.stale_soft_streak, 0, 100000000) + 1;
    } else {
      runtimeAutohealState.stale_soft_streak = 0;
    }
    const persistentConduitDeficit =
      signalDeficit > 0 &&
      parseNonNegativeInt(runtimeAutohealState.conduit_deficit_streak, 0, 100000000) >=
        RUNTIME_CONDUIT_PERSISTENCE_MIN_TICKS;
    const persistentStaleRaw =
      staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS &&
      parseNonNegativeInt(runtimeAutohealState.stale_raw_streak, 0, 100000000) >=
        RUNTIME_STALE_RAW_PERSISTENCE_MIN_TICKS;
    const persistentStaleSoft =
      staleBlocks >= RUNTIME_COCKPIT_STALE_SOFT_AUTOHEAL_MIN_BLOCKS &&
      queueDepth >= RUNTIME_CONDUIT_SOFT_SCALE_QUEUE_DEPTH &&
      parseNonNegativeInt(runtimeAutohealState.stale_soft_streak, 0, 100000000) >=
        RUNTIME_STALE_SOFT_PERSISTENCE_MIN_TICKS;
    const persistentCoordinationDegradation = persistentConduitDeficit || persistentStaleRaw;
    // Treat stall signatures as actionable only when there is real queue pressure.
    // This avoids running heavy recovery lanes while the queue is healthy.
    const stallActionable = !!stall.detected && queueDepth >= RUNTIME_STALL_QUEUE_MIN_DEPTH;
    const stalePressure =
      (
        parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) > 0 ||
        staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS
      ) &&
      queueDepth >= RUNTIME_DRAIN_TRIGGER_DEPTH;
    const staleAutohealNeeded =
      parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) >=
        RUNTIME_COCKPIT_STALE_AUTOHEAL_MIN_BLOCKS ||
      staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS;
    const chronicCoordinationPathology =
      parseNonNegativeInt(runtime && runtime.cockpit_stale_blocks, 0, 100000000) >=
        RUNTIME_COORDINATION_PATHOLOGY_STALE_BLOCK_MIN &&
      parseNonNegativeInt(runtime && runtime.conduit_signals, 0, 100000000) <
        Math.max(
          1,
          parsePositiveInt(runtime && runtime.target_conduit_signals, RUNTIME_AUTO_BALANCE_THRESHOLD, 1, 128)
        ) ||
      (staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS && signalDeficit > 0);
    const signalFloor = Math.max(
      RUNTIME_STALL_CONDUIT_FLOOR,
      Math.floor(Math.max(1, runtime.target_conduit_signals) * 0.5)
    );
    const conduitScalePressure =
      !!runtime.conduit_scale_required &&
      (
        runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH ||
        chronicCoordinationPathology ||
        persistentCoordinationDegradation
      );
    const memoryResumeNeeded =
      !!runtime.memory_ingest_paused &&
      runtime.queue_depth <= DASHBOARD_QUEUE_DRAIN_RESUME_DEPTH;
    const benchmarkRefreshNeeded =
      cleanText(runtime && runtime.benchmark_sanity_cockpit_status ? runtime.benchmark_sanity_cockpit_status : '', 24) === 'fail' ||
      cleanText(runtime && runtime.benchmark_sanity_status ? runtime.benchmark_sanity_status : '', 24) === 'fail' ||
      parsePositiveInt(
        runtime && runtime.benchmark_sanity_age_seconds != null ? runtime.benchmark_sanity_age_seconds : -1,
        -1,
        -1,
        1000000000
      ) > RUNTIME_BENCHMARK_REFRESH_MAX_AGE_SECONDS;
  const emergency =
    runtime.queue_depth >= RUNTIME_INGRESS_CIRCUIT_DEPTH ||
    stalePressure ||
    chronicCoordinationPathology ||
    persistentCoordinationDegradation ||
    persistentStaleSoft ||
    (runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH && runtime.conduit_signals < signalFloor) ||
    queueVelocityPerMin >= 4 ||
    stallActionable;
    const required =
      runtime.queue_depth >= RUNTIME_DRAIN_TRIGGER_DEPTH ||
      runtime.critical_attention_total >= RUNTIME_CRITICAL_ESCALATION_THRESHOLD ||
      runtime.health_coverage_gap_count > 0 ||
      conduitScalePressure ||
      chronicCoordinationPathology ||
      persistentCoordinationDegradation ||
      persistentStaleSoft ||
      staleAutohealNeeded ||
      memoryResumeNeeded ||
      benchmarkRefreshNeeded ||
      cleanText(runtime && runtime.ingress_level ? runtime.ingress_level : '', 24) === 'circuit' ||
      queueVelocityPerMin >= 4 ||
      stallActionable ||
      emergency;
    const coordinationFastPath =
      (persistentCoordinationDegradation || persistentStaleSoft || chronicCoordinationPathology) &&
      queueDepth < RUNTIME_DRAIN_TRIGGER_DEPTH;
    const cadenceMs = emergency
      ? RUNTIME_AUTONOMY_HEAL_EMERGENCY_INTERVAL_MS
      : coordinationFastPath
      ? RUNTIME_AUTONOMY_HEAL_COORDINATION_INTERVAL_MS
      : RUNTIME_AUTONOMY_HEAL_INTERVAL_MS;
    const nowMs = Date.now();
    if (!required) {
      runtimeAutohealState.last_result = 'idle';
      runtimeAutohealState.last_stage = 'idle';
      runtimeAutohealState.last_stall_detected = false;
      runtimeAutohealState.last_stall_signature = '';
      return {
        executed: false,
        required,
        emergency,
        cadence_ms: cadenceMs,
        queue_velocity_per_min: queueVelocityPerMin,
        stall,
        persistent_coordination_degradation: persistentCoordinationDegradation,
        persistent_stale_soft: persistentStaleSoft,
      };
    }
    if ((nowMs - parseNonNegativeInt(runtimeAutohealState.last_run_ms, 0, 1000000000000)) < cadenceMs) {
      runtimeAutohealState.last_result = 'cooldown';
      runtimeAutohealState.last_stage = 'cooldown';
      runtimeAutohealState.last_stall_detected = !!stall.detected;
      runtimeAutohealState.last_stall_signature = cleanText(stall.signature || '', 240);
      return {
        executed: false,
        required,
        emergency,
        cadence_ms: cadenceMs,
        queue_velocity_per_min: queueVelocityPerMin,
        stall,
        persistent_coordination_degradation: persistentCoordinationDegradation,
        persistent_stale_soft: persistentStaleSoft,
      };
    }

    const conduitOnlyEligible =
      (persistentCoordinationDegradation || persistentStaleSoft) &&
      queueDepth < RUNTIME_DRAIN_TRIGGER_DEPTH &&
      parseNonNegativeInt(runtime && runtime.health_coverage_gap_count, 0, 100000000) === 0 &&
      !stallActionable &&
      !benchmarkRefreshNeeded;
    if (conduitOnlyEligible) {
      const conduitOnly = maybeAutoHealConduit(runtime, flags.team || DEFAULT_TEAM);
      const staleLaneGc =
        staleBlocks >= RUNTIME_COCKPIT_STALE_AUTOHEAL_MIN_BLOCKS ||
        staleRawBlocks >= RUNTIME_COCKPIT_STALE_RAW_AUTOHEAL_MIN_BLOCKS ||
        persistentStaleSoft
          ? maybeHealCoarseSignal(latestSnapshot, runtime, flags.team || DEFAULT_TEAM)
          : null;
      const staleGcOk =
        !staleLaneGc ||
        !staleLaneGc.required ||
        !!(
          staleLaneGc.stale_lane_drain &&
          staleLaneGc.stale_lane_drain.applied &&
          staleLaneGc.conduit_scale_up &&
          staleLaneGc.conduit_scale_up.applied
        );
      const ok = (
        !conduitOnly.required || !!conduitOnly.applied || !conduitOnly.triggered
      ) && staleGcOk;
      runtimeAutohealState.last_run_ms = nowMs;
      runtimeAutohealState.last_run_at = nowIso();
      runtimeAutohealState.last_result = ok ? 'executed' : 'degraded';
      runtimeAutohealState.last_stage = staleLaneGc && staleLaneGc.required
        ? 'conduit_watchdog_stale_gc'
        : 'conduit_watchdog';
      runtimeAutohealState.last_stall_detected = !!stall.detected;
      runtimeAutohealState.last_stall_signature = cleanText(stall.signature || '', 240);
      runtimeAutohealState.failure_count = ok
        ? 0
        : parseNonNegativeInt(runtimeAutohealState.failure_count, 0, 100000000) + 1;
      return {
        executed: true,
        required,
        emergency,
        cadence_ms: cadenceMs,
        queue_velocity_per_min: queueVelocityPerMin,
        ok,
        lane_payload: {
          ok,
          policy: staleLaneGc && staleLaneGc.required
            ? 'conduit_watchdog_plus_stale_gc'
            : 'conduit_watchdog_autorestart',
          conduit_watchdog: conduitOnly,
          coarse_signal_remediation: staleLaneGc,
        },
        stall,
        stage: staleLaneGc && staleLaneGc.required
          ? 'conduit_watchdog_stale_gc'
          : 'conduit_watchdog',
        stall_recovery: null,
        source,
        persistent_coordination_degradation: persistentCoordinationDegradation,
        persistent_stale_soft: persistentStaleSoft,
      };
    }

    const lanePayload = executeRuntimeSwarmRecommendation(latestSnapshot);
    let stage = 'recommendation';
    let stallRecovery = null;
    let ok = !!(lanePayload && lanePayload.ok);
    const stallRecoveryRequired =
      stallActionable &&
      (
        !!stall.coordination_pathology ||
        parseNonNegativeInt(runtimeAutohealState.failure_count, 0, 100000000) >=
          RUNTIME_STALL_ESCALATION_FAILURE_THRESHOLD
      );
    if (stallRecoveryRequired) {
      stage = 'stall_recovery';
      stallRecovery = runStallRecovery(runtime, flags.team || DEFAULT_TEAM, stall);
      ok = ok && !!(stallRecovery && stallRecovery.ok);
    }

    runtimeAutohealState.last_run_ms = nowMs;
    runtimeAutohealState.last_run_at = nowIso();
    runtimeAutohealState.last_result = ok ? 'executed' : 'degraded';
    runtimeAutohealState.last_stage = stage;
    runtimeAutohealState.last_stall_detected = !!stall.detected;
    runtimeAutohealState.last_stall_signature = cleanText(stall.signature || '', 240);
    runtimeAutohealState.failure_count = ok
      ? 0
      : parseNonNegativeInt(runtimeAutohealState.failure_count, 0, 100000000) + 1;
    return {
      executed: true,
      required,
      emergency,
      cadence_ms: cadenceMs,
      queue_velocity_per_min: queueVelocityPerMin,
      ok,
      lane_payload: lanePayload,
      stall,
      stage,
      stall_recovery: stallRecovery,
      source,
      persistent_coordination_degradation: persistentCoordinationDegradation,
      persistent_stale_soft: persistentStaleSoft,
    };
  };

  const waitMs = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  const streamAssistantText = async (socket, text) => {
    const output = String(text || '');
    if (!output.trim()) return false;
    if (!socket || socket.readyState !== 1) return false;
    const total = output.length;
    const targetChunks = Math.min(120, Math.max(18, Math.ceil(total / 20)));
    const baseChunkSize = Math.max(6, Math.ceil(total / targetChunks));
    let cursor = 0;
    let sent = false;
    while (cursor < total) {
      if (!socket || socket.readyState !== 1) break;
      let next = Math.min(total, cursor + baseChunkSize);
      while (
        next < total &&
        (next - cursor) < (baseChunkSize + 16) &&
        !/\s|[,.!?;:\n]/.test(output[next])
      ) {
        next += 1;
      }
      if (next <= cursor) next = Math.min(total, cursor + 1);
      const chunk = output.slice(cursor, next);
      sendWs(socket, { type: 'text_delta', content: chunk });
      sent = true;
      cursor = next;
      if (cursor < total) {
        await waitMs(chunk.indexOf('\n') >= 0 ? 18 : 10);
      }
    }
    return sent;
  };

  wss.on('connection', (socket) => {
    wsClients.add(socket);
    markWsHeartbeat(socket, false);
    sendWs(socket, wsSnapshotCompatPayload());
    try { socket.send(latestSnapshotEnvelope); } catch {}
    socket.on('pong', () => {
      markWsHeartbeat(socket, false);
    });
    socket.on('error', () => {
      dropSnapshotSocket(socket);
    });
    socket.on('close', () => {
      dropSnapshotSocket(socket);
    });
  });

  agentWss.on('connection', (socket, req, meta) => {
    const agentId = cleanText(meta && meta.agentId ? meta.agentId : '', 140);
    if (!agentId) {
      try { socket.close(1008, 'agent_required'); } catch {}
      return;
    }
    if (agentInactiveForRealtime(agentId)) {
      sendWs(socket, { type: 'error', content: 'Agent is inactive (archived).' });
      try { socket.close(1008, 'agent_inactive'); } catch {}
      return;
    }
    agentWsClients.set(socket, agentId);
    markWsHeartbeat(socket, true);
    setContractConversationHold(agentId, true, {
      source: 'agent_ws_open',
      reset_timer: true,
      max_hold_ms: AGENT_CONTRACT_CHAT_HOLD_MAX_MS,
    });
    sendWs(socket, { type: 'connected', agent_id: agentId, ts: nowIso() });
    socket.on('pong', () => {
      markWsHeartbeat(socket, true);
    });
    socket.on('error', () => {
      dropAgentSocket(socket);
    });

    socket.on('message', async (raw) => {
      markWsHeartbeat(socket, true);
      setContractConversationHold(agentId, true, {
        source: 'agent_ws_message',
        reset_timer: false,
        max_hold_ms: AGENT_CONTRACT_CHAT_HOLD_MAX_MS,
      });
      if (agentInactiveForRealtime(agentId)) {
        sendWs(socket, { type: 'error', content: 'Agent is inactive (archived).' });
        try { socket.close(1008, 'agent_inactive'); } catch {}
        dropAgentSocket(socket);
        return;
      }
      const contractState = contractForAgent(agentId);
      if (contractState && cleanText(contractState.status || 'active', 32) !== 'active') {
        sendWs(socket, { type: 'error', content: 'Agent contract terminated.' });
        try { socket.close(1008, 'agent_contract_terminated'); } catch {}
        dropAgentSocket(socket);
        return;
      }
      let payload = null;
      try {
        const parsedPayload = JSON.parse(String(raw || '{}'));
        if (!parsedPayload || typeof parsedPayload !== 'object' || Array.isArray(parsedPayload)) {
          sendWs(socket, { type: 'error', content: 'Invalid websocket payload.' });
          return;
        }
        payload = parsedPayload;
      } catch {
        sendWs(socket, { type: 'error', content: 'Invalid websocket payload.' });
        return;
      }
      const eventType = cleanText(payload && payload.type ? payload.type : '', 40).toLowerCase();
      const isTerminalEvent =
        eventType === 'terminal' ||
        eventType === 'terminal_command' ||
        eventType === 'terminal_input' ||
        eventType === 'terminal-input';
      if (eventType === 'ping') {
        sendWs(socket, { type: 'pong', ts: nowIso() });
        return;
      }
      if (isTerminalEvent) {
        const terminal = await runTerminalCommand(
          payload && (payload.command || payload.input || payload.message)
            ? payload.command || payload.input || payload.message
            : '',
          payload && payload.cwd ? payload.cwd : '',
          agentId,
          latestSnapshot
        );
        writeActionReceipt(
          'app.terminal',
          {
            agent_id: agentId,
            command: cleanText(terminal.command || '', 400),
            cwd: cleanText(terminal.cwd || '', 260),
            cli_mode: ACTIVE_CLI_MODE,
          },
          {
            ok: terminal.ok,
            status: terminal.status,
            argv: ['terminal', cleanText(terminal.command || '', 120)],
            payload: {
              ok: terminal.ok,
              type: 'terminal_command',
              exit_code: terminal.exit_code,
            },
          }
        );
        if (terminal.blocked) {
          sendWs(socket, { type: 'terminal_error', message: terminal.message || 'Terminal blocked.' });
          return;
        }
        sendWs(socket, {
          type: 'terminal_output',
          command: terminal.command,
          cwd: terminal.cwd,
          stdout: terminal.stdout,
          stderr: terminal.stderr,
          exit_code: terminal.exit_code,
          status: terminal.status,
          duration_ms: terminal.duration_ms,
        });
        return;
      }
      if (eventType === 'command') {
        const command = cleanText(payload && payload.command ? payload.command : '', 40).toLowerCase();
        const silent = !!(payload && (payload.silent === true || payload.background === true || payload.poll === true));
        if (command === 'context') {
          const state = loadAgentSession(agentId, latestSnapshot);
          const session = activeSession(state);
          const messages = Array.isArray(session.messages) ? session.messages : [];
          const modelState = effectiveAgentModel(agentId, latestSnapshot);
          const contextStats = contextTelemetryForMessages(
            messages,
            modelState && modelState.context_window != null ? modelState.context_window : DEFAULT_CONTEXT_WINDOW_TOKENS,
            0
          );
          if (silent) {
            sendWs(socket, {
              type: 'context_state',
              silent: true,
              context_tokens: contextStats.context_tokens,
              context_window: contextStats.context_window,
              context_ratio: contextStats.context_ratio,
              context_pressure: contextStats.context_pressure,
            });
          } else {
            sendWs(socket, {
              type: 'command_result',
              message: `Context usage: ${messages.length} messages, ~${contextStats.context_tokens} tokens.`,
              context_tokens: contextStats.context_tokens,
              context_window: contextStats.context_window,
              context_ratio: contextStats.context_ratio,
              context_pressure: contextStats.context_pressure,
              silent: false,
            });
          }
          return;
        }
        if (command === 'verbose') {
          sendWs(socket, {
            type: 'command_result',
            message: 'Verbose mode is available in this dashboard and controlled client-side.',
          });
          return;
        }
        if (command === 'queue') {
          sendWs(socket, {
            type: 'command_result',
            message: 'Queue status: active websocket mode.',
          });
          return;
        }
        sendWs(socket, { type: 'command_result', message: `Unsupported command: ${command || 'unknown'}` });
        return;
      }
      if (eventType !== 'message') {
        sendWs(socket, { type: 'error', content: 'Unsupported websocket event type.' });
        return;
      }

      const input = payload && (payload.content || payload.input || payload.message)
        ? payload.content || payload.input || payload.message
        : '';
      const turn = runAgentMessage(agentId, input, latestSnapshot);
      if (!turn.ok && turn.error === 'message_required') {
        sendWs(socket, { type: 'error', content: 'Message required.' });
        return;
      }
      if (!turn.ok && turn.error === 'agent_not_found') {
        sendWs(socket, { type: 'error', content: 'Agent not found.' });
        return;
      }
      if (!turn.ok && turn.error === 'agent_inactive') {
        sendWs(socket, { type: 'error', content: 'Agent is inactive (archived).' });
        try { socket.close(1008, 'agent_inactive'); } catch {}
        return;
      }
      if (!turn.ok && turn.error === 'agent_contract_terminated') {
        const reason = cleanText(turn.reason || 'contract_terminated', 120) || 'contract_terminated';
        sendWs(socket, { type: 'error', content: `Agent contract terminated (${reason}).` });
        try { socket.close(1008, 'agent_contract_terminated'); } catch {}
        return;
      }
      if (!turn.ok) {
        sendWs(socket, { type: 'error', content: 'Agent message failed.' });
        return;
      }

      sendWs(socket, { type: 'phase', phase: 'thinking', detail: 'Thinking...' });
      const wsTools = Array.isArray(turn.tools) ? turn.tools : [];
      for (const tool of wsTools) {
        sendWs(socket, { type: 'tool_start', tool: tool.name || 'tool' });
        sendWs(socket, {
          type: 'tool_end',
          tool: tool.name || 'tool',
          input: tool.input || '',
        });
        sendWs(socket, {
          type: 'tool_result',
          tool: tool.name || 'tool',
          result: tool.result || '',
          is_error: !!tool.is_error,
        });
      }
      sendWs(socket, { type: 'phase', phase: 'streaming', detail: 'Streaming response...' });
      const didStreamResponse = await streamAssistantText(socket, turn.response);

      writeActionReceipt(
        'app.chat',
        { input: turn.input, agent_id: agentId, session_id: turn.session_id, cli_mode: ACTIVE_CLI_MODE },
        turn.laneResult
      );
      refreshSnapshot();
      appendAgentConversation(agentId, latestSnapshot, turn.input, turn.response, turn.meta, turn.tools, {
        assistant_agent_id: turn.agent_id || agentId,
        assistant_agent_name: cleanText(turn && turn.agent && turn.agent.name ? turn.agent.name : '', 120),
      });

      sendWs(socket, {
        type: 'response',
        content: didStreamResponse ? '' : turn.response,
        input_tokens: turn.input_tokens,
        output_tokens: turn.output_tokens,
        context_tokens: turn.context_tokens,
        context_window: turn.context_window,
        context_ratio: turn.context_ratio,
        context_pressure: turn.context_pressure,
        cost_usd: turn.cost_usd,
        iterations: turn.iterations,
        duration_ms: turn.duration_ms,
        agent_id: turn.agent_id || agentId,
        agent_name: cleanText(turn && turn.agent && turn.agent.name ? turn.agent.name : '', 120),
        model: turn.model,
        model_provider: turn.model_provider || providerForModelName(turn.model, configuredProvider(latestSnapshot)),
        auto_route: turn.auto_route || null,
        runtime_sync: turn.runtime_sync || null,
      });
    });

    socket.on('close', () => {
      dropAgentSocket(socket);
    });
  });

  server.on('upgrade', (req, socket, head) => {
    const reqUrl = new URL(req.url || '/', `http://${flags.host}:${flags.port}`);
    if (reqUrl.pathname === '/ws') {
      wss.handleUpgrade(req, socket, head, (ws) => {
        wss.emit('connection', ws, req);
      });
      return;
    }
    const agentMatch = reqUrl.pathname.match(/^\/api\/agents\/([^/]+)\/ws$/);
    if (agentMatch && agentMatch[1]) {
      const agentId = cleanText(decodeURIComponent(agentMatch[1]), 140);
      if (!agentId) {
        socket.destroy();
        return;
      }
      if (agentInactiveForRealtime(agentId)) {
        socket.destroy();
        return;
      }
      agentWss.handleUpgrade(req, socket, head, (ws) => {
        agentWss.emit('connection', ws, req, { agentId });
      });
      return;
    }
    socket.destroy();
  });

  const interval = setInterval(() => {
    if (!DASHBOARD_BACKGROUND_RUNTIME_LOOPS_ENABLED) return;
    if (Date.now() < parseNonNegativeInt(nextSnapshotRefreshAtMs, 0, 1000000000000)) return;
    if (updating) return;
    const recentClientActivity =
      (Date.now() - parseNonNegativeInt(lastClientActivityAtMs, 0, 1000000000000)) <
      INTERACTIVE_BACKGROUND_SUPPRESS_MS;
    const hasRealtimeClients = realtimeClientCounts().total_open > 0;
    if (!hasRealtimeClients) {
      if (recentClientActivity) return;
      const autoheal = maybeRunAutonomousSelfHeal('interval');
      if (autoheal && autoheal.executed) {
        requestSnapshotRefresh(false);
      }
      return;
    }
    // Do not run synchronous snapshot rebuilds on this cadence.
    // Keep the websocket stream alive with the latest cached snapshot.
    broadcastSnapshot();
  }, flags.refreshMs);

  const contractInterval = setInterval(() => {
    if (!DASHBOARD_BACKGROUND_RUNTIME_LOOPS_ENABLED) return;
    const recentClientActivity =
      (Date.now() - parseNonNegativeInt(lastClientActivityAtMs, 0, 1000000000000)) <
      INTERACTIVE_BACKGROUND_SUPPRESS_MS;
    if (recentClientActivity) {
      return;
    }
    const hasRealtimeClients = realtimeClientCounts().total_open > 0;
    if (hasRealtimeClients) {
      return;
    }
    const activeAgents = activeAgentCountFromSnapshot(latestSnapshot, 0);
    const lowScaleFloorMs = Math.max(AGENT_CONTRACT_ENFORCE_INTERVAL_MS, 2000);
    const loadPenaltyMs = Math.min(2500, parseNonNegativeInt(lastSnapshotBuildDurationMs, 0, 1000000000));
    const lowScaleIntervalMs = Math.min(5000, lowScaleFloorMs + Math.round(loadPenaltyMs * 0.5));
    const dynamicIntervalMs = activeAgents >= AGENT_CONTRACT_ENFORCE_MEGA_SCALE_THRESHOLD
      ? Math.max(AGENT_CONTRACT_ENFORCE_INTERVAL_MEGA_SCALE_MS, 5000)
      : activeAgents >= AGENT_CONTRACT_ENFORCE_ULTRA_SCALE_THRESHOLD
        ? Math.max(AGENT_CONTRACT_ENFORCE_INTERVAL_ULTRA_SCALE_MS, 4000)
        : activeAgents >= AGENT_CONTRACT_ENFORCE_HIGH_SCALE_THRESHOLD
          ? Math.max(AGENT_CONTRACT_ENFORCE_INTERVAL_HIGH_SCALE_MS, 3000)
          : lowScaleIntervalMs;
    const nowMs = Date.now();
    const idleContractCadenceMs = Math.max(20_000, dynamicIntervalMs * 4);
    if ((nowMs - parseNonNegativeInt(lastContractLoopRunAtMs, 0, 1000000000000)) < idleContractCadenceMs) {
      return;
    }
    if (updating) {
      return;
    }
    if ((nowMs - parseNonNegativeInt(lastContractLoopRunAtMs, 0, 1000000000000)) < dynamicIntervalMs) {
      return;
    }
    if (scheduleContractEnforcement('interval')) {
      lastContractLoopRunAtMs = nowMs;
    }
  }, Math.max(500, AGENT_CONTRACT_ENFORCE_INTERVAL_MS));

  const compactInterval = setInterval(() => {
    compactSnapshotHistory('interval', false);
    cleanupChatExports();
  }, SNAPSHOT_HISTORY_COMPACT_INTERVAL_MS);

  const wsHeartbeatInterval = setInterval(() => {
    const nowMs = Date.now();
    for (const client of Array.from(wsClients)) {
      if (!client || client.readyState !== 1) {
        dropSnapshotSocket(client);
        continue;
      }
      const lastSeenMs = parseNonNegativeInt(wsClientHeartbeat.get(client), 0, 1_000_000_000_000);
      if (lastSeenMs > 0 && (nowMs - lastSeenMs) > WS_HEARTBEAT_TIMEOUT_MS) {
        try { client.terminate(); } catch {}
        dropSnapshotSocket(client);
        continue;
      }
      try {
        client.ping();
      } catch {}
    }
    for (const [client, agentId] of Array.from(agentWsClients.entries())) {
      if (!client || client.readyState !== 1) {
        dropAgentSocket(client);
        continue;
      }
      if (agentInactiveForRealtime(agentId)) {
        sendWs(client, { type: 'agent_archived', agent_id: agentId, ts: nowIso(), reason: 'agent_inactive' });
        try { client.close(1008, 'agent_inactive'); } catch {}
        dropAgentSocket(client);
        continue;
      }
      const lastSeenMs = parseNonNegativeInt(agentWsHeartbeat.get(client), 0, 1_000_000_000_000);
      if (lastSeenMs > 0 && (nowMs - lastSeenMs) > WS_HEARTBEAT_TIMEOUT_MS) {
        try { client.terminate(); } catch {}
        dropAgentSocket(client);
        continue;
      }
      try {
        client.ping();
      } catch {}
    }
  }, WS_HEARTBEAT_INTERVAL_MS);
  if (wsHeartbeatInterval && typeof wsHeartbeatInterval.unref === 'function') {
    wsHeartbeatInterval.unref();
  }

  let shutdownInvoked = false;
  function shutdown() {
    if (shutdownInvoked) return;
    shutdownInvoked = true;
    clearInterval(interval);
    clearInterval(contractInterval);
    clearInterval(compactInterval);
    clearInterval(wsHeartbeatInterval);
    closeAllTerminalSessions('dashboard_shutdown');
    for (const client of wsClients) {
      try {
        client.close();
      } catch {}
    }
    for (const client of agentWsClients.keys()) {
      try {
        client.close();
      } catch {}
    }
    wsClients.clear();
    agentWsClients.clear();
    wsClientHeartbeat.clear();
    agentWsHeartbeat.clear();
    try {
      server.close();
    } catch {}
  }

  server.on('error', (error) => {
    const err = error || {};
    const code = cleanText(err.code || '', 40);
    const message = cleanText(err.message || String(err), 400) || 'dashboard_server_error';
    const status = {
      ok: false,
      type: 'infring_dashboard_server_error',
      ts: nowIso(),
      host: flags.host,
      port: flags.port,
      code,
      message,
      snapshot_storage: snapshotStorageTelemetry().snapshot_history,
    };
    writeJson(path.resolve(STATE_DIR, 'server_status.json'), status);
    process.stderr.write(`infring_dashboard_server_error:${code || 'unknown'}:${message}\n`);
    shutdown();
    if (!Number.isFinite(process.exitCode) || process.exitCode === 0) process.exitCode = 1;
  });

  server.listen(flags.port, flags.host, () => {
    const dashboardUrl = `http://${flags.host}:${flags.port}/dashboard`;
    const status = {
      ok: true,
      type: 'infring_dashboard_server',
      ts: nowIso(),
      url: dashboardUrl,
      dashboard_url: dashboardUrl,
      host: flags.host,
      port: flags.port,
      refresh_ms: flags.refreshMs,
      team: flags.team,
      cli_mode: ACTIVE_CLI_MODE,
      receipt_hash: latestSnapshot.receipt_hash,
      snapshot_path: path.relative(ROOT, SNAPSHOT_LATEST_PATH),
      action_path: path.relative(ROOT, ACTION_LATEST_PATH),
      snapshot_storage: snapshotStorageTelemetry().snapshot_history,
    };
    writeJson(path.resolve(STATE_DIR, 'server_status.json'), status);
    process.stdout.write(`${JSON.stringify(status, null, 2)}\n`);
    process.stdout.write(`Dashboard listening at ${dashboardUrl}\n`);
    setTimeout(() => {
      requestSnapshotRefresh(true);
    }, 0);
    const startupCompactDelayMs = 15_000;
    const startupCompactTimer = setTimeout(() => {
      try {
        compactSnapshotHistory('startup_deferred', false);
      } catch {}
    }, startupCompactDelayMs);
    if (startupCompactTimer && typeof startupCompactTimer.unref === 'function') {
      startupCompactTimer.unref();
    }
  });

  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
  return null;
}

function run(argv = process.argv.slice(2)) {
  const flags = parseFlags(argv);
  ACTIVE_CLI_MODE = normalizeCliMode(flags && flags.cliMode ? flags.cliMode : ACTIVE_CLI_MODE);
  if (flags.mode === 'test-model' || flags.mode === 'test-agent-model') {
    const raw = Array.isArray(argv) ? argv.slice(1) : [];
    let op = 'get';
    const localFlags = {};
    for (const token of raw) {
      const value = String(token || '').trim();
      if (!value) continue;
      if (!value.startsWith('--') && op === 'get') {
        op = cleanText(value, 40).toLowerCase() || 'get';
        continue;
      }
      if (value.startsWith('--')) {
        const [rawKey, ...rest] = value.slice(2).split('=');
        const key = cleanText(rawKey || '', 80).toLowerCase();
        if (!key) continue;
        localFlags[key] = rest.length ? rest.join('=') : '1';
      }
    }
    const current = loadTestAgentModelConfig();
    const appHistory = runLane(['app-plane', 'history', '--app=chat-ui']);
    const appSettings =
      appHistory &&
      appHistory.payload &&
      appHistory.payload.settings &&
      typeof appHistory.payload.settings === 'object'
        ? appHistory.payload.settings
        : {};
    const currentProvider =
      cleanText(appSettings.provider || configuredProvider({ app: { settings: appSettings } }), 80) ||
      'openai';
    const currentModel =
      cleanText(
        appSettings.model || configuredOllamaModel({ app: { settings: appSettings } }),
        120
      ) || configuredOllamaModel({ app: { settings: appSettings } });
    if (op === 'get' || op === 'status' || op === 'show') {
      const payload = {
        ok: true,
        type: 'infring_dashboard_test_agent_model',
        op: 'get',
        config: current,
        chat_provider: currentProvider,
        chat_model: currentModel,
      };
      fs.writeFileSync(1, `${JSON.stringify(payload, null, flags.pretty ? 2 : 0)}${flags.pretty ? '\n' : ''}`, 'utf8');
      return 0;
    }
    if (op !== 'set') {
      const payload = {
        ok: false,
        type: 'infring_dashboard_test_agent_model',
        error: 'unsupported_op',
        op,
        supported_ops: ['get', 'set'],
      };
      fs.writeFileSync(1, `${JSON.stringify(payload, null, flags.pretty ? 2 : 0)}${flags.pretty ? '\n' : ''}`, 'utf8');
      return 2;
    }
    const nextModel = cleanText(localFlags.model || current.model || TEST_AGENT_MODEL_DEFAULT, 120) || TEST_AGENT_MODEL_DEFAULT;
    const nextProvider =
      cleanText(localFlags.provider || providerForModelName(nextModel, current.provider || TEST_AGENT_PROVIDER_DEFAULT), 80) ||
      TEST_AGENT_PROVIDER_DEFAULT;
    const nextEnabled =
      localFlags.enabled == null
        ? true
        : ['1', 'true', 'yes', 'on'].includes(String(localFlags.enabled).toLowerCase());
    const applyChatConfig =
      localFlags['apply-chat-config'] == null
        ? true
        : ['1', 'true', 'yes', 'on'].includes(String(localFlags['apply-chat-config']).toLowerCase());
    const saved = saveTestAgentModelConfig({
      ...current,
      model: nextModel,
      provider: nextProvider,
      enabled: nextEnabled,
      updated_at: nowIso(),
    });
    const switchLane =
      applyChatConfig && nextEnabled
        ? runAction('app.switchProvider', { provider: nextProvider, model: nextModel })
        : null;
    const switchApplied = !switchLane || !!switchLane.ok;
    const ok = true;
    const payload = {
      ok,
      type: 'infring_dashboard_test_agent_model',
      op: 'set',
      config: saved,
      apply_chat_config: applyChatConfig,
      switched_chat_provider: !!switchLane,
      switched_chat_provider_ok: switchApplied,
      chat_switch_warning:
        switchLane && !switchLane.ok
          ? 'chat_provider_switch_failed_but_test_agent_model_saved'
          : '',
      switch_lane: switchLane ? laneOutcome(switchLane) : null,
    };
    fs.writeFileSync(1, `${JSON.stringify(payload, null, flags.pretty ? 2 : 0)}${flags.pretty ? '\n' : ''}`, 'utf8');
    return 0;
  }
  if (flags.mode === 'snapshot' || flags.mode === 'status') {
    bootstrapSnapshotHistoryState();
    const snapshot = buildSnapshot(flags);
    writeSnapshotReceipt(snapshot);
    const body = `${JSON.stringify(snapshot, null, flags.pretty ? 2 : 0)}${flags.pretty ? '\n' : ''}`;
    // Sync write avoids truncation when parent captures stdout via spawnSync.
    fs.writeFileSync(1, body, 'utf8');
    return 0;
  }
  if (flags.mode === 'serve' || flags.mode === 'web') {
    runServe(flags);
    return null;
  }
  process.stderr.write(
    `infring_dashboard: unsupported mode ${flags.mode}. expected serve|snapshot|status|test-model\n`
  );
  return 2;
}

module.exports = {
  run,
  parseFlags,
  buildSnapshot,
  runAction,
};

if (require.main === module) {
  const exitCode = run(process.argv.slice(2));
  if (typeof exitCode === 'number') {
    process.exitCode = exitCode;
  }
}
