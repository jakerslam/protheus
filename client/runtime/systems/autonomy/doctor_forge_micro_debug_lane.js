#!/usr/bin/env node
'use strict';

const tsBootstrap = require('../../lib/ts_bootstrap.ts');
tsBootstrap.bootstrap(__filename, module);
