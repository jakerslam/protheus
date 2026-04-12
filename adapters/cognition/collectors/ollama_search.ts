'use strict';
// Layer ownership: adapters/cognition/collectors (authoritative)

const adaptive = require('../../../client/cognition/shared/adaptive/sensory/eyes/collectors/ollama_search.ts');

async function run(options = {}) {
  return adaptive.run(options);
}

async function collectOllamaSearchNewest(options = {}) {
  return adaptive.collectOllamaSearchNewest(options);
}

async function preflightOllamaSearch() {
  return adaptive.preflightOllamaSearch();
}

function parseArgs(argv = []) {
  return adaptive.parseArgs(argv);
}

function extractOllamaModels(payload = {}) {
  return adaptive.extractOllamaModels(payload);
}

module.exports = {
  ...adaptive,
  run,
  parseArgs,
  extractOllamaModels,
  collectOllamaSearchNewest,
  preflightOllamaSearch
};
