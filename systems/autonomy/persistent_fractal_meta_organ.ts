#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-019
 *
 * Persistent Fractal Engine Meta-Organ:
 * - gated source triggers (nightly + high_success_receipt)
 * - bounded mutation proposals (habit code, memory schema, routing policy)
 * - isolated shadow-cell trials via existing self-improvement gates
 * - governed promotion lane with deterministic reversion path
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.PERSISTENT_FRACTAL_META_ORGAN_POLICY_PATH
  ? path.resolve(process.env.PERSISTENT_FRACTAL_META_ORGAN_POLICY_PATH)
  : path.join(ROOT, 'config', 'persistent_fractal_meta_organ_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/autonomy/persistent_fractal_meta_organ.js run [--source=nightly|high_success_receipt] [--apply=1|0] [--policy=<path>]');
  console.log('  node systems/autonomy/persistent_fractal_meta_organ.js promote --proposal-id=<id> [--approval-a=<id>] [--approval-b=<id>] [--policy=<path>]');
  console.log('  node systems/autonomy/persistent_fractal_meta_organ.js status [--policy=<path>]');
}

function parseJsonFromStdout(stdout: string) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try { return JSON.parse(text); } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function normalizeList(v: unknown) {
  if (Array.isArray(v)) return v.map((row) => cleanText(row, 320)).filter(Boolean);
  const raw = cleanText(v || '', 4000);
  if (!raw) return [];
  return raw.split(',').map((row) => cleanText(row, 320)).filter(Boolean);
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    shadow_only: true,
    allow_live_apply: false,
    objective_id: 'persistent_fractal_meta_organ',
    trigger: {
      allowed_sources: ['nightly', 'high_success_receipt'],
      cooldown_minutes: 120
    },
    mutation_domains: [
      {
        id: 'habit_code',
        target_path: 'systems/adaptive/habits/habit_runtime_sync.ts',
        risk: 'medium',
        summary: 'fractal_meta_habit_code'
      },
      {
        id: 'memory_schema',
        target_path: 'systems/ops/schema_evolution_contract.ts',
        risk: 'medium',
        summary: 'fractal_meta_memory_schema'
      },
      {
        id: 'routing_policy',
        target_path: 'systems/routing/model_router.ts',
        risk: 'medium',
        summary: 'fractal_meta_routing_policy'
      }
    ],
    scripts: {
      loop_script: 'systems/autonomy/gated_self_improvement_loop.js'
    },
    paths: {
      state_path: 'state/autonomy/persistent_fractal_meta_organ/state.json',
      latest_path: 'state/autonomy/persistent_fractal_meta_organ/latest.json',
      receipts_path: 'state/autonomy/persistent_fractal_meta_organ/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const trigger = raw.trigger && typeof raw.trigger === 'object' ? raw.trigger : {};
  const scripts = raw.scripts && typeof raw.scripts === 'object' ? raw.scripts : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const domainsRaw = Array.isArray(raw.mutation_domains) ? raw.mutation_domains : base.mutation_domains;
  const mutationDomains = domainsRaw
    .map((row: any) => ({
      id: normalizeToken(row && row.id || '', 64),
      target_path: cleanText(row && row.target_path || '', 320),
      risk: normalizeToken(row && row.risk || 'medium', 24) || 'medium',
      summary: cleanText(row && row.summary || '', 220)
    }))
    .filter((row: any) => row.id && row.target_path);

  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: toBool(raw.enabled, true),
    shadow_only: toBool(raw.shadow_only, true),
    allow_live_apply: toBool(raw.allow_live_apply, base.allow_live_apply),
    objective_id: normalizeToken(raw.objective_id || base.objective_id, 120) || base.objective_id,
    trigger: {
      allowed_sources: normalizeList(trigger.allowed_sources || base.trigger.allowed_sources)
        .map((row) => normalizeToken(row, 64))
        .filter(Boolean),
      cooldown_minutes: clampInt(trigger.cooldown_minutes, 0, 7 * 24 * 60, base.trigger.cooldown_minutes)
    },
    mutation_domains: mutationDomains,
    scripts: {
      loop_script: resolvePath(scripts.loop_script || base.scripts.loop_script, base.scripts.loop_script)
    },
    paths: {
      state_path: resolvePath(paths.state_path || base.paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path || base.paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path || base.paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(policy: any) {
  const src = readJson(policy.paths.state_path, null);
  if (!src || typeof src !== 'object') {
    return {
      schema_id: 'persistent_fractal_meta_organ_state',
      schema_version: '1.0',
      updated_at: nowIso(),
      last_run_at: null,
      last_source: null,
      runs: 0,
      proposals_created: 0,
      shadow_trials: 0,
      promotions: 0
    };
  }
  return {
    schema_id: 'persistent_fractal_meta_organ_state',
    schema_version: '1.0',
    updated_at: src.updated_at || nowIso(),
    last_run_at: src.last_run_at || null,
    last_source: src.last_source || null,
    runs: Math.max(0, Number(src.runs || 0)),
    proposals_created: Math.max(0, Number(src.proposals_created || 0)),
    shadow_trials: Math.max(0, Number(src.shadow_trials || 0)),
    promotions: Math.max(0, Number(src.promotions || 0))
  };
}

function saveState(policy: any, state: any) {
  writeJsonAtomic(policy.paths.state_path, {
    schema_id: 'persistent_fractal_meta_organ_state',
    schema_version: '1.0',
    updated_at: nowIso(),
    last_run_at: state.last_run_at || null,
    last_source: state.last_source || null,
    runs: Math.max(0, Number(state.runs || 0)),
    proposals_created: Math.max(0, Number(state.proposals_created || 0)),
    shadow_trials: Math.max(0, Number(state.shadow_trials || 0)),
    promotions: Math.max(0, Number(state.promotions || 0))
  });
}

function runNodeJson(scriptPath: string, args: string[], timeoutMs = 30000) {
  if (!fs.existsSync(scriptPath)) {
    return { ok: false, code: 127, payload: null, stderr: 'loop_script_missing', stdout: '' };
  }
  const run = spawnSync('node', [scriptPath, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: Math.max(1000, timeoutMs)
  });
  return {
    ok: Number(run.status || 0) === 0,
    code: Number.isFinite(run.status) ? Number(run.status) : 1,
    payload: parseJsonFromStdout(run.stdout),
    stderr: cleanText(run.stderr || '', 800),
    stdout: cleanText(run.stdout || '', 800)
  };
}

function sourceGate(policy: any, state: any, source: string) {
  const allowed = Array.isArray(policy.trigger.allowed_sources)
    && policy.trigger.allowed_sources.includes(source);
  const cooldownMin = Number(policy.trigger.cooldown_minutes || 0);
  let cooldownOk = true;
  if (cooldownMin > 0 && state.last_run_at) {
    const prev = Date.parse(String(state.last_run_at || ''));
    if (Number.isFinite(prev)) {
      const elapsed = (Date.now() - prev) / 60000;
      cooldownOk = elapsed >= cooldownMin;
    }
  }
  return {
    pass: allowed && cooldownOk,
    checks: {
      source_allowed: allowed,
      cooldown_ok: cooldownOk
    },
    cooldown_minutes: cooldownMin
  };
}

function cmdRun(args: any, policy: any) {
  const source = normalizeToken(args.source || 'nightly', 64) || 'nightly';
  const state = loadState(policy);
  const gate = sourceGate(policy, state, source);
  const applyRequested = toBool(args.apply, false);
  const applyAllowed = applyRequested && policy.allow_live_apply === true;

  if (!gate.pass) {
    const out = {
      ok: true,
      type: 'persistent_fractal_meta_organ_run',
      ts: nowIso(),
      source,
      triggered: false,
      gate,
      apply_requested: applyRequested,
      apply_allowed: applyAllowed,
      candidates: []
    };
    writeJsonAtomic(policy.paths.latest_path, out);
    appendJsonl(policy.paths.receipts_path, out);
    emit(out);
  }

  const candidates = [];
  let proposalsCreated = 0;
  let shadowTrials = 0;
  let promotions = 0;

  for (const domain of policy.mutation_domains) {
    const summary = cleanText(domain.summary || `meta_${domain.id}`, 180) || `meta_${domain.id}`;
    const propose = runNodeJson(policy.scripts.loop_script, [
      'propose',
      `--objective-id=${policy.objective_id}`,
      `--target-path=${domain.target_path}`,
      `--summary=${summary}`,
      `--risk=${domain.risk || 'medium'}`
    ], 30000);

    const proposalId = propose.payload && propose.payload.proposal_id
      ? String(propose.payload.proposal_id)
      : null;
    if (proposalId) proposalsCreated += 1;

    const shadowRun = proposalId
      ? runNodeJson(policy.scripts.loop_script, [
        'run',
        `--proposal-id=${proposalId}`,
        '--apply=0'
      ], 45000)
      : { ok: false, code: 1, payload: null, stderr: 'proposal_id_missing', stdout: '' };

    if (shadowRun.ok && shadowRun.payload && shadowRun.payload.ok === true) {
      shadowTrials += 1;
    }

    const promotable = !!(proposalId && shadowRun.ok && shadowRun.payload && shadowRun.payload.ok === true);
    let promotionResult = null;
    if (promotable && applyAllowed) {
      const applyArgs = [
        'run',
        `--proposal-id=${proposalId}`,
        '--apply=1'
      ];
      if (args['approval-a']) applyArgs.push(`--approval-a=${String(args['approval-a'])}`);
      if (args['approval-b']) applyArgs.push(`--approval-b=${String(args['approval-b'])}`);
      promotionResult = runNodeJson(policy.scripts.loop_script, applyArgs, 60000);
      if (promotionResult.ok && promotionResult.payload && promotionResult.payload.ok === true) promotions += 1;
    }

    candidates.push({
      domain_id: domain.id,
      target_path: domain.target_path,
      proposal_ok: propose.ok && !!(propose.payload && propose.payload.ok === true),
      proposal_id: proposalId,
      shadow_trial_ok: shadowRun.ok && !!(shadowRun.payload && shadowRun.payload.ok === true),
      shadow_stage: shadowRun.payload && shadowRun.payload.stage ? shadowRun.payload.stage : null,
      promotable,
      promotion_attempted: !!promotionResult,
      promotion_ok: !!(promotionResult && promotionResult.ok && promotionResult.payload && promotionResult.payload.ok === true),
      deterministic_reversion: proposalId
        ? `node systems/autonomy/gated_self_improvement_loop.js rollback --proposal-id=${proposalId} --reason=meta_organ_reversion`
        : null
    });
  }

  state.last_run_at = nowIso();
  state.last_source = source;
  state.runs += 1;
  state.proposals_created += proposalsCreated;
  state.shadow_trials += shadowTrials;
  state.promotions += promotions;
  saveState(policy, state);

  const out = {
    ok: true,
    type: 'persistent_fractal_meta_organ_run',
    ts: nowIso(),
    source,
    triggered: true,
    gate,
    apply_requested: applyRequested,
    apply_allowed: applyAllowed,
    proposals_created: proposalsCreated,
    shadow_trials_executed: shadowTrials,
    promotions_attempted: applyAllowed ? candidates.filter((row: any) => row.promotable).length : 0,
    promotions_succeeded: promotions,
    candidates
  };
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out);
}

function cmdPromote(args: any, policy: any) {
  if (!policy.allow_live_apply) {
    emit({
      ok: false,
      type: 'persistent_fractal_meta_organ_promote',
      error: 'live_apply_disabled_in_policy'
    }, 1);
  }
  const proposalId = cleanText(args['proposal-id'] || '', 120);
  if (!proposalId) emit({ ok: false, type: 'persistent_fractal_meta_organ_promote', error: 'proposal_id_required' }, 1);

  const applyArgs = [
    'run',
    `--proposal-id=${proposalId}`,
    '--apply=1'
  ];
  if (args['approval-a']) applyArgs.push(`--approval-a=${String(args['approval-a'])}`);
  if (args['approval-b']) applyArgs.push(`--approval-b=${String(args['approval-b'])}`);
  const run = runNodeJson(policy.scripts.loop_script, applyArgs, 60000);
  const ok = run.ok && !!(run.payload && run.payload.ok === true);
  const out = {
    ok,
    type: 'persistent_fractal_meta_organ_promote',
    ts: nowIso(),
    proposal_id: proposalId,
    promotion_result: run.payload || null,
    deterministic_reversion: `node systems/autonomy/gated_self_improvement_loop.js rollback --proposal-id=${proposalId} --reason=meta_organ_manual_reversion`
  };
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  if (!ok) emit(out, 1);
  emit(out);
}

function cmdStatus(policy: any) {
  const state = loadState(policy);
  emit({
    ok: true,
    type: 'persistent_fractal_meta_organ_status',
    ts: nowIso(),
    policy: {
      version: policy.version,
      shadow_only: policy.shadow_only,
      allow_live_apply: policy.allow_live_apply,
      objective_id: policy.objective_id,
      allowed_sources: policy.trigger.allowed_sources,
      cooldown_minutes: policy.trigger.cooldown_minutes
    },
    mutation_domains: policy.mutation_domains,
    state,
    latest: readJson(policy.paths.latest_path, null),
    paths: {
      state_path: path.relative(ROOT, policy.paths.state_path).replace(/\\/g, '/'),
      latest_path: path.relative(ROOT, policy.paths.latest_path).replace(/\\/g, '/'),
      receipts_path: path.relative(ROOT, policy.paths.receipts_path).replace(/\\/g, '/')
    }
  });
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === 'help' || cmd === '-h') {
    usage();
    return;
  }
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : POLICY_PATH);
  if (!policy.enabled) emit({ ok: false, error: 'persistent_fractal_meta_organ_disabled' }, 1);

  if (cmd === 'run') return cmdRun(args, policy);
  if (cmd === 'promote') return cmdPromote(args, policy);
  if (cmd === 'status') return cmdStatus(policy);
  usage();
  process.exit(1);
}

main();
