#!/usr/bin/env node
'use strict';
export {};

/**
 * protheusctl
 * Typed control client façade over protheus_control_plane.
 */

const path = require('path');
const { spawnSync } = require('child_process');
const perceptionLayer = require('./perception_layer.js');

function usage() {
  console.log('Usage: protheusctl <command> [flags]');
  console.log('Examples:');
  console.log('  protheus status');
  console.log('  protheus health');
  console.log('  protheusctl job-submit --kind=reconcile');
  console.log('  protheusctl rsi bootstrap --owner=jay');
  console.log('  protheusctl rsi step --owner=jay --target-path=systems/strategy/strategy_learner.ts');
  console.log('  protheusctl contract-lane status --owner=jay');
  console.log('  protheusctl edge start --owner=jay --profile=mobile_seed --remote-spine=https://host');
  console.log('  protheusctl edge lifecycle run --owner=jay --battery=62 --thermal=39');
  console.log('  protheusctl edge swarm enroll --owner=jay --device-id=phone_01 --provenance-attested=1');
  console.log('  protheusctl edge wrapper build --owner=jay --target=android_termux --version=0.1.0');
  console.log('  protheusctl edge benchmark run --owner=jay --scenario=ci_mobile_android --target=android');
  console.log('  protheusctl host detect');
  console.log('  protheusctl host adapt --dry-run=1');
  console.log('  protheusctl socket list');
  console.log('  protheusctl socket admission');
  console.log('  protheusctl mine dashboard --human=1');
  console.log('  protheusctl migrate --to=<org/repo|url> [--workspace=<path>] [--apply=1]');
  console.log('  protheusctl import --from=<engine> --path=<source> [--apply=1]');
  console.log('  protheusctl wasi2 run|status');
  console.log('  protheusctl rust run|report|status');
  console.log('  protheusctl rust-hybrid list|run|run-all|status');
  console.log('  protheusctl settle [--revert=1]|list|run|run-all|status|edit-core|edit-module');
  console.log('  protheusctl scale list|run|run-all|status');
  console.log('  protheusctl perception list|run|run-all|status');
  console.log('  protheusctl fluxlattice list|run|run-all|status');
  console.log('  protheusctl lensmap init|template add|simplify|polish|import|sync|expose|status');
  console.log('  protheus lens <persona> "<query>"');
  console.log('  protheusctl hold admit|rehydrate|simulate|status');
  console.log('  protheusctl suite list|run|run-all|status');
  console.log('  protheusctl audit illusion --strict=1');
  console.log('  protheusctl approve --rsi --owner=jay --approver=<you>');
}

function runScript(script: string, args: string[] = []) {
  const r = spawnSync('node', [script, ...args], { encoding: 'utf8' });
  if (r.stdout) process.stdout.write(r.stdout);
  if (r.stderr) process.stderr.write(r.stderr);
  process.exit(Number.isFinite(r.status) ? r.status : 1);
}

function parseJson(text: string) {
  const raw = String(text || '').trim();
  if (!raw) return null;
  try { return JSON.parse(raw); } catch {}
  const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try { return JSON.parse(lines[i]); } catch {}
  }
  return null;
}

function runScriptCapture(script: string, args: string[] = []) {
  const r = spawnSync('node', [script, ...args], { encoding: 'utf8' });
  return {
    status: Number.isFinite(r.status) ? Number(r.status) : 1,
    stdout: String(r.stdout || ''),
    stderr: String(r.stderr || ''),
    payload: parseJson(String(r.stdout || ''))
  };
}

function printWithEpilogue(result: any, epilogue: string | null) {
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (epilogue) {
    process.stdout.write(`${epilogue}\n`);
  }
  process.exit(result.status);
}

function hasHumanFlag(args: string[]) {
  return args.some((arg) => arg === '--human' || arg === '--human=1' || arg === '--format=human');
}

function printEdgeHuman(payload: any, scope: string) {
  if (!payload || typeof payload !== 'object') return false;
  if (scope === 'runtime' && payload.edge_session) {
    const s = payload.edge_session;
    console.log(`edge active=${s.active ? 'yes' : 'no'} owner=${s.owner_id || 'none'} profile=${s.profile || 'none'} online=${s.online ? 'yes' : 'no'}`);
    console.log(`sync=${s.last_sync_at || 'never'} rollback_count=${Number(s.rollback_count || 0)} cache_snapshots=${Number(payload.cache_snapshots || 0)}`);
    return true;
  }
  if (scope === 'lifecycle' && payload.lifecycle) {
    const s = payload.lifecycle;
    console.log(`lifecycle action=${s.action || 'unknown'} mode=${s.mode || 'unknown'} battery=${s.battery_pct != null ? s.battery_pct : 'n/a'} thermal=${s.thermal_c != null ? s.thermal_c : 'n/a'}`);
    console.log(`doze=${s.doze_mode ? 'yes' : 'no'} background_kills=${Number(s.background_kills || 0)} survives_72h=${s.survives_72h_target ? 'yes' : 'no'}`);
    return true;
  }
  if (scope === 'swarm' && typeof payload.enrolled_nodes !== 'undefined') {
    console.log(`swarm enrolled=${Number(payload.enrolled_nodes || 0)} active=${Number(payload.active_nodes || 0)} quarantined=${Number(payload.quarantined_nodes || 0)} evicted=${Number(payload.evicted_nodes || 0)}`);
    return true;
  }
  return false;
}

function routeEdge(rest: string[]) {
  const subcmd = String(rest[0] || 'status').trim().toLowerCase();
  const human = hasHumanFlag(rest);
  const stripHuman = (argv: string[]) => argv.filter((arg) => arg !== '--human' && arg !== '--human=1' && arg !== '--format=human');

  let script = path.join(__dirname, '..', 'edge', 'protheus_edge_runtime.js');
  let args = [subcmd, ...rest.slice(1)];
  let scope = 'runtime';

  if (subcmd === 'lifecycle') {
    script = path.join(__dirname, '..', 'edge', 'mobile_lifecycle_resilience.js');
    const action = String(rest[1] || 'status').trim().toLowerCase() || 'status';
    args = [action, ...rest.slice(2)];
    scope = 'lifecycle';
  } else if (subcmd === 'swarm') {
    script = path.join(__dirname, '..', 'spawn', 'mobile_edge_swarm_bridge.js');
    const action = String(rest[1] || 'status').trim().toLowerCase() || 'status';
    args = [action, ...rest.slice(2)];
    scope = 'swarm';
  } else if (subcmd === 'wrapper') {
    script = path.join(__dirname, 'mobile_wrapper_distribution_pack.js');
    const action = String(rest[1] || 'status').trim().toLowerCase() || 'status';
    args = [action, ...rest.slice(2)];
    scope = 'wrapper';
  } else if (subcmd === 'benchmark') {
    script = path.join(__dirname, 'mobile_competitive_benchmark_matrix.js');
    const action = String(rest[1] || 'status').trim().toLowerCase() || 'status';
    args = [action, ...rest.slice(2)];
    scope = 'benchmark';
  } else if (subcmd === 'top') {
    script = path.join(__dirname, '..', 'edge', 'mobile_ops_top.js');
    args = ['status', ...rest.slice(1)];
    scope = 'top';
  }

  const cleanArgs = stripHuman(args);
  if (!human) {
    runScript(script, cleanArgs);
    return;
  }
  const result = runScriptCapture(script, cleanArgs);
  if (result.stderr) process.stderr.write(result.stderr);
  const printed = printEdgeHuman(result.payload, scope);
  if (!printed && result.stdout) process.stdout.write(result.stdout);
  process.exit(result.status);
}

function main() {
  const argv = process.argv.slice(2);
  const cmd = String(argv[0] || 'status');
  const rest = argv.slice(1);
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    return;
  }

  if (cmd === 'status') {
    const script = path.join(__dirname, 'protheus_control_plane.js');
    const result = runScriptCapture(script, ['status', ...rest]);
    const flags = perceptionLayer.loadPerceptionFlags();
    const settledPanel = perceptionLayer.loadSettledPanel();
    const epilogue = perceptionLayer.buildStatusEpilogue(flags, settledPanel);
    printWithEpilogue(result, epilogue);
    return;
  }

  if (cmd === 'skills' && String(rest[0] || '') === 'discover') {
    const discoverScript = path.join(__dirname, 'protheusctl_skills_discover.js');
    runScript(discoverScript, rest.slice(1));
    return;
  }

  if (cmd === 'edge') {
    routeEdge(rest);
    return;
  }

  if (cmd === 'host') {
    const hostScript = path.join(__dirname, 'host_adaptation_operator_surface.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    runScript(hostScript, [sub, ...rest.slice(1)]);
    return;
  }

  if (cmd === 'socket') {
    const socketScript = path.join(__dirname, 'platform_socket_runtime.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'list') {
      runScript(socketScript, ['lifecycle', 'list', ...rest.slice(1)]);
      return;
    }
    if (sub === 'install' || sub === 'update' || sub === 'test') {
      runScript(socketScript, ['lifecycle', sub, ...rest.slice(1)]);
      return;
    }
    if (sub === 'admission' || sub === 'discover' || sub === 'activate' || sub === 'status') {
      runScript(socketScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(socketScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'mine') {
    const mineScript = path.join(__dirname, '..', 'economy', 'donor_mining_dashboard.js');
    const sub = String(rest[0] || 'dashboard').trim().toLowerCase() || 'dashboard';
    runScript(mineScript, [sub, ...rest.slice(1)]);
    return;
  }

  if (cmd === 'migrate') {
    const migrationScript = path.join(__dirname, '..', 'migration', 'core_migration_bridge.js');
    const sub = String(rest[0] || '').trim().toLowerCase();
    const supported = new Set(['run', 'status', 'rollback', 'help', '--help', '-h']);
    if (!sub || sub.startsWith('--') || !supported.has(sub)) {
      runScript(migrationScript, ['run', ...rest]);
      return;
    }
    if (sub === 'help' || sub === '--help' || sub === '-h') {
      runScript(migrationScript, ['help']);
      return;
    }
    runScript(migrationScript, [sub, ...rest.slice(1)]);
    return;
  }

  if (cmd === 'import') {
    const importerScript = path.join(__dirname, '..', 'migration', 'universal_importers.js');
    const sub = String(rest[0] || '').trim().toLowerCase();
    const supported = new Set(['run', 'status', 'help', '--help', '-h']);
    if (!sub || sub.startsWith('--') || !supported.has(sub)) {
      runScript(importerScript, ['run', ...rest]);
      return;
    }
    if (sub === 'help' || sub === '--help' || sub === '-h') {
      runScript(importerScript, ['help']);
      return;
    }
    runScript(importerScript, [sub, ...rest.slice(1)]);
    return;
  }

  if (cmd === 'wasi2') {
    const wasi2Script = path.join(__dirname, 'wasi2_execution_completeness_gate.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'run') {
      runScript(wasi2Script, ['run', ...rest.slice(1)]);
      return;
    }
    runScript(wasi2Script, ['status', ...rest.slice(1)]);
    return;
  }

  if (cmd === 'settle') {
    const settleScript = path.join(__dirname, 'settlement_program.js');
    let sub = String(rest[0] || '').trim().toLowerCase();
    const hasRevertFlag = rest.includes('--revert') || rest.includes('--revert=1') || rest.includes('--mode=revert');
    if (hasRevertFlag) sub = 'revert';
    const supported = new Set(['list', 'run', 'run-all', 'status', 'settle', 'revert', 'edit-core', 'edit-module', 'edit']);
    const scriptArgs = (!sub || sub.startsWith('--') || !supported.has(sub))
      ? ['settle', ...rest]
      : [sub, ...rest.slice(1)];
    const result = runScriptCapture(settleScript, scriptArgs);
    const flags = perceptionLayer.loadPerceptionFlags();
    const settledPanel = perceptionLayer.loadSettledPanel();
    const epilogue = perceptionLayer.buildStatusEpilogue(flags, settledPanel);
    printWithEpilogue(result, epilogue);
    return;
  }

  if (cmd === 'edit-core') {
    const settleScript = path.join(__dirname, 'settlement_program.js');
    const result = runScriptCapture(settleScript, ['edit-core', ...rest]);
    const flags = perceptionLayer.loadPerceptionFlags();
    const settledPanel = perceptionLayer.loadSettledPanel();
    const epilogue = perceptionLayer.buildStatusEpilogue(flags, settledPanel);
    printWithEpilogue(result, epilogue);
    return;
  }

  if (cmd === 'edit') {
    const settleScript = path.join(__dirname, 'settlement_program.js');
    const args = rest.length ? ['edit-module', ...rest] : ['edit-module'];
    const result = runScriptCapture(settleScript, args);
    const flags = perceptionLayer.loadPerceptionFlags();
    const settledPanel = perceptionLayer.loadSettledPanel();
    const epilogue = perceptionLayer.buildStatusEpilogue(flags, settledPanel);
    printWithEpilogue(result, epilogue);
    return;
  }

  if (cmd === 'scale') {
    const scaleScript = path.join(__dirname, 'scale_readiness_program.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'list' || sub === 'run' || sub === 'run-all' || sub === 'status') {
      runScript(scaleScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(scaleScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'perception') {
    const perceptionScript = path.join(__dirname, 'perception_polish_program.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'list' || sub === 'run' || sub === 'run-all' || sub === 'status') {
      runScript(perceptionScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(perceptionScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'fluxlattice') {
    const fluxScript = path.join(__dirname, 'fluxlattice_program.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'list' || sub === 'run' || sub === 'run-all' || sub === 'status') {
      runScript(fluxScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(fluxScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'lensmap') {
    const lensScript = path.join(__dirname, '..', '..', 'packages', 'lensmap', 'lensmap_cli.js');
    runScript(lensScript, rest);
    return;
  }

  if (cmd === 'lens') {
    const personaScript = path.join(__dirname, '..', 'personas', 'cli.js');
    runScript(personaScript, rest);
    return;
  }

  if (cmd === 'hold') {
    const holdScript = path.join(__dirname, '..', 'autonomy', 'hold_remediation_engine.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'admit' || sub === 'rehydrate' || sub === 'simulate' || sub === 'status') {
      runScript(holdScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(holdScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'rust') {
    const rustScript = path.join(__dirname, 'rust_authoritative_microkernel_acceleration.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'run' || sub === 'report' || sub === 'status') {
      runScript(rustScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(rustScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'rust-hybrid') {
    const hybridScript = path.join(__dirname, 'rust_hybrid_migration_program.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'list' || sub === 'run' || sub === 'run-all' || sub === 'status') {
      runScript(hybridScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(hybridScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'suite') {
    const suiteScript = path.join(__dirname, 'productized_suite_program.js');
    const sub = String(rest[0] || 'status').trim().toLowerCase() || 'status';
    if (sub === 'list' || sub === 'run' || sub === 'run-all' || sub === 'status') {
      runScript(suiteScript, [sub, ...rest.slice(1)]);
      return;
    }
    runScript(suiteScript, ['status', ...rest]);
    return;
  }

  if (cmd === 'rsi') {
    const rsiScript = path.join(__dirname, '..', '..', 'adaptive', 'rsi', 'rsi_bootstrap.js');
    const subcmd = String(rest[0] || 'status');
    runScript(rsiScript, [subcmd, ...rest.slice(1)]);
    return;
  }

  if (cmd === 'contract-lane' && String(rest[0] || '') === 'status') {
    const rsiScript = path.join(__dirname, '..', '..', 'adaptive', 'rsi', 'rsi_bootstrap.js');
    runScript(rsiScript, ['contract-lane-status', ...rest.slice(1)]);
    return;
  }

  if (cmd === 'approve' && rest.includes('--rsi')) {
    const rsiScript = path.join(__dirname, '..', '..', 'adaptive', 'rsi', 'rsi_bootstrap.js');
    runScript(rsiScript, ['approve', ...rest.filter((arg) => arg !== '--rsi')]);
    return;
  }

  const script = path.join(__dirname, 'protheus_control_plane.js');
  runScript(script, [cmd, ...rest]);
}

main();
