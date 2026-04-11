#!/usr/bin/env node
'use strict';

const path = require('path');
const { ROOT, createSwarmSessionsBridgeModule } = require('./swarm_bridge_modules.ts');

const bridge = createSwarmSessionsBridgeModule();
const DEFAULT_STATE_PATH = path.join(ROOT, 'local', 'state', 'ops', 'swarm_runtime', 'workflow_007_demo.json');

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

function cleanString(value) {
  return String(value == null ? '' : value).trim();
}

function statePath(parsed) {
  return cleanString(parsed['state-path'] || parsed.state_path) || DEFAULT_STATE_PATH;
}

function clone(value) {
  return value == null ? value : JSON.parse(JSON.stringify(value));
}

function getRef(refs, key) {
  const raw = cleanString(key);
  if (!raw.startsWith('$')) return raw;
  const pathParts = raw.slice(1).split('.').filter(Boolean);
  if (pathParts.length === 0) return raw;
  let cursor = refs[pathParts[0]];
  for (let i = 1; i < pathParts.length; i += 1) {
    if (cursor == null) return undefined;
    cursor = cursor[pathParts[i]];
  }
  return clone(cursor);
}

function resolveRefs(value, refs) {
  if (Array.isArray(value)) return value.map((row) => resolveRefs(row, refs));
  if (value && typeof value === 'object') {
    const out = {};
    for (const [key, inner] of Object.entries(value)) out[key] = resolveRefs(inner, refs);
    return out;
  }
  if (typeof value === 'string' && value.startsWith('$')) return getRef(refs, value);
  return value;
}

function stepRunner(step, refs, sharedStatePath) {
  const command = cleanString(step.command || step.op).toLowerCase();
  const options = resolveRefs({ ...step }, refs) || {};
  delete options.command;
  delete options.op;
  delete options.save_as;
  options.state_path = cleanString(options.state_path) || sharedStatePath;

  switch (command) {
    case 'spawn':
    case 'sessions_spawn':
      return bridge.sessionsSpawn(options);
    case 'handoff':
    case 'sessions_handoff':
      return bridge.sessionsHandoff(options);
    case 'context_put':
    case 'sessions_context_put':
      return bridge.sessionsContextPut(options);
    case 'context_get':
    case 'sessions_context_get':
      return bridge.sessionsContextGet(options);
    case 'register_tool':
    case 'tools_register_json_schema':
      return bridge.toolsRegisterJsonSchema(options);
    case 'invoke_tool':
    case 'tools_invoke':
      return bridge.toolsInvoke(options);
    case 'stream_emit':
      return bridge.streamEmit(options);
    case 'stream_render':
      return bridge.streamRender(options);
    case 'turns_run':
      return bridge.turnsRun(options);
    case 'turns_show':
      return bridge.turnsShow(options);
    case 'networks_create':
      return bridge.networksCreate(options);
    case 'networks_status':
      return bridge.networksStatus(options);
    case 'sessions_state':
    case 'state':
      return bridge.sessionsState(options);
    case 'sessions_bootstrap':
    case 'bootstrap':
      return bridge.sessionsBootstrap(options);
    case 'sessions_query':
    case 'query':
      return bridge.sessionsQuery(options);
    default:
      throw new Error(`unsupported_demo_step:${command}`);
  }
}

function runScript(options = {}) {
  const parsed = options && typeof options === 'object' ? options : {};
  const sharedStatePath = statePath(parsed);
  const steps = typeof parsed.script_json === 'string'
    ? JSON.parse(parsed.script_json)
    : (parsed.script_json || parsed.script || []);
  if (!Array.isArray(steps) || steps.length === 0) {
    throw new Error('demo_script_required');
  }
  const refs = {};
  const results = [];
  for (const step of steps) {
    const result = stepRunner(step, refs, sharedStatePath);
    results.push({ command: cleanString(step.command || step.op), result });
    const saveAs = cleanString(step.save_as || step.saveAs);
    if (saveAs) refs[saveAs] = result;
  }
  return {
    ok: true,
    type: 'swarm_repl_demo_script',
    state_path: sharedStatePath,
    refs,
    results,
  };
}

function fullDemoScript() {
  return [
    {
      command: 'spawn',
      task: 'workflow-007 coordinator',
      agentLabel: 'demo-coordinator',
      max_tokens: 1024,
      on_budget_exhausted: 'fail',
      save_as: 'coordinator',
    },
    {
      command: 'spawn',
      task: 'workflow-007 specialist',
      session_id: '$coordinator.session_id',
      agentLabel: 'demo-specialist',
      max_tokens: 192,
      on_budget_exhausted: 'fail',
      save_as: 'specialist',
    },
    {
      command: 'context_put',
      session_id: '$coordinator.session_id',
      context: {
        objective: 'workflow-007 demo',
        brief: 'delegate tool-backed analysis to the specialist',
        long_note: 'x'.repeat(9000),
      },
      save_as: 'context_receipt',
    },
    {
      command: 'handoff',
      session_id: '$coordinator.session_id',
      target_session_id: '$specialist.session_id',
      reason: 'delegate specialist analysis',
      importance: 0.8,
      context: {
        delegated_goal: 'produce governed answer',
        owner: 'demo-coordinator',
      },
      save_as: 'handoff',
    },
    {
      command: 'register_tool',
      session_id: '$specialist.session_id',
      toolName: 'context_patch',
      bridgePath: 'adapters/runtime/swarm_bridge_modules.ts',
      entrypoint: 'sessions_context_put',
      schema: {
        type: 'object',
        properties: {
          context: { type: 'object' },
          merge: { type: 'boolean' },
        },
        required: ['context'],
      },
      description: 'Patch specialist context through the governed bridge.',
      save_as: 'tool_manifest',
    },
    {
      command: 'invoke_tool',
      session_id: '$specialist.session_id',
      toolName: 'context_patch',
      args: {
        context: {
          tool_applied: true,
          tool_source: 'demo-shell',
        },
        merge: true,
      },
      save_as: 'tool_result',
    },
    {
      command: 'turns_run',
      session_id: '$specialist.session_id',
      label: 'demo-loop',
      turns: [
        {
          message: 'draft governed answer',
          fail_first_attempt: true,
          recovery: 'retry_once',
        },
        {
          tool_name: 'context_patch',
          tool_args: {
            context: {
              recovered: true,
              final_status: 'ready',
            },
            merge: true,
          },
        },
      ],
      save_as: 'turn_run',
    },
    {
      command: 'networks_create',
      session_id: '$coordinator.session_id',
      spec: {
        name: 'workflow-007-demo-network',
        nodes: [
          { label: 'planner', role: 'planner', task: 'plan answer', context: { lane: 'plan' } },
          { label: 'executor', role: 'executor', task: 'execute answer', context: { lane: 'execute' } },
        ],
        edges: [
          {
            from: 'planner',
            to: 'executor',
            relation: 'handoff',
            importance: 0.75,
            auto_handoff: true,
            reason: 'planner_to_executor',
          },
        ],
      },
      save_as: 'network',
    },
    {
      command: 'state',
      session_id: '$specialist.session_id',
      save_as: 'specialist_state',
    },
  ];
}

function demo(options = {}) {
  const parsed = options && typeof options === 'object' ? options : {};
  const kind = cleanString(parsed.kind || 'full').toLowerCase();
  if (kind !== 'full') throw new Error(`unsupported_demo_kind:${kind}`);
  const payload = runScript({ ...parsed, script: fullDemoScript() });
  const refs = payload.refs || {};
  const compactResults = Array.isArray(payload.results)
    ? payload.results.map((row) => ({
        command: cleanString(row && row.command),
        type: row && row.result && row.result.type ? row.result.type : null,
        ok: row && row.result && typeof row.result.ok === 'boolean' ? row.result.ok : true,
      }))
    : [];
  return {
    ok: true,
    type: 'swarm_repl_demo',
    kind,
    state_path: payload.state_path,
    summary: {
      coordinator_session_id: refs.coordinator && refs.coordinator.session_id,
      specialist_session_id: refs.specialist && refs.specialist.session_id,
      handoff_id:
        refs.handoff
        && refs.handoff.payload
        && refs.handoff.payload.handoff
        && refs.handoff.payload.handoff.handoff_id,
      tool_manifest_id:
        refs.tool_manifest
        && refs.tool_manifest.payload
        && refs.tool_manifest.payload.tool_manifest
        && refs.tool_manifest.payload.tool_manifest.manifest_id,
      run_id:
        refs.turn_run
        && refs.turn_run.payload
        && refs.turn_run.payload.run
        && refs.turn_run.payload.run.run_id,
      network_id:
        refs.network
        && refs.network.payload
        && refs.network.payload.network
        && refs.network.payload.network.network_id,
      context_mode:
        refs.specialist_state
        && refs.specialist_state.payload
        && refs.specialist_state.payload.session
        && refs.specialist_state.payload.session.context
        && refs.specialist_state.payload.session.context.mode,
    },
    results: compactResults,
    result_count: compactResults.length,
  };
}

function printUsage() {
  process.stdout.write(
    [
      'Usage:',
      '  node client/runtime/systems/autonomy/swarm_repl_demo.ts demo [--kind=full] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_repl_demo.ts script --script-json=<json-array> [--state-path=<path>]',
      '',
      'This shell is optional and non-authoritative. Every step delegates to swarm bridge adapters.',
    ].join('\n') + '\n'
  );
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = cleanString(parsed._[0] || 'help').toLowerCase();
  let payload;
  if (command === 'help' || command === '--help' || command === '-h') {
    printUsage();
    return 0;
  }
  if (command === 'demo') payload = demo(parsed);
  else if (command === 'script') payload = runScript(parsed);
  else throw new Error(`unsupported_command:${command}`);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  return 0;
}

module.exports = {
  DEFAULT_STATE_PATH,
  demo,
  fullDemoScript,
  run,
  runScript,
};
