#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import readline from 'node:readline';
import { ShellSocketGatewayClient } from '../socket/client/shell_socket_gateway_client.ts';
import { terminalNodeFetch } from './terminal_node_fetch.ts';
import { pollTerminalReplyProjection, selectTerminalDefaultAgent, submitTerminalUserInput, terminalSessionProjection } from './terminal_reply_projection.ts';
import { renderTerminalBlocks } from './terminal_output_renderer.ts';

const DEFAULT_GATEWAY_URL = 'http://127.0.0.1:5173';
const DEFAULT_OUT_JSON = 'core/local/artifacts/terminal_shell_response_test_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/TERMINAL_SHELL_RESPONSE_TEST_CURRENT.md';
const DEFAULT_INTERACTIVE_OUT_JSON = 'core/local/artifacts/terminal_shell_interactive_smoke_current.json';
const DEFAULT_INTERACTIVE_OUT_MARKDOWN = 'local/workspace/reports/TERMINAL_SHELL_INTERACTIVE_SMOKE_CURRENT.md';
const TERMINAL_PROMPT = 'infring>';

type FetchImpl = (input: string, init?: Record<string, unknown>) => Promise<any>;
type Writer = { write: (chunk: string) => unknown };

export type TerminalShellOptions = { baseUrl?: string; fetchImpl?: FetchImpl };
export type TerminalShellMode = 'fixture' | 'live';
export type TerminalShellStopReason = 'ctrl_z' | 'scripted_input_complete' | 'stdin_closed';
export type TerminalShellResponse = {
  ok: boolean; type: 'terminal_shell_response_test'; mode: TerminalShellMode; base_url: string;
  socket_capability: 'get_runtime_status'; state: string; label: string; receipt_ref: string;
  response_preview: string; error?: string;
};
export type TerminalShellInteractiveResult = {
  ok: boolean; type: 'terminal_shell_interactive_session'; mode: TerminalShellMode; base_url: string;
  selected_agent_id: string; selected_agent_label: string; turns: number; accepted_count: number;
  rejected_count: number; stopped_by: TerminalShellStopReason; transcript_preview: string; error?: string;
};

function clean(value: unknown, max = 400): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function readFlag(argv: string[], name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  for (let index = 0; index < argv.length; index += 1) {
    const token = clean(argv[index], 1200);
    if (token === `--${name}`) return clean(argv[index + 1], 1200);
    if (token.startsWith(prefix)) return clean(token.slice(prefix.length), 1200);
  }
  return fallback;
}

function parseBool(value: string, fallback = false): boolean {
  const normalized = clean(value, 32).toLowerCase();
  if (!normalized) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(normalized);
}

function writeJson(filePath: string, payload: unknown): void {
  const abs = path.resolve(process.cwd(), filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeMarkdownLines(filePath: string, lines: string[]): void {
  const abs = path.resolve(process.cwd(), filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${lines.join('\n')}\n`, 'utf8');
}

function writeMarkdown(filePath: string, response: TerminalShellResponse): void {
  const lines = ['# Terminal Shell Response Test', '', `ok: \`${response.ok}\``, `mode: \`${response.mode}\``, `base_url: \`${response.base_url}\``, `capability: \`${response.socket_capability}\``, `state: \`${response.state}\``, `label: \`${response.label}\``, `receipt_ref: \`${response.receipt_ref}\``];
  if (response.error) lines.push(`error: \`${response.error}\``);
  writeMarkdownLines(filePath, lines);
}

function writeInteractiveMarkdown(filePath: string, result: TerminalShellInteractiveResult): void {
  const lines = ['# Terminal Shell Interactive Smoke', '', `ok: \`${result.ok}\``, `mode: \`${result.mode}\``, `base_url: \`${result.base_url}\``, `selected_agent_id: \`${result.selected_agent_id}\``, `turns: \`${result.turns}\``, `accepted_count: \`${result.accepted_count}\``, `rejected_count: \`${result.rejected_count}\``, `stopped_by: \`${result.stopped_by}\``];
  if (result.error) lines.push(`error: \`${result.error}\``);
  writeMarkdownLines(filePath, lines);
}

function renderTerminalResponse(response: TerminalShellResponse): string {
  const label = response.ok ? response.label : response.error || response.label;
  return renderTerminalBlocks([
    { kind: 'header', title: 'Infring', state: response.state || 'unknown', agent: 'Gateway', session: response.mode },
    { kind: 'assistant', label: 'Gateway', text: label },
  ]);
}

function fixtureFetch(): FetchImpl {
  return async (input: string, init?: Record<string, unknown>) => {
    const pathname = new URL(input, 'http://terminal-shell.fixture').pathname;
    const method = clean(init?.method || 'GET', 20).toUpperCase();
    const ok =
      (method === 'GET' && pathname === '/api/shell-socket/runtime-status') ||
      (method === 'GET' && pathname === '/api/shell-socket/agents') ||
      (method === 'GET' && pathname === '/api/shell-socket/agents/misty/sessions') ||
      (method === 'GET' && pathname.startsWith('/api/shell-socket/sessions/misty%3A%3Adefault/messages')) ||
      (method === 'POST' && pathname === '/api/shell-socket/input');
    let payload: Record<string, unknown>;
    if (method === 'GET' && pathname === '/api/shell-socket/runtime-status') {
      payload = {
        state: 'ready',
        label: 'Terminal Shell fixture connected through Shell Socket',
        source: 'terminal_shell_fixture',
        receipt_ref: 'receipt:terminal-shell:fixture-runtime-status',
        correlation_id: 'terminal-shell.fixture.runtime-status',
      };
    } else if (method === 'GET' && pathname === '/api/shell-socket/agents') {
      payload = {
        agents: [{ id: 'misty', name: 'Misty', state: 'ready' }],
        agent_ids: ['misty'],
        active_agent_id: 'misty',
        labels: { misty: 'Misty' },
        status_counts: { ready: 1 },
        last_activity_preview: { misty: 'Fixture agent' },
        receipt_ref: 'receipt:terminal-shell:fixture-agents',
        correlation_id: 'terminal-shell.fixture.agents',
      };
    } else if (method === 'GET' && pathname === '/api/shell-socket/agents/misty/sessions') {
      payload = { active_session_id: 'misty::default', message_counts: { 'misty::default': 1 }, receipt_ref: 'receipt:terminal-shell:fixture-sessions' };
    } else if (method === 'GET' && pathname.startsWith('/api/shell-socket/sessions/misty%3A%3Adefault/messages')) {
      payload = { session_id: 'misty::default', total_count: 2, message_window: { rows: [{ id: 'message-2', role: 'assistant', text: 'Fixture reply received through Shell Socket.' }] } };
    } else if (method === 'POST' && pathname === '/api/shell-socket/input') {
      const body = typeof init?.body === 'string' ? JSON.parse(init.body) : {};
      payload = {
        accepted: Boolean(clean(body.agent_id, 120) && clean(body.message, 24000)),
        rejected: !clean(body.agent_id, 120) || !clean(body.message, 24000),
        reason_code: clean(body.agent_id, 120) && clean(body.message, 24000) ? 'accepted' : 'agent_id_and_message_required',
        receipt_ref: 'receipt:terminal-shell:fixture-submit-input',
        follow_up_ref: 'follow_up:terminal-shell:fixture-submit-input',
        correlation_id: 'terminal-shell.fixture.submit-input',
      };
    } else {
      payload = { error: 'terminal_shell_fixture_unknown_route', path: pathname };
    }
    return { ok, status: ok ? 200 : 404, text: async () => JSON.stringify(payload) };
  };
}

function writeBlock(output: Writer, blocks: Parameters<typeof renderTerminalBlocks>[0]): void {
  output.write(`${renderTerminalBlocks(blocks, { prompt: TERMINAL_PROMPT })}\n\n`);
}

function askLine(rl: readline.Interface, prompt: string): Promise<string | null> {
  return new Promise((resolve) => {
    let resolved = false;
    const onClose = () => {
      if (resolved) return;
      resolved = true;
      resolve(null);
    };
    rl.once('close', onClose);
    rl.question(`${prompt} `, (answer) => {
      if (resolved) return;
      resolved = true;
      rl.removeListener('close', onClose);
      resolve(answer);
    });
  });
}

export class TerminalShell {
  private readonly baseUrl: string;
  private readonly client: ShellSocketGatewayClient;

  constructor(options: TerminalShellOptions = {}) {
    this.baseUrl = clean(options.baseUrl || DEFAULT_GATEWAY_URL, 300);
    this.client = new ShellSocketGatewayClient({ baseUrl: this.baseUrl, fetchImpl: options.fetchImpl || terminalNodeFetch });
  }

  async responseTest(mode: 'fixture' | 'live'): Promise<TerminalShellResponse> {
    try {
      const status = (await this.client.getRuntimeStatus<Record<string, unknown>>()) || {};
      const state = clean(status.state || 'unknown', 80);
      const label = clean(status.label || 'Runtime status response received.', 180);
      const receipt = clean(status.receipt_ref || '', 240);
      return {
        ok: Boolean(state && receipt),
        type: 'terminal_shell_response_test',
        mode,
        base_url: this.baseUrl,
        socket_capability: 'get_runtime_status',
        state,
        label,
        receipt_ref: receipt,
        response_preview: `${state}: ${label}`,
      };
    } catch (error) {
      return {
        ok: false,
        type: 'terminal_shell_response_test',
        mode,
        base_url: this.baseUrl,
        socket_capability: 'get_runtime_status',
        state: 'unavailable',
        label: 'Terminal Shell did not receive a Shell Socket response.',
        receipt_ref: '',
        response_preview: 'unavailable',
        error: clean(error instanceof Error ? error.message : error, 300),
      };
    }
  }

  async renderAgentRoster(output: Writer) {
    const selection = await selectTerminalDefaultAgent(this.client);
    const label = selection.agentId
      ? `Selected ${selection.label} (${selection.agentId}) from ${selection.count || 1} Gateway agent row(s).`
      : `No Gateway agent is selected yet.${selection.error ? ` ${selection.error}` : ''}`;
    writeBlock(output, [{ kind: 'assistant', label: 'Gateway', text: label }]);
    return selection;
  }

  async startInteractive(options: {
    mode: 'fixture' | 'live';
    requireLive?: boolean;
    input?: any;
    output?: Writer;
    scriptedInputs?: string[];
  }): Promise<TerminalShellInteractiveResult> {
    const output = options.output || process.stdout;
    const transcript: string[] = [];
    const tee: Writer = {
      write: (chunk: string) => {
        transcript.push(chunk);
        output.write(chunk);
      },
    };
    const status = await this.responseTest(options.mode);
    let selection = await selectTerminalDefaultAgent(this.client);
    writeBlock(tee, [
      { kind: 'header', title: 'Infring Terminal Shell', state: status.state, agent: selection.label || 'Gateway', session: options.mode },
      { kind: 'assistant', label: 'Gateway', text: status.ok ? status.label : status.error || status.label },
      {
        kind: 'meta',
        text: `Commands: /status, /agents, /use <agent_id>, /help. Stop with Ctrl-Z.`,
      },
    ]);
    if (!status.ok && options.requireLive) {
      return {
        ok: false,
        type: 'terminal_shell_interactive_session',
        mode: options.mode,
        base_url: this.baseUrl,
        selected_agent_id: selection.agentId,
        selected_agent_label: selection.label,
        turns: 0,
        accepted_count: 0,
        rejected_count: 1,
        stopped_by: 'scripted_input_complete',
        transcript_preview: clean(transcript.join(''), 2000),
        error: status.error,
      };
    }
    if (selection.agentId) {
      writeBlock(tee, [{ kind: 'assistant', label: 'Gateway', text: `Selected ${selection.label} (${selection.agentId}).` }]);
    } else {
      writeBlock(tee, [{ kind: 'assistant', label: 'Gateway', text: 'No agent selected. Use /agents to refresh available agents.' }]);
    }

    let turns = 0;
    let acceptedCount = 0;
    let rejectedCount = 0;
    let stoppedBy: TerminalShellInteractiveResult['stopped_by'] = 'scripted_input_complete';

    const processLine = async (raw: string | null): Promise<boolean> => {
      if (raw == null) {
        stoppedBy = 'stdin_closed';
        return false;
      }
      const line = clean(raw, 24000);
      if (!line) return true;
      turns += 1;
      if (line === '/help') {
        writeBlock(tee, [
          {
            kind: 'assistant',
            label: 'Gateway',
            text: 'Type a message to send it through submit_input. Use /agents to refresh, /use <agent_id> to target a different agent, and Ctrl-Z to stop.',
          },
        ]);
        return true;
      }
      if (line === '/status') {
        const refreshed = await this.responseTest(options.mode);
        writeBlock(tee, [{ kind: 'assistant', label: 'Gateway', text: refreshed.ok ? refreshed.label : refreshed.error || refreshed.label }]);
        return true;
      }
      if (line === '/agents') {
        selection = await this.renderAgentRoster(tee);
        return true;
      }
      if (line.startsWith('/use ')) {
        const nextAgent = clean(line.slice('/use '.length), 120);
        selection = { agentId: nextAgent, label: nextAgent || 'No agent selected', count: selection.count, source: nextAgent ? 'first' : 'none' };
        writeBlock(tee, [{ kind: 'assistant', label: 'Gateway', text: nextAgent ? `Selected ${nextAgent}.` : 'No agent selected.' }]);
        return true;
      }
      if (line === '/exit' || line === 'exit' || line === 'quit') {
        writeBlock(tee, [{ kind: 'assistant', label: 'Gateway', text: 'Use Ctrl-Z to stop the Terminal Shell.' }]);
        return true;
      }
      const before = selection.agentId ? await terminalSessionProjection(this.client, selection.agentId) : { messageCount: 0 };
      const ack = await submitTerminalUserInput(this.client, selection.agentId, line);
      if (ack.accepted) acceptedCount += 1;
      else rejectedCount += 1;
      if (!ack.accepted) writeBlock(tee, [{ kind: 'assistant', label: selection.label || 'Gateway', text: ack.label }]);
      if (ack.accepted) {
        writeBlock(tee, [{ kind: 'assistant', label: selection.label || 'Agent', text: 'Thinking...' }]);
        const pollOptions = options.scriptedInputs ? { attempts: 12 } : { attempts: 90, intervalMs: 1000 };
        const reply = await pollTerminalReplyProjection(this.client, selection.agentId, before.messageCount, pollOptions);
        const rows = reply.rows.length ? reply.rows : [{ text: `No assistant reply appeared in the current message window yet. ${ack.label}` }];
        for (const row of rows) writeBlock(tee, [{ kind: 'assistant', label: selection.label || 'Agent', text: clean(row.text || row.content_preview, 8000) }]);
      }
      return true;
    };

    if (options.scriptedInputs) {
      for (const line of options.scriptedInputs) {
        tee.write(`${TERMINAL_PROMPT} ${line}\n`);
        const keepGoing = await processLine(line);
        if (!keepGoing) break;
      }
    } else {
      const input = options.input || process.stdin;
      const rl = readline.createInterface({ input, output: process.stdout, terminal: true });
      let stopped = false;
      let previousRawMode: boolean | null = null;
      const stop = () => {
        if (stopped) return;
        stopped = true; stoppedBy = 'ctrl_z';
        tee.write('\n[infring terminal] stopped by Ctrl-Z\n');
        rl.close();
      };
      const interrupt = () => tee.write('\nUse Ctrl-Z to stop the Terminal Shell.\n');
      const onKeypress = (_str: string, key: { ctrl?: boolean; name?: string; sequence?: string } = {}) => {
        if (key.sequence === '\u001a' || (key.ctrl && key.name === 'z')) stop();
        if (key.ctrl && key.name === 'c') interrupt();
      };
      readline.emitKeypressEvents(input, rl);
      if (input?.isTTY && typeof input.setRawMode === 'function') {
        previousRawMode = Boolean(input.isRaw); input.setRawMode(true);
      }
      input.on?.('keypress', onKeypress);
      process.on('SIGTSTP', stop);
      process.on('SIGINT', interrupt);
      try {
        while (!stopped) {
          const line = await askLine(rl, TERMINAL_PROMPT);
          const keepGoing = await processLine(line);
          if (!keepGoing) break;
        }
      } finally {
        input.removeListener?.('keypress', onKeypress);
        if (previousRawMode !== null) input.setRawMode(previousRawMode);
        process.removeListener('SIGTSTP', stop);
        process.removeListener('SIGINT', interrupt);
        rl.close();
      }
    }

    return { ok: rejectedCount === 0, type: 'terminal_shell_interactive_session', mode: options.mode, base_url: this.baseUrl, selected_agent_id: selection.agentId, selected_agent_label: selection.label, turns, accepted_count: acceptedCount, rejected_count: rejectedCount, stopped_by: stoppedBy, transcript_preview: clean(transcript.join(''), 2000) };
  }
}

export async function runTerminalShellResponseTest(options: {
  live?: boolean;
  baseUrl?: string;
  requireLive?: boolean;
  outJson?: string;
  outMarkdown?: string;
} = {}): Promise<TerminalShellResponse> {
  const mode = options.live ? 'live' : 'fixture';
  const shell = new TerminalShell({
    baseUrl: options.live ? options.baseUrl || DEFAULT_GATEWAY_URL : 'http://terminal-shell.fixture',
    fetchImpl: options.live ? undefined : fixtureFetch(),
  });
  const response = await shell.responseTest(mode);
  if (options.outJson) writeJson(options.outJson, response);
  if (options.outMarkdown) writeMarkdown(options.outMarkdown, response);
  if (options.live && !response.ok && !options.requireLive) return { ...response, ok: true };
  return response;
}

export async function runTerminalShellInteractiveSmoke(options: {
  live?: boolean;
  baseUrl?: string;
  requireLive?: boolean;
  outJson?: string;
  outMarkdown?: string;
} = {}): Promise<TerminalShellInteractiveResult> {
  const mode = options.live ? 'live' : 'fixture';
  const shell = new TerminalShell({
    baseUrl: options.live ? options.baseUrl || DEFAULT_GATEWAY_URL : 'http://terminal-shell.fixture',
    fetchImpl: options.live ? undefined : fixtureFetch(),
  });
  const result = await shell.startInteractive({
    mode,
    requireLive: options.requireLive,
    output: { write: () => undefined },
    scriptedInputs: ['/status', 'hello from terminal shell'],
  });
  if (options.outJson) writeJson(options.outJson, result);
  if (options.outMarkdown) writeInteractiveMarkdown(options.outMarkdown, result);
  if (options.live && !result.ok && !options.requireLive) return { ...result, ok: true };
  return result;
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const live = parseBool(readFlag(argv, 'live', '0'), false);
  const requireLive = parseBool(readFlag(argv, 'require-live', '0'), false);
  const baseUrl = readFlag(argv, 'base-url', DEFAULT_GATEWAY_URL);
  const outJson = readFlag(argv, 'out-json', DEFAULT_OUT_JSON);
  const outMarkdown = readFlag(argv, 'out-markdown', DEFAULT_OUT_MARKDOWN);
  const outTerminal = parseBool(readFlag(argv, 'out-terminal', '0'), false);
  const interactive = parseBool(readFlag(argv, 'interactive', '0'), false);
  const interactiveSmoke = parseBool(readFlag(argv, 'interactive-smoke', '0'), false);
  if (interactiveSmoke) {
    const result = await runTerminalShellInteractiveSmoke({
      live,
      requireLive,
      baseUrl,
      outJson: readFlag(argv, 'out-json', DEFAULT_INTERACTIVE_OUT_JSON),
      outMarkdown: readFlag(argv, 'out-markdown', DEFAULT_INTERACTIVE_OUT_MARKDOWN),
    });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    process.exitCode = result.ok ? 0 : 1;
    return;
  }
  if (interactive) {
    const shell = new TerminalShell({
      baseUrl: live ? baseUrl : 'http://terminal-shell.fixture',
      fetchImpl: live ? undefined : fixtureFetch(),
    });
    const result = await shell.startInteractive({ mode: live ? 'live' : 'fixture', requireLive });
    process.exitCode = result.ok ? 0 : 1;
    return;
  }
  const response = await runTerminalShellResponseTest({ live, requireLive, baseUrl, outJson, outMarkdown });
  process.stdout.write(outTerminal ? `${renderTerminalResponse(response)}\n` : `${JSON.stringify(response, null, 2)}\n`);
  process.exitCode = response.ok ? 0 : 1;
}

if (process.argv[1]?.endsWith('terminal_shell.ts')) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
