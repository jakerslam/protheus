#!/usr/bin/env node
'use strict';

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

type CanonicalPath = {
  script: string;
  command: string;
  summary: string;
};

type CommandProfile = {
  version: string;
  canonical_paths: Record<string, CanonicalPath>;
  namespace_order?: string[];
  namespace_descriptions?: Record<string, string>;
};

type ScriptEntry = {
  name: string;
  command: string;
  namespace: string;
};

type NamespaceEntry = {
  name: string;
  count: number;
  description: string;
  scripts: ScriptEntry[];
};

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseArgs(argv: string[]) {
  let json = false;
  let namespace = '';
  let match = '';
  let limit = 12;
  for (const raw of argv) {
    const token = clean(raw, 400);
    if (!token) continue;
    if (token === '--json' || token === '--json=1' || token === '--json=true') {
      json = true;
      continue;
    }
    if (token.startsWith('--namespace=')) {
      namespace = clean(token.slice('--namespace='.length), 120).toLowerCase();
      continue;
    }
    if (token.startsWith('--match=')) {
      match = clean(token.slice('--match='.length), 120).toLowerCase();
      continue;
    }
    if (token.startsWith('--limit=')) {
      const parsed = Number.parseInt(token.slice('--limit='.length), 10);
      if (Number.isFinite(parsed) && parsed > 0) {
        limit = Math.min(parsed, 100);
      }
    }
  }
  return { json, namespace, match, limit };
}

function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(resolve(path), 'utf8')) as T;
}

function topNamespace(scriptName: string): string {
  const normalized = clean(scriptName, 240);
  if (!normalized) return 'misc';
  if (!normalized.includes(':')) return 'misc';
  return clean(normalized.split(':', 1)[0], 80).toLowerCase() || 'misc';
}

function namespaceDescription(
  namespace: string,
  profile: CommandProfile,
  scripts: ScriptEntry[],
): string {
  const fromProfile = clean(profile.namespace_descriptions?.[namespace] || '', 240);
  if (fromProfile) return fromProfile;
  if (namespace === 'misc') {
    return 'Top-level scripts without a namespace prefix.';
  }
  const sample = scripts.slice(0, 3).map((entry) => entry.name).join(', ');
  return sample ? `Additional ${namespace} commands (for example: ${sample}).` : 'Additional commands.';
}

function compareNamespaces(
  left: NamespaceEntry,
  right: NamespaceEntry,
  order: string[],
): number {
  const leftIndex = order.indexOf(left.name);
  const rightIndex = order.indexOf(right.name);
  if (leftIndex >= 0 || rightIndex >= 0) {
    if (leftIndex < 0) return 1;
    if (rightIndex < 0) return -1;
    if (leftIndex !== rightIndex) return leftIndex - rightIndex;
  }
  if (left.count !== right.count) return right.count - left.count;
  return left.name.localeCompare(right.name);
}

function collectWorkspaceCommandIndex(
  packageJsonPath = 'package.json',
  profilePath = 'client/runtime/config/workspace_command_profiles.json',
) {
  const packageJson = readJson<{ scripts?: Record<string, string> }>(packageJsonPath);
  const profile = readJson<CommandProfile>(profilePath);
  const scripts = Object.entries(packageJson.scripts || {})
    .map(([name, command]) => ({
      name,
      command: clean(command, 800),
      namespace: topNamespace(name),
    }))
    .sort((a, b) => a.name.localeCompare(b.name));
  const grouped = new Map<string, ScriptEntry[]>();
  for (const entry of scripts) {
    const list = grouped.get(entry.namespace) || [];
    list.push(entry);
    grouped.set(entry.namespace, list);
  }
  const namespaceOrder = Array.isArray(profile.namespace_order)
    ? profile.namespace_order.map((row) => clean(row, 80).toLowerCase()).filter(Boolean)
    : [];
  const namespaces = [...grouped.entries()]
    .map(([name, rows]) => ({
      name,
      count: rows.length,
      description: namespaceDescription(name, profile, rows),
      scripts: rows,
    }))
    .sort((left, right) => compareNamespaces(left, right, namespaceOrder));
  return {
    ok: true,
    type: 'workspace_command_index',
    version: clean(profile.version || '1.0', 40),
    summary: {
      total_scripts: scripts.length,
      namespace_count: namespaces.length,
    },
    canonical_paths: profile.canonical_paths || {},
    namespaces,
  };
}

function filterIndex(
  payload: ReturnType<typeof collectWorkspaceCommandIndex>,
  namespace: string,
  match: string,
) {
  const normalizedNamespace = clean(namespace, 80).toLowerCase();
  const normalizedMatch = clean(match, 120).toLowerCase();
  const namespaces = payload.namespaces
    .filter((entry) => !normalizedNamespace || entry.name === normalizedNamespace)
    .map((entry) => {
      const scripts = normalizedMatch
        ? entry.scripts.filter((script) => {
            const haystack = `${script.name} ${script.command}`.toLowerCase();
            return haystack.includes(normalizedMatch);
          })
        : entry.scripts;
      return {
        ...entry,
        count: scripts.length,
        scripts,
      };
    })
    .filter((entry) => entry.scripts.length > 0);
  return {
    ...payload,
    summary: {
      ...payload.summary,
      filtered_namespace: normalizedNamespace || null,
      filtered_match: normalizedMatch || null,
      namespace_count: namespaces.length,
      total_scripts: namespaces.reduce((sum, entry) => sum + entry.scripts.length, 0),
    },
    namespaces,
  };
}

function renderCanonicalPaths(canonicalPaths: Record<string, CanonicalPath>): string[] {
  const lines = ['Canonical entrypoints'];
  for (const [key, value] of Object.entries(canonicalPaths)) {
    lines.push(`- ${key}: ${value.command}`);
    lines.push(`  ${value.summary}`);
  }
  return lines;
}

function renderNamespaces(namespaces: NamespaceEntry[], limit: number): string[] {
  const lines = ['Namespaces'];
  for (const entry of namespaces) {
    lines.push(`- ${entry.name} (${entry.count})`);
    lines.push(`  ${entry.description}`);
    for (const script of entry.scripts.slice(0, limit)) {
      lines.push(`  ${script.name}`);
    }
    if (entry.scripts.length > limit) {
      lines.push(`  ... ${entry.scripts.length - limit} more`);
    }
  }
  return lines;
}

function run(
  argv: string[] = process.argv.slice(2),
  packageJsonPath = 'package.json',
  profilePath = 'client/runtime/config/workspace_command_profiles.json',
): number {
  const { json, namespace, match, limit } = parseArgs(argv);
  const payload = filterIndex(
    collectWorkspaceCommandIndex(packageJsonPath, profilePath),
    namespace,
    match,
  );
  if (json) {
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
    return 0;
  }

  const lines = [
    'Workspace Command Index',
    '',
    `Total scripts: ${payload.summary.total_scripts}`,
    `Namespaces: ${payload.summary.namespace_count}`,
    '',
    ...renderCanonicalPaths(payload.canonical_paths),
    '',
    ...renderNamespaces(payload.namespaces, limit),
  ];
  process.stdout.write(`${lines.join('\n')}\n`);
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
  collectWorkspaceCommandIndex,
  filterIndex,
};
