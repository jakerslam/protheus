// Canonical Shell helper source: dashboard onboarding/auth prompt helpers.
// Loaded before app.ts by the dashboard asset router.
'use strict';

async function infringCheckOnboarding(page) {
  var target = page && typeof page === 'object' ? page : {};
  if (localStorage.getItem('infring-onboarded')) return;
  try {
    var config = await InfringAPI.get('/api/config');
    var apiKey = config && config.api_key;
    var noKey = !apiKey || apiKey === 'not set' || apiKey === '';
    if (noKey && target.agentCount === 0) target.showOnboarding = true;
  } catch(e) {
    if (target.agentCount === 0) target.showOnboarding = true;
  }
}

function infringDismissOnboarding(page) {
  var target = page && typeof page === 'object' ? page : {};
  target.showOnboarding = false;
  localStorage.setItem('infring-onboarded', 'true');
}

async function infringCheckAuth(page) {
  var target = page && typeof page === 'object' ? page : {};
  try {
    var authInfo = await InfringAPI.get('/api/auth/check');
    if (authInfo.mode === 'none') {
      target.authMode = 'apikey';
      target.sessionUser = null;
    } else if (authInfo.mode === 'session') {
      target.authMode = 'session';
      if (authInfo.authenticated) {
        target.sessionUser = authInfo.username;
        target.showAuthPrompt = false;
        return;
      }
      target.showAuthPrompt = true;
      return;
    }
  } catch(e) {}

  try {
    await InfringAPI.get('/api/tools');
    target.showAuthPrompt = false;
  } catch(e) {
    var message = e && e.message ? String(e.message) : '';
    if (message.indexOf('Not authorized') >= 0 || message.indexOf('401') >= 0 || message.indexOf('Missing Authorization') >= 0 || message.indexOf('Unauthorized') >= 0) {
      var saved = localStorage.getItem('infring-api-key');
      if (saved) {
        InfringAPI.setAuthToken('');
        localStorage.removeItem('infring-api-key');
      }
      target.showAuthPrompt = true;
    }
  }
}

function infringSubmitApiKey(page, key) {
  var target = page && typeof page === 'object' ? page : {};
  if (!key || !key.trim()) return;
  InfringAPI.setAuthToken(key.trim());
  localStorage.setItem('infring-api-key', key.trim());
  target.showAuthPrompt = false;
  if (typeof target.refreshAgents === 'function') target.refreshAgents();
}

async function infringSessionLogin(page, username, password) {
  var target = page && typeof page === 'object' ? page : {};
  try {
    var result = await InfringAPI.post('/api/shell-socket/auth/login', { username: username, password: password });
    if (result.status === 'ok') {
      target.sessionUser = result.username;
      target.showAuthPrompt = false;
      if (typeof target.refreshAgents === 'function') target.refreshAgents();
    } else {
      InfringToast.error(result.error || 'Login failed');
    }
  } catch(e) {
    InfringToast.error((e && e.message) || 'Login failed');
  }
}

async function infringSessionLogout(page) {
  var target = page && typeof page === 'object' ? page : {};
  try {
    await InfringAPI.post('/api/shell-socket/auth/logout');
  } catch(e) {}
  target.sessionUser = null;
  target.showAuthPrompt = true;
}

function infringClearApiKey() {
  InfringAPI.setAuthToken('');
  localStorage.removeItem('infring-api-key');
}
