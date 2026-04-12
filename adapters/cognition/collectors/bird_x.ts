'use strict';
// Layer ownership: adapters/cognition/collectors (authoritative)

const adaptive = require('../../../client/cognition/shared/adaptive/sensory/eyes/collectors/bird_x.ts');

async function run(options = {}) {
  return adaptive.run(options);
}

async function collectBirdX(options = {}) {
  return adaptive.collectBirdX(options);
}

async function preflightBirdX(options = {}) {
  return adaptive.preflightBirdX(options);
}

function parseArgs(argv = []) {
  return adaptive.parseArgs(argv);
}

module.exports = {
  ...adaptive,
  run,
  parseArgs,
  collectBirdX,
  preflightBirdX,
};
