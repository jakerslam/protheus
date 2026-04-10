#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

type Args = {
  strict: boolean;
  out: string;
};

type GateResult = {
  id: string;
  path: string;
  required: boolean;
  present: boolean;
  pass: boolean;
  detail: string;
};

const ROOT = process.cwd();

const REQUIRED_GATES = [
  {
    id: 'arch_boundary_conformance',
    path: 'core/local/artifacts/arch_boundary_conformance_current.json',
    required: true,
  },
  {
    id: 'orchestration_boundary_contract_compile',
    path: 'core/local/artifacts/orchestration_boundary_contract_compile_current.json',
    required: true,
  },
  {
    id: 'orchestration_hidden_state_guard',
    path: 'core/local/artifacts/orchestration_hidden_state_guard_current.json',
    required: true,
  },
  {
    id: 'debt_expiry_guard',
    path: 'core/local/artifacts/debt_expiry_guard_current.json',
    required: true,
  },
];

function parseArgs(argv: string[]): Args {
  const out: Args = {
    strict: false,
    out: 'core/local/artifacts/system_fitness_gate_current.json',
  };
  for (const arg of argv) {
    if (arg === '--strict' || arg === '--strict=1') out.strict = true;
    else if (arg.startsWith('--strict=')) {
      const value = arg.slice('--strict='.length).trim().toLowerCase();
      out.strict = value === '1' || value === 'true' || value === 'yes' || value === 'on';
    } else if (arg.startsWith('--out=')) {
      out.out = arg.slice('--out='.length).trim() || out.out;
    }
  }
  return out;
}

function readJson(absPath: string): any {
  return JSON.parse(fs.readFileSync(absPath, 'utf8'));
}

function summarizeGate(id: string, relPath: string, required: boolean): GateResult {
  const abs = path.resolve(ROOT, relPath);
  if (!fs.existsSync(abs)) {
    return {
      id,
      path: relPath,
      required,
      present: false,
      pass: false,
      detail: 'missing_artifact',
    };
  }
  let parsed: any = null;
  try {
    parsed = readJson(abs);
  } catch (err) {
    return {
      id,
      path: relPath,
      required,
      present: true,
      pass: false,
      detail: `invalid_json:${String(err)}`,
    };
  }
  const pass = Boolean(parsed?.summary?.pass === true);
  const violationCount = Number(parsed?.summary?.violation_count ?? parsed?.summary?.hard_violation_count ?? 0);
  return {
    id,
    path: relPath,
    required,
    present: true,
    pass,
    detail: pass ? 'ok' : `violations:${violationCount}`,
  };
}

function codexProgress(root: string): { done: number; queued: number } | null {
  const pathCandidates = [
    'local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.json',
    'local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.tsv',
  ];
  for (const relPath of pathCandidates) {
    const abs = path.resolve(root, relPath);
    if (!fs.existsSync(abs) || !abs.endsWith('.json')) continue;
    try {
      const rows = JSON.parse(fs.readFileSync(abs, 'utf8'));
      if (!Array.isArray(rows)) continue;
      const done = rows.filter((row: any) => String(row?.status || '').toLowerCase() === 'done').length;
      const queued = rows.filter((row: any) => String(row?.status || '').toLowerCase() === 'queued').length;
      return { done, queued };
    } catch {
      continue;
    }
  }
  return null;
}

function run(args: Args): number {
  const gates = REQUIRED_GATES.map((gate) => summarizeGate(gate.id, gate.path, gate.required));
  const missingRequired = gates.filter((gate) => gate.required && !gate.present);
  const failedRequired = gates.filter((gate) => gate.required && gate.present && !gate.pass);

  let score = 100;
  score -= missingRequired.length * 25;
  score -= failedRequired.length * 25;
  score = Math.max(0, score);

  const codex = codexProgress(ROOT);
  const payload = {
    type: 'system_fitness_gate',
    generated_at: new Date().toISOString(),
    summary: {
      score,
      required_gate_count: REQUIRED_GATES.length,
      missing_required_count: missingRequired.length,
      failed_required_count: failedRequired.length,
      pass: missingRequired.length === 0 && failedRequired.length === 0,
    },
    gates,
    codex_progress: codex,
  };

  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(payload, null, 2));

  if (args.strict && !payload.summary.pass) return 1;
  return 0;
}

process.exit(run(parseArgs(process.argv.slice(2))));
