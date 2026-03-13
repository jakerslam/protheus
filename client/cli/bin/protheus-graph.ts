#!/usr/bin/env node
'use strict';

const { run } = require('../../runtime/lib/ops_domain_conduit_runner.ts');

async function main(argv = process.argv.slice(2)) {
  const out = await run(['--domain=graph-toolkit', ...argv]);
  const payload = out && out.payload
    ? out.payload
    : {
      ok: false,
      type: 'graph_toolkit_cli_error',
      reason: 'missing_result',
      routed_via: 'conduit'
    };
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

if (require.main === module) {
  main().catch((err) => {
    process.stdout.write(
      `${JSON.stringify({
        ok: false,
        type: 'graph_toolkit_cli_error',
        reason: String(err && err.message ? err.message : err),
        routed_via: 'conduit'
      })}\n`
    );
    process.exit(1);
  });
}

module.exports = {
  main
};
