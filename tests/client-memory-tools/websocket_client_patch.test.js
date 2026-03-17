#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

class SimpleEventTarget {
  constructor() {
    this.listeners = new Map();
  }

  addEventListener(type, handler) {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set());
    this.listeners.get(type).add(handler);
  }

  removeEventListener(type, handler) {
    this.listeners.get(type)?.delete(handler);
  }

  dispatchEvent(event) {
    const handlers = Array.from(this.listeners.get(event.type) || []);
    for (const handler of handlers) {
      handler.call(this, event);
    }
    return true;
  }
}

class FakeSocket extends SimpleEventTarget {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  static created = [];

  constructor(url, protocols) {
    super();
    this.url = url;
    this.protocols = protocols;
    this.readyState = FakeSocket.CONNECTING;
    this.sent = [];
    FakeSocket.created.push(this);
  }

  send(data) {
    this.sent.push(data);
  }

  close(code = 1000, reason = '') {
    this.readyState = FakeSocket.CLOSED;
    this.dispatchEvent({ type: 'close', code, reason, wasClean: code === 1000 });
  }
}

function run() {
  const patchPath = path.resolve(__dirname, '..', '..', 'client', 'runtime', 'patches', 'websocket-client-patch.ts');
  const source = fs.readFileSync(patchPath, 'utf8');

  const timers = [];
  const storage = new Map();
  const context = {
    console,
    setTimeout(fn, delay) {
      timers.push({ fn, delay });
      return timers.length;
    },
    clearTimeout() {},
    Event: class Event {
      constructor(type) {
        this.type = type;
      }
    },
    MessageEvent: class MessageEvent {
      constructor(type, init = {}) {
        this.type = type;
        this.data = init.data;
        this.origin = init.origin;
      }
    },
    location: { search: '', origin: 'https://example.test' },
    sessionStorage: {
      getItem(key) {
        return storage.has(key) ? storage.get(key) : null;
      },
      setItem(key, value) {
        storage.set(key, String(value));
      }
    },
    document: {
      createElement() {
        return new SimpleEventTarget();
      }
    },
    fetch: async () => ({
      async json() {
        return { events: [] };
      }
    }),
    window: {
      WebSocket: FakeSocket
    }
  };
  context.window.window = context.window;
  context.window.document = context.document;
  context.window.location = context.location;
  context.window.sessionStorage = context.sessionStorage;
  context.window.fetch = context.fetch;
  context.window.console = console;
  context.window.Event = context.Event;
  context.window.MessageEvent = context.MessageEvent;
  context.window.setTimeout = context.setTimeout;
  context.window.clearTimeout = context.clearTimeout;

  vm.runInNewContext(source, context, { filename: patchPath });

  const PatchedSocket = context.window.WebSocket;
  const socket = new PatchedSocket('ws://example.test/socket');
  const rawSocket = FakeSocket.created[0];

  const seen = [];
  socket.addEventListener('open', () => seen.push('open'));
  socket.onmessage = (event) => seen.push(`message:${JSON.parse(event.data).payload}`);

  socket.send('queued-before-open');
  assert.deepStrictEqual(rawSocket.sent, []);

  rawSocket.readyState = FakeSocket.OPEN;
  rawSocket.dispatchEvent({ type: 'open' });

  assert.equal(seen[0], 'open');
  assert.equal(rawSocket.sent.length, 2);
  assert.equal(JSON.parse(rawSocket.sent[0]).type, 'subscribe');
  assert.equal(rawSocket.sent[1], 'queued-before-open');

  rawSocket.dispatchEvent({
    type: 'message',
    data: JSON.stringify({ type: 'event', event_id: 42, payload: 'ok' })
  });
  assert.equal(storage.get('ws_last_event_id'), '42');
  assert.equal(seen.includes('message:ok'), true);

  rawSocket.dispatchEvent({ type: 'close', code: 1006, reason: 'drop', wasClean: false });
  assert.equal(timers.length > 0, true);
}

run();
console.log(JSON.stringify({ ok: true, type: 'websocket_client_patch_test' }));
