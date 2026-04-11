#!/usr/bin/env node
'use strict';

const { runProtheusOps } = require('./run_protheus_ops.ts');

function normalizeArgv(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function normalizeSubcommand(raw, fallback = 'status', aliases = {}) {
  const sub = String(raw || fallback).trim().toLowerCase() || fallback;
  return Object.prototype.hasOwnProperty.call(aliases, sub) ? String(aliases[sub]) : sub;
}

function createDomainCommandModule(spec = {}) {
  const domain = String(spec.domain || '').trim();
  const fallback = String(spec.defaultSubcommand || 'status').trim() || 'status';
  const aliases = spec.aliases && typeof spec.aliases === 'object' ? { ...spec.aliases } : {};
  const unknownDomainFallback = spec.unknownDomainFallback !== false;

  function localNormalizeSubcommand(raw) {
    return normalizeSubcommand(raw, fallback, aliases);
  }

  function run(argv = process.argv.slice(2)) {
    const args = normalizeArgv(argv);
    const sub = localNormalizeSubcommand(args[0]);
    return runProtheusOps([domain, sub, ...args.slice(1)], { unknownDomainFallback });
  }

  return { run, normalizeSubcommand: localNormalizeSubcommand };
}

function createDirectCommandModule(spec = {}) {
  const buildArgs = typeof spec.buildArgs === 'function' ? spec.buildArgs : ((argv) => argv);
  const unknownDomainFallback = spec.unknownDomainFallback === true;

  function run(argv = process.argv.slice(2)) {
    return runProtheusOps(buildArgs(normalizeArgv(argv)), { unknownDomainFallback });
  }

  return { run };
}

function createStatusDashboardModule() {
  function hasWebFlag(argv) {
    return Array.isArray(argv) && argv.some((arg) => arg === '--web' || arg === 'web');
  }

  function stripDashboardCompatFlags(argv) {
    return normalizeArgv(Array.isArray(argv) ? argv : []).filter(
      (arg) =>
        arg !== '--dashboard' &&
        arg !== 'dashboard' &&
        arg !== '--web' &&
        arg !== 'web'
    );
  }

  function runDashboardUi(argv = process.argv.slice(2)) {
    const forward = stripDashboardCompatFlags(argv);
    return runProtheusOps(['daemon-control', 'start', ...forward], { unknownDomainFallback: true });
  }

  function run(argv = process.argv.slice(2)) {
    const args = normalizeArgv(argv);
    if (hasWebFlag(args)) {
      return runDashboardUi(args);
    }
    const passthrough = args.length ? args : ['daemon-control', 'status'];
    return runProtheusOps(passthrough, { unknownDomainFallback: true });
  }

  return { run, runDashboardUi };
}

function createReplModule() {
  function run(argv = process.argv.slice(2)) {
    const args = normalizeArgv(argv);
    if (args.includes('--help') || args.includes('-h')) {
      process.stdout.write(
        'Usage: infring repl\n' +
          'Lightweight REPL bootstrap for constrained installs.\n',
      );
      return 0;
    }
    const status = runProtheusOps(['command-list-kernel', '--mode=help'], {
      unknownDomainFallback: false,
    });
    if (status === 0 && process.stdin.isTTY) {
      process.stdout.write(
        '[infring repl] interactive shell is unavailable in slim runtime; showing command index.\n',
      );
    }
    return status;
  }

  return { run };
}

function createUnknownGuardModule() {
  function isJsonMode(argv) {
    return argv.some((arg) => arg === '--json' || arg === '--json=1');
  }

  function firstUnknownCommand(argv) {
    for (const raw of argv) {
      const token = String(raw || '').trim();
      if (!token) continue;
      if (token === '--json' || token === '--json=1') continue;
      if (token === '--help' || token === '-h') continue;
      if (token.startsWith('-')) continue;
      return token;
    }
    return '';
  }

  function run(argv = process.argv.slice(2)) {
    const tokens = normalizeArgv(argv);
    const unknown = firstUnknownCommand(tokens);
    const json = isJsonMode(tokens);
    if (json) {
      process.stdout.write(
        `${JSON.stringify({
          ok: false,
          type: 'protheus_unknown_guard',
          error: 'unknown_command',
          command: unknown,
          hint: 'Run `infring help` to list available commands.',
        })}\n`,
      );
      return 2;
    }
    if (unknown) {
      process.stderr.write(`[infring] unknown command: ${unknown}\n`);
    } else {
      process.stderr.write('[infring] unknown command\n');
    }
    runProtheusOps(['command-list-kernel', '--mode=help'], {
      unknownDomainFallback: false,
    });
    return 2;
  }

  return { run };
}

const backlogGithubSync = createDomainCommandModule({
  domain: 'backlog-github-sync',
});

const backlogRegistry = createDomainCommandModule({
  domain: 'backlog-registry',
  aliases: {
    metrics: 'status',
    triage: 'status',
  },
});

const contractCheck = createDirectCommandModule({
  buildArgs(argv) {
    return argv.length ? ['contract-check', ...argv] : ['contract-check', 'status'];
  },
  unknownDomainFallback: false,
});

const protheusControlPlane = createDomainCommandModule({
  domain: 'protheus-control-plane',
  aliases: {
    audit: 'run',
    health: 'status',
    'job-submit': 'run',
  },
});

const protheusRepl = createReplModule();
const protheusStatusDashboard = createStatusDashboardModule();
const protheusUnknownGuard = createUnknownGuardModule();

const rust50MigrationProgram = createDomainCommandModule({
  domain: 'rust50-migration-program',
});

const rustEnterpriseProductivityProgram = createDomainCommandModule({
  domain: 'rust-enterprise-productivity-program',
});

const venomContainmentLayer = createDomainCommandModule({
  domain: 'venom-containment-layer',
  aliases: {
    evolve: 'evaluate',
  },
});

module.exports = {
  backlogGithubSync,
  backlogRegistry,
  contractCheck,
  createDirectCommandModule,
  createDomainCommandModule,
  createReplModule,
  createStatusDashboardModule,
  createUnknownGuardModule,
  normalizeSubcommand,
  protheusControlPlane,
  protheusRepl,
  protheusStatusDashboard,
  protheusUnknownGuard,
  rust50MigrationProgram,
  rustEnterpriseProductivityProgram,
  venomContainmentLayer,
};
