#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type AliasPair = {
  canonical: string;
  compatibility: string;
};

type AliasManifest = {
  schema_id: string;
  schema_version: string;
  canonical_term: string;
  compatibility_alias: string;
  retirement_target_version: string;
  retirement_target_date: string;
  required_docs_paths: string[];
  required_notes_markers: string[];
  required_command_alias_pairs: AliasPair[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/shell_transition_tracker_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 500),
    outMarkdown: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/SHELL_TRANSITION_TRACKER_CURRENT.md',
      500,
    ),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'client/runtime/config/shell_transition_alias_manifest.json',
      500,
    ),
    notesPath: cleanText(readFlag(argv, 'notes') || 'docs/workspace/shell_transition_notes.md', 500),
    packagePath: cleanText(readFlag(argv, 'package') || 'package.json', 500),
  };
}

function readJson<T>(root: string, relPath: string): { ok: boolean; payload: T | null; detail: string } {
  const abs = path.resolve(root, relPath);
  try {
    return {
      ok: true,
      payload: JSON.parse(fs.readFileSync(abs, 'utf8')) as T,
      detail: 'loaded',
    };
  } catch (error) {
    return {
      ok: false,
      payload: null,
      detail: cleanText((error as Error)?.message || 'json_unavailable', 240),
    };
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Transition Tracker');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- docs_checked: ${payload.summary.docs_checked}`);
  lines.push(`- markers_checked: ${payload.summary.markers_checked}`);
  lines.push(`- command_pairs_checked: ${payload.summary.command_pairs_checked}`);
  lines.push(`- failures: ${payload.summary.failures}`);
  lines.push('');
  lines.push('## Failures');
  if (!Array.isArray(payload.failures) || payload.failures.length === 0) {
    lines.push('- none');
  } else {
    for (const row of payload.failures) {
      lines.push(`- ${cleanText(row.id || '', 120)}: ${cleanText(row.detail || '', 240)}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const manifestJson = readJson<AliasManifest>(root, args.manifestPath);
  if (!manifestJson.ok || !manifestJson.payload) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'shell_transition_tracker',
        error: 'shell_transition_manifest_unavailable',
        detail: manifestJson.detail,
        manifest_path: args.manifestPath,
      },
      { outPath: args.outPath, strict: args.strict, ok: false },
    );
  }

  const packageJson = readJson<{ scripts?: Record<string, string> }>(root, args.packagePath);
  if (!packageJson.ok || !packageJson.payload) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'shell_transition_tracker',
        error: 'package_json_unavailable',
        detail: packageJson.detail,
        package_path: args.packagePath,
      },
      { outPath: args.outPath, strict: args.strict, ok: false },
    );
  }

  const manifest = manifestJson.payload;
  const scripts = packageJson.payload.scripts || {};
  const failures: Array<{ id: string; detail: string }> = [];

  const notesAbs = path.resolve(root, args.notesPath);
  const notesExists = fs.existsSync(notesAbs);
  const notesSource = notesExists ? fs.readFileSync(notesAbs, 'utf8') : '';
  if (!notesExists) {
    failures.push({ id: 'shell_transition_notes_missing', detail: args.notesPath });
  }

  const docs = Array.isArray(manifest.required_docs_paths) ? manifest.required_docs_paths : [];
  for (const docPath of docs) {
    const relDoc = cleanText(docPath || '', 500);
    if (!relDoc) continue;
    if (!fs.existsSync(path.resolve(root, relDoc))) {
      failures.push({ id: 'shell_transition_required_doc_missing', detail: relDoc });
    }
  }

  const markers = Array.isArray(manifest.required_notes_markers) ? manifest.required_notes_markers : [];
  for (const marker of markers) {
    const token = cleanText(marker || '', 200);
    if (!token) continue;
    if (!notesSource.includes(token)) {
      failures.push({ id: 'shell_transition_notes_marker_missing', detail: token });
    }
  }

  const aliasPairs = Array.isArray(manifest.required_command_alias_pairs)
    ? manifest.required_command_alias_pairs
    : [];
  for (const pair of aliasPairs) {
    const canonical = cleanText(pair?.canonical || '', 160);
    const compatibility = cleanText(pair?.compatibility || '', 160);
    if (!canonical || !compatibility) {
      failures.push({
        id: 'shell_transition_alias_pair_invalid',
        detail: `${canonical || 'missing_canonical'}:${compatibility || 'missing_compatibility'}`,
      });
      continue;
    }
    const canonicalCommand = cleanText(scripts[canonical] || '', 2000);
    const compatibilityCommand = cleanText(scripts[compatibility] || '', 2000);
    if (!canonicalCommand) {
      failures.push({ id: 'shell_transition_canonical_command_missing', detail: canonical });
    }
    if (!compatibilityCommand) {
      failures.push({ id: 'shell_transition_compat_command_missing', detail: compatibility });
    }
    if (canonicalCommand && compatibilityCommand && !compatibilityCommand.includes(canonical)) {
      failures.push({
        id: 'shell_transition_compatibility_alias_not_linked',
        detail: `${compatibility} does not reference ${canonical}`,
      });
    }
  }

  if (!cleanText(manifest.canonical_term || '', 80).toLowerCase().includes('shell')) {
    failures.push({ id: 'shell_transition_manifest_canonical_term_invalid', detail: manifest.canonical_term || '' });
  }
  if (!cleanText(manifest.compatibility_alias || '', 80).toLowerCase().includes('client')) {
    failures.push({
      id: 'shell_transition_manifest_compat_alias_invalid',
      detail: manifest.compatibility_alias || '',
    });
  }
  if (!Number.isFinite(Date.parse(cleanText(manifest.retirement_target_date || '', 40)))) {
    failures.push({
      id: 'shell_transition_manifest_retirement_date_invalid',
      detail: manifest.retirement_target_date || '',
    });
  }

  const payload = {
    ok: failures.length === 0,
    type: 'shell_transition_tracker',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    strict: args.strict,
    manifest_path: args.manifestPath,
    notes_path: args.notesPath,
    package_path: args.packagePath,
    summary: {
      docs_checked: docs.length,
      markers_checked: markers.length,
      command_pairs_checked: aliasPairs.length,
      failures: failures.length,
    },
    failures,
  };

  writeTextArtifact(args.outMarkdown, markdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
