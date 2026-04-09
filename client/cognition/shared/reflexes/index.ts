#!/usr/bin/env node
// @ts-nocheck
'use strict';

// Layer ownership: client/cognition/shared/reflexes (authoritative)

const REFLEXES = [
  { id: 'read_snippet', description: 'Read a bounded snippet for quick context.', token_cap: 150 },
  { id: 'write_quick', description: 'Emit a compact actionable write-up.', token_cap: 150 },
  { id: 'summarize_brief', description: 'Summarize content into a short brief.', token_cap: 150 },
  { id: 'git_status', description: 'Return concise repository state.', token_cap: 150 },
  { id: 'memory_lookup', description: 'Lookup compact memory hints by query.', token_cap: 150 }
];

function parseArgs(argv = []) {
  const out = { _: [], flags: {} };
  for (const raw of Array.isArray(argv) ? argv : []) {
    const token = String(raw || '');
    if (token.startsWith('--')) {
      const body = token.slice(2);
      const eq = body.indexOf('=');
      if (eq >= 0) out.flags[body.slice(0, eq)] = body.slice(eq + 1);
      else out.flags[body] = '1';
      continue;
    }
    out._.push(token);
  }
  return out;
}

function print(value) {
  process.stdout.write(`${JSON.stringify(value, null, 2)}\n`);
}

function listReflexes() {
  return {
    ok: true,
    type: 'reflex_list',
    count: REFLEXES.length,
    reflexes: REFLEXES
  };
}

function runReflex(id, input) {
  const chosen = REFLEXES.find((row) => row.id === id);
  if (!chosen) {
    return {
      ok: false,
      type: 'reflex_run',
      error: `unknown_reflex:${id}`,
      known_ids: REFLEXES.map((row) => row.id)
    };
  }
  return {
    ok: true,
    type: 'reflex_run',
    reflex: chosen.id,
    token_cap: chosen.token_cap,
    input: String(input || ''),
    output: `[${chosen.id}] ${chosen.description}`
  };
}

function run(argv = []) {
  const parsed = parseArgs(argv);
  const cmd = String(parsed._[0] || 'list').trim().toLowerCase();
  if (cmd === 'list') {
    const result = listReflexes();
    print(result);
    return result;
  }
  if (cmd === 'run') {
    const id = String(parsed.flags.id || parsed._[1] || '').trim();
    const input = String(parsed.flags.input || '').trim();
    const result = runReflex(id, input);
    print(result);
    return result;
  }
  const result = {
    ok: false,
    type: 'reflex_command',
    error: `unsupported_command:${cmd}`,
    commands: ['list', 'run']
  };
  print(result);
  return result;
}

if (require.main === module) {
  const result = run(process.argv.slice(2));
  process.exit(result.ok ? 0 : 1);
}

module.exports = {
  REFLEXES,
  listReflexes,
  runReflex,
  run
};
