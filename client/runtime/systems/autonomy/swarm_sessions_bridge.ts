#!/usr/bin/env node
'use strict';

// Layer ownership: client/runtime/systems/autonomy (thin wrapper over core/layer0/ops).

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const OPS_WRAPPER = path.join(
  ROOT,
  'client',
  'runtime',
  'systems',
  'ops',
  'run_protheus_ops.js'
);
const DEFAULT_STATE_PATH = path.join(ROOT, 'local', 'state', 'ops', 'swarm_runtime', 'latest.json');

function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '');
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx >= 0) {
      out[token.slice(2, idx)] = token.slice(idx + 1);
      continue;
    }
    const key = token.slice(2);
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

function normalizedOptions(options) {
  if (options && typeof options === 'object' && !Array.isArray(options)) return options;
  return {};
}

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

function execOps(args, env = {}) {
  const run = spawnSync(process.execPath, [OPS_WRAPPER].concat(args), {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env, ...env },
  });
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  return {
    status,
    stdout: String(run.stdout || ''),
    stderr: String(run.stderr || ''),
    payload: parseLastJson(run.stdout),
  };
}

function requireBridgeOk(result, label) {
  if (result.status !== 0) {
    const reason =
      (result.payload && result.payload.error) ||
      result.stderr ||
      result.stdout ||
      `${label}_failed`;
    throw new Error(`${label}_failed:status=${result.status}:${reason}`);
  }
  if (!result.payload || result.payload.ok !== true) {
    throw new Error(`${label}_invalid_payload`);
  }
  return result.payload;
}

function invokeBridge(command, options = {}, label = command) {
  const parsed = normalizedOptions(options);
  if (!parsed['state-path'] && !parsed.state_path) {
    parsed.state_path = parsed.state_path || DEFAULT_STATE_PATH;
  }
  const run = execOps([
    'swarm-sessions-bridge',
    `--command=${command}`,
    `--options-json=${JSON.stringify(parsed)}`,
  ]);
  return requireBridgeOk(run, label);
}

function sessionsSpawn(options = {}) {
  return invokeBridge('sessions_spawn', options, 'sessions_spawn');
}

function sessionsSend(options = {}) {
  return invokeBridge('sessions_send', options, 'sessions_send');
}

function sessionsReceive(options = {}) {
  return invokeBridge('sessions_receive', options, 'sessions_receive');
}

function sessionsAck(options = {}) {
  return invokeBridge('sessions_ack', options, 'sessions_ack');
}

function sessionsHandoff(options = {}) {
  return invokeBridge('sessions_handoff', options, 'sessions_handoff');
}

function sessionsContextPut(options = {}) {
  return invokeBridge('sessions_context_put', options, 'sessions_context_put');
}

function sessionsContextGet(options = {}) {
  return invokeBridge('sessions_context_get', options, 'sessions_context_get');
}

function sessionsResume(options = {}) {
  return invokeBridge('sessions_resume', options, 'sessions_resume');
}

function sessionsBootstrap(options = {}) {
  return invokeBridge('sessions_bootstrap', options, 'sessions_bootstrap');
}

function sessionsDeadLetters(options = {}) {
  return invokeBridge('sessions_dead_letter', options, 'sessions_dead_letter');
}

function sessionsRetryDeadLetter(options = {}) {
  return invokeBridge('sessions_retry_dead_letter', options, 'sessions_retry_dead_letter');
}

function sessionsQuery(options = {}) {
  return invokeBridge('sessions_query', options, 'sessions_query');
}

function sessionsState(options = {}) {
  return invokeBridge('sessions_state', options, 'sessions_state');
}

function sessionsTick(options = {}) {
  return invokeBridge('sessions_tick', options, 'sessions_tick');
}

function toolsRegisterJsonSchema(options = {}) {
  return invokeBridge('tools_register_json_schema', options, 'tools_register_json_schema');
}

function toolsInvoke(options = {}) {
  return invokeBridge('tools_invoke', options, 'tools_invoke');
}

function streamEmit(options = {}) {
  return invokeBridge('stream_emit', options, 'stream_emit');
}

function streamRender(options = {}) {
  return invokeBridge('stream_render', options, 'stream_render');
}

function turnsRun(options = {}) {
  return invokeBridge('turns_run', options, 'turns_run');
}

function turnsShow(options = {}) {
  return invokeBridge('turns_show', options, 'turns_show');
}

function networksCreate(options = {}) {
  return invokeBridge('networks_create', options, 'networks_create');
}

function networksStatus(options = {}) {
  return invokeBridge('networks_status', options, 'networks_status');
}

function printUsage() {
  process.stdout.write(
    [
      'Usage:',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts <command> [--flags...]',
      '',
      'Commands:',
      '  sessions_spawn | spawn',
      '  sessions_send | send',
      '  sessions_receive | receive',
      '  sessions_ack | ack',
      '  sessions_handoff | handoff',
      '  sessions_context_put | context-put',
      '  sessions_context_get | context-get',
      '  sessions_resume | resume',
      '  sessions_bootstrap | bootstrap',
      '  sessions_dead_letter | dead-letter',
      '  sessions_retry_dead_letter | retry-dead-letter',
      '  sessions_query | query',
      '  sessions_state | state',
      '  sessions_tick | tick',
      '  tools_register_json_schema | register-json-schema',
      '  tools_invoke | tool-invoke',
      '  stream_emit | stream-emit',
      '  stream_render | stream-render',
      '  turns_run | turns-run',
      '  turns_show | turns-show',
      '  networks_create | networks-create',
      '  networks_status | networks-status',
      '',
    ].join('\n')
  );
}

function toOptions(parsed) {
  const out = { ...parsed };
  delete out._;
  return out;
}

function dispatch(command, options) {
  if (command === 'sessions_spawn' || command === 'spawn') return sessionsSpawn(options);
  if (command === 'sessions_send' || command === 'send') return sessionsSend(options);
  if (command === 'sessions_receive' || command === 'receive') return sessionsReceive(options);
  if (command === 'sessions_ack' || command === 'ack') return sessionsAck(options);
  if (command === 'sessions_handoff' || command === 'handoff') return sessionsHandoff(options);
  if (command === 'sessions_context_put' || command === 'context-put') return sessionsContextPut(options);
  if (command === 'sessions_context_get' || command === 'context-get') return sessionsContextGet(options);
  if (command === 'sessions_resume' || command === 'resume') return sessionsResume(options);
  if (command === 'sessions_bootstrap' || command === 'bootstrap') return sessionsBootstrap(options);
  if (command === 'sessions_dead_letter' || command === 'dead-letter') return sessionsDeadLetters(options);
  if (command === 'sessions_retry_dead_letter' || command === 'retry-dead-letter') return sessionsRetryDeadLetter(options);
  if (command === 'sessions_query' || command === 'query') return sessionsQuery(options);
  if (command === 'sessions_state' || command === 'state') return sessionsState(options);
  if (command === 'sessions_tick' || command === 'tick') return sessionsTick(options);
  if (command === 'tools_register_json_schema' || command === 'register-json-schema') return toolsRegisterJsonSchema(options);
  if (command === 'tools_invoke' || command === 'tool-invoke') return toolsInvoke(options);
  if (command === 'stream_emit' || command === 'stream-emit') return streamEmit(options);
  if (command === 'stream_render' || command === 'stream-render') return streamRender(options);
  if (command === 'turns_run' || command === 'turns-run') return turnsRun(options);
  if (command === 'turns_show' || command === 'turns-show') return turnsShow(options);
  if (command === 'networks_create' || command === 'networks-create') return networksCreate(options);
  if (command === 'networks_status' || command === 'networks-status') return networksStatus(options);
  throw new Error(`unknown_command:${command}`);
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed._[0] || 'sessions_spawn').trim().toLowerCase();

  if (
    parsed.help === true
    || parsed.h === true
    || command === 'help'
    || command === '--help'
    || command === '-h'
  ) {
    printUsage();
    return 0;
  }

  const payload = dispatch(command, toOptions(parsed));
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  return 0;
}

if (require.main === module) {
  try {
    process.exit(run(process.argv.slice(2)));
  } catch (err) {
    process.stderr.write(`${String((err && err.message) || err)}\n`);
    process.exit(1);
  }
}

module.exports = {
  ROOT,
  DEFAULT_STATE_PATH,
  parseArgs,
  sessionsSpawn,
  sessionsSend,
  sessionsReceive,
  sessionsAck,
  sessionsHandoff,
  sessionsContextPut,
  sessionsContextGet,
  sessionsResume,
  sessionsBootstrap,
  sessionsDeadLetters,
  sessionsRetryDeadLetter,
  sessionsQuery,
  sessionsState,
  sessionsTick,
  toolsRegisterJsonSchema,
  toolsInvoke,
  streamEmit,
  streamRender,
  turnsRun,
  turnsShow,
  networksCreate,
  networksStatus,
  run,
};
