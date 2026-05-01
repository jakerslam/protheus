// Infring API Client — Fetch wrapper, WebSocket manager, auth injection, toast notifications
'use strict';

// ── Toast Notification System ──
var InfringToast = (function() {
  var _toastId = 0;

  function toast(message, type, duration) {
    type = type || 'info';
    duration = duration || 4000;
    var id = ++_toastId;

    try {
      window.dispatchEvent(new CustomEvent('infring:toast', {
        detail: {
          id: id,
          message: String(message || ''),
          type: type,
          ts: Date.now(),
          duration: duration
        }
      }));
    } catch(_) {}
    return id;
  }

  function success(msg, duration) { return toast(msg, 'success', duration); }
  function error(msg, duration) { return toast(msg, 'error', duration || 6000); }
  function warn(msg, duration) { return toast(msg, 'warn', duration || 5000); }
  function info(msg, duration) { return toast(msg, 'info', duration); }

  // Styled confirmation modal — replaces native confirm()
  function confirm(title, message, onConfirm) {
    var overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';

    var modal = document.createElement('div');
    modal.className = 'confirm-modal';

    var titleEl = document.createElement('div');
    titleEl.className = 'confirm-title';
    titleEl.textContent = title;
    modal.appendChild(titleEl);

    var msgEl = document.createElement('div');
    msgEl.className = 'confirm-message';
    msgEl.textContent = message;
    modal.appendChild(msgEl);

    var actions = document.createElement('div');
    actions.className = 'confirm-actions';

    var cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn btn-ghost confirm-cancel';
    cancelBtn.textContent = 'Cancel';
    actions.appendChild(cancelBtn);

    var okBtn = document.createElement('button');
    okBtn.className = 'btn btn-danger confirm-ok';
    okBtn.textContent = 'Confirm';
    actions.appendChild(okBtn);

    modal.appendChild(actions);
    overlay.appendChild(modal);

    function close() { if (overlay.parentNode) overlay.parentNode.removeChild(overlay); document.removeEventListener('keydown', onKey); }
    cancelBtn.onclick = close;
    okBtn.onclick = function() { close(); if (onConfirm) onConfirm(); };
    overlay.addEventListener('click', function(e) { if (e.target === overlay) close(); });

    function onKey(e) { if (e.key === 'Escape') close(); }
    document.addEventListener('keydown', onKey);

    document.body.appendChild(overlay);
    okBtn.focus();
  }

  return {
    toast: toast,
    success: success,
    error: error,
    warn: warn,
    info: info,
    confirm: confirm
  };
})();

// ── Friendly Error Messages ──
function friendlyError(status, serverMsg) {
  if (status === 0 || !status) return 'Cannot reach daemon — is infring running?';
  if (status === 401) return 'Not authorized — check your API key';
  if (status === 403) return 'Permission denied';
  if (status === 404) return serverMsg || 'Resource not found';
  if (status === 429) return 'Rate limited — slow down and try again';
  if (status === 413) return 'Request too large';
  if (status === 500) return 'Server error — check daemon logs';
  if (status === 502 || status === 503) return 'Daemon unavailable — is it running?';
  return serverMsg || 'Unexpected error (' + status + ')';
}

// ── API Client ──
var InfringAPI = (function() {
  var BASE = window.location.origin;
  var WS_BASE = BASE.replace(/^http/, 'ws');
  var _authToken = '';

  // Connection state tracking
  var _connectionState = 'connecting';
  var _reconnectAttempt = 0;
  var _connectionListeners = [];
  var HTTP_RETRY_DELAYS_MS = [0, 1000, 1000, 1000, 1000];
  var HTTP_DEFAULT_TIMEOUT_MS = 12000;
  var HTTP_STATUS_TIMEOUT_MS = 4500;
  var HTTP_AGENT_LIST_TIMEOUT_MS = 5000;

  function setAuthToken(token) { _authToken = token; }

  function headers() {
    var h = { 'Content-Type': 'application/json' };
    if (_authToken) h['Authorization'] = 'Bearer ' + _authToken;
    return h;
  }

  function setConnectionState(state) {
    if (_connectionState === state) return;
    _connectionState = state;
    _connectionListeners.forEach(function(fn) { fn(state); });
  }

  function onConnectionChange(fn) { _connectionListeners.push(fn); }

  function waitMs(ms) {
    return new Promise(function(resolve) { setTimeout(resolve, ms); });
  }

  function isRetryableHttpStatus(status) {
    return status === 502 || status === 503 || status === 504;
  }

  function requestTimeoutMs(path) {
    var requestPath = String(path || '');
    if (requestPath === '/api/status' || requestPath === '/api/version') return HTTP_STATUS_TIMEOUT_MS;
    if (requestPath.indexOf('/api/agents') === 0 && requestPath.indexOf('view=sidebar') >= 0) return HTTP_AGENT_LIST_TIMEOUT_MS;
    return HTTP_DEFAULT_TIMEOUT_MS;
  }

  function requestMaxAttempts(path) {
    var requestPath = String(path || '');
    if (requestPath === '/api/status' || requestPath === '/api/version') return 1;
    if (requestPath.indexOf('/api/agents') === 0 && requestPath.indexOf('view=sidebar') >= 0) return 1;
    return HTTP_RETRY_DELAYS_MS.length;
  }

  function fetchWithTimeout(url, opts, timeoutMs) {
    if (typeof AbortController === 'undefined') return fetch(url, opts);
    var controller = new AbortController();
    var requestOpts = Object.assign({}, opts, { signal: controller.signal });
    var timer = setTimeout(function() { controller.abort(); }, timeoutMs);
    return fetch(url, requestOpts).finally(function() {
      clearTimeout(timer);
    });
  }

  function isFetchTimeoutError(err) {
    if (!err) return false;
    return err.name === 'AbortError';
  }

  function isFetchDisconnectError(err) {
    if (!err) return false;
    var message = String(err && err.message ? err.message : '');
    return err.name === 'TypeError' &&
      (message.indexOf('Failed to fetch') >= 0 || message.indexOf('fetch failed') >= 0);
  }

  function request(method, path, body) {
    var opts = { method: method, headers: headers() };
    var maxAttempts = requestMaxAttempts(path);
    var timeoutMs = requestTimeoutMs(path);
    if (body !== undefined) opts.body = JSON.stringify(body);
    if (_connectionState === 'disconnected') setConnectionState('connecting');
    function attemptRequest(attempt) {
      var delayMs = HTTP_RETRY_DELAYS_MS[Math.max(0, Math.min(HTTP_RETRY_DELAYS_MS.length - 1, attempt))] || 0;
      var start = delayMs > 0 ? waitMs(delayMs) : Promise.resolve();
      return start.then(function() {
        return fetchWithTimeout(BASE + path, opts, timeoutMs).then(function(r) {
          if (_connectionState !== 'connected') setConnectionState('connected');
          if (!r.ok) {
            // On 401, auto-show auth prompt so the user can re-enter their key
            if (r.status === 401 && typeof Alpine !== 'undefined') {
              try {
                var bridge = typeof InfringSharedShellServices !== 'undefined' && InfringSharedShellServices.appStore
                  ? InfringSharedShellServices.appStore
                  : null;
                var store = bridge && typeof bridge.current === 'function' ? bridge.current() : null;
                if (store && !store.showAuthPrompt) {
                  _authToken = '';
                  localStorage.removeItem('infring-api-key');
                  if (bridge && typeof bridge.set === 'function') bridge.set('showAuthPrompt', true);
                }
              } catch(e2) { /* ignore Alpine errors */ }
            }
            return r.text().then(function(text) {
              var msg = '';
              try {
                var json = JSON.parse(text);
                msg = json.error || r.statusText;
              } catch(e) {
                msg = r.statusText;
              }
              var httpErr = /** @type {any} */ (new Error(friendlyError(r.status, msg)));
              httpErr.status = r.status;
              throw httpErr;
            });
          }
          var ct = r.headers.get('content-type') || '';
          if (ct.indexOf('application/json') >= 0) return r.json();
          return r.text().then(function(t) {
            try { return JSON.parse(t); } catch(e) { return { text: t }; }
          });
        });
      }).catch(function(e) {
        var errAny = /** @type {any} */ (e);
        var status = Number(errAny && errAny.status ? errAny.status : 0);
        var retryable = isFetchDisconnectError(e) || isFetchTimeoutError(e) || isRetryableHttpStatus(status);
        var hasMoreRetries = (attempt + 1) < maxAttempts;
        if (retryable && hasMoreRetries) {
          return attemptRequest(attempt + 1);
        }
        if (isFetchTimeoutError(e)) {
          setConnectionState('reconnecting');
          throw new Error('Request timed out after ' + timeoutMs + 'ms: ' + path);
        }
        if (isFetchDisconnectError(e)) {
          setConnectionState('reconnecting');
          throw new Error('Cannot connect to daemon after ' + maxAttempts + ' attempts — is infring running?');
        }
        throw e;
      });
    }
    return attemptRequest(0);
  }
