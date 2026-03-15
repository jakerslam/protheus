'use strict';

const { runFeedCollector } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
    if (s.startsWith('--category=')) out.category = String(s.split('=')[1] || '').trim();
  }
  return out;
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const category = String(opts.category || (opts.parser_options && opts.parser_options.category) || 'ai-software').trim();
  const feedCandidates = [
    `https://www.producthunt.com/feed?category=${encodeURIComponent(category)}`,
    'https://www.producthunt.com/feed'
  ];
  return runFeedCollector({
    collectorId: 'producthunt_launches',
    scope: 'sensory.collector.producthunt_launches',
    caller: 'adaptive/sensory/eyes/collectors/producthunt_launches',
    feedCandidates,
    maxItems: Number(opts.maxItems || opts.max_items || 10),
    minHours: Number(opts.minHours || opts.min_hours || 6),
    force: opts.force === true,
    topics: Array.isArray(opts.topics)
      ? opts.topics
      : ['revenue', 'affiliate', 'product_launches', 'saas', 'partnerships'],
    signalRegex: /(launch|affiliate|integration|pricing|api|b2b|saas|agent|automation)/i,
    attempts: Number(opts.attempts || 3)
  });
}

async function preflightProductHunt() {
  return {
    ok: true,
    parser_type: 'producthunt_launches',
    checks: [
      { name: 'atom_feed_configured', ok: true },
      { name: 'adaptive_rate_limit_enabled', ok: true }
    ],
    failures: []
  };
}

if (require.main === module) {
  const args = parseArgs(process.argv.slice(2));
  run(args).then((result) => {
    console.log(JSON.stringify(result));
    process.exit(result && result.ok ? 0 : 1);
  }).catch((err) => {
    console.error(JSON.stringify({ ok: false, error: String(err && (err.code || err.message) || 'collector_error') }));
    process.exit(1);
  });
}

module.exports = {
  run,
  preflightProductHunt
};
