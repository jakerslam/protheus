#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin connector bridge over haystack-bridge)

const bridge = require('../../client/runtime/systems/workflow/haystack_bridge.ts');

function registerConnector(payload = {}) {
  return bridge.importConnector({
    bridge_path: 'adapters/protocol/haystack_connector_bridge.ts',
    ...payload,
  });
}

function registerDocumentStore(payload = {}) {
  return bridge.registerDocumentStore({
    bridge_path: 'adapters/protocol/haystack_connector_bridge.ts',
    ...payload,
  });
}

function retrieveDocuments(payload = {}) {
  return bridge.retrieveDocuments(payload);
}

module.exports = {
  registerConnector,
  registerDocumentStore,
  retrieveDocuments,
};
