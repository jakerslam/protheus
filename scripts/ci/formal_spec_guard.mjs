#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const STATE_DIR = path.join(ROOT, 'client/runtime/local/state/ops/formal_spec_guard');

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

function readText(filePath) {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
  }
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
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

export function run(rawArgs = {}) {
  const strict = String(rawArgs.strict || '0') === '1';
  const requiredFiles = [
    'planes/spec/README.md',
    'planes/spec/tla/three_plane_boundary.tla',
    'planes/spec/tla/three_plane_boundary.cfg',
    'planes/contracts/README.md',
    'planes/contracts/conduit_envelope.schema.json',
  ];
  const missingFiles = requiredFiles.filter((relPath) => !fs.existsSync(path.join(ROOT, relPath)));

  const tlaPath = path.join(ROOT, 'planes/spec/tla/three_plane_boundary.tla');
  const tlaSource = readText(tlaPath);
  const requiredTlaTokens = ['---- MODULE three_plane_boundary ----', 'VARIABLES', 'Init', 'Next'];
  const tlaMissingTokens = requiredTlaTokens.filter((token) => !tlaSource.includes(token));

  const schemaPath = path.join(ROOT, 'planes/contracts/conduit_envelope.schema.json');
  const schema = readJson(schemaPath);
  const requiredSchemaFields = ['$schema', 'type', 'properties', 'required'];
  const schemaMissingFields = requiredSchemaFields.filter(
    (key) => !(schema && Object.prototype.hasOwnProperty.call(schema, key)),
  );

  const architecturePath = path.join(ROOT, 'ARCHITECTURE.md');
  const architectureSource = readText(architecturePath);
  const requiredArchitectureRefs = ['planes/spec', 'planes/contracts'];
  const architectureMissingRefs = requiredArchitectureRefs.filter(
    (token) => !architectureSource.includes(token),
  );

  const failures =
    missingFiles.length +
    tlaMissingTokens.length +
    schemaMissingFields.length +
    architectureMissingRefs.length;

  const out = {
    ok: strict ? failures === 0 : true,
    type: 'formal_spec_guard',
    ts: nowIso(),
    strict,
    required_files: requiredFiles,
    missing_files: missingFiles,
    tla_missing_tokens: tlaMissingTokens,
    schema_missing_fields: schemaMissingFields,
    architecture_missing_refs: architectureMissingRefs,
  };

  const latestPath = path.join(STATE_DIR, 'latest.json');
  const receiptsPath = path.join(STATE_DIR, 'receipts.jsonl');
  writeJson(latestPath, out);
  appendJsonl(receiptsPath, out);

  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  if (strict && failures > 0) process.exit(1);
  return out;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'run');
  if (cmd !== 'run' && cmd !== 'check') {
    process.stderr.write(`formal_spec_guard: unsupported command '${cmd}'\n`);
    process.exit(2);
  }
  run(args);
}
