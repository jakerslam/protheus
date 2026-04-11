#!/usr/bin/env node
'use strict';

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

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
  const commands: GeneratedRow[] = [];
  const unmatchedPackageScripts: string[] = [];
  const scriptNames = Object.keys(pkg.scripts || {});
  const curatedOperatorSurface = (policy.curated_operator_surface || []).map((value) =>
    clean(value, 200),
  );

  for (const [name] of Object.entries(pkg.scripts || {}).sort((a, b) => a[0].localeCompare(b[0]))) {
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

  const payload = {
    ok:
      missingCuratedOperatorSurface.length === 0 &&
      unclassifiedCuratedOperatorSurface.length === 0 &&
      missingIndexedSurfaceScripts.length === 0,
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
    '',
    '## Canonical Indexed Lane Surface',
    '- `npm run -s lane:run -- --id=<ID>`',
    '- `npm run -s test:lane:run -- --id=<ID>`',
    '- `npm run -s lane:list -- --json=1`',
    '- `npm run -s test:lane:list -- --json=1`',
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
