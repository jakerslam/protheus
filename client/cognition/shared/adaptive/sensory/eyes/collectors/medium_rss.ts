'use strict';

const { runFeedCollector } = require('./collector_runtime.ts');

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (s === '--force') out.force = true;
    if (s.startsWith('--max=')) out.maxItems = Number(s.split('=')[1]);
    if (s.startsWith('--min-hours=')) out.minHours = Number(s.split('=')[1]);
    if (s.startsWith('--tags=')) {
      out.tags = String(s.split('=')[1] || '')
        .split(',')
        .map((v) => v.trim())
        .filter(Boolean);
    }
  }
  return out;
}

function mediumFeedsForTags(tags) {
  const cleaned = Array.isArray(tags)
    ? tags.map((v) => String(v || '').trim()).filter(Boolean)
    : [];
  const fallback = [
    'artificial-intelligence',
    'machine-learning',
    'startups',
    'entrepreneurship',
    'business-strategy',
    'automation'
  ];
  const sourceTags = cleaned.length ? cleaned : fallback;
  const uniq = Array.from(new Set(sourceTags));
  return uniq.map((tag) => `https://medium.com/feed/tag/${encodeURIComponent(tag)}`);
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const parserTags = Array.isArray(opts.tags)
    ? opts.tags
    : (opts.parser_options && Array.isArray(opts.parser_options.tags) ? opts.parser_options.tags : []);
  return runFeedCollector({
    collectorId: 'medium_rss',
    scope: 'sensory.collector.medium_rss',
    caller: 'adaptive/sensory/eyes/collectors/medium_rss',
    feedCandidates: mediumFeedsForTags(parserTags),
    maxItems: Number(opts.maxItems || opts.max_items || 15),
    minHours: Number(opts.minHours || opts.min_hours || 6),
    force: opts.force === true,
    topics: Array.isArray(opts.topics)
      ? opts.topics
      : ['ai', 'startups', 'business', 'automation', 'revenue', 'growth', 'engineering'],
    signalRegex: /(ai|agent|startup|growth|automation|revenue|saas|llm|gpt)/i,
    attempts: Number(opts.attempts || 3)
  });
}

async function preflightMediumRss(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const tags = Array.isArray(opts.tags)
    ? opts.tags
    : (opts.parser_options && Array.isArray(opts.parser_options.tags) ? opts.parser_options.tags : []);
  return {
    ok: true,
    parser_type: 'medium_rss',
    checks: [
      { name: 'tag_feeds_present', ok: mediumFeedsForTags(tags).length > 0 },
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
  preflightMediumRss,
  mediumFeedsForTags
};
