#!/usr/bin/env node
'use strict';

const { createManifestLaneBridge } = require('../../lib/rust_lane_bridge');

function remapArgs(argv) {
  const args = Array.isArray(argv) ? argv.slice(0) : [];
  if (!args.length) return ['help'];

  const cmd = String(args[0] || '').trim().toLowerCase();
  const rest = args.slice(1);

  if (cmd === 'query') {
    const mapped = rest.map((token) => {
      const raw = String(token || '');
      if (raw.startsWith('--q=')) return `--query=${raw.slice('--q='.length)}`;
      if (raw === '--q') return '--query';
      if (raw.startsWith('--top=')) return `--limit=${raw.slice('--top='.length)}`;
      if (raw === '--top') return '--limit';
      return raw;
    });
    return ['recall', ...mapped];
  }

  if (cmd === 'status') {
    return ['probe', ...rest];
  }

  return [cmd, ...rest];
}

const bridge = createManifestLaneBridge(__dirname, 'memory_recall', {
  manifestPath: 'crates/memory/Cargo.toml',
  binaryName: 'memory-cli',
  binaryEnvVar: 'PROTHEUS_MEMORY_CORE_BIN'
});

if (require.main === module) {
  bridge.runCli(remapArgs(process.argv.slice(2)));
}

module.exports = {
  lane: bridge.lane,
  run: (args = []) => bridge.run(remapArgs(args))
};
