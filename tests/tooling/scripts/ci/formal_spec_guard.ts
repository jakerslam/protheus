#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const STATE_DIR = path.join(ROOT, 'client/runtime/local/state/ops/formal_spec_guard');
const ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/formal_spec_guard_current.json');
const REPORT_PATH = path.join(ROOT, 'local/workspace/reports/FORMAL_SPEC_GUARD_CURRENT.md');

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

function writeMarkdownReport(filePath, out) {
  const lines = [
    '# Formal Spec Guard Current',
    '',
    `- ok: ${out.ok}`,
    `- strict: ${out.strict}`,
    `- missing files: ${out.missing_files.length}`,
    `- missing Layer0 invariant symbols: ${out.layer0_missing_invariant_symbols.length}`,
    `- missing formal surfaces: ${out.missing_formal_surfaces.length}`,
    `- missing proof layers: ${out.missing_proof_layers.length}`,
    '',
    '## Required Layer0 invariant symbols',
    '',
    ...out.required_layer0_invariant_symbols.map((symbol) => `- ${symbol}`),
    '',
    '## Required formal surfaces',
    '',
    ...out.required_formal_surfaces.map((surface) => `- ${surface}`),
    '',
  ];
  ensureDir(filePath);
  fs.writeFileSync(filePath, `${lines.join('\n')}\n`, 'utf8');
}

export function run(rawArgs = {}) {
  const strict = String(rawArgs.strict || '0') === '1';
  const requiredFiles = [
    'planes/spec/README.md',
    'planes/spec/tla/three_plane_boundary.tla',
    'planes/spec/tla/three_plane_boundary.cfg',
    'planes/contracts/README.md',
    'planes/contracts/conduit_envelope.schema.json',
    'proofs/layer0/Layer0Invariants.lean',
    'proofs/layer1/Layer1Invariants.lean',
    'proofs/layer0/core_formal_coverage_map.json',
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

  const coverageMapPath = path.join(ROOT, 'proofs/layer0/core_formal_coverage_map.json');
  const coverageMap = readJson(coverageMapPath);
  const layer0ProofPath = path.join(ROOT, 'proofs/layer0/Layer0Invariants.lean');
  const layer0ProofSource = readText(layer0ProofPath);
  const requiredLayer0InvariantSymbols = [
    'SchedulingFairness',
    'scheduling_fairness_enforced',
    'ResourceBoundsRespected',
    'resource_bound_enforced',
    'ReceiptCompleteness',
    'receipt_completeness_enforced',
    'layer0_runtime_closure_invariant_bundle',
  ];
  const layer0MissingInvariantSymbols = requiredLayer0InvariantSymbols.filter(
    (token) => !layer0ProofSource.includes(token),
  );
  const coverageSurfaces = Array.isArray(coverageMap?.surfaces) ? coverageMap.surfaces : [];
  const coverageSurfaceIds = coverageSurfaces.map((entry) => String(entry?.id || ''));
  const coverageProofLayers = Array.from(
    new Set(
      coverageSurfaces
        .map((entry) => String(entry?.artifact || ''))
        .filter((artifact) => artifact.startsWith('proofs/layer'))
        .map((artifact) => artifact.split('/').slice(0, 2).join('/')),
    ),
  ).sort();
  const requiredProofLayers = ['proofs/layer0', 'proofs/layer1'];
  const missingProofLayers = requiredProofLayers.filter((layer) => !coverageProofLayers.includes(layer));
  const requiredFormalSurfaces = [
    'core/layer2/execution::scheduler_fairness',
    'core/layer1/resource::resource_bounds',
    'core/layer2/execution::receipt_completeness',
  ];
  const missingFormalSurfaces = requiredFormalSurfaces.filter(
    (surface) => !coverageSurfaceIds.includes(surface),
  );
  const formalSurfaceCommandFailures = coverageSurfaces
    .filter((entry) => requiredFormalSurfaces.includes(String(entry?.id || '')))
    .filter((entry) => {
      const commands = Array.isArray(entry?.proof_commands) ? entry.proof_commands : [];
      return !commands.some(
        (command) =>
          command?.id === 'formal_spec_guard' &&
          command?.required === true &&
          Array.isArray(command?.argv) &&
          command.argv.join(' ') === 'npm run -s ops:formal-spec:check',
      );
    })
    .map((entry) => String(entry?.id || 'unknown'));

  const failures =
    missingFiles.length +
    tlaMissingTokens.length +
    schemaMissingFields.length +
    architectureMissingRefs.length +
    missingProofLayers.length +
    layer0MissingInvariantSymbols.length +
    missingFormalSurfaces.length +
    formalSurfaceCommandFailures.length;

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
    required_proof_layers: requiredProofLayers,
    proof_layers_present: coverageProofLayers,
    missing_proof_layers: missingProofLayers,
    required_layer0_invariant_symbols: requiredLayer0InvariantSymbols,
    layer0_missing_invariant_symbols: layer0MissingInvariantSymbols,
    required_formal_surfaces: requiredFormalSurfaces,
    missing_formal_surfaces: missingFormalSurfaces,
    formal_surface_command_failures: formalSurfaceCommandFailures,
  };

  const latestPath = path.join(STATE_DIR, 'latest.json');
  const receiptsPath = path.join(STATE_DIR, 'receipts.jsonl');
  writeJson(latestPath, out);
  appendJsonl(receiptsPath, out);
  writeJson(ARTIFACT_PATH, out);
  writeMarkdownReport(REPORT_PATH, out);

  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  if (strict && failures > 0) process.exit(1);
  return out;
}

if (require.main === module) {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'run');
  if (cmd !== 'run' && cmd !== 'check') {
    process.stderr.write(`formal_spec_guard: unsupported command '${cmd}'\n`);
    process.exit(2);
  }
  run(args);
}
