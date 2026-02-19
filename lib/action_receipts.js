'use strict';

const fs = require('fs');
const path = require('path');

function nowIso() {
  return new Date().toISOString();
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function appendJsonl(filePath, obj) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, JSON.stringify(obj) + '\n', 'utf8');
}

function withReceiptContract(record, { attempted = true, verified = false } = {}) {
  return {
    ...record,
    receipt_contract: {
      version: '1.0',
      attempted: attempted === true,
      verified: verified === true,
      recorded: true
    }
  };
}

function writeContractReceipt(filePath, record, { attempted = true, verified = false } = {}) {
  const withContract = withReceiptContract(record, { attempted, verified });
  appendJsonl(filePath, withContract);
  return withContract;
}

module.exports = {
  nowIso,
  appendJsonl,
  withReceiptContract,
  writeContractReceipt
};
