'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function repoRoot(scriptDir) {
  return path.resolve(scriptDir, '..', '..');
}

function parseJsonPayload(stdout) {
  const raw = String(stdout || '').trim();
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {}
  const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line || line[0] !== '{') continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

function normalizeStatus(v) {
  return Number.isFinite(Number(v)) ? Number(v) : 1;
}

function defaultEnv() {
  return {
    ...process.env,
    PROTHEUS_NODE_BINARY: process.execPath || 'node'
  };
}

function resolveRustBinary(root, candidateEnvVar, defaultBin) {
  const explicit = String(process.env[candidateEnvVar] || '').trim();
  if (explicit && fs.existsSync(explicit)) return explicit;
  const _unused = { root, defaultBin };
  void _unused;
  return null;
}

function runBridge(config, args = [], cliMode = false) {
  const root = repoRoot(config.scriptDir);
  const passArgs = Array.isArray(args) ? args.slice(0) : [];

  let command;
  let commandArgs;

  if (config.binaryName && config.manifestPath) {
    const binary = resolveRustBinary(
      root,
      config.binaryEnvVar || 'PROTHEUS_RUST_LANE_BIN',
      config.binaryName
    );
    if (binary) {
      command = binary;
      commandArgs = config.preArgs
        ? config.preArgs.concat(passArgs)
        : passArgs;
    } else {
      command = 'cargo';
      commandArgs = [
        'run',
        '--quiet',
        '--manifest-path',
        config.manifestPath,
        '--bin',
        config.binaryName,
        '--',
        ...(config.preArgs || []),
        ...passArgs
      ];
    }
  } else {
    throw new Error('invalid_rust_lane_bridge_config');
  }

  const run = spawnSync(command, commandArgs, {
    cwd: root,
    encoding: 'utf8',
    env: defaultEnv(),
    stdio: cliMode && config.inheritStdio ? 'inherit' : undefined
  });

  const status = normalizeStatus(run.status);
  const stdout = run.stdout || '';
  const stderr = run.stderr || '';
  const payload = cliMode && config.inheritStdio ? null : parseJsonPayload(stdout);

  return {
    ok: status === 0,
    status,
    stdout,
    stderr,
    payload,
    lane: config.lane,
    rust_command: command,
    rust_args: commandArgs
  };
}

function createOpsLaneBridge(scriptDir, lane, domain, opts = {}) {
  const config = {
    scriptDir,
    lane,
    manifestPath: 'crates/ops/Cargo.toml',
    binaryName: 'protheus-ops',
    binaryEnvVar: 'PROTHEUS_OPS_BIN',
    preArgs: [domain],
    inheritStdio: opts.inheritStdio === true
  };

  function run(args = []) {
    return runBridge(config, args, false);
  }

  function runCli(args = []) {
    const out = runBridge(config, args, config.inheritStdio === true);
    if (!config.inheritStdio) {
      if (out.stdout) process.stdout.write(out.stdout);
      if (out.stderr) process.stderr.write(out.stderr);
    }
    process.exit(out.status);
  }

  return {
    lane,
    run,
    runCli
  };
}

function createManifestLaneBridge(scriptDir, lane, options) {
  const config = {
    scriptDir,
    lane,
    manifestPath: options.manifestPath,
    binaryName: options.binaryName,
    binaryEnvVar: options.binaryEnvVar,
    preArgs: options.preArgs || [],
    inheritStdio: options.inheritStdio === true
  };

  function run(args = []) {
    return runBridge(config, args, false);
  }

  function runCli(args = []) {
    const out = runBridge(config, args, config.inheritStdio === true);
    if (!config.inheritStdio) {
      if (out.stdout) process.stdout.write(out.stdout);
      if (out.stderr) process.stderr.write(out.stderr);
    }
    process.exit(out.status);
  }

  return {
    lane,
    run,
    runCli
  };
}

module.exports = {
  createOpsLaneBridge,
  createManifestLaneBridge
};
