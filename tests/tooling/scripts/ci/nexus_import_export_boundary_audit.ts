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
const REQUIRED_FORBIDDEN_IMPORT_MARKERS = ['surface/orchestration', 'infring_orchestration_surface', 'client/runtime/systems'];
const EXPECTED_PUBLIC_MODULES = ['conduit_manager', 'main_nexus', 'policy', 'registry', 'route_lease', 'sub_nexus', 'template'];
const REQUIRED_BASELINE_MODULES = ['main_nexus', 'policy', 'registry', 'sub_nexus'];

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

function duplicateValues(values: string[]): string[] {
  const counts = new Map<string, number>();
  for (const value of values) counts.set(value, (counts.get(value) || 0) + 1);
  return [...counts.entries()]
    .filter(([, count]) => count > 1)
    .map(([value]) => value)
    .sort();
}

function isSnakeCaseToken(token: string): boolean {
  return /^[a-z][a-z0-9_]*$/.test(token);
}

function isCanonicalRelativePath(token: string): boolean {
  if (token.trim() !== token) return false;
  if (token.length === 0) return false;
  if (token.includes('\\')) return false;
  if (token.startsWith('/') || token.startsWith('./') || token.startsWith('../')) return false;
  const segments = token.split('/');
  if (segments.some((segment) => segment.length === 0 || segment === '.' || segment === '..')) return false;
  return true;
}

function parsePublicModuleLines(libSource: string): string[] {
  const out: string[] = [];
  for (const rawLine of libSource.split(/\r?\n/)) {
    const line = rawLine.trim();
    const match = /^pub mod ([a-z0-9_]+);$/.exec(line);
    if (match) out.push(match[1]);
  }
  return out;
}

function malformedPublicModuleLines(libSource: string): string[] {
  return libSource
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith('pub mod '))
    .filter((line) => !/^pub mod [a-z0-9_]+;$/.test(line));
}

function modulePathExists(moduleName: string): boolean {
  const flat = path.resolve(NEXUS_SRC, `${moduleName}.rs`);
  const nested = path.resolve(NEXUS_SRC, moduleName, 'mod.rs');
  return fs.existsSync(flat) || fs.existsSync(nested);
}

function main() {
  const common = parseStrictOutArgs(process.argv.slice(2), {});
  const out = cleanText(readFlag(process.argv.slice(2), 'out') || DEFAULT_OUT, 400);

  const policyFailures: Array<{ reason: string; file?: string; markers?: string[] }> = [];

  const nexusRel = rel(NEXUS_SRC);
  if (nexusRel !== 'core/layer2/nexus/src') {
    policyFailures.push({
      reason: 'nexus_source_root_noncanonical',
      markers: [nexusRel],
    });
  }
  if (!fs.existsSync(NEXUS_SRC)) {
    policyFailures.push({
      reason: 'nexus_source_root_missing',
      markers: ['core/layer2/nexus/src'],
    });
  } else if (!fs.statSync(NEXUS_SRC).isDirectory()) {
    policyFailures.push({
      reason: 'nexus_source_root_not_directory',
      markers: ['core/layer2/nexus/src'],
    });
  }

  if (EXPECTED_PUBLIC_MODULES.length === 0) {
    policyFailures.push({
      reason: 'expected_public_modules_empty',
    });
  }
  const duplicateExpectedModules = duplicateValues(EXPECTED_PUBLIC_MODULES);
  if (duplicateExpectedModules.length > 0) {
    policyFailures.push({
      reason: 'expected_public_modules_duplicate',
      markers: duplicateExpectedModules,
    });
  }
  const invalidExpectedModules = EXPECTED_PUBLIC_MODULES.filter((row) => !isSnakeCaseToken(row));
  if (invalidExpectedModules.length > 0) {
    policyFailures.push({
      reason: 'expected_public_modules_non_snake_case',
      markers: invalidExpectedModules.sort(),
    });
  }
  const missingRequiredBaselineModules = REQUIRED_BASELINE_MODULES.filter((row) => !EXPECTED_PUBLIC_MODULES.includes(row));
  if (missingRequiredBaselineModules.length > 0) {
    policyFailures.push({
      reason: 'required_baseline_modules_missing',
      markers: missingRequiredBaselineModules.sort(),
    });
  }

  if (FORBIDDEN_IMPORT_MARKERS.length === 0) {
    policyFailures.push({
      reason: 'forbidden_import_markers_empty',
    });
  }
  const duplicateForbiddenMarkers = duplicateValues(FORBIDDEN_IMPORT_MARKERS);
  if (duplicateForbiddenMarkers.length > 0) {
    policyFailures.push({
      reason: 'forbidden_import_markers_duplicate',
      markers: duplicateForbiddenMarkers,
    });
  }
  const malformedForbiddenMarkers = FORBIDDEN_IMPORT_MARKERS.filter(
    (row) => row.trim() !== row || row.length === 0 || row.includes('\\') || row.startsWith('/') || row.includes('..'),
  );
  if (malformedForbiddenMarkers.length > 0) {
    policyFailures.push({
      reason: 'forbidden_import_markers_malformed',
      markers: malformedForbiddenMarkers.sort(),
    });
  }
  const missingRequiredForbiddenMarkers = REQUIRED_FORBIDDEN_IMPORT_MARKERS.filter((row) => !FORBIDDEN_IMPORT_MARKERS.includes(row));
  if (missingRequiredForbiddenMarkers.length > 0) {
    policyFailures.push({
      reason: 'required_forbidden_markers_missing',
      markers: missingRequiredForbiddenMarkers.sort(),
    });
  }

  const files = listRustFiles(NEXUS_SRC).sort();
  if (files.length === 0) {
    policyFailures.push({
      reason: 'nexus_rust_files_missing',
      file: rel(NEXUS_SRC),
    });
  }
  const fileRelPaths = files.map((file) => rel(file));
  const duplicateFileRelPaths = duplicateValues(fileRelPaths);
  if (duplicateFileRelPaths.length > 0) {
    policyFailures.push({
      reason: 'nexus_rust_file_relpath_duplicate',
      markers: duplicateFileRelPaths,
    });
  }
  const nonCanonicalFileRelPaths = fileRelPaths.filter((row) => !isCanonicalRelativePath(row));
  if (nonCanonicalFileRelPaths.length > 0) {
    policyFailures.push({
      reason: 'nexus_rust_file_relpath_noncanonical',
      markers: nonCanonicalFileRelPaths.sort(),
    });
  }
  const outOfRootFiles = files.filter((file) => !file.startsWith(`${NEXUS_SRC}${path.sep}`) && file !== path.resolve(NEXUS_SRC, 'lib.rs'));
  if (outOfRootFiles.length > 0) {
    policyFailures.push({
      reason: 'nexus_rust_file_outside_root',
      markers: outOfRootFiles.map((row) => rel(row)).sort(),
    });
  }
  const nonRustFilesInRustInventory = files.filter((file) => !file.endsWith('.rs'));
  if (nonRustFilesInRustInventory.length > 0) {
    policyFailures.push({
      reason: 'nexus_inventory_non_rust_file',
      markers: nonRustFilesInRustInventory.map((row) => rel(row)).sort(),
    });
  }

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
  if (!fs.existsSync(libPath)) {
    policyFailures.push({
      reason: 'nexus_lib_missing',
      file: rel(libPath),
    });
  }
  if (!fileRelPaths.includes(rel(libPath))) {
    policyFailures.push({
      reason: 'nexus_lib_not_in_scan_inventory',
      file: rel(libPath),
    });
  }

  const libSource = fs.existsSync(libPath) ? fs.readFileSync(libPath, 'utf8') : '';
  const requiredLibModules = EXPECTED_PUBLIC_MODULES.map((row) => `pub mod ${row};`);
  const duplicateRequiredLibModules = duplicateValues(requiredLibModules);
  if (duplicateRequiredLibModules.length > 0) {
    policyFailures.push({
      reason: 'required_lib_modules_duplicate',
      markers: duplicateRequiredLibModules,
    });
  }
  const invalidRequiredLibModules = requiredLibModules.filter((row) => !/^pub mod [a-z0-9_]+;$/.test(row));
  if (invalidRequiredLibModules.length > 0) {
    policyFailures.push({
      reason: 'required_lib_modules_malformed',
      markers: invalidRequiredLibModules.sort(),
    });
  }

  for (const moduleLine of requiredLibModules) {
    const occurrences = libSource
      .split(/\r?\n/)
      .map((row) => row.trim())
      .filter((row) => row === moduleLine).length;
    if (occurrences === 0) {
      violations.push({
        file: rel(libPath),
        reason: 'required_lib_module_missing',
        markers: [moduleLine],
      });
    } else if (occurrences > 1) {
      policyFailures.push({
        file: rel(libPath),
        reason: 'required_lib_module_duplicate_declaration',
        markers: [moduleLine, String(occurrences)],
      });
    }
  }

  const malformedPubModRows = malformedPublicModuleLines(libSource);
  if (malformedPubModRows.length > 0) {
    policyFailures.push({
      file: rel(libPath),
      reason: 'lib_public_module_declaration_malformed',
      markers: malformedPubModRows,
    });
  }
  const declaredPublicModules = parsePublicModuleLines(libSource);
  if (declaredPublicModules.length === 0) {
    policyFailures.push({
      file: rel(libPath),
      reason: 'lib_public_module_declarations_missing',
    });
  }
  const duplicateDeclaredPublicModules = duplicateValues(declaredPublicModules);
  if (duplicateDeclaredPublicModules.length > 0) {
    policyFailures.push({
      file: rel(libPath),
      reason: 'lib_public_module_duplicate',
      markers: duplicateDeclaredPublicModules,
    });
  }
  const missingDeclaredPublicModules = EXPECTED_PUBLIC_MODULES.filter((row) => !declaredPublicModules.includes(row));
  if (missingDeclaredPublicModules.length > 0) {
    policyFailures.push({
      file: rel(libPath),
      reason: 'lib_public_module_expected_missing',
      markers: missingDeclaredPublicModules.sort(),
    });
  }
  const unexpectedDeclaredPublicModules = declaredPublicModules.filter((row) => !EXPECTED_PUBLIC_MODULES.includes(row));
  if (unexpectedDeclaredPublicModules.length > 0) {
    policyFailures.push({
      file: rel(libPath),
      reason: 'lib_public_module_unexpected',
      markers: [...new Set(unexpectedDeclaredPublicModules)].sort(),
    });
  }
  const declaredWithoutDupes = [...new Set(declaredPublicModules)];
  if (declaredWithoutDupes.join('|') !== EXPECTED_PUBLIC_MODULES.join('|')) {
    policyFailures.push({
      file: rel(libPath),
      reason: 'lib_public_module_order_drift',
      markers: declaredWithoutDupes,
    });
  }
  const missingModuleFiles = EXPECTED_PUBLIC_MODULES.filter((row) => !modulePathExists(row));
  if (missingModuleFiles.length > 0) {
    policyFailures.push({
      reason: 'expected_module_file_missing',
      markers: missingModuleFiles.sort(),
    });
  }

  const policyFailureCount = policyFailures.length;
  const violationCount = violations.length;
  const totalIssueCount = policyFailureCount + violationCount;
  const payload = {
    type: 'nexus_import_export_boundary_audit',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      scanned_files: files.length,
      violation_count: violationCount,
      policy_failure_count: policyFailureCount,
      total_issue_count: totalIssueCount,
      pass: totalIssueCount === 0,
    },
    forbidden_import_markers: FORBIDDEN_IMPORT_MARKERS,
    expected_public_modules: EXPECTED_PUBLIC_MODULES,
    policy_failures: policyFailures,
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
