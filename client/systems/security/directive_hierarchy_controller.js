#!/usr/bin/env node
'use strict';

// Compatibility shim: client/systems -> client/runtime/systems
module.exports = require('../../runtime/systems/security/directive_hierarchy_controller.js');

if (require.main === module) {
  const lane = module.exports;
  const out = lane.run ? lane.run(process.argv.slice(2)) : { status: 1 };
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}
