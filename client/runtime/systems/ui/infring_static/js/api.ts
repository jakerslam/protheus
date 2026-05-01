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

  function isFetchDisconnectError(err) {
    if (!err) return false;
    var message = String(err && err.message ? err.message : '');
    return err.name === 'TypeError' &&
      (message.indexOf('Failed to fetch') >= 0 || message.indexOf('fetch failed') >= 0);
  }

  function request(method, path, body) {
    var opts = { method: method, headers: headers() };
    if (body !== undefined) opts.body = JSON.stringify(body);
    if (_connectionState === 'disconnected') setConnectionState('connecting');
    function attemptRequest(attempt) {
      var delayMs = HTTP_RETRY_DELAYS_MS[Math.max(0, Math.min(HTTP_RETRY_DELAYS_MS.length - 1, attempt))] || 0;
      var start = delayMs > 0 ? waitMs(delayMs) : Promise.resolve();
      return start.then(function() {
        return fetch(BASE + path, opts).then(function(r) {
          if (_connectionState !== 'connected') setConnectionState('connected');
          if (!r.ok) {
            // On 401, auto-show auth prompt so the user can re-enter their key
            if (r.status === 401 && typeof Alpine !== 'undefined') {
              try {
                var store = Alpine.store('app');
                if (store && !store.showAuthPrompt) {
                  _authToken = '';
                  localStorage.removeItem('infring-api-key');
                  store.showAuthPrompt = true;
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
        var retryable = isFetchDisconnectError(e) || isRetryableHttpStatus(status);
        var hasMoreRetries = (attempt + 1) < HTTP_RETRY_DELAYS_MS.length;
        if (retryable && hasMoreRetries) {
          return attemptRequest(attempt + 1);
        }
        if (isFetchDisconnectError(e)) {
          setConnectionState('reconnecting');
          throw new Error('Cannot connect to daemon after 5 attempts — is infring running?');
        }
        throw e;
      });
    }
    return attemptRequest(0);
  }
  function get(path) { return request('GET', path); }
  function getDashboardSnapshot(sinceHash) {
    var since = String(sinceHash || '').trim();
    var suffix = since ? ('?since=' + encodeURIComponent(since)) : '';
    return get('/api/dashboard/snapshot' + suffix);
  }
  function post(path, body) { return request('POST', path, body); }
  function put(path, body) { return request('PUT', path, body); }
  function patch(path, body) { return request('PATCH', path, body); }
  function del(path) { return request('DELETE', path); }

  // WebSocket manager with auto-reconnect
  var _ws = null;
  var _wsCallbacks = {};
  var _wsConnected = false;
  var _wsAgentId = null;
  var _wsManualDisconnect = false;
  var _reconnectTimer = null;
  var _reconnectAttempts = 0;
  var MAX_RECONNECT = Number.MAX_SAFE_INTEGER;
  var WS_RECONNECT_DELAY_MS = 1000;
  var _wsLastSignal = { key: '', ts: 0 };

  function shouldEmitWsSignal(key, windowMs) {
    var now = Date.now();
    var win = Number(windowMs || 0);
    if (!Number.isFinite(win) || win < 100) win = 1200;
    if (_wsLastSignal.key === key && (now - Number(_wsLastSignal.ts || 0)) <= win) return false;
    _wsLastSignal = { key: key, ts: now };
    return true;
  }

  function isTerminalAuthCloseEvent(evt) {
    var code = Number(evt && evt.code ? evt.code : 0);
    var reason = String(evt && evt.reason ? evt.reason : '').toLowerCase();
    if (code === 1008 || code === 4008 || code === 4401 || code === 4403) return true;
    return (
      reason.indexOf('unauthorized') >= 0 ||
      reason.indexOf('auth') >= 0 ||
      reason.indexOf('token') >= 0 ||
      reason.indexOf('pairing') >= 0 ||
      reason.indexOf('forbidden') >= 0
    );
  }

  function wsConnect(agentId, callbacks) {
    wsDisconnect(false);
    _wsCallbacks = callbacks || {};
    _wsAgentId = agentId;
    _wsManualDisconnect = false;
    _reconnectAttempts = 0;
    _doConnect(agentId);
  }

  function _doConnect(agentId) {
    try {
      var url = WS_BASE + '/api/agents/' + agentId + '/ws';
      if (_authToken) url += '?token=' + encodeURIComponent(_authToken);
      _ws = new WebSocket(url);

      _ws.onopen = function() {
        _wsConnected = true;
        _reconnectAttempts = 0;
        setConnectionState('connected');
        if (_reconnectAttempt > 0) {
          _reconnectAttempt = 0;
        }
        if (_wsCallbacks.onOpen) _wsCallbacks.onOpen();
      };

      _ws.onmessage = function(e) {
        try {
          var data = JSON.parse(e.data);
          if (_wsCallbacks.onMessage) _wsCallbacks.onMessage(data);
        } catch(err) { /* ignore parse errors */ }
      };

      _ws.onclose = function(e) {
        _wsConnected = false;
        _ws = null;
        if (_wsManualDisconnect) {
          if (_wsCallbacks.onClose && shouldEmitWsSignal('close:manual:' + String(e.code || 0), 1200)) {
            _wsCallbacks.onClose();
          }
          return;
        }
        if (isTerminalAuthCloseEvent(e)) {
          _wsAgentId = null;
          setConnectionState('disconnected');
          if (_wsCallbacks.onError && shouldEmitWsSignal('authclose:' + String(e.code || 0), 1200)) {
            _wsCallbacks.onError({
              code: Number(e && e.code ? e.code : 0),
              reason: String(e && e.reason ? e.reason : '')
            });
          }
          if (_wsCallbacks.onClose && shouldEmitWsSignal('close:auth:' + String(e.code || 0), 1200)) {
            _wsCallbacks.onClose();
          }
          return;
        }
        if (_wsAgentId && _reconnectAttempts < MAX_RECONNECT && e.code !== 1000) {
          _reconnectAttempts++;
          _reconnectAttempt = _reconnectAttempts;
          setConnectionState('reconnecting');
          _reconnectTimer = setTimeout(function() { _doConnect(_wsAgentId); }, WS_RECONNECT_DELAY_MS);
          if (_wsCallbacks.onReconnect && shouldEmitWsSignal('reconnect:' + String(e.code || 0), 900)) {
            _wsCallbacks.onReconnect({
              code: Number(e && e.code ? e.code : 0),
              attempt: _reconnectAttempts
            });
          }
          return;
        }
        if (_wsAgentId && _reconnectAttempts >= MAX_RECONNECT) {
          setConnectionState('reconnecting');
        }
        if (_wsCallbacks.onClose && shouldEmitWsSignal('close:terminal:' + String(e.code || 0), 1200)) _wsCallbacks.onClose();
      };

      _ws.onerror = function() {
        _wsConnected = false;
        if (!_wsManualDisconnect && _wsAgentId) {
          setConnectionState('reconnecting');
          if (_wsCallbacks.onReconnect && shouldEmitWsSignal('reconnect:error:' + String(_wsAgentId || ''), 900)) {
            _wsCallbacks.onReconnect({
              code: 0,
              attempt: _reconnectAttempts
            });
          }
          return;
        }
        if (_wsCallbacks.onError && shouldEmitWsSignal('error:' + String(_wsAgentId || ''), 1200)) _wsCallbacks.onError();
      };
    } catch(e) {
      _wsConnected = false;
    }
  }

  function wsDisconnect(manual) {
    _wsManualDisconnect = manual !== false;
    _wsAgentId = null;
    _reconnectAttempts = MAX_RECONNECT;
    if (_reconnectTimer) { clearTimeout(_reconnectTimer); _reconnectTimer = null; }
    if (_ws) { _ws.close(1000); _ws = null; }
    _wsConnected = false;
  }

  function wsSend(data) {
    if (_ws && _ws.readyState === WebSocket.OPEN) {
      _ws.send(JSON.stringify(data));
      return true;
    }
    return false;
  }

  function isWsConnected() { return _wsConnected; }

  function getConnectionState() { return _connectionState; }

  function getToken() { return _authToken; }

  function upload(agentId, file) {
    var hdrs = {
      'Content-Type': file.type || 'application/octet-stream',
      'X-Filename': file.name
    };
    if (_authToken) hdrs['Authorization'] = 'Bearer ' + _authToken;
    return fetch(BASE + '/api/agents/' + agentId + '/upload', {
      method: 'POST',
      headers: hdrs,
      body: file
    }).then(function(r) {
      return r.text().then(function(raw) {
        var payload = {};
        if (raw && String(raw).trim()) {
          try { payload = JSON.parse(raw); } catch (_) { payload = {}; }
        }
        if (!r.ok) {
          var reason = (payload && payload.error) ? String(payload.error) : '';
          throw new Error(reason || 'upload_failed');
        }
        if (!payload || typeof payload !== 'object' || !payload.file_id) {
          if (payload && payload.type === 'infring_external_compat_stub') {
            throw new Error('upload_endpoint_stub_requires_dashboard_restart');
          }
          throw new Error('upload_invalid_response');
        }
        return payload;
      });
    });
  }

  return {
    setAuthToken: setAuthToken,
    getToken: getToken,
    get: get,
    getDashboardSnapshot: getDashboardSnapshot,
    post: post,
    put: put,
    patch: patch,
    del: del,
    delete: del,
    upload: upload,
    wsConnect: wsConnect,
    wsDisconnect: wsDisconnect,
    wsSend: wsSend,
    isWsConnected: isWsConnected,
    getConnectionState: getConnectionState,
    onConnectionChange: onConnectionChange
  };
})();
