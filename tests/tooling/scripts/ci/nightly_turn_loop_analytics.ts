#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const SESSIONS_DIR = path.join(
  ROOT,
  'client/runtime/local/state/ui/infring_dashboard/agent_sessions',
);
const PROVIDER_REGISTRY_PATH = path.join(
  ROOT,
  'client/runtime/local/state/ui/infring_dashboard/provider_registry.json',
);
const TRACKING_TUNING_PATH = path.join(
  ROOT,
  'local/state/ops/session_command_tracking/nightly_tuning.json',
);
const ADOPTION_REPORT_PATH = path.join(
  ROOT,
  'local/state/ops/session_command_tracking/nightly_adoption_report.json',
);

function nowIso(): string {
  return new Date().toISOString();
}

function tsSlug(iso: string): string {
  return iso.replaceAll(':', '-').replaceAll('.', '-');
}

function cleanText(raw: unknown, maxLen = 240): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function readJson(pathname: string): any {
  try {
    const raw = fs.readFileSync(pathname, 'utf8');
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function writeJson(pathname: string, payload: any): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function extractLastJsonLine(raw: string): any {
  const lines = String(raw || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    const candidate = lines[idx];
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {
      continue;
    }
  }
  return null;
}

function runOpsKernel(command: string, payload: any) {
  const payloadArg = `--payload=${JSON.stringify(payload || {})}`;
  const out = spawnSync(
    'cargo',
    [
      'run',
      '--quiet',
      '--manifest-path',
      'core/layer0/ops/Cargo.toml',
      '--bin',
      'protheus-ops',
      '--',
      ...command.split(' '),
      payloadArg,
    ],
    {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );
  const parsed = extractLastJsonLine(String(out.stdout || ''));
  const receiptPayload = parsed && parsed.payload ? parsed.payload : parsed;
  return {
    ok: (out.status ?? 1) === 0 && !!parsed,
    status: Number.isFinite(out.status) ? out.status : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    receipt: parsed,
    payload: receiptPayload || {},
  };
}

function isCommandLike(line: string): boolean {
  const first = cleanText(line, 200).split(' ')[0].toLowerCase();
  return [
    'git',
    'gh',
    'cargo',
    'npm',
    'npx',
    'pnpm',
    'python',
    'pytest',
    'node',
    'ls',
    'cat',
    'rg',
    'grep',
    'find',
    'tree',
    'curl',
    'wget',
    'docker',
    'kubectl',
    'infring',
    'infringctl',
    'protheus-ops',
  ].includes(first);
}

function collectSessionCommands() {
  const sessions: Array<{ session_id: string; commands: string[] }> = [];
  if (!fs.existsSync(SESSIONS_DIR)) return sessions;
  const files = fs
    .readdirSync(SESSIONS_DIR)
    .filter((name) => name.endsWith('.json'))
    .sort();
  for (const fileName of files) {
    const sessionDoc = readJson(path.join(SESSIONS_DIR, fileName));
    const agentId = cleanText(sessionDoc?.agent_id || fileName.replace(/\.json$/, ''), 120);
    const activeSessionId = cleanText(sessionDoc?.active_session_id || 'default', 120) || 'default';
    const allSessions = Array.isArray(sessionDoc?.sessions) ? sessionDoc.sessions : [];
    const activeSession =
      allSessions.find((row: any) => cleanText(row?.session_id, 120) === activeSessionId) || null;
    const messages = Array.isArray(activeSession?.messages) ? activeSession.messages : [];
    const commands: string[] = [];
    for (const msg of messages) {
      const role = cleanText(msg?.role, 40).toLowerCase();
      if (role !== 'user' && role !== 'terminal') continue;
      const text = String(msg?.text || msg?.content || '');
      for (const line of text.split('\n')) {
        const candidate = cleanText(line.trim().replace(/^\$\s+/, ''), 220);
        if (!candidate || !isCommandLike(candidate)) continue;
        if (commands.includes(candidate)) continue;
        commands.push(candidate);
        if (commands.length >= 16) break;
      }
      if (commands.length >= 16) break;
    }
    if (commands.length) {
      sessions.push({
        session_id: agentId || activeSessionId,
        commands,
      });
    }
  }
  return sessions;
}

function classifyCommandProfile(commands: string[]) {
  let code = 0;
  let infra = 0;
  let web = 0;
  for (const row of commands) {
    const first = cleanText(row, 200).split(' ')[0].toLowerCase();
    if (['git', 'gh', 'cargo', 'npm', 'npx', 'pnpm', 'python', 'pytest', 'node', 'rg', 'grep'].includes(first)) {
      code += 1;
    } else if (['docker', 'kubectl'].includes(first)) {
      infra += 1;
    } else if (['curl', 'wget'].includes(first)) {
      web += 1;
    }
  }
  return { code, infra, web };
}

function deriveDefaultBudgetMode(adoptionPct: number, unsupported: number, outputTokens: number): string {
  if (unsupported > 0 && adoptionPct < 70) return 'cheap';
  if (adoptionPct >= 92 && outputTokens >= 2000) return 'quality';
  return 'balanced';
}

function deriveModelBias(profile: { code: number; infra: number; web: number }) {
  const registry = readJson(PROVIDER_REGISTRY_PATH) || {};
  const providers = Array.isArray(registry?.providers) ? registry.providers : [];
  const out: Record<string, number> = {};
  for (const provider of providers) {
    const providerId = cleanText(provider?.id, 80).toLowerCase();
    const modelProfiles = provider && provider.model_profiles && typeof provider.model_profiles === 'object'
      ? provider.model_profiles
      : {};
    for (const [modelName, modelProfile] of Object.entries(modelProfiles)) {
      const model = cleanText(modelName, 160);
      if (!model) continue;
      const modelId = `${providerId}/${model}`;
      const specialty = cleanText((modelProfile as any)?.specialty || '', 80).toLowerCase();
      let bias = 0;
      if (profile.code > profile.infra && profile.code > 0 && specialty.includes('code')) bias += 0.8;
      if (profile.infra > profile.code && profile.infra > 0 && (specialty.includes('infra') || specialty.includes('ops'))) bias += 0.7;
      if (profile.web > 0 && specialty.includes('research')) bias += 0.5;
      if (bias > 0) out[modelId] = Number(bias.toFixed(2));
    }
  }
  return out;
}

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

const startedAt = nowIso();
const tracking = runOpsKernel('session-command-tracking-kernel summary', { since_days: 14 });
const trackingPayload = tracking.payload && typeof tracking.payload === 'object' ? tracking.payload : {};
const sessions = collectSessionCommands();
const adoptionInput =
  sessions.length > 0
    ? { sessions, limit: 50 }
    : {
        session_id: 'global',
        commands: (Array.isArray(trackingPayload.top_segments) ? trackingPayload.top_segments : [])
          .map((row: any) => cleanText(row?.segment, 220))
          .filter(Boolean)
          .slice(0, 12),
      };
const adoption = runOpsKernel('session-command-session-analytics-kernel adoption-report', adoptionInput);
const adoptionPayload = adoption.payload && typeof adoption.payload === 'object' ? adoption.payload : {};

const adoptionPct = Number(adoptionPayload.adoption_pct || trackingPayload.adoption_pct || 0);
const unsupported = Number(adoptionPayload.unsupported_commands || trackingPayload.unsupported_rows || 0);
const outputTokens = Number(
  adoptionPayload.total_output_tokens || trackingPayload.total_output_tokens || 0,
);

const allCommands = (sessions || []).flatMap((row) => row.commands || []);
const commandProfile = classifyCommandProfile(allCommands);
const defaultBudgetMode = deriveDefaultBudgetMode(adoptionPct, unsupported, outputTokens);
const modelBias = deriveModelBias(commandProfile);

const tuning = {
  type: 'nightly_turn_loop_tuning',
  schema_version: 1,
  generated_at: nowIso(),
  source: {
    tracking_ok: tracking.ok,
    adoption_ok: adoption.ok,
  },
  routing: {
    default_budget_mode: defaultBudgetMode,
    model_bias: modelBias,
  },
  suggestions: {
    blocked_stems: ['can you continue', 'can you verify', 'what should we'],
    blocked_phrases: ['does compare other', 'infring some root cause'],
  },
  metrics: {
    adoption_pct: Number.isFinite(adoptionPct) ? adoptionPct : 0,
    unsupported_commands: Number.isFinite(unsupported) ? unsupported : 0,
    total_output_tokens: Number.isFinite(outputTokens) ? outputTokens : 0,
    tracked_rows: Number(trackingPayload.tracked_rows || 0),
    sessions_scanned: Number(adoptionPayload.sessions_scanned || 0),
  },
};

writeJson(TRACKING_TUNING_PATH, tuning);
writeJson(ADOPTION_REPORT_PATH, adoptionPayload || {});

const report = {
  type: 'nightly_turn_loop_analytics_report',
  started_at: startedAt,
  finished_at: nowIso(),
  tracking: {
    ok: tracking.ok,
    status: tracking.status,
    payload: trackingPayload,
    stderr: cleanText(tracking.stderr, 500),
  },
  adoption: {
    ok: adoption.ok,
    status: adoption.status,
    payload: adoptionPayload,
    stderr: cleanText(adoption.stderr, 500),
  },
  tuning_path: TRACKING_TUNING_PATH,
  adoption_report_path: ADOPTION_REPORT_PATH,
  tuning,
};
report['ok'] = !!tracking.ok || !!adoption.ok;

const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `nightly_turn_loop_analytics_report_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'nightly_turn_loop_analytics_report_latest.json');
writeJson(stampedPath, report);
writeJson(latestPath, report);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(report.ok ? 0 : 1);
