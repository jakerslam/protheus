#!/usr/bin/env node
'use strict';
export {};

type AnyObj = Record<string, any>;

type ReflexSpec = {
  id: string;
  purpose: string;
  max_tokens_est: number;
  build: (input: string) => { instruction: string, command: string | null };
};

const REFLEX_MAX_TOKENS = 150;

const REFLEX_SPECS: ReflexSpec[] = [
  {
    id: 'read_snippet',
    purpose: 'Fetch one high-signal memory snippet with no large expansion.',
    max_tokens_est: 130,
    build: (input) => ({
      instruction: `Retrieve one concise memory snippet for "${input || 'current context'}" with no full-file reads.`,
      command: `node client/systems/memory/memory_recall.js query --q="${escapeArg(input || 'current context')}" --top=1 --expand=none --context-budget-tokens=1200 --context-budget-mode=trim`
    })
  },
  {
    id: 'write_quick',
    purpose: 'Capture a minimal deterministic note payload for follow-up.',
    max_tokens_est: 120,
    build: (input) => ({
      instruction: `Capture a quick note: "${input || 'follow-up needed'}". Keep it deterministic, one sentence, no extra analysis.`,
      command: null
    })
  },
  {
    id: 'summarize_brief',
    purpose: 'Produce a 3-line concise summary from supplied context.',
    max_tokens_est: 140,
    build: (input) => ({
      instruction: `Summarize in 3 short lines: "${input || 'context'}". Include only key decision, risk, and next action.`,
      command: null
    })
  },
  {
    id: 'git_status',
    purpose: 'Return concise repository status for operator situational awareness.',
    max_tokens_est: 90,
    build: () => ({
      instruction: 'Return concise git working tree status and branch.',
      command: 'git status --short && git branch --show-current'
    })
  },
  {
    id: 'memory_lookup',
    purpose: 'Run bounded lookup for top relevant memories without heavy expansion.',
    max_tokens_est: 145,
    build: (input) => ({
      instruction: `Run bounded memory lookup for "${input || 'relevant context'}" and return top 3 IDs only.`,
      command: `node client/systems/memory/memory_recall.js query --q="${escapeArg(input || 'relevant context')}" --top=3 --expand=none --context-budget-tokens=1500 --context-budget-mode=trim`
    })
  }
];

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function escapeArg(v: string) {
  return String(v || '').replace(/"/g, '\\"');
}

function estimateTokens(text: string) {
  const chars = String(text || '').length;
  if (chars <= 0) return 0;
  return Math.max(1, Math.ceil(chars / 4));
}

function trimToTokenBudget(text: string, maxTokens: number) {
  const safe = Math.max(1, Math.round(Number(maxTokens || 1)));
  const chars = Math.max(1, Math.floor(safe * 4));
  const raw = String(text || '');
  if (raw.length <= chars) return raw;
  if (chars <= 1) return '';
  return `${raw.slice(0, chars - 1).trimEnd()}…`;
}

function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
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

function buildReflexResult(spec: ReflexSpec, inputRaw: unknown) {
  const input = cleanText(inputRaw, 200);
  const built = spec.build(input);
  const maxTokens = Math.min(REFLEX_MAX_TOKENS, Math.max(1, Number(spec.max_tokens_est || REFLEX_MAX_TOKENS)));
  const instruction = trimToTokenBudget(cleanText(built.instruction, 1200), maxTokens);
  return {
    ok: true,
    type: 'client_reflex',
    id: spec.id,
    purpose: spec.purpose,
    input,
    max_tokens_est: maxTokens,
    instruction,
    token_est: estimateTokens(instruction),
    command: built.command ? cleanText(built.command, 800) : null
  };
}

function cmdList() {
  return {
    ok: true,
    type: 'client_reflex_registry',
    count: REFLEX_SPECS.length,
    max_tokens_hard_cap: REFLEX_MAX_TOKENS,
    reflexes: REFLEX_SPECS.map((spec) => ({
      id: spec.id,
      purpose: spec.purpose,
      max_tokens_est: Math.min(REFLEX_MAX_TOKENS, Math.max(1, Number(spec.max_tokens_est || REFLEX_MAX_TOKENS)))
    }))
  };
}

function cmdRun(args: AnyObj) {
  const id = cleanText(args.id || args.reflex || args._[1] || '', 80).toLowerCase();
  const spec = REFLEX_SPECS.find((row) => row.id === id);
  if (!spec) {
    return {
      ok: false,
      error: `unknown_reflex:${id || 'missing'}`,
      available: REFLEX_SPECS.map((row) => row.id)
    };
  }
  return buildReflexResult(spec, args.input || args.q || '');
}

function usage() {
  console.log('Usage:');
  console.log('  node client/reflexes/index.js list');
  console.log('  node client/reflexes/index.js run --id=<read_snippet|write_quick|summarize_brief|git_status|memory_lookup> [--input="..."]');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'list', 40).toLowerCase();
  if (cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    return;
  }
  const payload = cmd === 'list' ? cmdList()
    : cmd === 'run' ? cmdRun(args)
    : { ok: false, error: `unknown_command:${cmd}` };
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  if (!payload.ok) process.exit(1);
}

if (require.main === module) main();

module.exports = {
  REFLEX_MAX_TOKENS,
  REFLEX_SPECS,
  estimateTokens,
  trimToTokenBudget,
  buildReflexResult,
  cmdList,
  cmdRun
};
