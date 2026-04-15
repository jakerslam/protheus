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
