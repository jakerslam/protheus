function infringToggleTaskbarHeroMenu(page) {
  if (page.taskbarHeroActionPending) return;
  if (!page.taskbarHeroMenuOpen) page.closeTaskbarTextMenu();
  page.taskbarHeroMenuOpen = !page.taskbarHeroMenuOpen;
}

function infringRequestTaskbarRefresh(page) {
  page.closeTaskbarHeroMenu();
  var appStore = page.getAppStore ? page.getAppStore() : null;
  if (appStore && typeof appStore.bumpTaskbarRefreshTurn === 'function') {
    appStore.bumpTaskbarRefreshTurn();
  }
  if (page._taskbarRefreshOverlayTimer) {
    clearTimeout(page._taskbarRefreshOverlayTimer);
    page._taskbarRefreshOverlayTimer = 0;
  }
  if (page._taskbarRefreshReloadTimer) {
    clearTimeout(page._taskbarRefreshReloadTimer);
    page._taskbarRefreshReloadTimer = 0;
  }
  page._taskbarRefreshOverlayTimer = window.setTimeout(function() {
    page.bootSplashVisible = true;
    page._bootSplashStartedAt = Date.now();
    if (typeof page.resetBootProgress === 'function') page.resetBootProgress();
    if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('status_requesting');
    page._taskbarRefreshOverlayTimer = 0;
  }, 1000);
  page._taskbarRefreshReloadTimer = window.setTimeout(function() {
    page._taskbarRefreshReloadTimer = 0;
    try {
      window.location.reload();
    } catch (_) {
      try {
        window.location.href = window.location.href;
      } catch (_) {}
    }
  }, 1100);
}

async function infringPostTaskbarHeroSystemRoute(page, route, body, options) {
  var opts = (options && typeof options === 'object') ? options : {};
  var timeoutMs = Number(opts.timeoutMs);
  if (!Number.isFinite(timeoutMs) || timeoutMs < 250) timeoutMs = 1800;
  var allowTransientSuccess = opts.allowTransientSuccess === true;
  var controller = null;
  try {
    if (typeof AbortController !== 'undefined') controller = new AbortController();
  } catch (_) {
    controller = null;
  }
  var timer = 0;
  if (controller && typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
    timer = window.setTimeout(function() {
      try {
        controller.abort();
      } catch (_) {}
    }, timeoutMs);
  }
  try {
    var headers = { 'Content-Type': 'application/json' };
    try {
      var token = String(localStorage.getItem('infring-api-key') || '').trim();
      if (token) headers.Authorization = 'Bearer ' + token;
    } catch (_) {}
    var response = await fetch(route, {
      method: 'POST',
      headers: headers,
      body: JSON.stringify(body || {}),
      signal: controller ? controller.signal : undefined
    });
    var text = '';
    try {
      text = await response.text();
    } catch (_) {
      text = '';
    }
    var parsed = {};
    try {
      parsed = text ? JSON.parse(text) : {};
    } catch (_) {
      parsed = {};
    }
    if (!response.ok) {
      var error = new Error(String((parsed && (parsed.error || parsed.message)) || ('system_route_http_' + response.status)));
      error.status = response.status;
      error.payload = parsed;
      throw error;
    }
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch (error) {
    var message = String(error && error.message ? error.message : '');
    var aborted = !!(controller && controller.signal && controller.signal.aborted) || (error && error.name === 'AbortError');
    var disconnected =
      error &&
      error.name === 'TypeError' &&
      (message.indexOf('Failed to fetch') >= 0 || message.indexOf('fetch failed') >= 0);
    if (allowTransientSuccess && (aborted || disconnected)) {
      return {
        ok: true,
        type: 'dashboard_system_action_assumed',
        accepted_transient_disconnect: true
      };
    }
    throw error;
  } finally {
    if (timer) {
      try {
        clearTimeout(timer);
      } catch (_) {}
    }
  }
}

async function infringRunTaskbarHeroCommand(page, action) {
  var actionKey = String(action || '').trim().toLowerCase();
  if (!actionKey || page.taskbarHeroActionPending) return;
  var dashboardAction = '';
  var legacyRoute = '';
  var body = {};
  if (actionKey === 'restart') {
    dashboardAction = 'dashboard.system.restart';
    legacyRoute = '/api/system/restart';
  }
  else if (actionKey === 'shutdown') {
    dashboardAction = 'dashboard.system.shutdown';
    legacyRoute = '/api/system/shutdown';
  }
  else if (actionKey === 'update') {
    dashboardAction = 'dashboard.update.apply';
    legacyRoute = '/api/system/update';
    body = { apply: true };
  } else {
    return;
  }
  page.taskbarHeroActionPending = actionKey;
  try {
    var result = null;
    try {
      result = await page.postTaskbarHeroSystemRoute(legacyRoute, body, {
        timeoutMs: actionKey === 'update' ? 12000 : 1400,
        allowTransientSuccess: actionKey === 'restart' || actionKey === 'shutdown'
      });
    } catch (routeError) {
      var routeStatus = Number(routeError && routeError.status || 0);
      var routeMessage = String(routeError && routeError.message ? routeError.message : '').toLowerCase();
      var canFallbackToActionBus =
        !!dashboardAction &&
        (
          routeStatus === 404 ||
          routeStatus === 400 ||
          routeMessage.indexOf('unknown_action') >= 0 ||
          routeMessage.indexOf('resource not found') >= 0
        );
      if (!canFallbackToActionBus) throw routeError;
      result = await InfringAPI.post('/api/dashboard/action', {
        action: dashboardAction,
        payload: body
      });
    }
    var payload =
      result && result.lane && typeof result.lane === 'object'
        ? result.lane
        : (
          result && result.payload && typeof result.payload === 'object'
            ? result.payload
            : result
        );
    if (result && result.ok === false) {
      throw new Error(String(result.error || payload.error || (actionKey + '_failed')));
    }
    page.closeTaskbarHeroMenu();
    if (actionKey === 'restart') {
      InfringToast.success('Restart requested');
      page.requestTaskbarRefresh();
    } else if (actionKey === 'shutdown') {
      InfringToast.success('Shut down requested');
      page.connected = false;
      page.connectionState = 'disconnected';
      page.wsConnected = false;
    } else {
      var updateAvailable = payload.update_available;
      if (updateAvailable == null && payload.post_check && typeof payload.post_check === 'object') {
        updateAvailable = payload.post_check.has_update;
      }
      if (updateAvailable === false) {
        InfringToast.success('Already up to date');
      } else {
        InfringToast.success('Update requested');
      }
      page.requestTaskbarRefresh();
    }
  } catch (e) {
    InfringToast.error('Failed to ' + actionKey.replace(/_/g, ' ') + ': ' + (e && e.message ? e.message : 'unknown error'));
  } finally {
    page.taskbarHeroActionPending = '';
  }
}
