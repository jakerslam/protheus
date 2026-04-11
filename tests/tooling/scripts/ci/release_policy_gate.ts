#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';

type GateCheck = {
  id: string;
  ok: boolean;
  detail: string;
};

function cleanText(value: unknown, maxLen = 5000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseBool(value: string | undefined, fallback = false): boolean {
  const raw = cleanText(value ?? '', 32).toLowerCase();
  if (!raw) return fallback;
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function parseArgs(argv: string[]) {
  const out = {
    strict: false,
    outPath: 'core/local/artifacts/release_policy_gate_current.json',
  };
  for (const tokenRaw of argv) {
    const token = cleanText(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--strict=')) out.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--out=')) out.outPath = cleanText(token.slice(6), 400);
  }
  return out;
}

function readJson(filePath: string, fallback: any = null): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function checkFileExists(root: string, relPath: string, id: string): GateCheck {
  const ok = fs.existsSync(path.resolve(root, relPath));
  return {
    id,
    ok,
    detail: ok ? `found:${relPath}` : `missing:${relPath}`,
  };
}

function checkReleaseChannelPolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/release_channel_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const allowed = Array.isArray(policy.channels) ? policy.channels : [];
  const rules = Array.isArray(policy.promotion_rules) ? policy.promotion_rules : [];
  const ok =
    allowed.includes('alpha') &&
    allowed.includes('beta') &&
    allowed.includes('stable') &&
    rules.length > 0;
  return {
    id: 'release_channel_policy',
    ok,
    detail: ok
      ? `channels=${allowed.join(',')};rules=${rules.length}`
      : `invalid policy at ${rel}`,
  };
}

function checkDeprecationPolicy(root: string): GateCheck {
  const registryRel = 'client/runtime/config/api_cli_contract_registry.json';
  const policyRel = 'client/runtime/config/release_compatibility_policy.json';
  const registry = readJson(path.resolve(root, registryRel), {});
  const policy = readJson(path.resolve(root, policyRel), {});
  const minDays = Number(policy.required_deprecation_days ?? 90);
  const requireGuide = policy.require_migration_guide !== false;
  const contracts = ([] as any[])
    .concat(Array.isArray(registry.api_contracts) ? registry.api_contracts : [])
    .concat(Array.isArray(registry.cli_contracts) ? registry.cli_contracts : []);
  const violations: string[] = [];
  for (const row of contracts) {
    const name = cleanText(row?.name ?? 'unknown', 120);
    const days = Number(row?.deprecation_window_days ?? 0);
    if (!Number.isFinite(days) || days < minDays) {
      violations.push(`${name}:deprecation_window_days_lt_${minDays}`);
    }
    const status = cleanText(row?.status ?? '', 40).toLowerCase();
    if (status === 'deprecated' && requireGuide) {
      const guide = cleanText(row?.migration_guide ?? '', 240);
      const notice = cleanText(row?.deprecation_notice ?? '', 240);
      if (!guide || !notice) violations.push(`${name}:deprecated_missing_migration_guide_or_notice`);
    }
  }
  return {
    id: 'compatibility_deprecation_policy',
    ok: violations.length === 0,
    detail: violations.length ? violations.join('; ') : 'ok',
  };
}

function checkSchemaMigrationPolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/schema_versioning_gate_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const targets = Array.isArray(policy.targets) ? policy.targets : [];
  const missing: string[] = [];
  for (const row of targets) {
    const targetPath = cleanText(row?.path ?? '', 400);
    if (!targetPath) continue;
    if (!fs.existsSync(path.resolve(root, targetPath))) missing.push(targetPath);
  }
  const migrations = policy?.migrations ?? {};
  const hasMigrationGuard =
    typeof migrations === 'object' &&
    cleanText(migrations.target_default_version ?? '', 40).length > 0 &&
    typeof migrations.allow_add_missing_fields_only === 'boolean';
  const ok = missing.length === 0 && hasMigrationGuard;
  return {
    id: 'config_migration_gate',
    ok,
    detail: ok
      ? `targets=${targets.length};migration_guard=ok`
      : `missing_targets=${missing.join(',') || 'none'};migration_guard=${hasMigrationGuard}`,
  };
}

function checkDependencyPolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/dependency_update_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const blocked = Array.isArray(policy.blocked_packages) ? policy.blocked_packages : [];
  const slaDays = Number(policy.security_patch_sla_days ?? 0);
  const hasSla = Number.isFinite(slaDays) && slaDays > 0 && slaDays <= 30;
  const hasBlocked = blocked.length > 0;
  return {
    id: 'dependency_update_policy',
    ok: hasSla && hasBlocked,
    detail: hasSla && hasBlocked ? `sla_days=${slaDays};blocked=${blocked.length}` : `invalid:${rel}`,
  };
}

function checkDependabotSchedule(root: string): GateCheck {
  const rel = '.github/dependabot.yml';
  try {
    const raw = fs.readFileSync(path.resolve(root, rel), 'utf8');
    const ecosystems = new Set<string>();
    const re = /package-ecosystem:\s*["']?([a-zA-Z0-9_-]+)["']?/g;
    let match: RegExpExecArray | null = null;
    while ((match = re.exec(raw)) != null) {
      const ecosystem = cleanText(match[1] ?? '', 60);
      if (ecosystem) ecosystems.add(ecosystem);
    }
    const ok = ecosystems.has('npm') && ecosystems.has('cargo') && ecosystems.has('github-actions');
    return {
      id: 'dependabot_schedule',
      ok,
      detail: ok ? `ecosystems=${Array.from(ecosystems).sort().join(',')}` : `missing required ecosystems in ${rel}`,
    };
  } catch {
    return { id: 'dependabot_schedule', ok: false, detail: `invalid:${rel}` };
  }
}

function checkInstallerChecksumVerification(root: string): GateCheck {
  const installPath = path.resolve(root, 'install.sh');
  const script = fs.readFileSync(installPath, 'utf8');
  const strictDefault = /INSTALL_ALLOW_UNVERIFIED_ASSETS="\$\{INFRING_INSTALL_ALLOW_UNVERIFIED_ASSETS:-\$\{PROTHEUS_INSTALL_ALLOW_UNVERIFIED_ASSETS:-0\}\}"/.test(
    script
  );
  const hasVerifyFn = /verify_downloaded_asset\(\)/.test(script);
  const usedOnDownload = /verify_downloaded_asset "\$version_tag" "\$asset_name" "\$asset_out"/.test(script);
  const ok = strictDefault && hasVerifyFn && usedOnDownload;
  return {
    id: 'installer_checksum_verification',
    ok,
    detail: ok
      ? 'checksum verification enabled by default'
      : `strict_default=${strictDefault};verify_fn=${hasVerifyFn};download_hook=${usedOnDownload}`,
  };
}

function checkProductionTransportPolicy(root: string): GateCheck {
  const sdkPath = path.resolve(root, 'packages/infring-sdk/src/transports.ts');
  const bridgePath = path.resolve(root, 'adapters/runtime/ops_lane_bridge.ts');
  const runnerPath = path.resolve(root, 'adapters/runtime/run_protheus_ops.ts');
  const legacyHelperPath = path.resolve(root, 'adapters/runtime/dev_only/legacy_process_runner.ts');
  const processFallbackHelperPath = path.resolve(root, 'adapters/runtime/dev_only/ops_lane_process_fallback.ts');
  const sdkSource = fs.readFileSync(sdkPath, 'utf8');
  const bridgeSource = fs.readFileSync(bridgePath, 'utf8');
  const runnerSource = fs.readFileSync(runnerPath, 'utf8');
  const legacyHelperSource = fs.existsSync(legacyHelperPath) ? fs.readFileSync(legacyHelperPath, 'utf8') : '';
  const processFallbackHelperSource = fs.existsSync(processFallbackHelperPath)
    ? fs.readFileSync(processFallbackHelperPath, 'utf8')
    : '';

  const sdkLock =
    sdkSource.includes('process_transport_forbidden_in_production') &&
    sdkSource.includes('isProductionReleaseChannel') &&
    sdkSource.includes('INFRING_RELEASE_CHANNEL');
  const bridgeLock =
    bridgeSource.includes('process_fallback_forbidden_in_production') &&
    bridgeSource.includes('processFallbackPolicy') &&
    bridgeSource.includes('isProductionReleaseChannel') &&
    !bridgeSource.includes('spawnSync(') &&
    bridgeSource.includes("./dev_only/ops_lane_process_fallback.ts");
  const legacyLock =
    runnerSource.includes('legacyProcessRunnerForced') &&
    runnerSource.includes('isProductionReleaseChannel') &&
    runnerSource.includes('INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER') &&
    runnerSource.includes('legacy_process_runner_dev_only') &&
    runnerSource.includes('INFRING_DEV_ENABLE_LEGACY_PROCESS_RUNNER') &&
    !runnerSource.includes('spawnSync(') &&
    runnerSource.includes("./dev_only/legacy_process_runner.ts");
  const helperLock =
    legacyHelperSource.includes('legacy_process_runner_dev_only') &&
    legacyHelperSource.includes('spawnSync(') &&
    processFallbackHelperSource.includes('process_fallback_dev_only') &&
    processFallbackHelperSource.includes('spawnSync(');

  const ok = sdkLock && bridgeLock && legacyLock && helperLock;
  return {
    id: 'production_transport_policy',
    ok,
    detail: ok
      ? 'production release channel enforces resident IPC topology'
      : `sdk_lock=${sdkLock};bridge_lock=${bridgeLock};legacy_lock=${legacyLock};helper_lock=${helperLock}`,
  };
}

function checkAssimilationV1SupportContract(root: string): GateCheck {
  const rel = 'client/runtime/config/assimilation_v1_support_contract.json';
  const policy = readJson(path.resolve(root, rel), {});
  const canonicalSlice = cleanText(policy?.canonical_slice?.name ?? '', 120);
  const supportLevel = cleanText(policy?.production_contract?.support_level ?? '', 120);
  const ok =
    cleanText(policy?.status ?? '', 80) === 'frozen_v1_vertical_slice' &&
    canonicalSlice.length > 0 &&
    supportLevel === 'experimental_opt_in';
  return {
    id: 'assimilation_v1_support_contract',
    ok,
    detail: ok ? `slice=${canonicalSlice};support_level=${supportLevel}` : `invalid:${rel}`,
  };
}

function checkProductionClosurePolicy(root: string): GateCheck {
  const rel = 'client/runtime/config/production_readiness_closure_policy.json';
  const policy = readJson(path.resolve(root, rel), {});
  const canonical = cleanText(policy?.production_surface_contract?.canonical_surface ?? '', 40).toLowerCase();
  const transportMode = cleanText(policy?.release_candidate_topology?.transport_mode ?? '', 120);
  const forbidFallback = policy?.release_candidate_topology?.forbid_process_transport_in_production === true;
  const ok = canonical === 'rich' && transportMode === 'resident_ipc_authoritative' && forbidFallback;
  return {
    id: 'production_closure_policy',
    ok,
    detail: ok
      ? `canonical_surface=${canonical};transport_mode=${transportMode}`
      : `invalid:${rel}`,
  };
}

function buildReport(root: string) {
  const checks: GateCheck[] = [
    checkFileExists(root, 'client/runtime/config/release_channel_policy.json', 'release_channel_policy_file'),
    checkReleaseChannelPolicy(root),
    checkFileExists(root, 'client/runtime/config/release_compatibility_policy.json', 'release_compatibility_policy_file'),
    checkDeprecationPolicy(root),
    checkSchemaMigrationPolicy(root),
    checkFileExists(root, 'client/runtime/config/dependency_update_policy.json', 'dependency_update_policy_file'),
    checkDependencyPolicy(root),
    checkDependabotSchedule(root),
    checkInstallerChecksumVerification(root),
    checkProductionTransportPolicy(root),
    checkProductionClosurePolicy(root),
    checkAssimilationV1SupportContract(root),
  ];
  const ok = checks.every((row) => row.ok);
  return {
    ok,
    type: 'release_policy_gate',
    checks,
    failed: checks.filter((row) => !row.ok).map((row) => row.id),
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = path.resolve(__dirname, '../../../..');
  const args = parseArgs(argv);
  const report = {
    ...buildReport(root),
    strict: args.strict,
  };
  const outPath = path.resolve(root, args.outPath);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (args.strict && !report.ok) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  parseArgs,
  run,
};
