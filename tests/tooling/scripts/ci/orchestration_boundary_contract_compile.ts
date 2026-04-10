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

const ROOT = process.cwd();

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
      pass:
        missingBindings.length === 0 &&
        missingFiles.length === 0 &&
        (mismatches.length === 0 || (args.write && mismatches.length === repaired.length)),
    },
    missing_bindings: missingBindings,
    missing_files: missingFiles,
    mismatches,
    repaired,
  };
  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(payload, null, 2));
  if (args.strict && !payload.summary.pass) return 1;
  return 0;
}

process.exit(run(parseArgs(process.argv.slice(2))));
