#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

type MatrixRule = {
  path_prefix: string;
  spdx: string;
  scope?: string;
};

type LicenseMatrix = {
  schema_id: string;
  schema_version: string;
  default_spdx: string;
  precedence: string[];
  path_rules: MatrixRule[];
  release_metadata: {
    docker_image_spdx: string;
    release_bundle_spdx: string;
    npm_package_spdx: string;
  };
};

const DEFAULTS = {
  strict: false,
  matrixPath: 'LICENSE_MATRIX.json',
  outJson: 'core/local/artifacts/license_spdx_story_gate_current.json',
};

function parseArgs(argv: string[]) {
  const out = { ...DEFAULTS };
  for (const raw of argv) {
    const arg = String(raw || '').trim();
    if (!arg) continue;
    if (arg === '--strict' || arg === '--strict=1') {
      out.strict = true;
      continue;
    }
    if (arg.startsWith('--strict=')) {
      const v = arg.slice('--strict='.length).trim().toLowerCase();
      out.strict = ['1', 'true', 'yes', 'on'].includes(v);
      continue;
    }
    if (arg.startsWith('--matrix=')) {
      out.matrixPath = arg.slice('--matrix='.length).trim() || DEFAULTS.matrixPath;
      continue;
    }
    if (arg.startsWith('--out-json=')) {
      out.outJson = arg.slice('--out-json='.length).trim() || DEFAULTS.outJson;
      continue;
    }
  }
  return out;
}

function ensureParent(filePath: string) {
  mkdirSync(dirname(resolve(filePath)), { recursive: true });
}

function readJson<T>(filePath: string): T {
  return JSON.parse(readFileSync(resolve(filePath), 'utf8')) as T;
}

function normalizePath(raw: string): string {
  return String(raw || '').replace(/\\/g, '/').replace(/^\.\//, '').trim();
}

function isLikelyBinary(buf: Buffer): boolean {
  const sample = buf.subarray(0, Math.min(buf.length, 8192));
  for (const byte of sample) {
    if (byte === 0) return true;
  }
  return false;
}

function readHeaderSpdx(path: string): string | null {
  const abs = resolve(path);
  let buf: Buffer;
  try {
    buf = readFileSync(abs);
  } catch {
    return null;
  }
  if (buf.length === 0 || isLikelyBinary(buf)) return null;
  const text = buf.toString('utf8', 0, Math.min(buf.length, 64 * 1024));
  const lines = text.split(/\r?\n/).slice(0, 80);
  for (const line of lines) {
    const m = line.match(
      /^\s*(?:\/\/+|#|\/\*+|\*|<!--)\s*SPDX-License-Identifier:\s*([A-Za-z0-9.\-+() :]+?)\s*(?:\*\/|-->|$)/i,
    );
    if (!m) continue;
    const normalized = String(m[1] || '')
      .replace(/\s+/g, ' ')
      .trim();
    if (normalized) return normalized;
  }
  return null;
}

function listTrackedFiles(): string[] {
  const output = execSync('git ls-files -z', { encoding: 'utf8' });
  return output
    .split('\u0000')
    .map((row) => normalizePath(row))
    .filter(Boolean)
    .sort((a, b) => a.localeCompare(b));
}

function selectRule(path: string, rules: MatrixRule[]) {
  const matches = rules.filter((rule) => {
    const prefix = normalizePath(rule.path_prefix);
    if (!prefix) return false;
    return path === prefix || path.startsWith(prefix.endsWith('/') ? prefix : `${prefix}/`);
  });
  if (matches.length === 0) return { rule: null as MatrixRule | null, ambiguous: false };

  matches.sort((a, b) => normalizePath(b.path_prefix).length - normalizePath(a.path_prefix).length);
  const best = matches[0];
  const bestLen = normalizePath(best.path_prefix).length;
  const sameLen = matches.filter((rule) => normalizePath(rule.path_prefix).length === bestLen);
  const spdxSet = new Set(sameLen.map((rule) => String(rule.spdx || '').trim()));
  if (spdxSet.size > 1) {
    return { rule: null as MatrixRule | null, ambiguous: true };
  }
  return { rule: best, ambiguous: false };
}

function nonEmptyText(raw: unknown): string {
  return String(raw == null ? '' : raw).trim();
}

function validateMatrix(matrix: LicenseMatrix, violations: string[]) {
  if (!nonEmptyText(matrix.schema_id)) violations.push('matrix.schema_id_missing');
  if (!nonEmptyText(matrix.schema_version)) violations.push('matrix.schema_version_missing');
  if (!nonEmptyText(matrix.default_spdx)) violations.push('matrix.default_spdx_missing');
  if (!Array.isArray(matrix.path_rules)) violations.push('matrix.path_rules_missing');
  const pathRules = Array.isArray(matrix.path_rules) ? matrix.path_rules : [];
  const seen = new Set<string>();
  for (const rule of pathRules) {
    const prefix = normalizePath(rule.path_prefix);
    const spdx = nonEmptyText(rule.spdx);
    if (!prefix) violations.push('matrix.rule.path_prefix_missing');
    if (!spdx) violations.push(`matrix.rule.spdx_missing:${prefix || 'unknown'}`);
    if (prefix) {
      if (seen.has(prefix)) violations.push(`matrix.rule.duplicate_prefix:${prefix}`);
      seen.add(prefix);
    }
  }
  const release = matrix.release_metadata || ({} as LicenseMatrix['release_metadata']);
  if (!nonEmptyText(release.docker_image_spdx)) violations.push('matrix.release_metadata.docker_image_spdx_missing');
  if (!nonEmptyText(release.release_bundle_spdx)) violations.push('matrix.release_metadata.release_bundle_spdx_missing');
  if (!nonEmptyText(release.npm_package_spdx)) violations.push('matrix.release_metadata.npm_package_spdx_missing');
}

function checkStaticSurface(matrix: LicenseMatrix) {
  const violations: string[] = [];
  const checks: Array<{ file: string; pattern: RegExp; code: string }> = [
    { file: 'LICENSE_SCOPE.md', pattern: /LICENSE_MATRIX\.json/, code: 'license_scope.missing_matrix_link' },
    { file: 'README.md', pattern: /LICENSE_MATRIX\.json/, code: 'readme.missing_matrix_link' },
    { file: 'README.md', pattern: /Apache-2\.0/, code: 'readme.missing_apache_reference' },
    { file: 'README.md', pattern: /LicenseRef-InfRing-NC-1\.0/, code: 'readme.missing_nc_reference' },
    { file: 'SECURITY.md', pattern: /LICENSE_MATRIX\.json/, code: 'security.missing_matrix_link' },
    { file: 'SECURITY.md', pattern: /Apache-2\.0/, code: 'security.missing_apache_reference' },
    { file: 'SECURITY.md', pattern: /LicenseRef-InfRing-NC-1\.0/, code: 'security.missing_nc_reference' },
    {
      file: 'Dockerfile',
      pattern: new RegExp(`org\\.opencontainers\\.image\\.licenses=\\\"${matrix.release_metadata.docker_image_spdx.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\\"`),
      code: 'dockerfile.license_label_mismatch',
    },
    {
      file: 'package.json',
      pattern: new RegExp(`\\\"license\\\"\\s*:\\s*\\\"${matrix.release_metadata.npm_package_spdx.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\\"`),
      code: 'package_json.license_field_mismatch',
    },
    {
      file: '.github/workflows/release.yml',
      pattern: /release_licensing_manifest\.json/,
      code: 'release_yml.missing_license_manifest',
    },
    {
      file: '.github/workflows/release-security-artifacts.yml',
      pattern: /release_licensing_manifest\.json/,
      code: 'release_security_yml.missing_license_manifest',
    },
  ];

  for (const check of checks) {
    let body = '';
    try {
      body = readFileSync(resolve(check.file), 'utf8');
    } catch {
      violations.push(`${check.code}:missing_file:${check.file}`);
      continue;
    }
    if (!check.pattern.test(body)) {
      violations.push(`${check.code}:${check.file}`);
    }
  }
  return violations;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const violations: string[] = [];
  const now = new Date().toISOString();

  const matrix = readJson<LicenseMatrix>(args.matrixPath);
  validateMatrix(matrix, violations);
  violations.push(...checkStaticSurface(matrix));

  const files = listTrackedFiles();
  const resolvedRows: Array<{
    path: string;
    effective_spdx: string;
    source: 'header' | 'path_rule' | 'default';
    matched_rule_prefix: string | null;
  }> = [];
  const licenseCounts = new Map<string, number>();

  for (const file of files) {
    const headerSpdx = readHeaderSpdx(file);
    if (headerSpdx) {
      resolvedRows.push({
        path: file,
        effective_spdx: headerSpdx,
        source: 'header',
        matched_rule_prefix: null,
      });
      licenseCounts.set(headerSpdx, (licenseCounts.get(headerSpdx) || 0) + 1);
      continue;
    }

    const selected = selectRule(file, matrix.path_rules || []);
    if (selected.ambiguous) {
      violations.push(`ambiguous_path_rule:${file}`);
      continue;
    }
    if (selected.rule) {
      const spdx = nonEmptyText(selected.rule.spdx);
      if (!spdx) {
        violations.push(`empty_path_rule_spdx:${file}:${normalizePath(selected.rule.path_prefix)}`);
        continue;
      }
      resolvedRows.push({
        path: file,
        effective_spdx: spdx,
        source: 'path_rule',
        matched_rule_prefix: normalizePath(selected.rule.path_prefix),
      });
      licenseCounts.set(spdx, (licenseCounts.get(spdx) || 0) + 1);
      continue;
    }

    const defaultSpdx = nonEmptyText(matrix.default_spdx);
    if (!defaultSpdx) {
      violations.push(`default_spdx_missing_for_file:${file}`);
      continue;
    }
    resolvedRows.push({
      path: file,
      effective_spdx: defaultSpdx,
      source: 'default',
      matched_rule_prefix: null,
    });
    licenseCounts.set(defaultSpdx, (licenseCounts.get(defaultSpdx) || 0) + 1);
  }

  if (resolvedRows.length !== files.length) {
    violations.push(`file_resolution_mismatch:resolved=${resolvedRows.length}:tracked=${files.length}`);
  }

  const payload = {
    ok: violations.length === 0,
    type: 'license_spdx_story_gate',
    generated_at: now,
    matrix_path: args.matrixPath,
    summary: {
      strict: args.strict,
      total_tracked_files: files.length,
      total_resolved_files: resolvedRows.length,
      unique_spdx_count: licenseCounts.size,
      violations: violations.length,
    },
    license_counts: Object.fromEntries(
      Array.from(licenseCounts.entries()).sort((a, b) => a[0].localeCompare(b[0])),
    ),
    violations,
  };

  ensureParent(args.outJson);
  writeFileSync(resolve(args.outJson), `${JSON.stringify(payload, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(payload)}\n`);

  if (args.strict && !payload.ok) {
    process.exitCode = 1;
  }
}

main();
