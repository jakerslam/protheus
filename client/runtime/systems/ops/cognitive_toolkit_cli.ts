#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops + core/layer1/memory_runtime (authoritative)
// Thin TypeScript compatibility bridge for the public `toolkit` CLI surface.
const { runProtheusOps } = require('./run_protheus_ops.ts');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

const TOOLKIT_SURFACES = [
  'personas',
  'dictionary',
  'orchestration',
  'blob-morphing',
  'comment-mapper',
  'assimilate',
  'research',
  'web'
];

const DICTIONARY = {
  'binary blobs':
    'Opaque binary payloads that flow through deterministic ingestion, routing, and receipt checks.',
  conduit:
    'The deterministic messaging substrate that routes events between runtime planes with explicit contracts.',
  'attention queue':
    'Priority-aware queue that surfaces relevant context and execution work to active agents.',
  'verity plane':
    'Truth-fidelity and receipt-validation authority for assertions, provenance, and reconciliation.'
};

function normalizeArgs(argv) {
  if (!Array.isArray(argv)) return [];
  return argv.map((token) => String(token || '').trim()).filter(Boolean);
}

function jsonMode(args) {
  return args.some((arg) => arg === '--json' || arg === '--json=1');
}

function withoutJsonFlags(args) {
  return args.filter((arg) => arg !== '--json' && arg !== '--json=1');
}

function emit(payload, asJson) {
  if (asJson) {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return;
  }
  if (Array.isArray(payload.surfaces)) {
    process.stdout.write('Toolkit surfaces:\n');
    for (const item of payload.surfaces) {
      process.stdout.write(`  - ${item}\n`);
    }
    return;
  }
  if (payload.term && payload.definition) {
    process.stdout.write(`${payload.term}: ${payload.definition}\n`);
    return;
  }
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

function runDictionary(rawArgs, asJson) {
  const args = withoutJsonFlags(rawArgs);
  const sub = String(args[0] || 'list').toLowerCase();
  if (sub === 'list') {
    emit(
      {
        ok: true,
        type: 'toolkit_dictionary_list',
        terms: Object.keys(DICTIONARY).map((key) =>
          key
            .split(' ')
            .map((row) => row.slice(0, 1).toUpperCase() + row.slice(1))
            .join(' ')
        )
      },
      asJson
    );
    return 0;
  }

  if (sub === 'term' || sub === 'define') {
    const rawTerm = args.slice(1).join(' ').trim();
    if (!rawTerm) {
      emit(
        {
          ok: false,
          type: 'toolkit_dictionary_error',
          error: 'term_required'
        },
        true
      );
      return 1;
    }
    const key = rawTerm.toLowerCase();
    const definition = DICTIONARY[key];
    if (!definition) {
      emit(
        {
          ok: false,
          type: 'toolkit_dictionary_error',
          error: 'term_not_found',
          term: rawTerm
        },
        true
      );
      return 1;
    }
    emit(
      {
        ok: true,
        type: 'toolkit_dictionary_term',
        term: rawTerm,
        definition
      },
      asJson
    );
    return 0;
  }

  emit(
    {
      ok: false,
      type: 'toolkit_dictionary_error',
      error: 'unknown_subcommand',
      subcommand: sub
    },
    true
  );
  return 1;
}

function readFlag(args, key) {
  const prefix = `--${key}=`;
  const hit = args.find((arg) => arg.startsWith(prefix));
  if (!hit) return '';
  return hit.slice(prefix.length).trim();
}

function runCommentMapper(rawArgs, asJson) {
  const args = withoutJsonFlags(rawArgs);
  const persona = readFlag(args, 'persona') || 'default';
  const query = readFlag(args, 'query') || '';
  const gap = readFlag(args, 'gap') || '1';
  const active = readFlag(args, 'active') || '0';
  emit(
    {
      ok: true,
      type: 'toolkit_comment_mapper',
      persona,
      query,
      gap_seconds: Number(gap) || 1,
      active: active === '1' || active.toLowerCase() === 'true',
      mode: 'compat_bridge'
    },
    asJson
  );
  return 0;
}

function runBlobMorphing(rawArgs, asJson) {
  const args = withoutJsonFlags(rawArgs);
  const sub = String(args[0] || 'status').toLowerCase();
  if (sub !== 'status' && sub !== 'verify') {
    emit(
      {
        ok: false,
        type: 'toolkit_blob_morphing_error',
        error: 'unknown_subcommand',
        subcommand: sub
      },
      true
    );
    return 1;
  }
  emit(
    {
      ok: true,
      type: 'toolkit_blob_morphing_status',
      status: 'ready',
      mode: 'compat_bridge',
      supported: ['status', 'verify']
    },
    asJson
  );
  return 0;
}

function listPersonaIds() {
  const dir = path.join(ROOT, 'client', 'cognition', 'personas');
  let rows = [];
  try {
    rows = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  return rows
    .filter((row) => row && row.isDirectory && row.isDirectory())
    .map((row) => String(row.name || '').trim())
    .filter(Boolean)
    .sort((a, b) => a.localeCompare(b));
}

function runPersonas(rawArgs, asJson) {
  const args = withoutJsonFlags(rawArgs);
  const sub = String(args[0] || 'list').toLowerCase();
  const personas = listPersonaIds();
  if (sub === '--list' || sub === 'list' || sub === 'status') {
    emit(
      {
        ok: true,
        type: 'toolkit_personas',
        personas,
        count: personas.length,
        mode: 'compat_bridge'
      },
      asJson
    );
    return 0;
  }

  emit(
    {
      ok: false,
      type: 'toolkit_personas_error',
      error: 'unsupported_subcommand',
      subcommand: sub
    },
    true
  );
  return 1;
}

function runOrchestration(rawArgs, asJson) {
  const args = withoutJsonFlags(rawArgs);
  const sub = String(args[0] || 'status').toLowerCase();
  if (sub === 'status') {
    emit(
      {
        ok: true,
        type: 'toolkit_orchestration_status',
        mode: 'compat_bridge',
        queue_depth: 0,
        active_sessions: 0
      },
      asJson
    );
    return 0;
  }
  emit(
    {
      ok: false,
      type: 'toolkit_orchestration_error',
      error: 'unsupported_subcommand',
      subcommand: sub
    },
    true
  );
  return 1;
}

function dispatchBridgeSurface(surface, domain, rawArgs, asJson) {
  const args = withoutJsonFlags(rawArgs);
  const dispatchOnly = args.includes('--dispatch-only') || args.includes('--dispatch-only=1');
  const passthrough = args.filter((arg) => arg !== '--dispatch-only' && arg !== '--dispatch-only=1');
  if (dispatchOnly) {
    emit(
      {
        ok: true,
        type: 'toolkit_dispatch_preview',
        surface,
        domain,
        args: passthrough
      },
      asJson || true
    );
    return 0;
  }
  return runProtheusOps([domain, ...passthrough], {
    unknownDomainFallback: false,
    allowProcessFallback: false
  });
}

function printUsage() {
  process.stdout.write('Usage: infring toolkit <surface> [args]\n');
  process.stdout.write('Aliases: web-tooling -> web\n');
  process.stdout.write('Tips: add --dispatch-only to preview routed domain/args.\n');
  process.stdout.write('Surfaces:\n');
  for (const surface of TOOLKIT_SURFACES) {
    process.stdout.write(`  - ${surface}\n`);
  }
}

function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const asJson = jsonMode(args);
  const cmd = String(args[0] || 'list').toLowerCase();
  const rest = args.slice(1);

  if (cmd === 'list' || cmd === 'status') {
    emit(
      {
        ok: true,
        type: 'toolkit_list',
        surfaces: TOOLKIT_SURFACES
      },
      asJson
    );
    return 0;
  }

  if (cmd === 'dictionary') {
    return runDictionary(rest, asJson);
  }

  if (cmd === 'comment-mapper') {
    return runCommentMapper(rest, asJson);
  }

  if (cmd === 'blob-morphing') {
    return runBlobMorphing(rest, asJson);
  }

  if (cmd === 'personas') {
    return runPersonas(rest, asJson);
  }

  if (cmd === 'orchestration') {
    return runOrchestration(rest, asJson);
  }

  if (cmd === 'assimilate') {
    return dispatchBridgeSurface('assimilate', 'assimilate', rest, asJson);
  }

  if (cmd === 'research') {
    return dispatchBridgeSurface('research', 'research', rest, asJson);
  }

  if (cmd === 'web' || cmd === 'web-tooling') {
    return dispatchBridgeSurface('web', 'web-search', rest, asJson);
  }

  printUsage();
  return 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
