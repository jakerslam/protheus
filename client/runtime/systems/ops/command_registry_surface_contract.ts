#!/usr/bin/env node
'use strict';

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { execSync } from 'node:child_process';

type CommandPolicy = {
  version: string;
  paths: {
    package_json_path: string;
    registry_path: string;
    generated_registry_path: string;
    docs_path: string;
    latest_path: string;
    receipts_path: string;
  };
  tooling_governance?: {
    ci_script_root?: string;
    tooling_gate_registry_path?: string;
    base_ref?: string;
    shared_cli_import?: string;
    shared_result_import?: string;
  };
  curated_operator_surface?: string[];
  groups?: Array<{ prefix: string; owner: string; scope: string; risk_tier: number }>;
  explicit?: Record<string, { owner: string; scope: string; risk_tier: number }>;
};

type LaneRegistryEntry = {
  id: string;
  command: string;
  source_script?: string;
};

type LaneRegistry = {
  version: string;
  run?: Record<string, LaneRegistryEntry>;
  test?: Record<string, LaneRegistryEntry>;
};

type GeneratedRow = {
  command: string;
  owner: string;
  scope: string;
  risk_tier: number;
  indexed: boolean;
  source: 'package' | 'lane_registry';
};

type ToolingGateRegistry = {
  version?: string;
  gates?: Record<string, { script?: string; command?: string[] }>;
};

function clean(value: unknown, max = 400): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseArgs(argv: string[]) {
  const out = {
    command: clean(argv[0] || 'check', 32).toLowerCase(),
    strict: false,
    json: false,
    policy: 'client/runtime/config/command_registry_policy.json',
    laneRegistry: 'client/runtime/config/lane_command_registry.json',
  };
  for (const token of argv.slice(1)) {
    const value = clean(token, 600);
    if (!value) continue;
    if (value === '--strict' || value === '--strict=1' || value === '--strict=true') out.strict = true;
    else if (value === '--json' || value === '--json=1' || value === '--json=true') out.json = true;
    else if (value.startsWith('--policy=')) out.policy = clean(value.slice('--policy='.length), 260);
    else if (value.startsWith('--lane-registry=')) out.laneRegistry = clean(value.slice('--lane-registry='.length), 260);
  }
  return out;
}

function readJson<T>(filePath: string): T {
  return JSON.parse(readFileSync(resolve(filePath), 'utf8')) as T;
}

function classifyCommand(name: string, policy: CommandPolicy) {
  const explicit = policy.explicit?.[name];
  if (explicit) return explicit;
  for (const group of policy.groups || []) {
    if (name.startsWith(group.prefix)) {
      return {
        owner: clean(group.owner, 80),
        scope: clean(group.scope, 80),
        risk_tier: Number(group.risk_tier || 0),
      };
    }
  }
  return null;
}

function buildGeneratedRegistry(policy: CommandPolicy, laneRegistryPath: string) {
  const pkg = readJson<{ scripts?: Record<string, string> }>(policy.paths.package_json_path);
  const laneRegistry = readJson<LaneRegistry>(laneRegistryPath);
  const packageScripts = pkg.scripts || {};
  const commands: GeneratedRow[] = [];
  const unmatchedPackageScripts: string[] = [];
  const scriptNames = Object.keys(packageScripts);
  const curatedOperatorSurface = (policy.curated_operator_surface || []).map((value) =>
    clean(value, 200),
  );

  for (const [name] of Object.entries(packageScripts).sort((a, b) => a[0].localeCompare(b[0]))) {
    const classification = classifyCommand(name, policy);
    if (!classification) {
      unmatchedPackageScripts.push(name);
      continue;
    }
    commands.push({
      command: name,
      owner: classification.owner,
      scope: classification.scope,
      risk_tier: classification.risk_tier,
      indexed: false,
      source: 'package',
    });
  }

  commands.push({
    command: 'lane:run -- --id=<ID>',
    owner: 'race',
    scope: 'lane',
    risk_tier: 2,
    indexed: true,
    source: 'lane_registry',
  });
  commands.push({
    command: 'test:lane:run -- --id=<ID>',
    owner: 'qa',
    scope: 'validation',
    risk_tier: 1,
    indexed: true,
    source: 'lane_registry',
  });

  const missingCuratedOperatorSurface = curatedOperatorSurface.filter(
    (name) => !scriptNames.includes(name),
  );
  const unclassifiedCuratedOperatorSurface = curatedOperatorSurface.filter(
    (name) => !classifyCommand(name, policy),
  );
  const canonicalIndexedSurface = [
    'lane:run',
    'lane:list',
    'test:lane:run',
    'test:lane:list',
  ];
  const missingIndexedSurfaceScripts = canonicalIndexedSurface.filter(
    (name) => !scriptNames.includes(name),
  );
  const toolingGovernance = buildToolingGovernance(policy, packageScripts);

  const payload = {
    ok:
      missingCuratedOperatorSurface.length === 0 &&
      unclassifiedCuratedOperatorSurface.length === 0 &&
      missingIndexedSurfaceScripts.length === 0 &&
      toolingGovernance.pass,
    type: 'command_registry_surface_contract',
    version: clean(policy.version || '1.0', 40),
    generated_at: new Date().toISOString(),
    package_script_count: scriptNames.length,
    indexed_lane_run_count: Object.keys(laneRegistry.run || {}).length,
    indexed_lane_test_count: Object.keys(laneRegistry.test || {}).length,
    unmatched_package_scripts: unmatchedPackageScripts,
    curated_operator_surface_count: curatedOperatorSurface.length,
    missing_curated_operator_surface: missingCuratedOperatorSurface,
    unclassified_curated_operator_surface: unclassifiedCuratedOperatorSurface,
    missing_indexed_surface_scripts: missingIndexedSurfaceScripts,
    tooling_governance: toolingGovernance,
    commands,
  };

  const markdown = [
    '# Command Registry',
    '',
    `Generated: ${payload.generated_at}`,
    '',
    '## Summary',
    `- package scripts: ${payload.package_script_count}`,
    `- indexed lane run entries: ${payload.indexed_lane_run_count}`,
    `- indexed lane test entries: ${payload.indexed_lane_test_count}`,
    `- curated operator surface entries: ${payload.curated_operator_surface_count}`,
    `- unmatched package scripts: ${payload.unmatched_package_scripts.length}`,
    `- missing curated operator surface commands: ${payload.missing_curated_operator_surface.length}`,
    `- unclassified curated operator surface commands: ${payload.unclassified_curated_operator_surface.length}`,
    `- missing indexed surface commands: ${payload.missing_indexed_surface_scripts.length}`,
    `- tooling governance added-script pass: ${payload.tooling_governance.pass}`,
    `- tooling governance added scripts checked: ${payload.tooling_governance.added_script_count}`,
    '',
    '## Canonical Indexed Lane Surface',
    '- `npm run -s lane:run -- --id=<ID>`',
    '- `npm run -s test:lane:run -- --id=<ID>`',
    '- `npm run -s lane:list -- --json=1`',
    '- `npm run -s test:lane:list -- --json=1`',
    '',
    '## Tooling Governance',
    `- Added CI scripts checked: ${payload.tooling_governance.added_script_count}`,
    `- Registered added CI scripts: ${payload.tooling_governance.registered_added_script_count}`,
    `- Missing registry entries: ${payload.tooling_governance.unregistered_added_scripts.length}`,
    `- Missing shared cli import: ${payload.tooling_governance.missing_shared_cli_import.length}`,
    `- Missing shared result import: ${payload.tooling_governance.missing_shared_result_import.length}`,
    `- Missing emitStructuredResult: ${payload.tooling_governance.missing_emit_structured_result.length}`,
    `- Local parseArgs usage: ${payload.tooling_governance.local_parse_args_added_scripts.length}`,
    '',
    '## Curated Commands',
    '| Command | Owner | Scope | Risk | Source |',
    '| --- | --- | --- | ---: | --- |',
    ...commands.map(
      (row) =>
        `| \`${row.command}\` | ${row.owner} | ${row.scope} | ${row.risk_tier} | ${row.indexed ? 'indexed' : row.source} |`,
    ),
    '',
  ].join('\n');

  return { payload, markdown };
}

function normalizePath(value: string): string {
  return clean(value, 600).replace(/\\/g, '/');
}

function safeExec(command: string): string {
  try {
    return String(execSync(command, { encoding: 'utf8' }) || '');
  } catch (error) {
    return String((error as { stdout?: string })?.stdout || '');
  }
}

function resolveGovernanceBaseRef(baseRef: string): string {
  const explicit = clean(baseRef, 120);
  if (explicit) {
    const mergeBase = safeExec(`git merge-base HEAD ${explicit}`).trim();
    if (mergeBase) return mergeBase;
  }
  return clean(safeExec('git rev-parse HEAD~1').trim(), 120) || 'HEAD';
}

function collectAddedCiScripts(ciRoot: string, baseRef: string): string[] {
  const normalizedRoot = normalizePath(ciRoot).replace(/\/+$/, '');
  const out = new Set<string>();
  const mergeBase = resolveGovernanceBaseRef(baseRef);
  const diffRaw = safeExec(`git diff --name-only --diff-filter=A ${mergeBase}...HEAD -- ${normalizedRoot}`);
  for (const line of diffRaw.split('\n')) {
    const file = normalizePath(line);
    if (file.startsWith(`${normalizedRoot}/`) && file.endsWith('.ts')) out.add(file);
  }
  const statusRaw = safeExec(`git status --porcelain=v1 -uall -- ${normalizedRoot}`);
  for (const line of statusRaw.split('\n')) {
    const raw = String(line || '');
    if (raw.length < 4) continue;
    const status = raw.slice(0, 2);
    const file = normalizePath(raw.slice(3).trim());
    if (!file.startsWith(`${normalizedRoot}/`) || !file.endsWith('.ts')) continue;
    if (status === '??' || status.includes('A')) out.add(file);
  }
  return [...out].sort((a, b) => a.localeCompare(b));
}

function extractRegisteredCiScriptPaths(
  registryPath: string,
  packageScripts: Record<string, string>,
): Set<string> {
  const out = new Set<string>();
  const registry = readJson<ToolingGateRegistry>(registryPath);
  const filePattern = /tests\/tooling\/scripts\/ci\/[A-Za-z0-9_./-]+\.ts/g;
  for (const gate of Object.values(registry.gates || {})) {
    for (const part of gate.command || []) {
      const normalized = normalizePath(part);
      if (normalized.startsWith('tests/tooling/scripts/ci/') && normalized.endsWith('.ts')) {
        out.add(normalized);
      }
    }
    if (!gate.script) continue;
    const command = clean(packageScripts[gate.script] || '', 4000);
    if (!command) continue;
    const matches = command.match(filePattern) || [];
    for (const match of matches) out.add(normalizePath(match));
  }
  return out;
}

function buildToolingGovernance(
  policy: CommandPolicy,
  packageScripts: Record<string, string>,
) {
  const config = policy.tooling_governance || {};
  const ciRoot = clean(config.ci_script_root || 'tests/tooling/scripts/ci', 260);
  const registryPath = clean(
    config.tooling_gate_registry_path || 'tests/tooling/config/tooling_gate_registry.json',
    260,
  );
  const baseRef = clean(config.base_ref || 'origin/main', 120);
  const sharedCliImport = clean(config.shared_cli_import || '../../lib/cli.ts', 260);
  const sharedResultImport = clean(config.shared_result_import || '../../lib/result.ts', 260);
  const addedScripts = collectAddedCiScripts(ciRoot, baseRef);
  const registeredScripts = extractRegisteredCiScriptPaths(registryPath, packageScripts);
  const missingSharedCliImport: string[] = [];
  const missingSharedResultImport: string[] = [];
  const missingEmitStructuredResult: string[] = [];
  const localParseArgsAddedScripts: string[] = [];
  const unregisteredAddedScripts: string[] = [];

  for (const relPath of addedScripts) {
    const abs = resolve(relPath);
    const source = readFileSync(abs, 'utf8');
    if (!registeredScripts.has(relPath)) unregisteredAddedScripts.push(relPath);
    if (!source.includes(sharedCliImport)) missingSharedCliImport.push(relPath);
    if (!source.includes(sharedResultImport)) missingSharedResultImport.push(relPath);
    if (!source.includes('emitStructuredResult(')) missingEmitStructuredResult.push(relPath);
    if (
      /\bfunction\s+parseArgs\s*\(/.test(source) ||
      /\bfunction\s+parseCliFlags\s*\(/.test(source) ||
      /\bconst\s+parseArgs\s*=/.test(source)
    ) {
      localParseArgsAddedScripts.push(relPath);
    }
  }

  return {
    pass:
      unregisteredAddedScripts.length === 0 &&
      missingSharedCliImport.length === 0 &&
      missingSharedResultImport.length === 0 &&
      missingEmitStructuredResult.length === 0 &&
      localParseArgsAddedScripts.length === 0,
    base_ref: baseRef,
    ci_script_root: ciRoot,
    tooling_gate_registry_path: registryPath,
    added_script_count: addedScripts.length,
    registered_added_script_count: addedScripts.filter((row) => registeredScripts.has(row)).length,
    added_scripts: addedScripts,
    unregistered_added_scripts: unregisteredAddedScripts,
    missing_shared_cli_import: missingSharedCliImport,
    missing_shared_result_import: missingSharedResultImport,
    missing_emit_structured_result: missingEmitStructuredResult,
    local_parse_args_added_scripts: localParseArgsAddedScripts,
  };
}

function writeText(filePath: string, text: string) {
  const abs = resolve(filePath);
  mkdirSync(dirname(abs), { recursive: true });
  writeFileSync(abs, text);
}

function main(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const policy = readJson<CommandPolicy>(args.policy);
  const { payload, markdown } = buildGeneratedRegistry(policy, args.laneRegistry);

  writeText(policy.paths.generated_registry_path, `${JSON.stringify(payload, null, 2)}\n`);
  writeText(policy.paths.latest_path, `${JSON.stringify(payload, null, 2)}\n`);
  writeText(policy.paths.receipts_path, `${JSON.stringify(payload)}\n`);
  if (args.command === 'sync') writeText(policy.paths.docs_path, markdown);

  if (args.json || args.command !== 'sync') {
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  } else {
    process.stdout.write(markdown);
  }

  if (args.strict && !payload.ok) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(main(process.argv.slice(2)));
}

module.exports = {
  main,
  buildGeneratedRegistry,
};
