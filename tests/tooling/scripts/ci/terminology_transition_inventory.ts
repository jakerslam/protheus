#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type AliasPair = {
  canonical: string;
  alias: string;
};

type AliasMap = {
  command_aliases?: AliasPair[];
  artifact_aliases?: AliasPair[];
};

type Tracker = {
  retirement_target?: {
    version?: string;
    date?: string;
  };
  mapping_files?: {
    kernel?: string;
    gateway?: string;
  };
  scan_files?: string[];
  required_command_aliases?: AliasPair[];
  required_artifact_aliases?: string[];
  doc_alias_terms?: string[];
  doc_alias_context_regex?: string;
};

const ROOT = process.cwd();
const DEFAULT_TRACKER = 'client/runtime/config/terminology_transition_deprecation_tracker.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/terminology_transition_inventory_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/TERMINOLOGY_TRANSITION_INVENTORY_CURRENT.md';

function rel(value: string): string {
  return path.relative(ROOT, value).replace(/\\/g, '/');
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    tracker: cleanText(readFlag(argv, 'tracker') || DEFAULT_TRACKER, 400),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 400),
  };
}

function readJson<T>(file: string): T {
  return JSON.parse(fs.readFileSync(file, 'utf8')) as T;
}

function safeRegex(pattern: string, flags = 'g'): RegExp | null {
  try {
    return new RegExp(pattern, flags);
  } catch {
    return null;
  }
}

function extractLegacyTokens(source: string): string[] {
  const tokens = new Set<string>();
  const regex = /\b(core_[a-z0-9_]+|adapter_[a-z0-9_]+)\b/gi;
  for (const match of source.matchAll(regex)) {
    tokens.add(String(match[1] || '').trim());
  }
  return [...tokens].sort((a, b) => a.localeCompare(b, 'en'));
}

function isImmutablePathAliasContext(term: string, line: string): boolean {
  const loweredTerm = term.toLowerCase();
  if (loweredTerm === 'core') {
    return /\bcore\/(?:\*\*|[a-z0-9_\-./]*)/i.test(line);
  }
  if (loweredTerm === 'adapters') {
    return /\badapters\/(?:\*\*|[a-z0-9_\-./]*)/i.test(line);
  }
  return false;
}

function markdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Terminology Transition Inventory');
  lines.push('');
  lines.push(`- generated_at: ${report.generated_at}`);
  lines.push(`- revision: ${report.revision}`);
  lines.push(`- strict: ${report.strict}`);
  lines.push(`- pass: ${report.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- mapping_files_missing: ${report.summary.mapping_files_missing}`);
  lines.push(`- command_alias_pair_failures: ${report.summary.command_alias_pair_failures}`);
  lines.push(`- artifact_alias_failures: ${report.summary.artifact_alias_failures}`);
  lines.push(`- doc_alias_context_failures: ${report.summary.doc_alias_context_failures}`);
  lines.push(`- unmapped_legacy_token_count: ${report.summary.unmapped_legacy_token_count}`);
  lines.push('');
  lines.push('## Command Alias Pair Status');
  lines.push('| canonical | alias | canonical_present | alias_present | ok |');
  lines.push('| --- | --- | --- | --- | --- |');
  for (const row of report.command_alias_pairs || []) {
    lines.push(`| ${row.canonical} | ${row.alias} | ${row.canonical_present} | ${row.alias_present} | ${row.ok} |`);
  }
  if (!(report.command_alias_pairs || []).length) lines.push('| (none) | - | - | - | true |');
  lines.push('');
  lines.push('## Legacy Tokens (core_/adapter_)');
  lines.push('| token | file | mapped |');
  lines.push('| --- | --- | --- |');
  for (const row of (report.legacy_tokens || []).slice(0, 160)) {
    lines.push(`| ${row.token} | ${row.file} | ${row.mapped} |`);
  }
  if (!(report.legacy_tokens || []).length) lines.push('| (none) | - | true |');
  lines.push('');
  lines.push('## Doc Alias Context Failures');
  if (!(report.doc_alias_context_failures || []).length) {
    lines.push('- none');
  } else {
    for (const row of report.doc_alias_context_failures) {
      lines.push(`- ${row.file}:${row.line} term=${row.term} (${row.reason})`);
    }
  }
  lines.push('');
  lines.push('## Violations');
  if (!(report.violations || []).length) {
    lines.push('- none');
  } else {
    for (const row of report.violations) {
      lines.push(`- ${row.type}: ${row.detail}`);
    }
  }
  return `${lines.join('\n')}\n`;
}

function run() {
  const args = parseArgs(process.argv.slice(2));
  const trackerAbs = path.resolve(ROOT, args.tracker);
  const tracker = readJson<Tracker>(trackerAbs);

  const mappingPaths = {
    kernel: path.resolve(ROOT, String(tracker.mapping_files?.kernel || '')),
    gateway: path.resolve(ROOT, String(tracker.mapping_files?.gateway || '')),
  };

  const mappingMissing: string[] = [];
  const maps: AliasMap[] = [];
  for (const key of ['kernel', 'gateway'] as const) {
    const p = mappingPaths[key];
    if (!p || !fs.existsSync(p)) {
      mappingMissing.push(rel(p || String(tracker.mapping_files?.[key] || key)));
      continue;
    }
    maps.push(readJson<AliasMap>(p));
  }

  const mappedCommandAliases = new Set<string>();
  const mappedArtifactAliases = new Set<string>();
  for (const map of maps) {
    for (const row of map.command_aliases || []) {
      mappedCommandAliases.add(String(row.alias || '').trim());
    }
    for (const row of map.artifact_aliases || []) {
      mappedArtifactAliases.add(String(row.alias || '').trim());
    }
  }

  const packagePath = path.resolve(ROOT, 'package.json');
  const packageJson = readJson<any>(packagePath);
  const scriptKeys = Object.keys(packageJson.scripts || {});

  const commandAliasPairs = (tracker.required_command_aliases || []).map((row) => {
    const canonical = String(row.canonical || '').trim();
    const alias = String(row.alias || '').trim();
    const canonicalPresent = scriptKeys.includes(canonical);
    const aliasPresent = scriptKeys.includes(alias);
    return {
      canonical,
      alias,
      canonical_present: canonicalPresent,
      alias_present: aliasPresent,
      ok: canonicalPresent && aliasPresent,
    };
  });

  const registryPath = path.resolve(ROOT, 'tests/tooling/config/tooling_gate_registry.json');
  const registryJson = readJson<any>(registryPath);
  const artifactPaths = new Set<string>();
  for (const gate of Object.values<any>(registryJson.gates || {})) {
    const arr = Array.isArray(gate?.artifact_paths) ? gate.artifact_paths : [];
    for (const item of arr) artifactPaths.add(String(item));
  }

  const requiredArtifactAliases = (tracker.required_artifact_aliases || []).map((raw) => String(raw || '').trim()).filter(Boolean);
  const artifactAliasFailures = requiredArtifactAliases
    .filter((aliasStem) => {
      for (const artifactPath of artifactPaths) {
        if (path.basename(String(artifactPath)) === aliasStem) return false;
      }
      return true;
    })
    .sort((a, b) => a.localeCompare(b, 'en'));

  const scanFiles = (tracker.scan_files || []).map((v) => String(v).replace(/\\/g, '/')).filter(Boolean);
  const docAliasTerms = (tracker.doc_alias_terms || []).map((v) => String(v).trim()).filter(Boolean);
  const docContextRegex = safeRegex(String(tracker.doc_alias_context_regex || ''), 'i') || /compat(?:ibility)?\s+alias/i;

  const legacyTokens: Array<{ token: string; file: string; mapped: boolean }> = [];
  const docAliasContextFailures: Array<{ file: string; line: number; term: string; reason: string }> = [];

  for (const file of scanFiles) {
    const abs = path.resolve(ROOT, file);
    if (!fs.existsSync(abs)) continue;
    const source = fs.readFileSync(abs, 'utf8');

    for (const token of extractLegacyTokens(source)) {
      const mapped = mappedArtifactAliases.has(token);
      legacyTokens.push({ token, file, mapped });
    }

    if (/\.md$/i.test(file)) {
      const lines = source.split(/\r?\n/);
      for (let idx = 0; idx < lines.length; idx += 1) {
        const line = lines[idx];
        const lowered = line.toLowerCase();
        for (const term of docAliasTerms) {
          const termRegex = safeRegex(`\\b${term}\\b`);
          if (!termRegex || !termRegex.test(line)) continue;
          if (isImmutablePathAliasContext(term, line)) continue;
          if (lowered.includes('compat alias') || lowered.includes('compatibility alias')) continue;
          if (docContextRegex.test(line)) continue;
          docAliasContextFailures.push({
            file,
            line: idx + 1,
            term,
            reason: 'missing_compatibility_alias_context',
          });
        }
      }
    }
  }

  const unmappedLegacyTokens = legacyTokens.filter((row) => !row.mapped);
  const violations: Array<{ type: string; detail: string }> = [];

  for (const missing of mappingMissing) {
    violations.push({ type: 'missing_mapping_file', detail: missing });
  }

  for (const pair of commandAliasPairs) {
    if (!pair.ok) {
      violations.push({
        type: 'missing_command_alias_pair',
        detail: `${pair.canonical} <-> ${pair.alias}`,
      });
    }
    if (pair.alias && mappedCommandAliases.size && !mappedCommandAliases.has(pair.alias)) {
      violations.push({
        type: 'unmapped_command_alias',
        detail: pair.alias,
      });
    }
  }

  for (const alias of artifactAliasFailures) {
    violations.push({ type: 'missing_artifact_alias', detail: alias });
  }

  for (const row of unmappedLegacyTokens) {
    violations.push({
      type: 'unmapped_legacy_token',
      detail: `${row.token} @ ${row.file}`,
    });
  }

  for (const row of docAliasContextFailures) {
    violations.push({
      type: 'doc_alias_context_failure',
      detail: `${row.term} @ ${row.file}:${row.line}`,
    });
  }

  const retirementVersion = String(tracker.retirement_target?.version || '').trim();
  const retirementDate = String(tracker.retirement_target?.date || '').trim();
  if (!retirementVersion || !retirementDate) {
    violations.push({
      type: 'missing_retirement_target',
      detail: 'terminology_transition_deprecation_tracker.retirement_target',
    });
  }

  const report = {
    type: 'terminology_transition_inventory',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    tracker_path: rel(trackerAbs),
    summary: {
      mapping_files_missing: mappingMissing.length,
      command_alias_pair_failures: commandAliasPairs.filter((row) => !row.ok).length,
      artifact_alias_failures: artifactAliasFailures.length,
      doc_alias_context_failures: docAliasContextFailures.length,
      unmapped_legacy_token_count: unmappedLegacyTokens.length,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    command_alias_pairs: commandAliasPairs,
    legacy_tokens: legacyTokens,
    doc_alias_context_failures: docAliasContextFailures,
    violations,
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdown), markdown({ ...report, ok: report.summary.pass }));

  process.exit(
    emitStructuredResult(report, {
      outPath: path.resolve(ROOT, args.outJson),
      strict: args.strict,
      ok: report.summary.pass,
    }),
  );
}

run();
