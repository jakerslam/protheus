#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseStrictOutArgs, readFlag, cleanText } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT = 'core/local/artifacts/nexus_import_export_boundary_audit_current.json';
const NEXUS_SRC = path.resolve(ROOT, 'core/layer2/nexus/src');
const FORBIDDEN_IMPORT_MARKERS = ['surface/orchestration', 'infring_orchestration_surface', 'client/runtime/systems'];

function rel(p: string): string {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function listRustFiles(dir: string, out: string[] = []): string[] {
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.resolve(dir, entry.name);
    if (entry.isDirectory()) {
      listRustFiles(abs, out);
      continue;
    }
    if (entry.isFile() && entry.name.endsWith('.rs')) out.push(abs);
  }
  return out;
}

function includesAny(source: string, markers: string[]): string[] {
  return markers.filter((row) => source.includes(row));
}

function main() {
  const common = parseStrictOutArgs(process.argv.slice(2), {});
  const out = cleanText(readFlag(process.argv.slice(2), 'out') || DEFAULT_OUT, 400);

  const files = listRustFiles(NEXUS_SRC).sort();
  const violations: Array<{ file: string; reason: string; markers?: string[] }> = [];

  for (const file of files) {
    const source = fs.readFileSync(file, 'utf8');
    const matchedForbidden = includesAny(source, FORBIDDEN_IMPORT_MARKERS);
    if (matchedForbidden.length > 0) {
      violations.push({
        file: rel(file),
        reason: 'forbidden_import_marker_present',
        markers: matchedForbidden,
      });
    }
  }

  const libPath = path.resolve(NEXUS_SRC, 'lib.rs');
  const libSource = fs.existsSync(libPath) ? fs.readFileSync(libPath, 'utf8') : '';
  const requiredLibModules = ['pub mod policy;', 'pub mod registry;', 'pub mod main_nexus;', 'pub mod sub_nexus;'];
  for (const moduleLine of requiredLibModules) {
    if (!libSource.includes(moduleLine)) {
      violations.push({
        file: rel(libPath),
        reason: 'required_lib_module_missing',
        markers: [moduleLine],
      });
    }
  }

  const payload = {
    type: 'nexus_import_export_boundary_audit',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      scanned_files: files.length,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    forbidden_import_markers: FORBIDDEN_IMPORT_MARKERS,
    violations,
  };

  process.exit(
    emitStructuredResult(payload, {
      outPath: out,
      strict: common.strict,
      ok: payload.summary.pass,
    }),
  );
}

main();
