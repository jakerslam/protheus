#!/usr/bin/env node
/* eslint-disable no-console */
const fs = require('node:fs');
const path = require('node:path');
const esbuild = require('esbuild');
const { compile } = require('svelte/compiler');
const { cleanText, hasFlag, parseBool, readFlag } = require('../../lib/cli.ts');
const { emitStructuredResult } = require('../../lib/result.ts');

const SCRIPT_PATH = 'tests/tooling/scripts/ci/build_dashboard_svelte_islands.ts';
const ISLAND_SPECS = [
  {
    id: 'chat_bubble',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_bubble.bundle.ts',
    fallbackTag: 'infring-chat-bubble-render',
    filename: 'chat_bubble.svelte',
  },
  {
    id: 'chat_stream_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_stream_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_stream_shell.bundle.ts',
    fallbackTag: 'infring-chat-stream-shell',
    filename: 'chat_stream_shell.svelte',
  },
  {
    id: 'sidebar_rail_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_rail_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/sidebar_rail_shell.bundle.ts',
    fallbackTag: 'infring-sidebar-rail-shell',
    filename: 'sidebar_rail_shell.svelte',
  },
  {
    id: 'popup_window_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/popup_window_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/popup_window_shell.bundle.ts',
    fallbackTag: 'infring-popup-window-shell',
    filename: 'popup_window_shell.svelte',
  },
  {
    id: 'taskbar_menu_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_menu_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_menu_shell.bundle.ts',
    fallbackTag: 'infring-taskbar-menu-shell',
    filename: 'taskbar_menu_shell.svelte',
  },
  {
    id: 'chat_map_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/chat_map_shell.bundle.ts',
    fallbackTag: 'infring-chat-map-shell',
    filename: 'chat_map_shell.svelte',
  },
  {
    id: 'agent_details_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/agent_details_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/agent_details_shell.bundle.ts',
    fallbackTag: 'infring-agent-details-shell',
    filename: 'agent_details_shell.svelte',
  },
  {
    id: 'tool_card_stack_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/tool_card_stack_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/tool_card_stack_shell.bundle.ts',
    fallbackTag: 'infring-tool-card-stack-shell',
    filename: 'tool_card_stack_shell.svelte',
  },
  {
    id: 'composer_lane_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/composer_lane_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/composer_lane_shell.bundle.ts',
    fallbackTag: 'infring-composer-lane-shell',
    filename: 'composer_lane_shell.svelte',
  },
  {
    id: 'taskbar_dropdown_cluster_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_dropdown_cluster_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/taskbar_dropdown_cluster_shell.bundle.ts',
    fallbackTag: 'infring-taskbar-dropdown-cluster-shell',
    filename: 'taskbar_dropdown_cluster_shell.svelte',
  },
  {
    id: 'workspace_panel_shell',
    sourcePath: 'client/runtime/systems/ui/infring_static/js/svelte/workspace_panel_shell_svelte_source.ts',
    bundlePath: 'client/runtime/systems/ui/infring_static/js/svelte/workspace_panel_shell.bundle.ts',
    fallbackTag: 'infring-workspace-panel-shell',
    filename: 'workspace_panel_shell.svelte',
  },
];

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

function loadIslandSource(root, spec) {
  const sourceModulePath = path.resolve(root, spec.sourcePath);
  const sourceModule = require(sourceModulePath);
  const sourceText = String([
    sourceModule && sourceModule.COMPONENT_SOURCE,
    sourceModule && sourceModule.CHAT_BUBBLE_COMPONENT_SOURCE,
  ].find((value) => typeof value === 'string' && value.trim()) || '').trim();
  const tag = cleanText(
    (sourceModule && sourceModule.COMPONENT_TAG) || (sourceModule && sourceModule.CHAT_BUBBLE_TAG) || spec.fallbackTag,
    120
  ) || spec.fallbackTag;
  if (!sourceText) {
    throw new Error(`dashboard_svelte_source_missing:${spec.sourcePath}`);
  }
  return {
    id: spec.id,
    tag,
    source_text: sourceText,
    source_module: sourceModulePath,
    bundle_path: spec.bundlePath,
    filename: spec.filename || `${spec.id}.svelte`,
  };
}

async function buildDashboardSvelteIslands(options = {}, root = repoRoot(__dirname)) {
  const minify = options && options.minify !== false;
  const builtIslands = [];
  for (const spec of ISLAND_SPECS) {
    const source = loadIslandSource(root, spec);
    const outFile = path.resolve(root, source.bundle_path);
    fs.mkdirSync(path.dirname(outFile), { recursive: true });
    const compiled = compile(source.source_text, {
      filename: source.filename,
      generate: 'dom',
      dev: false,
      customElement: true,
    });
    await esbuild.build({
      stdin: {
        contents: String(compiled && compiled.js && compiled.js.code ? compiled.js.code : ''),
        loader: 'js',
        sourcefile: `${source.filename}.js`,
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
        js: `/* generated: dashboard svelte island bundle (${source.id}) */`,
      },
    });
    builtIslands.push({
      id: source.id,
      tag: source.tag,
      source_module: path.relative(root, source.source_module).replace(/\\/g, '/'),
      out_file: path.relative(root, outFile).replace(/\\/g, '/'),
      out_bytes: fs.statSync(outFile).size,
    });
  }

  return {
    ok: true,
    type: 'dashboard_svelte_islands_build',
    islands: builtIslands,
    island_count: builtIslands.length,
    chat_bubble_tag: builtIslands.find((item) => item.id === 'chat_bubble')?.tag || 'infring-chat-bubble-render',
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
  loadIslandSource,
  ISLAND_SPECS,
  buildDashboardSvelteIslands,
  run,
};
