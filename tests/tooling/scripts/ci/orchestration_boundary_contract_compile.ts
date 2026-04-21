#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

type Args = {
  contract: string;
  write: boolean;
  strict: boolean;
  out: string;
};

type Binding = {
  wrapper: string;
  script: string;
  ownership: string;
};

type ControlPlaneSubdomainSpec = {
  id: string;
  module: string;
};

type ControlPlaneContractAudit = {
  module_path: string;
  tests_path: string;
  required_subdomains: string[];
  missing_module_declarations: string[];
  missing_boundary_bindings: string[];
  missing_subdomain_tests: string[];
  missing_contract_markers: string[];
  pass: boolean;
};

const ROOT = process.cwd();
const CONTROL_PLANE_MODULE_PATH = 'surface/orchestration/src/control_plane/mod.rs';
const CONTROL_PLANE_TEST_PATH = 'surface/orchestration/tests/control_plane_subdomains.rs';
const REQUIRED_CONTROL_PLANE_SUBDOMAINS: ControlPlaneSubdomainSpec[] = [
  { id: 'intake_normalization', module: 'intake_normalization' },
  { id: 'decomposition_planning', module: 'decomposition_planning' },
  { id: 'workflow_graph_dependency_tracking', module: 'workflow_graph_dependency' },
  { id: 'recovery_escalation', module: 'recovery_escalation' },
  { id: 'result_shaping_packaging', module: 'result_shaping_packaging' },
];
const REQUIRED_CONTROL_PLANE_CONTRACT_MARKERS = [
  'pub fn control_plane_api_contract()',
  'allowed_kernel_inputs',
  'allowed_kernel_outputs',
  'forbidden_authority_domains',
  'message_boundary_invariants',
  'kernel_is_final_authority',
] as const;

function parseArgs(argv: string[]): Args {
  const out: Args = {
    contract: 'planes/contracts/orchestration/client_surface_wrapper_contract_v1.json',
    write: false,
    strict: false,
    out: 'core/local/artifacts/orchestration_boundary_contract_compile_current.json',
  };
  for (const token of argv) {
    if (token === '--write=1' || token === '--write' || token === '--apply=1') out.write = true;
    else if (token.startsWith('--contract=')) out.contract = token.slice('--contract='.length);
    else if (token === '--strict=1' || token === '--strict') out.strict = true;
    else if (token.startsWith('--out=')) out.out = token.slice('--out='.length);
  }
  return out;
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function normalize(text: string): string {
  return text.replace(/\r\n/g, '\n').trimEnd();
}

function wrapperTemplate(binding: Binding): string {
  const wrapperAbs = path.resolve(ROOT, binding.wrapper);
  const scriptAbs = path.resolve(ROOT, binding.script);
  const spec = path
    .relative(path.dirname(wrapperAbs), scriptAbs)
    .replace(/\\/g, '/');
  const normalizedSpec = spec.startsWith('.') ? spec : `./${spec}`;
  return `#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (${binding.ownership}); this file is a thin CLI bridge.

const impl = require('${normalizedSpec}');

function run(args = process.argv.slice(2)) {
  return impl.run(args);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...impl,
  run
};
`;
}

function scanClientSurfaceShims(): string[] {
  const root = path.resolve(ROOT, 'client/runtime/systems');
  if (!fs.existsSync(root)) return [];
  const out: string[] = [];
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const abs = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(abs);
        continue;
      }
      if (!entry.isFile() || !entry.name.endsWith('.ts')) continue;
      const source = fs.readFileSync(abs, 'utf8');
      if (source.includes('surface/orchestration/scripts/')) {
        out.push(rel(abs));
      }
    }
  }
  return out.sort();
}

function auditControlPlaneContract(): ControlPlaneContractAudit {
  const moduleAbs = path.resolve(ROOT, CONTROL_PLANE_MODULE_PATH);
  const testsAbs = path.resolve(ROOT, CONTROL_PLANE_TEST_PATH);
  const moduleText = fs.existsSync(moduleAbs) ? fs.readFileSync(moduleAbs, 'utf8') : '';
  const testsText = fs.existsSync(testsAbs) ? fs.readFileSync(testsAbs, 'utf8') : '';

  const missingModuleDeclarations: string[] = [];
  const missingBoundaryBindings: string[] = [];
  const missingSubdomainTests: string[] = [];
  for (const row of REQUIRED_CONTROL_PLANE_SUBDOMAINS) {
    if (!moduleText.includes(`pub mod ${row.module};`)) {
      missingModuleDeclarations.push(row.module);
    }
    if (!moduleText.includes(`${row.module}::boundary()`)) {
      missingBoundaryBindings.push(row.id);
    }
    if (!testsText.includes(`require_domain("${row.id}")`)) {
      missingSubdomainTests.push(row.id);
    }
  }

  const missingContractMarkers = REQUIRED_CONTROL_PLANE_CONTRACT_MARKERS.filter(
    (marker) => !moduleText.includes(marker),
  );
  if (!testsText.includes('control_plane_api_contract_enforces_kernel_boundary_rules')) {
    missingContractMarkers.push('control_plane_api_contract_enforces_kernel_boundary_rules');
  }

  return {
    module_path: CONTROL_PLANE_MODULE_PATH,
    tests_path: CONTROL_PLANE_TEST_PATH,
    required_subdomains: REQUIRED_CONTROL_PLANE_SUBDOMAINS.map((row) => row.id),
    missing_module_declarations: missingModuleDeclarations,
    missing_boundary_bindings: missingBoundaryBindings,
    missing_subdomain_tests: missingSubdomainTests,
    missing_contract_markers: missingContractMarkers,
    pass:
      missingModuleDeclarations.length === 0 &&
      missingBoundaryBindings.length === 0 &&
      missingSubdomainTests.length === 0 &&
      missingContractMarkers.length === 0,
  };
}

function run(args: Args): number {
  const contractPath = path.resolve(ROOT, args.contract);
  const contract = JSON.parse(fs.readFileSync(contractPath, 'utf8')) as {
    version?: string;
    bindings?: Binding[];
  };
  const bindings = Array.isArray(contract.bindings) ? contract.bindings : [];
  const mismatches: Array<{ wrapper: string; reason: string }> = [];
  const repaired: string[] = [];
  const missingBindings: string[] = [];
  const missingFiles: string[] = [];

  const bindingMap = new Map(bindings.map((row) => [row.wrapper, row]));
  for (const wrapper of scanClientSurfaceShims()) {
    if (!bindingMap.has(wrapper)) {
      missingBindings.push(wrapper);
    }
  }

  for (const binding of bindings) {
    const wrapperAbs = path.resolve(ROOT, binding.wrapper);
    const scriptAbs = path.resolve(ROOT, binding.script);
    if (!fs.existsSync(wrapperAbs) || !fs.existsSync(scriptAbs)) {
      missingFiles.push(binding.wrapper);
      continue;
    }
    const expected = wrapperTemplate(binding);
    const current = fs.readFileSync(wrapperAbs, 'utf8');
    if (normalize(current) !== normalize(expected)) {
      mismatches.push({ wrapper: binding.wrapper, reason: 'wrapper_drift' });
      if (args.write) {
        fs.writeFileSync(wrapperAbs, expected);
        repaired.push(binding.wrapper);
      }
    }
  }
  const controlPlaneContract = auditControlPlaneContract();

  const payload = {
    type: 'orchestration_boundary_contract_compile',
    generated_at: new Date().toISOString(),
    contract_path: rel(contractPath),
    contract_version: contract.version || 'unknown',
    summary: {
      bindings: bindings.length,
      missing_binding_count: missingBindings.length,
      missing_file_count: missingFiles.length,
      mismatch_count: mismatches.length,
      repaired_count: repaired.length,
      control_plane_contract_violations:
        controlPlaneContract.missing_module_declarations.length +
        controlPlaneContract.missing_boundary_bindings.length +
        controlPlaneContract.missing_subdomain_tests.length +
        controlPlaneContract.missing_contract_markers.length,
      control_plane_contract_pass: controlPlaneContract.pass,
      pass:
        missingBindings.length === 0 &&
        missingFiles.length === 0 &&
        controlPlaneContract.pass &&
        (mismatches.length === 0 || (args.write && mismatches.length === repaired.length)),
    },
    missing_bindings: missingBindings,
    missing_files: missingFiles,
    mismatches,
    repaired,
    control_plane_contract: controlPlaneContract,
  };
  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(payload, null, 2));
  if (args.strict && !payload.summary.pass) return 1;
  return 0;
}

process.exit(run(parseArgs(process.argv.slice(2))));
