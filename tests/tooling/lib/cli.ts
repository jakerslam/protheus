#!/usr/bin/env node
'use strict';

export function cleanText(value: unknown, max = 400): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

export function parseBool(value: string | undefined, fallback = false): boolean {
  const normalized = cleanText(value || '', 32).toLowerCase();
  if (!normalized) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(normalized);
}

export function readFlag(argv: string[], name: string): string | undefined {
  const prefix = `--${name}=`;
  for (let index = 0; index < argv.length; index += 1) {
    const token = cleanText(argv[index], 1200);
    if (!token) continue;
    if (token === `--${name}`) {
      const next = argv[index + 1];
      return next == null ? '' : cleanText(next, 1200);
    }
    if (token.startsWith(prefix)) {
      return cleanText(token.slice(prefix.length), 1200);
    }
  }
  return undefined;
}

export function hasFlag(argv: string[], name: string): boolean {
  return argv.some((token) => cleanText(token, 200) === `--${name}`);
}

export function parseStrictOutArgs(
  argv: string[],
  defaults: {
    out?: string;
    strict?: boolean;
    json?: boolean;
  } = {},
) {
  const strictRaw = readFlag(argv, 'strict');
  const jsonRaw = readFlag(argv, 'json');
  return {
    strict: hasFlag(argv, 'strict') || parseBool(strictRaw, defaults.strict || false),
    json: hasFlag(argv, 'json') || parseBool(jsonRaw, defaults.json || false),
    out: cleanText(readFlag(argv, 'out') || defaults.out || '', 400),
  };
}

