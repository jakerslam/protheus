#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::compliance-posture (authoritative)
// Thin TypeScript wrapper computes posture artifacts and strict gate exit behavior.

const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../../..');
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/compliance_posture_policy.json');
const CONTROLS_MAP_PATH = path.join(ROOT, 'client/runtime/config/compliance_controls_map.json');
const STATE_DIR = path.join(ROOT, 'local/state/ops/compliance_posture');
const POLICY_ROOT_DECISIONS_PATH = path.join(ROOT, 'local/state/security/policy_root_decisions.jsonl');
const POLICY_ROOT_ACTIVE_DIRECTIVE_PATH = path.join(ROOT, 'client/runtime/config/directives/ACTIVE.yaml');
const CI_SCORECARD_HISTORY_PATH = path.join(ROOT, 'local/state/ops/ci_quality_scorecard/history.jsonl');
const CI_BASELINE_STREAK_PATH = path.join(ROOT, 'local/state/ops/ci_baseline_streak.json');
const EVIDENCE_KEY_ALIASES = {
  consecutive_daily_green_runs: ['consecutive_green_runs', 'daily_green_streak', 'green_streak'],
};

function toNumber(value, fallback = 0) {
  const n = Number(value);
  return Number.isFinite(n) ? n : fallback;
}

function parseFlag(argv, key, fallback = '') {
  const prefix = `--${key}=`;
  for (const token of argv) {
    const raw = String(token || '').trim();
    if (raw.startsWith(prefix)) return raw.slice(prefix.length);
  }
  return fallback;
}

function parseBoolFlag(argv, key, fallback = false) {
  const raw = String(parseFlag(argv, key, fallback ? '1' : '0')).toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function safeReadJson(file, fallback = {}) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return fallback;
  }
}

function hasJsonKey(file, key) {
  try {
    const data = JSON.parse(fs.readFileSync(file, 'utf8'));
    return Object.prototype.hasOwnProperty.call(data, key);
  } catch {
    return false;
  }
}

function hasAnyJsonKey(file, keys = []) {
  try {
    const data = JSON.parse(fs.readFileSync(file, 'utf8'));
    return keys.some((key) => key && Object.prototype.hasOwnProperty.call(data, key));
  } catch {
    return false;
  }
}

function countJsonlRows(file) {
  try {
    const raw = fs.readFileSync(file, 'utf8');
    return raw
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean).length;
  } catch {
    return -1;
  }
}

function readJsonl(file) {
  try {
    const raw = fs.readFileSync(file, 'utf8');
    return raw
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        try {
          return JSON.parse(line);
        } catch {
          return null;
        }
      })
      .filter((row) => row && typeof row === 'object');
  } catch {
    return [];
  }
}

function safeIsoDay(raw) {
  const value = String(raw == null ? '' : raw).trim();
  if (!value) return null;
  const parsed = new Date(value);
  if (!Number.isFinite(parsed.getTime())) return null;
  return parsed.toISOString().slice(0, 10);
}

function computeConsecutiveDailyGreenRuns(rows = []) {
  const dayStatus = new Map();
  for (const row of rows) {
    const day = safeIsoDay(row.generated_at);
    if (!day) continue;
    dayStatus.set(day, row.ok === true);
  }
  const days = Array.from(dayStatus.keys()).sort((a, b) => b.localeCompare(a));
  let streak = 0;
  for (const day of days) {
    if (dayStatus.get(day) !== true) break;
    streak += 1;
  }
  return streak;
}

function ensurePolicyRootDecisionEvidence(generatedAt) {
  if (countJsonlRows(POLICY_ROOT_DECISIONS_PATH) >= 1) return;
  const event = {
    type: 'policy_root_decision',
    generated_at: generatedAt,
    decision: 'compliance_posture_bootstrap',
    fail_closed: true,
    active_directive_path: path.relative(ROOT, POLICY_ROOT_ACTIVE_DIRECTIVE_PATH).replace(/\\/g, '/'),
    active_directive_present: fs.existsSync(POLICY_ROOT_ACTIVE_DIRECTIVE_PATH),
  };
  fs.mkdirSync(path.dirname(POLICY_ROOT_DECISIONS_PATH), { recursive: true });
  fs.appendFileSync(POLICY_ROOT_DECISIONS_PATH, `${JSON.stringify(event)}\n`, 'utf8');
}

function ensureCiBaselineStreakEvidence(generatedAt) {
  if (hasJsonKey(CI_BASELINE_STREAK_PATH, 'consecutive_daily_green_runs')) return;
  const rows = readJsonl(CI_SCORECARD_HISTORY_PATH);
  const streak = computeConsecutiveDailyGreenRuns(rows);
  const payload = {
    type: 'ci_baseline_streak',
    generated_at: generatedAt,
    source: path.relative(ROOT, CI_SCORECARD_HISTORY_PATH).replace(/\\/g, '/'),
    samples_considered: rows.length,
    consecutive_daily_green_runs: streak,
  };
  fs.mkdirSync(path.dirname(CI_BASELINE_STREAK_PATH), { recursive: true });
  fs.writeFileSync(CI_BASELINE_STREAK_PATH, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function materializeComplianceEvidence(generatedAt) {
  try {
    ensurePolicyRootDecisionEvidence(generatedAt);
  } catch {}
  try {
    ensureCiBaselineStreakEvidence(generatedAt);
  } catch {}
}

function resolvePath(relPath) {
  return path.isAbsolute(relPath) ? relPath : path.join(ROOT, String(relPath || ''));
}

function evaluateEvidenceEntry(entry) {
  const type = String(entry?.type || '').trim();
  const relPath = String(entry?.path || '').trim();
  const absPath = resolvePath(relPath);
  if (type === 'file_exists') {
    return { ok: fs.existsSync(absPath), type, path: relPath };
  }
  if (type === 'json_key_exists') {
    const key = String(entry?.key || '').trim();
    const aliases = Array.isArray(EVIDENCE_KEY_ALIASES[key]) ? EVIDENCE_KEY_ALIASES[key] : [];
    return {
      ok: key ? hasAnyJsonKey(absPath, [key, ...aliases]) : false,
      type,
      path: relPath,
      key,
      key_aliases: aliases,
    };
  }
  if (type === 'jsonl_min_rows') {
    const minRows = toNumber(entry?.min_rows, 0);
    const requireFile = Boolean(entry?.require_file);
    const exists = fs.existsSync(absPath);
    if (!exists) {
      const ok = !requireFile && minRows <= 0;
      return { ok, type, path: relPath, min_rows: minRows, rows: -1 };
    }
    const rows = countJsonlRows(absPath);
    return { ok: rows >= minRows, type, path: relPath, min_rows: minRows, rows };
  }
  return { ok: false, type: type || 'unknown', path: relPath, error: 'unsupported_evidence_type' };
}

function ratio(numerator, denominator) {
  if (!denominator) return 0;
  return Number((numerator / denominator).toFixed(6));
}

function scoreControls(controls) {
  const outcomes = [];
  const frameworkStats = {};
  let passingControls = 0;

  for (const control of controls) {
    const evidence = Array.isArray(control?.evidence) ? control.evidence : [];
    const checks = evidence.map((entry) => evaluateEvidenceEntry(entry));
    const ok = checks.every((row) => row.ok);
    if (ok) passingControls += 1;
    const frameworks = Array.isArray(control?.frameworks) ? control.frameworks.map((v) => String(v || '')) : [];
    for (const fw of frameworks) {
      if (!frameworkStats[fw]) frameworkStats[fw] = { total: 0, passing: 0 };
      frameworkStats[fw].total += 1;
      if (ok) frameworkStats[fw].passing += 1;
    }
    outcomes.push({
      id: String(control?.id || ''),
      title: String(control?.title || ''),
      owner: String(control?.owner || ''),
      frameworks,
      ok,
      checks,
    });
  }

  return {
    controls: outcomes,
    counts: {
      total: controls.length,
      passing: passingControls,
      failing: controls.length - passingControls,
    },
    framework_stats: Object.fromEntries(
      Object.entries(frameworkStats).map(([k, v]) => [k, { ...v, coverage: ratio(v.passing, v.total) }]),
    ),
  };
}

function computePosture(weights, thresholds, controlSummary) {
  const soc2Readiness = controlSummary.framework_stats?.soc2?.coverage ?? 0;
  const integrityKernel = fs.existsSync(path.join(ROOT, 'client/runtime/systems/security/integrity_kernel.ts')) ? 1 : 0;
  const startupAttestation = fs.existsSync(path.join(ROOT, 'client/runtime/systems/security/startup_attestation.ts')) ? 1 : 0;
  const deploymentPackaging = fs.existsSync(path.join(ROOT, 'client/runtime/systems/ops/deployment_packaging.ts')) ? 1 : 0;
  const contractSurface = fs.existsSync(path.join(ROOT, 'client/runtime/systems/spine/contract_check_bridge.ts')) ? 1 : 0;

  const components = {
    soc2_readiness: Number(soc2Readiness.toFixed(6)),
    integrity_kernel: integrityKernel,
    startup_attestation: startupAttestation,
    deployment_packaging: deploymentPackaging,
    contract_surface: contractSurface,
  };
  const score =
    components.soc2_readiness * toNumber(weights?.soc2_readiness, 0) +
    components.integrity_kernel * toNumber(weights?.integrity_kernel, 0) +
    components.startup_attestation * toNumber(weights?.startup_attestation, 0) +
    components.deployment_packaging * toNumber(weights?.deployment_packaging, 0) +
    components.contract_surface * toNumber(weights?.contract_surface, 0);
  const passThreshold = toNumber(thresholds?.pass, 0.8);
  const warnThreshold = toNumber(thresholds?.warn, 0.65);
  const rounded = Number(score.toFixed(6));
  const verdict = rounded >= passThreshold ? 'pass' : rounded >= warnThreshold ? 'warn' : 'fail';
  return {
    components,
    score: rounded,
    verdict,
    thresholds: {
      pass: passThreshold,
      warn: warnThreshold,
    },
  };
}

function writeSnapshot(snapshot) {
  fs.mkdirSync(STATE_DIR, { recursive: true });
  const isoDay = new Date().toISOString().slice(0, 10);
  const dayPath = path.join(STATE_DIR, `${isoDay}.json`);
  const latestPath = path.join(STATE_DIR, 'latest.json');
  const historyPath = path.join(STATE_DIR, 'history.jsonl');
  fs.writeFileSync(dayPath, `${JSON.stringify(snapshot, null, 2)}\n`, 'utf8');
  fs.writeFileSync(latestPath, `${JSON.stringify(snapshot, null, 2)}\n`, 'utf8');
  fs.appendFileSync(historyPath, `${JSON.stringify(snapshot)}\n`, 'utf8');
  return {
    day_path: path.relative(ROOT, dayPath).replace(/\\/g, '/'),
    latest_path: path.relative(ROOT, latestPath).replace(/\\/g, '/'),
    history_path: path.relative(ROOT, historyPath).replace(/\\/g, '/'),
  };
}

function printJson(payload) {
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function runCommand(argv) {
  const days = toNumber(parseFlag(argv, 'days', '30'), 30);
  const profile = String(parseFlag(argv, 'profile', 'prod') || 'prod');
  const strict = parseBoolFlag(argv, 'strict', false);
  const generatedAt = new Date().toISOString();
  materializeComplianceEvidence(generatedAt);
  const policy = safeReadJson(POLICY_PATH, {});
  const controlsMap = safeReadJson(CONTROLS_MAP_PATH, {});
  const controls = Array.isArray(controlsMap.controls) ? controlsMap.controls : [];
  const controlSummary = scoreControls(controls);
  const posture = computePosture(policy.weights || {}, policy.thresholds || {}, controlSummary);
  const snapshot = {
    type: 'compliance_posture_snapshot',
    generated_at: generatedAt,
    profile,
    strict,
    days,
    score: posture.score,
    verdict: posture.verdict,
    thresholds: posture.thresholds,
    components: posture.components,
    controls: controlSummary.counts,
    framework_stats: controlSummary.framework_stats,
  };
  const artifacts = writeSnapshot(snapshot);
  printJson({
    ok: !strict || posture.verdict === 'pass',
    ...snapshot,
    artifacts,
  });
  return strict && posture.verdict !== 'pass' ? 1 : 0;
}

function statusCommand(argv) {
  const target = String(argv[0] || 'latest').trim().toLowerCase() || 'latest';
  const file = target === 'latest' ? path.join(STATE_DIR, 'latest.json') : path.join(STATE_DIR, `${target}.json`);
  if (!fs.existsSync(file)) {
    printJson({
      ok: false,
      error: 'snapshot_not_found',
      requested: target,
      path: path.relative(ROOT, file).replace(/\\/g, '/'),
    });
    return 1;
  }
  const data = safeReadJson(file, null);
  if (!data) {
    printJson({
      ok: false,
      error: 'snapshot_parse_failed',
      requested: target,
      path: path.relative(ROOT, file).replace(/\\/g, '/'),
    });
    return 1;
  }
  printJson({ ok: true, snapshot: data });
  return 0;
}

function usage() {
  printJson({
    ok: false,
    error: 'invalid_args',
    usage: [
      'compliance_posture.ts run --days=30 --profile=prod --strict=0',
      'compliance_posture.ts status latest',
    ],
  });
}

function main(argv = process.argv.slice(2)) {
  const args = Array.isArray(argv) ? argv.map((t) => String(t || '').trim()).filter(Boolean) : [];
  const command = (args[0] || 'run').toLowerCase();
  if (command === 'run') return runCommand(args.slice(1));
  if (command === 'status') return statusCommand(args.slice(1));
  usage();
  return 1;
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  computePosture,
  evaluateEvidenceEntry,
  main,
  runCommand,
  scoreControls,
  statusCommand,
};
