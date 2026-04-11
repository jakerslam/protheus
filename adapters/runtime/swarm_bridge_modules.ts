#!/usr/bin/env node
'use strict';

const path = require('path');
const {
  ROOT,
  invokeProtheusOpsViaBridge,
  runProtheusOps,
} = require('./run_protheus_ops.ts');

const SWARM_RUNTIME_DEFAULT_STATE_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'swarm_runtime',
  'latest.json',
);

const SWARM_ORCHESTRATION_DEFAULT_STATE_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'swarm_orchestration_runtime_latest.json',
);

const SWARM_SESSION_COMMAND_SPECS = [
  { key: 'sessionsSpawn', command: 'sessions_spawn', aliases: ['spawn'] },
  { key: 'sessionsSend', command: 'sessions_send', aliases: ['send'] },
  { key: 'sessionsReceive', command: 'sessions_receive', aliases: ['receive'] },
  { key: 'sessionsAck', command: 'sessions_ack', aliases: ['ack'] },
  { key: 'sessionsHandoff', command: 'sessions_handoff', aliases: ['handoff'] },
  { key: 'sessionsContextPut', command: 'sessions_context_put', aliases: ['context-put'] },
  { key: 'sessionsContextGet', command: 'sessions_context_get', aliases: ['context-get'] },
  { key: 'sessionsResume', command: 'sessions_resume', aliases: ['resume'] },
  { key: 'sessionsBootstrap', command: 'sessions_bootstrap', aliases: ['bootstrap'] },
  { key: 'sessionsDeadLetters', command: 'sessions_dead_letter', aliases: ['dead-letter'] },
  { key: 'sessionsRetryDeadLetter', command: 'sessions_retry_dead_letter', aliases: ['retry-dead-letter'] },
  { key: 'sessionsQuery', command: 'sessions_query', aliases: ['query'] },
  { key: 'sessionsState', command: 'sessions_state', aliases: ['state'] },
  { key: 'sessionsTick', command: 'sessions_tick', aliases: ['tick'] },
  { key: 'toolsRegisterJsonSchema', command: 'tools_register_json_schema', aliases: ['register-json-schema'] },
  { key: 'toolsInvoke', command: 'tools_invoke', aliases: ['tool-invoke'] },
  { key: 'streamEmit', command: 'stream_emit', aliases: ['stream-emit'] },
  { key: 'streamRender', command: 'stream_render', aliases: ['stream-render'] },
  { key: 'turnsRun', command: 'turns_run', aliases: ['turns-run'] },
  { key: 'turnsShow', command: 'turns_show', aliases: ['turns-show'] },
  { key: 'networksCreate', command: 'networks_create', aliases: ['networks-create'] },
  { key: 'networksStatus', command: 'networks_status', aliases: ['networks-status'] },
];

function parseCliArgs(argv = []) {
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

function normalizedOptions(options) {
  if (options && typeof options === 'object' && !Array.isArray(options)) {
    return { ...options };
  }
  return {};
}

function toOptions(parsed) {
  const out = { ...parsed };
  delete out._;
  return out;
}

function intFlag(value, fallback) {
  const parsed = Number.parseInt(String(value == null ? '' : value), 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function optionalIntFlag(value, min = 1) {
  const parsed = Number.parseInt(String(value == null ? '' : value), 10);
  if (!Number.isFinite(parsed) || parsed < min) return null;
  return parsed;
}

function optionalFloatFlag(value, min = 0, max = 1) {
  const parsed = Number.parseFloat(String(value == null ? '' : value));
  if (!Number.isFinite(parsed) || parsed < min || parsed > max) return null;
  return parsed;
}

function withState(args, parsed, defaultStatePath) {
  if (args.some((arg) => String(arg).startsWith('--state-path='))) return args;
  const explicit = String(parsed['state-path'] || parsed.state_path || '').trim();
  return args.concat(`--state-path=${explicit || defaultStatePath}`);
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

function writeBridgePayload(payload) {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

function createSwarmSessionsBridgeModule(options = {}) {
  const defaultStatePath = String(options.defaultStatePath || SWARM_RUNTIME_DEFAULT_STATE_PATH);

  function invokeBridge(command, rawOptions = {}, label = command) {
    const parsed = normalizedOptions(rawOptions);
    if (!parsed['state-path'] && !parsed.state_path) {
      parsed.state_path = defaultStatePath;
    }
    const run =
      invokeProtheusOpsViaBridge(
        [
          'swarm-sessions-bridge',
          `--command=${command}`,
          `--options-json=${JSON.stringify(parsed)}`,
        ],
        {
          allowProcessFallback: false,
          unknownDomainFallback: false,
        },
      ) || {
        status: 1,
        stdout: '',
        stderr: 'resident_ipc_bridge_unavailable',
        payload: null,
      };
    const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
    return requireBridgeOk(
      {
        status,
        stdout: String(run.stdout || ''),
        stderr: String(run.stderr || ''),
        payload:
          run && run.payload && typeof run.payload === 'object'
            ? run.payload
            : parseLastJson(run.stdout),
      },
      label,
    );
  }

  const api = {
    ROOT,
    DEFAULT_STATE_PATH: defaultStatePath,
    parseArgs: parseCliArgs,
    invokeBridge,
  };

  for (const spec of SWARM_SESSION_COMMAND_SPECS) {
    api[spec.key] = function bridgeCall(rawOptions = {}) {
      return invokeBridge(spec.command, rawOptions, spec.command);
    };
  }

  api.printUsage = function printUsage() {
    process.stdout.write(
      [
        'Usage:',
        '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts <command> [--flags...]',
        '',
        'Commands:',
        ...SWARM_SESSION_COMMAND_SPECS.map((spec) => {
          const alias = spec.aliases[0] ? ` | ${spec.aliases[0]}` : '';
          return `  ${spec.command}${alias}`;
        }),
        '',
      ].join('\n'),
    );
  };

  api.dispatch = function dispatch(command, rawOptions) {
    const normalizedCommand = String(command || '').trim().toLowerCase();
    for (const spec of SWARM_SESSION_COMMAND_SPECS) {
      if (normalizedCommand === spec.command || spec.aliases.includes(normalizedCommand)) {
        return api[spec.key](rawOptions);
      }
    }
    throw new Error(`unknown_command:${normalizedCommand}`);
  };

  api.run = function run(argv = process.argv.slice(2)) {
    const parsed = parseCliArgs(argv);
    const command = String(parsed._[0] || 'sessions_spawn').trim().toLowerCase();
    if (
      parsed.help === true ||
      parsed.h === true ||
      command === 'help' ||
      command === '--help' ||
      command === '-h'
    ) {
      api.printUsage();
      return 0;
    }
    const payload = api.dispatch(command, toOptions(parsed));
    writeBridgePayload(payload);
    return 0;
  };

  return api;
}

function bindSwarmSessionsBridgeModule(
  currentModule,
  argv = process.argv.slice(2),
  options = {},
) {
  const mod = createSwarmSessionsBridgeModule(options);
  if (currentModule && require.main === currentModule) {
    try {
      process.exit(mod.run(argv));
    } catch (err) {
      process.stderr.write(`${String((err && err.message) || err)}\n`);
      process.exit(1);
    }
  }
  return mod;
}

function createSwarmOrchestrationRuntimeModule(options = {}) {
  const defaultStatePath = String(
    options.defaultStatePath || SWARM_ORCHESTRATION_DEFAULT_STATE_PATH,
  );

  function runOps(args) {
    return runProtheusOps(args, { unknownDomainFallback: true });
  }

  function runRecursive(parsed) {
    const levels = Math.max(2, intFlag(parsed.levels || parsed.team_size, 5));
    const maxDepth = Math.max(levels + 1, intFlag(parsed['max-depth'], levels + 1));
    return runOps(
      withState(
        ['swarm-runtime', 'test', 'recursive', `--levels=${levels}`, `--max-depth=${maxDepth}`],
        parsed,
        defaultStatePath,
      ),
    );
  }

  function runByzantine(parsed) {
    const agents = Math.max(3, intFlag(parsed.agents || parsed.team_size, 5));
    const corruptDefault = Math.max(1, Math.floor(agents / 3));
    const corrupt = Math.max(1, intFlag(parsed.corrupt, corruptDefault));
    const enableStatus = runOps(
      withState(['swarm-runtime', 'byzantine-test', 'enable'], parsed, defaultStatePath),
    );
    if (enableStatus !== 0) return enableStatus;
    return runOps(
      withState(
        ['swarm-runtime', 'test', 'byzantine', `--agents=${agents}`, `--corrupt=${corrupt}`],
        parsed,
        defaultStatePath,
      ),
    );
  }

  function runCommunication(parsed) {
    const delivery = String(parsed.delivery || 'at_least_once').trim() || 'at_least_once';
    const firstAttemptFailure = String(parsed['simulate-first-attempt-fail'] || '1').trim();
    return runOps(
      withState(
        [
          'swarm-runtime',
          'test',
          'communication',
          `--delivery=${delivery}`,
          `--simulate-first-attempt-fail=${firstAttemptFailure}`,
        ],
        parsed,
        defaultStatePath,
      ),
    );
  }

  function runSpawn(parsed) {
    const objective = String(parsed.objective || parsed.task || 'generic').trim() || 'generic';
    const teamSize = Math.max(1, intFlag(parsed.team_size, 3));
    const args = [
      'swarm-runtime',
      'spawn',
      `--task=objective:${objective}`,
      '--recursive=1',
      `--levels=${Math.max(2, teamSize)}`,
      '--verify=1',
      '--metrics=detailed',
    ];

    const tokenBudget = optionalIntFlag(
      parsed['token-budget'] ?? parsed.token_budget ?? parsed['max-tokens'] ?? parsed.max_tokens,
      1,
    );
    if (tokenBudget != null) args.push(`--token-budget=${tokenBudget}`);

    const tokenWarningAt = optionalFloatFlag(
      parsed['token-warning-at'] ?? parsed.token_warning_at,
      0,
      1,
    );
    if (tokenWarningAt != null) args.push(`--token-warning-at=${tokenWarningAt}`);

    const onBudgetExhausted = String(
      parsed['on-budget-exhausted'] ?? parsed.on_budget_exhausted ?? '',
    )
      .trim()
      .toLowerCase();
    if (onBudgetExhausted === 'fail' || onBudgetExhausted === 'warn' || onBudgetExhausted === 'compact') {
      args.push(`--on-budget-exhausted=${onBudgetExhausted}`);
    }

    if (Object.prototype.hasOwnProperty.call(parsed, 'adaptive-complexity')) {
      args.push(`--adaptive-complexity=${String(parsed['adaptive-complexity'])}`);
    }

    return runOps(withState(args, parsed, defaultStatePath));
  }

  function printUsage() {
    process.stdout.write(
      [
        'Usage:',
        '  node surface/orchestration/scripts/swarm_orchestration_runtime.ts run [--objective=<name>] [--team_size=<n>] [--token-budget=<n>] [--token-warning-at=<0..1>] [--on-budget-exhausted=<fail|warn|compact>] [--adaptive-complexity=1|0] [--state-path=<path>]',
        '  node surface/orchestration/scripts/swarm_orchestration_runtime.ts test --id=<2|3|6|all> [flags]',
        '  node surface/orchestration/scripts/swarm_orchestration_runtime.ts status [--state-path=<path>]',
        '',
        'Test IDs:',
        '  2 -> recursive decomposition',
        '  3 -> byzantine fault mode',
        '  6 -> inter-agent communication',
        '  all -> runs 2, 3, 6 in sequence',
        '',
      ].join('\n'),
    );
  }

  function run(argv = process.argv.slice(2)) {
    const parsed = parseCliArgs(argv);
    const command = String(parsed._[0] || 'run').trim().toLowerCase();
    if (command === 'help' || command === '--help' || command === '-h') {
      printUsage();
      return 0;
    }
    if (command === 'status') {
      return runOps(withState(['swarm-runtime', 'status'], parsed, defaultStatePath));
    }
    if (command === 'run') return runSpawn(parsed);
    if (command === 'test') {
      const id = String(parsed.id || parsed._[1] || 'all').trim().toLowerCase();
      if (id === '2' || id === 'recursive') return runRecursive(parsed);
      if (id === '3' || id === 'byzantine') return runByzantine(parsed);
      if (id === '6' || id === 'communication') return runCommunication(parsed);
      if (id === 'all') {
        const recursiveStatus = runRecursive(parsed);
        if (recursiveStatus !== 0) return recursiveStatus;
        const byzantineStatus = runByzantine(parsed);
        if (byzantineStatus !== 0) return byzantineStatus;
        return runCommunication(parsed);
      }
      process.stderr.write(`unknown_test_id:${id}\n`);
      return 2;
    }
    if (command === 'test2') return runRecursive(parsed);
    if (command === 'test3') return runByzantine(parsed);
    if (command === 'test6') return runCommunication(parsed);
    process.stderr.write(`unknown_command:${command}\n`);
    printUsage();
    return 2;
  }

  return {
    ROOT,
    DEFAULT_STATE_PATH: defaultStatePath,
    parseArgs: parseCliArgs,
    runRecursive,
    runByzantine,
    runCommunication,
    runSpawn,
    printUsage,
    run,
  };
}

function bindSwarmOrchestrationRuntimeModule(
  currentModule,
  argv = process.argv.slice(2),
  options = {},
) {
  const mod = createSwarmOrchestrationRuntimeModule(options);
  if (currentModule && require.main === currentModule) {
    process.exit(mod.run(argv));
  }
  return mod;
}

module.exports = {
  ROOT,
  SWARM_RUNTIME_DEFAULT_STATE_PATH,
  SWARM_ORCHESTRATION_DEFAULT_STATE_PATH,
  parseCliArgs,
  parseLastJson,
  normalizedOptions,
  createSwarmSessionsBridgeModule,
  bindSwarmSessionsBridgeModule,
  createSwarmOrchestrationRuntimeModule,
  bindSwarmOrchestrationRuntimeModule,
};
