#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::command-list-kernel (authoritative)
// Thin TypeScript launcher wrapper only.
const { runProtheusOps } = require('./run_protheus_ops.ts');
const DEFAULT_ARGS = ['--mode=help'];

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function resolveArgs(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  return args.length > 0 ? args : DEFAULT_ARGS.slice(0);
}

function resolveRuntimeMode(env = process.env) {
  const raw = String(
    (env && (env.INFRING_INSTALL_MODE || env.INFRING_RUNTIME_MODE || env.PROTHEUS_RUNTIME_MODE)) || ''
  )
    .trim()
    .toLowerCase();
  if (['full', 'minimal', 'pure', 'tiny-max'].includes(raw)) return raw;
  const tinyMax = String((env && env.INFRING_TINY_MAX_MODE) || '').trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(tinyMax)) return 'tiny-max';
  const pure = String((env && env.INFRING_PURE_MODE) || '').trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(pure)) return 'pure';
  return 'full';
}

function shouldEmitModeHelpHints(args = []) {
  const normalized = normalizeArgs(args).map((row) => row.toLowerCase());
  const jsonMode = normalized.includes('--json') || normalized.includes('--json=1');
  if (jsonMode) return false;
  const helpMode = normalized.length === 0 || normalized.includes('--mode=help');
  return helpMode;
}

function emitModeHelpHints(mode) {
  const normalized = String(mode || '').trim().toLowerCase();
  if (!['pure', 'tiny-max', 'minimal'].includes(normalized)) return;
  process.stderr.write(`[infring help] mode contract: ${normalized}\n`);
  if (normalized === 'pure' || normalized === 'tiny-max') {
    process.stderr.write(
      '[infring help] capability note: dashboard-ui/client bundle commands may be unavailable in rust-only mode.\n'
    );
  }
  if (normalized === 'tiny-max') {
    process.stderr.write(
      '[infring help] capability note: tiny-max profile prioritizes minimal footprint; optional surfaces are disabled by design.\n'
    );
  }
  if (normalized === 'minimal') {
    process.stderr.write(
      '[infring help] capability note: minimal profile may require `infring setup` before optional onboarding surfaces appear.\n'
    );
  }
}

function run(argv = process.argv.slice(2)): number {
  const resolvedArgs = resolveArgs(argv);
  if (shouldEmitModeHelpHints(resolvedArgs)) {
    emitModeHelpHints(resolveRuntimeMode(process.env));
  }
  return runProtheusOps(['command-list-kernel', ...resolvedArgs], {
    env: {
      PROTHEUS_OPS_USE_PREBUILT: process.env.PROTHEUS_OPS_USE_PREBUILT || '0',
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000'
    },
    unknownDomainFallback: false
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  DEFAULT_ARGS,
  emitModeHelpHints,
  normalizeArgs,
  resolveRuntimeMode,
  resolveArgs,
  run,
  shouldEmitModeHelpHints,
};
