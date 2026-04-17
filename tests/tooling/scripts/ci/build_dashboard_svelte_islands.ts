#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('node:fs');
const path = require('node:path');
const esbuild = require('esbuild');
const { compile } = require('svelte/compiler');
const { cleanText, hasFlag, parseBool, readFlag } = require('../../lib/cli.ts');
const { emitStructuredResult } = require('../../lib/result.ts');

const SCRIPT_PATH = 'tests/tooling/scripts/ci/build_dashboard_svelte_islands.ts';
const CHAT_BUBBLE_SOURCE_PATH = 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble_svelte_source.ts';
const CHAT_BUBBLE_BUNDLE_PATH = 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble.bundle.ts';

function repoRoot(startDir = __dirname) {
  let dir = path.resolve(startDir);
  while (true) {
    const cargo = path.join(dir, 'Cargo.toml');
    const coreOps = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    if (fs.existsSync(cargo) && fs.existsSync(coreOps)) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return path.resolve(__dirname, '..', '..', '..', '..');
}

function parseArgs(argv) {
  const out = {
    minify: true,
    out: '',
  };
  const minifyFlag = readFlag(argv, 'minify');
  out.minify = hasFlag(argv, 'minify') || (minifyFlag != null ? parseBool(minifyFlag, true) : true);
  out.out = cleanText(readFlag(argv, 'out') || '', 400);
  return out;
}

function loadChatBubbleSource(root) {
  const sourceModulePath = path.resolve(root, CHAT_BUBBLE_SOURCE_PATH);
  const sourceModule = require(sourceModulePath);
  const sourceText = String(
    (sourceModule && sourceModule.CHAT_BUBBLE_COMPONENT_SOURCE) || ''
  ).trim();
  const tag = cleanText(
    (sourceModule && sourceModule.CHAT_BUBBLE_TAG) || 'infring-chat-bubble-render',
    120
  ) || 'infring-chat-bubble-render';
  if (!sourceText) {
    throw new Error(`dashboard_svelte_source_missing:${CHAT_BUBBLE_SOURCE_PATH}`);
  }
  return {
    tag,
    source_text: sourceText,
    source_module: sourceModulePath,
  };
}

async function buildDashboardSvelteIslands(options = {}, root = repoRoot(__dirname)) {
  const minify = options && options.minify !== false;
  const source = loadChatBubbleSource(root);
  const outFile = path.resolve(root, CHAT_BUBBLE_BUNDLE_PATH);
  fs.mkdirSync(path.dirname(outFile), { recursive: true });

  const compiled = compile(source.source_text, {
    filename: 'chat_bubble.svelte',
    generate: 'dom',
    dev: false,
    customElement: true,
  });

  await esbuild.build({
    stdin: {
      contents: String(compiled && compiled.js && compiled.js.code ? compiled.js.code : ''),
      loader: 'js',
      sourcefile: 'chat_bubble.svelte.js',
      resolveDir: root,
    },
    bundle: true,
    outfile: outFile,
    platform: 'browser',
    format: 'iife',
    target: 'es2020',
    sourcemap: false,
    minify,
    logLevel: 'silent',
    legalComments: 'none',
    banner: {
      js: '/* generated: dashboard svelte island bundle (chat bubble) */',
    },
  });

  return {
    ok: true,
    type: 'dashboard_svelte_islands_build',
    chat_bubble_tag: source.tag,
    source_module: path.relative(root, source.source_module).replace(/\\/g, '/'),
    out_file: path.relative(root, outFile).replace(/\\/g, '/'),
    out_bytes: fs.statSync(outFile).size,
    minify: Boolean(minify),
  };
}

async function run(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  try {
    const payload = await buildDashboardSvelteIslands({ minify: options.minify });
    emitStructuredResult(payload, {
      outPath: options.out || undefined,
      strict: false,
      ok: true,
      history: false,
      stdout: false,
    });
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return 0;
  } catch (error) {
    const payload = {
      ok: false,
      type: 'dashboard_svelte_islands_build_failed',
      error: cleanText(error && error.message ? error.message : String(error), 320),
    };
    emitStructuredResult(payload, {
      outPath: options.out || undefined,
      strict: true,
      ok: false,
      history: false,
      stdout: false,
    });
    process.stderr.write(`${JSON.stringify(payload)}\n`);
    return 1;
  }
}

if (require.main === module) {
  run(process.argv.slice(2)).then((code) => process.exit(code));
}

module.exports = {
  SCRIPT_PATH,
  repoRoot,
  parseArgs,
  loadChatBubbleSource,
  buildDashboardSvelteIslands,
  run,
};
