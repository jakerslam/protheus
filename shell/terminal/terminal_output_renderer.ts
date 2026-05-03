#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const DEFAULT_OUT_TEXT = 'core/local/artifacts/terminal_shell_render_fixture_current.txt';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/TERMINAL_SHELL_RENDER_FIXTURE_CURRENT.md';

export type TerminalHeaderBlock = {
  kind: 'header';
  title?: string;
  state?: string;
  agent?: string;
  session?: string;
};

export type TerminalSpeakerBlock = {
  kind: 'user' | 'assistant';
  label?: string;
  text: string;
};

export type TerminalToolBlock = {
  kind: 'tool';
  name: string;
  status: string;
  summary?: string;
};

export type TerminalMetaBlock = {
  kind: 'meta';
  text: string;
};

export type TerminalRenderBlock = TerminalHeaderBlock | TerminalSpeakerBlock | TerminalToolBlock | TerminalMetaBlock;

export type TerminalRenderOptions = {
  prompt?: string;
};

function clean(value: unknown, max = 1200): string {
  return String(value == null ? '' : value).replace(/\r\n/g, '\n').trim().slice(0, max);
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

function writeText(filePath: string, body: string): void {
  const abs = path.resolve(process.cwd(), filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, body.endsWith('\n') ? body : `${body}\n`, 'utf8');
}

function parts(values: Array<string | undefined>): string {
  return values.map((value) => clean(value, 160)).filter(Boolean).join(' | ');
}

function renderUserText(text: string, prompt: string): string {
  const lines = clean(text, 4000).split('\n');
  return lines.map((line) => `${prompt} ${line}`.trimEnd()).join('\n');
}

function renderBlock(block: TerminalRenderBlock, options: Required<TerminalRenderOptions>): string {
  if (block.kind === 'header') {
    const title = clean(block.title || 'Infring', 80);
    const status = parts([
      block.state,
      block.agent ? `agent: ${block.agent}` : undefined,
      block.session ? `session: ${block.session}` : undefined,
    ]);
    return status ? `${title}\n${status}` : title;
  }
  if (block.kind === 'user') {
    return `${clean(block.label || 'You', 80)}\n${renderUserText(block.text, options.prompt)}`;
  }
  if (block.kind === 'assistant') {
    return `${clean(block.label || 'Infring', 80)}\n${clean(block.text, 8000)}`;
  }
  if (block.kind === 'tool') {
    return `Tool\n${parts([block.name, block.status, block.summary])}`;
  }
  return clean(block.text, 2000);
}

export function renderTerminalBlocks(blocks: TerminalRenderBlock[], options: TerminalRenderOptions = {}): string {
  const resolved = { prompt: clean(options.prompt || '>', 12) || '>' };
  return blocks.map((block) => renderBlock(block, resolved)).filter(Boolean).join('\n\n');
}

export function terminalRenderFixtureBlocks(): TerminalRenderBlock[] {
  return [
    { kind: 'header', title: 'Infring', state: 'ready', agent: 'Misty', session: 'current' },
    { kind: 'user', label: 'You', text: 'compare these files' },
    { kind: 'assistant', label: 'Misty', text: 'Thinking...' },
    { kind: 'tool', name: 'read_file', status: 'done', summary: '2 files' },
    { kind: 'assistant', label: 'Misty', text: "Here's the comparison..." },
  ];
}

export function renderTerminalFixture(): string {
  return renderTerminalBlocks(terminalRenderFixtureBlocks());
}

function markdown(body: string): string {
  return ['# Terminal Shell Render Fixture', '', '```text', body, '```', ''].join('\n');
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const fixture = parseBool(readFlag(argv, 'fixture', '1'), true);
  if (!fixture) throw new Error('Only --fixture=1 is supported before the interactive Terminal Shell exists.');
  const output = renderTerminalFixture();
  const outText = readFlag(argv, 'out-text', DEFAULT_OUT_TEXT);
  const outMarkdown = readFlag(argv, 'out-markdown', DEFAULT_OUT_MARKDOWN);
  writeText(outText, output);
  writeText(outMarkdown, markdown(output));
  process.stdout.write(`${output}\n`);
}

if (process.argv[1]?.endsWith('terminal_output_renderer.ts')) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
