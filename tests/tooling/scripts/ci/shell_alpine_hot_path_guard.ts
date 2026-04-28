#!/usr/bin/env node
/* eslint-disable no-console */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { extname, isAbsolute, resolve, relative } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SCAN_ROOT = 'client/runtime/systems/ui/infring_static/index_body.html.parts';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_alpine_hot_path_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_ALPINE_HOT_PATH_GUARD_CURRENT.md';

type Args = {
  strict: boolean;
  scanRoot: string;
  scanFiles: string[];
  outJson: string;
  outMarkdown: string;
};

type Finding = {
  path: string;
  line: number;
  column: number;
  expression: string;
  collection: string;
  inactive: boolean;
  rule_id: string;
  detail: string;
};

type Rule = {
  id: string;
  label: string;
  matches: (collection: string) => boolean;
};

const HOT_PATH_RULES: Rule[] = [
  { id: 'messages_collection', label: 'message thread collection', matches: (value) => collectionStartsWith(value, 'messages') },
  { id: 'filtered_messages_collection', label: 'filtered message collection', matches: (value) => collectionStartsWith(value, 'filteredMessages') },
  { id: 'all_filtered_messages_collection', label: 'full filtered message collection', matches: (value) => collectionStartsWith(value, 'allFilteredMessages') },
  { id: 'agent_collection', label: 'agent collection', matches: (value) => collectionStartsWith(value, 'agents') || collectionStartsWith(value, '$store.app.agents') },
  { id: 'session_collection', label: 'session collection', matches: (value) => collectionStartsWith(value, 'sessions') || collectionStartsWith(value, 'filteredSessions') },
  { id: 'chat_sidebar_collection', label: 'chat sidebar collection', matches: (value) => /^chatSidebar(?:VisibleRows|Rows|Agents|SearchResults)\b/.test(normalizeCollectionRoot(value)) },
];

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  const scanFilesRaw = cleanText(readFlag(argv, 'scan-files') || '', 2000);
  return {
    strict: common.strict,
    scanRoot: cleanText(readFlag(argv, 'scan-root') || DEFAULT_SCAN_ROOT, 400),
    scanFiles: scanFilesRaw ? scanFilesRaw.split(',').map((file) => cleanText(file, 400)).filter(Boolean) : [],
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function displayPath(path: string): string {
  const abs = isAbsolute(path) ? path : resolve(ROOT, path);
  const rel = relative(ROOT, abs);
  return rel && !rel.startsWith('..') ? rel : abs;
}

function readText(path: string): string {
  return readFileSync(isAbsolute(path) ? path : resolve(ROOT, path), 'utf8');
}

function gitFiles(args: string[]): string[] {
  try {
    return execFileSync('git', args, { cwd: ROOT, encoding: 'utf8' }).split('\0').map((file) => file.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

function filesToScan(args: Args): string[] {
  if (args.scanFiles.length) return args.scanFiles.filter((file) => existsSync(isAbsolute(file) ? file : resolve(ROOT, file)));
  const files = new Set([...gitFiles(['ls-files', '-z']), ...gitFiles(['ls-files', '--others', '--exclude-standard', '-z'])]);
  return [...files].filter((file) => {
    return (file === args.scanRoot || file.startsWith(`${args.scanRoot}/`)) &&
      extname(file) === '.html' &&
      existsSync(resolve(ROOT, file));
  }).sort();
}

function lineColumn(source: string, offset: number): { line: number; column: number } {
  let line = 1;
  let column = 1;
  for (let i = 0; i < offset; i += 1) {
    if (source[i] === '\n') {
      line += 1;
      column = 1;
    } else {
      column += 1;
    }
  }
  return { line, column };
}

function isFalseTemplate(token: string): boolean {
  return /\bx-if\s*=\s*(['"])\s*false\s*\1/i.test(token);
}

function hasDisabledTemplateAncestor(source: string, offset: number): boolean {
  const stack: boolean[] = [];
  const tokenRegex = /<\/template\s*>|<template\b[^>]*>/gi;
  let match = tokenRegex.exec(source);
  while (match && match.index < offset) {
    const token = match[0];
    if (/^<\//.test(token)) {
      stack.pop();
    } else {
      stack.push(stack.some(Boolean) || isFalseTemplate(token));
    }
    match = tokenRegex.exec(source);
  }
  return stack.some(Boolean);
}

function collectionExpression(expression: string): string {
  const normalized = expression.replace(/\s+/g, ' ').trim();
  const match = /\bin\s+(.+)$/i.exec(normalized);
  if (!match) return normalized;
  return match[1].replace(/^\((.*)\)$/u, '$1').trim();
}

function normalizeCollectionRoot(collection: string): string {
  return collection
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^\(+/, '')
    .replace(/\)+$/u, '')
    .trim()
    .replace(/\s*\|\|\s*\[\]\s*$/u, '')
    .trim();
}

function collectionStartsWith(collection: string, name: string): boolean {
  const root = normalizeCollectionRoot(collection).replace(/\s*\.\s*/g, '.');
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`^${escaped}(?:\\b|\\s|\\.|\\[|\\||\\)|$)`).test(root);
}

function matchingRule(collection: string): Rule | null {
  return HOT_PATH_RULES.find((rule) => rule.matches(collection)) || null;
}

function findingsForFile(path: string): Finding[] {
  const source = readText(path);
  const findings: Finding[] = [];
  const xForRegex = /<template\b[^>]*\bx-for\s*=\s*(['"])([\s\S]*?)\1[^>]*>/gi;
  let match = xForRegex.exec(source);
  while (match) {
    const expression = match[2].replace(/\s+/g, ' ').trim();
    const collection = collectionExpression(expression);
    const rule = matchingRule(collection);
    if (rule) {
      const position = lineColumn(source, match.index);
      const inactive = hasDisabledTemplateAncestor(source, match.index);
      findings.push({
        path: displayPath(path),
        line: position.line,
        column: position.column,
        expression,
        collection,
        inactive,
        rule_id: rule.id,
        detail: inactive
          ? `Ignored because the x-for is inside a template x-if="false" legacy island.`
          : `Active Alpine x-for over ${rule.label} is forbidden on shell hot paths.`,
      });
    }
    match = xForRegex.exec(source);
  }
  return findings;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Alpine Hot Path Guard');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push(`- scanned_files: ${payload.summary.scanned_files}`);
  lines.push(`- active_violations: ${payload.summary.active_violations}`);
  lines.push(`- inactive_legacy_hits: ${payload.summary.inactive_legacy_hits}`);
  lines.push('');
  lines.push('## Active Violations');
  if (!payload.active_violations.length) lines.push('- none');
  for (const row of payload.active_violations) {
    lines.push(`- ${row.path}:${row.line}:${row.column} ${row.rule_id} \`${row.expression}\``);
  }
  lines.push('');
  lines.push('## Inactive Legacy Hits');
  if (!payload.inactive_legacy_hits.length) lines.push('- none');
  for (const row of payload.inactive_legacy_hits) {
    lines.push(`- ${row.path}:${row.line}:${row.column} ${row.rule_id} \`${row.expression}\``);
  }
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = readArgs(argv);
  const files = filesToScan(args);
  const findings = files.flatMap((file) => findingsForFile(file));
  const active = findings.filter((finding) => !finding.inactive);
  const inactive = findings.filter((finding) => finding.inactive);
  const payload = {
    ok: active.length === 0,
    type: 'shell_alpine_hot_path_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    scan_root: args.scanFiles.length ? null : args.scanRoot,
    summary: {
      pass: active.length === 0,
      scanned_files: files.length,
      active_violations: active.length,
      inactive_legacy_hits: inactive.length,
    },
    active_violations: active,
    inactive_legacy_hits: inactive,
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: args.outJson });
  if (!payload.ok && args.strict) process.exitCode = 1;
}

run().catch((error) => {
  const payload = { ok: false, type: 'shell_alpine_hot_path_guard', error: error instanceof Error ? error.message : String(error) };
  emitStructuredResult(payload, { ok: false, outPath: DEFAULT_OUT_JSON });
  process.exitCode = 1;
});
