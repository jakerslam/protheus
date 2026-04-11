#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';

function cleanText(value: unknown, maxLen = 2000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out = {
    outPath: 'client/runtime/local/state/release/scorecard/release_scorecard.json',
    semverPath: '/tmp/release-plan.json',
    commitLintPath: 'core/local/artifacts/conventional_commit_gate_current.json',
    policyPath: 'core/local/artifacts/release_policy_gate_current.json',
    canaryPath: 'core/local/artifacts/release_canary_gate_current.json',
    changelogPath: 'client/runtime/local/state/release/CHANGELOG.auto.md',
    closurePolicyPath: 'client/runtime/config/production_readiness_closure_policy.json',
    supportBundlePath: 'core/local/artifacts/support_bundle_latest.json',
    topologyPath: 'core/local/artifacts/production_topology_diagnostic_current.json',
    stateCompatPath: 'core/local/artifacts/stateful_upgrade_rollback_gate_current.json',
    blockersPath: 'core/local/artifacts/release_blocker_rubric_current.json',
    ipcSoakPath: 'local/state/ops/ops_ipc_bridge_stability_soak/latest.json',
    drPath: 'local/state/ops/dr_gameday/latest.json',
  };
  for (const tokenRaw of argv) {
    const token = cleanText(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--out=')) out.outPath = cleanText(token.slice(6), 400);
    else if (token.startsWith('--semver=')) out.semverPath = cleanText(token.slice(9), 400);
    else if (token.startsWith('--commit-lint=')) out.commitLintPath = cleanText(token.slice(14), 400);
    else if (token.startsWith('--policy=')) out.policyPath = cleanText(token.slice(9), 400);
    else if (token.startsWith('--canary=')) out.canaryPath = cleanText(token.slice(9), 400);
    else if (token.startsWith('--changelog=')) out.changelogPath = cleanText(token.slice(12), 400);
    else if (token.startsWith('--closure-policy=')) out.closurePolicyPath = cleanText(token.slice(17), 400);
    else if (token.startsWith('--support-bundle=')) out.supportBundlePath = cleanText(token.slice(17), 400);
    else if (token.startsWith('--topology=')) out.topologyPath = cleanText(token.slice(11), 400);
    else if (token.startsWith('--state-compat=')) out.stateCompatPath = cleanText(token.slice(15), 400);
    else if (token.startsWith('--blockers=')) out.blockersPath = cleanText(token.slice(11), 400);
    else if (token.startsWith('--ipc-soak=')) out.ipcSoakPath = cleanText(token.slice(11), 400);
    else if (token.startsWith('--dr=')) out.drPath = cleanText(token.slice(5), 400);
  }
  return out;
}

function readJsonMaybe(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function resolveMaybe(root: string, maybePath: string): string {
  if (path.isAbsolute(maybePath)) return maybePath;
  return path.resolve(root, maybePath);
}

function gateRow(id: string, ok: boolean, detail: string) {
  return { id, ok, detail };
}

function safeNumber(value: unknown, fallback = 0): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

function releaseChannel(raw: unknown): 'alpha' | 'beta' | 'stable' {
  const normalized = cleanText(raw ?? '', 40).toLowerCase();
  if (normalized === 'alpha' || normalized === 'beta' || normalized === 'stable') {
    return normalized;
  }
  return 'stable';
}

function main() {
  const root = path.resolve(__dirname, '../../../..');
  const args = parseArgs(process.argv.slice(2));
  const semverPath = resolveMaybe(root, args.semverPath);
  const commitLintPath = resolveMaybe(root, args.commitLintPath);
  const policyPath = resolveMaybe(root, args.policyPath);
  const canaryPath = resolveMaybe(root, args.canaryPath);
  const changelogPath = resolveMaybe(root, args.changelogPath);
  const closurePolicyPath = resolveMaybe(root, args.closurePolicyPath);
  const supportBundlePath = resolveMaybe(root, args.supportBundlePath);
  const topologyPath = resolveMaybe(root, args.topologyPath);
  const stateCompatPath = resolveMaybe(root, args.stateCompatPath);
  const blockersPath = resolveMaybe(root, args.blockersPath);
  const ipcSoakPath = resolveMaybe(root, args.ipcSoakPath);
  const drPath = resolveMaybe(root, args.drPath);

  const semver = readJsonMaybe(semverPath) ?? {};
  const commitLint = readJsonMaybe(commitLintPath) ?? {};
  const policy = readJsonMaybe(policyPath) ?? {};
  const canary = readJsonMaybe(canaryPath) ?? {};
  const closurePolicy = readJsonMaybe(closurePolicyPath) ?? {};
  const supportBundle = readJsonMaybe(supportBundlePath) ?? {};
  const topology = readJsonMaybe(topologyPath) ?? {};
  const stateCompat = readJsonMaybe(stateCompatPath) ?? {};
  const blockers = readJsonMaybe(blockersPath) ?? {};
  const ipcSoak = readJsonMaybe(ipcSoakPath) ?? {};
  const dr = readJsonMaybe(drPath) ?? {};
  const channel = releaseChannel(semver?.release_channel);

  const changelogExists = fs.existsSync(changelogPath);
  const canaryOk = canary?.ok === true;
  const canaryRequired = channel === 'stable';
  const canaryGateOk = canaryRequired ? canaryOk : true;
  const thresholds = closurePolicy?.numeric_thresholds ?? {};
  const ipcRows = Array.isArray(ipcSoak?.rows) ? ipcSoak.rows : [];
  const ipcSuccessRate =
    ipcRows.length === 0 ? 0 : ipcRows.filter((row: any) => row && row.ok === true).length / ipcRows.length;
  const receiptCompleteness = safeNumber(supportBundle?.metrics?.receipt_completeness_rate, 0);
  const maxCommandLatencyMs = safeNumber(
    supportBundle?.metrics?.max_command_latency_ms,
    Number.POSITIVE_INFINITY,
  );
  const observedRtoMinutes = safeNumber(dr?.observed_rto_minutes, Number.POSITIVE_INFINITY);
  const observedRpoHours = safeNumber(dr?.observed_rpo_hours, Number.POSITIVE_INFINITY);
  const gates = [
    gateRow(
      'semver_plan',
      !!semver && semver.ok === true && typeof semver.next_tag === 'string',
      `next_tag=${cleanText(semver?.next_tag ?? 'none', 120)}`
    ),
    gateRow(
      'conventional_commit_lint',
      !!commitLint && (commitLint.ok === true || commitLint.strict === false),
      `invalid_count=${Number(commitLint?.invalid_count ?? 0)}`
    ),
    gateRow(
      'release_policy_gate',
      !!policy && policy.ok === true,
      `failed=${Array.isArray(policy?.failed) ? policy.failed.join(',') : 'none'}`
    ),
    gateRow(
      'canary_rollback_gate',
      canaryGateOk,
      canaryRequired
        ? `required=true;canary_ok=${canaryOk}`
        : `required=false;canary_ok=${canaryOk}`
    ),
    gateRow(
      'production_topology_diagnostic',
      topology?.ok === true && topology?.supported_production_topology === true,
      `support_level=${cleanText(topology?.support_level ?? 'unknown', 80)}`
    ),
    gateRow(
      'stateful_upgrade_rollback_gate',
      stateCompat?.ok === true,
      `errors=${Array.isArray(stateCompat?.errors) ? stateCompat.errors.length : 0}`
    ),
    gateRow(
      'release_blocker_rubric_gate',
      blockers?.ok === true,
      `open_release_blockers=${Array.isArray(blockers?.open_release_blockers) ? blockers.open_release_blockers.length : 0}`
    ),
    gateRow(
      'ipc_success_rate_threshold',
      ipcSuccessRate >= safeNumber(thresholds.ipc_success_rate_min, 0.95),
      `value=${ipcSuccessRate.toFixed(4)};min=${safeNumber(thresholds.ipc_success_rate_min, 0.95)}`
    ),
    gateRow(
      'receipt_completeness_threshold',
      receiptCompleteness >= safeNumber(thresholds.receipt_completeness_rate_min, 1),
      `value=${receiptCompleteness.toFixed(4)};min=${safeNumber(thresholds.receipt_completeness_rate_min, 1)}`
    ),
    gateRow(
      'supported_command_latency_threshold',
      maxCommandLatencyMs <= safeNumber(thresholds.supported_command_latency_ms_max, 2500),
      `value=${maxCommandLatencyMs};max=${safeNumber(thresholds.supported_command_latency_ms_max, 2500)}`
    ),
    gateRow(
      'recovery_rto_threshold',
      observedRtoMinutes <= safeNumber(thresholds.recovery_rto_minutes_max, 30),
      `value=${observedRtoMinutes};max=${safeNumber(thresholds.recovery_rto_minutes_max, 30)}`
    ),
    gateRow(
      'recovery_rpo_threshold',
      observedRpoHours <= safeNumber(thresholds.recovery_rpo_hours_max, 24),
      `value=${observedRpoHours};max=${safeNumber(thresholds.recovery_rpo_hours_max, 24)}`
    ),
    gateRow('changelog_generated', changelogExists, `path=${path.relative(root, changelogPath)}`),
  ];
  const overall = gates.every((row) => row.ok);
  const report = {
    ok: overall,
    type: 'release_scorecard',
    generated_at: new Date().toISOString(),
    channel,
    tag: cleanText(semver?.next_tag ?? 'none', 120),
    version: cleanText(semver?.next_version ?? semver?.current_version ?? '0.0.0', 120),
    thresholds: {
      ipc_success_rate: Number(ipcSuccessRate.toFixed(4)),
      receipt_completeness_rate: Number(receiptCompleteness.toFixed(4)),
      max_command_latency_ms: maxCommandLatencyMs,
      observed_rto_minutes: observedRtoMinutes,
      observed_rpo_hours: observedRpoHours,
    },
    gates,
  };

  const outPath = resolveMaybe(root, args.outPath);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (!overall) process.exitCode = 1;
}

main();
