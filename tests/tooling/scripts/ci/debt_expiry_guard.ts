#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

type Args = {
  strict: boolean;
  out: string;
  warnDays: number;
};

type PolicySpec = {
  file: string;
  array_path: string;
  required_fields: string[];
  expiry_field: string;
};

type Violation = {
  file: string;
  reason: string;
  detail: string;
};

const ROOT = process.cwd();

const POLICY_SPECS: PolicySpec[] = [
  {
    file: 'client/runtime/config/arch_boundary_conformance_policy.json',
    array_path: 'allowed_violations',
    required_fields: ['file', 'reason', 'detail_contains', 'owner', 'ticket', 'expires_at'],
    expiry_field: 'expires_at',
  },
  {
    file: 'docs/workspace/repo_file_size_policy.json',
    array_path: 'exceptions',
    required_fields: ['path', 'owner', 'reason', 'expires'],
    expiry_field: 'expires',
  },
  {
    file: 'docs/workspace/rust_core_file_size_policy.json',
    array_path: 'exceptions',
    required_fields: ['path', 'owner', 'reason', 'expires'],
    expiry_field: 'expires',
  },
];

function parseArgs(argv: string[]): Args {
  const out: Args = {
    strict: false,
    out: 'core/local/artifacts/debt_expiry_guard_current.json',
    warnDays: 7,
  };
  for (const arg of argv) {
    if (arg === '--strict' || arg === '--strict=1') out.strict = true;
    else if (arg.startsWith('--strict=')) {
      const value = arg.slice('--strict='.length).trim().toLowerCase();
      out.strict = value === '1' || value === 'true' || value === 'yes' || value === 'on';
    } else if (arg.startsWith('--out=')) {
      out.out = arg.slice('--out='.length).trim() || out.out;
    } else if (arg.startsWith('--warn-days=')) {
      const value = Number(arg.slice('--warn-days='.length).trim());
      if (Number.isFinite(value) && value >= 0) out.warnDays = Math.floor(value);
    }
  }
  return out;
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function nestedGet(source: unknown, dotted: string): unknown {
  const parts = dotted.split('.').filter(Boolean);
  let cursor: any = source;
  for (const part of parts) {
    if (!cursor || typeof cursor !== 'object' || !(part in cursor)) return undefined;
    cursor = cursor[part];
  }
  return cursor;
}

function run(args: Args): number {
  const violations: Violation[] = [];
  const perPolicy = [];
  const now = Date.now();
  const warnThreshold = now + args.warnDays * 24 * 60 * 60 * 1000;
  const expiringSoon: Array<{ file: string; detail: string; expires: string }> = [];

  for (const spec of POLICY_SPECS) {
    const abs = path.resolve(ROOT, spec.file);
    if (!fs.existsSync(abs)) {
      violations.push({
        file: spec.file,
        reason: 'policy_file_missing',
        detail: spec.file,
      });
      continue;
    }
    let parsed: any = null;
    try {
      parsed = JSON.parse(fs.readFileSync(abs, 'utf8'));
    } catch (err) {
      violations.push({
        file: spec.file,
        reason: 'policy_parse_failed',
        detail: String(err),
      });
      continue;
    }
    const rows = nestedGet(parsed, spec.array_path);
    if (!Array.isArray(rows)) {
      violations.push({
        file: spec.file,
        reason: 'policy_array_missing',
        detail: spec.array_path,
      });
      continue;
    }
    let expiredCount = 0;
    let metadataCount = 0;
    for (let idx = 0; idx < rows.length; idx += 1) {
      const row = rows[idx] as Record<string, unknown>;
      if (!row || typeof row !== 'object') {
        violations.push({
          file: spec.file,
          reason: 'rule_invalid_type',
          detail: `${spec.array_path}[${idx}]`,
        });
        continue;
      }
      const missing = spec.required_fields.filter((field) => {
        const value = row[field];
        return typeof value !== 'string' || String(value).trim().length === 0;
      });
      if (missing.length > 0) {
        metadataCount += 1;
        violations.push({
          file: spec.file,
          reason: 'rule_missing_metadata',
          detail: `${spec.array_path}[${idx}] missing=${missing.join(',')}`,
        });
        continue;
      }
      const expiry = String(row[spec.expiry_field] || '').trim();
      const expiryTs = Date.parse(`${expiry}T00:00:00Z`);
      if (!Number.isFinite(expiryTs)) {
        metadataCount += 1;
        violations.push({
          file: spec.file,
          reason: 'rule_invalid_expiry',
          detail: `${spec.array_path}[${idx}] ${spec.expiry_field}=${expiry}`,
        });
        continue;
      }
      if (expiryTs < now) {
        expiredCount += 1;
        violations.push({
          file: spec.file,
          reason: 'rule_expired',
          detail: `${spec.array_path}[${idx}] expired=${expiry}`,
        });
      } else if (expiryTs <= warnThreshold) {
        expiringSoon.push({
          file: spec.file,
          detail: `${spec.array_path}[${idx}]`,
          expires: expiry,
        });
      }
    }
    perPolicy.push({
      file: spec.file,
      array_path: spec.array_path,
      row_count: rows.length,
      expired_count: expiredCount,
      metadata_violation_count: metadataCount,
    });
  }

  const payload = {
    type: 'debt_expiry_guard',
    generated_at: new Date().toISOString(),
    summary: {
      policy_count: POLICY_SPECS.length,
      violation_count: violations.length,
      expiring_soon_count: expiringSoon.length,
      warn_days: args.warnDays,
      pass: violations.length === 0,
    },
    policies: perPolicy,
    expiring_soon: expiringSoon.sort((left, right) =>
      left.expires.localeCompare(right.expires) || left.file.localeCompare(right.file),
    ),
    violations,
  };

  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(payload, null, 2));
  if (args.strict && violations.length > 0) return 1;
  return 0;
}

process.exit(run(parseArgs(process.argv.slice(2))));
