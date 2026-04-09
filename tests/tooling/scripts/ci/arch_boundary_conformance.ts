#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

type Args = {
  strict: boolean;
  out: string;
  policy: string;
};

type Violation = {
  file: string;
  reason: string;
  detail: string;
};

const ROOT = process.cwd();

function parseArgs(argv: string[]): Args {
  const args: Args = {
    strict: false,
    out: 'core/local/artifacts/arch_boundary_conformance_current.json',
    policy: 'client/runtime/config/arch_boundary_conformance_policy.json',
  };
  for (const token of argv) {
    if (token === '--strict') args.strict = true;
    else if (token.startsWith('--strict=')) {
      const value = token.slice('--strict='.length).toLowerCase();
      args.strict = value === '1' || value === 'true' || value === 'yes' || value === 'on';
    } else if (token.startsWith('--out=')) {
      args.out = token.slice('--out='.length);
    } else if (token.startsWith('--policy=')) {
      args.policy = token.slice('--policy='.length);
    }
  }
  return args;
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function walk(base: string, exts: Set<string>): string[] {
  if (!fs.existsSync(base)) return [];
  const out: string[] = [];
  const stack = [base];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const abs = path.join(current, entry.name);
      const rp = rel(abs);
      if (rp.includes('/node_modules/') || rp.includes('/dist/') || rp.includes('/target/')) {
        continue;
      }
      if (entry.isDirectory()) {
        stack.push(abs);
      } else if (entry.isFile() && exts.has(path.extname(entry.name))) {
        out.push(abs);
      }
    }
  }
  return out.sort();
}

function importSpecs(source: string): string[] {
  const specs: string[] = [];
  const re = /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null = null;
  while ((match = re.exec(source)) != null) {
    specs.push(match[1]);
  }
  return specs;
}

function run(args: Args): number {
  const violations: Violation[] = [];
  const clientFiles = walk(path.join(ROOT, 'client'), new Set(['.ts', '.tsx']));
  const coreFiles = walk(path.join(ROOT, 'core'), new Set(['.rs', '.ts']));
  const orchestrationCargo = path.join(ROOT, 'surface', 'orchestration', 'Cargo.toml');
  const surfaceFiles = walk(path.join(ROOT, 'surface', 'orchestration', 'src'), new Set(['.rs']));

  for (const file of clientFiles) {
    const source = fs.readFileSync(file, 'utf8');
    for (const spec of importSpecs(source)) {
      if (
        spec.includes('core/') ||
        spec.startsWith('../core') ||
        spec.startsWith('../../core') ||
        spec.startsWith('/core/')
      ) {
        violations.push({
          file: rel(file),
          reason: 'client_imports_core_forbidden',
          detail: spec,
        });
      }
      if (
        spec.includes('surface/orchestration/') ||
        spec.startsWith('../surface') ||
        spec.startsWith('../../surface')
      ) {
        violations.push({
          file: rel(file),
          reason: 'client_imports_orchestration_internals_forbidden',
          detail: spec,
        });
      }
    }
  }

  for (const file of coreFiles) {
    const source = fs.readFileSync(file, 'utf8');
    if (source.includes('infring_orchestration_surface_v1')) {
      violations.push({
        file: rel(file),
        reason: 'core_depends_on_orchestration_forbidden',
        detail: 'detected reference to infring_orchestration_surface_v1',
      });
    }
    if (source.includes("surface/orchestration")) {
      violations.push({
        file: rel(file),
        reason: 'core_references_orchestration_path_forbidden',
        detail: 'detected path reference surface/orchestration',
      });
    }
  }

  if (fs.existsSync(orchestrationCargo)) {
    const cargoSource = fs.readFileSync(orchestrationCargo, 'utf8');
    const allowedCoreContracts = new Set([
      'infring-layer1-memory',
      'infring-task-fabric-core-v1',
      'protheus-tooling-core-v1',
    ]);
    const dependencyLine =
      /^([a-zA-Z0-9_-]+)\s*=\s*\{[^}]*path\s*=\s*"([^"]+)"[^}]*\}/gm;
    let match: RegExpExecArray | null = null;
    while ((match = dependencyLine.exec(cargoSource)) != null) {
      const depName = match[1];
      const depPath = match[2].replace(/\\/g, '/');
      if (depPath.includes('/core/') || depPath.startsWith('../../core/')) {
        if (!allowedCoreContracts.has(depName)) {
          violations.push({
            file: rel(orchestrationCargo),
            reason: 'orchestration_depends_on_core_internal_forbidden',
            detail: `${depName} -> ${depPath}`,
          });
        }
      }
    }
  }

  for (const file of surfaceFiles) {
    const source = fs.readFileSync(file, 'utf8');
    if (source.includes("core/layer")) {
      violations.push({
        file: rel(file),
        reason: 'orchestration_source_references_core_internal_paths_forbidden',
        detail: 'detected core/layer path literal',
      });
    }
  }

  const report = {
    policy_path: args.policy,
    type: 'arch_boundary_conformance',
    generated_at: new Date().toISOString(),
    strict: args.strict,
    summary: {
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    violations,
  };

  const policyPath = path.resolve(ROOT, args.policy);
  let allowedViolations: Violation[] = [];
  let hardViolations = violations.slice();
  if (fs.existsSync(policyPath)) {
    const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as {
      allowed_violations?: Array<{ file?: string; reason?: string; detail_contains?: string }>;
    };
    const allow = Array.isArray(policy.allowed_violations) ? policy.allowed_violations : [];
    hardViolations = [];
    for (const violation of violations) {
      const isAllowed = allow.some((rule) => {
        const fileOk = !rule.file || rule.file === violation.file;
        const reasonOk = !rule.reason || rule.reason === violation.reason;
        const detailOk =
          !rule.detail_contains || violation.detail.includes(String(rule.detail_contains));
        return fileOk && reasonOk && detailOk;
      });
      if (isAllowed) {
        allowedViolations.push(violation);
      } else {
        hardViolations.push(violation);
      }
    }
  }

  report.summary = {
    violation_count: violations.length,
    allowed_violation_count: allowedViolations.length,
    hard_violation_count: hardViolations.length,
    pass: hardViolations.length === 0,
  };
  (report as any).allowed_violations = allowedViolations;
  report.violations = hardViolations;

  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));

  if (args.strict && hardViolations.length > 0) {
    return 1;
  }
  return 0;
}

const exitCode = run(parseArgs(process.argv.slice(2)));
process.exit(exitCode);
