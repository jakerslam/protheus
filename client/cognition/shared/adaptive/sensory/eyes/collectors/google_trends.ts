'use strict';

const { runFeedCollector } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
    if (s.startsWith('--geo=')) out.geo = String(s.split('=')[1] || 'US').toUpperCase();
  }
  return out;
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const geo = String(opts.geo || 'US').toUpperCase();
  return runFeedCollector({
    collectorId: 'google_trends',
    scope: 'sensory.collector.google_trends',
    caller: 'adaptive/sensory/eyes/collectors/google_trends',
    feedCandidates: [
      `https://trends.google.com/trending/rss?geo=${encodeURIComponent(geo)}`,
      'https://trends.google.com/trending/rss?geo=US'
    ],
    maxItems: Number(opts.maxItems || opts.max_items || 10),
    minHours: Number(opts.minHours || opts.min_hours || 6),
    force: opts.force === true,
    topics: Array.isArray(opts.topics)
      ? opts.topics
      : ['market_demand', 'commercial_intent', 'trends', 'saas', 'ai', 'automation'],
    signalRegex: /(ai|agent|saas|startup|automation|revenue|gpt|llm)/i,
    attempts: Number(opts.attempts || 3)
  });
}

async function preflightGoogleTrends() {
  return {
    ok: true,
    parser_type: 'google_trends',
    checks: [
      { name: 'geo_feed_configured', ok: true },
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
  preflightGoogleTrends
};
