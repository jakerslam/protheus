'use strict';
// Layer ownership: adapters/cognition/collectors (authoritative)

const adaptive = require('../../../client/cognition/shared/adaptive/sensory/eyes/collectors/ollama_search.ts');

async function collectOllamaSearchNewest(options = {}) {
  return adaptive.collectOllamaSearchNewest(options);
}

async function preflightOllamaSearch() {
  return adaptive.preflightOllamaSearch();
}

module.exports = {
  ...adaptive,
  collectOllamaSearchNewest,
  preflightOllamaSearch
};
