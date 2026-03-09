#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/security (authoritative)
const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('remote-tamper-heartbeat', args);
}

if (require.main === module) {
  runSecurityPlaneCli('remote-tamper-heartbeat', process.argv.slice(2));
}

module.exports = {
  run
};
