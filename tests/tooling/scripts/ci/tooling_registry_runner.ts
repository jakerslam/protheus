#!/usr/bin/env node
'use strict';

import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { DEFAULT_GATE_REGISTRY_PATH, DEFAULT_VERIFY_PROFILES_PATH, collectRegistrySummary, executeGate, executeProfile } from '../../lib/runner.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type Mode = 'list' | 'gate' | 'profile';

function parseArgs(argv: string[]) {
  const mode = cleanText(argv[0] || 'list', 24).toLowerCase() as Mode;
  const common = parseStrictOutArgs(argv.slice(1), {});
  return {
    mode: mode === 'gate' || mode === 'profile' ? mode : 'list',
    id: cleanText(readFlag(argv.slice(1), 'id') || '', 160),
    registry: cleanText(readFlag(argv.slice(1), 'registry') || DEFAULT_GATE_REGISTRY_PATH, 260),
    profiles: cleanText(readFlag(argv.slice(1), 'profiles') || DEFAULT_VERIFY_PROFILES_PATH, 260),
    strict: common.strict,
    json: common.json,
    out: cleanText(common.out || '', 400),
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  if (args.mode === 'list') {
    const payload = collectRegistrySummary(args.registry, args.profiles);
    return emitStructuredResult(payload, {
      outPath: args.out || '',
      strict: args.strict,
      ok: true,
      stdout: args.json || true,
    });
  }

  if (!args.id) {
    const payload = {
      ok: false,
      type: 'tooling_registry_runner',
      generated_at: new Date().toISOString(),
      summary: { pass: false },
      failures: [{ id: 'missing_id', detail: `mode=${args.mode}` }],
      inputs: {
        mode: args.mode,
        registry_path: args.registry,
        profiles_path: args.profiles,
      },
      artifact_paths: [],
    };
    return emitStructuredResult(payload, {
      outPath: args.out || '',
      strict: args.strict,
      ok: false,
    });
  }

  const payload =
    args.mode === 'gate'
      ? executeGate(args.id, {
          registryPath: args.registry,
          strict: args.strict,
          outPath: args.out || undefined,
        })
      : executeProfile(args.id, {
          registryPath: args.registry,
          profilesPath: args.profiles,
          strict: args.strict,
          outPath: args.out || undefined,
        });
  return emitStructuredResult(payload, {
    outPath: '',
    strict: args.strict,
    ok: Boolean(payload.ok),
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};

