#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, hasFlag, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type Contract = 'transparent' | 'layout-host' | 'interactive-surface';
type ShellUse = {
  tag: string;
  file: string;
  hasClass: boolean;
};

const ROOT = process.cwd();
const DEFAULT_CONTRACT_PATH =
  'client/runtime/systems/ui/infring_static/js/svelte/svelte_shell_contracts.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/dashboard_svelte_shell_contract_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/DASHBOARD_SVELTE_SHELL_CONTRACT_GUARD_CURRENT.md';
const BODY_PARTS_DIR = 'client/runtime/systems/ui/infring_static/index_body.html.parts';
const SVELTE_SOURCE_DIR = 'client/runtime/systems/ui/infring_static/js/svelte';
const VALID_CONTRACTS = new Set<Contract>(['transparent', 'layout-host', 'interactive-surface']);

function args(argv: string[]) {
  return {
    strict: hasFlag(argv, 'strict') || parseBool(readFlag(argv, 'strict'), false),
    contractPath: cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT_PATH, 400),
    outJson: cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 400),
  };
}

function readJson(filePath: string): any {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8'));
}

function readTextMaybe(filePath: string): string {
  const resolved = path.resolve(ROOT, filePath);
  return fs.existsSync(resolved) ? fs.readFileSync(resolved, 'utf8') : '';
}

function filesIn(dirPath: string, suffix: string): string[] {
  const resolved = path.resolve(ROOT, dirPath);
  if (!fs.existsSync(resolved)) return [];
  return fs
    .readdirSync(resolved, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(suffix))
    .map((entry) => path.join(dirPath, entry.name))
    .sort((a, b) => a.localeCompare(b, 'en'));
}

function htmlShellUses(): ShellUse[] {
  const out: ShellUse[] = [];
  for (const file of filesIn(BODY_PARTS_DIR, '.html')) {
    const text = readTextMaybe(file);
    for (const match of text.matchAll(/<(?<tag>infring-[a-z0-9-]+-shell)\b(?<attrs>[^>]*)>/g)) {
      const tag = match.groups?.tag || '';
      const attrs = match.groups?.attrs || '';
      out.push({ tag, file, hasClass: /\bclass\s*=/.test(attrs) });
    }
  }
  return out;
}

function sourceShellTags(): string[] {
  const tags = new Set<string>();
  for (const file of filesIn(SVELTE_SOURCE_DIR, '_svelte_source.ts')) {
    const text = readTextMaybe(file);
    const match = text.match(/COMPONENT_TAG\s*=\s*['"](?<tag>infring-[a-z0-9-]+-shell)['"]/);
    if (match?.groups?.tag) tags.add(match.groups.tag);
  }
  return [...tags].sort((a, b) => a.localeCompare(b, 'en'));
}

function cssHasDisplayContentsRule(cssText: string, tag: string): boolean {
  return cssText
    .split('}')
    .some((block) => block.includes(tag) && /display\s*:\s*contents\s*;?/i.test(block));
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Dashboard Svelte Shell Contract Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- html_shell_tags: ${payload.summary.html_shell_tags}`);
  lines.push(`- source_shell_tags: ${payload.summary.source_shell_tags}`);
  lines.push(`- contract_entries: ${payload.summary.contract_entries}`);
  lines.push(`- transparent_contracts: ${payload.summary.transparent_contracts}`);
  lines.push(`- layout_host_contracts: ${payload.summary.layout_host_contracts}`);
  lines.push(`- interactive_surface_contracts: ${payload.summary.interactive_surface_contracts}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const row of payload.violations) {
    lines.push(`- ${row.type}: ${row.tag || row.contract || row.file || 'unknown'} ${row.detail || ''}`);
  }
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const options = args(argv);
  const config = readJson(options.contractPath);
  const contracts = (config.contracts || {}) as Record<string, Contract>;
  const cssPath = cleanText(config.display_contents_css || '', 400);
  const cssText = readTextMaybe(cssPath);
  const uses = htmlShellUses();
  const htmlTags = new Set(uses.map((item) => item.tag));
  const sourceTags = new Set(sourceShellTags());
  const contractTags = new Set(Object.keys(contracts));
  const violations: any[] = [];

  for (const [tag, contract] of Object.entries(contracts)) {
    if (!VALID_CONTRACTS.has(contract)) {
      violations.push({ type: 'invalid_contract_value', tag, contract });
    }
  }

  for (const tag of new Set([...htmlTags, ...sourceTags])) {
    if (!contractTags.has(tag)) violations.push({ type: 'missing_contract', tag });
  }

  for (const tag of contractTags) {
    if (!htmlTags.has(tag) && !sourceTags.has(tag)) violations.push({ type: 'stale_contract', tag });
  }

  for (const tag of [...htmlTags].sort((a, b) => a.localeCompare(b, 'en'))) {
    const contract = contracts[tag];
    const tagUses = uses.filter((item) => item.tag === tag);
    if (contract === 'transparent') {
      if (!cssHasDisplayContentsRule(cssText, tag)) {
        violations.push({ type: 'transparent_missing_display_contents', tag, detail: cssPath });
      }
      for (const use of tagUses.filter((item) => item.hasClass)) {
        violations.push({ type: 'transparent_shell_has_classed_usage', tag, file: use.file });
      }
    }
    if ((contract === 'layout-host' || contract === 'interactive-surface') && cssHasDisplayContentsRule(cssText, tag)) {
      violations.push({ type: 'host_shell_marked_display_contents', tag, detail: cssPath });
    }
    if (contract === 'interactive-surface') {
      for (const use of tagUses.filter((item) => !item.hasClass)) {
        violations.push({ type: 'interactive_surface_unclassed_usage', tag, file: use.file });
      }
    }
  }

  const contractValues = Object.values(contracts);
  const payload = {
    ok: violations.length === 0 || !options.strict,
    type: 'dashboard_svelte_shell_contract_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: options.strict,
      contract_path: options.contractPath,
      display_contents_css: cssPath,
    },
    summary: {
      html_shell_tags: htmlTags.size,
      source_shell_tags: sourceTags.size,
      contract_entries: contractTags.size,
      transparent_contracts: contractValues.filter((item) => item === 'transparent').length,
      layout_host_contracts: contractValues.filter((item) => item === 'layout-host').length,
      interactive_surface_contracts: contractValues.filter((item) => item === 'interactive-surface').length,
      violations: violations.length,
    },
    violations,
  };

  emitStructuredResult(payload, {
    outPath: options.outJson,
    strict: options.strict,
    ok: payload.ok,
    history: false,
    stdout: false,
  });
  writeTextArtifact(options.outMarkdown, markdown(payload));
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  return payload.ok ? 0 : 1;
}

if (require.main === module) {
  run().then((code) => process.exit(code));
}

module.exports = { run };
