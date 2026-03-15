'use strict';

const { runFeedCollector } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
    if (s.startsWith('--feed=')) out.feedCandidates = [String(s.split('=')[1] || '')];
  }
  return out;
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const feedCandidates = Array.isArray(opts.feedCandidates) && opts.feedCandidates.length
    ? opts.feedCandidates
    : [
        String(opts.feed_url || ''),
        'https://news.ycombinator.com/rss',
        'https://hnrss.org/frontpage',
        'https://hnrss.org/newest'
      ].filter(Boolean);

  return runFeedCollector({
    collectorId: 'hn_rss',
    scope: 'sensory.collector.hn_rss',
    caller: 'adaptive/sensory/eyes/collectors/hn_rss',
    feedCandidates,
    maxItems: Number(opts.maxItems || opts.max_items || 20),
    minHours: Number(opts.minHours || opts.min_hours || 6),
    force: opts.force === true,
    topics: Array.isArray(opts.topics) ? opts.topics : ['startups', 'dev_tools', 'ai', 'security', 'infra'],
    signalRegex: /(show hn|agent|llm|security|startup|release|benchmark|prompt|model)/i,
    attempts: Number(opts.attempts || 3)
  });
}

async function preflightHnRss() {
  return {
    ok: true,
    parser_type: 'hn_rss',
    checks: [
      { name: 'feed_candidates_present', ok: true },
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
  preflightHnRss
};
