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

  const semver = readJsonMaybe(semverPath) ?? {};
  const commitLint = readJsonMaybe(commitLintPath) ?? {};
  const policy = readJsonMaybe(policyPath) ?? {};
  const canary = readJsonMaybe(canaryPath) ?? {};
  const channel = releaseChannel(semver?.release_channel);

  const changelogExists = fs.existsSync(changelogPath);
  const canaryOk = canary?.ok === true;
  const canaryRequired = channel === 'stable';
  const canaryGateOk = canaryRequired ? canaryOk : true;
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
    gates,
  };

  const outPath = resolveMaybe(root, args.outPath);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (!overall) process.exitCode = 1;
}

main();
