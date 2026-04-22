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
  mapping_files?: Record<string, string>;
  scan_files?: string[];
  required_command_aliases?: AliasPair[];
  required_artifact_aliases?: string[];
  legacy_token_enforcement_prefixes?: string[];
  legacy_token_exemptions?: string[];
  doc_alias_terms?: string[];
  doc_alias_context_regex?: string;
  temporary_compatibility_bridge_exemptions?: Array<{
    id?: string;
    bridge?: string;
    owner?: string;
    reason?: string;
    status?: string;
    expires_at?: string;
  }>;
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
  const regex = /\b(core_[a-z0-9_]+|adapter_[a-z0-9_]+|client_[a-z0-9_]+)\b/gi;
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
  if (loweredTerm === 'client') {
    return /\bclient\/(?:\*\*|[a-z0-9_\-./]*)/i.test(line);
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
  lines.push(`- doc_primary_authority_term_failures: ${report.summary.doc_primary_authority_term_failures}`);
  lines.push(`- unmapped_legacy_token_count: ${report.summary.unmapped_legacy_token_count}`);
  lines.push(`- temporary_bridge_exemption_count: ${report.summary.temporary_bridge_exemption_count}`);
  lines.push(`- temporary_bridge_exemption_expired_count: ${report.summary.temporary_bridge_exemption_expired_count}`);
  lines.push(`- temporary_bridge_exemption_invalid_count: ${report.summary.temporary_bridge_exemption_invalid_count}`);
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
  lines.push('## Doc Primary-Authority Term Failures');
  if (!(report.doc_primary_authority_term_failures || []).length) {
    lines.push('- none');
  } else {
    for (const row of report.doc_primary_authority_term_failures) {
      lines.push(`- ${row.file}:${row.line} term=${row.term} (${row.reason})`);
    }
  }
  lines.push('');
  lines.push('## Temporary Compatibility Bridge Exemptions');
  lines.push('| id | bridge | owner | status | expires_at | expired | valid |');
  lines.push('| --- | --- | --- | --- | --- | --- | --- |');
  for (const row of report.temporary_compatibility_bridge_exemptions || []) {
    lines.push(
      `| ${row.id} | ${row.bridge} | ${row.owner} | ${row.status} | ${row.expires_at} | ${row.expired} | ${row.valid} |`,
    );
  }
  if (!(report.temporary_compatibility_bridge_exemptions || []).length) {
    lines.push('| (none) | - | - | - | - | false | true |');
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

  const mappingFiles = tracker.mapping_files && typeof tracker.mapping_files === 'object'
    ? tracker.mapping_files
    : {};

  const mappingMissing: string[] = [];
  const maps: AliasMap[] = [];
  for (const [key, relPath] of Object.entries(mappingFiles)) {
    const p = path.resolve(ROOT, String(relPath || ''));
    if (!p || !fs.existsSync(p)) {
      mappingMissing.push(rel(p || String(relPath || key)));
      continue;
    }
    maps.push(readJson<AliasMap>(p));
  }

  const mappedCommandAliases = new Set<string>();
  const mappedArtifactAliases = new Set<string>();
  const mappedArtifactAliasesLower = new Set<string>();
  for (const map of maps) {
    for (const row of map.command_aliases || []) {
      mappedCommandAliases.add(String(row.alias || '').trim());
    }
    for (const row of map.artifact_aliases || []) {
      const alias = String(row.alias || '').trim();
      mappedArtifactAliases.add(alias);
      mappedArtifactAliasesLower.add(alias.toLowerCase());
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
  const legacyTokenEnforcementPrefixes =
    Array.isArray(tracker.legacy_token_enforcement_prefixes) && tracker.legacy_token_enforcement_prefixes.length > 0
      ? tracker.legacy_token_enforcement_prefixes
          .map((value) => String(value || '').trim().toLowerCase())
          .filter(Boolean)
      : ['core_', 'adapter_'];
  const legacyTokenExemptions = new Set(
    (Array.isArray(tracker.legacy_token_exemptions) ? tracker.legacy_token_exemptions : [])
      .map((value) => String(value || '').trim().toLowerCase())
      .filter(Boolean),
  );
  const docAliasTerms = (tracker.doc_alias_terms || []).map((v) => String(v).trim()).filter(Boolean);
  const docContextRegex = safeRegex(String(tracker.doc_alias_context_regex || ''), 'i') || /compat(?:ibility)?\s+alias/i;

  const legacyTokens: Array<{ token: string; file: string; mapped: boolean }> = [];
  const docAliasContextFailures: Array<{ file: string; line: number; term: string; reason: string }> = [];
  const docPrimaryAuthorityTermFailures: Array<{
    file: string;
    line: number;
    term: string;
    reason: string;
  }> = [];

  for (const file of scanFiles) {
    const abs = path.resolve(ROOT, file);
    if (!fs.existsSync(abs)) continue;
    const source = fs.readFileSync(abs, 'utf8');

    for (const token of extractLegacyTokens(source)) {
      const mapped = mappedArtifactAliases.has(token) || mappedArtifactAliasesLower.has(token.toLowerCase());
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
        const coreAsPrimaryAuthority =
          /\bcore\b/i.test(line) &&
          (lowered.includes('authority') ||
            lowered.includes('canonical truth') ||
            lowered.includes('canonical term') ||
            lowered.includes('authoritative')) &&
          !lowered.includes('must not present') &&
          !lowered.includes('never as the primary authority term') &&
          !lowered.includes('not a standalone primary authority') &&
          !isImmutablePathAliasContext('core', line) &&
          !lowered.includes('compat alias') &&
          !lowered.includes('compatibility alias') &&
          !docContextRegex.test(line);
        if (coreAsPrimaryAuthority) {
          docPrimaryAuthorityTermFailures.push({
            file,
            line: idx + 1,
            term: 'Core',
            reason: 'core_presented_as_primary_authority_term',
          });
        }
      }
    }
  }

  const unmappedLegacyTokens = legacyTokens.filter((row) => {
    if (row.mapped) return false;
    const tokenLower = String(row.token || '').toLowerCase();
    if (legacyTokenExemptions.has(tokenLower)) return false;
    return legacyTokenEnforcementPrefixes.some((prefix) => tokenLower.startsWith(prefix));
  });
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
  for (const row of docPrimaryAuthorityTermFailures) {
    violations.push({
      type: 'doc_primary_authority_term_failure',
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
      doc_primary_authority_term_failures: docPrimaryAuthorityTermFailures.length,
      unmapped_legacy_token_count: unmappedLegacyTokens.length,
      temporary_bridge_exemption_count: 0,
      temporary_bridge_exemption_expired_count: 0,
      temporary_bridge_exemption_invalid_count: 0,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    command_alias_pairs: commandAliasPairs,
    legacy_tokens: legacyTokens,
    doc_alias_context_failures: docAliasContextFailures,
    doc_primary_authority_term_failures: docPrimaryAuthorityTermFailures,
    violations,
    temporary_compatibility_bridge_exemptions: [] as Array<{
      id: string;
      bridge: string;
      owner: string;
      reason: string;
      status: string;
      expires_at: string;
      expired: boolean;
      valid: boolean;
    }>,
  };

  const nowEpoch = Date.now();
  const temporaryExemptions = Array.isArray(tracker.temporary_compatibility_bridge_exemptions)
    ? tracker.temporary_compatibility_bridge_exemptions
    : [];
  const normalizedTemporaryExemptions = temporaryExemptions.map((row) => {
    const id = cleanText(String(row.id || ''), 160);
    const bridge = cleanText(String(row.bridge || ''), 200);
    const owner = cleanText(String(row.owner || ''), 120);
    const reason = cleanText(String(row.reason || ''), 260);
    const status = cleanText(String(row.status || 'active'), 40).toLowerCase();
    const expiresAt = cleanText(String(row.expires_at || ''), 40);
    const expiresEpoch = Number.isFinite(Date.parse(expiresAt)) ? Date.parse(expiresAt) : Number.NaN;
    const valid =
      !!id &&
      !!bridge &&
      !!owner &&
      !!reason &&
      !!expiresAt &&
      Number.isFinite(expiresEpoch) &&
      (status === 'active' || status === 'retired');
    const expired = status === 'active' && Number.isFinite(expiresEpoch) && nowEpoch > expiresEpoch;
    return {
      id,
      bridge,
      owner,
      reason,
      status: status || 'active',
      expires_at: expiresAt,
      expired,
      valid,
    };
  });
  for (const row of normalizedTemporaryExemptions) {
    if (!row.valid) {
      report.violations.push({
        type: 'invalid_temporary_compatibility_bridge_exemption',
        detail: row.id || row.bridge || 'unknown',
      });
    }
    if (row.expired) {
      report.violations.push({
        type: 'expired_temporary_compatibility_bridge_exemption',
        detail: `${row.id}:${row.expires_at}`,
      });
    }
  }
  report.temporary_compatibility_bridge_exemptions = normalizedTemporaryExemptions;
  report.summary.temporary_bridge_exemption_count = normalizedTemporaryExemptions.length;
  report.summary.temporary_bridge_exemption_expired_count = normalizedTemporaryExemptions.filter(
    (row) => row.expired,
  ).length;
  report.summary.temporary_bridge_exemption_invalid_count = normalizedTemporaryExemptions.filter(
    (row) => !row.valid,
  ).length;
  report.summary.violation_count = report.violations.length;
  report.summary.pass = report.violations.length === 0;

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
