#!/usr/bin/env node
'use strict';
export {};

/**
 * protheusd
 * Daemon facade over protheus_control_plane with optional conduit-first routing
 * for lifecycle commands (`start`, `stop`, `status`).
 */

const path = require('path');
const { spawnSync } = require('child_process');

function usage() {
  console.log('Usage: protheusd start|stop|restart|status|tick [--policy=<path>] [--conduit]');
}

function runLegacy(command: string, extraArgs: string[]) {
  const script = path.join(__dirname, 'protheus_control_plane.js');
  const args = [script, command, ...extraArgs];
  const r = spawnSync('node', args, { encoding: 'utf8' });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  process.exit(Number.isFinite(r.status) ? r.status : 1);
}

function conduitEnabled(argv: string[]): boolean {
  if (argv.includes('--no-conduit')) return false;
  if (argv.includes('--conduit')) return true;
  return process.env.PROTHEUS_CONDUIT_ENABLED === '1';
}

function stripConduitFlags(argv: string[]): string[] {
  return argv.filter((arg) => arg !== '--conduit' && arg !== '--no-conduit');
}

function parseAgentId(args: string[]): string {
  const explicit = args.find((arg) => arg.startsWith('--agent='));
  if (explicit) return String(explicit.slice('--agent='.length) || '').trim() || 'protheus-default';
  return 'protheus-default';
}

async function runConduit(command: string, extraArgs: string[]): Promise<boolean> {
  if (!['start', 'stop', 'status'].includes(command)) {
    return false;
  }

  const { ConduitClient } = require('../conduit/conduit-client');
  const daemonCommand = process.env.CONDUIT_DAEMON_CMD || 'cargo';
  const daemonArgs = process.env.CONDUIT_DAEMON_ARGS
    ? process.env.CONDUIT_DAEMON_ARGS.split(' ').filter(Boolean)
    : ['run', '--quiet', '-p', 'conduit', '--bin', 'conduit_daemon'];

  const client = ConduitClient.overStdio(daemonCommand, daemonArgs, process.cwd());
  try {
    const requestId = `protheusd-${Date.now()}`;
    const message =
      command === 'start'
        ? { type: 'start_agent', agent_id: parseAgentId(extraArgs) }
        : command === 'stop'
          ? { type: 'stop_agent', agent_id: parseAgentId(extraArgs) }
          : { type: 'get_system_status' };

    const response = await client.send(message as any, requestId);
    process.stdout.write(`${JSON.stringify(response)}\n`);
    process.exit(response.validation.ok ? 0 : 1);
    return true;
  } catch (error: any) {
    process.stderr.write(`conduit_fallback_to_legacy:${error?.message || String(error)}\n`);
    return false;
  } finally {
    await client.close();
  }
}

async function main() {
  const argv = process.argv.slice(2);
  const cmd = String(argv[0] || 'status');
  const rest = stripConduitFlags(argv.slice(1));

  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    return;
  }

  if (conduitEnabled(argv)) {
    const routed = await runConduit(cmd, rest);
    if (routed) {
      return;
    }
  }

  if (cmd === 'tick') {
    runLegacy('job-runner', rest);
    return;
  }
  runLegacy(cmd, rest);
}

main().catch((error) => {
  process.stderr.write(`protheusd_error:${(error as Error)?.message || String(error)}\n`);
  process.exit(1);
});
