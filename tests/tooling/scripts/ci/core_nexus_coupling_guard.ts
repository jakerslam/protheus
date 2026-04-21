#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision, trackedFiles } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY_PATH = 'tests/tooling/config/core_nexus_coupling_policy.json';
const DEFAULT_OUT = 'core/local/artifacts/core_nexus_coupling_guard_current.json';

type ImportEdgeExemption = {
  from_package: string;
  to_crate: string;
  reason?: string;
  expires?: string;
};

type CargoPathEdgeExemption = {
  from_manifest: string;
  to_path: string;
  reason?: string;
  expires?: string;
};

type CouplingPolicy = {
  version?: string;
  scope?: {
    rust_scan_roots?: string[];
    cargo_scan_roots?: string[];
  };
  nexus_crates?: string[];
  allowed_foundation_crates?: string[];
  fail_on_expired_exemptions?: boolean;
  exemptions?: {
    import_edges?: ImportEdgeExemption[];
    cargo_path_edges?: CargoPathEdgeExemption[];
  };
};

type Args = {
  strict: boolean;
  out: string;
  policy: string;
};

type CoreManifest = {
  manifest: string;
  package_name: string;
  crate_name: string;
  dir: string;
};

type ImportViolation = {
  file: string;
  line: number;
  from_package: string;
  to_crate: string;
  rule: string;
};

type CargoPathViolation = {
  manifest: string;
  line: number;
  from_package: string;
  to_crate: string;
  to_path: string;
  rule: string;
};

type ExemptionDrift = {
  kind: 'import_edge' | 'cargo_path_edge';
  key: string;
  reason: string;
};

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/');
}

function normalizeCrateName(value: string): string {
  return value.trim().replace(/-/g, '_');
}

function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10);
}

function isExpired(expires: string | undefined): boolean {
  if (!expires) return false;
  const date = cleanText(expires, 20);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return false;
  return date < todayIsoDate();
}

function parseArgs(argv: string[]): Args {
  const strictOut = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT,
  });
  return {
    strict: strictOut.strict,
    out: cleanText(readFlag(argv, 'out') || strictOut.out || DEFAULT_OUT, 500),
    policy: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 500),
  };
}

function loadPolicy(policyPath: string): CouplingPolicy {
  const abs = path.resolve(ROOT, policyPath);
  return JSON.parse(fs.readFileSync(abs, 'utf8')) as CouplingPolicy;
}

function isUnderAnyRoot(file: string, roots: string[]): boolean {
  if (roots.length === 0) return true;
  return roots.some((root) => file === root || file.startsWith(`${root}/`));
}

function listCoreManifests(coreTrackedFiles: string[]): CoreManifest[] {
  const manifests: CoreManifest[] = [];
  for (const file of coreTrackedFiles) {
    if (!file.startsWith('core/')) continue;
    if (!file.endsWith('/Cargo.toml')) continue;
    let source = '';
    try {
      source = fs.readFileSync(path.resolve(ROOT, file), 'utf8');
    } catch {
      continue;
    }
    const match = source.match(/^name\s*=\s*"([^"]+)"/m);
    if (!match) continue;
    const packageName = cleanText(match[1], 200);
    manifests.push({
      manifest: file,
      package_name: packageName,
      crate_name: normalizeCrateName(packageName),
      dir: normalizePath(path.dirname(file)),
    });
  }
  manifests.sort((a, b) => b.dir.length - a.dir.length);
  return manifests;
}

function nearestManifest(file: string, manifests: CoreManifest[]): CoreManifest | null {
  const normalized = normalizePath(file);
  for (const manifest of manifests) {
    if (normalized === manifest.dir || normalized.startsWith(`${manifest.dir}/`)) {
      return manifest;
    }
  }
  return null;
}

function importEdgeKey(fromPackageCrate: string, toCrate: string): string {
  return `${fromPackageCrate}->${toCrate}`;
}

function cargoPathEdgeKey(fromManifest: string, toPath: string): string {
  return `${normalizePath(fromManifest)}->${normalizePath(toPath)}`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const policy = loadPolicy(args.policy);

  const rustRoots = (policy.scope?.rust_scan_roots || []).map((v) => normalizePath(cleanText(v, 300)));
  const cargoRoots = (policy.scope?.cargo_scan_roots || []).map((v) => normalizePath(cleanText(v, 300)));
  const nexusCrates = new Set((policy.nexus_crates || []).map((v) => normalizeCrateName(cleanText(v, 200))));
  const allowedFoundationCrates = new Set(
    (policy.allowed_foundation_crates || []).map((v) => normalizeCrateName(cleanText(v, 200))),
  );

  const tracked = trackedFiles(ROOT).map((v) => normalizePath(v));
  const manifests = listCoreManifests(tracked);
  const allCoreCrates = new Set(manifests.map((m) => m.crate_name));

  const importExemptions = new Set<string>();
  const cargoPathExemptions = new Set<string>();
  const expiredExemptions: ExemptionDrift[] = [];
  const usedImportExemptions = new Set<string>();
  const usedCargoPathExemptions = new Set<string>();

  for (const exemption of policy.exemptions?.import_edges || []) {
    const key = importEdgeKey(
      normalizeCrateName(cleanText(exemption.from_package, 200)),
      normalizeCrateName(cleanText(exemption.to_crate, 200)),
    );
    importExemptions.add(key);
    if (isExpired(exemption.expires)) {
      expiredExemptions.push({
        kind: 'import_edge',
        key,
        reason: cleanText(exemption.reason || 'expired_import_edge_exemption', 240),
      });
    }
  }

  for (const exemption of policy.exemptions?.cargo_path_edges || []) {
    const key = cargoPathEdgeKey(
      cleanText(exemption.from_manifest, 400),
      cleanText(exemption.to_path, 400),
    );
    cargoPathExemptions.add(key);
    if (isExpired(exemption.expires)) {
      expiredExemptions.push({
        kind: 'cargo_path_edge',
        key,
        reason: cleanText(exemption.reason || 'expired_cargo_path_edge_exemption', 240),
      });
    }
  }

  const importViolations: ImportViolation[] = [];
  const cargoPathViolations: CargoPathViolation[] = [];

  const rustFiles = tracked
    .filter((file) => file.endsWith('.rs'))
    .filter((file) => file.startsWith('core/'))
    .filter((file) => isUnderAnyRoot(file, rustRoots))
    .sort((a, b) => a.localeCompare(b, 'en'));

  const importLineRegex = /^\s*(?:pub\s+)?use\s+([A-Za-z_][A-Za-z0-9_]*)::/;

  for (const file of rustFiles) {
    const srcManifest = nearestManifest(file, manifests);
    if (!srcManifest) continue;
    const srcCrate = srcManifest.crate_name;
    let source = '';
    try {
      source = fs.readFileSync(path.resolve(ROOT, file), 'utf8');
    } catch {
      continue;
    }

    const lines = source.split('\n');
    for (let i = 0; i < lines.length; i += 1) {
      const match = lines[i].match(importLineRegex);
      if (!match) continue;
      const toCrate = normalizeCrateName(match[1]);
      if (!allCoreCrates.has(toCrate)) continue;
      if (toCrate === srcCrate) continue;
      if (nexusCrates.has(srcCrate) || nexusCrates.has(toCrate)) continue;
      if (allowedFoundationCrates.has(toCrate)) continue;

      const edge = importEdgeKey(srcCrate, toCrate);
      if (importExemptions.has(edge)) {
        usedImportExemptions.add(edge);
        continue;
      }

      importViolations.push({
        file,
        line: i + 1,
        from_package: srcCrate,
        to_crate: toCrate,
        rule: 'non_nexus_to_non_nexus_core_import_forbidden',
      });
    }
  }

  const cargoFiles = tracked
    .filter((file) => file.endsWith('/Cargo.toml'))
    .filter((file) => file.startsWith('core/'))
    .filter((file) => isUnderAnyRoot(file, cargoRoots))
    .sort((a, b) => a.localeCompare(b, 'en'));

  const cargoPathRegex = /path\s*=\s*"([^"]+)"/;

  for (const manifestPath of cargoFiles) {
    const srcManifest = manifests.find((m) => m.manifest === manifestPath);
    if (!srcManifest) continue;
    const srcCrate = srcManifest.crate_name;
    let source = '';
    try {
      source = fs.readFileSync(path.resolve(ROOT, manifestPath), 'utf8');
    } catch {
      continue;
    }

    const lines = source.split('\n');
    for (let i = 0; i < lines.length; i += 1) {
      const match = lines[i].match(cargoPathRegex);
      if (!match) continue;
      const rawPath = cleanText(match[1], 400);
      const resolved = normalizePath(
        path.relative(ROOT, path.resolve(path.dirname(path.resolve(ROOT, manifestPath)), rawPath)),
      );
      if (!resolved.startsWith('core/')) continue;

      const targetManifest = nearestManifest(resolved, manifests);
      if (!targetManifest) continue;
      const targetCrate = targetManifest.crate_name;
      if (targetCrate === srcCrate) continue;
      if (nexusCrates.has(srcCrate) || nexusCrates.has(targetCrate)) continue;
      if (allowedFoundationCrates.has(targetCrate)) continue;

      const edge = cargoPathEdgeKey(manifestPath, rawPath);
      if (cargoPathExemptions.has(edge)) {
        usedCargoPathExemptions.add(edge);
        continue;
      }

      cargoPathViolations.push({
        manifest: manifestPath,
        line: i + 1,
        from_package: srcCrate,
        to_crate: targetCrate,
        to_path: rawPath,
        rule: 'non_nexus_to_non_nexus_core_path_dependency_forbidden',
      });
    }
  }

  const staleExemptions: ExemptionDrift[] = [];
  for (const edge of importExemptions) {
    if (!usedImportExemptions.has(edge)) {
      staleExemptions.push({
        kind: 'import_edge',
        key: edge,
        reason: 'exemption_not_observed_in_current_scan',
      });
    }
  }
  for (const edge of cargoPathExemptions) {
    if (!usedCargoPathExemptions.has(edge)) {
      staleExemptions.push({
        kind: 'cargo_path_edge',
        key: edge,
        reason: 'exemption_not_observed_in_current_scan',
      });
    }
  }

  const failOnExpired = policy.fail_on_expired_exemptions !== false;
  const pass =
    importViolations.length === 0 &&
    cargoPathViolations.length === 0 &&
    (!failOnExpired || expiredExemptions.length === 0);

  const payload = {
    type: 'core_nexus_coupling_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    policy_path: args.policy,
    summary: {
      rust_files_scanned: rustFiles.length,
      cargo_manifests_scanned: cargoFiles.length,
      import_violations: importViolations.length,
      cargo_path_violations: cargoPathViolations.length,
      expired_exemptions: expiredExemptions.length,
      stale_exemptions: staleExemptions.length,
      pass,
    },
    violations: {
      import_edges: importViolations,
      cargo_path_edges: cargoPathViolations,
      expired_exemptions: expiredExemptions,
      stale_exemptions: staleExemptions,
    },
    policy_scope: {
      rust_scan_roots: rustRoots,
      cargo_scan_roots: cargoRoots,
      nexus_crates: Array.from(nexusCrates.values()),
      allowed_foundation_crates: Array.from(allowedFoundationCrates.values()),
    },
  };

  process.exit(
    emitStructuredResult(payload, {
      outPath: args.out,
      strict: args.strict,
      ok: pass,
    }),
  );
}

main();
