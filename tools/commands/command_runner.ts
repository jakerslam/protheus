#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = process.cwd();
const REGISTRY_PATH = 'tools/commands/command_registry.json';
type Entry = {
  id: string;
  group: string;
  command: string;
  lifecycle?: string;
  owner?: string;
  domain?: string;
  work_gate?: string;
  description?: string;
};
function registry(): { entries: Entry[]; script_count: number; group_counts: Record<string, number> } {
  return JSON.parse(fs.readFileSync(path.join(ROOT, REGISTRY_PATH), 'utf8'));
}
function quote(arg: string): string {
  if (/^[A-Za-z0-9_./:=@%+-]+$/.test(arg)) return arg;
  return "'" + arg.replace(/'/g, "'\\''") + "'";
}
function flag(args: string[], name: string): string {
  const prefix = `${name}=`;
  return args.find((arg) => arg.startsWith(prefix))?.slice(prefix.length) || '';
}
function includesText(entry: Entry, needle: string): boolean {
  if (!needle) return true;
  const haystack = [entry.id, entry.group, entry.domain, entry.work_gate, entry.lifecycle, entry.owner, entry.description, entry.command]
    .filter(Boolean)
    .join('\n')
    .toLowerCase();
  return haystack.includes(needle.toLowerCase());
}
function usage(): void {
  console.error('Usage: npm run -s cmd -- <command-id> [args...]');
  console.error('       npm run -s cmd -- list [--group=<group>] [--domain=<domain>] [--work-gate=<gate>] [--lifecycle=<state>] [--search=<text>]');
  console.error('       npm run -s cmd -- info <command-id>');
  console.error('       npm run -s cmd -- groups');
}
function publicEntry(entry: Entry): Partial<Entry> {
  return {
    id: entry.id,
    group: entry.group,
    lifecycle: entry.lifecycle,
    owner: entry.owner,
    domain: entry.domain,
    work_gate: entry.work_gate,
    description: entry.description,
  };
}
function main(): void {
  const data = registry();
  const args = process.argv.slice(2);
  const first = args[0] || 'help';
  if (first === 'help' || first === '--help' || first === '-h') { usage(); return; }
  if (first === 'list') {
    const filters = {
      group: flag(args, '--group'),
      domain: flag(args, '--domain'),
      workGate: flag(args, '--work-gate'),
      lifecycle: flag(args, '--lifecycle'),
      search: flag(args, '--search'),
    };
    const rows = data.entries.filter((entry) => {
      if (filters.group && entry.group !== filters.group) return false;
      if (filters.domain && entry.domain !== filters.domain) return false;
      if (filters.workGate && entry.work_gate !== filters.workGate) return false;
      if (filters.lifecycle && entry.lifecycle !== filters.lifecycle) return false;
      return includesText(entry, filters.search);
    });
    console.log(JSON.stringify({ ok: true, type: 'command_registry_list', count: rows.length, filters, entries: rows.map(publicEntry) }, null, 2));
    return;
  }
  if (first === 'info') {
    const id = args[1] || '';
    const entry = data.entries.find((row) => row.id === id);
    if (!entry) {
      console.error(`Unknown command id: ${id}`);
      process.exit(2);
    }
    console.log(JSON.stringify({ ok: true, type: 'command_registry_info', entry }, null, 2));
    return;
  }
  if (first === 'groups') {
    const domains = data.entries.reduce((acc: Record<string, number>, entry) => {
      const key = entry.domain || 'unclassified';
      acc[key] = (acc[key] || 0) + 1;
      return acc;
    }, {});
    const workGates = data.entries.reduce((acc: Record<string, number>, entry) => {
      const key = entry.work_gate || 'unclassified';
      acc[key] = (acc[key] || 0) + 1;
      return acc;
    }, {});
    console.log(JSON.stringify({ ok: true, type: 'command_registry_groups', script_count: data.script_count, group_counts: data.group_counts, domains, work_gates: workGates }, null, 2));
    return;
  }
  const entry = data.entries.find((row) => row.id === first);
  if (!entry) {
    console.error(`Unknown command id: ${first}`);
    usage();
    process.exit(2);
  }
  const forwarded = args.slice(1);
  const command = forwarded.length ? `${entry.command} ${forwarded.map(quote).join(' ')}` : entry.command;
  const result = spawnSync(command, { cwd: ROOT, shell: true, stdio: 'inherit', env: process.env });
  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }
  process.exit(typeof result.status === 'number' ? result.status : 1);
}
main();
