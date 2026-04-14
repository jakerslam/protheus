#!/usr/bin/env node
'use strict';

// Layer ownership: adapters/protocol (thin connector bridge over haystack-bridge)

const bridge = require('../../client/runtime/systems/workflow/haystack_bridge.ts');
const BRIDGE_PATH = 'adapters/protocol/haystack_connector_bridge.ts';
const FRAMEWORK = 'haystack';

function withBridgeMetadata(payload = {}) {
  return {
    bridge_path: BRIDGE_PATH,
    framework: FRAMEWORK,
    ...payload,
  };
}

function status(payload = {}) {
  return bridge.status(withBridgeMetadata(payload));
}

function registerConnector(payload = {}) {
  return bridge.importConnector(withBridgeMetadata(payload));
}

function registerDocumentStore(payload = {}) {
  return bridge.registerDocumentStore(withBridgeMetadata(payload));
}

function retrieveDocuments(payload = {}) {
  return bridge.retrieveDocuments(withBridgeMetadata(payload));
}

module.exports = {
  BRIDGE_PATH,
  FRAMEWORK,
  withBridgeMetadata,
  status,
  registerConnector,
  registerDocumentStore,
  retrieveDocuments,
};
