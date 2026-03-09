#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/security (authoritative)
const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('critical-path-formal-verifier', args);
}

if (require.main === module) {
  runSecurityPlaneCli('critical-path-formal-verifier', process.argv.slice(2));
}

module.exports = {
  run
};
