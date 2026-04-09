#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/dependency_boundary_manifest.json');

function nowIso() {
  return new Date().toISOString();
}

function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) {
      out._.push(tok);
      continue;
    }
    const eq = tok.indexOf('=');
    if (eq >= 0) {
      out[tok.slice(2, eq)] = tok.slice(eq + 1);
      continue;
    }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function rel(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function readJson(filePath, fallback) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function writeJson(filePath, value) {
  ensureDir(filePath);
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath, value) {
  ensureDir(filePath);
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}

function listFiles(baseDir, extSet, excludeContains) {
  if (!fs.existsSync(baseDir)) return [];
  const out = [];
  const stack = [baseDir];
  while (stack.length) {
    const cur = stack.pop();
    for (const ent of fs.readdirSync(cur, { withFileTypes: true })) {
      const abs = path.join(cur, ent.name);
      const relPath = rel(abs);
      if (excludeContains.some((token) => relPath.includes(token))) continue;
      if (ent.isDirectory()) {
        stack.push(abs);
      } else if (ent.isFile() && extSet.has(path.extname(abs))) {
        out.push(abs);
      }
    }
  }
  return out.sort();
}

function detectLayer(relPath, layers) {
  for (const [layer, roots] of Object.entries(layers || {})) {
    for (const root of roots || []) {
      const normalized = String(root).replace(/\\/g, '/').replace(/\/+$/, '');
      if (relPath === normalized || relPath.startsWith(`${normalized}/`)) return layer;
    }
  }
  return null;
}

function parseSpecs(source) {
  const specs = [];
  const regex = /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match;
  while ((match = regex.exec(source)) != null) {
    specs.push(match[1]);
  }
  return specs;
}

function resolveLocalSpec(fromFile, spec) {
  const base = path.resolve(path.dirname(fromFile), spec);
  const stemCandidates = [];
  const ext = path.extname(base);
  if (ext === '.js' || ext === '.mjs' || ext === '.cjs') {
    stemCandidates.push(base.slice(0, -ext.length));
  }
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.js`,
    `${base}.mjs`,
    path.join(base, 'index.ts'),
    path.join(base, 'index.js'),
    path.join(base, 'index.mjs'),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
  }
  for (const stem of stemCandidates) {
    const stemChecks = [
      stem,
      `${stem}.ts`,
      `${stem}.js`,
      `${stem}.mjs`,
      path.join(stem, 'index.ts'),
      path.join(stem, 'index.js'),
      path.join(stem, 'index.mjs'),
    ];
    for (const candidate of stemChecks) {
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) return candidate;
    }
  }
  return null;
}

export function run(rawArgs = {}) {
  const strict = String(rawArgs.strict || '0') === '1';
  const policy = readJson(POLICY_PATH, null) || {};
  const scan = policy.scan || {};
  const includeDirs = Array.isArray(scan.include_dirs) ? scan.include_dirs : [];
  const includeExt = new Set((scan.include_ext || ['.ts', '.js']).map((v) => String(v)));
  const excludeContains = Array.isArray(scan.exclude_contains) ? scan.exclude_contains : [];

  const files = [];
  for (const dir of includeDirs) {
    files.push(...listFiles(path.join(ROOT, dir), includeExt, excludeContains));
  }

  const enforceLayers = new Set((policy.enforce_layers || []).map((v) => String(v)));
  const allowImports = policy.allow_imports || {};
  const layers = policy.layers || {};

  const layerViolations = [];
  const conduitViolations = [];
  const missingLocalImports = [];
  const conduit = policy.conduit_boundary || {};
  const conduitAllow = new Set((conduit.allowlisted_files || []).map((v) => String(v).replace(/\\/g, '/')));
  const conduitExtSet = new Set((conduit.include_ext || ['.ts', '.js']).map((v) => String(v)));
  const conduitDirs = Array.isArray(conduit.include_dirs) ? conduit.include_dirs : [];
  const conduitExcludes = Array.isArray(conduit.exclude_contains) ? conduit.exclude_contains : [];
  const forbiddenPatterns = Array.isArray(conduit.forbidden_patterns) ? conduit.forbidden_patterns : [];
  const conduitRoots = conduitDirs.map((d) => String(d).replace(/\\/g, '/').replace(/\/+$/, ''));

  for (const filePath of files) {
    const relPath = rel(filePath);
    const source = fs.readFileSync(filePath, 'utf8');
    const sourceLayer = detectLayer(relPath, layers);
    const specs = parseSpecs(source);

    for (const spec of specs) {
      if (!spec.startsWith('.')) continue;
      const resolved = resolveLocalSpec(filePath, spec);
      if (!resolved) {
        missingLocalImports.push({ file: relPath, spec });
        continue;
      }
      const targetRel = rel(resolved);
      const targetLayer = detectLayer(targetRel, layers);
      if (!sourceLayer || !targetLayer) continue;
      if (!enforceLayers.has(sourceLayer)) continue;
      const allowed = new Set((allowImports[sourceLayer] || []).map((v) => String(v)));
      if (!allowed.has(targetLayer)) {
        layerViolations.push({
          file: relPath,
          source_layer: sourceLayer,
          spec,
          resolved: targetRel,
          target_layer: targetLayer,
        });
      }
    }

    const inConduitScope =
      conduitRoots.some((root) => relPath === root || relPath.startsWith(`${root}/`)) &&
      conduitExtSet.has(path.extname(relPath)) &&
      !conduitExcludes.some((token) => relPath.includes(token));
    if (!inConduitScope || conduitAllow.has(relPath)) continue;
    for (const token of forbiddenPatterns) {
      if (!token) continue;
      if (source.includes(token)) {
        conduitViolations.push({ file: relPath, forbidden_pattern: token });
      }
    }
  }

  const violations = [...layerViolations, ...conduitViolations];
  const missingCount = missingLocalImports.length;
  const ok = violations.length === 0 && missingCount === 0;
  const out = {
    ok: strict ? ok : true,
    type: 'dependency_boundary_guard',
    ts: nowIso(),
    strict,
    scanned_files: files.length,
    layer_violations: layerViolations,
    conduit_violations: conduitViolations,
    missing_local_imports: missingLocalImports,
  };

  const latestPath = path.join(
    ROOT,
    (policy.paths && policy.paths.latest_path) ||
      'client/runtime/local/state/ops/dependency_boundary_guard/latest.json',
  );
  const receiptsPath = path.join(
    ROOT,
    (policy.paths && policy.paths.receipts_path) ||
      'client/runtime/local/state/ops/dependency_boundary_guard/receipts.jsonl',
  );
  writeJson(latestPath, out);
  appendJsonl(receiptsPath, out);

  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  if (strict && !ok) process.exit(1);
  return out;
}

if (require.main === module) {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'check');
  if (cmd !== 'check' && cmd !== 'run') {
    process.stderr.write(`dependency_boundary_guard: unsupported command '${cmd}'\n`);
    process.exit(2);
  }
  run(args);
}
