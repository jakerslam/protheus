#!/usr/bin/env node
'use strict';

// Layer ownership: client/cognition/habits/routines (authoritative)
// Thin compatibility wrapper only.
module.exports = require("./adaptive_candidate_proxy.js");
