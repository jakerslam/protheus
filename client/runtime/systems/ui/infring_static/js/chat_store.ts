// InfringChatStore — writable stores for chat state (Svelte store contract).
// Loaded before Alpine so Svelte components and chat.ts can share reactive state
// without going through Alpine's proxy system.
'use strict';

window.InfringChatStore = (function() {
  function writable(initialValue) {
    var _value = initialValue;
    var _subscribers = [];
    function subscribe(run) {
      _subscribers.push(run);
      run(_value);
      return function() {
        var i = _subscribers.indexOf(run);
        if (i >= 0) _subscribers.splice(i, 1);
      };
    }
    function set(newValue) {
      _value = newValue;
      for (var i = 0; i < _subscribers.length; i++) {
        try { _subscribers[i](_value); } catch (_e) {}
      }
    }
    function update(fn) {
      set(fn(_value));
    }
    function get() {
      return _value;
    }
    return { subscribe: subscribe, set: set, update: update, get: get };
  }

  return {
    messages: writable([]),
    filteredMessages: writable([]),
    currentAgent: writable(null),
    agents: writable([]),
    sidebarAgents: writable([]),
    sessionLoading: writable(false),
    sending: writable(false),
    tokenCount: writable(0),
    inputText: writable(''),
    wsConnected: writable(false),
    showScrollDown: writable(false),
    stickToBottom: writable(true),
    mapStepIndex: writable(-1),
    focusMode: writable(false),
    connectionState: writable(''),
    theme: writable(''),
  };
}());
