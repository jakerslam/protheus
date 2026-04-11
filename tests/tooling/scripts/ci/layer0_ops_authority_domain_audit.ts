#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();

type DomainRow = {
  id: string;
  shard: string;
  owner: string;
  purpose: string;
};

function parseArgs(argv: string[]) {
  return {
    registry: String(readFlag(argv, 'registry') || 'core/layer0/ops/authority_domains.json'),
    libIndex: String(readFlag(argv, 'lib-index') || 'core/layer0/ops/src/lib.rs.inc'),
    out: String(readFlag(argv, 'out') || ''),
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
  };
}

function readJson<T>(filePath: string): T {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8')) as T;
}

function normalizePath(value: string): string {
  return String(value).replace(/\\/g, '/');
}

function canonicalShardId(value: string): string {
  const normalized = normalizePath(value);
  const marker = 'lib.index.';
  const index = normalized.lastIndexOf(marker);
  return index >= 0 ? normalized.slice(index) : normalized;
}

function parseShardModules(shardPath: string): string[] {
  const abs = path.resolve(ROOT, shardPath);
  if (!fs.existsSync(abs)) return [];
  const raw = fs.readFileSync(abs, 'utf8');
  return [...raw.matchAll(/^\s*pub mod ([a-zA-Z0-9_]+);\s*$/gm)]
    .map((match) => match[1])
    .sort();
}

function parseLibShardOrder(libIndexPath: string): string[] {
  const raw = fs.readFileSync(path.resolve(ROOT, libIndexPath), 'utf8');
  return [...raw.matchAll(/"([^"]*lib\.index\.[^"]+\.rs)"/g)].map((match) =>
    canonicalShardId(match[1]),
  );
}

function revision(): string {
  try {
    return execSync('git rev-parse HEAD', { cwd: ROOT, encoding: 'utf8' }).trim();
  } catch {
    return 'unknown';
  }
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const registry = readJson<{ version: string; domains: DomainRow[] }>(args.registry);
  const shardOrder = parseLibShardOrder(args.libIndex);
  const shardSet = new Set(shardOrder);
  const registeredShardSet = new Set(
    (registry.domains || []).map((row) => canonicalShardId(row.shard)),
  );
  const duplicateModules = new Map<string, string[]>();
  const moduleOwners = new Map<string, string>();
  const emptyDomains: string[] = [];
  const missingShardRegistrations: string[] = [];
  const danglingRegistryShards: string[] = [];
  const domainRows = [];

  for (const shard of shardOrder) {
    if (!registeredShardSet.has(shard)) missingShardRegistrations.push(shard);
  }
  for (const row of registry.domains || []) {
    const shard = canonicalShardId(row.shard);
    if (!shardSet.has(shard)) danglingRegistryShards.push(shard);
    const modules = parseShardModules(normalizePath(row.shard));
    if (modules.length === 0) emptyDomains.push(row.id);
    for (const mod of modules) {
      const existing = moduleOwners.get(mod);
      if (existing && existing !== row.id) {
        const owners = new Set([existing, row.id, ...(duplicateModules.get(mod) || [])]);
        duplicateModules.set(mod, [...owners].sort());
      } else {
        moduleOwners.set(mod, row.id);
      }
    }
    domainRows.push({
      id: row.id,
      shard,
      owner: row.owner,
      purpose: row.purpose,
      module_count: modules.length,
      modules,
    });
  }

  const payload = {
    ok:
      missingShardRegistrations.length === 0 &&
      danglingRegistryShards.length === 0 &&
      emptyDomains.length === 0 &&
      duplicateModules.size === 0,
    type: 'layer0_ops_authority_domain_audit',
    generated_at: new Date().toISOString(),
    revision: revision(),
    registry_path: normalizePath(args.registry),
    lib_index_path: normalizePath(args.libIndex),
    summary: {
      domain_count: domainRows.length,
      shard_count: shardOrder.length,
      module_count: [...moduleOwners.keys()].length,
      missing_shard_registration_count: missingShardRegistrations.length,
      dangling_registry_shard_count: danglingRegistryShards.length,
      empty_domain_count: emptyDomains.length,
      duplicate_module_count: duplicateModules.size,
      pass:
        missingShardRegistrations.length === 0 &&
        danglingRegistryShards.length === 0 &&
        emptyDomains.length === 0 &&
        duplicateModules.size === 0,
    },
    domains: domainRows,
    missing_shard_registrations: missingShardRegistrations,
    dangling_registry_shards: danglingRegistryShards,
    empty_domains: emptyDomains,
    duplicate_modules: Object.fromEntries(
      [...duplicateModules.entries()].sort((a, b) => a[0].localeCompare(b[0])),
    ),
  };

  return emitStructuredResult(payload, {
    outPath: args.out || '',
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(main());
}

module.exports = {
  main,
};
