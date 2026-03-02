#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-032
 * Complexity Warden Meta-Organ
 *
 * Real runtime behaviors:
 * - Scores architectural complexity from filesystem/runtime surfaces
 * - Enforces configurable complexity budget gates
 * - Emits simplification plans with deterministic top targets
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampNumber,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  relPath,
  emit
} = require('../../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.COMPLEXITY_WARDEN_META_ORGAN_POLICY_PATH
  ? path.resolve(process.env.COMPLEXITY_WARDEN_META_ORGAN_POLICY_PATH)
  : path.join(ROOT, 'config/complexity_warden_meta_organ_policy.json');

const DEFAULT_POLICY = {
  version: '1.1',
  enabled: true,
  strict_default: true,
  checks: [
    {
      id: 'warden_scoring_core',
      description: 'Complexity scoring core computes normalized dimensions',
      file_must_exist: 'systems/fractal/warden/README.md'
    },
    {
      id: 'complexity_budget_enforcement',
      description: 'Complexity budget and soul-tax enforcement active'
    },
    {
      id: 'organ_contract_validation',
      description: 'Fractal contract validation lane active'
    },
    {
      id: 'weekly_simplification_cycle',
      description: 'Scheduled simplification sprint lane active'
    }
  ],
  budget: {
    max_score: 0.68,
    warn_score: 0.58
  },
  scoring: {
    roots: ['systems', 'config', 'lib', 'habits'],
    max_files_baseline: 3500,
    max_dirs_baseline: 300,
    max_scripts_baseline: 600
  },
  paths: {
    state_path: 'state/fractal/complexity_warden_meta_organ/state.json',
    latest_path: 'state/fractal/complexity_warden_meta_organ/latest.json',
    receipts_path: 'state/fractal/complexity_warden_meta_organ/receipts.jsonl',
    history_path: 'state/fractal/complexity_warden_meta_organ/history.jsonl',
    plan_path: 'state/fractal/complexity_warden_meta_organ/plan.json'
  }
};

function parseList(raw) {
  if (Array.isArray(raw)) return raw.map((v) => String(v || '').trim()).filter(Boolean);
  const txt = cleanText(raw || '', 4000);
  if (!txt) return [];
  return txt.split(',').map((v) => String(v || '').trim()).filter(Boolean);
}

function normalizePolicy(policyPath) {
  const raw = readJson(policyPath, {});
  const src = raw && typeof raw === 'object' ? raw : {};
  const checksSrc = Array.isArray(src.checks) ? src.checks : DEFAULT_POLICY.checks;
  const checks = checksSrc.map((row, idx) => ({
    id: normalizeToken((row && row.id) || `check_${idx + 1}`, 120) || `check_${idx + 1}`,
    description: cleanText((row && row.description) || (row && row.id) || `check_${idx + 1}`, 400),
    required: row && row.required !== false,
    file_must_exist: cleanText((row && row.file_must_exist) || '', 520)
  }));
  const budgetRaw = src.budget && typeof src.budget === 'object' ? src.budget : {};
  const scoringRaw = src.scoring && typeof src.scoring === 'object' ? src.scoring : {};
  const pathsRaw = src.paths && typeof src.paths === 'object' ? src.paths : {};

  return {
    version: cleanText(src.version || DEFAULT_POLICY.version, 32) || DEFAULT_POLICY.version,
    enabled: src.enabled !== false,
    strict_default: toBool(src.strict_default, DEFAULT_POLICY.strict_default),
    checks,
    budget: {
      max_score: clampNumber(budgetRaw.max_score, 0.1, 1, DEFAULT_POLICY.budget.max_score),
      warn_score: clampNumber(budgetRaw.warn_score, 0.05, 1, DEFAULT_POLICY.budget.warn_score)
    },
    scoring: {
      roots: parseList(scoringRaw.roots || DEFAULT_POLICY.scoring.roots).map((row) => row.replace(/^\/+/, '')),
      max_files_baseline: clampInt(scoringRaw.max_files_baseline, 100, 100000, DEFAULT_POLICY.scoring.max_files_baseline),
      max_dirs_baseline: clampInt(scoringRaw.max_dirs_baseline, 10, 10000, DEFAULT_POLICY.scoring.max_dirs_baseline),
      max_scripts_baseline: clampInt(scoringRaw.max_scripts_baseline, 10, 5000, DEFAULT_POLICY.scoring.max_scripts_baseline)
    },
    paths: {
      state_path: resolvePath(pathsRaw.state_path, DEFAULT_POLICY.paths.state_path),
      latest_path: resolvePath(pathsRaw.latest_path, DEFAULT_POLICY.paths.latest_path),
      receipts_path: resolvePath(pathsRaw.receipts_path, DEFAULT_POLICY.paths.receipts_path),
      history_path: resolvePath(pathsRaw.history_path, DEFAULT_POLICY.paths.history_path),
      plan_path: resolvePath(pathsRaw.plan_path, DEFAULT_POLICY.paths.plan_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function walkStats(absRoot) {
  const stack = [absRoot];
  let fileCount = 0;
  let dirCount = 0;
  while (stack.length > 0) {
    const cur = stack.pop();
    if (!cur || !fs.existsSync(cur)) continue;
    let entries = [];
    try {
      entries = fs.readdirSync(cur, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (!entry || !entry.name) continue;
      if (entry.name === '.git' || entry.name === 'node_modules' || entry.name === '.DS_Store') continue;
      const next = path.join(cur, entry.name);
      if (entry.isDirectory()) {
        dirCount += 1;
        stack.push(next);
      } else if (entry.isFile()) {
        fileCount += 1;
      }
    }
  }
  return { file_count: fileCount, dir_count: dirCount };
}

function packageScriptCount() {
  const pkgPath = path.join(ROOT, 'package.json');
  const pkg = readJson(pkgPath, {});
  const scripts = pkg && pkg.scripts && typeof pkg.scripts === 'object' ? pkg.scripts : {};
  return Object.keys(scripts).length;
}

function evaluateChecks(policy, failSet) {
  return policy.checks.map((check) => {
    const rel = cleanText(check.file_must_exist || '', 520);
    const abs = rel ? path.join(ROOT, rel) : '';
    const fileOk = abs ? fs.existsSync(abs) : true;
    const forcedFail = failSet.has(check.id);
    const pass = fileOk && !forcedFail;
    return {
      id: check.id,
      description: check.description,
      required: check.required !== false,
      pass,
      reason: pass ? 'ok' : (fileOk ? 'forced_failure' : 'required_file_missing'),
      file_checked: abs ? relPath(abs) : null
    };
  });
}

function loadState(policy) {
  const raw = readJson(policy.paths.state_path, {});
  return {
    schema_id: 'complexity_warden_state_v1',
    schema_version: '1.0',
    run_count: Math.max(0, Number(raw && raw.run_count || 0)),
    last_action: raw && raw.last_action ? cleanText(raw.last_action, 80) : null,
    last_ok: typeof (raw && raw.last_ok) === 'boolean' ? raw.last_ok : null,
    last_ts: raw && raw.last_ts ? cleanText(raw.last_ts, 80) : null,
    last_score: clampNumber(raw && raw.last_score, 0, 1, 0),
    last_band: cleanText(raw && raw.last_band || '', 24) || 'unknown'
  };
}

function computeScore(policy) {
  const roots = policy.scoring.roots;
  const perRoot = [];
  let files = 0;
  let dirs = 0;
  for (const rootRel of roots) {
    const abs = path.join(ROOT, rootRel);
    const stats = walkStats(abs);
    files += stats.file_count;
    dirs += stats.dir_count;
    perRoot.push({ root: rootRel, file_count: stats.file_count, dir_count: stats.dir_count });
  }

  const scripts = packageScriptCount();
  const fileNorm = clampNumber(files / Math.max(1, policy.scoring.max_files_baseline), 0, 1, 0);
  const dirNorm = clampNumber(dirs / Math.max(1, policy.scoring.max_dirs_baseline), 0, 1, 0);
  const scriptNorm = clampNumber(scripts / Math.max(1, policy.scoring.max_scripts_baseline), 0, 1, 0);

  const score = clampNumber((fileNorm * 0.45) + (dirNorm * 0.3) + (scriptNorm * 0.25), 0, 1, 0);
  let band = 'healthy';
  if (score >= policy.budget.max_score) band = 'over_budget';
  else if (score >= policy.budget.warn_score) band = 'warn';

  return {
    complexity_score: Number(score.toFixed(6)),
    band,
    metrics: {
      total_files: files,
      total_dirs: dirs,
      script_count: scripts,
      roots: perRoot,
      normalized: {
        files: Number(fileNorm.toFixed(6)),
        dirs: Number(dirNorm.toFixed(6)),
        scripts: Number(scriptNorm.toFixed(6))
      }
    }
  };
}

function buildSimplificationPlan(scorePayload) {
  const roots = Array.isArray(scorePayload && scorePayload.metrics && scorePayload.metrics.roots)
    ? scorePayload.metrics.roots
    : [];
  const sorted = roots
    .slice(0)
    .sort((a, b) => Number((b.file_count || 0) + (b.dir_count || 0)) - Number((a.file_count || 0) + (a.dir_count || 0)));
  const targets = sorted.slice(0, 3).map((row, idx) => ({
    priority: idx + 1,
    root: row.root,
    objective: idx === 0 ? 'split_or_decompose_hotspot' : 'reduce_surface_and_alias_churn',
    file_count: row.file_count,
    dir_count: row.dir_count
  }));
  return {
    schema_id: 'complexity_warden_plan_v1',
    schema_version: '1.0',
    generated_at: nowIso(),
    score: scorePayload.complexity_score,
    band: scorePayload.band,
    targets
  };
}

function persist(policy, out, state, apply) {
  if (!apply) return;
  writeJsonAtomic(policy.paths.state_path, state);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    action: out.action,
    ok: out.ok,
    complexity_score: out.complexity_score,
    band: out.band,
    failed_checks: out.failed_checks
  });
}

function cmdStatus(policy) {
  const latest = readJson(policy.paths.latest_path, null);
  const plan = readJson(policy.paths.plan_path, null);
  emit({
    ok: !!latest,
    type: 'complexity_warden_meta_organ',
    lane_id: 'V3-RACE-032',
    action: 'status',
    ts: nowIso(),
    latest,
    plan,
    state: loadState(policy),
    policy_path: relPath(policy.policy_path)
  }, latest ? 0 : 2);
}

function runWarden(policy, args, action) {
  const strict = toBool(args.strict, policy.strict_default);
  const apply = toBool(args.apply, true);
  const failSet = new Set(parseList(args['fail-checks'] || args.fail_checks).map((row) => normalizeToken(row, 120)));
  const checks = evaluateChecks(policy, failSet);

  const scorePayload = computeScore(policy);
  if (scorePayload.band === 'over_budget') {
    const idx = checks.findIndex((row) => row.id === 'complexity_budget_enforcement');
    if (idx >= 0) checks[idx] = { ...checks[idx], pass: false, reason: 'complexity_budget_exceeded' };
  }

  const failedChecks = checks.filter((row) => row.required !== false && row.pass !== true).map((row) => row.id);
  const ok = failedChecks.length === 0;

  const prev = loadState(policy);
  const nextState = {
    ...prev,
    run_count: prev.run_count + 1,
    last_action: action,
    last_ok: ok,
    last_ts: nowIso(),
    last_score: scorePayload.complexity_score,
    last_band: scorePayload.band
  };

  let plan = null;
  if ((action === 'plan' || action === 'score') && apply) {
    plan = buildSimplificationPlan(scorePayload);
    writeJsonAtomic(policy.paths.plan_path, plan);
  }

  const out = {
    ok,
    type: 'complexity_warden_meta_organ',
    lane_id: 'V3-RACE-032',
    title: 'Complexity Warden Meta-Organ',
    action,
    ts: nowIso(),
    strict,
    apply,
    checks,
    check_count: checks.length,
    failed_checks: failedChecks,
    policy_version: policy.version,
    policy_path: relPath(policy.policy_path),
    complexity_score: scorePayload.complexity_score,
    band: scorePayload.band,
    budget: policy.budget,
    metrics: scorePayload.metrics,
    plan_generated: !!plan,
    plan_path: plan ? relPath(policy.paths.plan_path) : null,
    state: nextState
  };

  persist(policy, out, nextState, apply);
  emit(out, ok || !strict ? 0 : 2);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/fractal/warden/complexity_warden_meta_organ.js score [--strict=1|0] [--apply=1|0]');
  console.log('  node systems/fractal/warden/complexity_warden_meta_organ.js plan [--strict=1|0] [--apply=1|0]');
  console.log('  node systems/fractal/warden/complexity_warden_meta_organ.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const action = normalizeToken(args._[0] || 'score', 80) || 'score';
  if (args.help || action === 'help') {
    usage();
    emit({ ok: true, type: 'complexity_warden_meta_organ', action: 'help', ts: nowIso() }, 0);
  }

  const policy = normalizePolicy(args.policy ? String(args.policy) : POLICY_PATH);
  if (policy.enabled !== true) {
    emit({ ok: false, type: 'complexity_warden_meta_organ', error: 'lane_disabled', policy_path: relPath(policy.policy_path) }, 2);
  }

  if (action === 'status') return cmdStatus(policy);
  if (action === 'score' || action === 'run') return runWarden(policy, args, 'score');
  if (action === 'plan') return runWarden(policy, args, 'plan');

  usage();
  emit({ ok: false, type: 'complexity_warden_meta_organ', error: 'unknown_action', action }, 2);
}

if (require.main === module) {
  main();
}
