#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision, trackedFiles } from '../../lib/git.ts';
import { DEFAULT_GATE_REGISTRY_PATH, DEFAULT_VERIFY_PROFILES_PATH, collectRegistrySummary, executeGate, executeProfile } from '../../lib/runner.ts';
import { emitStructuredResult } from '../../lib/result.ts';
import { runCommand } from '../../lib/process.ts';
import { loadGateRegistry, loadVerifyProfiles } from '../../lib/manifest.ts';

const ROOT = process.cwd();
const DEFAULT_REPO_HEALTH_ARTIFACT_PATH = 'core/local/artifacts/repo_health_gate_current.json';
const DEFAULT_PACKAGE_JSON_PATH = 'package.json';
const DEFAULT_COMMAND_REGISTRY_POLICY_PATH = 'client/runtime/config/command_registry_policy.json';
const DEFAULT_ROOT_SURFACE_CONTRACT_PATH = 'client/runtime/config/root_surface_contract.json';
const DEFAULT_FILE_SIZE_POLICY_PATH = 'docs/workspace/repo_file_size_policy.json';

type Mode = 'list' | 'gate' | 'profile' | 'health';

function readJson(relativePath: string): Record<string, any> {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, relativePath), 'utf8'));
}

function isGitIgnored(entry: string): boolean {
  const probe = runCommand(['git', 'check-ignore', '-q', '--', entry], {
    cwd: ROOT,
    timeoutSec: 10,
  });
  return probe.status === 0;
}

function collectToolingAdoption(
  ciScriptRoot: string,
  sharedImportMarkers: string[],
) {
  const files = trackedFiles(ROOT)
    .filter((file) => file.startsWith(`${ciScriptRoot}/`))
    .filter((file) => file.endsWith('.ts'));
  const sharedToolingFiles: string[] = [];
  const standaloneParseArgsFiles: string[] = [];
  for (const file of files) {
    const text = fs.readFileSync(path.resolve(ROOT, file), 'utf8');
    if (sharedImportMarkers.some((marker) => marker && text.includes(marker))) {
      sharedToolingFiles.push(file);
    }
    if (/\bfunction\s+parseArgs\s*\(/.test(text)) {
      standaloneParseArgsFiles.push(file);
    }
  }
  const adoptionRatio = files.length > 0 ? sharedToolingFiles.length / files.length : 1;
  return {
    ci_gate_files: files.length,
    shared_tooling_files: sharedToolingFiles.length,
    shared_tooling_adoption_ratio: Number(adoptionRatio.toFixed(4)),
    standalone_parse_args_files: standaloneParseArgsFiles.length,
    shared_tooling_examples: sharedToolingFiles.slice(0, 12),
    standalone_parse_args_examples: standaloneParseArgsFiles.slice(0, 12),
  };
}

function collectRootSurfaceDrift(rootContractPath: string) {
  const contract = readJson(rootContractPath);
  const allowedFiles = new Set<string>(contract.allowed_root_files || []);
  const allowedDirs = new Set<string>(contract.allowed_root_dirs || []);
  const deprecated = new Set<string>(contract.deprecated_root_entries || []);
  const hardViolations: Array<{ entry: string; kind: 'file' | 'dir'; reason: string }> = [];
  const deprecatedPresent: string[] = [];

  for (const entry of fs.readdirSync(ROOT).sort()) {
    if (entry === '.git') continue;
    if (isGitIgnored(entry)) continue;
    const abs = path.join(ROOT, entry);
    const isDir = fs.lstatSync(abs).isDirectory();
    if (isDir) {
      if (allowedDirs.has(entry)) continue;
      if (deprecated.has(entry)) {
        deprecatedPresent.push(entry);
        continue;
      }
      hardViolations.push({ entry, kind: 'dir', reason: 'root_dir_not_allowlisted' });
      continue;
    }
    if (allowedFiles.has(entry)) continue;
    if (deprecated.has(entry)) {
      deprecatedPresent.push(entry);
      continue;
    }
    hardViolations.push({ entry, kind: 'file', reason: 'root_file_not_allowlisted' });
  }

  return {
    hard_violations: hardViolations,
    deprecated_present: deprecatedPresent,
  };
}

function buildRepoHealthReport() {
  const started = Date.now();
  const packageJson = readJson(DEFAULT_PACKAGE_JSON_PATH);
  const commandRegistryPolicy = readJson(DEFAULT_COMMAND_REGISTRY_POLICY_PATH);
  const rootSurfaceContract = readJson(DEFAULT_ROOT_SURFACE_CONTRACT_PATH);
  const fileSizePolicy = readJson(DEFAULT_FILE_SIZE_POLICY_PATH);
  const gateRegistry = loadGateRegistry(DEFAULT_GATE_REGISTRY_PATH);
  const verifyProfiles = loadVerifyProfiles(DEFAULT_VERIFY_PROFILES_PATH);

  const packageScriptCount = Object.keys(packageJson.scripts || {}).length;
  const exceptionCount = Array.isArray(fileSizePolicy.exceptions) ? fileSizePolicy.exceptions.length : 0;
  const exceptionCeiling = Math.max(
    1,
    Math.floor(Number(fileSizePolicy.exception_count_ceiling || exceptionCount)),
  );
  const toolingGovernance = commandRegistryPolicy.tooling_governance || {};
  const ciScriptRoot = cleanText(toolingGovernance.ci_script_root || 'tests/tooling/scripts/ci', 200);
  const sharedImportMarkers = Array.from(
    new Set(
      [
        cleanText(toolingGovernance.shared_cli_import || '', 200),
        cleanText(toolingGovernance.shared_result_import || '', 200),
        '../../lib/process.ts',
        '../../lib/git.ts',
        '../../lib/artifacts.ts',
        '../../lib/manifest.ts',
        '../../lib/runner.ts',
        '../../lib/',
      ].filter(Boolean),
    ),
  );
  const toolingAdoption = collectToolingAdoption(ciScriptRoot, sharedImportMarkers);
  const rootSurfaceDrift = collectRootSurfaceDrift(DEFAULT_ROOT_SURFACE_CONTRACT_PATH);

  const thresholds = {
    max_package_scripts: 950,
    max_file_size_exceptions: exceptionCeiling,
    min_shared_tooling_adoption_ratio: 0.25,
    min_shared_tooling_file_count: 25,
    max_standalone_parse_args_files: 65,
    max_root_hard_violations: 0,
    max_deprecated_root_entries: 20,
  };

  const failures: Array<{ id: string; detail: string }> = [];
  const warnings: Array<{ id: string; detail: string }> = [];

  if (packageScriptCount > thresholds.max_package_scripts) {
    failures.push({
      id: 'package_script_budget_exceeded',
      detail: `script_count=${packageScriptCount}; max=${thresholds.max_package_scripts}`,
    });
  } else if (packageScriptCount >= thresholds.max_package_scripts - 25) {
    warnings.push({
      id: 'package_script_budget_near_limit',
      detail: `script_count=${packageScriptCount}; max=${thresholds.max_package_scripts}`,
    });
  }

  if (exceptionCount > thresholds.max_file_size_exceptions) {
    failures.push({
      id: 'file_size_exception_ceiling_exceeded',
      detail: `exception_count=${exceptionCount}; max=${thresholds.max_file_size_exceptions}`,
    });
  } else if (exceptionCount >= Math.max(1, thresholds.max_file_size_exceptions - 10)) {
    warnings.push({
      id: 'file_size_exception_ceiling_near_limit',
      detail: `exception_count=${exceptionCount}; max=${thresholds.max_file_size_exceptions}`,
    });
  }

  if (
    toolingAdoption.shared_tooling_adoption_ratio < thresholds.min_shared_tooling_adoption_ratio ||
    toolingAdoption.shared_tooling_files < thresholds.min_shared_tooling_file_count
  ) {
    failures.push({
      id: 'shared_tooling_adoption_too_low',
      detail:
        `shared_tooling_files=${toolingAdoption.shared_tooling_files}; ` +
        `ci_gate_files=${toolingAdoption.ci_gate_files}; ` +
        `adoption_ratio=${toolingAdoption.shared_tooling_adoption_ratio}; ` +
        `minimum_ratio=${thresholds.min_shared_tooling_adoption_ratio}; ` +
        `minimum_files=${thresholds.min_shared_tooling_file_count}`,
    });
  }

  if (toolingAdoption.standalone_parse_args_files > thresholds.max_standalone_parse_args_files) {
    failures.push({
      id: 'standalone_parse_args_budget_exceeded',
      detail:
        `standalone_parse_args_files=${toolingAdoption.standalone_parse_args_files}; ` +
        `max=${thresholds.max_standalone_parse_args_files}`,
    });
  } else if (
    toolingAdoption.standalone_parse_args_files >= thresholds.max_standalone_parse_args_files - 5
  ) {
    warnings.push({
      id: 'standalone_parse_args_budget_near_limit',
      detail:
        `standalone_parse_args_files=${toolingAdoption.standalone_parse_args_files}; ` +
        `max=${thresholds.max_standalone_parse_args_files}`,
    });
  }

  if (rootSurfaceDrift.hard_violations.length > thresholds.max_root_hard_violations) {
    failures.push({
      id: 'root_surface_contract_violation',
      detail:
        `hard_violations=${rootSurfaceDrift.hard_violations.length}; ` +
        `examples=${rootSurfaceDrift.hard_violations
          .slice(0, 5)
          .map((row) => row.entry)
          .join(', ')}`,
    });
  }

  if (rootSurfaceDrift.deprecated_present.length > thresholds.max_deprecated_root_entries) {
    failures.push({
      id: 'deprecated_root_drift_budget_exceeded',
      detail:
        `deprecated_present=${rootSurfaceDrift.deprecated_present.length}; ` +
        `max=${thresholds.max_deprecated_root_entries}`,
    });
  } else if (rootSurfaceDrift.deprecated_present.length > 0) {
    warnings.push({
      id: 'deprecated_root_drift_present',
      detail:
        `deprecated_present=${rootSurfaceDrift.deprecated_present.length}; ` +
        `examples=${rootSurfaceDrift.deprecated_present.slice(0, 8).join(', ')}`,
    });
  }

  return {
    ok: failures.length === 0,
    type: 'repo_health_gate',
    generated_at: new Date().toISOString(),
    duration_ms: Date.now() - started,
    owner: 'ops',
    revision: currentRevision(ROOT),
    inputs: {
      package_json_path: DEFAULT_PACKAGE_JSON_PATH,
      command_registry_policy_path: DEFAULT_COMMAND_REGISTRY_POLICY_PATH,
      gate_registry_path: DEFAULT_GATE_REGISTRY_PATH,
      verify_profiles_path: DEFAULT_VERIFY_PROFILES_PATH,
      root_surface_contract_path: DEFAULT_ROOT_SURFACE_CONTRACT_PATH,
      file_size_policy_path: DEFAULT_FILE_SIZE_POLICY_PATH,
      ci_script_root: ciScriptRoot,
    },
    thresholds,
    summary: {
      pass: failures.length === 0,
      package_script_count: packageScriptCount,
      file_size_exception_count: exceptionCount,
      tooling_gate_count: Object.keys(gateRegistry.gates || {}).length,
      verify_profile_count: Object.keys(verifyProfiles.profiles || {}).length,
      curated_operator_surface_count: Array.isArray(commandRegistryPolicy.curated_operator_surface)
        ? commandRegistryPolicy.curated_operator_surface.length
        : 0,
      ci_gate_file_count: toolingAdoption.ci_gate_files,
      shared_tooling_file_count: toolingAdoption.shared_tooling_files,
      shared_tooling_adoption_ratio: toolingAdoption.shared_tooling_adoption_ratio,
      standalone_parse_args_file_count: toolingAdoption.standalone_parse_args_files,
      root_hard_violation_count: rootSurfaceDrift.hard_violations.length,
      deprecated_root_entry_count: rootSurfaceDrift.deprecated_present.length,
    },
    failures,
    warnings,
    artifact_paths: [DEFAULT_REPO_HEALTH_ARTIFACT_PATH],
    metrics: {
      shared_tooling_examples: toolingAdoption.shared_tooling_examples,
      standalone_parse_args_examples: toolingAdoption.standalone_parse_args_examples,
      deprecated_root_entries: rootSurfaceDrift.deprecated_present,
      root_hard_violations: rootSurfaceDrift.hard_violations,
      package_script_keys_sample: Object.keys(packageJson.scripts || {}).sort().slice(0, 25),
    },
  };
}

function parseArgs(argv: string[]) {
  const mode = cleanText(argv[0] || 'list', 24).toLowerCase() as Mode;
  const common = parseStrictOutArgs(argv.slice(1), {});
  return {
    mode: mode === 'gate' || mode === 'profile' || mode === 'health' ? mode : 'list',
    id: cleanText(readFlag(argv.slice(1), 'id') || '', 160),
    registry: cleanText(readFlag(argv.slice(1), 'registry') || DEFAULT_GATE_REGISTRY_PATH, 260),
    profiles: cleanText(readFlag(argv.slice(1), 'profiles') || DEFAULT_VERIFY_PROFILES_PATH, 260),
    strict: common.strict,
    json: common.json,
    out: cleanText(common.out || '', 400),
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  if (args.mode === 'list') {
    const payload = collectRegistrySummary(args.registry, args.profiles);
    return emitStructuredResult(payload, {
      outPath: args.out || '',
      strict: args.strict,
      ok: true,
      stdout: args.json || true,
    });
  }

  if (args.mode === 'health') {
    const payload = buildRepoHealthReport();
    return emitStructuredResult(payload, {
      outPath: args.out || DEFAULT_REPO_HEALTH_ARTIFACT_PATH,
      strict: args.strict,
      ok: Boolean(payload.ok),
    });
  }

  if (!args.id) {
    const payload = {
      ok: false,
      type: 'tooling_registry_runner',
      generated_at: new Date().toISOString(),
      summary: { pass: false },
      failures: [{ id: 'missing_id', detail: `mode=${args.mode}` }],
      inputs: {
        mode: args.mode,
        registry_path: args.registry,
        profiles_path: args.profiles,
      },
      artifact_paths: [],
    };
    return emitStructuredResult(payload, {
      outPath: args.out || '',
      strict: args.strict,
      ok: false,
    });
  }

  const payload =
    args.mode === 'gate'
      ? executeGate(args.id, {
          registryPath: args.registry,
          strict: args.strict,
          outPath: args.out || undefined,
        })
      : executeProfile(args.id, {
          registryPath: args.registry,
          profilesPath: args.profiles,
          strict: args.strict,
          outPath: args.out || undefined,
        });
  return emitStructuredResult(payload, {
    outPath: '',
    strict: args.strict,
    ok: Boolean(payload.ok),
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
