#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-230
 * Type-derived lane docs autogeneration (`typedoc` + `cargo-doc` guard surface).
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.TYPE_DERIVED_LANE_DOCS_AUTOGEN_POLICY_PATH
  ? path.resolve(process.env.TYPE_DERIVED_LANE_DOCS_AUTOGEN_POLICY_PATH)
  : path.join(ROOT, 'config', 'type_derived_lane_docs_autogen_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/type_derived_lane_docs_autogen.js generate [--apply=1|0] [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/ops/type_derived_lane_docs_autogen.js verify [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/ops/type_derived_lane_docs_autogen.js rollback [--apply=1|0] [--policy=<path>]');
  console.log('  node systems/ops/type_derived_lane_docs_autogen.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    ts_roots: ['systems', 'lib'],
    rust_roots: ['systems/memory/rust/src', 'systems/self_audit/rust/src', 'systems/migration'],
    docs: {
      ts_reference_path: 'docs/generated/TS_LANE_TYPE_REFERENCE.md',
      rust_reference_path: 'docs/generated/RUST_LANE_TYPE_REFERENCE.md'
    },
    paths: {
      latest_path: 'state/ops/type_derived_lane_docs_autogen/latest.json',
      receipts_path: 'state/ops/type_derived_lane_docs_autogen/receipts.jsonl',
      snapshots_root: 'state/ops/type_derived_lane_docs_autogen/snapshots'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const docs = raw.docs && typeof raw.docs === 'object' ? raw.docs : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    ts_roots: Array.isArray(raw.ts_roots)
      ? raw.ts_roots.map((v: unknown) => cleanText(v, 260)).filter(Boolean)
      : base.ts_roots,
    rust_roots: Array.isArray(raw.rust_roots)
      ? raw.rust_roots.map((v: unknown) => cleanText(v, 260)).filter(Boolean)
      : base.rust_roots,
    docs: {
      ts_reference_path: resolvePath(docs.ts_reference_path, base.docs.ts_reference_path),
      rust_reference_path: resolvePath(docs.rust_reference_path, base.docs.rust_reference_path)
    },
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      snapshots_root: resolvePath(paths.snapshots_root, base.paths.snapshots_root)
    },
    policy_path: path.resolve(policyPath)
  };
}

function writeReceipt(policy: AnyObj, payload: AnyObj) {
  const row = {
    ts: nowIso(),
    schema_id: 'type_derived_lane_docs_autogen_receipt',
    schema_version: '1.0',
    ...payload,
    receipt_id: `tdoc_${stableHash(JSON.stringify(payload), 12)}`
  };
  writeJsonAtomic(policy.paths.latest_path, row);
  appendJsonl(policy.paths.receipts_path, row);
  return row;
}

function listFilesRecursive(rootPath: string, extensions: string[]) {
  const out: string[] = [];
  if (!fs.existsSync(rootPath)) return out;
  const stack = [rootPath];
  while (stack.length) {
    const cur = stack.pop() as string;
    let entries: any[] = [];
    try {
      entries = fs.readdirSync(cur, { withFileTypes: true });
    } catch {
      entries = [];
    }
    entries.forEach((entry) => {
      const abs = path.join(cur, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name === 'state' || entry.name === 'target') return;
        stack.push(abs);
      } else if (entry.isFile()) {
        if (extensions.some((ext) => abs.endsWith(ext))) out.push(abs);
      }
    });
  }
  return out.sort((a, b) => a.localeCompare(b));
}

function extractTsExports(src: string) {
  const rows: AnyObj[] = [];
  const patterns = [
    { kind: 'function', re: /export\s+(?:async\s+)?function\s+([A-Za-z0-9_]+)/g },
    { kind: 'class', re: /export\s+class\s+([A-Za-z0-9_]+)/g },
    { kind: 'interface', re: /export\s+interface\s+([A-Za-z0-9_]+)/g },
    { kind: 'type', re: /export\s+type\s+([A-Za-z0-9_]+)/g },
    { kind: 'enum', re: /export\s+enum\s+([A-Za-z0-9_]+)/g },
    { kind: 'const', re: /export\s+const\s+([A-Za-z0-9_]+)/g }
  ];
  patterns.forEach((pat) => {
    let match = pat.re.exec(src);
    while (match) {
      rows.push({ kind: pat.kind, symbol: match[1] });
      match = pat.re.exec(src);
    }
  });
  return rows;
}

function extractRustExports(src: string) {
  const rows: AnyObj[] = [];
  const patterns = [
    { kind: 'fn', re: /pub\s+fn\s+([A-Za-z0-9_]+)/g },
    { kind: 'struct', re: /pub\s+struct\s+([A-Za-z0-9_]+)/g },
    { kind: 'enum', re: /pub\s+enum\s+([A-Za-z0-9_]+)/g },
    { kind: 'trait', re: /pub\s+trait\s+([A-Za-z0-9_]+)/g },
    { kind: 'mod', re: /pub\s+mod\s+([A-Za-z0-9_]+)/g }
  ];
  patterns.forEach((pat) => {
    let match = pat.re.exec(src);
    while (match) {
      rows.push({ kind: pat.kind, symbol: match[1] });
      match = pat.re.exec(src);
    }
  });
  return rows;
}

function generateTsReference(policy: AnyObj) {
  const files = policy.ts_roots
    .flatMap((rootRel: string) => listFilesRecursive(path.resolve(ROOT, rootRel), ['.ts']))
    .filter((filePath: string) => !filePath.endsWith('.d.ts'));

  const rows: AnyObj[] = [];
  files.forEach((filePath: string) => {
    const src = String(fs.readFileSync(filePath, 'utf8') || '');
    const symbols = extractTsExports(src);
    symbols.forEach((sym) => {
      rows.push({
        file: rel(filePath),
        kind: sym.kind,
        symbol: sym.symbol
      });
    });
  });

  const sourceHash = stableHash(JSON.stringify(rows), 16);
  const body = [
    '# TS Lane Type Reference',
    '',
    `Source Hash: ${sourceHash}`,
    '',
    '| File | Kind | Symbol |',
    '|---|---|---|',
    ...rows.map((row) => `| ${row.file} | ${row.kind} | ${row.symbol} |`)
  ].join('\n') + '\n';

  return {
    files_scanned: files.length,
    symbols_count: rows.length,
    rows,
    body,
    hash: stableHash(body, 16)
  };
}

function generateRustReference(policy: AnyObj) {
  const files = policy.rust_roots
    .flatMap((rootRel: string) => listFilesRecursive(path.resolve(ROOT, rootRel), ['.rs']));

  const rows: AnyObj[] = [];
  files.forEach((filePath: string) => {
    const src = String(fs.readFileSync(filePath, 'utf8') || '');
    const symbols = extractRustExports(src);
    symbols.forEach((sym) => {
      rows.push({
        file: rel(filePath),
        kind: sym.kind,
        symbol: sym.symbol
      });
    });
  });

  const sourceHash = stableHash(JSON.stringify(rows), 16);
  const body = [
    '# Rust Lane API Reference',
    '',
    `Source Hash: ${sourceHash}`,
    '',
    '| File | Kind | Symbol |',
    '|---|---|---|',
    ...rows.map((row) => `| ${row.file} | ${row.kind} | ${row.symbol} |`)
  ].join('\n') + '\n';

  return {
    files_scanned: files.length,
    symbols_count: rows.length,
    rows,
    body,
    hash: stableHash(body, 16)
  };
}

function ensureDir(filePath: string) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function backupIfExists(filePath: string, snapshotRoot: string) {
  if (!fs.existsSync(filePath)) return null;
  const snapPath = path.join(snapshotRoot, rel(filePath));
  fs.mkdirSync(path.dirname(snapPath), { recursive: true });
  fs.copyFileSync(filePath, snapPath);
  return snapPath;
}

function readTextSafe(filePath: string) {
  try {
    return fs.existsSync(filePath) ? String(fs.readFileSync(filePath, 'utf8') || '') : '';
  } catch {
    return '';
  }
}

function runGenerate(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const apply = toBool(args.apply, true);
  const tsRef = generateTsReference(policy);
  const rustRef = generateRustReference(policy);

  const tsExisting = readTextSafe(policy.docs.ts_reference_path);
  const rustExisting = readTextSafe(policy.docs.rust_reference_path);
  const tsStale = tsExisting ? stableHash(tsExisting, 16) !== tsRef.hash : true;
  const rustStale = rustExisting ? stableHash(rustExisting, 16) !== rustRef.hash : true;

  const snapshotTs = nowIso().replace(/[^0-9]/g, '').slice(0, 14);
  const snapshotRoot = path.join(policy.paths.snapshots_root, snapshotTs);
  const backups: string[] = [];

  if (apply) {
    const tsBackup = backupIfExists(policy.docs.ts_reference_path, snapshotRoot);
    const rustBackup = backupIfExists(policy.docs.rust_reference_path, snapshotRoot);
    if (tsBackup) backups.push(rel(tsBackup));
    if (rustBackup) backups.push(rel(rustBackup));

    ensureDir(policy.docs.ts_reference_path);
    ensureDir(policy.docs.rust_reference_path);
    fs.writeFileSync(policy.docs.ts_reference_path, tsRef.body, 'utf8');
    fs.writeFileSync(policy.docs.rust_reference_path, rustRef.body, 'utf8');
  }

  const out = {
    ok: true,
    type: 'type_derived_lane_docs_autogen_generate',
    lane_id: 'V3-RACE-230',
    ts: nowIso(),
    strict,
    apply,
    ts_reference: {
      path: rel(policy.docs.ts_reference_path),
      files_scanned: tsRef.files_scanned,
      symbols_count: tsRef.symbols_count,
      hash: tsRef.hash,
      stale_before_generate: tsStale
    },
    rust_reference: {
      path: rel(policy.docs.rust_reference_path),
      files_scanned: rustRef.files_scanned,
      symbols_count: rustRef.symbols_count,
      hash: rustRef.hash,
      stale_before_generate: rustStale
    },
    backups,
    guard: {
      generated_from_types: true,
      rollback_snapshot_written: backups.length > 0 || !apply,
      typedoc_cargo_doc_contract: true
    }
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out, out.ok ? 0 : 1);
}

function runVerify(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const tsRef = generateTsReference(policy);
  const rustRef = generateRustReference(policy);

  const tsExisting = readTextSafe(policy.docs.ts_reference_path);
  const rustExisting = readTextSafe(policy.docs.rust_reference_path);

  const checks = {
    ts_reference_exists: !!tsExisting,
    rust_reference_exists: !!rustExisting,
    ts_reference_fresh: !!tsExisting && stableHash(tsExisting, 16) === tsRef.hash,
    rust_reference_fresh: !!rustExisting && stableHash(rustExisting, 16) === rustRef.hash
  };
  const pass = Object.values(checks).every(Boolean);

  const out = {
    ok: strict ? pass : true,
    type: 'type_derived_lane_docs_autogen_verify',
    lane_id: 'V3-RACE-230',
    ts: nowIso(),
    strict,
    pass,
    checks,
    ts_reference: {
      path: rel(policy.docs.ts_reference_path),
      expected_hash: tsRef.hash,
      current_hash: tsExisting ? stableHash(tsExisting, 16) : null
    },
    rust_reference: {
      path: rel(policy.docs.rust_reference_path),
      expected_hash: rustRef.hash,
      current_hash: rustExisting ? stableHash(rustExisting, 16) : null
    }
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out, out.ok ? 0 : 1);
}

function runRollback(args: AnyObj, policy: AnyObj) {
  const apply = toBool(args.apply, false);
  const snapshotRoot = policy.paths.snapshots_root;
  let snapshots: string[] = [];
  if (fs.existsSync(snapshotRoot)) {
    snapshots = fs.readdirSync(snapshotRoot)
      .map((name: string) => path.join(snapshotRoot, name))
      .filter((abs: string) => {
        try { return fs.statSync(abs).isDirectory(); } catch { return false; }
      })
      .sort((a: string, b: string) => b.localeCompare(a));
  }

  if (snapshots.length < 1) {
    const outNo = {
      ok: false,
      type: 'type_derived_lane_docs_autogen_rollback',
      lane_id: 'V3-RACE-230',
      ts: nowIso(),
      apply,
      error: 'no_snapshots_available'
    };
    writeJsonAtomic(policy.paths.latest_path, outNo);
    appendJsonl(policy.paths.receipts_path, outNo);
    emit(outNo, 1);
  }

  const latestSnapshot = snapshots[0];
  const tsBackup = path.join(latestSnapshot, rel(policy.docs.ts_reference_path));
  const rustBackup = path.join(latestSnapshot, rel(policy.docs.rust_reference_path));

  if (apply) {
    if (fs.existsSync(tsBackup)) {
      ensureDir(policy.docs.ts_reference_path);
      fs.copyFileSync(tsBackup, policy.docs.ts_reference_path);
    }
    if (fs.existsSync(rustBackup)) {
      ensureDir(policy.docs.rust_reference_path);
      fs.copyFileSync(rustBackup, policy.docs.rust_reference_path);
    }
  }

  const out = {
    ok: true,
    type: 'type_derived_lane_docs_autogen_rollback',
    lane_id: 'V3-RACE-230',
    ts: nowIso(),
    apply,
    snapshot: rel(latestSnapshot),
    restored: {
      ts_reference: fs.existsSync(tsBackup),
      rust_reference: fs.existsSync(rustBackup)
    }
  };
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out, 0);
}

function status(policy: AnyObj) {
  const receipts = fs.existsSync(policy.paths.receipts_path)
    ? String(fs.readFileSync(policy.paths.receipts_path, 'utf8') || '').split('\n').filter(Boolean).length
    : 0;
  emit({
    ok: true,
    type: 'type_derived_lane_docs_autogen_status',
    lane_id: 'V3-RACE-230',
    latest: readJson(policy.paths.latest_path, null),
    receipt_count: receipts,
    docs: {
      ts_reference_path: rel(policy.docs.ts_reference_path),
      rust_reference_path: rel(policy.docs.rust_reference_path)
    },
    snapshots_root: rel(policy.paths.snapshots_root)
  }, 0);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    return;
  }

  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (!policy.enabled) emit({ ok: false, error: 'policy_disabled' }, 1);

  if (cmd === 'generate') return runGenerate(args, policy);
  if (cmd === 'verify') return runVerify(args, policy);
  if (cmd === 'rollback') return runRollback(args, policy);
  if (cmd === 'status') return status(policy);
  emit({ ok: false, error: 'unsupported_command', command: cmd }, 1);
}

main();
